//! Relay, TLS, authentication, timeout, limit, and policy configuration values.
//!
//! Hard caps come from ADR-0009. A transport may lower a limit but must never
//! raise it above the approved maximum; `Limits::with_*` enforces that rule.

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
