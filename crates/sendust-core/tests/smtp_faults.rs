//! SMTP adapter fault matrix against the loopback fake SMTP/TLS harness.
//!
//! Every test talks to 127.0.0.1 only, trusts the committed fixture CA, and
//! asserts relay-level semantics from ADR-0005 and the error model. A positive
//! result is relay acceptance, never mailbox delivery.

mod support;

use std::net::{SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::time::Duration;

use sendust_core::{
    Delivery, DeliveryStatus, ErrorCode, Limits, Mailbox, OutgoingMessage, RecipientStatus,
    SmtpConfig, SmtpCredentials, SmtpSecurity, SmtpTransport, Stage, Timeouts,
};
use support::tls;
use support::{AfterBody, FakeSmtp, Reply, Script};

fn tls_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("tests/fixtures/tls")
}

fn ca_pem() -> Vec<u8> {
    std::fs::read(tls_dir().join("ca.crt")).expect("read ca fixture")
}

/// A harness serving implicit TLS with the committed `valid-server` chain.
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

fn mailbox(value: &str) -> Mailbox {
    value.parse().expect("valid mailbox")
}

fn message(to: &[&str]) -> OutgoingMessage {
    OutgoingMessage {
        from: mailbox("alice@example.test"),
        to: to.iter().map(|value| mailbox(value)).collect(),
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
    Delivery::new(SmtpTransport::new(config), Limits::default(), false)
}

#[tokio::test]
async fn accepts_a_single_recipient_transaction() {
    let server = serving(Script::default());
    let mut delivery = delivery(config(server.addr()));

    let outcome = delivery
        .send(message(&["bob@example.test"]))
        .await
        .expect("sent");

    assert_eq!(outcome.status(), DeliveryStatus::Accepted);
    assert_eq!(outcome.recipients().len(), 1);
    assert_eq!(outcome.recipients()[0].status, RecipientStatus::Accepted);
    assert_eq!(outcome.response().expect("response").code, 250);

    let trace = server.trace();
    assert_eq!(trace.count_prefix("MAIL"), 1);
    assert_eq!(trace.count_prefix("RCPT"), 1);
    assert_eq!(trace.count_prefix("DATA"), 1);
    assert!(
        trace.body_text().contains("Subject: Hello"),
        "body was not transmitted: {:?}",
        trace.body_text()
    );
}

#[tokio::test]
async fn all_recipients_refused_never_sends_data() {
    let script = Script::default().rcpt_reply(Reply::line("550 5.1.1 No such user"));
    let server = serving(script);
    let mut delivery = delivery(config(server.addr()));

    let outcome = delivery
        .send(message(&["one@example.test", "two@example.test"]))
        .await
        .expect("rejected outcome");

    assert_eq!(outcome.status(), DeliveryStatus::Rejected);
    assert!(
        outcome
            .recipients()
            .iter()
            .all(|recipient| recipient.status == RecipientStatus::Rejected)
    );
    assert!(outcome.response().is_none());
    assert_eq!(server.trace().count_prefix("RCPT"), 2);
    assert_eq!(server.trace().count_prefix("DATA"), 0);
}

#[tokio::test]
async fn mixed_recipient_replies_share_one_data_transaction() {
    let script = Script::default().rcpt_replies(vec![
        Reply::line("250 2.1.5 Recipient ok"),
        Reply::line("550 5.1.1 No such user"),
    ]);
    let server = serving(script);
    let mut delivery = delivery(config(server.addr()));

    let outcome = delivery
        .send(message(&["ok@example.test", "missing@example.test"]))
        .await
        .expect("sent");

    assert_eq!(outcome.status(), DeliveryStatus::Accepted);
    assert_eq!(outcome.recipients()[0].status, RecipientStatus::Accepted);
    assert_eq!(outcome.recipients()[1].status, RecipientStatus::Rejected);

    let trace = server.trace();
    assert_eq!(trace.count_prefix("MAIL"), 1, "no per-recipient fan-out");
    assert_eq!(trace.count_prefix("RCPT"), 2);
    assert_eq!(trace.count_prefix("DATA"), 1);
}

#[tokio::test]
async fn explicit_final_data_rejection_is_a_rejected_outcome() {
    let script =
        Script::default().after_body(AfterBody::Reply(Reply::line("451 4.3.0 Try again later")));
    let server = serving(script);
    let mut delivery = delivery(config(server.addr()));

    let outcome = delivery
        .send(message(&["bob@example.test"]))
        .await
        .expect("rejected outcome");

    assert_eq!(outcome.status(), DeliveryStatus::Rejected);
    assert_eq!(outcome.recipients()[0].status, RecipientStatus::Rejected);
    assert_eq!(outcome.response().expect("response").code, 451);
    assert_eq!(server.trace().count_prefix("DATA"), 1, "no retry");
}

#[tokio::test]
async fn missing_final_reply_after_data_is_unknown() {
    let script = Script::default().after_body(AfterBody::Close);
    let server = serving(script);
    let mut delivery = delivery(config(server.addr()));

    let outcome = delivery
        .send(message(&["bob@example.test"]))
        .await
        .expect("unknown outcome");

    assert_eq!(outcome.status(), DeliveryStatus::Unknown);
    assert_eq!(outcome.recipients()[0].status, RecipientStatus::Unknown);
    assert!(outcome.response().is_none());
    assert_eq!(
        server.trace().count_prefix("DATA"),
        1,
        "an unknown outcome is never retried"
    );
}

#[tokio::test]
async fn auth_failure_is_an_error_and_never_mails() {
    let script = Script {
        ehlo: Reply::lines(&["250-sendust.test", "250-AUTH PLAIN LOGIN", "250 8BITMIME"]),
        auth: Reply::line("535 5.7.8 Bad credentials"),
        ..Script::default()
    };
    let server = serving(script);
    let credentials = SmtpCredentials::new("relay-user", "hunter2").expect("valid credentials");
    let mut delivery = delivery(config(server.addr()).with_credentials(credentials));

    let error = delivery
        .send(message(&["bob@example.test"]))
        .await
        .expect_err("auth failure");

    assert_eq!(error.code(), ErrorCode::AuthFailed);
    assert_eq!(error.details().smtp_status(), Some(535));
    assert!(!error.to_string().contains("hunter2"));
    assert_eq!(server.trace().count_prefix("MAIL"), 0);
    assert_eq!(server.trace().count_prefix("DATA"), 0);
}

#[tokio::test]
async fn auth_without_a_supported_mechanism_is_unsupported() {
    let server = serving(Script::default());
    let credentials = SmtpCredentials::new("relay-user", "hunter2").expect("valid credentials");
    let mut delivery = delivery(config(server.addr()).with_credentials(credentials));

    let error = delivery
        .send(message(&["bob@example.test"]))
        .await
        .expect_err("no mechanism");

    assert_eq!(error.code(), ErrorCode::AuthUnsupported);
    assert_eq!(server.trace().count_prefix("MAIL"), 0);
    assert_eq!(server.trace().count_prefix("DATA"), 0);
}

#[tokio::test]
async fn an_untrusted_certificate_fails_closed() {
    let server = serving(Script::default());
    let untrusted = SmtpConfig::new("127.0.0.1", server.addr().port(), SmtpSecurity::ImplicitTls)
        .expect("valid relay");
    let mut delivery = delivery(untrusted);

    let error = delivery
        .send(message(&["bob@example.test"]))
        .await
        .expect_err("untrusted certificate");

    assert_eq!(error.code(), ErrorCode::TlsCertificate);
    assert_eq!(error.category().as_str(), "tls");
    assert_eq!(server.trace().count_prefix("MAIL"), 0);
}

#[tokio::test]
async fn connection_refused_is_a_protocol_error() {
    let port = {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
        listener.local_addr().expect("addr").port()
    };
    let unreachable = SmtpConfig::new("127.0.0.1", port, SmtpSecurity::ImplicitTls)
        .expect("valid relay")
        .with_root_certificate_pem(ca_pem())
        .expect("trusted fixture root");
    let mut delivery = delivery(unreachable);

    let error = delivery
        .send(message(&["bob@example.test"]))
        .await
        .expect_err("connection refused");

    assert_eq!(error.code(), ErrorCode::ProtocolConnectionReset);
    assert_eq!(error.details().port_number(), Some(port));
}

#[tokio::test]
async fn starttls_is_required_and_fails_closed_without_the_extension() {
    let server = FakeSmtp::start(Script::default()).expect("plain fake smtp");
    let mut delivery = delivery(
        SmtpConfig::new("127.0.0.1", server.addr().port(), SmtpSecurity::StartTls)
            .expect("valid relay")
            .with_root_certificate_pem(ca_pem())
            .expect("trusted fixture root"),
    );

    let error = delivery
        .send(message(&["bob@example.test"]))
        .await
        .expect_err("starttls required");

    assert_eq!(error.code(), ErrorCode::TlsHandshake);
    assert_eq!(server.trace().count_prefix("MAIL"), 0);
    assert_eq!(server.trace().count_prefix("DATA"), 0);
}

#[tokio::test]
async fn a_refused_sender_is_a_rejected_outcome() {
    let script = Script {
        mail: Reply::line("550 5.7.1 Relay access denied"),
        ..Script::default()
    };
    let server = serving(script);
    let mut delivery = delivery(config(server.addr()));

    let outcome = delivery
        .send(message(&["bob@example.test"]))
        .await
        .expect("rejected outcome");

    assert_eq!(outcome.status(), DeliveryStatus::Rejected);
    assert_eq!(outcome.recipients()[0].status, RecipientStatus::Rejected);
    assert_eq!(outcome.response().expect("response").code, 550);
    assert_eq!(server.trace().count_prefix("RCPT"), 0);
    assert_eq!(server.trace().count_prefix("DATA"), 0);
}

#[tokio::test]
async fn an_intermediate_rcpt_reply_is_a_protocol_error() {
    let script = Script::default().rcpt_reply(Reply::line("354 Send data now"));
    let server = serving(script);
    let mut delivery = delivery(config(server.addr()));

    let error = delivery
        .send(message(&["bob@example.test"]))
        .await
        .expect_err("protocol violation");

    assert_eq!(error.code(), ErrorCode::ProtocolUnexpectedResponse);
    assert_eq!(error.details().operation_stage(), Some(Stage::Rcpt));
    assert_eq!(error.details().smtp_status(), Some(354));
    assert_eq!(server.trace().count_prefix("DATA"), 0);
}

/// Budgets lowered in proportion to the approved maximums so a stall of the
/// same shape as the 11 s / 31 s / 121 s cases runs in seconds.
fn lowered(connect: u64, operation: u64, total: u64, close: u64) -> Timeouts {
    Timeouts::default()
        .with_connect(Duration::from_secs(connect))
        .expect("connect budget")
        .with_operation(Duration::from_secs(operation))
        .expect("operation budget")
        .with_total(Duration::from_secs(total))
        .expect("total budget")
        .with_close(Duration::from_secs(close))
        .expect("close budget")
}

#[tokio::test]
async fn every_bad_certificate_fixture_fails_before_mail() {
    for name in ["self-signed", "expired", "wrong-host", "incomplete-chain"] {
        let script = Script {
            tls: Some(tls::server_config_named(&tls_dir(), name)),
            ..Script::default()
        };
        let server = FakeSmtp::start(script).expect("start fake smtp");
        let mut delivery = delivery(config(server.addr()));

        let error = delivery
            .send(message(&["bob@example.test"]))
            .await
            .expect_err(name);

        assert_eq!(error.code(), ErrorCode::TlsCertificate, "{name}");
        assert_eq!(server.trace().count_prefix("MAIL"), 0, "{name}");
        assert_eq!(server.trace().count_prefix("DATA"), 0, "{name}");
    }

    let malformed = std::fs::read(tls_dir().join("malformed.pem")).expect("malformed fixture");
    let error = SmtpConfig::new("127.0.0.1", 465, SmtpSecurity::ImplicitTls)
        .expect("valid relay")
        .with_root_certificate_pem(malformed)
        .expect_err("malformed anchor");
    assert_eq!(error.code(), ErrorCode::ValidationInvalidOption);
}

#[tokio::test]
async fn greeting_reset_is_a_protocol_error() {
    // A plaintext greeting keeps the reset inside the SMTP session: the TLS
    // handshake is not involved, so the failure is the missing banner.
    let server = FakeSmtp::start(Script::default().hangup(support::Stage::Greeting))
        .expect("start plain fake smtp");
    let mut delivery = delivery(
        SmtpConfig::new("127.0.0.1", server.addr().port(), SmtpSecurity::StartTls)
            .expect("valid relay")
            .with_root_certificate_pem(ca_pem())
            .expect("trusted fixture root"),
    );

    let error = delivery
        .send(message(&["bob@example.test"]))
        .await
        .expect_err("greeting reset");

    assert_eq!(error.code(), ErrorCode::ProtocolConnectionReset);
    assert_eq!(
        error.details().operation_stage(),
        Some(Stage::ConnectionSetup)
    );
}

#[tokio::test]
async fn data_command_reset_is_a_protocol_error() {
    let server = serving(Script::default().hangup(support::Stage::Data));
    let mut delivery = delivery(config(server.addr()));

    let error = delivery
        .send(message(&["bob@example.test"]))
        .await
        .expect_err("reset before the DATA reply");

    assert_eq!(error.code(), ErrorCode::ProtocolConnectionReset);
    assert_eq!(error.details().operation_stage(), Some(Stage::DataCommand));
    assert!(server.trace().body().is_empty(), "no body was written");
}

#[tokio::test]
async fn connect_stall_is_a_connect_timeout() {
    let script = Script::default().delay(support::Stage::Greeting, Duration::from_millis(2_500));
    let server = serving(script);
    let mut delivery = delivery(
        config(server.addr())
            .with_timeouts(lowered(2, 2, 2, 2))
            .expect("lowered budgets"),
    );

    let error = delivery
        .send(message(&["bob@example.test"]))
        .await
        .expect_err("connect stall");

    assert_eq!(error.code(), ErrorCode::TimeoutConnect);
    assert_eq!(
        error.details().operation_stage(),
        Some(Stage::ConnectionSetup)
    );
}

#[tokio::test]
async fn command_stall_is_an_operation_timeout() {
    let script = Script::default().delay(support::Stage::Mail, Duration::from_millis(1_500));
    let server = serving(script);
    let mut delivery = delivery(
        config(server.addr())
            .with_timeouts(lowered(1, 1, 2, 2))
            .expect("lowered budgets"),
    );

    let error = delivery
        .send(message(&["bob@example.test"]))
        .await
        .expect_err("command stall");

    assert_eq!(error.code(), ErrorCode::TimeoutOperation);
    assert_eq!(error.details().operation_stage(), Some(Stage::Mail));
}

#[tokio::test]
async fn total_stall_is_a_total_timeout() {
    let script = Script::default()
        .delay(support::Stage::Greeting, Duration::from_millis(1_500))
        .delay(support::Stage::Rcpt, Duration::from_millis(2_000));
    let server = serving(script);
    let mut delivery = delivery(
        config(server.addr())
            .with_timeouts(lowered(2, 1, 2, 2))
            .expect("lowered budgets"),
    );

    let error = delivery
        .send(message(&["bob@example.test"]))
        .await
        .expect_err("total stall");

    assert_eq!(error.code(), ErrorCode::TimeoutTotal);
}
