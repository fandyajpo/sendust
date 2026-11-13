//! Deterministic validation ordering.
//!
//! Validation runs in a fixed stage order and stops at the first failure, so
//! the same invalid input always produces the same error. Modules report their
//! failure with the stage that owns the rule; the ordering itself is owned here
//! and is the single source of truth for error precedence.

use crate::errors::Error;

/// Validation stages in their approved precedence order (P02-002).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ValidationStage {
    TransportConfig = 0,
    MessageIdentity = 1,
    Recipients = 2,
    Headers = 3,
    BodyEncoding = 4,
    AttachmentMetadata = 5,
    AttachmentBytes = 6,
    WireSize = 7,
}

impl ValidationStage {
    /// Every stage in precedence order.
    pub const ALL: &'static [ValidationStage] = &[
        Self::TransportConfig,
        Self::MessageIdentity,
        Self::Recipients,
        Self::Headers,
        Self::BodyEncoding,
        Self::AttachmentMetadata,
        Self::AttachmentBytes,
        Self::WireSize,
    ];

    /// Stable diagnostic name for this stage.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TransportConfig => "transport_config",
            Self::MessageIdentity => "message_identity",
            Self::Recipients => "recipients",
            Self::Headers => "headers",
            Self::BodyEncoding => "body_encoding",
            Self::AttachmentMetadata => "attachment_metadata",
            Self::AttachmentBytes => "attachment_bytes",
            Self::WireSize => "wire_size",
        }
    }
}

/// Stage-tagged failures retained for deterministic precedence.
#[derive(Debug, Default)]
pub struct Failures {
    entries: Vec<(ValidationStage, Error)>,
}

impl Failures {
    pub fn record(&mut self, stage: ValidationStage, error: Error) {
        self.entries.push((stage, error));
    }

    /// The earliest stage wins; at equal stages the first recorded failure wins.
    pub fn first(&self) -> Option<&Error> {
        self.entries
            .iter()
            .min_by_key(|(stage, _)| *stage)
            .map(|(_, error)| error)
    }

    pub fn into_error(self) -> Option<Error> {
        self.first().cloned()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

/// Runs every check and returns the failure of the earliest stage.
///
/// Checks may be evaluated in any order; the returned error never depends on
/// that order unless two failures share a stage, where the first one recorded
/// is kept.
pub fn validate_in_order<I>(checks: I) -> Result<(), Error>
where
    I: IntoIterator<Item = (ValidationStage, Result<(), Error>)>,
{
    let mut failures = Failures::default();
    for (stage, outcome) in checks {
        if let Err(error) = outcome {
            failures.record(stage, error);
        }
    }
    failures.into_error().map_or(Ok(()), Err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::ErrorCode;

    fn error(code: ErrorCode) -> Error {
        Error::new(code)
    }

    #[test]
    fn stages_are_ordered_by_the_approved_precedence() {
        for pair in ValidationStage::ALL.windows(2) {
            assert!(
                pair[0] < pair[1],
                "{} must precede {}",
                pair[0].as_str(),
                pair[1].as_str()
            );
        }
    }

    #[test]
    fn earliest_stage_wins_regardless_of_evaluation_order() {
        let result = validate_in_order(vec![
            (
                ValidationStage::Headers,
                Err(error(ErrorCode::ValidationInvalidHeader)),
            ),
            (
                ValidationStage::TransportConfig,
                Err(error(ErrorCode::ConfigInvalidRelay)),
            ),
            (
                ValidationStage::Recipients,
                Err(error(ErrorCode::LimitRecipients)),
            ),
        ]);

        let error = result.expect_err("a failure must be returned");
        assert_eq!(error.code(), ErrorCode::ConfigInvalidRelay);
    }

    #[test]
    fn every_adjacent_stage_pair_prefers_the_earlier_stage() {
        for pair in ValidationStage::ALL.windows(2) {
            let result = validate_in_order(vec![
                (pair[1], Err(error(ErrorCode::ValidationInvalidHeader))),
                (pair[0], Err(error(ErrorCode::ConfigInvalidRelay))),
            ]);
            let error = result.expect_err("a failure must be returned");
            assert_eq!(
                error.code(),
                ErrorCode::ConfigInvalidRelay,
                "{} must win over {}",
                pair[0].as_str(),
                pair[1].as_str()
            );
        }
    }

    #[test]
    fn failures_in_the_same_stage_keep_the_first_recorded() {
        let result = validate_in_order(vec![
            (
                ValidationStage::Recipients,
                Err(error(ErrorCode::ValidationMissingRecipient)),
            ),
            (
                ValidationStage::Recipients,
                Err(error(ErrorCode::LimitRecipients)),
            ),
        ]);
        assert_eq!(
            result.expect_err("a failure must be returned").code(),
            ErrorCode::ValidationMissingRecipient
        );
    }

    #[test]
    fn validates_when_every_check_passes() {
        let result = validate_in_order(vec![
            (ValidationStage::TransportConfig, Ok(())),
            (ValidationStage::Recipients, Ok(())),
        ]);
        assert!(result.is_ok());
    }

    #[test]
    fn failures_are_inspectable_before_conversion() {
        let mut failures = Failures::default();
        assert!(failures.is_empty());
        failures.record(ValidationStage::WireSize, error(ErrorCode::LimitWireBytes));
        failures.record(
            ValidationStage::Headers,
            error(ErrorCode::ValidationHeaderInjection),
        );
        assert_eq!(failures.len(), 2);
        assert_eq!(
            failures.first().expect("a failure").code(),
            ErrorCode::ValidationHeaderInjection
        );
        assert_eq!(
            failures.into_error().expect("a failure").code(),
            ErrorCode::ValidationHeaderInjection
        );
    }
}
