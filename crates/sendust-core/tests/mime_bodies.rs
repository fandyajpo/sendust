//! Golden MIME body fixtures (P07-002).
//!
//! Each golden value is the exact serialized wire form: CRLF line endings, the
//! deterministic boundary, the charset declaration, and base64 transfer
//! encoding. The serializer must reproduce it byte for byte.

use sendust_core::{ErrorCode, Limits, Mailbox, OutgoingMessage, serialize};

fn mailbox(value: &str) -> Mailbox {
    value.parse().expect("valid mailbox")
}

fn message(text: Option<&str>, html: Option<&str>) -> OutgoingMessage {
    OutgoingMessage {
        from: mailbox("Alice <alice@example.test>"),
        to: vec![mailbox("bob@example.test")],
        cc: Vec::new(),
        bcc: Vec::new(),
        reply_to: Vec::new(),
        subject: "Hello".to_string(),
        text: text.map(str::to_string),
        html: html.map(str::to_string),
        headers: Vec::new(),
        attachments: Vec::new(),
    }
}

fn wire(text: Option<&str>, html: Option<&str>) -> Vec<u8> {
    serialize(&message(text, html), Limits::default(), false).expect("serialized")
}

/// Text-only message: `text/plain`, UTF-8, base64 ("hello world").
const GOLDEN_TEXT_ONLY: &str = concat!(
    "From: \"Alice\" <alice@example.test>\r\n",
    "To: bob@example.test\r\n",
    "Subject: Hello\r\n",
    "MIME-Version: 1.0\r\n",
    "Content-Type: text/plain; charset=utf-8\r\n",
    "Content-Transfer-Encoding: base64\r\n",
    "\r\n",
    "aGVsbG8gd29ybGQ=\r\n",
);

/// Text + HTML message: `multipart/alternative` with a deterministic boundary.
const GOLDEN_ALTERNATIVE: &str = concat!(
    "From: \"Alice\" <alice@example.test>\r\n",
    "To: bob@example.test\r\n",
    "Subject: Hello\r\n",
    "MIME-Version: 1.0\r\n",
    "Content-Type: multipart/alternative; boundary=\"sendust-boundary-0001\"\r\n",
    "\r\n",
    "--sendust-boundary-0001\r\n",
    "Content-Type: text/plain; charset=utf-8\r\n",
    "Content-Transfer-Encoding: base64\r\n",
    "\r\n",
    "aGVsbG8gd29ybGQ=\r\n",
    "\r\n",
    "--sendust-boundary-0001\r\n",
    "Content-Type: text/html; charset=utf-8\r\n",
    "Content-Transfer-Encoding: base64\r\n",
    "\r\n",
    "PHA+aGk8L3A+\r\n",
    "\r\n",
    "--sendust-boundary-0001--\r\n",
);

#[test]
fn text_only_body_matches_the_golden_fixture() {
    let bytes = wire(Some("hello world"), None);
    assert_eq!(String::from_utf8(bytes).expect("utf8"), GOLDEN_TEXT_ONLY);
}

#[test]
fn alternative_body_matches_the_golden_fixture() {
    let bytes = wire(Some("hello world"), Some("<p>hi</p>"));
    assert_eq!(String::from_utf8(bytes).expect("utf8"), GOLDEN_ALTERNATIVE);
}

#[test]
fn html_only_body_is_text_html() {
    let wire = String::from_utf8(wire(None, Some("<p>hi</p>"))).expect("utf8");
    assert!(wire.contains("Content-Type: text/html; charset=utf-8\r\n"));
    assert!(wire.contains("PHA+aGk8L3A+\r\n"));
    assert!(!wire.contains("multipart/alternative"));
}

#[test]
fn empty_string_bodies_produce_an_empty_part() {
    let wire = String::from_utf8(wire(Some(""), None)).expect("utf8");
    assert!(wire.ends_with("Content-Transfer-Encoding: base64\r\n\r\n"));

    let wire = String::from_utf8(wire(Some(""), Some(""))).expect("utf8");
    assert!(wire.contains("multipart/alternative"));
    assert!(wire.contains("text/plain"));
    assert!(wire.contains("text/html"));
}

#[test]
fn line_endings_normalize_to_crlf_before_encoding() {
    let lf = wire(Some("line one\nline two"), None);
    let crlf = wire(Some("line one\r\nline two"), None);
    let cr = wire(Some("line one\rline two"), None);
    assert_eq!(lf, crlf);
    assert_eq!(lf, cr);
}

#[test]
fn a_message_without_a_body_is_rejected() {
    let error = serialize(&message(None, None), Limits::default(), false)
        .expect_err("no body");
    assert_eq!(error.code(), ErrorCode::ValidationInvalidOption);
}
