//! Send use-case orchestration and delivery-certainty mapping.
//!
//! Orchestration validates, composes, and submits exactly once. It owns no
//! network code, no retry, and no queue: a failed or ambiguous send is returned
//! to the caller as an outcome or an error.

use crate::attachments::{Attachment, AttachmentPolicy, normalize_filename, read_attachment};
use crate::config::Limits;
use crate::errors::{Error, ErrorCode};
use crate::message::OutgoingMessage;
use crate::mime;
use crate::ports::{DeliveryPort, DeliveryRequest};
use crate::results::SendOutcome;

/// Drives one send through a delivery port.
pub struct Delivery<P> {
    port: P,
    limits: Limits,
    attachment_policy: AttachmentPolicy,
}

impl<P: DeliveryPort> Delivery<P> {
    pub fn new(port: P, limits: Limits, attachment_policy: AttachmentPolicy) -> Self {
        Self {
            port,
            limits,
            attachment_policy,
        }
    }

    pub fn port(&self) -> &P {
        &self.port
    }

    /// Loads file attachments, validates and serializes the message, then submits
    /// it exactly once.
    ///
    /// A validation or limit failure returns before the port is called, so no
    /// connection can be attempted for an invalid message.
    pub async fn send(&mut self, message: OutgoingMessage) -> Result<SendOutcome, Error> {
        // Message stages run before any file is opened, so the approved error
        // precedence holds: identity and recipients outrank attachment reads.
        message.validate(self.limits, self.attachment_policy)?;
        let message = load_attachments(message, self.limits, self.attachment_policy)?;
        let wire = mime::serialize_at(
            &message,
            self.limits,
            self.attachment_policy,
            std::time::SystemTime::now(),
        )?;
        let request = DeliveryRequest { message, wire };
        self.port.deliver(request).await
    }
}

/// Replaces file attachments with the bytes read from disk.
///
/// Descriptor checks and the aggregate cap of already-known bytes run first;
/// each file is then read within the per-file cap, and the running total is
/// checked so many small files cannot add up past the aggregate cap unnoticed.
fn load_attachments(
    mut message: OutgoingMessage,
    limits: Limits,
    policy: AttachmentPolicy,
) -> Result<OutgoingMessage, Error> {
    if message.attachments.is_empty() {
        return Ok(message);
    }

    let mut total = crate::attachments::validate(&message.attachments, limits, policy)?;
    let mut loaded = Vec::with_capacity(message.attachments.len());

    for attachment in std::mem::take(&mut message.attachments) {
        match attachment {
            Attachment::Bytes { .. } => loaded.push(attachment),
            Attachment::File(file) => {
                let bytes = read_attachment(&file, limits)?;
                total = total.saturating_add(bytes.len() as u64);
                if total > limits.max_total_attachment_bytes() {
                    return Err(Error::new(ErrorCode::LimitAttachmentTotal));
                }
                let filename = match &file.display_name {
                    Some(name) => name.clone(),
                    None => normalize_filename(&file.path)?,
                };
                loaded.push(Attachment::Bytes {
                    filename,
                    content_type: file.content_type,
                    bytes,
                });
            }
        }
    }

    message.attachments = loaded;
    Ok(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address::Mailbox;
    use crate::errors::ErrorCode;
    use crate::ports::DeliveryFuture;
    use crate::results::{AmbiguityStage, RecipientOutcome, RecipientStatus, SmtpResponse};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct FakePort {
        calls: Arc<AtomicUsize>,
        last_wire: Arc<std::sync::Mutex<Vec<u8>>>,
        outcome: Option<(u16, &'static str)>,
    }

    impl DeliveryPort for FakePort {
        fn deliver<'a>(&'a mut self, request: DeliveryRequest) -> DeliveryFuture<'a> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            *self.last_wire.lock().expect("lock") = request.wire.clone();
            let recipients = vec![RecipientOutcome::new(
                "bob@example.test",
                RecipientStatus::Accepted,
            )];
            let outcome = match self.outcome {
                Some((code, message)) => SendOutcome::Rejected {
                    recipients,
                    response: Some(SmtpResponse {
                        code,
                        message: message.to_string(),
                    }),
                },
                None => SendOutcome::Unknown {
                    recipients,
                    stage: AmbiguityStage::FinalResponse,
                },
            };
            Box::pin(async move { Ok(outcome) })
        }
    }

    fn mailbox(value: &str) -> Mailbox {
        Mailbox::parse(value).expect("valid mailbox")
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

    fn fake(
        outcome: Option<(u16, &'static str)>,
    ) -> (FakePort, Arc<AtomicUsize>, Arc<std::sync::Mutex<Vec<u8>>>) {
        let calls = Arc::new(AtomicUsize::new(0));
        let wire = Arc::new(std::sync::Mutex::new(Vec::new()));
        let port = FakePort {
            calls: Arc::clone(&calls),
            last_wire: Arc::clone(&wire),
            outcome,
        };
        (port, calls, wire)
    }

    #[tokio::test]
    async fn submits_once_with_serialized_wire_bytes() {
        let (port, calls, wire) = fake(None);
        let mut delivery = Delivery::new(port, Limits::default(), AttachmentPolicy::Disabled);

        let outcome = delivery.send(message()).await.expect("sent");

        assert_eq!(outcome.status().as_str(), "unknown");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let bytes = wire.lock().expect("lock").clone();
        assert!(String::from_utf8_lossy(&bytes).contains("Subject: Hello"));
    }

    #[tokio::test]
    async fn returns_a_rejected_outcome_unchanged() {
        let (port, calls, _) = fake(Some((451, "4.3.0 Try later")));
        let mut delivery = Delivery::new(port, Limits::default(), AttachmentPolicy::Disabled);

        let outcome = delivery.send(message()).await.expect("sent");

        assert_eq!(outcome.status().as_str(), "rejected");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn a_validation_failure_never_reaches_the_port() {
        let (port, calls, _) = fake(None);
        let mut delivery = Delivery::new(port, Limits::default(), AttachmentPolicy::Disabled);
        let mut invalid = message();
        invalid.to.clear();

        let error = delivery.send(invalid).await.expect_err("invalid");

        assert_eq!(error.code(), ErrorCode::ValidationMissingRecipient);
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "no dial on validation failure"
        );
    }

    #[tokio::test]
    async fn a_limit_failure_never_reaches_the_port() {
        let (port, calls, _) = fake(None);
        let limits = Limits::default()
            .with_max_wire_bytes(10)
            .expect("lowered limit");
        let mut delivery = Delivery::new(port, limits, AttachmentPolicy::Disabled);

        let error = delivery.send(message()).await.expect_err("limit");

        assert_eq!(error.code(), ErrorCode::LimitWireBytes);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    fn temp_file(name: &str, len: usize) -> std::path::PathBuf {
        use std::sync::atomic::AtomicU64;

        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "sendust-delivery-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join(name);
        std::fs::write(&path, vec![b'a'; len]).expect("write fixture");
        path
    }

    fn remove_temp(path: &std::path::Path) {
        if let Some(dir) = path.parent() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    fn file_attachment(path: &std::path::Path, name: &str) -> Attachment {
        Attachment::File(crate::attachments::FileDescriptor {
            display_name: Some(name.to_string()),
            content_type: Some("text/plain".to_string()),
            path: path.to_string_lossy().into_owned(),
        })
    }

    #[tokio::test]
    async fn a_message_error_precedes_a_file_read_error() {
        let path = temp_file("gone.bin", 3);
        let missing = path.with_file_name("absent.bin");
        let mut message = message();
        message.to.clear();
        message.attachments = vec![file_attachment(&missing, "absent.bin")];

        let (port, calls, _) = fake(None);
        let mut delivery = Delivery::new(port, Limits::default(), AttachmentPolicy::Enabled);
        let error = delivery.send(message).await.expect_err("no recipient");

        assert_eq!(error.code(), ErrorCode::ValidationMissingRecipient);
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        remove_temp(&path);
    }

    #[tokio::test]
    async fn file_attachments_are_loaded_into_the_wire() {
        let path = temp_file("notes.txt", 5);
        let mut message = message();
        message.attachments = vec![file_attachment(&path, "notes.txt")];

        let (port, calls, wire) = fake(None);
        let mut delivery = Delivery::new(port, Limits::default(), AttachmentPolicy::Enabled);
        delivery.send(message).await.expect("sent");

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let bytes = wire.lock().expect("lock").clone();
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("filename="), "{text}");
        assert!(text.contains("notes.txt"), "{text}");
        assert!(text.contains("YWFhYWE="), "base64 body missing: {text}");

        remove_temp(&path);
    }

    #[tokio::test]
    async fn an_oversized_file_never_reaches_the_port() {
        let path = temp_file("big.bin", 5);
        let limits = Limits::default()
            .with_max_attachment_bytes(4)
            .expect("lowering is allowed");
        let mut message = message();
        message.attachments = vec![file_attachment(&path, "big.bin")];

        let (port, calls, _) = fake(None);
        let mut delivery = Delivery::new(port, limits, AttachmentPolicy::Enabled);
        let error = delivery.send(message).await.expect_err("over the cap");

        assert_eq!(error.code(), ErrorCode::LimitAttachmentBytes);
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        remove_temp(&path);
    }

    #[tokio::test]
    async fn an_aggregate_over_the_limit_never_reaches_the_port() {
        let first = temp_file("a.bin", 3);
        let second = temp_file("b.bin", 3);
        let limits = Limits::default()
            .with_max_total_attachment_bytes(4)
            .expect("lowering is allowed");
        let mut message = message();
        message.attachments = vec![
            file_attachment(&first, "a.bin"),
            file_attachment(&second, "b.bin"),
        ];

        let (port, calls, _) = fake(None);
        let mut delivery = Delivery::new(port, limits, AttachmentPolicy::Enabled);
        let error = delivery.send(message).await.expect_err("over the total");

        assert_eq!(error.code(), ErrorCode::LimitAttachmentTotal);
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        remove_temp(&first);
        remove_temp(&second);
    }

    #[tokio::test]
    async fn the_wire_cap_counts_attachment_encoding() {
        let path = temp_file("one.bin", 1024);
        let limits = Limits::default()
            .with_max_wire_bytes(1024)
            .expect("lowering is allowed");
        let mut message = message();
        message.attachments = vec![file_attachment(&path, "one.bin")];

        let (port, calls, _) = fake(None);
        let mut delivery = Delivery::new(port, limits, AttachmentPolicy::Enabled);
        let error = delivery.send(message).await.expect_err("wire over the cap");

        assert_eq!(error.code(), ErrorCode::LimitWireBytes);
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        remove_temp(&path);
    }
}
