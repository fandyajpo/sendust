//! Thin Node-API inbound adapter for Sendust.
//!
//! Owns JavaScript conversion, owned-memory snapshots, asynchronous task and
//! lifecycle bridging, and error/result mapping only. SMTP, MIME, validation,
//! and delivery policy belong to `sendust-core` and must not be duplicated here.

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

use napi::Status;
use napi::bindgen_prelude::{Result, Uint8Array};
use napi_derive::napi;
use sendust_core::{
    Attachment, AuthMechanism, Delivery, DeliveryProgress, DkimSigner, Error as CoreError,
    ErrorCode, FileDescriptor, Header, Limits, Mailbox, OutgoingMessage, ReusePolicy, SmtpConfig,
    SmtpCredentials, SmtpSecurity, SmtpTransport, Timeouts,
};

const OPEN: u8 = 0;
const CLOSING: u8 = 1;
const CLOSED: u8 = 2;
/// The spawned send ended abnormally; the transport cannot be used again.
const FAILED: u8 = 3;

/// Diagnostic version string for the native adapter and the core it wraps.
#[napi]
pub fn native_version() -> String {
    format!(
        "sendust-napi {} (core {})",
        env!("CARGO_PKG_VERSION"),
        sendust_core::version()
    )
}

/// Relay credentials: a password for PLAIN/LOGIN, or a bearer token for
/// XOAUTH2 (ADR-0023). Exactly one secret is set; the facade validates the
/// combination first, and a direct native caller gets the same stable code.
#[napi(object)]
pub struct AuthOptions {
    pub user: String,
    pub pass: Option<String>,
    pub access_token: Option<String>,
    pub method: Option<String>,
}

/// DKIM signing (ADR-0024). The key is parsed in the constructor, so a key that
/// is not usable fails `new Transport(...)` rather than the first send.
#[napi(object)]
pub struct DkimOptions {
    /// Signing domain, the DKIM `d=` tag.
    pub domain: String,
    /// Key selector, the DKIM `s=` tag.
    pub selector: String,
    /// PEM private key, PKCS#1 or PKCS#8.
    pub private_key: String,
    /// Header fields to sign; the core default applies when omitted.
    pub headers: Option<Vec<String>>,
}

/// Transport configuration. Mirrors the public contract; core validates it.
#[napi(object)]
pub struct TransportOptions {
    pub host: String,
    pub port: u32,
    pub secure: String,
    pub auth: Option<AuthOptions>,
    pub ehlo_name: Option<String>,
    pub connect_ms: Option<u32>,
    pub operation_ms: Option<u32>,
    pub total_ms: Option<u32>,
    pub close_ms: Option<u32>,
    pub max_recipients: Option<u32>,
    pub max_attachment_bytes: Option<u32>,
    pub max_total_attachment_bytes: Option<u32>,
    pub max_wire_bytes: Option<u32>,
    pub allow_file_access: Option<bool>,
    /// Additive PEM trust anchor; verification stays on (ADR-0018).
    pub tls_root_certificate: Option<String>,
    /// Messages one authenticated session may serve (ADR-0022).
    pub max_messages_per_session: Option<u32>,
    /// How long an idle session may be reused before a fresh dial (ADR-0022).
    pub idle_reuse_ms: Option<u32>,
    /// DKIM signing; a message is sent unsigned only when this is absent.
    pub dkim: Option<DkimOptions>,
}

/// One custom header.
#[napi(object)]
pub struct HeaderPair {
    pub name: String,
    pub value: String,
}

/// One attachment. Exactly one of `path` or `content` must be set.
///
/// `content` is copied synchronously into Rust-owned memory; `path` is only
/// opened later, inside the async task, when filesystem access is enabled.
#[napi(object)]
pub struct AttachmentOptions {
    pub filename: Option<String>,
    pub content_type: Option<String>,
    pub path: Option<String>,
    pub content: Option<Uint8Array>,
}

/// One outgoing message, fully copied before any async work begins.
#[napi(object)]
pub struct MessageOptions {
    pub from: Option<String>,
    pub to: Option<Vec<String>>,
    pub cc: Option<Vec<String>>,
    pub bcc: Option<Vec<String>>,
    pub reply_to: Option<Vec<String>>,
    pub subject: Option<String>,
    pub text: Option<String>,
    pub html: Option<String>,
    pub headers: Option<Vec<HeaderPair>>,
    pub attachments: Option<Vec<AttachmentOptions>>,
}

/// Per-recipient result.
#[napi(object)]
pub struct RecipientOutcomeJs {
    pub address: String,
    pub status: String,
}

/// Final SMTP response, when one was observed.
#[napi(object)]
pub struct SmtpResponseJs {
    pub code: u32,
    pub message: String,
}

/// Relay outcome: accepted, rejected, or unknown. Never a delivery claim.
#[napi(object)]
pub struct SendResultJs {
    pub status: String,
    pub recipients: Vec<RecipientOutcomeJs>,
    pub response: Option<SmtpResponseJs>,
}

enum Active {
    Idle,
    Sending(Option<tokio::task::AbortHandle>),
}

struct Inner {
    slot: tokio::sync::Mutex<Option<SmtpTransport>>,
    active: tokio::sync::Mutex<Active>,
    idle: tokio::sync::Notify,
    phase: AtomicU8,
    limits: Limits,
    filesystem: bool,
    close_timeout: Duration,
    progress: Arc<DeliveryProgress>,
    dkim: Option<DkimSigner>,
}

/// One open transport. One active send; concurrent sends fail with BUSY.
#[napi]
pub struct Transport {
    inner: Arc<Inner>,
}

#[napi]
impl Transport {
    /// A panic in the sync boundary is caught and rethrown as a JS error rather
    /// than unwinding across the FFI (async methods are settled by the runtime).
    #[napi(catch_unwind, constructor)]
    pub fn new(options: TransportOptions) -> Result<Self> {
        if options.port == 0 || options.port > u32::from(u16::MAX) {
            return Err(to_napi(CoreError::new(ErrorCode::ConfigInvalidRelay)));
        }
        let port = options.port as u16;

        let security = match options.secure.as_str() {
            "tls" | "implicit-tls" => SmtpSecurity::ImplicitTls,
            "starttls" => SmtpSecurity::StartTls,
            _ => return Err(to_napi(CoreError::new(ErrorCode::ValidationInvalidOption))),
        };

        let mut config = SmtpConfig::new(options.host, port, security).map_err(to_napi)?;

        if let Some(pem) = options.tls_root_certificate {
            config = config.with_root_certificate_pem(pem).map_err(to_napi)?;
        }

        if let Some(auth) = options.auth {
            let mut credentials = match (auth.access_token, auth.pass) {
                (Some(token), None) => SmtpCredentials::with_access_token(auth.user, token),
                (None, Some(pass)) => SmtpCredentials::new(auth.user, pass),
                _ => Err(CoreError::new(ErrorCode::ValidationInvalidOption)),
            }
            .map_err(to_napi)?;
            if let Some(method) = auth.method {
                let mechanism = match method.as_str() {
                    "plain" => AuthMechanism::Plain,
                    "login" => AuthMechanism::Login,
                    "xoauth2" => AuthMechanism::XOAuth2,
                    _ => {
                        return Err(to_napi(CoreError::new(ErrorCode::ValidationInvalidOption)));
                    }
                };
                credentials = credentials.with_mechanism(mechanism);
            }
            config = config.with_credentials(credentials);
        }

        if let Some(hello_name) = options.ehlo_name {
            config = config.with_hello_name(hello_name).map_err(to_napi)?;
        }

        let mut timeouts = Timeouts::default();
        if let Some(ms) = options.connect_ms {
            timeouts = timeouts
                .with_connect(Duration::from_millis(u64::from(ms)))
                .map_err(to_napi)?;
        }
        if let Some(ms) = options.operation_ms {
            timeouts = timeouts
                .with_operation(Duration::from_millis(u64::from(ms)))
                .map_err(to_napi)?;
        }
        if let Some(ms) = options.total_ms {
            timeouts = timeouts
                .with_total(Duration::from_millis(u64::from(ms)))
                .map_err(to_napi)?;
        }
        if let Some(ms) = options.close_ms {
            timeouts = timeouts
                .with_close(Duration::from_millis(u64::from(ms)))
                .map_err(to_napi)?;
        }
        config = config.with_timeouts(timeouts).map_err(to_napi)?;

        let mut reuse = ReusePolicy::default();
        if let Some(value) = options.max_messages_per_session {
            reuse = reuse.with_max_messages(value as usize);
        }
        if let Some(ms) = options.idle_reuse_ms {
            reuse = reuse.with_idle(Duration::from_millis(u64::from(ms)));
        }
        config = config.with_reuse(reuse);

        let mut limits = Limits::default();
        if let Some(value) = options.max_recipients {
            limits = limits
                .with_max_recipients(value as usize)
                .map_err(to_napi)?;
        }
        if let Some(value) = options.max_attachment_bytes {
            limits = limits
                .with_max_attachment_bytes(u64::from(value))
                .map_err(to_napi)?;
        }
        if let Some(value) = options.max_total_attachment_bytes {
            limits = limits
                .with_max_total_attachment_bytes(u64::from(value))
                .map_err(to_napi)?;
        }
        if let Some(value) = options.max_wire_bytes {
            limits = limits
                .with_max_wire_bytes(u64::from(value))
                .map_err(to_napi)?;
        }

        let progress = Arc::new(DeliveryProgress::new());
        let dkim = match options.dkim {
            Some(dkim) => {
                let mut signer = DkimSigner::new(dkim.domain, dkim.selector, &dkim.private_key)
                    .map_err(to_napi)?;
                if let Some(headers) = dkim.headers {
                    signer = signer.with_headers(&headers).map_err(to_napi)?;
                }
                Some(signer)
            }
            None => None,
        };
        Ok(Self {
            inner: Arc::new(Inner {
                slot: tokio::sync::Mutex::new(Some(
                    SmtpTransport::new(config).with_progress(Arc::clone(&progress)),
                )),
                active: tokio::sync::Mutex::new(Active::Idle),
                idle: tokio::sync::Notify::new(),
                phase: AtomicU8::new(OPEN),
                limits,
                filesystem: options.allow_file_access.unwrap_or(false),
                close_timeout: timeouts.close(),
                progress,
                dkim,
            }),
        })
    }

    /// `open`, `closing`, or `closed`.
    #[napi(catch_unwind)]
    pub fn state(&self) -> String {
        match self.inner.phase.load(Ordering::SeqCst) {
            OPEN => "open".to_string(),
            CLOSING => "closing".to_string(),
            _ => "closed".to_string(),
        }
    }

    /// The error a caller sees before a send, if the transport cannot take one.
    ///
    /// A transport whose send ended abnormally reports `INTERNAL`: the failure is
    /// in this library, not an ambiguous relay answer, and it is not
    /// `TRANSPORT_BUSY`, which would suggest another send owns the transport.
    fn phase_error(&self) -> Option<napi::Error> {
        match self.inner.phase.load(Ordering::SeqCst) {
            OPEN => None,
            FAILED => Some(to_napi(CoreError::new(ErrorCode::Internal))),
            _ => Some(to_napi(CoreError::new(ErrorCode::TransportClosed))),
        }
    }

    /// Submits one message. Inputs are copied before the first await.
    #[napi]
    pub async fn send(&self, message: MessageOptions) -> Result<SendResultJs> {
        if let Some(error) = self.phase_error() {
            return Err(error);
        }

        {
            let mut active = self.inner.active.lock().await;
            if !matches!(*active, Active::Idle) {
                return Err(to_napi(CoreError::new(ErrorCode::TransportBusy)));
            }
            *active = Active::Sending(None);
        }

        let transport = {
            let mut slot = match self.inner.slot.try_lock() {
                Ok(slot) => slot,
                Err(_) => {
                    *self.inner.active.lock().await = Active::Idle;
                    self.inner.idle.notify_waiters();
                    return Err(to_napi(CoreError::new(ErrorCode::TransportBusy)));
                }
            };
            match slot.take() {
                Some(transport) => transport,
                None => {
                    *self.inner.active.lock().await = Active::Idle;
                    self.inner.idle.notify_waiters();
                    return Err(to_napi(CoreError::new(ErrorCode::TransportBusy)));
                }
            }
        };

        if self.inner.phase.load(Ordering::SeqCst) != OPEN {
            *self.inner.slot.lock().await = Some(transport);
            *self.inner.active.lock().await = Active::Idle;
            self.inner.idle.notify_waiters();
            return Err(to_napi(CoreError::new(ErrorCode::TransportClosed)));
        }

        let prepared = match build_message(message) {
            Ok(prepared) => prepared,
            Err(error) => {
                *self.inner.slot.lock().await = Some(transport);
                *self.inner.active.lock().await = Active::Idle;
                self.inner.idle.notify_waiters();
                return Err(to_napi(error));
            }
        };

        let inner = Arc::clone(&self.inner);
        // Reset the cancellation progress before the task exists: a close that
        // lands before the task reaches `run` must not read the previous
        // message's stage and recipients as this message's.
        self.inner.progress.reset();
        let handle = tokio::spawn(async move {
            let mut transport = transport;
            let mut delivery = Delivery::new(
                transport.session(),
                inner.limits,
                sendust_core::AttachmentPolicy::from_flag(inner.filesystem),
            );
            if let Some(signer) = inner.dkim.clone() {
                delivery = delivery.with_dkim(signer);
            }
            let outcome = delivery.send(prepared).await;
            (transport, outcome)
        });

        {
            let mut active = self.inner.active.lock().await;
            if matches!(*active, Active::Sending(None)) {
                *active = Active::Sending(Some(handle.abort_handle()));
            }
        }
        // Wake close so it can abort this handle if it started before we stored it.
        self.inner.idle.notify_waiters();

        if self.inner.phase.load(Ordering::SeqCst) != OPEN {
            handle.abort();
        }

        let joined = handle.await;
        {
            let mut active = self.inner.active.lock().await;
            *active = Active::Idle;
        }
        self.inner.idle.notify_waiters();

        match joined {
            Ok((transport, outcome)) => {
                if self.inner.phase.load(Ordering::SeqCst) == OPEN {
                    *self.inner.slot.lock().await = Some(transport);
                }
                outcome.map(to_result).map_err(to_napi)
            }
            Err(error) if error.is_cancelled() => match self.inner.progress.cancelled_outcome() {
                // The body may have reached the relay before the abort, so the
                // disposition is ambiguous rather than a definitive cancellation.
                Some(outcome) => Ok(to_result(outcome)),
                None => Err(to_napi(CoreError::new(ErrorCode::TransportClosed))),
            },
            Err(_) => {
                // The task ended abnormally and took the transport with it, so the
                // slot stays empty. That state is recorded explicitly, or every
                // later send would report TRANSPORT_BUSY forever, with the wrong
                // cause and no way to recover.
                self.inner.phase.store(FAILED, Ordering::SeqCst);
                Err(to_napi(CoreError::new(ErrorCode::Internal)))
            }
        }
    }

    /// Idempotent, bounded close. Aborts any active send and waits for it to
    /// stop within the close budget; a timeout is reported, never hidden.
    #[napi]
    pub async fn close(&self) -> Result<()> {
        let previous = self.inner.phase.swap(CLOSING, Ordering::SeqCst);
        if previous == CLOSED {
            self.inner.phase.store(CLOSED, Ordering::SeqCst);
            return Ok(());
        }

        let deadline = tokio::time::Instant::now() + self.inner.close_timeout;
        loop {
            {
                let active = self.inner.active.lock().await;
                match &*active {
                    Active::Idle => break,
                    Active::Sending(Some(handle)) => handle.abort(),
                    Active::Sending(None) => {}
                }
            }

            let notified = self.inner.idle.notified();
            if matches!(*self.inner.active.lock().await, Active::Idle) {
                break;
            }
            if tokio::time::timeout_at(deadline, notified).await.is_err() {
                self.inner.phase.store(CLOSED, Ordering::SeqCst);
                return Err(to_napi(CoreError::new(ErrorCode::TimeoutClose)));
            }
        }

        // A retained session gets a polite QUIT before the slot is dropped. An
        // aborted in-flight send already dropped its session, which is why this
        // is only reached once `active` is idle.
        let retained = match self.inner.slot.try_lock() {
            Ok(mut slot) => slot.take(),
            Err(_) => None,
        };
        if let Some(mut transport) = retained {
            let budget = self.inner.close_timeout;
            let _ = tokio::time::timeout(budget, transport.shutdown()).await;
        }
        self.inner.phase.store(CLOSED, Ordering::SeqCst);
        Ok(())
    }
}

fn mailboxes(values: Option<Vec<String>>) -> std::result::Result<Vec<Mailbox>, CoreError> {
    values
        .unwrap_or_default()
        .iter()
        .map(|value| Mailbox::parse(value))
        .collect()
}

fn build_message(options: MessageOptions) -> std::result::Result<OutgoingMessage, CoreError> {
    let headers = options
        .headers
        .unwrap_or_default()
        .into_iter()
        .map(|header| Header::new(&header.name, &header.value))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let attachments = options
        .attachments
        .unwrap_or_default()
        .into_iter()
        .map(attachment)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let from = options
        .from
        .ok_or_else(|| CoreError::new(ErrorCode::ValidationMissingFrom))?;

    Ok(OutgoingMessage {
        from: Mailbox::parse(&from)?,
        to: mailboxes(options.to)?,
        cc: mailboxes(options.cc)?,
        bcc: mailboxes(options.bcc)?,
        reply_to: mailboxes(options.reply_to)?,
        subject: options.subject.unwrap_or_default(),
        text: options.text,
        html: options.html,
        headers,
        attachments,
    })
}

/// Maps one JavaScript attachment, copying buffer content into owned memory.
fn attachment(options: AttachmentOptions) -> std::result::Result<Attachment, CoreError> {
    match (options.path, options.content) {
        (Some(path), None) => Ok(Attachment::File(FileDescriptor {
            display_name: options.filename,
            content_type: options.content_type,
            path,
        })),
        (None, Some(content)) => {
            let filename = options
                .filename
                .ok_or_else(|| CoreError::new(ErrorCode::ValidationInvalidAttachment))?;
            Ok(Attachment::Bytes {
                filename,
                content_type: options.content_type,
                bytes: content.to_vec(),
            })
        }
        _ => Err(CoreError::new(ErrorCode::ValidationInvalidAttachment)),
    }
}

fn to_result(outcome: sendust_core::SendOutcome) -> SendResultJs {
    let recipients = outcome
        .recipients()
        .iter()
        .map(|recipient| RecipientOutcomeJs {
            address: recipient.address.clone(),
            status: recipient.status.as_str().to_string(),
        })
        .collect();

    SendResultJs {
        status: outcome.status().as_str().to_string(),
        recipients,
        response: outcome.response().map(|response| SmtpResponseJs {
            code: u32::from(response.code),
            message: response.message.clone(),
        }),
    }
}

fn to_napi(error: CoreError) -> napi::Error {
    let mut details: Vec<String> = Vec::new();
    if let Some(host) = error.details().host_name() {
        details.push(format!("\"host\":\"{}\"", escape(host)));
    }
    if let Some(port) = error.details().port_number() {
        details.push(format!("\"port\":{port}"));
    }
    if let Some(stage) = error.details().operation_stage() {
        details.push(format!("\"stage\":\"{}\"", stage.as_str()));
    }
    if let Some(code) = error.details().smtp_status() {
        details.push(format!("\"smtpCode\":{code}"));
    }
    if let Some(enhanced) = error.details().smtp_enhanced() {
        details.push(format!("\"smtpEnhancedCode\":\"{}\"", escape(enhanced)));
    }
    if let Some(timeout) = error.details().timeout_millis() {
        details.push(format!("\"timeoutMs\":{timeout}"));
    }
    if let Some(elapsed) = error.details().elapsed_millis() {
        details.push(format!("\"elapsedMs\":{elapsed}"));
    }
    if let Some(recipient) = error.details().recipient_address() {
        details.push(format!("\"recipient\":\"{}\"", escape(recipient)));
    }

    let payload = format!(
        "{{\"code\":\"{}\",\"category\":\"{}\",\"message\":\"{}\",\"details\":{{{}}}}}",
        error.code().as_str(),
        error.category().as_str(),
        escape(error.message()),
        details.join(",")
    );

    napi::Error::new(Status::GenericFailure, format!("SENDUST:{payload}"))
}

fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if control.is_control() => out.push(' '),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Options that build a transport without opening a connection: the core
    /// validates the relay at construction and dials only on the first send.
    fn options() -> TransportOptions {
        TransportOptions {
            host: "127.0.0.1".to_string(),
            port: 1,
            secure: "tls".to_string(),
            auth: None,
            ehlo_name: None,
            connect_ms: None,
            operation_ms: None,
            total_ms: None,
            close_ms: None,
            max_recipients: None,
            max_attachment_bytes: None,
            max_total_attachment_bytes: None,
            max_wire_bytes: None,
            allow_file_access: None,
            tls_root_certificate: None,
            max_messages_per_session: None,
            idle_reuse_ms: None,
            dkim: None,
        }
    }

    #[test]
    fn a_transport_is_open_before_any_send() {
        let transport = Transport::new(options()).expect("constructed");
        assert!(transport.phase_error().is_none());
        assert_eq!(transport.state(), "open");
    }

    #[test]
    fn a_transport_whose_task_ended_abnormally_is_internal_and_never_busy() {
        let transport = Transport::new(options()).expect("constructed");
        // The abnormal end of the spawned task leaves the slot empty. Before the
        // fix this was indistinguishable from "another send owns it", so every
        // later send answered TRANSPORT_BUSY for the life of the process.
        transport.inner.phase.store(FAILED, Ordering::SeqCst);

        let error = transport.phase_error().expect("an error").reason.clone();
        assert!(error.contains("\"code\":\"INTERNAL\""), "{error}");
        assert!(!error.contains("TRANSPORT_BUSY"), "{error}");
        assert_eq!(transport.state(), "closed");
    }

    #[test]
    fn a_closed_transport_reports_closed() {
        let transport = Transport::new(options()).expect("constructed");
        transport.inner.phase.store(CLOSED, Ordering::SeqCst);

        let error = transport.phase_error().expect("an error").reason.clone();
        assert!(error.contains("\"code\":\"TRANSPORT_CLOSED\""), "{error}");
    }
}
