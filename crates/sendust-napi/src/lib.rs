//! Thin Node-API inbound adapter for Sendust.
//!
//! Owns JavaScript conversion, owned-memory snapshots, asynchronous task and
//! lifecycle bridging, and error/result mapping only. SMTP, MIME, validation,
//! and delivery policy belong to `sendust-core` and must not be duplicated here.

use napi_derive::napi;

/// Diagnostic version string for the native adapter and the core it wraps.
#[napi]
pub fn native_version() -> String {
    format!(
        "sendust-napi {} (core {})",
        env!("CARGO_PKG_VERSION"),
        sendust_core::version()
    )
}
