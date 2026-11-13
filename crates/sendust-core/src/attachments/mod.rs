//! Attachment input policy, bounded buffer handling, and safe file reads.
//!
//! P04-003 owns descriptor validation and byte caps for in-memory attachments.
//! Reading file attachments safely, and enforcing their byte caps at read time,
//! belongs to P08; until then a file descriptor is policy-checked but not opened.

use crate::config::Limits;
use crate::errors::{Error, ErrorCode};

/// Maximum filename length accepted by the model.
const MAX_FILENAME: usize = 255;

/// An attachment supplied by the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Attachment {
    /// Bytes already supplied by the caller; copied into owned memory.
    Bytes {
        filename: String,
        content_type: Option<String>,
        bytes: Vec<u8>,
    },
    /// A regular file to read later, only when filesystem access is enabled.
    File(FileDescriptor),
}

/// Alias kept explicit for readability in match arms and tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDescriptor {
    pub display_name: Option<String>,
    pub content_type: Option<String>,
    pub path: String,
}

impl Attachment {
    /// Raw byte length when it is already known.
    pub fn known_len(&self) -> Option<u64> {
        match self {
            Self::Bytes { bytes, .. } => Some(bytes.len() as u64),
            Self::File(_) => None,
        }
    }

    /// Display name, which is safe to echo in errors.
    pub fn display_name(&self) -> Option<&str> {
        match self {
            Self::Bytes { filename, .. } => Some(filename),
            Self::File(file) => file.display_name.as_deref(),
        }
    }

    pub fn content_type(&self) -> Option<&str> {
        match self {
            Self::Bytes { content_type, .. } => content_type.as_deref(),
            Self::File(file) => file.content_type.as_deref(),
        }
    }
}

/// Validates attachment descriptors and returns the raw byte total for
/// attachments whose bytes are already known.
///
/// File attachments contribute zero until P08 reads them, which is why the wire
/// check is always performed on the final serialized bytes.
pub fn validate(
    attachments: &[Attachment],
    limits: Limits,
    filesystem_enabled: bool,
) -> Result<u64, Error> {
    let mut total: u64 = 0;

    for attachment in attachments {
        validate_descriptor(attachment, filesystem_enabled)?;

        if let Some(len) = attachment.known_len() {
            if len > limits.max_attachment_bytes() {
                return Err(Error::new(ErrorCode::LimitAttachmentBytes));
            }
            total = total.saturating_add(len);
            if total > limits.max_total_attachment_bytes() {
                return Err(Error::new(ErrorCode::LimitAttachmentTotal));
            }
        }
    }

    Ok(total)
}

fn invalid_attachment() -> Error {
    Error::new(ErrorCode::ValidationInvalidAttachment)
}

fn normalize_filename(raw: &str) -> Result<String, Error> {
    let name = raw.trim();
    if name.is_empty() || name.len() > MAX_FILENAME {
        return Err(invalid_attachment());
    }
    if name.chars().any(char::is_control) {
        return Err(Error::new(ErrorCode::ValidationHeaderInjection));
    }
    // A filename is a display name, never a path: separators are rejected so
    // nothing can look like a directory traversal or leak an absolute path.
    if name.contains('/') || name.contains('\\') {
        return Err(invalid_attachment());
    }
    if !name.is_ascii() {
        // RFC 2231 filename encoding arrives in P07; ASCII-only until then.
        return Err(invalid_attachment());
    }
    Ok(name.to_string())
}

fn normalize_content_type(raw: &str) -> Result<String, Error> {
    let value = raw.trim();
    let (kind, subtype) = value.split_once('/').ok_or_else(invalid_attachment)?;
    if kind.is_empty() || subtype.is_empty() || subtype.contains('/') {
        return Err(invalid_attachment());
    }
    if !kind.chars().all(is_token_character) || !subtype.chars().all(is_token_character) {
        return Err(invalid_attachment());
    }
    Ok(value.to_ascii_lowercase())
}

fn is_field_name_character(character: char) -> bool {
    character.is_ascii_alphanumeric()
        || matches!(
            character,
            '!' | '#' | '$' | '&' | '-' | '^' | '_' | '.' | '+'
        )
}

fn is_token_character(character: char) -> bool {
    is_field_name_character(character)
}

fn validate_descriptor(attachment: &Attachment, filesystem_enabled: bool) -> Result<(), Error> {
    match attachment {
        Attachment::Bytes {
            filename,
            content_type,
            ..
        } => {
            normalize_filename(filename)?;
            if let Some(value) = content_type {
                normalize_content_type(value)?;
            }
            Ok(())
        }
        Attachment::File(file) => {
            if !filesystem_enabled {
                return Err(Error::new(ErrorCode::ConfigFileAccessDisabled));
            }
            if let Some(name) = &file.display_name {
                normalize_filename(name)?;
            }
            if let Some(value) = &file.content_type {
                normalize_content_type(value)?;
            }
            if file.path.trim().is_empty() {
                return Err(invalid_attachment());
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::ErrorCode;

    fn bytes(filename: &str, len: u64) -> Attachment {
        Attachment::Bytes {
            filename: filename.to_string(),
            content_type: Some("application/octet-stream".to_string()),
            bytes: vec![0u8; len as usize],
        }
    }

    fn file(name: Option<&str>) -> Attachment {
        Attachment::File(FileDescriptor {
            display_name: name.map(str::to_string),
            content_type: Some("application/pdf".to_string()),
            path: "/tmp/does-not-exist".to_string(),
        })
    }

    #[test]
    fn accepts_a_buffer_attachment() {
        let total = validate(&[bytes("report.pdf", 3)], Limits::default(), false).expect("valid");
        assert_eq!(total, 3);
    }

    #[test]
    fn enforces_the_per_file_cap_inclusively() {
        let limits = Limits::default();
        assert!(
            validate(
                &[bytes("at-cap", limits.max_attachment_bytes())],
                limits,
                false
            )
            .is_ok()
        );
        assert_eq!(
            code(&[bytes("over", limits.max_attachment_bytes() + 1)]),
            ErrorCode::LimitAttachmentBytes
        );
    }

    #[test]
    fn enforces_the_aggregate_cap() {
        use crate::config::MIB;

        let at_cap = vec![bytes("a.bin", 10 * MIB), bytes("b.bin", 10 * MIB)];
        assert_eq!(
            validate(&at_cap, Limits::default(), false).expect("at cap"),
            20 * MIB
        );

        // Each file is within the per-file cap; the total is one byte over.
        let over = vec![
            bytes("a.bin", 10 * MIB),
            bytes("b.bin", 10 * MIB),
            bytes("c.bin", 1),
        ];
        assert_eq!(
            validate(&over, Limits::default(), false)
                .expect_err("over total")
                .code(),
            ErrorCode::LimitAttachmentTotal
        );
    }

    #[test]
    fn rejects_invalid_filenames_and_content_types() {
        for name in ["", "   ", "dir/name.txt", "dir\\name.txt", "big.bin/"] {
            assert_eq!(
                code(&[bytes(name, 1)]),
                ErrorCode::ValidationInvalidAttachment,
                "{name}"
            );
        }
        for name in ["bad\r\nname.txt", "big\0.bin"] {
            assert_eq!(
                code(&[bytes(name, 1)]),
                ErrorCode::ValidationHeaderInjection,
                "{name}"
            );
        }

        let mut attachment = bytes("ok.txt", 1);
        if let Attachment::Bytes { content_type, .. } = &mut attachment {
            *content_type = Some("not-a-content-type".to_string());
        }
        assert_eq!(code(&[attachment]), ErrorCode::ValidationInvalidAttachment);
    }

    #[test]
    fn rejects_file_attachments_without_opt_in() {
        assert_eq!(
            code(&[file(Some("notes.txt"))]),
            ErrorCode::ConfigFileAccessDisabled
        );
    }

    #[test]
    fn accepts_file_descriptor_with_opt_in_without_reading_it() {
        assert_eq!(
            validate(&[file(Some("report.pdf"))], Limits::default(), true).expect("valid"),
            0
        );
    }

    #[test]
    fn rejects_paths_as_filenames() {
        assert_eq!(
            code(&[bytes("/etc/passwd", 1)]),
            ErrorCode::ValidationInvalidAttachment
        );
    }

    fn code(attachments: &[Attachment]) -> ErrorCode {
        validate(attachments, Limits::default(), false)
            .expect_err("must be rejected")
            .code()
    }
}
