//! MIME tree composition, encoding, serialization, and wire-size accounting.
//!
//! This module owns the RFC 5322 header order, the deterministic serializer and
//! the 30 MiB wire cap; header encoding and folding live in `headers`, and the
//! model and its duplicate policy live in `message`. The composed bytes are the
//! single source of truth for transport and for the size cap.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

use crate::attachments::Attachment;
use crate::config::Limits;
use crate::errors::{Error, ErrorCode};
use crate::message::OutgoingMessage;

mod headers;

use headers::CRLF;

// Header order is owned here (ADR-0010) and is fixed: `From`, `To`, `Cc`,
// `Reply-To`, `Subject`, then the caller's custom headers in insertion order,
// then the composed `MIME-Version` and body headers. Adjusting the order is a
// contract change, not a formatting choice.

/// Serializes a validated message and enforces the wire-size cap.
///
/// Validation runs before any byte is produced, so an invalid message can never
/// reach a transport. File attachments require the P08 bounded reader; until it
/// exists a file descriptor cannot be serialized.
pub fn serialize(
    message: &OutgoingMessage,
    limits: Limits,
    filesystem_enabled: bool,
) -> Result<Vec<u8>, Error> {
    message.validate(limits, filesystem_enabled)?;

    if message
        .attachments
        .iter()
        .any(|attachment| matches!(attachment, Attachment::File(_)))
    {
        // P08 reads bounded regular files into owned bytes before composing.
        return Err(Error::new(ErrorCode::ValidationInvalidAttachment));
    }

    let body = build_body(message)?;
    let attachments: Vec<Vec<u8>> = message
        .attachments
        .iter()
        .filter_map(|attachment| match attachment {
            Attachment::Bytes {
                filename,
                content_type,
                bytes,
            } => Some(build_attachment_part(
                filename,
                content_type.as_deref(),
                bytes,
            )),
            Attachment::File(_) => None,
        })
        .collect::<Result<_, Error>>()?;

    let mut out = Vec::new();
    push_address_header(&mut out, "From", std::slice::from_ref(&message.from))?;
    push_address_header(&mut out, "To", &message.to)?;
    if !message.cc.is_empty() {
        push_address_header(&mut out, "Cc", &message.cc)?;
    }
    if !message.reply_to.is_empty() {
        push_address_header(&mut out, "Reply-To", &message.reply_to)?;
    }
    push_header(&mut out, "Subject", message.subject.trim())?;
    for header in &message.headers {
        push_header(&mut out, header.name(), header.value())?;
    }
    push_header(&mut out, "MIME-Version", "1.0")?;

    if attachments.is_empty() {
        body.push_top_headers(&mut out)?;
        out.extend_from_slice(CRLF.as_bytes());
        body.push_content(&mut out);
    } else {
        let mut parts = vec![body.as_part()?];
        parts.extend(attachments);
        let boundary = unique_boundary(&parts);
        push_header(
            &mut out,
            "Content-Type",
            &format!("multipart/mixed; boundary=\"{boundary}\""),
        )?;
        out.extend_from_slice(CRLF.as_bytes());
        push_multipart(&mut out, &boundary, &parts);
    }

    let len = out.len() as u64;
    if len > limits.max_wire_bytes() {
        return Err(Error::new(ErrorCode::LimitWireBytes));
    }
    Ok(out)
}

enum Body {
    Single {
        content_type: String,
        bytes: Vec<u8>,
    },
    Multipart {
        subtype: &'static str,
        boundary: String,
        parts: Vec<Vec<u8>>,
    },
}

impl Body {
    fn push_top_headers(&self, out: &mut Vec<u8>) -> Result<(), Error> {
        match self {
            Self::Single { content_type, .. } => {
                push_header(out, "Content-Type", content_type)?;
                push_header(out, "Content-Transfer-Encoding", "base64")?;
            }
            Self::Multipart {
                subtype, boundary, ..
            } => {
                push_header(
                    out,
                    "Content-Type",
                    &format!("multipart/{subtype}; boundary=\"{boundary}\""),
                )?;
            }
        }
        Ok(())
    }

    fn push_content(&self, out: &mut Vec<u8>) {
        match self {
            Self::Single { bytes, .. } => push_base64(out, bytes),
            Self::Multipart {
                boundary, parts, ..
            } => push_multipart(out, boundary, parts),
        }
    }

    fn as_part(&self) -> Result<Vec<u8>, Error> {
        let mut part = Vec::new();
        self.push_top_headers(&mut part)?;
        part.extend_from_slice(CRLF.as_bytes());
        self.push_content(&mut part);
        Ok(part)
    }
}

fn build_body(message: &OutgoingMessage) -> Result<Body, Error> {
    let text = message.text.as_deref().unwrap_or_default();
    let html = message.html.as_deref().unwrap_or_default();

    Ok(match (message.text.is_some(), message.html.is_some()) {
        (true, false) => Body::Single {
            content_type: "text/plain; charset=utf-8".to_string(),
            bytes: text.as_bytes().to_vec(),
        },
        (false, true) => Body::Single {
            content_type: "text/html; charset=utf-8".to_string(),
            bytes: html.as_bytes().to_vec(),
        },
        (true, true) => {
            let parts = vec![
                Body::Single {
                    content_type: "text/plain; charset=utf-8".to_string(),
                    bytes: text.as_bytes().to_vec(),
                }
                .as_part()?,
                Body::Single {
                    content_type: "text/html; charset=utf-8".to_string(),
                    bytes: html.as_bytes().to_vec(),
                }
                .as_part()?,
            ];
            let boundary = unique_boundary(&parts);
            Body::Multipart {
                subtype: "alternative",
                boundary,
                parts,
            }
        }
        (false, false) => Body::Single {
            content_type: "text/plain; charset=utf-8".to_string(),
            bytes: Vec::new(),
        },
    })
}

fn build_attachment_part(
    filename: &str,
    content_type: Option<&str>,
    bytes: &[u8],
) -> Result<Vec<u8>, Error> {
    let mut part = Vec::new();
    push_header(
        &mut part,
        "Content-Type",
        content_type.unwrap_or("application/octet-stream"),
    )?;
    push_header(&mut part, "Content-Transfer-Encoding", "base64")?;
    push_header(
        &mut part,
        "Content-Disposition",
        &format!("attachment; filename=\"{}\"", escape_quotes(filename)),
    )?;
    part.extend_from_slice(CRLF.as_bytes());
    push_base64(&mut part, bytes);
    Ok(part)
}

fn push_multipart(out: &mut Vec<u8>, boundary: &str, parts: &[Vec<u8>]) {
    for part in parts {
        out.extend_from_slice(format!("--{boundary}{CRLF}").as_bytes());
        out.extend_from_slice(part);
        out.extend_from_slice(CRLF.as_bytes());
    }
    out.extend_from_slice(format!("--{boundary}--{CRLF}").as_bytes());
}

fn push_address_header(
    out: &mut Vec<u8>,
    name: &str,
    addresses: &[crate::address::Mailbox],
) -> Result<(), Error> {
    out.extend_from_slice(&headers::render_addresses(name, addresses)?);
    Ok(())
}

fn push_header(out: &mut Vec<u8>, name: &str, value: &str) -> Result<(), Error> {
    out.extend_from_slice(&headers::render(name, value)?);
    Ok(())
}

fn escape_quotes(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn unique_boundary(parts: &[Vec<u8>]) -> String {
    let mut counter = 1u32;
    loop {
        let boundary = format!("sendust-boundary-{counter:04}");
        let marker = boundary.as_bytes();
        if !parts.iter().any(|part| contains(part, marker)) {
            return boundary;
        }
        counter += 1;
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

fn push_base64(out: &mut Vec<u8>, data: &[u8]) {
    for line in base64(data).as_bytes().chunks(76) {
        out.extend_from_slice(line);
        out.extend_from_slice(CRLF.as_bytes());
    }
}

fn base64(data: &[u8]) -> String {
    STANDARD.encode(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address::Mailbox;
    use crate::message::OutgoingMessage;

    fn mailbox(value: &str) -> Mailbox {
        Mailbox::parse(value).expect("valid mailbox")
    }

    fn message() -> OutgoingMessage {
        OutgoingMessage {
            from: mailbox("Alice <alice@example.test>"),
            to: vec![mailbox("bob@example.test")],
            cc: Vec::new(),
            bcc: Vec::new(),
            reply_to: Vec::new(),
            subject: "Hello".to_string(),
            text: Some("hello world".to_string()),
            html: None,
            headers: Vec::new(),
            attachments: Vec::new(),
        }
    }

    fn text(bytes: &[u8]) -> String {
        String::from_utf8_lossy(bytes).into_owned()
    }

    #[test]
    fn serializes_a_plain_text_message() {
        let bytes = serialize(&message(), Limits::default(), false).expect("serialized");
        let wire = text(&bytes);

        assert!(wire.starts_with("From: \"Alice\" <alice@example.test>\r\n"));
        assert!(wire.contains("To: bob@example.test\r\n"));
        assert!(wire.contains("Subject: Hello\r\n"));
        assert!(wire.contains("Content-Type: text/plain; charset=utf-8\r\n"));
        assert!(wire.contains("Content-Transfer-Encoding: base64\r\n"));
        assert!(wire.ends_with("aGVsbG8gd29ybGQ=\r\n"));
    }

    #[test]
    fn serializes_html_only() {
        let mut message = message();
        message.text = None;
        message.html = Some("<p>hi</p>".to_string());

        let wire = text(&serialize(&message, Limits::default(), false).expect("serialized"));
        assert!(wire.contains("Content-Type: text/html; charset=utf-8\r\n"));
        assert!(wire.contains(&base64(b"<p>hi</p>")));
    }

    #[test]
    fn serializes_alternative_bodies() {
        let mut message = message();
        message.html = Some("<p>hi</p>".to_string());

        let wire = text(&serialize(&message, Limits::default(), false).expect("serialized"));
        assert!(wire.contains("multipart/alternative; boundary=\""));
        assert!(wire.contains("Content-Type: text/plain; charset=utf-8"));
        assert!(wire.contains("Content-Type: text/html; charset=utf-8"));
        assert!(wire.trim_end().ends_with("--"));
    }

    #[test]
    fn serializes_attachments_and_never_leaks_bcc() {
        let mut message = message();
        message.bcc = vec![mailbox("hidden@example.test")];
        message.attachments = vec![Attachment::Bytes {
            filename: "notes.txt".to_string(),
            content_type: Some("text/plain".to_string()),
            bytes: b"attachment body".to_vec(),
        }];

        let wire = text(&serialize(&message, Limits::default(), false).expect("serialized"));
        assert!(wire.contains("multipart/mixed; boundary=\""));
        assert!(wire.contains("Content-Disposition: attachment; filename=\"notes.txt\""));
        assert!(wire.contains(&base64(b"attachment body")));
        assert!(
            !wire.contains("hidden@example.test"),
            "BCC must never appear"
        );
    }

    #[test]
    fn encodes_non_ascii_subjects_and_display_names() {
        let mut message = message();
        message.subject = "Grüße".to_string();
        message.from = mailbox("Böb <bob@example.test>");

        let bytes = serialize(&message, Limits::default(), false).expect("serialized");
        let wire = text(&bytes);
        assert!(wire.contains("Subject: =?UTF-8?B?"));
        assert!(wire.contains("From: =?UTF-8?B?"));
        assert!(
            bytes.iter().all(u8::is_ascii),
            "headers must be ASCII clean"
        );
    }

    #[test]
    fn is_deterministic() {
        let first = serialize(&message(), Limits::default(), false).expect("first");
        let second = serialize(&message(), Limits::default(), false).expect("second");
        assert_eq!(first, second);
    }

    #[test]
    fn every_line_is_within_the_rfc_limit() {
        let mut message = message();
        message.text = Some("x".repeat(4000));
        let bytes = serialize(&message, Limits::default(), false).expect("serialized");
        for line in bytes.split(|byte| *byte == b'\n') {
            assert!(
                line.len() <= headers::MAX_LINE + 1,
                "line of {} bytes",
                line.len()
            );
        }
    }

    #[test]
    fn wire_cap_is_inclusive_and_exact() {
        let bytes = serialize(&message(), Limits::default(), false).expect("serialized");
        let exact = Limits::default()
            .with_max_wire_bytes(bytes.len() as u64)
            .expect("lowering is allowed");
        assert!(serialize(&message(), exact, false).is_ok());

        let under = Limits::default()
            .with_max_wire_bytes(bytes.len() as u64 - 1)
            .expect("lowering is allowed");
        assert_eq!(
            serialize(&message(), under, false)
                .expect_err("over the cap")
                .code(),
            ErrorCode::LimitWireBytes
        );
    }

    #[test]
    fn enforces_the_default_wire_cap_on_large_bodies() {
        let mut message = message();
        message.text = Some("a".repeat(21 * 1024 * 1024));
        let bytes = serialize(&message, Limits::default(), false).expect("under the cap");
        assert!(bytes.len() as u64 <= Limits::default().max_wire_bytes());

        message.text = Some("a".repeat(23 * 1024 * 1024));
        assert_eq!(
            serialize(&message, Limits::default(), false)
                .expect_err("over the cap")
                .code(),
            ErrorCode::LimitWireBytes
        );
    }

    #[test]
    fn file_attachments_cannot_be_serialized_before_p08() {
        let mut message = message();
        message.attachments = vec![Attachment::File(crate::attachments::FileDescriptor {
            display_name: Some("notes.txt".to_string()),
            content_type: Some("text/plain".to_string()),
            path: "/tmp/notes.txt".to_string(),
        })];

        assert_eq!(
            serialize(&message, Limits::default(), true)
                .expect_err("not readable yet")
                .code(),
            ErrorCode::ValidationInvalidAttachment
        );
    }

    #[test]
    fn includes_custom_headers() {
        let mut message = message();
        message.headers =
            vec![crate::message::Header::new("X-Trace-Id", "trace-42").expect("valid")];
        let wire = text(&serialize(&message, Limits::default(), false).expect("serialized"));
        assert!(wire.contains("X-Trace-Id: trace-42"));
    }

    #[test]
    fn encodes_non_ascii_custom_header_values() {
        let mut message = message();
        message.headers = vec![crate::message::Header::new("X-Note", "Grüße").expect("valid")];
        let bytes = serialize(&message, Limits::default(), false).expect("serialized");
        assert!(text(&bytes).contains("X-Note: =?UTF-8?B?"));
        assert!(bytes.iter().all(u8::is_ascii));
    }

    #[test]
    fn folds_unbreakable_header_values_within_the_line_limit() {
        let mut message = message();
        message.subject = "x".repeat(990);
        let bytes = serialize(&message, Limits::default(), false).expect("serialized");
        for line in bytes.split(|byte| *byte == b'\n') {
            assert!(
                line.len() <= headers::MAX_LINE + 1,
                "line of {} bytes",
                line.len()
            );
        }
    }

    #[test]
    fn encodes_utf8_bodies_without_loss() {
        let mut message = message();
        message.text = Some("héllo".to_string());
        let wire = text(&serialize(&message, Limits::default(), false).expect("serialized"));
        assert!(wire.contains(&base64("héllo".as_bytes())));
    }

    #[test]
    fn emits_headers_in_the_approved_order() {
        let mut message = message();
        message.cc = vec![mailbox("cc@example.test")];
        message.reply_to = vec![mailbox("reply@example.test")];
        message
            .headers
            .push(crate::message::Header::new("X-Trace-Id", "abc").expect("valid"));

        let wire = text(&serialize(&message, Limits::default(), false).expect("serialized"));
        let names: Vec<&str> = wire
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with(' '))
            .map(|line| line.split(':').next().unwrap_or_default())
            .collect();

        let position = |name: &str| names.iter().position(|candidate| *candidate == name);
        assert!(position("From") < position("To"));
        assert!(position("To") < position("Cc"));
        assert!(position("Cc") < position("Reply-To"));
        assert!(position("Reply-To") < position("Subject"));
        assert!(position("Subject") < position("X-Trace-Id"));
        assert!(position("X-Trace-Id") < position("MIME-Version"));
    }

    #[test]
    fn renders_exact_encoded_word_bytes() {
        let mut message = message();
        message.subject = "Grüße".to_string();
        let wire = text(&serialize(&message, Limits::default(), false).expect("serialized"));
        assert!(wire.contains("Subject: =?UTF-8?B?R3LDvMOfZQ==?=\r\n"));
    }

    #[test]
    fn base64_matches_known_vectors() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }
}
