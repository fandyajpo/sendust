//! Outgoing message and header model.
//!
//! Header names and values are validated newtypes: a value can never carry a
//! CR, LF, NUL, or other control character, so it cannot introduce a new header
//! or a new recipient. The full message model is built on these types in P04-003.

use crate::errors::{Error, ErrorCode};

/// Maximum header-name length accepted by the model.
const MAX_HEADER_NAME: usize = 78;
/// Maximum header-value length; RFC 5322 caps a line at 998 octets.
const MAX_HEADER_VALUE: usize = 998;

/// A validated header field name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HeaderName(String);

impl HeaderName {
    pub fn parse(raw: &str) -> Result<Self, Error> {
        let name = raw.trim();
        if name.is_empty() || name.len() > MAX_HEADER_NAME {
            return Err(Error::new(ErrorCode::ValidationInvalidHeader));
        }
        if name.chars().any(char::is_control) {
            return Err(Error::new(ErrorCode::ValidationHeaderInjection));
        }
        if !name.chars().all(is_field_name_character) {
            return Err(Error::new(ErrorCode::ValidationInvalidHeader));
        }
        Ok(Self(name.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::str::FromStr for HeaderName {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::parse(input)
    }
}

/// A validated header field value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderValue(String);

impl HeaderValue {
    pub fn parse(raw: &str) -> Result<Self, Error> {
        // Tab is the only control character allowed inside a header value; CR,
        // LF, NUL, and the rest are injection attempts.
        if raw
            .chars()
            .any(|character| character.is_control() && character != '\t')
        {
            return Err(Error::new(ErrorCode::ValidationHeaderInjection));
        }
        let value = raw.trim();
        if value.len() > MAX_HEADER_VALUE {
            return Err(Error::new(ErrorCode::ValidationInvalidHeader));
        }
        Ok(Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::str::FromStr for HeaderValue {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::parse(input)
    }
}

/// A validated `Name: Value` pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    name: HeaderName,
    value: HeaderValue,
}

impl Header {
    pub fn new(name: impl AsRef<str>, value: impl AsRef<str>) -> Result<Self, Error> {
        Ok(Self {
            name: HeaderName::parse(name.as_ref())?,
            value: HeaderValue::parse(value.as_ref())?,
        })
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn value(&self) -> &str {
        self.value.as_str()
    }
}

fn is_field_name_character(character: char) -> bool {
    character.is_ascii_alphanumeric()
        || matches!(
            character,
            '!' | '#'
                | '$'
                | '%'
                | '&'
                | '\''
                | '*'
                | '+'
                | '-'
                | '.'
                | '^'
                | '_'
                | '`'
                | '|'
                | '~'
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name_error(input: &str) -> ErrorCode {
        HeaderName::parse(input)
            .expect_err("must be rejected")
            .code()
    }

    fn value_error(input: &str) -> ErrorCode {
        HeaderValue::parse(input)
            .expect_err("must be rejected")
            .code()
    }

    #[test]
    fn accepts_standard_headers() {
        let header = Header::new("X-Request-Id", "abc-123").expect("valid");
        assert_eq!(header.name(), "X-Request-Id");
        assert_eq!(header.value(), "abc-123");
    }

    #[test]
    fn accepts_unicode_header_values() {
        let header = Header::new("Subject", "Grüße aus Berlin").expect("valid");
        assert_eq!(header.value(), "Grüße aus Berlin");
    }

    #[test]
    fn trims_values_but_not_into_injection() {
        let header = Header::new("X-Trace", "  padded  ").expect("valid");
        assert_eq!(header.value(), "padded");
    }

    #[test]
    fn rejects_invalid_header_names() {
        for input in ["", "   ", "Bad Name", "Bad:Name", "Bad@Name", "Bad(Name)"] {
            assert_eq!(
                name_error(input),
                ErrorCode::ValidationInvalidHeader,
                "{input}"
            );
        }
    }

    #[test]
    fn rejects_control_characters_in_names_as_injection() {
        for input in ["Bad\rName", "Bad\nName", "Bad\0Name"] {
            assert_eq!(
                name_error(input),
                ErrorCode::ValidationHeaderInjection,
                "{input}"
            );
        }
    }

    #[test]
    fn rejects_header_injection_in_values() {
        for input in [
            "value\r\nBcc: evil@example.com",
            "value\nBcc: evil@example.com",
            "value\0",
            "value\r",
            "value\x0b",
        ] {
            assert_eq!(
                value_error(input),
                ErrorCode::ValidationHeaderInjection,
                "{input}"
            );
        }
    }

    #[test]
    fn allows_tab_inside_values() {
        let value = HeaderValue::parse("a\tb").expect("valid");
        assert_eq!(value.as_str(), "a\tb");
    }

    #[test]
    fn rejects_overlong_values() {
        let long = "a".repeat(MAX_HEADER_VALUE + 1);
        assert_eq!(value_error(&long), ErrorCode::ValidationInvalidHeader);
    }
}
