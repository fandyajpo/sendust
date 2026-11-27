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

/// Maximum length of an `AUTH` or challenge-response line including CRLF.
///
/// RFC 4954 §4 lets an initial response exceed the 512-octet command line —
/// servers must accept at least 1,228 octets — and a bearer token is routinely
/// far longer than a password. The bound matches the largest response
/// `validate_token` accepts, so the two limits cannot disagree.
pub const MAX_AUTH_LINE_BYTES: usize = 12_288 + 64;

/// AUTH mechanisms Sendust implements (ADR-0017, ADR-0023).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMechanism {
    Plain,
    Login,
    XOAuth2,
}

impl AuthMechanism {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Plain => "PLAIN",
            Self::Login => "LOGIN",
            Self::XOAuth2 => "XOAUTH2",
        }
    }
}

/// One SMTP command, ready to serialize.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Ehlo {
        identity: String,
    },
    /// The fallback greeting a server that refuses `EHLO` requires
    /// (RFC 5321 §4.1.1.1).
    Helo {
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
        /// The message size to declare, when the relay advertises `SIZE`
        /// (RFC 1870). A relay that never advertises it must not be sent one.
        size: Option<u64>,
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
            Self::Helo { identity } => {
                validate_token(identity, 255)?;
                format!("HELO {identity}")
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
                // RFC 7628: an XOAUTH2 error challenge is answered with an empty
                // line so the relay can send its final failure reply.
                if response.is_empty() {
                    String::new()
                } else {
                    validate_token(response, 12_288)?;
                    response.clone()
                }
            }
            Self::MailFrom { address, size } => {
                validate_address(address)?;
                match size {
                    Some(size) => format!("MAIL FROM:<{address}> SIZE={size}"),
                    None => format!("MAIL FROM:<{address}>"),
                }
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

        // A bearer token is not a password: XOAUTH2's initial response, and the
        // base64 challenge answers of AUTH LOGIN, are far longer than the command
        // line RFC 5321 §4.5.3.1.6 caps. Those two variants use the bound their
        // own validator already enforces.
        let limit = match self {
            Self::Auth { .. } | Self::AuthResponse { .. } => MAX_AUTH_LINE_BYTES,
            _ => MAX_COMMAND_BYTES,
        };
        if line.len() + 2 > limit {
            return Err(invalid_option());
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

/// RFC 7628 initial response for AUTH XOAUTH2: the base64 of
/// `user=<mailbox>\x01auth=Bearer <token>\x01\x01`.
pub fn auth_xoauth2_initial(user: &str, token: &str) -> String {
    let raw = format!("user={user}\u{1}auth=Bearer {token}\u{1}\u{1}");
    STANDARD.encode(raw)
}

fn validate_token(value: &str, max: usize) -> Result<(), Error> {
    // A value that is too long is a limit, not an injection attempt: the two are
    // different failures for the caller, and only the second is a security report.
    if value.is_empty() || value.len() > max {
        return Err(invalid_option());
    }
    if value.chars().any(|c| c.is_control() || c == ' ') {
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

/// A caller-supplied value outside the protocol's bound.
fn invalid_option() -> Error {
    Error::new(ErrorCode::ValidationInvalidOption)
}

/// A caller-supplied value that could inject a command.
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
        assert_eq!(
            text(Command::Helo {
                identity: "client.example.test".to_string()
            }),
            "HELO client.example.test\r\n"
        );
        assert_eq!(text(Command::StartTls), "STARTTLS\r\n");
        assert_eq!(
            text(Command::MailFrom {
                address: "alice@example.test".to_string(),
                size: None,
            }),
            "MAIL FROM:<alice@example.test>\r\n"
        );
        // RFC 1870: the size is declared only when the relay advertises SIZE.
        assert_eq!(
            text(Command::MailFrom {
                address: "alice@example.test".to_string(),
                size: Some(2_048),
            }),
            "MAIL FROM:<alice@example.test> SIZE=2048\r\n"
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
        let error = Command::MailFrom {
            address,
            size: None,
        }
        .to_bytes()
        .expect_err("must be rejected");
        // An address longer than the protocol allows is a limit, not injection.
        assert_eq!(error.code(), ErrorCode::ValidationInvalidOption);
    }

    #[test]
    fn an_auth_line_may_exceed_the_command_line_limit() {
        // A Microsoft 365 or Graph access token is a JWT of 800-2,000+
        // characters, so base64 of the XOAUTH2 payload is far past 512 octets.
        let token = "a".repeat(1_400);
        let response = auth_xoauth2_initial("sender@example.test", &token);
        assert!(
            response.len() > MAX_COMMAND_BYTES,
            "the fixture must exceed the command line, or it proves nothing"
        );

        let bytes = Command::Auth {
            mechanism: AuthMechanism::XOAuth2,
            initial_response: Some(response),
        }
        .to_bytes()
        .expect("a long bearer token is a valid AUTH line");

        assert!(bytes.starts_with(b"AUTH XOAUTH2 "));
        assert!(bytes.ends_with(b"\r\n"));
        assert!(bytes.len() > MAX_COMMAND_BYTES + 2);
    }

    #[test]
    fn an_auth_response_beyond_the_protocol_bound_is_rejected() {
        let response = "a".repeat(MAX_AUTH_LINE_BYTES);
        let error = Command::Auth {
            mechanism: AuthMechanism::Plain,
            initial_response: Some(response),
        }
        .to_bytes()
        .expect_err("over the protocol bound");
        // Too long is a limit; it is not the injection code.
        assert_eq!(error.code(), ErrorCode::ValidationInvalidOption);
    }

    #[test]
    fn an_over_long_argument_is_a_limit_not_an_injection() {
        let error = Command::Ehlo {
            identity: "a".repeat(256),
        }
        .to_bytes()
        .expect_err("over the identity bound");
        assert_eq!(error.code(), ErrorCode::ValidationInvalidOption);
    }
}
