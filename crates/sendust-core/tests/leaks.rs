//! Resource-cleanup probes for the delivery path.
//!
//! Repeated success and failure loops must not leave file descriptors behind.
//! Everything here talks to loopback only; no external network is used.

mod support;

use std::net::{SocketAddr, TcpListener};
use std::path::{Path, PathBuf};

use sendust_core::{
    AttachmentPolicy, Delivery, DeliveryStatus, ErrorCode, Limits, Mailbox, OutgoingMessage,
    SmtpConfig, SmtpSecurity, SmtpTransport,
};
use support::tls;
use support::{FakeSmtp, Script};

fn tls_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("tests/fixtures/tls")
}

fn ca_pem() -> Vec<u8> {
    std::fs::read(tls_dir().join("ca.crt")).expect("read ca fixture")
}

fn serving(script: Script) -> FakeSmtp {
    let mut script = script;
    script.tls = Some(tls::server_config(&tls_dir()));
    FakeSmtp::start(script).expect("start fake smtp over tls")
}

fn config(addr: SocketAddr) -> SmtpConfig {
    SmtpConfig::new("127.0.0.1", addr.port(), SmtpSecurity::ImplicitTls)
        .expect("valid relay")
        .with_root_certificate_pem(ca_pem())
        .expect("trusted fixture root")
}

fn message() -> OutgoingMessage {
    let mailbox = |value: &str| -> Mailbox { value.parse().expect("valid mailbox") };
    OutgoingMessage {
        from: mailbox("alice@example.test"),
        to: vec![mailbox("bob@example.test")],
        cc: Vec::new(),
        bcc: Vec::new(),
        reply_to: Vec::new(),
        subject: "Hello".to_string(),
        text: Some("body".to_string()),
        html: None,
        headers: Vec::new(),
        attachments: Vec::new(),
    }
}

fn delivery(config: SmtpConfig) -> Delivery<SmtpTransport> {
    Delivery::new(
        SmtpTransport::new(config),
        Limits::default(),
        AttachmentPolicy::Disabled,
    )
}

#[cfg(unix)]
fn open_fds() -> usize {
    std::fs::read_dir("/dev/fd")
        .map(|entries| entries.count())
        .unwrap_or(0)
}

/// Runs one probe with the process to itself.
///
/// Descriptor counts are process-wide, so only one probe may hold a runtime at
/// a time. A `#[tokio::test]` would build its runtime before the body runs,
/// which leaves a waiting probe holding three descriptors (the reactor and its
/// wakers) while the running probe measures: on a loaded runner those arrive
/// after the baseline snapshot and look exactly like a leak, which is how one
/// of these probes failed on CI with a jump of nine. Taking a blocking guard and
/// building the runtime behind it means a probe that has not started has opened
/// nothing, and the runtime's descriptors are closed before the guard is
/// released because it is declared last.
#[cfg(unix)]
fn probe(body: impl std::future::Future<Output = ()>) {
    static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build the test runtime");
    runtime.block_on(body);
}

/// The probe fails only if descriptors stay above the bound.
///
/// Closing a socket is asynchronous in the kernel and the runtime may open a
/// bookkeeping descriptor late, so a count taken immediately after the loop can
/// see a few descriptors that disappear milliseconds later. A bounded grace
/// period keeps the bound strict without turning that into a flake.
#[cfg(unix)]
async fn assert_descriptors_settled(before: usize, limit: usize, label: &str) {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(500);
    loop {
        let after = open_fds();
        if after <= before + limit {
            return;
        }
        if tokio::time::Instant::now() >= deadline {
            panic!("{label}: {before} -> {after}");
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}

#[cfg(unix)]
#[test]
fn repeated_successful_sends_do_not_leak_descriptors() {
    probe(async {
        let server = serving(Script::default());
        let config = config(server.addr());

        for _ in 0..3 {
            delivery(config.clone())
                .send(message())
                .await
                .expect("warm-up send");
        }
        let before = open_fds();

        for _ in 0..20 {
            delivery(config.clone())
                .send(message())
                .await
                .expect("send");
        }

        assert_descriptors_settled(before, 2, "descriptor leak across successes").await;
    });
}

#[cfg(unix)]
#[test]
fn repeated_failed_connections_do_not_leak_descriptors() {
    probe(async {
        let port = {
            let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
            listener.local_addr().expect("addr").port()
        };
        let config = SmtpConfig::new("127.0.0.1", port, SmtpSecurity::ImplicitTls)
            .expect("valid relay")
            .with_root_certificate_pem(ca_pem())
            .expect("trusted fixture root");

        for _ in 0..3 {
            delivery(config.clone())
                .send(message())
                .await
                .expect_err("refused");
        }
        let before = open_fds();

        for _ in 0..50 {
            let error = delivery(config.clone())
                .send(message())
                .await
                .expect_err("refused");
            assert_eq!(error.code(), ErrorCode::ProtocolConnectionReset);
        }

        assert_descriptors_settled(before, 2, "descriptor leak across refused connections").await;
    });
}

#[cfg(unix)]
#[test]
fn mixed_stress_loop_keeps_descriptors_and_outcomes_stable() {
    probe(async {
        let server = serving(Script::default());
        let good = config(server.addr());
        let refused_port = {
            let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
            listener.local_addr().expect("addr").port()
        };
        let refused = SmtpConfig::new("127.0.0.1", refused_port, SmtpSecurity::ImplicitTls)
            .expect("valid relay")
            .with_root_certificate_pem(ca_pem())
            .expect("trusted fixture root");

        for _ in 0..3 {
            delivery(good.clone())
                .send(message())
                .await
                .expect("warm-up");
            delivery(refused.clone())
                .send(message())
                .await
                .expect_err("warm-up refused");
        }
        let before = open_fds();
        let data_before = server.trace().count_prefix("DATA");

        for iteration in 0..100 {
            if iteration % 2 == 0 {
                let outcome = delivery(good.clone())
                    .send(message())
                    .await
                    .expect("stress send");
                assert_eq!(outcome.status(), DeliveryStatus::Accepted);
            } else {
                let error = delivery(refused.clone())
                    .send(message())
                    .await
                    .expect_err("stress refusal");
                assert_eq!(error.code(), ErrorCode::ProtocolConnectionReset);
            }
        }

        assert_descriptors_settled(before, 2, "descriptor leak across the stress loop").await;
        assert_eq!(server.trace().count_prefix("DATA") - data_before, 50);
    });
}

#[cfg(unix)]
#[test]
fn repeated_tls_failures_do_not_leak_descriptors() {
    probe(async {
        let server = serving(Script::default());
        // No fixture root is trusted here, so every handshake fails closed.
        let config = SmtpConfig::new("127.0.0.1", server.addr().port(), SmtpSecurity::ImplicitTls)
            .expect("valid relay");

        for _ in 0..3 {
            delivery(config.clone())
                .send(message())
                .await
                .expect_err("untrusted");
        }
        let before = open_fds();

        for _ in 0..20 {
            let error = delivery(config.clone())
                .send(message())
                .await
                .expect_err("untrusted");
            assert_eq!(error.code(), ErrorCode::TlsCertificate);
        }

        assert_descriptors_settled(before, 2, "descriptor leak across TLS failures").await;
        assert_eq!(server.trace().count_prefix("MAIL"), 0);
    });
}
