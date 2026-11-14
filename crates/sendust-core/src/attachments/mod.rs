//! Attachment input policy, bounded buffer handling, and safe file reads.
//!
//! P04-003 owns descriptor validation and byte caps for in-memory attachments.
//! Reading file attachments safely, and enforcing their byte caps at read time,
//! belongs to P08; until then a file descriptor is policy-checked but not opened.

use std::path::Path;

use crate::config::Limits;
use crate::errors::{Error, ErrorCode};

/// Maximum filename length accepted by the model.
const MAX_FILENAME: usize = 255;
/// Maximum path length accepted for a file attachment.
const MAX_PATH: usize = 1_024;

/// Filesystem attachment policy.
///
/// Disabled unless the caller opts in explicitly. There is no global or
/// ambient switch: the policy is transport-scoped and passed to every send.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AttachmentPolicy {
    /// File attachments are refused before any filesystem operation.
    #[default]
    Disabled,
    /// File attachments may be read, subject to the caps and file-type rules.
    Enabled,
}

impl AttachmentPolicy {
    pub const fn filesystem_enabled(self) -> bool {
        matches!(self, Self::Enabled)
    }

    pub const fn from_flag(enabled: bool) -> Self {
        if enabled {
            Self::Enabled
        } else {
            Self::Disabled
        }
    }
}

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
    policy: AttachmentPolicy,
) -> Result<u64, Error> {
    let mut total: u64 = 0;
    let mut names: Vec<String> = Vec::with_capacity(attachments.len());

    for attachment in attachments {
        validate_descriptor(attachment, policy)?;

        // Two attachments with the same display name would collide in most
        // clients, so the ambiguity is rejected instead of guessed at. The
        // comparison uses the stripped basename both renderers will emit.
        if let Some(name) = attachment.display_name() {
            let key = normalize_filename(name)?.to_lowercase();
            if names.contains(&key) {
                return Err(invalid_attachment());
            }
            names.push(key);
        }

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

pub(crate) fn normalize_filename(raw: &str) -> Result<String, Error> {
    // A filename is a display name, never a path. Keep the last component so a
    // caller cannot leak a local directory or traverse, and so a path from
    // `path.join` still works instead of failing the whole send.
    let name = raw.rsplit(['/', '\\']).next().unwrap_or_default().trim();
    if name.is_empty() || name.len() > MAX_FILENAME || name == "." || name == ".." {
        return Err(invalid_attachment());
    }
    if name.chars().any(char::is_control) {
        return Err(Error::new(ErrorCode::ValidationHeaderInjection));
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

fn validate_descriptor(attachment: &Attachment, policy: AttachmentPolicy) -> Result<(), Error> {
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
            if !policy.filesystem_enabled() {
                return Err(Error::new(ErrorCode::ConfigFileAccessDisabled));
            }
            if let Some(name) = &file.display_name {
                normalize_filename(name)?;
            }
            if let Some(value) = &file.content_type {
                normalize_content_type(value)?;
            }
            validate_path(&file.path)?;
            Ok(())
        }
    }
}

/// A file attachment path must be absolute and free of control characters, so a
/// relative path cannot resolve against an unexpected working directory and an
/// error can never echo an injection attempt.
fn validate_path(path: &str) -> Result<(), Error> {
    let value = path.trim();
    if value.is_empty() || value.len() > MAX_PATH {
        return Err(invalid_attachment());
    }
    if value.chars().any(char::is_control) {
        return Err(Error::new(ErrorCode::ValidationHeaderInjection));
    }
    if !Path::new(value).is_absolute() {
        return Err(invalid_attachment());
    }
    Ok(())
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
        let total = validate(
            &[bytes("report.pdf", 3)],
            Limits::default(),
            AttachmentPolicy::Disabled,
        )
        .expect("valid");
        assert_eq!(total, 3);
    }

    #[test]
    fn enforces_the_per_file_cap_inclusively() {
        let limits = Limits::default();
        assert!(
            validate(
                &[bytes("at-cap", limits.max_attachment_bytes())],
                limits,
                AttachmentPolicy::Disabled
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
            validate(&at_cap, Limits::default(), AttachmentPolicy::Disabled).expect("at cap"),
            20 * MIB
        );

        // Each file is within the per-file cap; the total is one byte over.
        let over = vec![
            bytes("a.bin", 10 * MIB),
            bytes("b.bin", 10 * MIB),
            bytes("c.bin", 1),
        ];
        assert_eq!(
            validate(&over, Limits::default(), AttachmentPolicy::Disabled)
                .expect_err("over total")
                .code(),
            ErrorCode::LimitAttachmentTotal
        );
    }

    #[test]
    fn rejects_invalid_filenames_and_content_types() {
        for name in ["", "   ", "big.bin/", ".", ".."] {
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
    fn strips_paths_down_to_the_display_name() {
        assert_eq!(
            normalize_filename("/etc/passwd").expect("stripped"),
            "passwd"
        );
        assert_eq!(
            normalize_filename("dir\\report.pdf").expect("stripped"),
            "report.pdf"
        );
        assert_eq!(
            normalize_filename("../../secrets/notes.txt").expect("stripped"),
            "notes.txt"
        );
        assert_eq!(
            normalize_filename("  report.pdf  ").expect("trimmed"),
            "report.pdf"
        );
    }

    #[test]
    fn accepts_unicode_filenames() {
        assert_eq!(
            normalize_filename("Grüße.pdf").expect("unicode"),
            "Grüße.pdf"
        );
        assert_eq!(
            validate(
                &[bytes("Grüße.pdf", 1)],
                Limits::default(),
                AttachmentPolicy::Disabled
            )
            .expect("valid"),
            1
        );
    }

    #[test]
    fn rejects_filenames_over_the_length_limit() {
        let exactly = "a".repeat(MAX_FILENAME);
        assert!(normalize_filename(&exactly).is_ok());
        let over = "a".repeat(MAX_FILENAME + 1);
        assert_eq!(
            normalize_filename(&over).expect_err("too long").code(),
            ErrorCode::ValidationInvalidAttachment
        );
    }

    #[test]
    fn rejects_duplicate_filenames_case_insensitively() {
        assert_eq!(
            code(&[bytes("Report.pdf", 1), bytes("report.PDF", 1)]),
            ErrorCode::ValidationInvalidAttachment
        );
        assert_eq!(
            validate(
                &[bytes("a.txt", 1), bytes("b.txt", 1)],
                Limits::default(),
                AttachmentPolicy::Disabled
            )
            .expect("distinct names"),
            2
        );
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
            validate(
                &[file(Some("report.pdf"))],
                Limits::default(),
                AttachmentPolicy::Enabled
            )
            .expect("valid"),
            0
        );
    }

    #[test]
    fn filesystem_access_is_disabled_by_default() {
        assert_eq!(AttachmentPolicy::default(), AttachmentPolicy::Disabled);
        assert!(!AttachmentPolicy::default().filesystem_enabled());
        assert!(AttachmentPolicy::Enabled.filesystem_enabled());
        assert_eq!(AttachmentPolicy::from_flag(true), AttachmentPolicy::Enabled);
        assert_eq!(
            AttachmentPolicy::from_flag(false),
            AttachmentPolicy::Disabled
        );
    }

    #[test]
    fn rejects_relative_or_malformed_paths() {
        let mut descriptor = FileDescriptor {
            display_name: Some("notes.txt".to_string()),
            content_type: None,
            path: "relative/notes.txt".to_string(),
        };
        assert_eq!(
            code_file(&descriptor),
            ErrorCode::ValidationInvalidAttachment
        );

        descriptor.path = "  ".to_string();
        assert_eq!(
            code_file(&descriptor),
            ErrorCode::ValidationInvalidAttachment
        );

        descriptor.path = "/tmp/notes\r\n.txt".to_string();
        assert_eq!(code_file(&descriptor), ErrorCode::ValidationHeaderInjection);

        descriptor.path = format!("/tmp/{}", "a".repeat(MAX_PATH));
        assert_eq!(
            code_file(&descriptor),
            ErrorCode::ValidationInvalidAttachment
        );

        descriptor.path = "/tmp/notes.txt".to_string();
        assert!(
            validate(
                &[Attachment::File(descriptor)],
                Limits::default(),
                AttachmentPolicy::Enabled
            )
            .is_ok()
        );
    }

    fn code_file(descriptor: &FileDescriptor) -> ErrorCode {
        validate(
            &[Attachment::File(descriptor.clone())],
            Limits::default(),
            AttachmentPolicy::Enabled,
        )
        .expect_err("must fail")
        .code()
    }

    fn code(attachments: &[Attachment]) -> ErrorCode {
        validate(attachments, Limits::default(), AttachmentPolicy::Disabled)
            .expect_err("must be rejected")
            .code()
    }
}
