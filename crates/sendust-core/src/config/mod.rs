//! Relay, TLS, authentication, timeout, limit, and policy configuration values.
//!
//! Hard caps come from ADR-0009. A transport may lower a limit but must never
//! raise it above the approved maximum; `Limits::with_*` enforces that rule.

use std::time::{Duration, Instant};

use crate::errors::{Error, ErrorCode, ErrorDetails, Stage};

/// One mebibyte, the unit used by the approved caps.
pub const MIB: u64 = 1024 * 1024;

/// Hard caps for recipients, attachment bytes, and wire size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    max_recipients: usize,
    max_attachment_bytes: u64,
    max_total_attachment_bytes: u64,
    max_wire_bytes: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_recipients: 100,
            max_attachment_bytes: 10 * MIB,
            max_total_attachment_bytes: 20 * MIB,
            max_wire_bytes: 30 * MIB,
        }
    }
}

impl Limits {
    pub fn max_recipients(&self) -> usize {
        self.max_recipients
    }

    pub fn max_attachment_bytes(&self) -> u64 {
        self.max_attachment_bytes
    }

    pub fn max_total_attachment_bytes(&self) -> u64 {
        self.max_total_attachment_bytes
    }

    pub fn max_wire_bytes(&self) -> u64 {
        self.max_wire_bytes
    }

    /// Lowers the recipient cap. Raising it above the approved maximum fails.
    pub fn with_max_recipients(mut self, value: usize) -> Result<Self, Error> {
        let approved = Limits::default().max_recipients;
        if value == 0 || value > approved {
            return Err(Error::new(ErrorCode::ValidationInvalidOption));
        }
        self.max_recipients = value;
        Ok(self)
    }

    /// Lowers the per-attachment cap. Raising it above the approved maximum fails.
    pub fn with_max_attachment_bytes(mut self, value: u64) -> Result<Self, Error> {
        let approved = Limits::default().max_attachment_bytes;
        if value == 0 || value > approved {
            return Err(Error::new(ErrorCode::ValidationInvalidOption));
        }
        self.max_attachment_bytes = value;
        Ok(self)
    }

    /// Lowers the aggregate attachment cap. Raising it fails.
    pub fn with_max_total_attachment_bytes(mut self, value: u64) -> Result<Self, Error> {
        let approved = Limits::default().max_total_attachment_bytes;
        if value == 0 || value > approved {
            return Err(Error::new(ErrorCode::ValidationInvalidOption));
        }
        self.max_total_attachment_bytes = value;
        Ok(self)
    }

    /// Lowers the wire cap. Raising it above the approved maximum fails.
    pub fn with_max_wire_bytes(mut self, value: u64) -> Result<Self, Error> {
        let approved = Limits::default().max_wire_bytes;
        if value == 0 || value > approved {
            return Err(Error::new(ErrorCode::ValidationInvalidOption));
        }
        self.max_wire_bytes = value;
        Ok(self)
    }
}

/// Smallest accepted budget. A timeout is a whole number of seconds.
const MIN_BUDGET: Duration = Duration::from_secs(1);

/// Connect, operation, total, and close budgets.
///
/// Defaults are hard maximums: a value may be lowered but never raised, connect
/// and operation may not exceed the total, and the close budget must be at
/// least the total so a maximal send can still drain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timeouts {
    connect: Duration,
    operation: Duration,
    total: Duration,
    close: Duration,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self {
            connect: Duration::from_secs(10),
            operation: Duration::from_secs(30),
            total: Duration::from_secs(120),
            close: Duration::from_secs(125),
        }
    }
}

impl Timeouts {
    pub fn connect(&self) -> Duration {
        self.connect
    }

    pub fn operation(&self) -> Duration {
        self.operation
    }

    pub fn total(&self) -> Duration {
        self.total
    }

    pub fn close(&self) -> Duration {
        self.close
    }

    /// Rejects zero, sub-second, raised, and inconsistent budgets.
    pub fn validate(&self) -> Result<(), Error> {
        let defaults = Timeouts::default();

        for budget in [self.connect, self.operation, self.total, self.close] {
            if budget < MIN_BUDGET {
                return Err(invalid_timeout());
            }
        }

        if self.connect > defaults.connect
            || self.operation > defaults.operation
            || self.total > defaults.total
            || self.close > defaults.close
        {
            return Err(invalid_timeout());
        }

        if self.connect > self.total || self.operation > self.total || self.close < self.total {
            return Err(invalid_timeout());
        }

        Ok(())
    }

    pub fn with_connect(mut self, value: Duration) -> Result<Self, Error> {
        self.connect = value;
        self.validated()
    }

    pub fn with_operation(mut self, value: Duration) -> Result<Self, Error> {
        self.operation = value;
        self.validated()
    }

    pub fn with_total(mut self, value: Duration) -> Result<Self, Error> {
        self.total = value;
        self.validated()
    }

    pub fn with_close(mut self, value: Duration) -> Result<Self, Error> {
        self.close = value;
        self.validated()
    }

    fn validated(self) -> Result<Self, Error> {
        self.validate()?;
        Ok(self)
    }
}

fn invalid_timeout() -> Error {
    Error::new(ErrorCode::ValidationInvalidTimeout)
}

/// Per-send deadlines derived from a [`Timeouts`] value and one start instant.
///
/// A single total deadline governs the send, so independent timers cannot
/// overrun it. `operation` bounds each command/response wait; `connect` bounds
/// DNS, TCP, greeting, EHLO, and TLS as one outer boundary.
#[derive(Debug, Clone, Copy)]
pub struct SendDeadlines {
    started: Instant,
    connect_by: Instant,
    total_by: Instant,
    operation: Duration,
}

impl SendDeadlines {
    pub fn start(timeouts: Timeouts, now: Instant) -> Self {
        Self {
            started: now,
            connect_by: now + timeouts.connect(),
            total_by: now + timeouts.total(),
            operation: timeouts.operation(),
        }
    }

    pub fn connect_by(&self) -> Instant {
        self.connect_by
    }

    pub fn total_by(&self) -> Instant {
        self.total_by
    }

    pub fn elapsed(&self, now: Instant) -> Duration {
        now.saturating_duration_since(self.started)
    }

    pub fn remaining_total(&self, now: Instant) -> Duration {
        self.total_by.saturating_duration_since(now)
    }

    pub fn connect_expired(&self, now: Instant) -> bool {
        now >= self.connect_by || now >= self.total_by
    }

    pub fn operation_expired(&self, now: Instant) -> bool {
        now >= self.operation_deadline(now) || now >= self.total_by
    }

    /// Deadline for a single command: the operation budget, capped by the total.
    pub fn operation_deadline(&self, now: Instant) -> Instant {
        std::cmp::min(now + self.operation, self.total_by)
    }

    /// Honest timeout classification: connect only before the session is up.
    pub fn timeout_error(&self, now: Instant, connected: bool, stage: Stage) -> Error {
        let code = if !connected && now >= self.connect_by {
            ErrorCode::TimeoutConnect
        } else if now >= self.total_by {
            ErrorCode::TimeoutTotal
        } else {
            ErrorCode::TimeoutOperation
        };

        Error::new(code).with_details(
            ErrorDetails::default()
                .stage(stage)
                .elapsed_ms(u64::try_from(self.elapsed(now).as_millis()).unwrap_or(u64::MAX)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn defaults_match_the_approved_caps() {
        let limits = Limits::default();
        assert_eq!(limits.max_recipients(), 100);
        assert_eq!(limits.max_attachment_bytes(), 10 * MIB);
        assert_eq!(limits.max_total_attachment_bytes(), 20 * MIB);
        assert_eq!(limits.max_wire_bytes(), 30 * MIB);
    }

    #[test]
    fn limits_can_be_lowered() {
        let limits = Limits::default()
            .with_max_recipients(2)
            .expect("lowering is allowed")
            .with_max_attachment_bytes(MIB)
            .expect("lowering is allowed")
            .with_max_total_attachment_bytes(2 * MIB)
            .expect("lowering is allowed")
            .with_max_wire_bytes(3 * MIB)
            .expect("lowering is allowed");

        assert_eq!(limits.max_recipients(), 2);
        assert_eq!(limits.max_attachment_bytes(), MIB);
        assert_eq!(limits.max_total_attachment_bytes(), 2 * MIB);
        assert_eq!(limits.max_wire_bytes(), 3 * MIB);
    }

    #[test]
    fn timeout_defaults_match_the_approved_budgets() {
        let timeouts = Timeouts::default();
        assert_eq!(timeouts.connect(), Duration::from_secs(10));
        assert_eq!(timeouts.operation(), Duration::from_secs(30));
        assert_eq!(timeouts.total(), Duration::from_secs(120));
        assert_eq!(timeouts.close(), Duration::from_secs(125));
        assert!(timeouts.validate().is_ok());
    }

    #[test]
    fn timeouts_can_be_lowered_consistently() {
        let timeouts = Timeouts::default()
            .with_connect(Duration::from_secs(5))
            .expect("lower ok")
            .with_operation(Duration::from_secs(10))
            .expect("lower ok")
            .with_total(Duration::from_secs(20))
            .expect("lower ok")
            .with_close(Duration::from_secs(25))
            .expect("lower ok");
        assert_eq!(timeouts.total(), Duration::from_secs(20));
        assert_eq!(timeouts.close(), Duration::from_secs(25));
    }

    #[test]
    fn rejects_raised_zero_sub_second_and_inconsistent_timeouts() {
        let default = Timeouts::default();
        let cases = [
            default.with_connect(Duration::from_secs(11)),
            default.with_operation(Duration::from_secs(31)),
            default.with_total(Duration::from_secs(121)),
            default.with_close(Duration::from_secs(126)),
            default.with_connect(Duration::ZERO),
            default.with_connect(Duration::from_millis(999)),
            default.with_total(Duration::from_millis(500)),
            default.with_close(Duration::from_secs(100)),
        ];
        for outcome in cases {
            assert_eq!(
                outcome.expect_err("must be rejected").code(),
                ErrorCode::ValidationInvalidTimeout
            );
        }
    }

    #[test]
    fn connect_and_operation_cannot_exceed_total() {
        let default = Timeouts::default();
        for outcome in [
            default.with_total(Duration::from_secs(5)),
            default.with_total(Duration::from_secs(20)),
        ] {
            assert_eq!(
                outcome.expect_err("must be rejected").code(),
                ErrorCode::ValidationInvalidTimeout
            );
        }
    }

    #[test]
    fn limits_cannot_be_raised_or_zeroed() {
        for outcome in [
            Limits::default().with_max_recipients(101),
            Limits::default().with_max_recipients(0),
            Limits::default().with_max_attachment_bytes(10 * MIB + 1),
            Limits::default().with_max_total_attachment_bytes(20 * MIB + 1),
            Limits::default().with_max_wire_bytes(30 * MIB + 1),
            Limits::default().with_max_attachment_bytes(0),
        ] {
            assert_eq!(
                outcome.expect_err("must be rejected").code(),
                ErrorCode::ValidationInvalidOption
            );
        }
    }

    #[test]
    fn deadlines_use_the_approved_budgets() {
        let now = Instant::now();
        let deadlines = SendDeadlines::start(Timeouts::default(), now);

        assert_eq!(deadlines.connect_by(), now + Duration::from_secs(10));
        assert_eq!(deadlines.total_by(), now + Duration::from_secs(120));
        assert!(
            !deadlines.connect_expired(now + Duration::from_secs(10) - Duration::from_millis(1))
        );
        assert!(deadlines.connect_expired(now + Duration::from_secs(10)));
    }

    #[test]
    fn operation_deadline_is_capped_by_the_total() {
        let now = Instant::now();
        let deadlines = SendDeadlines::start(Timeouts::default(), now);

        assert_eq!(
            deadlines.operation_deadline(now),
            now + Duration::from_secs(30)
        );

        let late = now + Duration::from_secs(100);
        assert_eq!(
            deadlines.operation_deadline(late),
            now + Duration::from_secs(120)
        );
        assert!(deadlines.operation_expired(now + Duration::from_secs(120)));
    }

    #[test]
    fn remaining_total_never_underflows() {
        let now = Instant::now();
        let deadlines = SendDeadlines::start(Timeouts::default(), now);
        assert_eq!(deadlines.remaining_total(now), Duration::from_secs(120));
        assert_eq!(
            deadlines.remaining_total(now + Duration::from_secs(200)),
            Duration::ZERO
        );
    }

    #[test]
    fn timeout_classification_is_honest_about_the_connect_phase() {
        let now = Instant::now();
        let deadlines = SendDeadlines::start(Timeouts::default(), now);

        let connect =
            deadlines.timeout_error(now + Duration::from_secs(10), false, Stage::ConnectionSetup);
        assert_eq!(connect.code(), ErrorCode::TimeoutConnect);
        assert_eq!(
            connect.details().operation_stage(),
            Some(Stage::ConnectionSetup)
        );

        let total = deadlines.timeout_error(now + Duration::from_secs(120), true, Stage::DataBody);
        assert_eq!(total.code(), ErrorCode::TimeoutTotal);

        let operation = deadlines.timeout_error(now + Duration::from_secs(40), true, Stage::Mail);
        assert_eq!(operation.code(), ErrorCode::TimeoutOperation);
    }
}
