//! SMTP session, TLS, AUTH, envelope, RCPT/DATA, and response mapping via lettre.
//!
//! The adapter owns every lettre type: domain and application modules never see
//! an SMTP session. It implements the narrow [`DeliveryPort`] once, submits one
//! message per call, never retries, and never reuses a connection.
//!
//! Recipient handling follows ADR-0005. Every `RCPT TO` reply is observed
//! individually, only the recipients the relay accepted take part in the single
//! `DATA` transaction, and a recipient is reported `accepted` only after a
//! positive final `DATA` response. A refusal at `RCPT` is a recipient outcome,
//! not an error, and nothing is ever fanned out into per-recipient messages.

pub mod client;
pub mod proto;

use std::fmt;
use std::future::Future;
use std::time::{Duration, Instant};

use lettre::Address;
use lettre::transport::smtp::Error as LettreError;
use lettre::transport::smtp::authentication::{Credentials, Mechanism};
use lettre::transport::smtp::client::{
    AsyncSmtpConnection, Certificate, CertificateStore, TlsParameters,
};
use lettre::transport::smtp::commands::{Data, Mail, Rcpt};
use lettre::transport::smtp::extension::ClientId;
use lettre::transport::smtp::response::Response;

use crate::config::{SendDeadlines, Timeouts};
use crate::errors::{Error, ErrorCode, ErrorDetails, Stage};
use crate::message::OutgoingMessage;
use crate::ports::{DeliveryFuture, DeliveryPort, DeliveryRequest};
use crate::results::{
    AmbiguityStage, RecipientOutcome, RecipientStatus, SendOutcome, SmtpResponse,
};

/// EHLO name used when the caller does not configure one.
const DEFAULT_HELLO_NAME: &str = "localhost";

/// How the session is encrypted. Opportunistic plaintext fallback is never
/// allowed: a configured STARTTLS that the relay does not advertise fails the
/// send instead of downgrading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmtpSecurity {
    /// TLS from the first byte, usually port 465.
    ImplicitTls,
    /// Plaintext greeting followed by a required STARTTLS upgrade, usually
    /// port 587.
    StartTls,
}

/// Relay credentials. The values are never rendered, logged, or placed in an
/// error; only the adapter converts them into lettre credentials.
#[derive(Clone, PartialEq, Eq)]
pub struct SmtpCredentials {
    username: String,
    password: String,
}

impl SmtpCredentials {
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Result<Self, Error> {
        let username = username.into();
        let password = password.into();
        if username.is_empty()
            || password.is_empty()
            || username.chars().any(char::is_control)
            || password.chars().any(char::is_control)
        {
            return Err(Error::new(ErrorCode::ValidationInvalidOption));
        }
        Ok(Self { username, password })
    }

    /// Username, safe to display because it is not the secret half.
    pub fn username(&self) -> &str {
        &self.username
    }

    fn to_lettre(&self) -> Credentials {
        Credentials::new(self.username.clone(), self.password.clone())
    }
}

impl fmt::Debug for SmtpCredentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SmtpCredentials")
            .field("username", &"<redacted>")
            .field("password", &"<redacted>")
            .finish()
    }
}

/// Relay endpoint and session policy. Built once per transport; it carries no
/// connection and no runtime handle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmtpConfig {
    host: String,
    port: u16,
    security: SmtpSecurity,
    credentials: Option<SmtpCredentials>,
    hello_name: String,
    timeouts: Timeouts,
    root_certificates: Vec<Vec<u8>>,
}

impl SmtpConfig {
    pub fn new(host: impl Into<String>, port: u16, security: SmtpSecurity) -> Result<Self, Error> {
        let host = host.into();
        if host.trim().is_empty() || host.chars().any(char::is_control) || port == 0 {
            return Err(Error::new(ErrorCode::ConfigInvalidRelay)
                .with_details(ErrorDetails::default().host(host).port(port)));
        }
        Ok(Self {
            host,
            port,
            security,
            credentials: None,
            hello_name: DEFAULT_HELLO_NAME.to_string(),
            timeouts: Timeouts::default(),
            root_certificates: Vec::new(),
        })
    }

    pub fn with_credentials(mut self, credentials: SmtpCredentials) -> Self {
        self.credentials = Some(credentials);
        self
    }

    pub fn with_hello_name(mut self, hello_name: impl Into<String>) -> Result<Self, Error> {
        let hello_name = hello_name.into();
        if hello_name.trim().is_empty() || hello_name.chars().any(char::is_control) {
            return Err(Error::new(ErrorCode::ValidationInvalidOption));
        }
        self.hello_name = hello_name;
        Ok(self)
    }

    /// Replaces the budgets. They are validated against the approved caps.
    pub fn with_timeouts(mut self, timeouts: Timeouts) -> Result<Self, Error> {
        timeouts.validate()?;
        self.timeouts = timeouts;
        Ok(self)
    }

    /// Adds a PEM trust anchor on top of the bundled WebPKI roots.
    ///
    /// Verification is never disabled; a send fails closed when the relay
    /// certificate is not trusted.
    pub fn with_root_certificate_pem(mut self, pem: impl Into<Vec<u8>>) -> Result<Self, Error> {
        let pem = pem.into();
        client::tls::validate_anchor(&pem)
            .map_err(|_| Error::new(ErrorCode::ValidationInvalidOption))?;
        self.root_certificates.push(pem);
        Ok(self)
    }

    /// Extra PEM trust anchors supplied by the caller.
    pub fn root_certificates(&self) -> &[Vec<u8>] {
        &self.root_certificates
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn security(&self) -> SmtpSecurity {
        self.security
    }

    pub fn credentials(&self) -> Option<&SmtpCredentials> {
        self.credentials.as_ref()
    }

    pub fn hello_name(&self) -> &str {
        &self.hello_name
    }

    pub fn timeouts(&self) -> Timeouts {
        self.timeouts
    }

    fn tls_parameters(&self) -> Result<TlsParameters, Error> {
        let mut builder = TlsParameters::builder(self.host.clone())
            .certificate_store(CertificateStore::WebpkiRoots);

        for pem in &self.root_certificates {
            let certificate = Certificate::from_pem(pem)
                .map_err(|_| Error::new(ErrorCode::ValidationInvalidOption))?;
            builder = builder.add_root_certificate(certificate);
        }

        builder.build().map_err(|_| {
            Error::new(ErrorCode::TlsHandshake).with_details(
                ErrorDetails::default()
                    .host(self.host.clone())
                    .port(self.port),
            )
        })
    }
}

/// The single MVP delivery adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmtpTransport {
    config: SmtpConfig,
}

impl SmtpTransport {
    pub fn new(config: SmtpConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &SmtpConfig {
        &self.config
    }

    /// Runs exactly one delivery over this transport.
    ///
    /// The caller keeps ownership, which is what the FFI layer needs to reuse a
    /// single transport across `send()` calls.
    pub async fn run(&mut self, request: DeliveryRequest) -> Result<SendOutcome, Error> {
        let config = &self.config;
        let deadlines = SendDeadlines::start(config.timeouts, Instant::now());
        let envelope = Envelope::of(&request.message)?;

        let tls = match config.security {
            SmtpSecurity::ImplicitTls => Some(config.tls_parameters()?),
            SmtpSecurity::StartTls => None,
        };
        let hello = ClientId::Domain(config.hello_name.clone());

        let connect_budget = config
            .timeouts
            .connect()
            .min(deadlines.remaining_total(Instant::now()));

        let mut connection = match tokio::time::timeout(
            connect_budget,
            AsyncSmtpConnection::connect_tokio1(
                (config.host.as_str(), config.port),
                Some(config.timeouts.connect()),
                &hello,
                tls,
                None,
            ),
        )
        .await
        {
            Err(_) => return Err(connect_timeout(config, deadlines)),
            Ok(Ok(connection)) => connection,
            Ok(Err(error)) => {
                return Err(session_error(
                    &error,
                    Stage::ConnectionSetup,
                    config,
                    deadlines,
                ));
            }
        };

        if config.security == SmtpSecurity::StartTls {
            let parameters = config.tls_parameters()?;
            if !connection.can_starttls() {
                connection.abort().await;
                return Err(Error::new(ErrorCode::TlsHandshake).with_details(
                    ErrorDetails::default()
                        .host(config.host.clone())
                        .port(config.port)
                        .stage(Stage::ConnectionSetup),
                ));
            }
            match bounded(
                deadlines,
                Stage::ConnectionSetup,
                connection.starttls(parameters, &hello),
            )
            .await
            {
                Err(failure) => {
                    connection.abort().await;
                    return Err(failure);
                }
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    connection.abort().await;
                    return Err(session_error(
                        &error,
                        Stage::ConnectionSetup,
                        config,
                        deadlines,
                    ));
                }
            }
        }

        if let Some(credentials) = config.credentials.as_ref() {
            let mechanisms = [Mechanism::Plain, Mechanism::Login];
            let lettre_credentials = credentials.to_lettre();

            match bounded(
                deadlines,
                Stage::Auth,
                connection.auth(&mechanisms, &lettre_credentials),
            )
            .await?
            {
                Ok(response) => {
                    let code = u16::from(response.code());
                    if code != 235 {
                        connection.abort().await;
                        return Err(auth_error(ErrorCode::AuthFailed, Some(code), config));
                    }
                }
                Err(error) => {
                    let code = status_of(&error);
                    let failure = if error.is_client() && code.is_none() {
                        ErrorCode::AuthUnsupported
                    } else {
                        ErrorCode::AuthFailed
                    };
                    return Err(auth_error(failure, code, config));
                }
            }
        }

        let mail = Mail::new(Some(envelope.from.clone()), Vec::new());
        match bounded(deadlines, Stage::Mail, connection.command(mail)).await? {
            Ok(response) => {
                let code = u16::from(response.code());
                if !is_completion(code) {
                    connection.abort().await;
                    return Err(unexpected_response(config, Stage::Mail, code));
                }
            }
            Err(error) => {
                connection.abort().await;
                return match status_of(&error) {
                    Some(code) => Ok(rejected_outcome(
                        &envelope,
                        &vec![RecipientStatus::Rejected; envelope.recipients.len()],
                        Some(response_of(code, error.to_string())),
                    )),
                    None => Err(session_error(&error, Stage::Mail, config, deadlines)),
                };
            }
        }

        let mut statuses = vec![RecipientStatus::Rejected; envelope.recipients.len()];
        let mut accepted: Vec<usize> = Vec::new();
        let mut session_closing = false;

        for (index, (_, recipient)) in envelope.recipients.iter().enumerate() {
            if session_closing {
                break;
            }
            let rcpt = Rcpt::new(recipient.clone(), Vec::new());
            match bounded(deadlines, Stage::Rcpt, connection.command(rcpt)).await? {
                Ok(response) => {
                    let code = u16::from(response.code());
                    if is_completion(code) {
                        statuses[index] = RecipientStatus::Unknown;
                        accepted.push(index);
                    } else {
                        connection.abort().await;
                        return Err(unexpected_response(config, Stage::Rcpt, code));
                    }
                }
                Err(error) => match status_of(&error) {
                    Some(421) => {
                        // A 421 closes the transmission channel: stop the
                        // envelope and never issue DATA.
                        statuses[index] = RecipientStatus::Rejected;
                        session_closing = true;
                    }
                    Some(_) => statuses[index] = RecipientStatus::Rejected,
                    None => {
                        connection.abort().await;
                        return Err(session_error(&error, Stage::Rcpt, config, deadlines));
                    }
                },
            }
        }

        if session_closing {
            connection.abort().await;
            return Ok(rejected_outcome(&envelope, &statuses, None));
        }

        if accepted.is_empty() {
            connection.abort().await;
            return Ok(rejected_outcome(&envelope, &statuses, None));
        }

        match bounded(deadlines, Stage::DataCommand, connection.command(Data)).await? {
            Ok(response) => {
                let code = u16::from(response.code());
                if code != 354 {
                    connection.abort().await;
                    return Err(unexpected_response(config, Stage::DataCommand, code));
                }
            }
            Err(error) => {
                connection.abort().await;
                return match status_of(&error) {
                    Some(code) => Ok(rejected_outcome(
                        &envelope,
                        &vec![RecipientStatus::Rejected; envelope.recipients.len()],
                        Some(response_of(code, error.to_string())),
                    )),
                    None => Err(session_error(&error, Stage::DataCommand, config, deadlines)),
                };
            }
        }

        let now = Instant::now();
        let body_budget = deadlines
            .operation_deadline(now)
            .saturating_duration_since(now)
            .min(deadlines.remaining_total(now));

        let transmitted = if body_budget.is_zero() {
            None
        } else {
            Some(tokio::time::timeout(body_budget, connection.message(&request.wire)).await)
        };

        match transmitted {
            None | Some(Err(_)) => {
                connection.abort().await;
                settle(&mut statuses, &accepted, RecipientStatus::Unknown);
                Ok(SendOutcome::Unknown {
                    recipients: outcomes(&envelope, &statuses),
                    stage: AmbiguityStage::FinalResponse,
                })
            }
            Some(Ok(Ok(response))) => {
                let code = u16::from(response.code());
                if is_completion(code) {
                    let _ =
                        tokio::time::timeout(config.timeouts.operation(), connection.quit()).await;
                    settle(&mut statuses, &accepted, RecipientStatus::Accepted);
                    Ok(SendOutcome::Accepted {
                        recipients: outcomes(&envelope, &statuses),
                        response: Some(response_of(code, text_of(&response))),
                    })
                } else {
                    connection.abort().await;
                    settle(&mut statuses, &accepted, RecipientStatus::Unknown);
                    Ok(SendOutcome::Unknown {
                        recipients: outcomes(&envelope, &statuses),
                        stage: AmbiguityStage::FinalResponse,
                    })
                }
            }
            Some(Ok(Err(error))) => match status_of(&error) {
                Some(code) => {
                    let _ =
                        tokio::time::timeout(config.timeouts.operation(), connection.quit()).await;
                    settle(&mut statuses, &accepted, RecipientStatus::Rejected);
                    Ok(rejected_outcome(
                        &envelope,
                        &statuses,
                        Some(response_of(code, error.to_string())),
                    ))
                }
                None => {
                    connection.abort().await;
                    settle(&mut statuses, &accepted, RecipientStatus::Unknown);
                    Ok(SendOutcome::Unknown {
                        recipients: outcomes(&envelope, &statuses),
                        stage: AmbiguityStage::FinalResponse,
                    })
                }
            },
        }
    }
}

impl DeliveryPort for SmtpTransport {
    fn deliver<'a>(&'a mut self, request: DeliveryRequest) -> DeliveryFuture<'a> {
        Box::pin(self.run(request))
    }
}

/// Borrowed handle that submits one message without giving up the transport.
pub struct SmtpSession<'a> {
    transport: &'a mut SmtpTransport,
}

impl DeliveryPort for SmtpSession<'_> {
    fn deliver<'a>(&'a mut self, request: DeliveryRequest) -> DeliveryFuture<'a> {
        Box::pin(self.transport.run(request))
    }
}

impl SmtpTransport {
    /// Borrows this transport for one delivery.
    pub fn session(&mut self) -> SmtpSession<'_> {
        SmtpSession { transport: self }
    }
}

/// The envelope addresses, parsed into lettre addresses exactly once.
struct Envelope {
    from: Address,
    recipients: Vec<(String, Address)>,
}

impl Envelope {
    fn of(message: &OutgoingMessage) -> Result<Self, Error> {
        let from = parse_address(&message.from.address())?;
        let mut recipients = Vec::with_capacity(message.recipient_count());
        for mailbox in message.to.iter().chain(&message.cc).chain(&message.bcc) {
            let address = mailbox.address();
            let parsed = parse_address(&address)?;
            recipients.push((address, parsed));
        }
        Ok(Self { from, recipients })
    }
}

fn parse_address(value: &str) -> Result<Address, Error> {
    value
        .parse::<Address>()
        .map_err(|_| Error::new(ErrorCode::ValidationInvalidAddress))
}

/// Runs one SMTP step under the operation budget capped by the total budget.
async fn bounded<F, T>(
    deadlines: SendDeadlines,
    stage: Stage,
    future: F,
) -> Result<Result<T, LettreError>, Error>
where
    F: Future<Output = Result<T, LettreError>>,
{
    let now = Instant::now();
    let budget = deadlines
        .operation_deadline(now)
        .saturating_duration_since(now)
        .min(deadlines.remaining_total(now));

    if budget.is_zero() {
        return Err(deadlines.timeout_error(now, true, stage));
    }

    match tokio::time::timeout(budget, future).await {
        Err(_) => Err(deadlines.timeout_error(Instant::now(), true, stage)),
        Ok(outcome) => Ok(outcome),
    }
}

fn is_completion(code: u16) -> bool {
    (200..300).contains(&code)
}

fn status_of(error: &LettreError) -> Option<u16> {
    error.status().map(u16::from)
}

fn text_of(response: &Response) -> String {
    response.message().collect::<Vec<_>>().join(" ")
}

fn response_of(code: u16, message: impl Into<String>) -> SmtpResponse {
    SmtpResponse {
        code,
        message: message.into(),
    }
}

fn outcomes(envelope: &Envelope, statuses: &[RecipientStatus]) -> Vec<RecipientOutcome> {
    envelope
        .recipients
        .iter()
        .zip(statuses)
        .map(|((address, _), status)| RecipientOutcome::new(address.clone(), *status))
        .collect()
}

fn settle(statuses: &mut [RecipientStatus], accepted: &[usize], status: RecipientStatus) {
    for index in accepted {
        if let Some(slot) = statuses.get_mut(*index) {
            *slot = status;
        }
    }
}

fn rejected_outcome(
    envelope: &Envelope,
    statuses: &[RecipientStatus],
    response: Option<SmtpResponse>,
) -> SendOutcome {
    SendOutcome::Rejected {
        recipients: outcomes(envelope, statuses),
        response,
    }
}

fn connect_timeout(config: &SmtpConfig, deadlines: SendDeadlines) -> Error {
    Error::new(ErrorCode::TimeoutConnect).with_details(
        ErrorDetails::default()
            .host(config.host.clone())
            .port(config.port)
            .stage(Stage::ConnectionSetup)
            .elapsed_ms(millis(deadlines.elapsed(Instant::now())))
            .timeout_ms(millis(config.timeouts.connect())),
    )
}

fn unexpected_response(config: &SmtpConfig, stage: Stage, code: u16) -> Error {
    Error::new(ErrorCode::ProtocolUnexpectedResponse).with_details(
        ErrorDetails::default()
            .host(config.host.clone())
            .port(config.port)
            .stage(stage)
            .smtp_code(code),
    )
}

fn auth_error(code: ErrorCode, smtp_code: Option<u16>, config: &SmtpConfig) -> Error {
    let mut details = ErrorDetails::default()
        .host(config.host.clone())
        .port(config.port)
        .stage(Stage::Auth);
    if let Some(smtp_code) = smtp_code {
        details = details.smtp_code(smtp_code);
    }
    Error::new(code).with_details(details)
}

/// Maps a pre-outcome session failure: TLS, timeout, or the network itself.
fn session_error(
    error: &LettreError,
    stage: Stage,
    config: &SmtpConfig,
    deadlines: SendDeadlines,
) -> Error {
    // Implicit-TLS handshake failures arrive from lettre as connection errors,
    // so the typed rustls cause decides before the generic fallbacks do.
    let code = if error.is_timeout() {
        ErrorCode::TimeoutOperation
    } else if let Some(tls) = tls_code(error) {
        tls
    } else if error.is_tls() {
        ErrorCode::TlsHandshake
    } else {
        ErrorCode::ProtocolConnectionReset
    };

    Error::new(code).with_details(
        ErrorDetails::default()
            .host(config.host.clone())
            .port(config.port)
            .stage(stage)
            .elapsed_ms(millis(deadlines.elapsed(Instant::now()))),
    )
}

/// Certificate failures and negotiation failures are distinct in the public
/// contract, so the rustls error hidden in the source chain is inspected rather
/// than guessed from a message string.
fn tls_code(error: &LettreError) -> Option<ErrorCode> {
    let mut source = std::error::Error::source(error);
    while let Some(current) = source {
        if let Some(code) = classify_rustls(current) {
            return Some(code);
        }
        // rustls causes reach the chain inside an `io::Error` payload.
        if let Some(io_error) = current.downcast_ref::<std::io::Error>()
            && let Some(inner) = io_error.get_ref()
            && let Some(code) = classify_rustls(inner)
        {
            return Some(code);
        }
        source = current.source();
    }
    None
}

fn classify_rustls(error: &(dyn std::error::Error + 'static)) -> Option<ErrorCode> {
    error
        .downcast_ref::<rustls::Error>()
        .map(|error| match error {
            rustls::Error::InvalidCertificate(_) => ErrorCode::TlsCertificate,
            _ => ErrorCode::TlsHandshake,
        })
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_are_redacted_and_validated() {
        let credentials = SmtpCredentials::new("relay-user", "hunter2").expect("valid");
        let rendered = format!("{credentials:?}");
        assert!(!rendered.contains("hunter2"));
        assert!(!rendered.contains("relay-user"));

        assert_eq!(
            SmtpCredentials::new("", "secret")
                .expect_err("empty username")
                .code(),
            ErrorCode::ValidationInvalidOption
        );
        assert_eq!(
            SmtpCredentials::new("user", "")
                .expect_err("empty password")
                .code(),
            ErrorCode::ValidationInvalidOption
        );
    }

    #[test]
    fn relay_configuration_is_validated() {
        assert_eq!(
            SmtpConfig::new("", 587, SmtpSecurity::StartTls)
                .expect_err("empty host")
                .code(),
            ErrorCode::ConfigInvalidRelay
        );
        assert_eq!(
            SmtpConfig::new("relay.example.test", 0, SmtpSecurity::StartTls)
                .expect_err("zero port")
                .code(),
            ErrorCode::ConfigInvalidRelay
        );
        assert_eq!(
            SmtpConfig::new("relay.example.test", 587, SmtpSecurity::StartTls)
                .expect("valid")
                .with_hello_name("")
                .expect_err("empty hello name")
                .code(),
            ErrorCode::ValidationInvalidOption
        );
    }

    #[test]
    fn raised_timeouts_are_rejected_by_the_configuration() {
        let timeouts = Timeouts::default()
            .with_operation(Duration::from_secs(31))
            .expect_err("raised operation budget");
        assert_eq!(timeouts.code(), ErrorCode::ValidationInvalidTimeout);
    }
}
