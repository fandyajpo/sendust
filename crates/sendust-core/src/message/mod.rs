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

use crate::attachments::{self, Attachment, AttachmentPolicy};
use crate::config::Limits;

/// Header names the model owns. A custom header may not override them, or it
/// could rewrite the envelope, hide BCC, or replace the composed MIME structure.
const RESERVED_HEADERS: &[&str] = &[
    "from",
    "to",
    "cc",
    "bcc",
    "subject",
    "date",
    "reply-to",
    "return-path",
    "message-id",
    "received",
    "mime-version",
    "content-type",
    "content-transfer-encoding",
    "content-disposition",
];

/// A complete outgoing message, validated before any network work begins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutgoingMessage {
    pub from: crate::address::Mailbox,
    pub to: Vec<crate::address::Mailbox>,
    pub cc: Vec<crate::address::Mailbox>,
    pub bcc: Vec<crate::address::Mailbox>,
    pub reply_to: Vec<crate::address::Mailbox>,
    pub subject: String,
    pub text: Option<String>,
    pub html: Option<String>,
    pub headers: Vec<Header>,
    pub attachments: Vec<Attachment>,
}

impl OutgoingMessage {
    /// Total envelope recipients: To, CC, and BCC are all deliverable.
    pub fn recipient_count(&self) -> usize {
        self.to.len() + self.cc.len() + self.bcc.len()
    }

    /// Runs the deterministic stages this model owns, in approved order.
    ///
    /// Stages 1 (transport configuration) and 8 (wire size) are owned elsewhere:
    /// the transport validates its options, and wire size needs the serialized
    /// bytes produced by `mime`. Callers run all stages through
    /// `validation::validate_in_order`, so precedence stays stable.
    pub fn validate(&self, limits: Limits, policy: AttachmentPolicy) -> Result<(), Error> {
        // Stage 2: message identity. `from` is a typed mailbox, so a missing
        // From address is rejected where absence is representable (the binding),
        // not here; an empty Subject is legal per RFC 5322.
        // Stage 3: recipient presence and count.
        if self.recipient_count() == 0 {
            return Err(Error::new(ErrorCode::ValidationMissingRecipient));
        }
        if self.recipient_count() > limits.max_recipients() {
            return Err(Error::new(ErrorCode::LimitRecipients));
        }
        // Stage 4: headers and injection. Header values are validated at
        // construction; the subject is caller-supplied text and is checked here.
        HeaderValue::parse(&self.subject)?;
        let mut seen: Vec<String> = Vec::with_capacity(self.headers.len());
        for header in &self.headers {
            HeaderName::parse(header.name())?;
            HeaderValue::parse(header.value())?;
            let lowered = header.name().to_ascii_lowercase();
            if RESERVED_HEADERS.iter().any(|reserved| *reserved == lowered) {
                return Err(Error::new(ErrorCode::ValidationInvalidHeader));
            }
            // One value per custom header keeps rendering deterministic and
            // blocks a second header that shadows the first for a reader.
            if seen.contains(&lowered) {
                return Err(Error::new(ErrorCode::ValidationInvalidHeader));
            }
            seen.push(lowered);
        }
        // Stage 5: message content. A message must carry text, HTML, or both;
        // an empty string is a valid body, but neither body is not a message.
        if self.text.is_none() && self.html.is_none() {
            return Err(Error::new(ErrorCode::ValidationInvalidOption));
        }
        // Stages 6 and 7: attachment metadata and known byte caps.
        attachments::validate(&self.attachments, limits, policy)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Limits;

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

    fn mailbox(value: &str) -> crate::address::Mailbox {
        crate::address::Mailbox::parse(value).expect("valid mailbox")
    }

    fn message_with(recipients: usize) -> OutgoingMessage {
        OutgoingMessage {
            from: mailbox("alice@example.test"),
            to: (0..recipients)
                .map(|index| mailbox(&format!("user{index}@example.test")))
                .collect(),
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

    #[test]
    fn accepts_a_complete_message() {
        let message = message_with(1);
        assert!(message.validate(Limits::default(), AttachmentPolicy::disabled).is_ok());
    }

    #[test]
    fn counts_recipients_across_to_cc_and_bcc() {
        let mut message = message_with(1);
        message.cc.push(mailbox("cc@example.test"));
        message.bcc.push(mailbox("bcc@example.test"));
        assert_eq!(message.recipient_count(), 3);
    }

    #[test]
    fn recipient_cap_is_inclusive() {
        assert!(message_with(100).validate(Limits::default(), AttachmentPolicy::disabled).is_ok());
        assert_eq!(
            message_with(101)
                .validate(Limits::default(), AttachmentPolicy::disabled)
                .expect_err("over cap")
                .code(),
            ErrorCode::LimitRecipients
        );
    }

    #[test]
    fn requires_at_least_one_recipient() {
        assert_eq!(
            message_with(0)
                .validate(Limits::default(), AttachmentPolicy::disabled)
                .expect_err("no recipients")
                .code(),
            ErrorCode::ValidationMissingRecipient
        );
    }

    #[test]
    fn requires_at_least_one_body() {
        let mut message = message_with(1);
        message.text = None;
        message.html = None;
        assert_eq!(
            message
                .validate(Limits::default(), AttachmentPolicy::disabled)
                .expect_err("no body")
                .code(),
            ErrorCode::ValidationInvalidOption
        );
    }

    #[test]
    fn allows_empty_string_bodies() {
        let mut message = message_with(1);
        message.text = Some(String::new());
        assert!(message.validate(Limits::default(), AttachmentPolicy::disabled).is_ok());
    }

    #[test]
    fn rejects_subject_injection() {
        let mut message = message_with(1);
        message.subject = "Hello\r\nBcc: evil@example.test".to_string();
        assert_eq!(
            message
                .validate(Limits::default(), AttachmentPolicy::disabled)
                .expect_err("injection")
                .code(),
            ErrorCode::ValidationHeaderInjection
        );
    }

    #[test]
    fn empty_subject_is_allowed() {
        let mut message = message_with(1);
        message.subject.clear();
        assert!(message.validate(Limits::default(), AttachmentPolicy::disabled).is_ok());
    }

    #[test]
    fn rejects_reserved_custom_headers() {
        for name in ["Subject", "BCC", "Content-Type", "Return-Path"] {
            let mut message = message_with(1);
            message
                .headers
                .push(Header::new(name, "value").expect("valid header"));
            assert_eq!(
                message
                    .validate(Limits::default(), AttachmentPolicy::disabled)
                    .expect_err("reserved")
                    .code(),
                ErrorCode::ValidationInvalidHeader,
                "{name}"
            );
        }
    }

    #[test]
    fn accepts_a_custom_header() {
        let mut message = message_with(1);
        message
            .headers
            .push(Header::new("X-Trace-Id", "abc").expect("valid header"));
        assert!(message.validate(Limits::default(), AttachmentPolicy::disabled).is_ok());
    }

    #[test]
    fn propagates_attachment_failures() {
        let mut message = message_with(1);
        message.attachments.push(Attachment::Bytes {
            filename: "ok.txt".to_string(),
            content_type: None,
            bytes: vec![0u8; Limits::default().max_attachment_bytes() as usize + 1],
        });
        assert_eq!(
            message
                .validate(Limits::default(), AttachmentPolicy::disabled)
                .expect_err("over cap")
                .code(),
            ErrorCode::LimitAttachmentBytes
        );
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

#[cfg(test)]
mod duplicate_policy_tests {
    use super::*;
    use crate::config::Limits;

    fn message() -> OutgoingMessage {
        OutgoingMessage {
            from: crate::address::Mailbox::parse("alice@example.test").expect("valid"),
            to: vec![crate::address::Mailbox::parse("bob@example.test").expect("valid")],
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

    #[test]
    fn rejects_a_duplicate_custom_header_case_insensitively() {
        let mut message = message();
        message
            .headers
            .push(Header::new("X-Trace-Id", "one").expect("valid"));
        message
            .headers
            .push(Header::new("x-trace-id", "two").expect("valid"));

        assert_eq!(
            message
                .validate(Limits::default(), AttachmentPolicy::disabled)
                .expect_err("duplicate")
                .code(),
            ErrorCode::ValidationInvalidHeader
        );
    }

    #[test]
    fn rejects_a_custom_message_id_or_received_header() {
        for name in ["Message-ID", "Received"] {
            let mut message = message();
            message
                .headers
                .push(Header::new(name, "value").expect("valid"));
            assert_eq!(
                message
                    .validate(Limits::default(), AttachmentPolicy::disabled)
                    .expect_err("reserved")
                    .code(),
                ErrorCode::ValidationInvalidHeader,
                "{name}"
            );
        }
    }

    #[test]
    fn distinct_custom_headers_are_allowed() {
        let mut message = message();
        message
            .headers
            .push(Header::new("X-A", "1").expect("valid"));
        message
            .headers
            .push(Header::new("X-B", "2").expect("valid"));
        assert!(message.validate(Limits::default(), AttachmentPolicy::disabled).is_ok());
    }
}
