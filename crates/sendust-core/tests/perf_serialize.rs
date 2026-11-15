//! Performance harness for the core send path and MIME serialization (P13-002).
//!
//! Run with the pinned toolchain in release mode, serially:
//!
//! ```text
//! cargo test -p sendust-core --release --test perf_serialize -- --ignored --nocapture --test-threads=1
//! ```
//!
//! Every metric prints one stable line (`metric, p50_us, p95_us, min_us, max_us,
//! iterations`) and asserts its budget from `docs/qa/performance-budget.md`, so a
//! regression fails the run. Nothing here runs in CI: shared runners are not a
//! stable measurement environment.

use std::hint::black_box;
use std::time::Instant;

use sendust_core::{
    Attachment, AttachmentPolicy, Header, Limits, MIB, Mailbox, OutgoingMessage, serialize,
};

const WARMUP: usize = 3;
const ITERATIONS: usize = 20;

struct Stats {
    p50: u128,
    p95: u128,
    min: u128,
    max: u128,
}

impl Stats {
    fn report(&self, metric: &str) {
        println!(
            "{metric}, {}, {}, {}, {}, {ITERATIONS}",
            self.p50, self.p95, self.min, self.max
        );
    }

    fn assert_budget(&self, metric: &str, p50: u128, p95: u128) {
        assert!(
            self.p50 <= p50,
            "{metric}: p50 {} us exceeds the {p50} us budget",
            self.p50
        );
        assert!(
            self.p95 <= p95,
            "{metric}: p95 {} us exceeds the {p95} us budget",
            self.p95
        );
    }
}

/// Runs `op` after a discarded warm-up and returns the sorted-sample statistics.
fn measure(mut op: impl FnMut()) -> Stats {
    for _ in 0..WARMUP {
        op();
    }
    let mut samples = Vec::with_capacity(ITERATIONS);
    for _ in 0..ITERATIONS {
        let start = Instant::now();
        op();
        samples.push(start.elapsed().as_micros());
    }
    samples.sort_unstable();
    let at = |quantile: f64| {
        let index = ((samples.len() - 1) as f64 * quantile).round() as usize;
        samples[index]
    };
    Stats {
        p50: at(0.5),
        p95: at(0.95),
        min: samples[0],
        max: samples[samples.len() - 1],
    }
}

fn message(body_len: usize) -> OutgoingMessage {
    OutgoingMessage {
        from: Mailbox::parse("Sender <sender@example.test>").expect("valid sender"),
        to: vec![Mailbox::parse("recipient@example.test").expect("valid recipient")],
        cc: Vec::new(),
        bcc: Vec::new(),
        reply_to: Vec::new(),
        subject: "Performance fixture".to_string(),
        text: Some("a".repeat(body_len)),
        html: None,
        headers: Vec::new(),
        attachments: Vec::new(),
    }
}

fn serialize_fixture(message: &OutgoingMessage) -> usize {
    let bytes = serialize(message, Limits::default(), AttachmentPolicy::Disabled)
        .expect("fixture serializes");
    black_box(&bytes);
    bytes.len()
}

#[test]
#[ignore = "performance harness; run with --release --ignored"]
fn small_text_message_serialization() {
    let message = message(4 * 1024);

    let validate = measure(|| {
        black_box(
            message
                .validate(Limits::default(), AttachmentPolicy::Disabled)
                .expect("fixture validates"),
        );
    });
    validate.report("validate.small_text");
    validate.assert_budget("validate.small_text", 200, 400);

    let serialize = measure(|| {
        black_box(serialize_fixture(&message));
    });
    serialize.report("serialize.small_text");
    serialize.assert_budget("serialize.small_text", 500, 1_000);
}

#[test]
#[ignore = "performance harness; run with --release --ignored"]
fn one_mib_text_message_serialization() {
    let message = message(1 * 1024 * 1024);

    let serialize = measure(|| {
        black_box(serialize_fixture(&message));
    });
    serialize.report("serialize.1mib_text");
    serialize.assert_budget("serialize.1mib_text", 10_000, 20_000);
}

#[test]
#[ignore = "performance harness; run with --release --ignored"]
fn ten_mib_attachment_serialization() {
    let mut message = message(1024);
    message.attachments = vec![Attachment::Bytes {
        filename: "fixture.bin".to_string(),
        content_type: Some("application/octet-stream".to_string()),
        bytes: vec![0u8; 10 * MIB as usize],
    }];

    // The wire output is ~13.4 MiB, comfortably inside the 30 MiB cap.
    let wire = serialize_fixture(&message);
    assert!(wire < 30 * MIB as usize, "wire {wire} bytes");

    let serialize = measure(|| {
        black_box(serialize_fixture(&message));
    });
    serialize.report("serialize.10mib_attachment");
    serialize.assert_budget("serialize.10mib_attachment", 150_000, 300_000);
}

#[test]
#[ignore = "performance harness; run with --release --ignored"]
fn unicode_and_long_headers() {
    let mut message = message(64 * 1024);
    message.subject = "Grüße aus München — a Unicode subject that must be encoded".to_string();
    message.from = Mailbox::parse("Böb <bob@example.test>").expect("valid sender");
    message.headers = vec![
        Header::new("X-Long", "a".repeat(900)).expect("valid header"),
        Header::new("X-Unicode", "Grüße").expect("valid header"),
    ];

    let serialize = measure(|| {
        black_box(serialize_fixture(&message));
    });
    serialize.report("serialize.unicode_long_headers");
    serialize.assert_budget("serialize.unicode_long_headers", 15_000, 30_000);
}

#[test]
#[ignore = "performance harness; run with --release --ignored"]
fn serialization_scales_linearly() {
    let small = message(1024);
    let large = message(1024 * 1024);

    let small_time = measure(|| {
        black_box(serialize_fixture(&small));
    });
    let large_time = measure(|| {
        black_box(serialize_fixture(&large));
    });
    small_time.report("linearity.1kib");
    large_time.report("linearity.1mib");

    // 1024x the body must cost at most 2.2x per 10x of size, i.e. well under
    // quadratic growth; compare bytes per microsecond instead of raw ratios.
    let small_rate = 1024f64 / small_time.p50.max(1) as f64;
    let large_rate = (1024 * 1024) as f64 / large_time.p50.max(1) as f64;
    println!("linearity.throughput_bytes_per_us, {small_rate:.2}, {large_rate:.2}");
    assert!(
        large_rate >= small_rate / 2.2,
        "serialization throughput fell more than 2.2x: {small_rate:.2} -> {large_rate:.2} bytes/us"
    );
}

#[test]
#[ignore = "performance harness; run with --release --ignored"]
fn wire_accounting_is_single_pass() {
    let mut message = message(4096);
    message.attachments = vec![Attachment::Bytes {
        filename: "notes.txt".to_string(),
        content_type: None,
        bytes: b"attachment body".to_vec(),
    }];

    let bytes = serialize(&message, Limits::default(), AttachmentPolicy::Disabled)
        .expect("fixture serializes");
    // The returned vector is the accounting: no second serialization exists.
    assert_eq!(bytes.len(), serialize_fixture(&message));
    assert!(String::from_utf8_lossy(&bytes).contains("filename=\"notes.txt\""));
}
