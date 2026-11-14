//! RFC 5322 header rendering and RFC 2047 encoded words (P07-001).
//!
//! Rendering is total and lossless: folding only replaces an existing space with
//! CRLF + a single space, so unfolding restores the exact value. A value that
//! cannot be folded legally is re-encoded as encoded words, which are short
//! enough to fold between and decode back to the original text.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

use crate::address::Mailbox;
use crate::errors::{Error, ErrorCode};

/// Preferred fold width (RFC 5322 §2.1.1 recommends 78 characters).
pub(crate) const FOLD_WIDTH: usize = 78;
/// Hard header line limit, excluding CRLF.
pub(crate) const MAX_LINE: usize = 998;
/// Raw bytes per encoded word: 60 base64 characters, 72 with the delimiters.
const MAX_ENCODED_BYTES: usize = 45;
pub(crate) const CRLF: &str = "\r\n";

/// Renders one header as folded `Name: value\r\n`.
pub(crate) fn render(name: &str, value: &str) -> Result<Vec<u8>, Error> {
    let text = encode_text(value);
    match fold(name, &text) {
        Ok(bytes) => Ok(bytes),
        // A single unbreakable token longer than any legal line is re-encoded so
        // it can be folded between encoded words instead of being corrupted.
        Err(_) => fold(name, &encode_chunks(value)),
    }
}

/// Renders an address-list header (`From`, `To`, `Cc`, `Reply-To`).
pub(crate) fn render_addresses(name: &str, addresses: &[Mailbox]) -> Result<Vec<u8>, Error> {
    let value = addresses
        .iter()
        .map(format_address)
        .collect::<Vec<_>>()
        .join(", ");
    fold(name, &value)
}

fn format_address(mailbox: &Mailbox) -> String {
    match mailbox.display_name() {
        // `formatted` quotes and escapes ASCII names, so they cannot inject.
        Some(display) if display.is_ascii() => mailbox.formatted(),
        // An encoded word is a phrase atom: it must not be quoted.
        Some(display) => format!("{} <{}>", encode_chunks(display), mailbox.address()),
        None => mailbox.address(),
    }
}

/// RFC 2047 encoding for a header value: ASCII words stay as they are, and each
/// non-ASCII word becomes one or more encoded words.
fn encode_text(value: &str) -> String {
    if value.is_ascii() {
        return value.to_string();
    }
    value
        .split(' ')
        .map(|word| {
            if word.is_ascii() {
                word.to_string()
            } else {
                encode_chunks(word)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Splits text into encoded words that never split a character and never exceed
/// the 75-character encoded-word limit.
fn encode_chunks(value: &str) -> String {
    let mut out = String::new();
    let mut chunk = Vec::new();
    for character in value.chars() {
        let mut buffer = [0u8; 4];
        let encoded = character.encode_utf8(&mut buffer).as_bytes();
        if chunk.len() + encoded.len() > MAX_ENCODED_BYTES {
            push_word(&mut out, &chunk);
            chunk.clear();
        }
        chunk.extend_from_slice(encoded);
    }
    if !chunk.is_empty() {
        push_word(&mut out, &chunk);
    }
    out
}

fn push_word(out: &mut String, chunk: &[u8]) {
    if !out.is_empty() {
        out.push(' ');
    }
    out.push_str("=?UTF-8?B?");
    out.push_str(&STANDARD.encode(chunk));
    out.push_str("?=");
}

/// Folds at spaces only. Unfolding a line therefore yields the original value:
/// a CRLF immediately followed by a space is removed by RFC 5322 unfolding, and
/// that space is the separator the fold replaced.
fn fold(name: &str, value: &str) -> Result<Vec<u8>, Error> {
    let prefix = format!("{name}:");
    let mut out = String::with_capacity(prefix.len() + value.len() + 16);
    out.push_str(&prefix);

    if value.is_empty() {
        out.push_str(CRLF);
        return Ok(out.into_bytes());
    }

    let mut line_len = prefix.len();
    for token in value.split(' ') {
        if line_len + 1 + token.len() > FOLD_WIDTH {
            out.push_str(CRLF);
            out.push(' ');
            line_len = 1;
        } else {
            out.push(' ');
            line_len += 1;
        }
        if line_len + token.len() > MAX_LINE {
            return Err(Error::new(ErrorCode::ValidationInvalidHeader));
        }
        out.push_str(token);
        line_len += token.len();
    }

    out.push_str(CRLF);
    Ok(out.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rendered(name: &str, value: &str) -> String {
        String::from_utf8(render(name, value).expect("rendered")).expect("utf8")
    }

    fn unfold(header: &str) -> String {
        header.replace("\r\n", "")
    }

    #[test]
    fn renders_ascii_values_without_encoding() {
        assert_eq!(rendered("Subject", "Hello"), "Subject: Hello\r\n");
        assert_eq!(rendered("X-Trace", "abc-123"), "X-Trace: abc-123\r\n");
    }

    #[test]
    fn renders_an_empty_value() {
        assert_eq!(rendered("Subject", ""), "Subject:\r\n");
    }

    #[test]
    fn encodes_a_unicode_word_as_one_encoded_word() {
        assert_eq!(
            rendered("Subject", "Grüße"),
            "Subject: =?UTF-8?B?R3LDvMOfZQ==?=\r\n"
        );
    }

    #[test]
    fn keeps_ascii_words_next_to_encoded_ones() {
        assert_eq!(
            rendered("Subject", "Grüße aus Berlin"),
            "Subject: =?UTF-8?B?R3LDvMOfZQ==?= aus Berlin\r\n"
        );
    }

    #[test]
    fn splits_long_unicode_into_encoded_words_within_the_limit() {
        let value = "ü".repeat(120);
        let header = rendered("Subject", &value);
        let words: Vec<&str> = header
            .split([' ', '\r', '\n'])
            .filter(|word| !word.is_empty())
            .collect();
        let encoded: Vec<&str> = words
            .iter()
            .copied()
            .filter(|word| word.starts_with("=?"))
            .collect();

        assert!(
            encoded.len() > 1,
            "a long value must use several encoded words"
        );
        for word in &encoded {
            assert!(word.len() <= 75, "{word} is too long");
            assert!(word.ends_with("?="), "{word} is malformed");
        }
        assert!(words.iter().all(|word| word.is_ascii()));
        assert!(header.contains("=?UTF-8?B?"));
    }

    #[test]
    fn folds_at_spaces_and_unfolds_to_the_original_value() {
        let value = "word ".repeat(40);
        let value = value.trim_end().to_string();
        let header = rendered("Subject", &value);
        assert!(header.contains("\r\n "));
        assert!(header.split("\r\n").all(|line| line.len() <= FOLD_WIDTH));
        assert_eq!(unfold(&header), format!("Subject: {value}"));
    }

    #[test]
    fn never_corrupts_an_unbreakable_token() {
        let value = "x".repeat(990);
        let header = rendered("Subject", &value);
        for line in header.trim_end_matches("\r\n").split("\r\n") {
            assert!(line.len() <= MAX_LINE, "line of {} bytes", line.len());
        }
        assert_eq!(unfold(&header), format!("Subject: {value}"));
    }

    #[test]
    fn encodes_unicode_display_names_without_quoting() {
        let mailbox = Mailbox::parse("Böb <bob@example.test>").expect("valid");
        let header = String::from_utf8(
            render_addresses("From", std::slice::from_ref(&mailbox)).expect("rendered"),
        )
        .expect("utf8");
        assert!(header.contains("=?UTF-8?B?"));
        assert!(!header.contains("\"=?UTF-8"));
        assert!(header.ends_with("<bob@example.test>\r\n"));
    }

    #[test]
    fn quotes_ascii_display_names() {
        let mailbox = Mailbox::parse("Alice <alice@example.test>").expect("valid");
        let header = String::from_utf8(
            render_addresses("From", std::slice::from_ref(&mailbox)).expect("rendered"),
        )
        .expect("utf8");
        assert_eq!(header, "From: \"Alice\" <alice@example.test>\r\n");
    }

    #[test]
    fn joins_an_address_list() {
        let first = Mailbox::parse("bob@example.test").expect("valid");
        let second = Mailbox::parse("carol@example.test").expect("valid");
        let header = String::from_utf8(render_addresses("To", &[first, second]).expect("rendered"))
            .expect("utf8");
        assert_eq!(header, "To: bob@example.test, carol@example.test\r\n");
    }
}
