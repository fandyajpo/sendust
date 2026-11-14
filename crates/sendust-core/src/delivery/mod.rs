//! Send use-case orchestration and delivery-certainty mapping.
//!
//! Orchestration validates, composes, and submits exactly once. It owns no
//! network code, no retry, and no queue: a failed or ambiguous send is returned
//! to the caller as an outcome or an error.

use crate::attachments::AttachmentPolicy;
use crate::config::Limits;
use crate::errors::Error;
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

    /// Validates and serializes the message, then submits it exactly once.
    ///
    /// A validation or limit failure returns before the port is called, so no
    /// connection can be attempted for an invalid message.
    pub async fn send(&mut self, message: OutgoingMessage) -> Result<SendOutcome, Error> {
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
        let mut delivery = Delivery::new(port, Limits::default(), AttachmentPolicy::disabled);

        let outcome = delivery.send(message()).await.expect("sent");

        assert_eq!(outcome.status().as_str(), "unknown");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let bytes = wire.lock().expect("lock").clone();
        assert!(String::from_utf8_lossy(&bytes).contains("Subject: Hello"));
    }

    #[tokio::test]
    async fn returns_a_rejected_outcome_unchanged() {
        let (port, calls, _) = fake(Some((451, "4.3.0 Try later")));
        let mut delivery = Delivery::new(port, Limits::default(), AttachmentPolicy::disabled);

        let outcome = delivery.send(message()).await.expect("sent");

        assert_eq!(outcome.status().as_str(), "rejected");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn a_validation_failure_never_reaches_the_port() {
        let (port, calls, _) = fake(None);
        let mut delivery = Delivery::new(port, Limits::default(), AttachmentPolicy::disabled);
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
        let mut delivery = Delivery::new(port, limits, AttachmentPolicy::disabled);

        let error = delivery.send(message()).await.expect_err("limit");

        assert_eq!(error.code(), ErrorCode::LimitWireBytes);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
}
