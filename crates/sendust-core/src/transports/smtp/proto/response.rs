//! SMTP reply parsing (RFC 5321 §4.2, RFC 3463).
//!
//! The parser is bounded and total: it accepts only three-digit reply codes,
//! enforces the RFC line length, rejects malformed separators and mismatched
//! multiline codes, and never buffers more than a fixed reply size.

use crate::errors::{Error, ErrorCode};

/// Maximum reply line length including CRLF (RFC 5321 §4.5.3.1.6).
pub const MAX_LINE_BYTES: usize = 1000;
/// Maximum bytes buffered for a single reply.
pub const MAX_REPLY_BYTES: usize = 64 * 1024;

/// Reply class from the first digit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplyClass {
    /// 2xx: the requested action completed.
    Positive,
    /// 3xx: the server needs more input.
    Intermediate,
    /// 4xx: transient failure; the command may be retried later by the caller.
    Transient,
    /// 5xx: permanent failure.
    Permanent,
}

/// A validated three-digit SMTP reply code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplyCode(u16);

impl ReplyCode {
    pub fn from_u16(value: u16) -> Result<Self, Error> {
        if (200..=599).contains(&value) {
            Ok(Self(value))
        } else {
            Err(parse_error())
        }
    }

    pub const fn value(self) -> u16 {
        self.0
    }

    pub const fn class(self) -> ReplyClass {
        match self.0 / 100 {
            2 => ReplyClass::Positive,
            3 => ReplyClass::Intermediate,
            4 => ReplyClass::Transient,
            _ => ReplyClass::Permanent,
        }
    }

    pub const fn is_positive(self) -> bool {
        matches!(self.class(), ReplyClass::Positive)
    }
}

/// RFC 3463 enhanced status code (`x.y.z`) when the relay supplied one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnhancedStatus {
    raw: String,
    class: u8,
    subject: u8,
    detail: u16,
}

impl EnhancedStatus {
    pub fn raw(&self) -> &str {
        &self.raw
    }

    pub const fn class(&self) -> u8 {
        self.class
    }

    pub const fn subject(&self) -> u8 {
        self.subject
    }

    pub const fn detail(&self) -> u16 {
        self.detail
    }
}

fn parse_enhanced(text: &str) -> Option<EnhancedStatus> {
    let token = text.split_whitespace().next()?;
    let mut parts = token.split('.');
    let class = parts.next()?.parse::<u8>().ok()?;
    let subject = parts.next()?.parse::<u8>().ok()?;
    let detail = parts.next()?.parse::<u16>().ok()?;
    if parts.next().is_some() || class > 5 || subject > 9 || detail > 999 {
        return None;
    }
    Some(EnhancedStatus {
        raw: token.to_string(),
        class,
        subject,
        detail,
    })
}

/// A complete, validated SMTP reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    code: ReplyCode,
    lines: Vec<String>,
    enhanced: Option<EnhancedStatus>,
}

impl Reply {
    pub fn code(&self) -> ReplyCode {
        self.code
    }

    pub fn value(&self) -> u16 {
        self.code.value()
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// All reply text joined with a single space, for diagnostics.
    pub fn text(&self) -> String {
        self.lines.join(" ")
    }

    pub fn enhanced(&self) -> Option<&EnhancedStatus> {
        self.enhanced.as_ref()
    }

    pub fn is_positive(&self) -> bool {
        self.code.is_positive()
    }
}

/// Incremental reply builder for a line-oriented reader.
#[derive(Debug)]
pub struct ReplyBuilder {
    code: Option<u16>,
    lines: Vec<String>,
    bytes: usize,
    complete: bool,
}

impl Default for ReplyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplyBuilder {
    pub fn new() -> Self {
        Self {
            code: None,
            lines: Vec::new(),
            bytes: 0,
            complete: false,
        }
    }

    /// Feeds one CRLF-terminated line. Returns `true` when the reply is complete.
    pub fn push_line(&mut self, line: &[u8]) -> Result<bool, Error> {
        if self.complete {
            return Err(parse_error());
        }

        let line = line
            .strip_suffix(b"\r\n")
            .or_else(|| line.strip_suffix(b"\n"))
            .unwrap_or(line);

        if line.len() + 2 > MAX_LINE_BYTES {
            return Err(parse_error());
        }
        self.bytes = self.bytes.saturating_add(line.len() + 2);
        if self.bytes > MAX_REPLY_BYTES {
            return Err(parse_error());
        }

        if line.len() < 3 {
            return Err(parse_error());
        }
        let code = std::str::from_utf8(&line[..3])
            .ok()
            .and_then(|digits| digits.parse::<u16>().ok())
            .ok_or_else(parse_error)?;
        let code = ReplyCode::from_u16(code)?.value();

        match self.code {
            Some(existing) if existing != code => return Err(parse_error()),
            None => self.code = Some(code),
            Some(_) => {}
        }

        let last = match line.len() {
            3 => true,
            _ => match line[3] {
                b' ' => true,
                b'-' => false,
                _ => return Err(parse_error()),
            },
        };

        let text_start = if line.len() > 3 { 4 } else { 3 };
        self.lines
            .push(String::from_utf8_lossy(&line[text_start..]).into_owned());
        self.complete = last;
        Ok(last)
    }

    pub fn finish(self) -> Result<Reply, Error> {
        let code = self.code.ok_or_else(parse_error)?;
        if self.lines.is_empty() || !self.complete {
            return Err(parse_error());
        }
        let enhanced = self.lines.first().and_then(|text| parse_enhanced(text));
        Ok(Reply {
            code: ReplyCode::from_u16(code)?,
            lines: self.lines,
            enhanced,
        })
    }
}

/// Parses one complete reply from a byte buffer, returning the bytes consumed.
pub fn parse_reply(input: &[u8]) -> Result<(Reply, usize), Error> {
    let mut builder = ReplyBuilder::new();
    let mut start = 0usize;

    for (index, byte) in input.iter().enumerate() {
        if *byte == b'\n' {
            let end = index + 1;
            if builder.push_line(&input[start..end])? {
                return Ok((builder.finish()?, end));
            }
            start = end;
        }
    }

    Err(parse_error())
}

fn parse_error() -> Error {
    Error::new(ErrorCode::ProtocolUnexpectedResponse)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> Reply {
        let (reply, consumed) = parse_reply(input.as_bytes()).expect("parsed");
        assert_eq!(consumed, input.len());
        reply
    }

    #[test]
    fn parses_a_single_line_reply() {
        let reply = parse("250 2.0.0 Message accepted\r\n");
        assert_eq!(reply.value(), 250);
        assert_eq!(reply.code().class(), ReplyClass::Positive);
        assert_eq!(reply.text(), "2.0.0 Message accepted");
        assert_eq!(reply.enhanced().map(EnhancedStatus::raw), Some("2.0.0"));
    }

    #[test]
    fn parses_a_multiline_reply() {
        let reply = parse("250-first\r\n250-second\r\n250 last\r\n");
        assert_eq!(reply.value(), 250);
        assert_eq!(reply.lines(), ["first", "second", "last"]);
    }

    #[test]
    fn parses_enhanced_status_fields() {
        let reply = parse("450 4.3.0 Try again\r\n");
        let enhanced = reply.enhanced().expect("enhanced status");
        assert_eq!(enhanced.class(), 4);
        assert_eq!(enhanced.subject(), 3);
        assert_eq!(enhanced.detail(), 0);
        assert_eq!(reply.code().class(), ReplyClass::Transient);
    }

    #[test]
    fn rejects_mismatched_multiline_codes() {
        let error = parse_reply(b"250-first\r\n550 last\r\n").expect_err("mismatch");
        assert_eq!(error.code(), ErrorCode::ProtocolUnexpectedResponse);
    }

    #[test]
    fn rejects_malformed_codes_and_separators() {
        let cases: [&[u8]; 3] = [b"250X bad\r\n", b"2a0 bad\r\n", b"600 bad\r\n"];
        for case in cases {
            let error = parse_reply(case).expect_err("malformed");
            assert_eq!(error.code(), ErrorCode::ProtocolUnexpectedResponse);
        }
    }

    #[test]
    fn rejects_an_overlong_line() {
        let mut line = b"250 ".to_vec();
        line.extend(std::iter::repeat_n(b'a', MAX_LINE_BYTES));
        line.extend_from_slice(b"\r\n");
        let error = parse_reply(&line).expect_err("overlong");
        assert_eq!(error.code(), ErrorCode::ProtocolUnexpectedResponse);
    }

    #[test]
    fn rejects_continuation_without_a_final_line() {
        let mut builder = ReplyBuilder::new();
        assert!(!builder.push_line(b"250-still going\r\n").expect("line"));
        let error = builder.finish().expect_err("incomplete");
        assert_eq!(error.code(), ErrorCode::ProtocolUnexpectedResponse);
    }

    #[test]
    fn parse_reply_consumes_only_the_first_reply() {
        let input = b"250 OK\r\n220 next greeting\r\n";
        let (reply, consumed) = parse_reply(input).expect("parsed");
        assert_eq!(reply.value(), 250);
        assert_eq!(consumed, 8);
    }
}
