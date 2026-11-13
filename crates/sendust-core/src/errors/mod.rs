//! Stable internal error taxonomy.
//!
//! Errors describe why Sendust could not produce a trustworthy delivery
//! outcome; delivery outcomes live in `results` and are not errors.
//!
//! Codes and category strings are the public contract: they are append-only
//! within a major version and messages are diagnostics, never a contract.
//! Construction is restricted to a code plus an explicitly whitelisted detail
//! set, so credentials, message bodies, attachment bytes, and absolute paths
//! cannot leak into an error by accident.

use std::error::Error as StdError;
use std::fmt;

/// Error category, exposed as the public `category` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    Validation,
    Configuration,
    Limit,
    Tls,
    Authentication,
    Protocol,
    Timeout,
    Closed,
    Busy,
    Internal,
}

impl ErrorCategory {
    /// Stable public string for this category.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Validation => "validation",
            Self::Configuration => "configuration",
            Self::Limit => "limit",
            Self::Tls => "tls",
            Self::Authentication => "authentication",
            Self::Protocol => "protocol",
            Self::Timeout => "timeout",
            Self::Closed => "closed",
            Self::Busy => "busy",
            Self::Internal => "internal",
        }
    }
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

macro_rules! error_codes {
    ($($variant:ident => ($code:literal, $category:ident, $message:literal)),+ $(,)?) => {
        /// Stable machine-readable error code.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum ErrorCode {
            $($variant),+
        }

        impl ErrorCode {
            /// Every code, in declaration order. Used by documentation parity tests.
            pub const ALL: &'static [ErrorCode] = &[$(ErrorCode::$variant),+];

            /// Stable public string for this code.
            pub const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $code),+ }
            }

            /// Category this code belongs to.
            pub const fn category(self) -> ErrorCategory {
                match self { $(Self::$variant => ErrorCategory::$category),+ }
            }

            /// Default redacted diagnostic message.
            pub const fn default_message(self) -> &'static str {
                match self { $(Self::$variant => $message),+ }
            }
        }
    };
}

error_codes! {
    ValidationInvalidOption => ("VALIDATION_INVALID_OPTION", Validation, "invalid transport option"),
    ValidationInvalidTimeout => ("VALIDATION_INVALID_TIMEOUT", Validation, "invalid timeout value"),
    ValidationMissingFrom => ("VALIDATION_MISSING_FROM", Validation, "missing From address"),
    ValidationMissingRecipient => ("VALIDATION_MISSING_RECIPIENT", Validation, "missing recipient"),
    ValidationInvalidAddress => ("VALIDATION_INVALID_ADDRESS", Validation, "invalid address"),
    ValidationNonAsciiLocalPart => (
        "VALIDATION_NON_ASCII_LOCAL_PART",
        Validation,
        "non-ASCII local part is not supported"
    ),
    ValidationInvalidHeader => ("VALIDATION_INVALID_HEADER", Validation, "invalid header"),
    ValidationHeaderInjection => (
        "VALIDATION_HEADER_INJECTION",
        Validation,
        "header or address contains control characters"
    ),
    ValidationInvalidAttachment => (
        "VALIDATION_INVALID_ATTACHMENT",
        Validation,
        "invalid attachment"
    ),
    ConfigUnsupportedSecurity => ("CONFIG_UNSUPPORTED_SECURITY", Configuration, "unsupported security mode"),
    ConfigUnsupportedAuth => ("CONFIG_UNSUPPORTED_AUTH", Configuration, "unsupported authentication mode"),
    ConfigFileAccessDisabled => (
        "CONFIG_FILE_ACCESS_DISABLED",
        Configuration,
        "file attachments are disabled"
    ),
    ConfigInvalidRelay => ("CONFIG_INVALID_RELAY", Configuration, "invalid relay configuration"),
    LimitRecipients => ("LIMIT_RECIPIENTS", Limit, "too many recipients"),
    LimitAttachmentBytes => ("LIMIT_ATTACHMENT_BYTES", Limit, "attachment exceeds the per-file limit"),
    LimitAttachmentTotal => ("LIMIT_ATTACHMENT_TOTAL", Limit, "attachments exceed the aggregate limit"),
    LimitWireBytes => ("LIMIT_WIRE_BYTES", Limit, "message exceeds the wire limit"),
    TlsCertificate => ("TLS_CERTIFICATE", Tls, "TLS certificate verification failed"),
    TlsHandshake => ("TLS_HANDSHAKE", Tls, "TLS handshake failed"),
    TlsProtocol => ("TLS_PROTOCOL", Tls, "TLS protocol not supported"),
    AuthFailed => ("AUTH_FAILED", Authentication, "authentication failed"),
    AuthUnsupported => ("AUTH_UNSUPPORTED", Authentication, "no supported authentication mechanism"),
    ProtocolRejected => ("PROTOCOL_REJECTED", Protocol, "protocol rejected the message"),
    ProtocolUnexpectedResponse => ("PROTOCOL_UNEXPECTED_RESPONSE", Protocol, "unexpected SMTP response"),
    ProtocolConnectionReset => ("PROTOCOL_CONNECTION_RESET", Protocol, "connection reset by the relay"),
    TimeoutConnect => ("TIMEOUT_CONNECT", Timeout, "connect timeout"),
    TimeoutOperation => ("TIMEOUT_OPERATION", Timeout, "operation timeout"),
    TimeoutTotal => ("TIMEOUT_TOTAL", Timeout, "total send timeout"),
    TimeoutClose => ("TIMEOUT_CLOSE", Timeout, "close timeout"),
    TransportClosed => ("TRANSPORT_CLOSED", Closed, "transport is closed"),
    TransportBusy => ("TRANSPORT_BUSY", Busy, "transport is busy"),
    Internal => ("INTERNAL", Internal, "internal error"),
}

/// Coarse operation boundary reported with a timeout or protocol error.
///
/// Only boundaries proven observable at P01-003 are reported; internal socket
/// sub-stages are never inferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stage {
    ConnectionSetup,
    Auth,
    Mail,
    Rcpt,
    DataCommand,
    DataBody,
    Cleanup,
}

impl Stage {
    /// Stable public string for this stage.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ConnectionSetup => "connection_setup",
            Self::Auth => "auth",
            Self::Mail => "mail",
            Self::Rcpt => "rcpt",
            Self::DataCommand => "data_command",
            Self::DataBody => "data_body",
            Self::Cleanup => "cleanup",
        }
    }
}

/// Whitelisted error details. Nothing outside these fields can reach a caller.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ErrorDetails {
    host: Option<String>,
    port: Option<u16>,
    timeout_ms: Option<u64>,
    elapsed_ms: Option<u64>,
    stage: Option<Stage>,
    recipient: Option<String>,
    smtp_code: Option<u16>,
    smtp_enhanced_code: Option<String>,
}

impl ErrorDetails {
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub fn timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    pub fn elapsed_ms(mut self, elapsed_ms: u64) -> Self {
        self.elapsed_ms = Some(elapsed_ms);
        self
    }

    pub fn stage(mut self, stage: Stage) -> Self {
        self.stage = Some(stage);
        self
    }

    pub fn recipient(mut self, recipient: impl Into<String>) -> Self {
        self.recipient = Some(recipient.into());
        self
    }

    pub fn smtp_code(mut self, smtp_code: u16) -> Self {
        self.smtp_code = Some(smtp_code);
        self
    }

    pub fn smtp_enhanced_code(mut self, smtp_enhanced_code: impl Into<String>) -> Self {
        self.smtp_enhanced_code = Some(smtp_enhanced_code.into());
        self
    }

    pub fn host_name(&self) -> Option<&str> {
        self.host.as_deref()
    }

    pub fn port_number(&self) -> Option<u16> {
        self.port
    }

    pub fn timeout_millis(&self) -> Option<u64> {
        self.timeout_ms
    }

    pub fn elapsed_millis(&self) -> Option<u64> {
        self.elapsed_ms
    }

    pub fn operation_stage(&self) -> Option<Stage> {
        self.stage
    }

    pub fn recipient_address(&self) -> Option<&str> {
        self.recipient.as_deref()
    }

    pub fn smtp_status(&self) -> Option<u16> {
        self.smtp_code
    }

    pub fn smtp_enhanced(&self) -> Option<&str> {
        self.smtp_enhanced_code.as_deref()
    }
}

/// A core error: a stable code, a redacted message, and whitelisted details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    code: ErrorCode,
    message: String,
    details: Box<ErrorDetails>,
}

impl Error {
    pub fn new(code: ErrorCode) -> Self {
        Self {
            code,
            message: code.default_message().to_string(),
            details: Box::new(ErrorDetails::default()),
        }
    }

    pub fn with_details(mut self, details: ErrorDetails) -> Self {
        self.details = Box::new(details);
        self
    }

    pub fn code(&self) -> ErrorCode {
        self.code
    }

    pub fn category(&self) -> ErrorCategory {
        self.code.category()
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn details(&self) -> &ErrorDetails {
        &self.details
    }
}

impl From<ErrorCode> for Error {
    fn from(code: ErrorCode) -> Self {
        Self::new(code)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl StdError for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Documentation parity table: code string and category from
    /// docs/architecture/error-model.md section 3.1.
    const EXPECTED: &[(&str, &str)] = &[
        ("VALIDATION_INVALID_OPTION", "validation"),
        ("VALIDATION_INVALID_TIMEOUT", "validation"),
        ("VALIDATION_MISSING_FROM", "validation"),
        ("VALIDATION_MISSING_RECIPIENT", "validation"),
        ("VALIDATION_INVALID_ADDRESS", "validation"),
        ("VALIDATION_NON_ASCII_LOCAL_PART", "validation"),
        ("VALIDATION_INVALID_HEADER", "validation"),
        ("VALIDATION_HEADER_INJECTION", "validation"),
        ("VALIDATION_INVALID_ATTACHMENT", "validation"),
        ("CONFIG_UNSUPPORTED_SECURITY", "configuration"),
        ("CONFIG_UNSUPPORTED_AUTH", "configuration"),
        ("CONFIG_FILE_ACCESS_DISABLED", "configuration"),
        ("CONFIG_INVALID_RELAY", "configuration"),
        ("LIMIT_RECIPIENTS", "limit"),
        ("LIMIT_ATTACHMENT_BYTES", "limit"),
        ("LIMIT_ATTACHMENT_TOTAL", "limit"),
        ("LIMIT_WIRE_BYTES", "limit"),
        ("TLS_CERTIFICATE", "tls"),
        ("TLS_HANDSHAKE", "tls"),
        ("TLS_PROTOCOL", "tls"),
        ("AUTH_FAILED", "authentication"),
        ("AUTH_UNSUPPORTED", "authentication"),
        ("PROTOCOL_REJECTED", "protocol"),
        ("PROTOCOL_UNEXPECTED_RESPONSE", "protocol"),
        ("PROTOCOL_CONNECTION_RESET", "protocol"),
        ("TIMEOUT_CONNECT", "timeout"),
        ("TIMEOUT_OPERATION", "timeout"),
        ("TIMEOUT_TOTAL", "timeout"),
        ("TIMEOUT_CLOSE", "timeout"),
        ("TRANSPORT_CLOSED", "closed"),
        ("TRANSPORT_BUSY", "busy"),
        ("INTERNAL", "internal"),
    ];

    #[test]
    fn every_code_is_documented_and_unique() {
        assert_eq!(ErrorCode::ALL.len(), EXPECTED.len());
        for (code, (expected_code, expected_category)) in ErrorCode::ALL.iter().zip(EXPECTED) {
            assert_eq!(code.as_str(), *expected_code);
            assert_eq!(code.category().as_str(), *expected_category);
        }

        let mut codes: Vec<&str> = ErrorCode::ALL.iter().map(|code| code.as_str()).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(
            codes.len(),
            ErrorCode::ALL.len(),
            "error codes must be unique"
        );
    }

    #[test]
    fn categories_expose_stable_strings() {
        let categories = [
            ErrorCategory::Validation,
            ErrorCategory::Configuration,
            ErrorCategory::Limit,
            ErrorCategory::Tls,
            ErrorCategory::Authentication,
            ErrorCategory::Protocol,
            ErrorCategory::Timeout,
            ErrorCategory::Closed,
            ErrorCategory::Busy,
            ErrorCategory::Internal,
        ];
        let strings: Vec<&str> = categories
            .iter()
            .map(|category| category.as_str())
            .collect();
        assert_eq!(
            strings,
            vec![
                "validation",
                "configuration",
                "limit",
                "tls",
                "authentication",
                "protocol",
                "timeout",
                "closed",
                "busy",
                "internal",
            ]
        );
    }

    #[test]
    fn stages_expose_stable_strings() {
        let stages = [
            Stage::ConnectionSetup,
            Stage::Auth,
            Stage::Mail,
            Stage::Rcpt,
            Stage::DataCommand,
            Stage::DataBody,
            Stage::Cleanup,
        ];
        let strings: Vec<&str> = stages.iter().map(|stage| stage.as_str()).collect();
        assert_eq!(
            strings,
            vec![
                "connection_setup",
                "auth",
                "mail",
                "rcpt",
                "data_command",
                "data_body",
                "cleanup"
            ]
        );
    }

    #[test]
    fn defaults_are_redacted_and_carry_the_code() {
        for code in ErrorCode::ALL {
            let error = Error::new(*code);
            let rendered = error.to_string();
            assert!(rendered.starts_with(code.as_str()));
            let lowered = rendered.to_ascii_lowercase();
            for marker in ["password", "secret", "token", "body", "private key"] {
                assert!(!lowered.contains(marker), "{rendered} leaks {marker}");
            }
            assert_eq!(error.category(), code.category());
        }
    }

    #[test]
    fn details_are_whitelisted_and_readable() {
        let details = ErrorDetails::default()
            .host("relay.example.test")
            .port(2525)
            .timeout_ms(30_000)
            .elapsed_ms(30_001)
            .stage(Stage::Auth)
            .recipient("user@example.test")
            .smtp_code(535)
            .smtp_enhanced_code("5.7.8");

        let error = Error::new(ErrorCode::AuthFailed).with_details(details);
        assert_eq!(error.code(), ErrorCode::AuthFailed);
        assert_eq!(error.message(), "authentication failed");
        assert_eq!(error.details().host_name(), Some("relay.example.test"));
        assert_eq!(error.details().port_number(), Some(2525));
        assert_eq!(error.details().timeout_millis(), Some(30_000));
        assert_eq!(error.details().elapsed_millis(), Some(30_001));
        assert_eq!(error.details().operation_stage(), Some(Stage::Auth));
        assert_eq!(
            error.details().recipient_address(),
            Some("user@example.test")
        );
        assert_eq!(error.details().smtp_status(), Some(535));
        assert_eq!(error.details().smtp_enhanced(), Some("5.7.8"));
    }

    #[test]
    fn code_converts_into_error() {
        let error: Error = ErrorCode::TimeoutTotal.into();
        assert_eq!(error.code().as_str(), "TIMEOUT_TOTAL");
        assert_eq!(error.category(), ErrorCategory::Timeout);
    }
}
