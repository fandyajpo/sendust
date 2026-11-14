//! Dev-only differential oracle: the native SMTP adapter against lettre (P06-004).
//!
//! The native engine must reproduce the relay-level behaviour the fault matrix
//! already proved against lettre. lettre is a dev-dependency only: no `src/`
//! module may reference it and it is absent from the production graph.
//!
//! Scope is implicit TLS without AUTH; the authentication matrix lives in
//! `smtp_session.rs` and `smtp_faults.rs`.

mod support;

use std::future::Future;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use lettre::Address;
use lettre::transport::smtp::Error as LettreError;
use lettre::transport::smtp::client::{
    AsyncSmtpConnection, Certificate, CertificateStore, TlsParameters,
};
use lettre::transport::smtp::commands::{Data, Mail, Rcpt};
use lettre::transport::smtp::extension::ClientId;
use lettre::transport::smtp::response::Response;

use sendust_core::{
    AmbiguityStage, Delivery, Limits, Mailbox, OutgoingMessage, RecipientOutcome, RecipientStatus,
    SendOutcome, SmtpConfig, SmtpResponse, SmtpSecurity, SmtpTransport,
};
use support::tls;
use support::{AfterBody, FakeSmtp, Reply, Script, Trace};

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

fn verbs(trace: &Trace) -> Vec<String> {
    trace
        .lines()
        .iter()
        .map(|line| {
            line.split(' ')
                .next()
                .unwrap_or_default()
                .to_ascii_uppercase()
        })
        .collect()
}

/// One matrix case: both backends must agree on the outcome and the envelope
/// command sequence, not just on the final status.
async fn compare(script: Script, to: &[&str]) {
    let native_server = serving(script.clone());
    let mut native_delivery = Delivery::new(
        SmtpTransport::new(config(native_server.addr())),
        Limits::default(),
        false,
    );
    let native = native_delivery
        .send(message(to))
        .await
        .expect("native send");
    let native_trace = verbs(native_server.trace());

    let oracle_server = serving(script);
    let oracle_config = config(oracle_server.addr());
    let oracle = lettre_send(&oracle_config, &message(to))
        .await
        .expect("lettre oracle send");
    let oracle_trace = verbs(oracle_server.trace());

    assert_eq!(native.status(), oracle.status());
    assert_eq!(native.recipients(), oracle.recipients());
    assert_eq!(native_trace, oracle_trace);
}

#[tokio::test]
async fn success_matches_the_lettre_oracle() {
    compare(
        Script::default(),
        &["bob@example.test", "carol@example.test"],
    )
    .await;
}

#[tokio::test]
async fn all_recipients_refused_matches_the_lettre_oracle() {
    compare(
        Script::default().rcpt_reply(Reply::line("550 5.1.1 No such user")),
        &["one@example.test", "two@example.test"],
    )
    .await;
}

#[tokio::test]
async fn mixed_recipients_match_the_lettre_oracle() {
    compare(
        Script::default().rcpt_replies(vec![
            Reply::line("250 2.1.5 Recipient ok"),
            Reply::line("550 5.1.1 No such user"),
        ]),
        &["ok@example.test", "missing@example.test"],
    )
    .await;
}

#[tokio::test]
async fn channel_close_at_rcpt_matches_the_lettre_oracle() {
    compare(
        Script::default().rcpt_replies(vec![
            Reply::line("250 2.1.5 Recipient ok"),
            Reply::line("421 4.4.1 Closing connection"),
        ]),
        &[
            "ok@example.test",
            "two@example.test",
            "three@example.test",
        ],
    )
    .await;
}

#[tokio::test]
async fn explicit_final_rejection_matches_the_lettre_oracle() {
    let script = Script::default()
        .after_body(AfterBody::Reply(Reply::line("451 4.3.0 Try again later")));
    compare(script, &["bob@example.test"]).await;
}

#[tokio::test]
async fn missing_final_reply_matches_the_lettre_oracle() {
    compare(
        Script::default().after_body(AfterBody::Close),
        &["bob@example.test"],
    )
    .await;
}

/// One lettre step that failed, with the relay code when the relay stated one.
struct Failure {
    status: Option<u16>,
    message: String,
}

async fn bounded<F, T>(budget: Duration, future: F) -> Result<T, Failure>
where
    F: Future<Output = Result<T, LettreError>>,
{
    match tokio::time::timeout(budget, future).await {
        Err(_) => Err(Failure {
            status: None,
            message: "timeout".to_string(),
        }),
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => Err(Failure {
            status: error.status().map(u16::from),
            message: error.to_string(),
        }),
    }
}

/// The pre-P06-004 delivery path, kept as the differential oracle.
async fn lettre_send(config: &SmtpConfig, message: &OutgoingMessage) -> Result<SendOutcome, String> {
    let wire = sendust_core::serialize(message, Limits::default(), false)
        .map_err(|error| error.to_string())?;
    let from = message
        .from
        .address()
        .parse::<Address>()
        .map_err(|error| error.to_string())?;
    let recipients: Vec<String> = message
        .to
        .iter()
        .chain(&message.cc)
        .chain(&message.bcc)
        .map(Mailbox::address)
        .collect();

    let mut builder = TlsParameters::builder(config.host().to_string())
        .certificate_store(CertificateStore::WebpkiRoots);
    for pem in config.root_certificates() {
        let certificate = Certificate::from_pem(pem).map_err(|error| error.to_string())?;
        builder = builder.add_root_certificate(certificate);
    }
    let tls = builder.build().map_err(|error| error.to_string())?;

    let hello = ClientId::Domain(config.hello_name().to_string());
    let mut connection = AsyncSmtpConnection::connect_tokio1(
        (config.host(), config.port()),
        Some(config.timeouts().connect()),
        &hello,
        Some(tls),
        None,
    )
    .await
    .map_err(|error| error.to_string())?;

    let mail = Mail::new(Some(from), Vec::new());
    match bounded(config.timeouts().operation(), connection.command(mail)).await {
        Ok(response) => {
            let code = u16::from(response.code());
            if !is_completion(code) {
                connection.abort().await;
                return Err(format!("unexpected MAIL response {code}"));
            }
        }
        Err(failure) => {
            connection.abort().await;
            return match failure.status {
                Some(code) => Ok(rejected_all(&recipients, code, failure.message)),
                None => Err(failure.message),
            };
        }
    }

    let mut statuses = vec![RecipientStatus::Rejected; recipients.len()];
    let mut accepted: Vec<usize> = Vec::new();
    let mut closing = false;

    for (index, recipient) in recipients.iter().enumerate() {
        if closing {
            break;
        }
        let address = recipient
            .parse::<Address>()
            .map_err(|error| error.to_string())?;
        let rcpt = Rcpt::new(address, Vec::new());
        match bounded(config.timeouts().operation(), connection.command(rcpt)).await {
            Ok(response) => {
                let code = u16::from(response.code());
                if is_completion(code) {
                    statuses[index] = RecipientStatus::Unknown;
                    accepted.push(index);
                } else {
                    connection.abort().await;
                    return Err(format!("unexpected RCPT response {code}"));
                }
            }
            Err(failure) => match failure.status {
                Some(421) => closing = true,
                Some(_) => {}
                None => {
                    connection.abort().await;
                    return Err(failure.message);
                }
            },
        }
    }

    if closing || accepted.is_empty() {
        connection.abort().await;
        return Ok(rejected_outcome(&recipients, &statuses, None));
    }

    match bounded(config.timeouts().operation(), connection.command(Data)).await {
        Ok(response) => {
            let code = u16::from(response.code());
            if code != 354 {
                connection.abort().await;
                return Err(format!("unexpected DATA response {code}"));
            }
        }
        Err(failure) => {
            connection.abort().await;
            return match failure.status {
                Some(code) => Ok(rejected_all(&recipients, code, failure.message)),
                None => Err(failure.message),
            };
        }
    }

    match bounded(config.timeouts().operation(), connection.message(&wire)).await {
        Ok(response) => {
            let code = u16::from(response.code());
            if is_completion(code) {
                let _ =
                    tokio::time::timeout(config.timeouts().operation(), connection.quit()).await;
                settle(&mut statuses, &accepted, RecipientStatus::Accepted);
                Ok(SendOutcome::Accepted {
                    recipients: outcomes(&recipients, &statuses),
                    response: Some(SmtpResponse {
                        code,
                        message: text_of(&response),
                    }),
                })
            } else {
                connection.abort().await;
                settle(&mut statuses, &accepted, RecipientStatus::Unknown);
                Ok(unknown(&recipients, &statuses))
            }
        }
        Err(failure) => match failure.status {
            Some(code) => {
                let _ =
                    tokio::time::timeout(config.timeouts().operation(), connection.quit()).await;
                settle(&mut statuses, &accepted, RecipientStatus::Rejected);
                Ok(rejected_outcome(
                    &recipients,
                    &statuses,
                    Some(SmtpResponse {
                        code,
                        message: failure.message,
                    }),
                ))
            }
            None => {
                connection.abort().await;
                settle(&mut statuses, &accepted, RecipientStatus::Unknown);
                Ok(unknown(&recipients, &statuses))
            }
        },
    }
}

fn is_completion(code: u16) -> bool {
    (200..300).contains(&code)
}

fn text_of(response: &Response) -> String {
    response.message().collect::<Vec<_>>().join(" ")
}

fn outcomes(recipients: &[String], statuses: &[RecipientStatus]) -> Vec<RecipientOutcome> {
    recipients
        .iter()
        .zip(statuses)
        .map(|(address, status)| RecipientOutcome::new(address.clone(), *status))
        .collect()
}

fn settle(statuses: &mut [RecipientStatus], accepted: &[usize], status: RecipientStatus) {
    for index in accepted {
        if let Some(slot) = statuses.get_mut(*index) {
            *slot = status;
        }
    }
}

fn rejected_all(recipients: &[String], code: u16, message: String) -> SendOutcome {
    SendOutcome::Rejected {
        recipients: recipients
            .iter()
            .map(|address| RecipientOutcome::new(address.clone(), RecipientStatus::Rejected))
            .collect(),
        response: Some(SmtpResponse { code, message }),
    }
}

fn rejected_outcome(
    recipients: &[String],
    statuses: &[RecipientStatus],
    response: Option<SmtpResponse>,
) -> SendOutcome {
    SendOutcome::Rejected {
        recipients: outcomes(recipients, statuses),
        response,
    }
}

fn unknown(recipients: &[String], statuses: &[RecipientStatus]) -> SendOutcome {
    SendOutcome::Unknown {
        recipients: outcomes(recipients, statuses),
        stage: AmbiguityStage::FinalResponse,
    }
}
