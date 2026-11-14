//! Independent MIME parser conformance (P07-005).
//!
//! `mail-parser` is a dev-only dependency: it re-reads the emitted bytes with a
//! second implementation, so the composer and an independent parser must agree.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use mail_parser::{MessageParser, MimeHeaders};
use sendust_core::{Attachment, Limits, Mailbox, OutgoingMessage, serialize_at};

const NOW: u64 = 1_700_000_000;

fn fixed() -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(NOW)
}

fn mailbox(value: &str) -> Mailbox {
    value.parse().expect("valid mailbox")
}

fn message() -> OutgoingMessage {
    OutgoingMessage {
        from: mailbox("Alice <alice@example.test>"),
        to: vec![mailbox("bob@example.test")],
        cc: Vec::new(),
        bcc: vec![mailbox("hidden@example.test")],
        reply_to: Vec::new(),
        subject: "Hello".to_string(),
        text: Some("hello world".to_string()),
        html: None,
        headers: Vec::new(),
        attachments: Vec::new(),
    }
}

fn wire(message: &OutgoingMessage) -> Vec<u8> {
    serialize_at(message, Limits::default(), false, fixed()).expect("serialized")
}

#[test]
fn parses_a_text_only_message() {
    let bytes = wire(&message());
    let parsed = MessageParser::default().parse(&bytes).expect("parsed");

    let from = parsed.from().expect("from").first().expect("from address");
    assert_eq!(from.address.as_deref(), Some("alice@example.test"));
    assert_eq!(from.name.as_deref(), Some("Alice"));

    let to = parsed.to().expect("to").first().expect("to address");
    assert_eq!(to.address.as_deref(), Some("bob@example.test"));

    assert_eq!(parsed.subject(), Some("Hello"));
    assert_eq!(
        parsed.body_text(0).expect("body").trim(),
        "hello world"
    );

    assert!(
        parsed
            .date()
            .expect("date")
            .to_rfc3339()
            .starts_with("2023-11-14T22:13:20")
    );

    let id = parsed.message_id().expect("message id");
    assert!(id.starts_with("1700000000."), "{id}");
    assert!(id.ends_with("@example.test"), "{id}");

    assert!(
        !String::from_utf8_lossy(&bytes).contains("hidden@example.test"),
        "BCC must never appear"
    );
}

#[test]
fn parses_alternative_bodies_and_unicode_subjects() {
    let mut message = message();
    message.subject = "Grüße".to_string();
    message.html = Some("<p>hi</p>".to_string());

    let bytes = wire(&message);
    let parsed = MessageParser::default().parse(&bytes).expect("parsed");

    assert_eq!(parsed.subject(), Some("Grüße"));
    assert!(parsed.body_text(0).expect("text").contains("hello world"));
    assert!(parsed.body_html(0).expect("html").contains("<p>hi</p>"));
}

#[test]
fn parses_an_attachment_with_a_unicode_filename() {
    let mut message = message();
    message.attachments = vec![Attachment::Bytes {
        filename: "Grüße.pdf".to_string(),
        content_type: Some("application/pdf".to_string()),
        bytes: b"pdf".to_vec(),
    }];

    let bytes = wire(&message);
    let parsed = MessageParser::default().parse(&bytes).expect("parsed");

    let attachments: Vec<_> = parsed.attachments().collect();
    assert_eq!(attachments.len(), 1);

    let attachment = attachments[0];
    assert_eq!(attachment.attachment_name(), Some("Grüße.pdf"));
    assert!(attachment.is_content_type("application", "pdf"));
}

#[test]
fn parses_utf8_bodies_without_loss() {
    let mut message = message();
    message.text = Some("héllo".to_string());

    let bytes = wire(&message);
    let parsed = MessageParser::default().parse(&bytes).expect("parsed");

    assert_eq!(parsed.body_text(0).expect("body").trim(), "héllo");
}
