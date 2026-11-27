//! DATA body transparency (RFC 5321 §4.5.2).

/// Dot-stuffs a message body and appends the DATA terminator.
///
/// Lines beginning with `.` gain a leading `.`; the payload always ends with
/// CRLF followed by `.<CRLF>` so the terminator is unambiguous.
pub fn encode(wire: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(wire.len() + 8);
    let mut copied = 0;
    let mut index = 0;
    let mut at_line_start = true;

    // Copy whole runs between the lines that need a dot, so a body that has none
    // costs one memcpy per line run rather than one `push` per byte.
    while index < wire.len() {
        if at_line_start && wire[index] == b'.' {
            out.extend_from_slice(&wire[copied..index]);
            out.push(b'.');
            copied = index;
        }
        at_line_start = wire[index] == b'\n';
        index += 1;
    }
    out.extend_from_slice(&wire[copied..]);

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
