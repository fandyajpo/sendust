//! SMTP delivery adapter over the native session (ADR-0017).
//!
//! The adapter owns every SMTP type: domain and application modules never see a
//! session, a reply, or a command. It implements the narrow [`DeliveryPort`]
//! once, submits one message per call, never retries, and never reuses a
//! connection.
//!
//! Recipient handling follows ADR-0005. Every `RCPT TO` reply is observed
//! individually, only the recipients the relay accepted take part in the single
//! `DATA` transaction, and a recipient is reported `accepted` only after a
//! positive final `DATA` response. A refusal at `RCPT` is a recipient outcome,
//! not an error, and nothing is ever fanned out into per-recipient messages.
//!
//! TLS is mandatory: implicit TLS is encrypted from the first byte, and
//! `StartTls` requires an advertised upgrade that is then performed. A relay
//! that does not offer STARTTLS fails the send instead of downgrading.

pub mod client;
pub mod proto;
pub mod session;

pub use self::proto::command::AuthMechanism;

use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::config::{SendDeadlines, Timeouts};
use crate::errors::{Error, ErrorCode, ErrorDetails, Stage};
use crate::message::OutgoingMessage;
use crate::ports::{DeliveryFuture, DeliveryPort, DeliveryRequest};
use crate::results::{
    AmbiguityStage, DeliveryProgress, RecipientOutcome, RecipientStatus, SendOutcome, SmtpResponse,
};
use crate::transports::smtp::proto::response::{Reply, ReplyClass};
use crate::transports::smtp::session::Session;

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
/// error; only the adapter converts them into an AUTH exchange.
#[derive(Clone, PartialEq, Eq)]
pub struct SmtpCredentials {
    username: String,
    password: String,
    mechanism: Option<AuthMechanism>,
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
        Ok(Self {
            username,
            password,
            mechanism: None,
        })
    }

    /// Forces one AUTH mechanism; an unadvertised preference fails the send
    /// instead of silently falling back to the other one.
    pub fn with_mechanism(mut self, mechanism: AuthMechanism) -> Self {
        self.mechanism = Some(mechanism);
        self
    }

    pub fn mechanism(&self) -> Option<AuthMechanism> {
        self.mechanism
    }

    /// Username, safe to display because it is not the secret half.
    pub fn username(&self) -> &str {
        &self.username
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
    reuse: ReusePolicy,
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
            reuse: ReusePolicy::default(),
        })
    }

    pub fn with_credentials(mut self, credentials: SmtpCredentials) -> Self {
        self.credentials = Some(credentials);
        self
    }

    pub fn with_hello_name(mut self, hello_name: impl Into<String>) -> Result<Self, Error> {
        let hello_name = hello_name.into();
        // EHLO takes one token: printable ASCII only, with no space, control or
        // non-ASCII character, and at most the 255 bytes the command accepts.
        if hello_name.is_empty()
            || hello_name.len() > 255
            || !hello_name.chars().all(|c| c.is_ascii_graphic())
        {
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

    /// How long one authenticated session may serve further messages.
    pub fn reuse(&self) -> ReusePolicy {
        self.reuse
    }

    /// Replaces the reuse policy (ADR-0022).
    pub fn with_reuse(mut self, reuse: ReusePolicy) -> Self {
        self.reuse = reuse;
        self
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
}

/// How long one authenticated session may serve further messages (ADR-0022).
///
/// A session is kept only after a definitive outcome and only when the relay
/// answers the `RSET` that closes the transaction; any error or an `unknown`
/// disposition drops it, so the next send dials again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReusePolicy {
    max_messages: usize,
    idle: Duration,
}

impl Default for ReusePolicy {
    fn default() -> Self {
        Self {
            max_messages: 100,
            idle: Duration::from_secs(30),
        }
    }
}

impl ReusePolicy {
    /// Messages one session may serve before it is closed and replaced.
    pub fn max_messages(&self) -> usize {
        self.max_messages
    }

    /// How long an idle session may be reused before a fresh dial.
    pub fn idle(&self) -> Duration {
        self.idle
    }

    /// At least one message per session; zero would disable reuse.
    pub fn with_max_messages(mut self, max_messages: usize) -> Self {
        self.max_messages = max_messages.max(1);
        self
    }

    pub fn with_idle(mut self, idle: Duration) -> Self {
        self.idle = idle;
        self
    }
}

/// The single MVP delivery adapter.
pub struct SmtpTransport {
    config: SmtpConfig,
    progress: Option<Arc<DeliveryProgress>>,
    /// A session kept between sends, already authenticated (ADR-0022).
    session: Option<Session>,
    /// Messages this session has served, for `ReusePolicy::max_messages`.
    served: usize,
    /// When the session last became idle, for `ReusePolicy::idle`.
    idle_since: Option<Instant>,
}

impl fmt::Debug for SmtpTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SmtpTransport")
            .field("host", &self.config.host)
            .field("port", &self.config.port)
            .field("session", &self.session.is_some())
            .field("served", &self.served)
            .finish()
    }
}

impl PartialEq for SmtpTransport {
    fn eq(&self, other: &Self) -> bool {
        self.config == other.config
    }
}

impl Eq for SmtpTransport {}

impl SmtpTransport {
    pub fn new(config: SmtpConfig) -> Self {
        Self {
            config,
            progress: None,
            session: None,
            served: 0,
            idle_since: None,
        }
    }

    /// Observes send progress, so a caller can tell whether a cancellation hit
    /// before or after message transmission began.
    pub fn with_progress(mut self, progress: Arc<DeliveryProgress>) -> Self {
        self.progress = Some(progress);
        self
    }

    fn mark(&self, stage: Stage) {
        if let Some(progress) = &self.progress {
            progress.set_stage(stage);
        }
    }

    fn reset_progress(&self) {
        if let Some(progress) = &self.progress {
            progress.reset();
        }
    }

    fn accept_recipient(&self, address: &str) {
        if let Some(progress) = &self.progress {
            progress.accept_recipient(address);
        }
    }

    pub fn config(&self) -> &SmtpConfig {
        &self.config
    }

    /// Runs exactly one delivery over this transport.
    ///
    /// The session is opened on first use and kept after a definitive outcome,
    /// so a second message pays no TCP, TLS, or AUTH round trip. Any error or an
    /// `unknown` disposition drops it, because its protocol state cannot be
    /// trusted (ADR-0022).
    ///
    /// The caller keeps ownership, which is what the FFI layer needs to reuse a
    /// single transport across `send()` calls.
    pub async fn run(&mut self, request: DeliveryRequest) -> Result<SendOutcome, Error> {
        let deadlines = SendDeadlines::start(self.config.timeouts, Instant::now());
        let envelope = Envelope::of(&request.message)?;
        self.reset_progress();

        let mut session = self.acquire_session(deadlines).await?;
        match self
            .transact(&mut session, deadlines, envelope, &request.wire)
            .await
        {
            Ok((outcome, state)) => {
                if state.reusable {
                    self.retain(session, state.needs_reset).await;
                }
                Ok(outcome)
            }
            Err(error) => Err(error),
        }
    }

    /// Closes a retained session politely. Bounded and best-effort: shutdown
    /// never reports a relay failure of its own.
    pub async fn shutdown(&mut self) {
        if let Some(mut session) = self.session.take() {
            let budget = self.config.timeouts.operation();
            let _ = tokio::time::timeout(budget, session.quit()).await;
        }
    }

    /// A warm session when one may be reused, otherwise a fresh authenticated
    /// one. An expired session is closed before the new dial, never after.
    async fn acquire_session(&mut self, deadlines: SendDeadlines) -> Result<Session, Error> {
        if let Some(mut session) = self.session.take() {
            let policy = self.config.reuse;
            let idle_expired = self
                .idle_since
                .is_some_and(|idle| idle.elapsed() >= policy.idle());
            if self.served < policy.max_messages() && !idle_expired {
                session.begin_transaction(self.config.timeouts);
                return Ok(session);
            }
            let budget = self.config.timeouts.operation();
            let _ = tokio::time::timeout(budget, session.quit()).await;
        }
        self.served = 0;
        self.idle_since = None;
        self.open_session(deadlines).await
    }

    /// A new session, with STARTTLS and AUTH when the relay configuration asks
    /// for them. `Session::open` already greets and sends EHLO.
    async fn open_session(&self, deadlines: SendDeadlines) -> Result<Session, Error> {
        let config = &self.config;
        let mut session = Session::open(config)
            .await
            .map_err(|error| annotate(error, config, deadlines, Stage::ConnectionSetup))?;

        if config.security == SmtpSecurity::StartTls {
            session
                .starttls()
                .await
                .map_err(|error| annotate(error, config, deadlines, Stage::ConnectionSetup))?;
        }

        if let Some(credentials) = config.credentials.as_ref() {
            self.mark(Stage::Auth);
            session
                .authenticate(
                    &credentials.username,
                    &credentials.password,
                    credentials.mechanism(),
                )
                .await
                .map_err(|error| annotate(error, config, deadlines, Stage::Auth))?;
        }
        Ok(session)
    }

    /// Keeps a session for the next send when it is still healthy.
    ///
    /// A completed transaction leaves the session idle, so no `RSET` is sent
    /// then: it would only add a round trip to every message. `RSET` is sent
    /// only when a transaction was left open, and a session that refuses it, or
    /// that reached its message cap, is closed instead. The outcome the relay
    /// already gave is never changed by this cleanup.
    async fn retain(&mut self, mut session: Session, needs_reset: bool) {
        self.served += 1;
        if self.served >= self.config.reuse.max_messages() {
            let budget = self.config.timeouts.operation();
            let _ = tokio::time::timeout(budget, session.quit()).await;
            return;
        }
        if needs_reset && session.reset().await.is_err() {
            return;
        }
        self.session = Some(session);
        self.idle_since = Some(Instant::now());
    }

    /// One SMTP transaction over an already authenticated session.
    ///
    /// Returns the outcome with what the session owes the next message: whether
    /// a transaction was left open, and whether the relay ended the channel.
    async fn transact(
        &self,
        session: &mut Session,
        deadlines: SendDeadlines,
        envelope: Envelope,
        wire: &[u8],
    ) -> Result<(SendOutcome, SessionState), Error> {
        let config = &self.config;
        self.mark(Stage::Mail);
        match session.mail_from(&envelope.from).await {
            Ok(reply) if reply.is_positive() => {}
            Ok(reply) => {
                return match verdict(&reply) {
                    Verdict::Refused => Ok((
                        rejected_outcome(
                            &envelope,
                            &vec![RecipientStatus::Rejected; envelope.recipients.len()],
                            Some(smtp_response(&reply)),
                        ),
                        SessionState::clean(),
                    )),
                    Verdict::Complete | Verdict::Invalid => {
                        Err(unexpected_response(config, Stage::Mail, reply.value()))
                    }
                };
            }
            Err(error) => return Err(annotate(error, config, deadlines, Stage::Mail)),
        }

        let mut statuses = vec![RecipientStatus::Unknown; envelope.recipients.len()];
        let mut accepted: Vec<usize> = Vec::new();
        let mut closing = false;

        for (index, recipient) in envelope.recipients.iter().enumerate() {
            if closing {
                break;
            }
            self.mark(Stage::Rcpt);
            match session.rcpt_to(recipient).await {
                Ok(reply) if reply.is_positive() => {
                    self.accept_recipient(recipient);
                    accepted.push(index);
                }
                Ok(reply) if reply.value() == 421 => {
                    // A 421 closes the transmission channel: stop the envelope
                    // and never issue DATA. It is not a per-recipient refusal, so
                    // this recipient and every unattempted one stay `unknown`.
                    closing = true;
                }
                Ok(reply) if verdict(&reply) == Verdict::Refused => {
                    statuses[index] = RecipientStatus::Rejected;
                }
                Ok(reply) => {
                    return Err(unexpected_response(config, Stage::Rcpt, reply.value()));
                }
                Err(error) => return Err(annotate(error, config, deadlines, Stage::Rcpt)),
            }
        }

        if closing || accepted.is_empty() {
            // The envelope was opened but never completed, and a 421 means the
            // relay is shutting the channel down, so it is not reusable.
            return Ok((
                rejected_outcome(&envelope, &statuses, None),
                SessionState {
                    needs_reset: true,
                    reusable: !closing,
                },
            ));
        }

        self.mark(Stage::DataCommand);
        match session.data_command().await {
            Ok(reply) if reply.value() == 354 => {}
            Ok(reply) => {
                return match verdict(&reply) {
                    Verdict::Refused => Ok((
                        rejected_outcome(
                            &envelope,
                            &vec![RecipientStatus::Rejected; envelope.recipients.len()],
                            Some(smtp_response(&reply)),
                        ),
                        SessionState::aborted(reply.value() != 421),
                    )),
                    Verdict::Complete | Verdict::Invalid => Err(unexpected_response(
                        config,
                        Stage::DataCommand,
                        reply.value(),
                    )),
                };
            }
            Err(error) => return Err(annotate(error, config, deadlines, Stage::DataCommand)),
        }

        self.mark(Stage::DataBody);
        match session.data_body(wire).await {
            Ok(reply) => match verdict(&reply) {
                Verdict::Complete => {
                    settle(&mut statuses, &accepted, RecipientStatus::Accepted);
                    Ok((
                        SendOutcome::Accepted {
                            recipients: outcomes(&envelope, &statuses),
                            response: Some(smtp_response(&reply)),
                        },
                        SessionState::clean(),
                    ))
                }
                Verdict::Refused => {
                    settle(&mut statuses, &accepted, RecipientStatus::Rejected);
                    Ok((
                        rejected_outcome(&envelope, &statuses, Some(smtp_response(&reply))),
                        SessionState::clean(),
                    ))
                }
                Verdict::Invalid => {
                    settle(&mut statuses, &accepted, RecipientStatus::Unknown);
                    Ok((
                        SendOutcome::Unknown {
                            recipients: outcomes(&envelope, &statuses),
                            stage: AmbiguityStage::FinalResponse,
                        },
                        SessionState::clean(),
                    ))
                }
            },
            Err(_) => {
                // A timeout or connection failure after the DATA terminator
                // leaves the disposition unknown. It is never retried.
                settle(&mut statuses, &accepted, RecipientStatus::Unknown);
                Ok((
                    SendOutcome::Unknown {
                        recipients: outcomes(&envelope, &statuses),
                        stage: AmbiguityStage::FinalResponse,
                    },
                    SessionState::drained(),
                ))
            }
        }
    }
}

/// What one finished transaction owes the next message on the same session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SessionState {
    /// A transaction was left open, so `RSET` must precede the next `MAIL`.
    needs_reset: bool,
    /// The relay did not close the channel; a 421 means it did.
    reusable: bool,
}

impl SessionState {
    /// The transaction completed or never started: nothing to clean up.
    const fn clean() -> Self {
        Self {
            needs_reset: false,
            reusable: true,
        }
    }

    /// The envelope was opened and abandoned, so the next send must reset first.
    const fn aborted(reusable: bool) -> Self {
        Self {
            needs_reset: true,
            reusable,
        }
    }

    /// The transaction may have reached the relay: the session is never reused.
    const fn drained() -> Self {
        Self {
            needs_reset: false,
            reusable: false,
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

/// The envelope addresses, validated exactly once before the session opens.
struct Envelope {    from: String,
    recipients: Vec<String>,
}

impl Envelope {
    fn of(message: &OutgoingMessage) -> Result<Self, Error> {
        let from = envelope_address(message.from.address())?;
        let mut recipients = Vec::with_capacity(message.recipient_count());
        for mailbox in message.to.iter().chain(&message.cc).chain(&message.bcc) {
            recipients.push(envelope_address(mailbox.address())?);
        }
        Ok(Self { from, recipients })
    }
}

/// Re-checks the envelope address at the protocol boundary.
///
/// `Mailbox` already validated the value; this keeps a hand-built
/// `OutgoingMessage` from reaching the codec, where the failure would surface
/// as an injection error instead of an invalid address.
fn envelope_address(address: String) -> Result<String, Error> {
    if address.is_empty()
        || address.len() > 320
        || address
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || matches!(c, '<' | '>' | ','))
    {
        return Err(Error::new(ErrorCode::ValidationInvalidAddress));
    }
    Ok(address)
}

/// How a reply classifies for the transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    /// 2xx: the command completed.
    Complete,
    /// 4xx or 5xx: the relay refused; that is an outcome, not an error.
    Refused,
    /// 1xx or 3xx where a completion was required: a protocol violation.
    Invalid,
}

fn verdict(reply: &Reply) -> Verdict {
    match reply.code().class() {
        ReplyClass::Positive => Verdict::Complete,
        ReplyClass::Transient | ReplyClass::Permanent => Verdict::Refused,
        ReplyClass::Intermediate => Verdict::Invalid,
    }
}

fn smtp_response(reply: &Reply) -> SmtpResponse {
    SmtpResponse {
        code: reply.value(),
        message: reply.text(),
    }
}

fn outcomes(envelope: &Envelope, statuses: &[RecipientStatus]) -> Vec<RecipientOutcome> {
    envelope
        .recipients
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

/// Adds the relay context the public error contract reports, without dropping
/// anything the session already classified.
fn annotate(error: Error, config: &SmtpConfig, deadlines: SendDeadlines, stage: Stage) -> Error {
    let existing = error.details();
    let mut details = ErrorDetails::default()
        .host(
            existing
                .host_name()
                .map(str::to_owned)
                .unwrap_or_else(|| config.host.clone()),
        )
        .port(existing.port_number().unwrap_or(config.port))
        .stage(existing.operation_stage().unwrap_or(stage))
        .elapsed_ms(
            existing
                .elapsed_millis()
                .unwrap_or_else(|| millis(deadlines.elapsed(Instant::now()))),
        );

    if let Some(smtp_code) = existing.smtp_status() {
        details = details.smtp_code(smtp_code);
    }
    if let Some(enhanced) = existing.smtp_enhanced() {
        details = details.smtp_enhanced_code(enhanced);
    }
    if let Some(timeout_ms) = existing.timeout_millis() {
        details = details.timeout_ms(timeout_ms);
    }
    if let Some(recipient) = existing.recipient_address() {
        details = details.recipient(recipient);
    }

    Error::new(error.code()).with_details(details)
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

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transports::smtp::proto::response::parse_reply;

    fn reply(text: &str) -> Reply {
        parse_reply(text.as_bytes()).expect("parsed reply").0
    }

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
    fn hello_name_is_one_ascii_token() {
        let relay = || SmtpConfig::new("relay.test", 587, SmtpSecurity::StartTls).expect("valid");
        for name in ["localhost", "client.example.test", "[127.0.0.1]"] {
            assert!(
                relay().with_hello_name(name).is_ok(),
                "{name} should be accepted"
            );
        }
        let long = "a".repeat(256);
        for name in [
            "",
            "bad name",
            "tab\tname",
            "line\r\nname",
            "héllo",
            long.as_str(),
        ] {
            let error = relay().with_hello_name(name).expect_err("must be rejected");
            assert_eq!(error.code(), ErrorCode::ValidationInvalidOption, "{name:?}");
        }
    }

    #[test]
    fn raised_timeouts_are_rejected_by_the_configuration() {
        let timeouts = Timeouts::default()
            .with_operation(Duration::from_secs(31))
            .expect_err("raised operation budget");
        assert_eq!(timeouts.code(), ErrorCode::ValidationInvalidTimeout);
    }

    #[test]
    fn reply_classes_map_to_verdicts() {
        assert_eq!(verdict(&reply("250 2.0.0 Ok\r\n")), Verdict::Complete);
        assert_eq!(verdict(&reply("451 4.3.0 Later\r\n")), Verdict::Refused);
        assert_eq!(verdict(&reply("550 5.1.1 No user\r\n")), Verdict::Refused);
        assert_eq!(verdict(&reply("354 Send data\r\n")), Verdict::Invalid);
        assert_eq!(verdict(&reply("334 VXNlcm5hbWU6\r\n")), Verdict::Invalid);
    }

    #[test]
    fn envelope_rejects_addresses_the_codec_would_misread() {
        assert_eq!(
            envelope_address(String::new()).expect_err("empty").code(),
            ErrorCode::ValidationInvalidAddress
        );
        assert_eq!(
            envelope_address("a b@example.test".to_string())
                .expect_err("whitespace")
                .code(),
            ErrorCode::ValidationInvalidAddress
        );
        assert_eq!(
            envelope_address("user@example.test\r\nDATA".to_string())
                .expect_err("control")
                .code(),
            ErrorCode::ValidationInvalidAddress
        );
        assert_eq!(
            envelope_address("bob@example.test".to_string()).expect("valid"),
            "bob@example.test"
        );
    }

    #[test]
    fn annotations_add_relay_context_and_keep_the_code() {
        let config = SmtpConfig::new("relay.example.test", 587, SmtpSecurity::StartTls)
            .expect("valid relay");
        let deadlines = SendDeadlines::start(config.timeouts(), Instant::now());

        let error = annotate(
            Error::new(ErrorCode::ProtocolConnectionReset),
            &config,
            deadlines,
            Stage::Rcpt,
        );

        assert_eq!(error.code(), ErrorCode::ProtocolConnectionReset);
        assert_eq!(error.details().host_name(), Some("relay.example.test"));
        assert_eq!(error.details().port_number(), Some(587));
        assert_eq!(error.details().operation_stage(), Some(Stage::Rcpt));
        assert!(error.details().elapsed_millis().is_some());
    }
}
