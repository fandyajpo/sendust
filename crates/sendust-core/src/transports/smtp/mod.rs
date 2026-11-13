//! SMTP session, TLS, AUTH, envelope, RCPT/DATA, and response mapping via lettre.

#[cfg(test)]
mod tests {
    use lettre::transport::smtp::SmtpTransport;

    #[test]
    fn pinned_smtp_transport_is_available() {
        // Constructing the builder proves the pinned `smtp-transport` feature is
        // enabled and the configured lettre dependency is compiled. No
        // connection or DNS lookup is attempted.
        let _builder = SmtpTransport::relay("localhost");
    }
}
