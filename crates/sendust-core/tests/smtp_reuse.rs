//! Session reuse across messages (ADR-0022).
//!
//! Two sends over one transport must pay one TCP/TLS/AUTH setup, not two, and a
//! session that ended ambiguously must not be reused.

mod support;

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use sendust_core::{
    AttachmentPolicy, Delivery, Limits, Mailbox, OutgoingMessage, ReusePolicy, SmtpConfig,
    SmtpSecurity, SmtpTransport,
};
use support::tls;
use support::{AfterBody, FakeSmtp, Reply, Script};

fn tls_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("tests/fixtures/tls")
}

fn serving(script: Script) -> FakeSmtp {
    let mut script = script;
    script.tls = Some(tls::server_config(&tls_dir()));
    FakeSmtp::start(script).expect("start fake smtp over tls")
}

fn config(addr: SocketAddr) -> SmtpConfig {
    SmtpConfig::new("127.0.0.1", addr.port(), SmtpSecurity::ImplicitTls)
        .expect("valid relay")
        .with_root_certificate_pem(std::fs::read(tls_dir().join("ca.crt")).expect("ca fixture"))
        .expect("trusted fixture root")
}

fn mailbox(value: &str) -> Mailbox {
    value.parse().expect("valid mailbox")
}

fn message() -> OutgoingMessage {
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

#[tokio::test]
async fn a_second_message_reuses_the_authenticated_session() {
    let server = serving(Script::default());
    let mut delivery = delivery(config(server.addr()));

    let first = delivery.send(message()).await.expect("first send");
    assert_eq!(first.status().as_str(), "accepted");
    let second = delivery.send(message()).await.expect("second send");
    assert_eq!(second.status().as_str(), "accepted");

    let trace = server.trace();
    assert_eq!(
        trace.count_prefix("EHLO"),
        1,
        "one greeting for both messages"
    );
    assert_eq!(
        trace.count_prefix("MAIL"),
        2,
        "both messages used the envelope"
    );
    assert_eq!(
        trace.count_prefix("RSET"),
        0,
        "a completed transaction needs no reset, so no round trip is spent on it"
    );
}

#[tokio::test]
async fn an_abandoned_envelope_is_reset_before_the_session_is_reused() {
    let server = serving(Script::default().rcpt_replies(vec![Reply::line("550 5.1.1 no such user")]));
    let mut delivery = delivery(config(server.addr()));

    let first = delivery.send(message()).await.expect("first send");
    assert_eq!(first.status().as_str(), "rejected");
    let second = delivery.send(message()).await.expect("second send");
    assert_eq!(second.status().as_str(), "rejected");

    let trace = server.trace();
    assert_eq!(
        trace.count_prefix("EHLO"),
        1,
        "the session survived a refused envelope"
    );
    assert_eq!(
        trace.count_prefix("RSET"),
        2,
        "each abandoned transaction was reset before reuse"
    );
}

#[tokio::test]
async fn the_message_cap_closes_the_session_before_the_next_send() {
    let server = serving(Script::default());
    let capped = config(server.addr()).with_reuse(ReusePolicy::default().with_max_messages(1));
    let mut delivery = delivery(capped);

    delivery.send(message()).await.expect("first send");
    delivery.send(message()).await.expect("second send");

    let trace = server.trace();
    assert_eq!(trace.count_prefix("EHLO"), 2, "a new session per message");
    assert_eq!(
        trace.count_prefix("QUIT"),
        2,
        "each capped session was closed"
    );
}

#[tokio::test]
async fn an_unknown_disposition_is_not_reused() {
    let server = serving(Script {
        after_body: AfterBody::Close,
        ..Script::default()
    });
    let mut delivery = delivery(config(server.addr()));

    let first = delivery.send(message()).await.expect("first send");
    assert_eq!(first.status().as_str(), "unknown");
    let second = delivery.send(message()).await.expect("second send");
    assert_eq!(second.status().as_str(), "unknown");

    let trace = server.trace();
    assert_eq!(
        trace.count_prefix("EHLO"),
        2,
        "an ambiguous session is never trusted again"
    );
    assert_eq!(trace.count_prefix("RSET"), 0);
}

#[tokio::test]
async fn shutdown_quits_a_retained_session() {
    let server = serving(Script::default());
    let mut transport = SmtpTransport::new(config(server.addr()));
    let mut delivery = Delivery::new(
        &mut transport,
        Limits::default(),
        AttachmentPolicy::Disabled,
    );
    delivery.send(message()).await.expect("send");
    transport.shutdown().await;

    assert_eq!(server.trace().count_prefix("QUIT"), 1);
}
