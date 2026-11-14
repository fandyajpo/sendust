#![allow(dead_code)]
//! Native SMTP connection: TCP, implicit TLS, STARTTLS, bounded reply I/O.
//!
//! Wired into the delivery adapter by P06-004; the temporary `dead_code` allow
//! disappears then.

pub mod connection;
pub(crate) mod tls;
