//! SMTP command serialization (RFC 5321 §4.1, RFC 3207 §4, RFC 4954 §4).
//!
//! Every command is rendered from validated parts into exact wire bytes.
//! Arguments reject control characters and whitespace, so a caller cannot
//! inject a second command, a header-like sequence, or a line break.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

use crate::errors::{Error, ErrorCode};

/// Maximum command line length including CRLF (RFC 5321 §4.5.3.1.6).
pub const MAX_COMMAND_BYTES: usize = 512;

/// AUTH mechanisms Sendust implements (ADR-0017).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMechanism {
    Plain,
    Login,
}

impl AuthMechanism {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Plain => "PLAIN",
            Self::Login => "LOGIN",
        }
    }
}

/// One SMTP command, ready to serialize.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Ehlo {
        identity: String,
    },
    StartTls,
    Auth {
        mechanism: AuthMechanism,
        initial_response: Option<String>,
    },
    /// One base64 challenge response (AUTH LOGIN, or a multi-step exchange).
    AuthResponse {
        response: String,
    },
    MailFrom {
        address: String,
    },
    RcptTo {
        address: String,
    },
    Data,
    Reset,
    NoOp,
    Quit,
}

impl Command {
    /// Exact wire bytes, including the terminating CRLF.
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let line = match self {
            Self::Ehlo { identity } => {
                validate_token(identity, 255)?;
                format!("EHLO {identity}")
            }
            Self::StartTls => "STARTTLS".to_string(),
            Self::Auth {
                mechanism,
                initial_response,
            } => match initial_response {
                Some(response) => {
                    validate_token(response, 12_288)?;
                    format!("AUTH {} {response}", mechanism.as_str())
                }
                None => format!("AUTH {}", mechanism.as_str()),
            },
            Self::AuthResponse { response } => {
                validate_token(response, 12_288)?;
                response.clone()
            }
            Self::MailFrom { address } => {
                validate_address(address)?;
                format!("MAIL FROM:<{address}>")
            }
            Self::RcptTo { address } => {
                validate_address(address)?;
                format!("RCPT TO:<{address}>")
            }
            Self::Data => "DATA".to_string(),
            Self::Reset => "RSET".to_string(),
            Self::NoOp => "NOOP".to_string(),
            Self::Quit => "QUIT".to_string(),
        };

        if line.len() + 2 > MAX_COMMAND_BYTES {
            return Err(protocol_error());
        }
        let mut bytes = line.into_bytes();
        bytes.extend_from_slice(b"\r\n");
        Ok(bytes)
    }
}

/// RFC 4616 initial response for AUTH PLAIN: `\0user\0password` in base64.
pub fn auth_plain_initial(user: &str, password: &str) -> String {
    let mut raw = Vec::with_capacity(user.len() + password.len() + 2);
    raw.push(0);
    raw.extend_from_slice(user.as_bytes());
    raw.push(0);
    raw.extend_from_slice(password.as_bytes());
    STANDARD.encode(raw)
}

fn validate_token(value: &str, max: usize) -> Result<(), Error> {
    if value.is_empty() || value.len() > max || value.chars().any(|c| c.is_control() || c == ' ') {
        return Err(protocol_error());
    }
    Ok(())
}

fn validate_address(value: &str) -> Result<(), Error> {
    validate_token(value, 320)?;
    if value
        .chars()
        .any(|c| c == '<' || c == '>' || c == ' ' || c == ',')
    {
        return Err(protocol_error());
    }
    Ok(())
}

fn protocol_error() -> Error {
    Error::new(ErrorCode::ValidationHeaderInjection)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(command: Command) -> String {
        String::from_utf8(command.to_bytes().expect("serialized")).expect("utf8")
    }

    #[test]
    fn serializes_each_command_with_exact_bytes() {
        assert_eq!(
            text(Command::Ehlo {
                identity: "client.example.test".to_string()
            }),
            "EHLO client.example.test\r\n"
        );
        assert_eq!(text(Command::StartTls), "STARTTLS\r\n");
        assert_eq!(
            text(Command::MailFrom {
                address: "alice@example.test".to_string()
            }),
            "MAIL FROM:<alice@example.test>\r\n"
        );
        assert_eq!(
            text(Command::RcptTo {
                address: "bob@example.test".to_string()
            }),
            "RCPT TO:<bob@example.test>\r\n"
        );
        assert_eq!(text(Command::Data), "DATA\r\n");
        assert_eq!(text(Command::Reset), "RSET\r\n");
        assert_eq!(text(Command::NoOp), "NOOP\r\n");
        assert_eq!(text(Command::Quit), "QUIT\r\n");
    }

    #[test]
    fn serializes_auth_in_both_forms() {
        assert_eq!(
            text(Command::Auth {
                mechanism: AuthMechanism::Login,
                initial_response: None
            }),
            "AUTH LOGIN\r\n"
        );
        assert_eq!(
            text(Command::Auth {
                mechanism: AuthMechanism::Plain,
                initial_response: Some("AGFsaWNlAHNlY3JldA==".to_string()),
            }),
            "AUTH PLAIN AGFsaWNlAHNlY3JldA==\r\n"
        );
        assert_eq!(
            text(Command::AuthResponse {
                response: "dXNlcg==".to_string()
            }),
            "dXNlcg==\r\n"
        );
    }

    #[test]
    fn builds_the_plain_initial_response() {
        // Input is NUL + "alice" + NUL + "secret".
        assert_eq!(
            auth_plain_initial("alice", "secret"),
            "AGFsaWNlAHNlY3JldA=="
        );
    }

    #[test]
    fn rejects_injection_and_whitespace_in_arguments() {
        for address in [
            "user@example.test\r\nDATA",
            "user@example.test\nDATA",
            "user@example.test\0",
            "a b",
        ] {
            let error = Command::RcptTo {
                address: address.to_string(),
            }
            .to_bytes()
            .expect_err("must be rejected");
            assert_eq!(error.code(), ErrorCode::ValidationHeaderInjection);
        }
    }

    #[test]
    fn rejects_overlong_commands() {
        let address = format!("{}@example.test", "a".repeat(MAX_COMMAND_BYTES));
        let error = Command::MailFrom { address }
            .to_bytes()
            .expect_err("must be rejected");
        assert_eq!(error.code(), ErrorCode::ValidationHeaderInjection);
    }
}
