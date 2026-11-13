//! Relay, TLS, authentication, timeout, limit, and policy configuration values.
//!
//! Hard caps come from ADR-0009. A transport may lower a limit but must never
//! raise it above the approved maximum; `Limits::with_*` enforces that rule.

use std::time::Duration;

use crate::errors::{Error, ErrorCode};

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
