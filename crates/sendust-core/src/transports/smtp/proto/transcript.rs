//! DATA body transparency (RFC 5321 §4.5.2).

/// Dot-stuffs a message body and appends the DATA terminator.
///
/// Lines beginning with `.` gain a leading `.`; the payload always ends with
/// CRLF followed by `.<CRLF>` so the terminator is unambiguous.
pub fn encode(wire: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(wire.len() + 8);
    let mut at_line_start = true;

    for &byte in wire {
        if at_line_start && byte == b'.' {
            out.push(b'.');
        }
        out.push(byte);
        at_line_start = byte == b'\n';
    }

    if !wire.ends_with(b"\r\n") {
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(b".\r\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stuffs_dot_lines_and_appends_the_terminator() {
        let encoded = encode(b"Subject: x\r\n\r\n.hidden\r\n");
        assert_eq!(encoded, b"Subject: x\r\n\r\n..hidden\r\n.\r\n".to_vec());
    }

    #[test]
    fn adds_a_missing_trailing_crlf_before_the_terminator() {
        assert_eq!(encode(b"body"), b"body\r\n.\r\n".to_vec());
    }
}
