//! Relay-acceptance results and per-recipient outcomes.
//!
//! An outcome is a normal return value describing what the relay did. It is not
//! an error, and `accepted` means the relay accepted responsibility, never that
//! a mailbox received the message.

use std::sync::Mutex;
use std::sync::PoisonError;
use std::sync::atomic::{AtomicU8, Ordering};

use crate::errors::Stage;

/// Overall delivery status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryStatus {
    Accepted,
    Rejected,
    Unknown,
}

impl DeliveryStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Unknown => "unknown",
        }
    }
}

/// Outcome for one recipient of the envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipientStatus {
    Accepted,
    Rejected,
    Unknown,
}

impl RecipientStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Unknown => "unknown",
        }
    }
}

/// One recipient and what the relay said about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipientOutcome {
    pub address: String,
    pub status: RecipientStatus,
}

impl RecipientOutcome {
    pub fn new(address: impl Into<String>, status: RecipientStatus) -> Self {
        Self {
            address: address.into(),
            status,
        }
    }
}

/// Final SMTP response, when one was observed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmtpResponse {
    pub code: u16,
    pub message: String,
}

/// Order of the coarse transaction stages, from connect to cleanup.
const fn stage_rank(stage: Stage) -> u8 {
    match stage {
        Stage::ConnectionSetup => 0,
        Stage::Auth => 1,
        Stage::Mail => 2,
        Stage::Rcpt => 3,
        Stage::DataCommand => 4,
        Stage::DataBody => 5,
        Stage::Cleanup => 6,
    }
}

/// Progress of one send, observable by the lifecycle layer.
///
/// The adapter records the coarsest stage it has reached and every recipient the
/// relay accepted at RCPT. When a close cancels the send, the lifecycle layer
/// uses this to choose between a definitive cancellation (transmission had not
/// begun) and an honest `unknown` for the accepted subset.
#[derive(Debug, Default)]
pub struct DeliveryProgress {
    stage: AtomicU8,
    accepted: Mutex<Vec<String>>,
}

impl DeliveryProgress {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a stage; stages never move backwards.
    pub fn set_stage(&self, stage: Stage) {
        self.stage
            .fetch_max(stage_rank(stage), Ordering::SeqCst);
    }

    /// Records one recipient the relay accepted at `RCPT`.
    pub fn accept_recipient(&self, address: &str) {
        self.accepted
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(address.to_string());
    }

    /// True once the message body could have reached the relay.
    pub fn transmission_began(&self) -> bool {
        self.stage.load(Ordering::SeqCst) >= stage_rank(Stage::DataBody)
    }

    /// Outcome for a send cancelled after data transmission began.
    ///
    /// `None` when transmission had not begun, so the caller reports the
    /// definitive cancellation instead of inventing an ambiguous outcome.
    pub fn cancelled_outcome(&self) -> Option<SendOutcome> {
        if !self.transmission_began() {
            return None;
        }
        let accepted = self
            .accepted
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        if accepted.is_empty() {
            return None;
        }
        Some(SendOutcome::Unknown {
            recipients: accepted
                .into_iter()
                .map(|address| RecipientOutcome::new(address, RecipientStatus::Unknown))
                .collect(),
            stage: AmbiguityStage::FinalResponse,
        })
    }
}

/// Where the final disposition became ambiguous after DATA transmission began.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmbiguityStage {
    DataBody,
    FinalResponse,
}

/// What the relay did with the message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendOutcome {
    Accepted {
        recipients: Vec<RecipientOutcome>,
        response: Option<SmtpResponse>,
    },
    Rejected {
        recipients: Vec<RecipientOutcome>,
        response: Option<SmtpResponse>,
    },
    Unknown {
        recipients: Vec<RecipientOutcome>,
        stage: AmbiguityStage,
    },
}

impl SendOutcome {
    pub fn status(&self) -> DeliveryStatus {
        match self {
            Self::Accepted { .. } => DeliveryStatus::Accepted,
            Self::Rejected { .. } => DeliveryStatus::Rejected,
            Self::Unknown { .. } => DeliveryStatus::Unknown,
        }
    }

    pub fn recipients(&self) -> &[RecipientOutcome] {
        match self {
            Self::Accepted { recipients, .. }
            | Self::Rejected { recipients, .. }
            | Self::Unknown { recipients, .. } => recipients,
        }
    }

    pub fn response(&self) -> Option<&SmtpResponse> {
        match self {
            Self::Accepted { response, .. } | Self::Rejected { response, .. } => response.as_ref(),
            Self::Unknown { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recipient(address: &str, status: RecipientStatus) -> RecipientOutcome {
        RecipientOutcome::new(address, status)
    }

    #[test]
    fn accepted_reports_relay_acceptance() {
        let outcome = SendOutcome::Accepted {
            recipients: vec![recipient("bob@example.test", RecipientStatus::Accepted)],
            response: Some(SmtpResponse {
                code: 250,
                message: "2.0.0 Message accepted".to_string(),
            }),
        };

        assert_eq!(outcome.status(), DeliveryStatus::Accepted);
        assert_eq!(outcome.status().as_str(), "accepted");
        assert_eq!(outcome.recipients().len(), 1);
        assert_eq!(outcome.response().expect("response").code, 250);
    }

    #[test]
    fn rejected_keeps_per_recipient_detail() {
        let outcome = SendOutcome::Rejected {
            recipients: vec![
                recipient("ok@example.test", RecipientStatus::Rejected),
                recipient("bad@example.test", RecipientStatus::Rejected),
            ],
            response: Some(SmtpResponse {
                code: 451,
                message: "4.3.0 Try later".to_string(),
            }),
        };

        assert_eq!(outcome.status(), DeliveryStatus::Rejected);
        assert_eq!(outcome.recipients()[0].status, RecipientStatus::Rejected);
        assert_eq!(outcome.response().expect("response").code, 451);
    }

    #[test]
    fn unknown_carries_the_ambiguity_stage_and_no_response() {
        let outcome = SendOutcome::Unknown {
            recipients: vec![recipient("bob@example.test", RecipientStatus::Unknown)],
            stage: AmbiguityStage::FinalResponse,
        };

        assert_eq!(outcome.status(), DeliveryStatus::Unknown);
        assert!(outcome.response().is_none());
        assert_eq!(outcome.recipients()[0].status.as_str(), "unknown");
    }

    #[test]
    fn status_strings_match_the_public_contract() {
        assert_eq!(DeliveryStatus::Accepted.as_str(), "accepted");
        assert_eq!(DeliveryStatus::Rejected.as_str(), "rejected");
        assert_eq!(DeliveryStatus::Unknown.as_str(), "unknown");
    }

    #[test]
    fn a_cancellation_before_data_is_not_ambiguous() {
        let progress = DeliveryProgress::new();
        progress.set_stage(Stage::Rcpt);
        progress.accept_recipient("bob@example.test");

        assert!(!progress.transmission_began());
        assert!(progress.cancelled_outcome().is_none());
    }

    #[test]
    fn a_cancellation_after_data_reports_unknown_for_the_accepted_subset() {
        let progress = DeliveryProgress::new();
        progress.set_stage(Stage::DataCommand);
        progress.accept_recipient("bob@example.test");
        progress.accept_recipient("carol@example.test");
        progress.set_stage(Stage::DataBody);

        let outcome = progress.cancelled_outcome().expect("ambiguous");
        assert_eq!(outcome.status(), DeliveryStatus::Unknown);
        assert_eq!(outcome.recipients().len(), 2);
        assert!(
            outcome
                .recipients()
                .iter()
                .all(|recipient| recipient.status == RecipientStatus::Unknown)
        );
    }

    #[test]
    fn stages_never_move_backwards() {
        let progress = DeliveryProgress::new();
        progress.set_stage(Stage::DataBody);
        progress.set_stage(Stage::ConnectionSetup);

        assert!(progress.transmission_began());
        progress.accept_recipient("bob@example.test");
        assert!(progress.cancelled_outcome().is_some());
    }
}
