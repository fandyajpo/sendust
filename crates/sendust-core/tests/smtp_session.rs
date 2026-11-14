//! Session and AUTH tests against the loopback harness (P06-003).

mod support;

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use sendust_core::__internal::Session;
use sendust_core::ErrorCode;
use sendust_core::{SmtpConfig, SmtpSecurity};
use support::tls;
use support::{FakeSmtp, Reply, Script};

fn tls_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("tests/fixtures/tls")
}

fn ca_pem() -> Vec<u8> {
    std::fs::read(tls_dir().join("ca.crt")).expect("ca fixture")
}

fn config(addr: SocketAddr, security: SmtpSecurity) -> SmtpConfig {
    SmtpConfig::new("127.0.0.1", addr.port(), security)
        .expect("valid relay")
        .with_root_certificate_pem(ca_pem())
        .expect("trusted fixture root")
}

fn serving(script: Script) -> FakeSmtp {
    let mut script = script;
    script.tls = Some(tls::server_config(&tls_dir()));
    FakeSmtp::start(script).expect("start fake smtp")
}

fn serving_starttls(script: Script) -> FakeSmtp {
    let mut script = script;
    script.tls = Some(tls::server_config(&tls_dir()));
    script.starttls_mode = true;
    FakeSmtp::start(script).expect("start fake smtp")
}

fn auth_ehlo() -> Reply {
    Reply::lines(&["250-sendust.test", "250-AUTH PLAIN LOGIN", "250 8BITMIME"])
}

#[tokio::test]
async fn authenticates_with_plain() {
    let script = Script {
        ehlo: auth_ehlo(),
        auth_flow: vec![Reply::line("235 2.7.0 Authentication successful")],
        ..Script::default()
    };
    let server = serving(script);
    let mut session = Session::open(&config(server.addr(), SmtpSecurity::ImplicitTls))
        .await
        .expect("open");

    session
        .authenticate("relay-user", "hunter2")
        .await
        .expect("authenticated");
    assert_eq!(server.trace().count_prefix("AUTH"), 1);
}

#[tokio::test]
async fn authenticates_with_login_challenges() {
    let script = Script {
        ehlo: Reply::lines(&["250-sendust.test", "250-AUTH LOGIN", "250 8BITMIME"]),
        auth_flow: vec![
            Reply::line("334 VXNlcm5hbWU6"),
            Reply::line("334 UGFzc3dvcmQ6"),
            Reply::line("235 2.7.0 Authentication successful"),
        ],
        ..Script::default()
    };
    let server = serving(script);
    let mut session = Session::open(&config(server.addr(), SmtpSecurity::ImplicitTls))
        .await
        .expect("open");

    session
        .authenticate("relay-user", "hunter2")
        .await
        .expect("authenticated");
    assert_eq!(server.trace().count_prefix("AUTH"), 1);
}

#[tokio::test]
async fn auth_rejection_is_a_stable_error() {
    let script = Script {
        ehlo: auth_ehlo(),
        auth_flow: vec![Reply::line("535 5.7.8 Bad credentials")],
        ..Script::default()
    };
    let server = serving(script);
    let mut session = Session::open(&config(server.addr(), SmtpSecurity::ImplicitTls))
        .await
        .expect("open");

    let error = session
        .authenticate("relay-user", "hunter2")
        .await
        .expect_err("rejected");
    assert_eq!(error.code(), ErrorCode::AuthFailed);
    assert_eq!(error.details().smtp_status(), Some(535));
    assert!(!error.to_string().contains("hunter2"));
}

#[tokio::test]
async fn auth_without_a_mechanism_is_unsupported() {
    let server = serving(Script::default());
    let mut session = Session::open(&config(server.addr(), SmtpSecurity::ImplicitTls))
        .await
        .expect("open");

    let error = session
        .authenticate("relay-user", "hunter2")
        .await
        .expect_err("no mechanism");
    assert_eq!(error.code(), ErrorCode::AuthUnsupported);
}

#[tokio::test]
async fn auth_before_tls_is_refused() {
    let script = Script {
        ehlo: auth_ehlo(),
        ..Script::default()
    };
    let server = serving_starttls(script);
    let mut session = Session::open(&config(server.addr(), SmtpSecurity::StartTls))
        .await
        .expect("open");
    assert!(!session.is_encrypted());

    let error = session
        .authenticate("relay-user", "hunter2")
        .await
        .expect_err("must refuse");
    assert_eq!(error.code(), ErrorCode::AuthUnsupported);
    assert_eq!(server.trace().count_prefix("AUTH"), 0);
}

#[tokio::test]
async fn runs_a_full_transaction_with_dot_stuffing() {
    let server = serving(Script::default());
    let mut session = Session::open(&config(server.addr(), SmtpSecurity::ImplicitTls))
        .await
        .expect("open");

    let reply = session.mail_from("alice@example.test").await.expect("mail");
    assert_eq!(reply.value(), 250);

    let reply = session.rcpt_to("bob@example.test").await.expect("rcpt");
    assert_eq!(reply.value(), 250);

    let reply = session.data_command().await.expect("data");
    assert_eq!(reply.value(), 354);

    let reply = session
        .data_body(b"Subject: x\r\n\r\n.hidden\r\n")
        .await
        .expect("body");
    assert_eq!(reply.value(), 250);
    assert!(server.trace().body_text().contains("..hidden"));

    let reply = session.quit().await.expect("quit");
    assert_eq!(reply.value(), 221);
}
