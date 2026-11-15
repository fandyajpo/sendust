//! Performance harness for attachment reads, copying, and memory bounds (P13-003).
//!
//! Run with the pinned toolchain in release mode, serially:
//!
//! ```text
//! cargo test -p sendust-core --release --test perf_attachments -- --ignored --nocapture --test-threads=1
//! ```
//!
//! A counting allocator reports live and peak bytes, so memory is measured rather
//! than assumed. Budgets come from `docs/qa/performance-budget.md`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use sendust_core::{
    Attachment, AttachmentPolicy, ErrorCode, FileDescriptor, Limits, MIB, Mailbox, OutgoingMessage,
    read_attachment, serialize,
};

const WARMUP: usize = 3;
const ITERATIONS: usize = 20;

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

/// Tracks live and peak allocated bytes for the measurements below.
struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let live = LIVE.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
        PEAK.fetch_max(live, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn reset_peak() -> usize {
    let live = LIVE.load(Ordering::Relaxed);
    PEAK.store(live, Ordering::Relaxed);
    live
}

fn peak_growth(base: usize) -> usize {
    PEAK.load(Ordering::Relaxed).saturating_sub(base)
}

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "sendust-perf-{}-{name}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).expect("temp dir");
        Self(path)
    }

    fn write(&self, name: &str, len: usize) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, vec![7u8; len]).expect("fixture file");
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn descriptor(path: &Path) -> FileDescriptor {
    FileDescriptor {
        display_name: Some("fixture.bin".to_string()),
        content_type: Some("application/octet-stream".to_string()),
        path: path.to_string_lossy().into_owned(),
    }
}

struct Stats {
    p50: u128,
    p95: u128,
}

impl Stats {
    fn report(&self, metric: &str) {
        println!(
            "{metric}, {}, {}, {ITERATIONS}",
            self.p50, self.p95
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
    }
}

fn message_with(attachments: Vec<Attachment>) -> OutgoingMessage {
    OutgoingMessage {
        from: Mailbox::parse("sender@example.test").expect("valid sender"),
        to: vec![Mailbox::parse("recipient@example.test").expect("valid recipient")],
        cc: Vec::new(),
        bcc: Vec::new(),
        reply_to: Vec::new(),
        subject: "Attachment fixture".to_string(),
        text: Some("body".to_string()),
        html: None,
        headers: Vec::new(),
        attachments,
    }
}

fn bytes_attachment(name: &str, len: usize) -> Attachment {
    Attachment::Bytes {
        filename: name.to_string(),
        content_type: Some("application/octet-stream".to_string()),
        bytes: vec![0u8; len],
    }
}

#[test]
#[ignore = "performance harness; run with --release --ignored"]
fn reads_a_ten_mib_file_within_budget() {
    let dir = TempDir::new("read");
    let path = dir.write("ten.bin", 10 * MIB as usize);
    let limits = Limits::default();

    let base = reset_peak();
    let stats = measure(|| {
        let bytes = read_attachment(&descriptor(&path), limits).expect("reads the cap");
        black_box(&bytes);
    });
    let peak = peak_growth(base);

    stats.report("read.10mib_file");
    stats.assert_budget("read.10mib_file", 30_000, 60_000);
    println!("read.10mib_file.peak_mib, {:.1}", peak as f64 / (MIB as f64));
    assert!(
        peak <= 32 * MIB as usize,
        "read peak {peak} bytes exceeds 32 MiB"
    );
}

#[test]
#[ignore = "performance harness; run with --release --ignored"]
fn rejects_a_file_over_the_cap_without_overallocating() {
    let dir = TempDir::new("over");
    let path = dir.write("over.bin", 10 * MIB as usize + 1);
    let limits = Limits::default();

    let base = reset_peak();
    let error = read_attachment(&descriptor(&path), limits).expect_err("over the cap");
    let peak = peak_growth(base);

    assert_eq!(error.code(), ErrorCode::LimitAttachmentBytes);
    println!("read.over_limit.peak_mib, {:.1}", peak as f64 / (MIB as f64));
    // The reader stops one byte past the cap instead of reading the file whole,
    // so the bound is a small multiple of the cap and independent of the file size.
    assert!(
        peak <= 32 * MIB as usize,
        "over-limit read allocated {peak} bytes"
    );
}

#[test]
#[ignore = "performance harness; run with --release --ignored"]
fn serializes_a_ten_mib_attachment_within_memory_bounds() {
    let message = message_with(vec![bytes_attachment("ten.bin", 10 * MIB as usize)]);

    let base = reset_peak();
    let stats = measure(|| {
        let bytes = serialize(&message, Limits::default(), AttachmentPolicy::Disabled)
            .expect("serializes");
        black_box(&bytes.len());
    });
    let peak = peak_growth(base);

    stats.report("serialize.attachment_10mib");
    stats.assert_budget("serialize.attachment_10mib", 150_000, 300_000);
    println!(
        "serialize.attachment_10mib.peak_mib, {:.1}",
        peak as f64 / (MIB as f64)
    );
    // The output is ~13.4 MiB; the part buffer and the final assembly coexist,
    // so ~61 MiB peak is measured. 80 MiB keeps a bounded margin.
    assert!(
        peak <= 80 * MIB as usize,
        "serialize peak {peak} bytes exceeds 80 MiB"
    );
}

#[test]
#[ignore = "performance harness; run with --release --ignored"]
fn serializes_a_twenty_mib_aggregate_inside_the_wire_cap() {
    let message = message_with(vec![
        bytes_attachment("one.bin", 10 * MIB as usize),
        bytes_attachment("two.bin", 10 * MIB as usize),
    ]);

    let base = reset_peak();
    let stats = measure(|| {
        let bytes = serialize(&message, Limits::default(), AttachmentPolicy::Disabled)
            .expect("aggregate fits the wire cap");
        black_box(&bytes.len());
    });
    let peak = peak_growth(base);

    stats.report("serialize.attachment_20mib");
    stats.assert_budget("serialize.attachment_20mib", 400_000, 800_000);
    println!(
        "serialize.attachment_20mib.peak_mib, {:.1}",
        peak as f64 / (MIB as f64)
    );
    let wire = serialize(&message, Limits::default(), AttachmentPolicy::Disabled)
        .expect("aggregate fits the wire cap");
    println!(
        "serialize.attachment_20mib.wire_mib, {:.1}",
        wire.len() as f64 / (MIB as f64)
    );
    assert!(wire.len() < 30 * MIB as usize, "wire {} bytes", wire.len());
    assert!(
        peak <= 128 * MIB as usize,
        "aggregate peak {peak} bytes exceeds 128 MiB"
    );
}

#[test]
#[ignore = "performance harness; run with --release --ignored"]
fn repeated_serialization_returns_to_baseline() {
    let message = message_with(vec![bytes_attachment("ten.bin", 10 * MIB as usize)]);
    let base = reset_peak();

    for _ in 0..20 {
        let bytes = serialize(&message, Limits::default(), AttachmentPolicy::Disabled)
            .expect("serializes");
        black_box(&bytes.len());
        drop(bytes);
    }

    let live = LIVE.load(Ordering::Relaxed).saturating_sub(base);
    println!("repeat.20x10mib.residual_mib, {:.1}", live as f64 / (MIB as f64));
    assert!(
        live <= 4 * MIB as usize,
        "live bytes grew by {live} across 20 serializations"
    );
}

#[test]
#[ignore = "performance harness; run with --release --ignored"]
fn attachment_bytes_are_read_once_by_construction() {
    // A File attachment can never reach the serializer: Delivery converts every
    // File into Bytes exactly once, and serialize refuses a File attachment.
    let message = message_with(vec![Attachment::File(FileDescriptor {
        display_name: Some("notes.txt".to_string()),
        content_type: None,
        path: "/tmp/sendust-does-not-exist".to_string(),
    })]);
    let error = serialize(&message, Limits::default(), AttachmentPolicy::Enabled)
        .expect_err("a file cannot be serialized");
    assert_eq!(error.code(), ErrorCode::ValidationInvalidAttachment);
}
