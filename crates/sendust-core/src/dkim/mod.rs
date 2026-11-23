//! DKIM signing (RFC 6376, ADR-0024).
//!
//! Signing runs on the composed wire bytes and inserts exactly one
//! `DKIM-Signature` field, so the transport keeps sending bytes that are already
//! signed and `DeliveryRequest::wire` stays the single source of truth.
//!
//! Both canonicalizations are relaxed, the algorithm is `rsa-sha256`, and the
//! signature is RSASSA-PKCS1-v1_5 over the canonicalized header block — the
//! signature every verifier accepts, including relays that reject `rsa-sha1`.
//!
//! The module owns no DNS, no key store and no clock: the caller supplies an RSA
//! private key, and the instant is the one the delivery path already stamped
//! into `Date` and `Message-ID`. A message that cannot be signed fails before
//! the relay is contacted, never after `DATA`.

use std::fmt;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use rsa::RsaPrivateKey;
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::pkcs1v15::SigningKey;
use rsa::pkcs8::DecodePrivateKey;
use rsa::signature::{SignatureEncoding, Signer};
use rsa::traits::PublicKeyParts;
use sha2::{Digest as _, Sha256};

use crate::errors::{Error, ErrorCode};
use crate::mime::{CRLF, FOLD_WIDTH, MAX_LINE};

/// The one header field this module adds.
const SIGNATURE_HEADER: &str = "DKIM-Signature";

/// Header fields signed when the caller names none, in signing order.
///
/// `From` is mandatory (RFC 6376 §5.4). The rest are the fields a receiver
/// shows or that describe the body, so a change to any of them breaks the
/// signature. A field the message does not carry is skipped, never invented.
pub const DEFAULT_SIGNED_HEADERS: &[&str] = &[
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
];

/// Fields a signer must not list in `h=` (RFC 6376 §5.4): they are added or
/// rewritten in transit, so including one would make every signature fail.
const RESTRICTED_HEADERS: &[&str] = &[
    "return-path",
    "received",
    "comments",
    "keywords",
    "bcc",
    "resent-bcc",
    "dkim-signature",
];

/// Smallest key size accepted: RFC 8301 forbids signing with less than 1024 bits.
const MIN_KEY_BITS: usize = 1024;
/// Largest key size accepted, so one `b=` value stays on a legal header line.
const MAX_KEY_BITS: usize = 4096;

/// One DKIM signing identity: a domain, a selector, and an RSA private key.
///
/// The type is cheap to clone — the parsed key is shared — so a transport can
/// hold one and hand it to every send.
#[derive(Clone)]
pub struct DkimSigner {
    domain: String,
    selector: String,
    key: Arc<RsaPrivateKey>,
    headers: Vec<String>,
}

impl DkimSigner {
    /// Parses the key immediately: a bad key fails where the caller built the
    /// signer, not on the first send.
    ///
    /// The PEM may be PKCS#1 (`BEGIN RSA PRIVATE KEY`) or PKCS#8
    /// (`BEGIN PRIVATE KEY`).
    pub fn new(
        domain: impl Into<String>,
        selector: impl Into<String>,
        private_key_pem: &str,
    ) -> Result<Self, Error> {
        let domain = domain.into();
        let selector = selector.into();
        if !is_domain(&domain) || !is_selector(&selector) {
            return Err(Error::new(ErrorCode::ValidationInvalidOption));
        }

        let key = parse_key(private_key_pem)?;
        let bits = key.n().bits();
        if !(MIN_KEY_BITS..=MAX_KEY_BITS).contains(&bits) {
            return Err(Error::new(ErrorCode::ConfigInvalidDkimKey));
        }

        Ok(Self {
            domain,
            selector,
            key: Arc::new(key),
            headers: DEFAULT_SIGNED_HEADERS
                .iter()
                .map(|name| (*name).to_string())
                .collect(),
        })
    }

    /// Replaces the signed header set.
    ///
    /// Names are case-insensitive and the order is the order they are hashed and
    /// listed in `h=`. `From` is always signed, even when the caller leaves it
    /// out, and a field RFC 6376 forbids is rejected instead of ignored.
    pub fn with_headers(mut self, headers: &[String]) -> Result<Self, Error> {
        if headers.is_empty() {
            return Err(Error::new(ErrorCode::ValidationInvalidOption));
        }

        let mut selected: Vec<String> = Vec::with_capacity(headers.len() + 1);
        for name in headers {
            let parsed = crate::message::HeaderName::parse(name.trim())?;
            let lowered = parsed.as_str().to_ascii_lowercase();
            if RESTRICTED_HEADERS
                .iter()
                .any(|restricted| *restricted == lowered)
            {
                return Err(Error::new(ErrorCode::ValidationInvalidHeader));
            }
            if !selected.contains(&lowered) {
                selected.push(lowered);
            }
        }
        if !selected.iter().any(|name| name == "from") {
            selected.insert(0, "from".to_string());
        }

        self.headers = selected;
        Ok(self)
    }

    pub fn domain(&self) -> &str {
        &self.domain
    }

    pub fn selector(&self) -> &str {
        &self.selector
    }

    /// Signed header field names, lowercased, in signing order.
    pub fn headers(&self) -> &[String] {
        &self.headers
    }

    /// Returns `wire` with one `DKIM-Signature` field prepended.
    ///
    /// The same bytes and the same instant always produce the same signature, so
    /// a caller can pin the output in a test.
    pub fn sign(&self, wire: &[u8], now: SystemTime) -> Result<Vec<u8>, Error> {
        let text = std::str::from_utf8(wire)
            .map_err(|_| Error::new(ErrorCode::ValidationInvalidHeader))?;
        let separator = text
            .find("\r\n\r\n")
            .ok_or_else(|| Error::new(ErrorCode::ValidationInvalidHeader))?;
        let head = &text[..separator];
        let body = &text.as_bytes()[separator + 4..];

        let fields = parse_fields(head)?;
        let selected = self.select(&fields)?;
        let signed_names = selected
            .iter()
            .map(|(name, _)| name.to_ascii_lowercase())
            .collect::<Vec<_>>()
            .join(":");
        let body_hash = STANDARD.encode(Sha256::digest(canonical_body(body)).as_slice());
        let timestamp = now
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs())
            .unwrap_or_default();

        // `b=` is last and empty while hashing; the signature replaces nothing
        // because the value was never part of the hash.
        let unsigned = format!(
            "v=1; a=rsa-sha256; c=relaxed/relaxed; d={}; s={}; t={timestamp}; h={signed_names}; bh={body_hash}; b=",
            self.domain, self.selector
        );

        let mut signed = Vec::new();
        for (name, value) in &selected {
            signed.extend_from_slice(relaxed_header(name, value).as_bytes());
            signed.extend_from_slice(CRLF.as_bytes());
        }
        signed.extend_from_slice(relaxed_header(SIGNATURE_HEADER, &unsigned).as_bytes());

        let key = SigningKey::<Sha256>::new((*self.key).clone());
        let signature = key
            .try_sign(&signed)
            .map_err(|_| Error::new(ErrorCode::ConfigInvalidDkimKey))?;
        let value = format!("{unsigned}{}", STANDARD.encode(signature.to_vec()));
        let header = fold_header(SIGNATURE_HEADER, &value)?;

        let mut out = Vec::with_capacity(header.len() + wire.len());
        out.extend_from_slice(&header);
        out.extend_from_slice(wire);
        Ok(out)
    }

    /// The fields this signature covers, taken from the message itself.
    ///
    /// The last instance of a field is the one a receiver displays, so it is the
    /// one that gets signed (RFC 6376 §5.4.2).
    fn select(&self, fields: &[(String, String)]) -> Result<Vec<(String, String)>, Error> {
        let mut selected = Vec::with_capacity(self.headers.len());
        for wanted in &self.headers {
            if let Some((name, value)) = fields
                .iter()
                .rev()
                .find(|(name, _)| name.eq_ignore_ascii_case(wanted))
            {
                selected.push((name.clone(), value.clone()));
            }
        }
        if !selected
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("from"))
        {
            return Err(Error::new(ErrorCode::ValidationMissingFrom));
        }
        Ok(selected)
    }
}

impl fmt::Debug for DkimSigner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DkimSigner")
            .field("domain", &self.domain)
            .field("selector", &self.selector)
            .field("key", &"<redacted>")
            .field("headers", &self.headers)
            .finish()
    }
}

/// A parsed header field: its name, and its still-folded value.
type Field = (String, String);

fn parse_key(pem: &str) -> Result<RsaPrivateKey, Error> {
    let invalid = || Error::new(ErrorCode::ConfigInvalidDkimKey);
    if !pem.contains("-----BEGIN") || pem.contains("ENCRYPTED") {
        return Err(invalid());
    }
    RsaPrivateKey::from_pkcs8_pem(pem)
        .or_else(|_| RsaPrivateKey::from_pkcs1_pem(pem))
        .map_err(|_| invalid())
}

/// Splits the header block into fields, keeping each fold so the relaxed
/// canonicalization can unfold it and collapse the whitespace it carried.
fn parse_fields(head: &str) -> Result<Vec<Field>, Error> {
    let mut fields: Vec<Field> = Vec::new();
    for line in head.split(CRLF) {
        if line.starts_with(' ') || line.starts_with('\t') {
            match fields.last_mut() {
                Some((_, value)) => {
                    value.push_str(CRLF);
                    value.push_str(line);
                }
                None => return Err(Error::new(ErrorCode::ValidationInvalidHeader)),
            }
            continue;
        }
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| Error::new(ErrorCode::ValidationInvalidHeader))?;
        fields.push((name.trim().to_string(), value.to_string()));
    }
    Ok(fields)
}

/// RFC 6376 §3.4.2: lowercase the name, unfold, collapse whitespace runs to one
/// space, and drop the whitespace around the colon and at the end of the value.
fn relaxed_header(name: &str, value: &str) -> String {
    let mut out = String::with_capacity(name.len() + value.len() + 1);
    out.push_str(&name.to_ascii_lowercase());
    out.push(':');

    let mut pending = false;
    let mut started = false;
    for character in value.chars() {
        match character {
            // Unfolding removes the line break; the space that followed it is
            // then just whitespace between tokens.
            '\r' | '\n' => {}
            ' ' | '\t' => pending = true,
            other => {
                if pending && started {
                    out.push(' ');
                }
                pending = false;
                out.push(other);
                started = true;
            }
        }
    }
    out
}

/// RFC 6376 §3.4.4: one space for a whitespace run, no trailing whitespace, no
/// empty lines at the end, and a single CRLF after a non-empty body.
fn canonical_body(body: &[u8]) -> Vec<u8> {
    let mut lines: Vec<Vec<u8>> = Vec::new();
    for raw in body.split(|byte| *byte == b'\n') {
        let line = if raw.last() == Some(&b'\r') {
            &raw[..raw.len() - 1]
        } else {
            raw
        };
        lines.push(relaxed_line(line));
    }
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }

    let mut out = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            out.extend_from_slice(CRLF.as_bytes());
        }
        out.extend_from_slice(line);
    }
    if !out.is_empty() {
        out.extend_from_slice(CRLF.as_bytes());
    }
    out
}

fn relaxed_line(line: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(line.len());
    let mut pending = false;
    for byte in line {
        if *byte == b' ' || *byte == b'\t' {
            pending = true;
            continue;
        }
        if pending {
            out.push(b' ');
            pending = false;
        }
        out.push(*byte);
    }
    out
}

/// Renders the signature field, folding between tags only.
///
/// A fold replaces one of the spaces after a `;`, which relaxed canonicalization
/// turns back into that same space, so the folded field canonicalizes to the
/// bytes that were signed.
fn fold_header(name: &str, value: &str) -> Result<Vec<u8>, Error> {
    let tags: Vec<&str> = value.split("; ").collect();
    let mut out = String::with_capacity(value.len() + 32);
    out.push_str(name);
    out.push_str(": ");
    let mut line_len = name.len() + 2;

    for (index, tag) in tags.iter().enumerate() {
        let piece = if index + 1 < tags.len() {
            format!("{tag};")
        } else {
            (*tag).to_string()
        };
        if index > 0 {
            if line_len + 1 + piece.len() > FOLD_WIDTH {
                out.push_str(CRLF);
                out.push(' ');
                line_len = 1;
            } else {
                out.push(' ');
                line_len += 1;
            }
        }
        if line_len + piece.len() > MAX_LINE {
            return Err(Error::new(ErrorCode::ValidationInvalidHeader));
        }
        out.push_str(&piece);
        line_len += piece.len();
    }

    out.push_str(CRLF);
    Ok(out.into_bytes())
}

/// A DKIM `d=` value: dot-separated labels of letters, digits and hyphens.
fn is_domain(domain: &str) -> bool {
    domain.len() <= 253
        && domain.contains('.')
        && domain.split('.').all(is_label)
        && !domain.starts_with('.')
        && !domain.ends_with('.')
}

/// A DKIM `s=` value: one label, because a selector has no dot.
fn is_selector(selector: &str) -> bool {
    is_label(selector) && !selector.contains('.')
}

fn is_label(label: &str) -> bool {
    !label.is_empty()
        && label.len() <= 63
        && !label.starts_with('-')
        && !label.ends_with('-')
        && label
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
}

#[cfg(test)]
pub(crate) mod test_support {
    //! Fixture key shared by the core unit tests and the delivery tests.

    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD;
    use rsa::RsaPrivateKey;
    use rsa::pkcs1::DecodeRsaPrivateKey;
    use rsa::pkcs8::{EncodePrivateKey, LineEnding};

    use super::DkimSigner;

    /// A throwaway 1024-bit RSA key in PKCS#1 DER — the form
    /// `openssl genpkey -algorithm RSA -outform DER` writes — base64 without line
    /// breaks.
    ///
    /// 1024 bits is the smallest size the signer accepts, which keeps the
    /// fixture short; it protects nothing and exists only for tests.
    pub(crate) const KEY_DER_B64: &str = "MIICXQIBAAKBgQC03ufx4VKPV3ayvt/DbXKSUaFZhHgldIUzLJDo1nJEVNjLNjsfoio1bunv7LN/BgO4c07HfDxQ1LS3VdwUKeJWQ/EO5vqlbGyhHWu0PoGFulE8dSZofA62RUg6Mo0fu7uziAUgPuMoSoS1sNfyRhoRgIvE+QeK/jYBGJ/2EgRKawIDAQABAoGBAKUsPvMECwo+HHbFmsD0TUHT7B48vB+wVgfStg7772P5e/Wv87RCunsLa/Wf2pewnWsvWg0QpmBGrzYR4aZETZ1hp+YvMGJrwCmiDvcPQUoBcYuNadmyHlmxUWNyjzKciY6cRuCvAMooPVzhtwFU1WSkSXgwRaN+LviNgXtc93ohAkEA6Ubg+xOIu/kc9OON2DfWYz/NMRdKyCiF2/Z+h3Z3LnThM8cXIwRCN0A9sBOgZkI74YKdo3olobpGbgWaWp4RewJBAMZ9MtcVA812ainWkmmN5nkbBTc2Uo8gOvpjVNj+0/6dco/3TMP4w4ArK+g7ZQ8yCO9GjvTBOgGHwiiPcnUkf9ECQQDoXZsDXmzoWPd4FCcZxlz+VfJNwRbTlFHqTnmw89c55mDhE6K/VaGjqZEJEDPJSJafRl1CFwzfMU6wj1I2i5d7AkArWikChRqBg72ISc9MsquGA/rNplW94QJAsfVUN8RBtk3RV+gyXWEF22gJnv9hpEIDUNvkZVewAeDMTxhmsYQhAkBh9lZSf2QlQh6kZGQaP6lKR9Oc9NKpxK1F5ySrEBG6wMia1X0PnPfdCodJs+NOm+UjJ6ipvTzqJrXD427tyVXi";

    /// The fixture key as a PKCS#8 PEM, the form the facade passes in.
    pub(crate) fn pem() -> String {
        let der = STANDARD.decode(KEY_DER_B64).expect("fixture der");
        let key = RsaPrivateKey::from_pkcs1_der(&der).expect("fixture key");
        let pem = key.to_pkcs8_pem(LineEnding::LF).expect("fixture pem");
        pem.as_str().to_string()
    }

    /// A signer for `example.test` with the default header set.
    pub(crate) fn signer() -> DkimSigner {
        DkimSigner::new("example.test", "sel1", &pem()).expect("fixture signer")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attachments::AttachmentPolicy;
    use crate::config::Limits;
    use crate::message::OutgoingMessage;
    use std::time::Duration;

    fn fixed() -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(1_700_000_000)
    }

    fn message() -> OutgoingMessage {
        OutgoingMessage {
            from: "Alice <alice@example.test>".parse().expect("valid"),
            to: vec!["bob@example.test".parse().expect("valid")],
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

    fn wire() -> Vec<u8> {
        crate::mime::serialize_at(
            &message(),
            Limits::default(),
            AttachmentPolicy::Disabled,
            fixed(),
        )
        .expect("serialized")
    }

    fn signed() -> String {
        let bytes = test_support::signer()
            .sign(&wire(), fixed())
            .expect("signed");
        String::from_utf8(bytes).expect("utf8")
    }

    #[test]
    fn relaxes_a_header_value_as_rfc_6376_describes() {
        assert_eq!(relaxed_header("A", " X"), "a:X");
        assert_eq!(relaxed_header("B", " Y\t\r\n\tZ  "), "b:Y Z");
        assert_eq!(
            relaxed_header("From", " Alice <a@b.test>"),
            "from:Alice <a@b.test>"
        );
    }

    #[test]
    fn relaxes_a_body_as_rfc_6376_describes() {
        assert_eq!(canonical_body(b"Hello World.\r\n"), b"Hello World.\r\n");
        assert_eq!(canonical_body(b"a  b\t\r\n"), b"a b\r\n");
        assert_eq!(canonical_body(b"a\r\n\r\n\r\n"), b"a\r\n");
        assert_eq!(canonical_body(b""), b"");
        assert_eq!(canonical_body(b"\r\n"), b"");
        assert_eq!(canonical_body(b"a"), b"a\r\n");
    }

    #[test]
    fn signs_a_message_and_names_the_covered_headers() {
        let text = signed();
        assert!(
            text.starts_with(
                "DKIM-Signature: v=1; a=rsa-sha256; c=relaxed/relaxed; d=example.test; s=sel1;"
            ),
            "{text}"
        );
        assert!(text.contains(" t=1700000000;"), "{text}");
        assert!(
            text.contains(
                "h=from:to:subject:date:message-id:mime-version:content-type:content-transfer-encoding;"
            ),
            "{text}"
        );
        assert!(text.contains("bh="), "{text}");
        assert!(text.contains("b="), "{text}");
        assert!(
            text.contains("\r\nFrom: \"Alice\" <alice@example.test>"),
            "{text}"
        );
    }

    #[test]
    fn every_line_of_the_signature_is_legal() {
        let text = signed();
        for line in text.split("\r\n") {
            assert!(line.len() <= MAX_LINE, "line of {} bytes", line.len());
        }
        let header = text.split("\r\n\r\n").next().expect("headers");
        assert!(header.contains("\r\n "), "the signature must be folded");
    }

    #[test]
    fn is_deterministic_for_one_instant_and_changes_with_it() {
        let signer = test_support::signer();
        let first = signer.sign(&wire(), fixed()).expect("first");
        let second = signer.sign(&wire(), fixed()).expect("second");
        assert_eq!(first, second);

        let later = signer
            .sign(&wire(), fixed() + Duration::from_secs(1))
            .expect("later");
        assert_ne!(first, later);
    }

    #[test]
    fn signs_the_from_field_even_when_the_caller_leaves_it_out() {
        let signer = test_support::signer()
            .with_headers(&["subject".to_string()])
            .expect("headers");
        assert_eq!(
            signer.headers(),
            ["from".to_string(), "subject".to_string()]
        );

        let text = String::from_utf8(signer.sign(&wire(), fixed()).expect("signed")).expect("utf8");
        assert!(text.contains("h=from:subject;"), "{text}");
    }

    #[test]
    fn skips_a_signed_field_the_message_does_not_carry() {
        let signer = test_support::signer()
            .with_headers(&["from".to_string(), "cc".to_string(), "subject".to_string()])
            .expect("headers");
        let text = String::from_utf8(signer.sign(&wire(), fixed()).expect("signed")).expect("utf8");
        assert!(text.contains("h=from:subject;"), "{text}");
        assert!(!text.contains("h=from:subject:cc"), "{text}");
    }

    #[test]
    fn rejects_a_field_rfc_6376_forbids_signing() {
        for name in ["bcc", "Received", "DKIM-Signature", "Return-Path"] {
            let error = test_support::signer()
                .with_headers(&[name.to_string()])
                .expect_err("must be rejected");
            assert_eq!(error.code(), ErrorCode::ValidationInvalidHeader, "{name}");
        }
        assert_eq!(
            test_support::signer()
                .with_headers(&[])
                .expect_err("empty")
                .code(),
            ErrorCode::ValidationInvalidOption
        );
    }

    #[test]
    fn rejects_an_invalid_domain_selector_or_key() {
        let pem = test_support::pem();
        for domain in [
            "",
            "example",
            ".example.test",
            "example.test.",
            "exa mple.test",
        ] {
            assert_eq!(
                DkimSigner::new(domain, "sel1", &pem)
                    .expect_err("domain")
                    .code(),
                ErrorCode::ValidationInvalidOption,
                "{domain:?}"
            );
        }
        for selector in ["", "sel.1", "-sel", "sel_1"] {
            assert_eq!(
                DkimSigner::new("example.test", selector, &pem)
                    .expect_err("selector")
                    .code(),
                ErrorCode::ValidationInvalidOption,
                "{selector:?}"
            );
        }
        for key in [
            "",
            "not a pem",
            "-----BEGIN PRIVATE KEY-----\nnope\n-----END PRIVATE KEY-----",
        ] {
            assert_eq!(
                DkimSigner::new("example.test", "sel1", key)
                    .expect_err("key")
                    .code(),
                ErrorCode::ConfigInvalidDkimKey,
                "{key:?}"
            );
        }
    }

    #[test]
    fn rejects_a_key_outside_the_supported_size() {
        let key = rsa::RsaPrivateKey::from_pkcs1_der(
            &STANDARD
                .decode(test_support::KEY_DER_B64)
                .expect("fixture der"),
        )
        .expect("fixture key");

        // The fixture sits exactly on the floor, and the window has two ends.
        assert_eq!(key.n().bits(), MIN_KEY_BITS);
        assert!((MIN_KEY_BITS..=MAX_KEY_BITS).contains(&key.n().bits()));
        assert!(!(MIN_KEY_BITS..=MAX_KEY_BITS).contains(&(MIN_KEY_BITS - 1)));
        assert!(!(MIN_KEY_BITS..=MAX_KEY_BITS).contains(&(MAX_KEY_BITS + 1)));
    }

    #[test]
    fn accepts_a_pkcs1_key_as_well() {
        use rsa::pkcs1::EncodeRsaPrivateKey;
        let pem = test_support::pem();
        let key = RsaPrivateKey::from_pkcs8_pem(&pem).expect("fixture key");
        let pkcs1 = key
            .to_pkcs1_pem(rsa::pkcs1::LineEnding::LF)
            .expect("pkcs1 pem");
        assert!(pkcs1.contains("BEGIN RSA PRIVATE KEY"));
        assert!(DkimSigner::new("example.test", "sel1", pkcs1.as_str()).is_ok());
    }

    #[test]
    fn rejects_wire_that_is_not_a_message() {
        for bytes in [
            b"".as_slice(),
            b"no separator".as_slice(),
            b"\r\n\r\n".as_slice(),
        ] {
            let error = test_support::signer()
                .sign(bytes, fixed())
                .expect_err("not a message");
            assert_eq!(error.code(), ErrorCode::ValidationInvalidHeader);
        }
    }

    #[test]
    fn never_renders_the_key() {
        let rendered = format!("{:?}", test_support::signer());
        assert!(rendered.contains("example.test"));
        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains("PRIVATE"));
    }
}
