//! Native SMTP connection tests against the loopback harness (P06-002).

mod support;

use std::net::{SocketAddr, TcpListener};
use std::path::{Path, PathBuf};

use sendust_core::__internal::{Command, Connection};
use sendust_core::{ErrorCode, SmtpConfig, SmtpSecurity, Stage};
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

#[tokio::test]
async fn connects_over_implicit_tls_and_reads_capabilities() {
    let server = serving(Script::default());
    let mut connection = Connection::connect(&config(server.addr(), SmtpSecurity::ImplicitTls))
        .await
        .expect("connected");

    assert!(!connection.capabilities().starttls());
    assert!(connection.capabilities().eight_bit_mime());

    let reply = connection
        .command(Command::NoOp, Stage::ConnectionSetup)
        .await
        .expect("noop");
    assert_eq!(reply.value(), 250);
}

#[tokio::test]
async fn upgrades_with_starttls_and_rehandshakes() {
    let script = Script {
        ehlo: Reply::lines(&["250-sendust.test", "250-STARTTLS", "250 8BITMIME"]),
        ..Script::default()
    };
    let server = serving_starttls(script);

    let mut connection = Connection::connect(&config(server.addr(), SmtpSecurity::StartTls))
        .await
        .expect("connected");
    assert!(connection.capabilities().starttls());

    connection.starttls().await.expect("starttls upgrade");

    let reply = connection
        .command(Command::NoOp, Stage::ConnectionSetup)
        .await
        .expect("noop over tls");
    assert_eq!(reply.value(), 250);
    assert_eq!(server.trace().count_prefix("EHLO"), 2);
}

#[tokio::test]
async fn refuses_starttls_when_not_advertised() {
    let server = FakeSmtp::start(Script::default()).expect("plain server");
    let mut connection = Connection::connect(&config(server.addr(), SmtpSecurity::StartTls))
        .await
        .expect("connected");

    assert!(!connection.capabilities().starttls());
    let error = connection.starttls().await.expect_err("must fail closed");
    assert_eq!(error.code(), ErrorCode::TlsHandshake);
}

#[tokio::test]
async fn falls_back_to_helo_when_ehlo_is_refused() {
    // RFC 5321 §4.1.1.1: a server may refuse EHLO, and the client must then
    // try HELO. The session works, but it advertises no extensions at all.
    let script = Script {
        ehlo: Reply::line("500 5.5.1 Command unrecognized"),
        helo: Reply::line("250 sendust.test"),
        ..Script::default()
    };
    let server = serving(script);

    let connection = Connection::connect(&config(server.addr(), SmtpSecurity::ImplicitTls))
        .await
        .expect("connected with HELO");

    assert!(!connection.capabilities().starttls());
    assert!(connection.capabilities().auth_mechanisms().is_empty());
    assert_eq!(connection.capabilities().size(), None);
    assert_eq!(server.trace().count_prefix("EHLO"), 1);
    assert_eq!(server.trace().count_prefix("HELO"), 1);
}

#[tokio::test]
async fn a_transient_ehlo_reply_is_not_worked_around() {
    // A 4xx is the relay saying "later", not "I do not speak EHLO", so the
    // greeting is reported instead of retried in another form.
    let script = Script {
        ehlo: Reply::line("421 4.3.2 Service not available"),
        ..Script::default()
    };
    let server = serving(script);

    let error = Connection::connect(&config(server.addr(), SmtpSecurity::ImplicitTls))
        .await
        .expect_err("a 421 must be reported");

    assert_eq!(error.code(), ErrorCode::ProtocolUnexpectedResponse);
    assert_eq!(error.details().smtp_status(), Some(421));
    assert_eq!(server.trace().count_prefix("HELO"), 0);
}

#[tokio::test]
async fn a_refused_helo_is_the_same_protocol_error() {
    let script = Script {
        ehlo: Reply::line("502 5.5.1 Command not implemented"),
        helo: Reply::line("502 5.5.1 Command not implemented"),
        ..Script::default()
    };
    let server = serving(script);

    let error = Connection::connect(&config(server.addr(), SmtpSecurity::ImplicitTls))
        .await
        .expect_err("no greeting is understood");

    assert_eq!(error.code(), ErrorCode::ProtocolUnexpectedResponse);
    assert_eq!(error.details().smtp_status(), Some(502));
    assert_eq!(server.trace().count_prefix("EHLO"), 1);
    assert_eq!(server.trace().count_prefix("HELO"), 1);
}

#[tokio::test]
async fn a_helo_only_relay_can_never_starttls() {
    // Nothing is advertised without EHLO, so a required STARTTLS still fails
    // closed: there is no plaintext fallback to find here.
    let script = Script {
        ehlo: Reply::line("500 5.5.1 Command unrecognized"),
        helo: Reply::line("250 sendust.test"),
        ..Script::default()
    };
    let server = serving_starttls(script);

    let mut connection = Connection::connect(&config(server.addr(), SmtpSecurity::StartTls))
        .await
        .expect("connected with HELO");

    assert!(!connection.capabilities().starttls());
    let error = connection.starttls().await.expect_err("must fail closed");
    assert_eq!(error.code(), ErrorCode::TlsHandshake);
    assert_eq!(server.trace().count_prefix("STARTTLS"), 0);
}

fn config_port(port: u16, security: SmtpSecurity) -> SmtpConfig {
    SmtpConfig::new("127.0.0.1", port, security)
        .expect("valid relay")
        .with_root_certificate_pem(ca_pem())
        .expect("trusted fixture root")
}

#[tokio::test]
async fn fails_closed_for_each_bad_certificate_fixture() {
    for name in ["self-signed", "expired", "wrong-host", "incomplete-chain"] {
        let script = Script {
            tls: Some(tls::server_config_named(&tls_dir(), name)),
            ..Script::default()
        };
        let server = FakeSmtp::start(script).expect("start fake smtp");

        let error =
            match Connection::connect(&config(server.addr(), SmtpSecurity::ImplicitTls)).await {
                Ok(_) => panic!("{name} must fail closed"),
                Err(error) => error,
            };
        assert_eq!(error.code(), ErrorCode::TlsCertificate, "{name}");
    }
}

#[tokio::test]
async fn rejects_a_malformed_trust_anchor() {
    let pem = std::fs::read(tls_dir().join("malformed.pem")).expect("malformed fixture");
    let error = SmtpConfig::new("127.0.0.1", 465, SmtpSecurity::ImplicitTls)
        .expect("relay")
        .with_root_certificate_pem(pem)
        .expect_err("must reject the anchor");
    assert_eq!(error.code(), ErrorCode::ValidationInvalidOption);
}

#[tokio::test]
async fn refused_connection_is_a_protocol_error() {
    let port = {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
        listener.local_addr().expect("addr").port()
    };

    let error = match Connection::connect(&config_port(port, SmtpSecurity::ImplicitTls)).await {
        Ok(_) => panic!("must be refused"),
        Err(error) => error,
    };
    assert_eq!(error.code(), ErrorCode::ProtocolConnectionReset);
}
