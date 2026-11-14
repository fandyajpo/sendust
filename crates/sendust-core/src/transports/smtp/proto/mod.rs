#![allow(dead_code)]
//! I/O-free SMTP protocol primitives: command codec, reply parser, capabilities.
//!
//! ADR-0017 makes the SMTP engine Sendust-owned. This module owns the wire
//! syntax and nothing else: it never touches a socket, a runtime, or a TLS
//! stack, so every rule here is unit-testable without network access.
//!
//! Wired into the session by P06-002/P06-003; the temporary `dead_code` allow
//! disappears when those land.

pub mod capabilities;
pub mod command;
pub mod response;
pub mod transcript;
