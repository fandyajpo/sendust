//! Thin Node-API inbound adapter for Sendust.
//!
//! Owns JavaScript conversion, owned-memory snapshots, asynchronous task and
//! lifecycle bridging, and error/result mapping only. SMTP, MIME, validation,
//! and delivery policy belong to `sendust-core` and must not be duplicated here.

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

use napi::Status;
use napi::bindgen_prelude::Result;
use napi_derive::napi;
use sendust_core::{
    Delivery, Error as CoreError, ErrorCode, Header, Limits, Mailbox, OutgoingMessage, SmtpConfig,
    SmtpCredentials, SmtpSecurity, SmtpTransport, Timeouts,
};

const OPEN: u8 = 0;
const CLOSING: u8 = 1;
const CLOSED: u8 = 2;

/// Diagnostic version string for the native adapter and the core it wraps.
#[napi]
pub fn native_version() -> String {
    format!(
        "sendust-napi {} (core {})",
        env!("CARGO_PKG_VERSION"),
        sendust_core::version()
    )
}

/// Relay credentials, copied into Rust-owned strings immediately.
#[napi(object)]
pub struct AuthOptions {
    pub user: String,
    pub pass: String,
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
}

/// One custom header.
#[napi(object)]
pub struct HeaderPair {
    pub name: String,
    pub value: String,
}

/// One outgoing message, fully copied before any async work begins.
#[napi(object)]
pub struct MessageOptions {
    pub from: String,
    pub to: Option<Vec<String>>,
    pub cc: Option<Vec<String>>,
    pub bcc: Option<Vec<String>>,
    pub reply_to: Option<Vec<String>>,
    pub subject: String,
    pub text: Option<String>,
    pub html: Option<String>,
    pub headers: Option<Vec<HeaderPair>>,
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

struct Inner {
    slot: tokio::sync::Mutex<Option<SmtpTransport>>,
    phase: AtomicU8,
    limits: Limits,
    filesystem: bool,
    close_timeout: Duration,
}

/// One open transport. One active send; concurrent sends fail with BUSY.
#[napi]
pub struct Transport {
    inner: Arc<Inner>,
}

#[napi]
impl Transport {
    #[napi(constructor)]
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

        if let Some(auth) = options.auth {
            let credentials = SmtpCredentials::new(auth.user, auth.pass).map_err(to_napi)?;
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

        Ok(Self {
            inner: Arc::new(Inner {
                slot: tokio::sync::Mutex::new(Some(SmtpTransport::new(config))),
                phase: AtomicU8::new(OPEN),
                limits,
                filesystem: options.allow_file_access.unwrap_or(false),
                close_timeout: timeouts.close(),
            }),
        })
    }

    /// `open`, `closing`, or `closed`.
    #[napi]
    pub fn state(&self) -> String {
        match self.inner.phase.load(Ordering::SeqCst) {
            OPEN => "open".to_string(),
            CLOSING => "closing".to_string(),
            _ => "closed".to_string(),
        }
    }

    /// Submits one message. Inputs are copied before the first await.
    #[napi]
    pub async fn send(&self, message: MessageOptions) -> Result<SendResultJs> {
        if self.inner.phase.load(Ordering::SeqCst) != OPEN {
            return Err(to_napi(CoreError::new(ErrorCode::TransportClosed)));
        }

        let mut guard = match self.inner.slot.try_lock() {
            Ok(guard) => guard,
            Err(_) => return Err(to_napi(CoreError::new(ErrorCode::TransportBusy))),
        };

        if self.inner.phase.load(Ordering::SeqCst) != OPEN {
            return Err(to_napi(CoreError::new(ErrorCode::TransportClosed)));
        }

        let Some(mut transport) = guard.take() else {
            return Err(to_napi(CoreError::new(ErrorCode::TransportBusy)));
        };

        let prepared = match build_message(message) {
            Ok(prepared) => prepared,
            Err(error) => {
                *guard = Some(transport);
                return Err(to_napi(error));
            }
        };

        let outcome = {
            let mut delivery = Delivery::new(
                transport.session(),
                self.inner.limits,
                sendust_core::AttachmentPolicy::from_flag(self.inner.filesystem),
            );
            delivery.send(prepared).await
        };

        *guard = Some(transport);

        match outcome {
            Ok(outcome) => Ok(to_result(outcome)),
            Err(error) => Err(to_napi(error)),
        }
    }

    /// Idempotent, bounded close. Drain is capped at the configured close budget.
    #[napi]
    pub async fn close(&self) -> Result<()> {
        let previous = self.inner.phase.swap(CLOSING, Ordering::SeqCst);
        if previous == CLOSED {
            // Another close already finished; stay closed.
            self.inner.phase.store(CLOSED, Ordering::SeqCst);
            return Ok(());
        }

        let waited = tokio::time::timeout(self.inner.close_timeout, self.inner.slot.lock()).await;
        match waited {
            Ok(mut slot) => {
                *slot = None;
                self.inner.phase.store(CLOSED, Ordering::SeqCst);
                Ok(())
            }
            Err(_) => {
                self.inner.phase.store(CLOSED, Ordering::SeqCst);
                Err(to_napi(CoreError::new(ErrorCode::TimeoutClose)))
            }
        }
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

    Ok(OutgoingMessage {
        from: Mailbox::parse(&options.from)?,
        to: mailboxes(options.to)?,
        cc: mailboxes(options.cc)?,
        bcc: mailboxes(options.bcc)?,
        reply_to: mailboxes(options.reply_to)?,
        subject: options.subject,
        text: options.text,
        html: options.html,
        headers,
        attachments: Vec::new(),
    })
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
