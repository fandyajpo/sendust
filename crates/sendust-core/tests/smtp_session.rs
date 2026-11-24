//! Session and AUTH tests against the loopback harness (P06-003).

mod support;

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use sendust_core::__internal::Session;
use sendust_core::{AuthMechanism, ErrorCode, SmtpConfig, SmtpSecurity};
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

fn xoauth2_ehlo() -> Reply {
    Reply::lines(&["250-sendust.test", "250-AUTH XOAUTH2", "250 8BITMIME"])
}

/// Decodes the base64 payload of the single AUTH command in the trace.
fn auth_payload(server: &FakeSmtp) -> String {
    use base64::Engine as _;
    let line = server
        .trace()
        .lines()
        .into_iter()
        .find(|line| line.to_ascii_uppercase().starts_with("AUTH"))
        .expect("an AUTH command");
    let encoded = line.split_whitespace().nth(2).expect("initial response");
    String::from_utf8(
        base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .expect("base64"),
    )
    .expect("utf8")
}

#[tokio::test]
async fn authenticates_with_xoauth2() {
    let script = Script {
        ehlo: xoauth2_ehlo(),
        auth_flow: vec![Reply::line("235 2.7.0 Authentication successful")],
        ..Script::default()
    };
    let server = serving(script);
    let mut session = Session::open(&config(server.addr(), SmtpSecurity::ImplicitTls))
        .await
        .expect("open");

    session
        .authenticate(
            "sender@example.test",
            "ya29.a0Af.token",
            Some(AuthMechanism::XOAuth2),
        )
        .await
        .expect("authenticated");

    assert_eq!(server.trace().count_prefix("AUTH"), 1);
    assert_eq!(
        auth_payload(&server),
        "user=sender@example.test\u{1}auth=Bearer ya29.a0Af.token\u{1}\u{1}"
    );
}

#[tokio::test]
async fn authenticates_with_a_long_xoauth2_token() {
    // A Microsoft 365 or Graph access token is an 800-2,000+ character JWT, so the
    // base64 payload is far past the 512-octet command line RFC 5321 caps. Every
    // other AUTH case here uses a short token, which is why the cap went unnoticed.
    let script = Script {
        ehlo: xoauth2_ehlo(),
        auth_flow: vec![Reply::line("235 2.7.0 Authentication successful")],
        ..Script::default()
    };
    let server = serving(script);
    let mut session = Session::open(&config(server.addr(), SmtpSecurity::ImplicitTls))
        .await
        .expect("open");

    let token = format!("ya29.{}", "a".repeat(1_400));
    session
        .authenticate(
            "sender@example.test",
            &token,
            Some(AuthMechanism::XOAuth2),
        )
        .await
        .expect("a long bearer token authenticates");

    assert_eq!(server.trace().count_prefix("AUTH"), 1);
    let payload = auth_payload(&server);
    assert!(payload.starts_with("user=sender@example.test\u{1}auth=Bearer ya29."));
    assert!(payload.contains(&token), "the whole token must reach the relay");
}

#[tokio::test]
async fn an_xoauth2_error_challenge_gets_the_final_failure() {
    let script = Script {
        ehlo: xoauth2_ehlo(),
        auth_flow: vec![
            Reply::line("334 eyJzdGF0dXMiOiI0MDEifQ=="),
            Reply::line("535 5.7.8 Username and Password not accepted"),
        ],
        ..Script::default()
    };
    let server = serving(script);
    let mut session = Session::open(&config(server.addr(), SmtpSecurity::ImplicitTls))
        .await
        .expect("open");

    let error = session
        .authenticate(
            "sender@example.test",
            "expired-token",
            Some(AuthMechanism::XOAuth2),
        )
        .await
        .expect_err("rejected");

    assert_eq!(error.code(), ErrorCode::AuthFailed);
    assert_eq!(error.details().smtp_status(), Some(535));
    assert!(!error.to_string().contains("expired-token"));
    assert_eq!(server.trace().count_prefix("AUTH"), 1);
}

#[tokio::test]
async fn xoauth2_that_is_not_advertised_is_unsupported() {
    let script = Script {
        ehlo: auth_ehlo(),
        ..Script::default()
    };
    let server = serving(script);
    let mut session = Session::open(&config(server.addr(), SmtpSecurity::ImplicitTls))
        .await
        .expect("open");

    let error = session
        .authenticate(
            "sender@example.test",
            "ya29.a0Af.token",
            Some(AuthMechanism::XOAuth2),
        )
        .await
        .expect_err("not advertised");
    assert_eq!(error.code(), ErrorCode::AuthUnsupported);
    assert_eq!(server.trace().count_prefix("AUTH"), 0);
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
        .authenticate("relay-user", "hunter2", None)
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
        .authenticate("relay-user", "hunter2", None)
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
        .authenticate("relay-user", "hunter2", None)
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
        .authenticate("relay-user", "hunter2", None)
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
        .authenticate("relay-user", "hunter2", None)
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
