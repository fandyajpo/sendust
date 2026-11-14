//! EHLO capability parsing (RFC 5321 §4.1.1.1, RFC 3207 §4, RFC 4954 §4).

use super::command::AuthMechanism;
use super::response::Reply;

/// Capabilities advertised by the relay in its EHLO reply.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Capabilities {
    starttls: bool,
    auth: Vec<AuthMechanism>,
    size: Option<u64>,
    eight_bit_mime: bool,
}

impl Capabilities {
    /// Parses the extension lines of an EHLO reply (the first line is the greeting).
    pub fn parse(reply: &Reply) -> Self {
        let mut caps = Self::default();

        for line in reply.lines().iter().skip(1) {
            let mut words = line.split_whitespace();
            let Some(keyword) = words.next() else {
                continue;
            };

            match keyword.to_ascii_uppercase().as_str() {
                "STARTTLS" => caps.starttls = true,
                "AUTH" => {
                    for word in words {
                        match word.to_ascii_uppercase().as_str() {
                            "PLAIN" => caps.auth.push(AuthMechanism::Plain),
                            "LOGIN" => caps.auth.push(AuthMechanism::Login),
                            _ => {}
                        }
                    }
                }
                "SIZE" => {
                    caps.size = words.next().and_then(|value| value.parse().ok());
                }
                "8BITMIME" => caps.eight_bit_mime = true,
                _ => {}
            }
        }

        caps
    }

    pub const fn starttls(&self) -> bool {
        self.starttls
    }

    pub fn auth_mechanisms(&self) -> &[AuthMechanism] {
        &self.auth
    }

    pub fn supports_auth(&self, mechanism: AuthMechanism) -> bool {
        self.auth.contains(&mechanism)
    }

    pub const fn size(&self) -> Option<u64> {
        self.size
    }

    pub const fn eight_bit_mime(&self) -> bool {
        self.eight_bit_mime
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transports::smtp::proto::response::ReplyBuilder;

    fn reply(lines: &[&str]) -> Reply {
        let mut builder = ReplyBuilder::new();
        for line in lines {
            let wire = format!("{line}\r\n");
            builder.push_line(wire.as_bytes()).expect("line");
        }
        builder.finish().expect("reply")
    }

    #[test]
    fn parses_advertised_extensions() {
        let caps = Capabilities::parse(&reply(&[
            "250-smtp.test",
            "250-STARTTLS",
            "250-AUTH PLAIN LOGIN",
            "250-SIZE 10485760",
            "250 8BITMIME",
        ]));
        assert!(caps.starttls());
        assert!(caps.supports_auth(AuthMechanism::Plain));
        assert!(caps.supports_auth(AuthMechanism::Login));
        assert_eq!(caps.size(), Some(10_485_760));
        assert!(caps.eight_bit_mime());
    }

    #[test]
    fn defaults_to_nothing_when_not_advertised() {
        let caps = Capabilities::parse(&reply(&["250 smtp.test"]));
        assert!(!caps.starttls());
        assert!(!caps.supports_auth(AuthMechanism::Plain));
        assert!(caps.auth_mechanisms().is_empty());
        assert_eq!(caps.size(), None);
        assert!(!caps.eight_bit_mime());
    }
}
