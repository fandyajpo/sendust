//! Mailbox parsing, normalization, and recipient identity.
//!
//! Envelope rules follow the approved policy: the local part is ASCII only
//! (SMTPUTF8 is rejected as post-MVP), the domain is IDNA/UTS #46 converted to
//! its ASCII form, and a display name may be Unicode. Control characters are
//! rejected everywhere so an address can never inject a header.

use std::fmt;
use std::str::FromStr;

use crate::errors::{Error, ErrorCode};

/// Maximum local-part length (RFC 5321).
const MAX_LOCAL_PART: usize = 64;
/// Maximum `local@domain` length (RFC 5321 reverse-path).
const MAX_ADDRESS: usize = 254;
/// Maximum domain length in ASCII form.
const MAX_DOMAIN: usize = 253;
/// Maximum stored display-name length.
const MAX_DISPLAY_NAME: usize = 256;

/// A validated mailbox: ASCII local part, IDNA domain, optional display name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mailbox {
    display_name: Option<String>,
    local_part: String,
    domain: String,
}

impl Mailbox {
    /// Parses `address` or `Display Name <address>` into a validated mailbox.
    pub fn parse(input: &str) -> Result<Self, Error> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(invalid_address());
        }
        reject_control_characters(trimmed)?;

        let (raw_display, raw_address) = split_display_name(trimmed)?;
        let (raw_local, raw_domain) = raw_address.rsplit_once('@').ok_or_else(invalid_address)?;

        let local_part = normalize_local_part(raw_local)?;
        let domain = normalize_domain(raw_domain)?;
        let display_name = normalize_display_name(raw_display)?;

        if local_part.len() + 1 + domain.len() > MAX_ADDRESS {
            return Err(invalid_address());
        }

        Ok(Self {
            display_name,
            local_part,
            domain,
        })
    }

    /// Local part, ASCII only.
    pub fn local_part(&self) -> &str {
        &self.local_part
    }

    /// Domain in ASCII (IDNA) form.
    pub fn domain(&self) -> &str {
        &self.domain
    }

    /// Display name, which may contain Unicode.
    pub fn display_name(&self) -> Option<&str> {
        self.display_name.as_deref()
    }

    /// `local@domain` without any display name.
    pub fn address(&self) -> String {
        format!("{}@{}", self.local_part, self.domain)
    }

    /// Injection-safe `Display Name <address>` form.
    ///
    /// The display name is always quoted and escaped, so it cannot introduce a
    /// header or a second address. RFC 2047 encoding of non-ASCII names is
    /// performed during MIME composition (P07).
    pub fn formatted(&self) -> String {
        match &self.display_name {
            Some(name) => format!("\"{}\" <{}>", escape_quoted(name), self.address()),
            None => self.address(),
        }
    }
}

impl fmt::Display for Mailbox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.formatted())
    }
}

impl FromStr for Mailbox {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::parse(input)
    }
}

fn invalid_address() -> Error {
    Error::new(ErrorCode::ValidationInvalidAddress)
}

fn invalid_header() -> Error {
    Error::new(ErrorCode::ValidationHeaderInjection)
}

fn reject_control_characters(input: &str) -> Result<(), Error> {
    if input.chars().any(char::is_control) {
        return Err(invalid_header());
    }
    Ok(())
}

fn split_display_name(input: &str) -> Result<(Option<&str>, &str), Error> {
    match input.rfind('<') {
        Some(start) => {
            if !input.ends_with('>') {
                return Err(invalid_address());
            }
            let name = &input[..start];
            let address = &input[start + 1..input.len() - 1];
            if name.contains('>') || address.contains('<') || address.contains('>') {
                return Err(invalid_address());
            }
            Ok((Some(name), address))
        }
        None => {
            if input.contains('>') {
                return Err(invalid_address());
            }
            Ok((None, input))
        }
    }
}

fn normalize_display_name(raw: Option<&str>) -> Result<Option<String>, Error> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let unquoted = unquote(trimmed)?;
    if unquoted.is_empty() {
        return Ok(None);
    }
    if unquoted.len() > MAX_DISPLAY_NAME {
        return Err(invalid_address());
    }
    Ok(Some(unquoted))
}

fn unquote(value: &str) -> Result<String, Error> {
    if !(value.starts_with('"') && value.ends_with('"') && value.len() >= 2) {
        return Ok(value.to_string());
    }

    let inner = &value[1..value.len() - 1];
    let mut out = String::with_capacity(inner.len());
    let mut escaped = false;
    for character in inner.chars() {
        if escaped {
            out.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            return Err(invalid_address());
        } else {
            out.push(character);
        }
    }
    if escaped {
        return Err(invalid_address());
    }
    Ok(out)
}

fn escape_quoted(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for character in value.chars() {
        if character == '"' || character == '\\' {
            out.push('\\');
        }
        out.push(character);
    }
    out
}

fn normalize_local_part(raw: &str) -> Result<String, Error> {
    if raw.is_empty() {
        return Err(invalid_address());
    }
    if !raw.is_ascii() {
        return Err(Error::new(ErrorCode::ValidationNonAsciiLocalPart));
    }
    if raw.len() > MAX_LOCAL_PART {
        return Err(invalid_address());
    }
    if raw.starts_with('.') || raw.ends_with('.') || raw.contains("..") {
        return Err(invalid_address());
    }
    if !raw.chars().all(is_local_part_character) {
        return Err(invalid_address());
    }
    Ok(raw.to_string())
}

fn is_local_part_character(character: char) -> bool {
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
                | '/'
                | '='
                | '?'
                | '^'
                | '_'
                | '`'
                | '{'
                | '|'
                | '}'
                | '~'
                | '.'
        )
}

fn normalize_domain(raw: &str) -> Result<String, Error> {
    if raw.is_empty() {
        return Err(invalid_address());
    }
    // Trailing-dot policy: rejected as ambiguous for envelope use.
    if raw.ends_with('.') || raw.starts_with('.') {
        return Err(invalid_address());
    }
    if raw.len() > MAX_DOMAIN {
        return Err(invalid_address());
    }

    // STD3 rules, hyphen checks, and DNS length/label verification.
    let ascii = idna::domain_to_ascii_strict(raw).map_err(|_| invalid_address())?;
    if ascii.is_empty()
        || ascii.ends_with('.')
        || ascii.starts_with('.')
        || ascii.len() > MAX_DOMAIN
        || ascii.contains("..")
    {
        return Err(invalid_address());
    }
    Ok(ascii)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code(input: &str) -> ErrorCode {
        Mailbox::parse(input).expect_err("must be rejected").code()
    }

    #[test]
    fn parses_plain_ascii_address() {
        let mailbox = Mailbox::parse("user@example.com").expect("valid");
        assert_eq!(mailbox.local_part(), "user");
        assert_eq!(mailbox.domain(), "example.com");
        assert_eq!(mailbox.display_name(), None);
        assert_eq!(mailbox.address(), "user@example.com");
        assert_eq!(mailbox.formatted(), "user@example.com");
    }

    #[test]
    fn converts_idna_domain_to_ascii() {
        let mailbox = Mailbox::parse("user@bücher.example").expect("valid");
        assert_eq!(mailbox.domain(), "xn--bcher-kva.example");
        assert_eq!(mailbox.address(), "user@xn--bcher-kva.example");
    }

    #[test]
    fn lowercases_ascii_domains_and_keeps_punycode() {
        let upper = Mailbox::parse("USER@EXAMPLE.COM").expect("valid");
        assert_eq!(upper.address(), "USER@example.com");

        let punycode = Mailbox::parse("user@xn--bcher-kva.example").expect("valid");
        assert_eq!(punycode.domain(), "xn--bcher-kva.example");
    }

    #[test]
    fn keeps_unicode_display_name_and_quotes_it_safely() {
        let mailbox = Mailbox::parse("Böb <user@example.com>").expect("valid");
        assert_eq!(mailbox.display_name(), Some("Böb"));
        assert_eq!(mailbox.address(), "user@example.com");
        assert_eq!(mailbox.formatted(), "\"Böb\" <user@example.com>");
    }

    #[test]
    fn unquotes_and_quotes_display_names_safely() {
        let quoted = Mailbox::parse("\"Doe, John\" <john@example.com>").expect("valid");
        assert_eq!(quoted.display_name(), Some("Doe, John"));
        assert_eq!(quoted.formatted(), "\"Doe, John\" <john@example.com>");

        let escaped = Mailbox::parse("\"A \\\"quote\\\"\" <a@example.com>").expect("valid");
        assert_eq!(escaped.display_name(), Some("A \"quote\""));
        assert_eq!(escaped.formatted(), "\"A \\\"quote\\\"\" <a@example.com>");
    }

    #[test]
    fn accepts_empty_display_name_with_brackets() {
        let mailbox = Mailbox::parse("<user@example.com>").expect("valid");
        assert_eq!(mailbox.display_name(), None);
    }

    #[test]
    fn rejects_non_ascii_local_part() {
        assert_eq!(
            code("üser@example.com"),
            ErrorCode::ValidationNonAsciiLocalPart
        );
    }

    #[test]
    fn rejects_malformed_addresses() {
        for input in [
            "",
            "   ",
            "no-at-sign",
            "@example.com",
            "user@",
            ".user@example.com",
            "user.@example.com",
            "us..er@example.com",
            "user@example..com",
            "user@.example.com",
            "user@example.com.",
            "user@exa_mple.com",
            "user@-example.com",
            "user@example.com<>",
            "Name <user@example.com> trailing",
            "Name <user@example.com",
        ] {
            assert_eq!(code(input), ErrorCode::ValidationInvalidAddress, "{input}");
        }
    }

    #[test]
    fn rejects_overlong_parts() {
        let long_local = format!("{}@example.com", "a".repeat(MAX_LOCAL_PART + 1));
        assert_eq!(code(&long_local), ErrorCode::ValidationInvalidAddress);

        let long_label = format!("user@{}.example", "a".repeat(64));
        assert_eq!(code(&long_label), ErrorCode::ValidationInvalidAddress);
    }

    #[test]
    fn rejects_header_injection_in_address_and_display_name() {
        for input in [
            "user@example.com\r\nBcc: evil@example.com",
            "user@example.com\nBcc: evil@example.com",
            "user@example.com\0",
            "Evil\r\nBcc: <evil@example.com>",
            "Evil\tName <user@example.com>",
        ] {
            assert_eq!(code(input), ErrorCode::ValidationHeaderInjection, "{input}");
        }
    }

    #[test]
    fn rejects_header_like_display_names() {
        assert_eq!(
            code("Admin\r\nBcc: evil@example.com <user@example.com>"),
            ErrorCode::ValidationHeaderInjection
        );
    }

    #[test]
    fn accepts_quoted_display_name_with_colon() {
        let mailbox = Mailbox::parse("Support: Billing <billing@example.com>").expect("valid");
        assert_eq!(mailbox.display_name(), Some("Support: Billing"));
        assert_eq!(
            mailbox.formatted(),
            "\"Support: Billing\" <billing@example.com>"
        );
    }

    #[test]
    fn display_is_the_formatted_form() {
        let mailbox = Mailbox::parse("Böb <user@example.com>").expect("valid");
        assert_eq!(mailbox.to_string(), mailbox.formatted());
    }

    #[test]
    fn parses_from_str() {
        let mailbox: Mailbox = "user@example.com".parse().expect("valid");
        assert_eq!(mailbox.address(), "user@example.com");
    }
}
