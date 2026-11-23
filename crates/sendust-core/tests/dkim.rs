//! DKIM conformance (ADR-0024).
//!
//! The verifier in this file is a second implementation: it re-derives the body
//! hash and the signed header block from the emitted bytes, using only RFC 6376
//! and the raw RSA primitive. If the signer and this verifier agree, the output
//! is a signature a receiver can validate, not merely bytes that round-trip
//! through the code that produced them.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::pkcs8::EncodePrivateKey;
use rsa::{Pkcs1v15Sign, RsaPrivateKey};
use sha2::{Digest as _, Sha256};

use sendust_core::{
    AttachmentPolicy, DEFAULT_SIGNED_HEADERS, DkimSigner, ErrorCode, Limits, Mailbox,
    OutgoingMessage, serialize_at,
};

/// The same throwaway 1024-bit RSA key the core unit tests use, as PKCS#1 DER.
const KEY_DER_B64: &str = "MIICXQIBAAKBgQC03ufx4VKPV3ayvt/DbXKSUaFZhHgldIUzLJDo1nJEVNjLNjsfoio1bunv7LN/BgO4c07HfDxQ1LS3VdwUKeJWQ/EO5vqlbGyhHWu0PoGFulE8dSZofA62RUg6Mo0fu7uziAUgPuMoSoS1sNfyRhoRgIvE+QeK/jYBGJ/2EgRKawIDAQABAoGBAKUsPvMECwo+HHbFmsD0TUHT7B48vB+wVgfStg7772P5e/Wv87RCunsLa/Wf2pewnWsvWg0QpmBGrzYR4aZETZ1hp+YvMGJrwCmiDvcPQUoBcYuNadmyHlmxUWNyjzKciY6cRuCvAMooPVzhtwFU1WSkSXgwRaN+LviNgXtc93ohAkEA6Ubg+xOIu/kc9OON2DfWYz/NMRdKyCiF2/Z+h3Z3LnThM8cXIwRCN0A9sBOgZkI74YKdo3olobpGbgWaWp4RewJBAMZ9MtcVA812ainWkmmN5nkbBTc2Uo8gOvpjVNj+0/6dco/3TMP4w4ArK+g7ZQ8yCO9GjvTBOgGHwiiPcnUkf9ECQQDoXZsDXmzoWPd4FCcZxlz+VfJNwRbTlFHqTnmw89c55mDhE6K/VaGjqZEJEDPJSJafRl1CFwzfMU6wj1I2i5d7AkArWikChRqBg72ISc9MsquGA/rNplW94QJAsfVUN8RBtk3RV+gyXWEF22gJnv9hpEIDUNvkZVewAeDMTxhmsYQhAkBh9lZSf2QlQh6kZGQaP6lKR9Oc9NKpxK1F5ySrEBG6wMia1X0PnPfdCodJs+NOm+UjJ6ipvTzqJrXD427tyVXi";

fn fixed() -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(1_700_000_000)
}

fn key() -> RsaPrivateKey {
    let der = STANDARD.decode(KEY_DER_B64).expect("fixture der");
    RsaPrivateKey::from_pkcs1_der(&der).expect("fixture key")
}

fn signer() -> DkimSigner {
    let pem = key()
        .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
        .expect("fixture pem");
    DkimSigner::new("example.test", "sel1", pem.as_str()).expect("fixture signer")
}

fn mailbox(value: &str) -> Mailbox {
    Mailbox::parse(value).expect("valid mailbox")
}

fn message() -> OutgoingMessage {
    OutgoingMessage {
        from: mailbox("Alice <alice@example.test>"),
        to: vec![mailbox("bob@example.test")],
        cc: vec![mailbox("carol@example.test")],
        bcc: vec![mailbox("hidden@example.test")],
        reply_to: Vec::new(),
        subject: "Grüße".to_string(),
        text: Some("hello world".to_string()),
        html: Some("<p>hello world</p>".to_string()),
        headers: Vec::new(),
        attachments: Vec::new(),
    }
}

/// The composed message the signer receives, without any signature.
fn wire() -> Vec<u8> {
    serialize_at(
        &message(),
        Limits::default(),
        AttachmentPolicy::Disabled,
        fixed(),
    )
    .expect("serialized")
}

/// Unsigned wire bytes for the given message, at the fixed instant.
fn wire_of(message: &OutgoingMessage) -> Vec<u8> {
    serialize_at(
        message,
        Limits::default(),
        AttachmentPolicy::Disabled,
        fixed(),
    )
    .expect("serialized")
}

/// RFC 6376 §3.4.2, written as a second implementation: unfold, then collapse
/// every whitespace run to one space and trim the ends.
fn relaxed_header(name: &str, value: &str) -> String {
    let unfolded = value.replace("\r\n", "");
    let collapsed = unfolded.split_whitespace().collect::<Vec<_>>().join(" ");
    format!("{}:{}", name.to_ascii_lowercase(), collapsed)
}

/// RFC 6376 §3.4.4 for the bodies this project composes: base64 lines, no
/// leading whitespace, CRLF endings. Trailing blank lines go and one CRLF stays.
fn relaxed_body(body: &str) -> String {
    let mut lines: Vec<String> = body
        .split("\r\n")
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect();
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    let mut out = lines.join("\r\n");
    if !out.is_empty() {
        out.push_str("\r\n");
    }
    out
}

/// Splits the header block into `(name, folded value)` pairs.
fn fields(head: &str) -> Vec<(String, String)> {
    let mut fields: Vec<(String, String)> = Vec::new();
    for line in head.split("\r\n") {
        if line.starts_with(' ') || line.starts_with('\t') {
            if let Some((_, value)) = fields.last_mut() {
                value.push_str("\r\n");
                value.push_str(line);
            }
            continue;
        }
        if let Some((name, value)) = line.split_once(':') {
            fields.push((name.trim().to_string(), value.to_string()));
        }
    }
    fields
}

/// The tag list of one DKIM-Signature value.
fn tags(value: &str) -> Vec<(String, String)> {
    value
        .replace("\r\n", "")
        .split(';')
        .filter_map(|tag| {
            let (name, value) = tag.split_once('=')?;
            Some((name.trim().to_ascii_lowercase(), value.trim().to_string()))
        })
        .collect()
}

fn tag<'a>(tags: &'a [(String, String)], name: &str) -> &'a str {
    tags.iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
        .unwrap_or_else(|| panic!("missing {name} tag"))
}

/// Rebuilds what the signer signed and verifies it with the raw RSA primitive.
fn verify(wire: &[u8]) {
    let text = String::from_utf8(wire.to_vec()).expect("utf8");
    let (head, body) = text.split_once("\r\n\r\n").expect("header block");
    let fields = fields(head);

    let (_, signature_value) = fields
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("DKIM-Signature"))
        .expect("a DKIM-Signature field");
    let tags = tags(signature_value);

    assert_eq!(tag(&tags, "v"), "1");
    assert_eq!(tag(&tags, "a"), "rsa-sha256");
    assert_eq!(tag(&tags, "c"), "relaxed/relaxed");
    assert_eq!(tag(&tags, "d"), "example.test");
    assert_eq!(tag(&tags, "s"), "sel1");

    let expected_body_hash = STANDARD.encode(Sha256::digest(relaxed_body(body).as_bytes()));
    assert_eq!(tag(&tags, "bh"), expected_body_hash, "body hash");

    let mut data = String::new();
    let signed: Vec<&str> = tag(&tags, "h").split(':').collect();
    assert_eq!(signed.first(), Some(&"from"), "From must always be signed");
    for name in &signed {
        let (field_name, value) = fields
            .iter()
            .rev()
            .find(|(field, _)| field.eq_ignore_ascii_case(name))
            .unwrap_or_else(|| panic!("signed field {name} is missing"));
        data.push_str(&relaxed_header(field_name, value));
        data.push_str("\r\n");
    }

    // The signature covers its own field with an empty `b=` value.
    let mut own = relaxed_header("DKIM-Signature", signature_value);
    let cut = own.rfind("; b=").expect("b tag");
    own.truncate(cut + 4);
    data.push_str(&own);

    let signature = STANDARD.decode(tag(&tags, "b")).expect("base64 signature");
    let digest = Sha256::digest(data.as_bytes());
    key()
        .to_public_key()
        .verify(Pkcs1v15Sign::new::<Sha256>(), &digest, &signature)
        .expect("the signature verifies against the public key");
}

#[test]
fn a_signed_message_verifies() {
    let signed = signer().sign(&wire(), fixed()).expect("signed");
    verify(&signed);
}

#[test]
fn a_message_without_attachments_and_with_attachments_verifies() {
    let mut with_attachment = message();
    with_attachment.attachments = vec![sendust_core::Attachment::Bytes {
        filename: "notes.txt".to_string(),
        content_type: Some("text/plain".to_string()),
        bytes: b"attachment body".to_vec(),
    }];
    let signed = signer()
        .sign(&wire_of(&with_attachment), fixed())
        .expect("signed");
    verify(&signed);

    let mut empty_body = message();
    empty_body.text = Some(String::new());
    empty_body.html = None;
    let signed = signer()
        .sign(&wire_of(&empty_body), fixed())
        .expect("signed");
    verify(&signed);
}

#[test]
fn a_custom_header_set_verifies() {
    let signer = signer()
        .with_headers(&[
            "from".to_string(),
            "to".to_string(),
            "subject".to_string(),
            "date".to_string(),
        ])
        .expect("headers");
    let signed = signer.sign(&wire(), fixed()).expect("signed");
    verify(&signed);

    let text = String::from_utf8(signed).expect("utf8");
    assert!(text.contains("h=from:to:subject:date;"), "{text}");
}

#[test]
fn bcc_is_signed_never_carried_and_never_listed() {
    let signed = signer().sign(&wire(), fixed()).expect("signed");
    let text = String::from_utf8(signed).expect("utf8");
    assert!(text.contains("h=from:to:cc:"), "{text}");
    assert!(!text.to_ascii_lowercase().contains("hidden@example.test"));
    verify(text.as_bytes());
}

#[test]
fn the_default_header_set_is_the_documented_one() {
    assert_eq!(
        DEFAULT_SIGNED_HEADERS,
        &[
            "from",
            "to",
            "cc",
            "reply-to",
            "subject",
            "date",
            "message-id",
            "mime-version",
            "content-type",
            "content-transfer-encoding",
        ]
    );
    assert_eq!(
        signer()
            .headers()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        DEFAULT_SIGNED_HEADERS
    );
}

#[test]
fn an_unusable_key_is_a_stable_error() {
    assert_eq!(
        DkimSigner::new("example.test", "sel1", "not a pem")
            .expect_err("bad key")
            .code(),
        ErrorCode::ConfigInvalidDkimKey
    );
}

#[test]
fn a_signature_changes_with_the_message() {
    let signer = signer();
    let first = signer.sign(&wire(), fixed()).expect("first");

    let mut other = message();
    other.subject = "Something else".to_string();
    let second = signer.sign(&wire_of(&other), fixed()).expect("second");

    assert_ne!(first, second);
    verify(&first);
    verify(&second);
}
