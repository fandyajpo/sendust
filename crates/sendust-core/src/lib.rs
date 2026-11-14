//! Runtime-independent Sendust domain and delivery engine.
//!
//! This crate must never depend on Node.js, `napi`, or any JavaScript runtime API.
//!
//! The module tree follows ADR-0016: domain modules, application orchestration,
//! one narrow outbound delivery port, and the single SMTP adapter. Public
//! contracts are re-exported from this facade as later phases implement them.

mod address;
mod attachments;
mod config;
mod delivery;
mod errors;
mod lifecycle;
mod message;
mod mime;
mod ports;
mod results;
mod transports;
mod validation;

pub use crate::address::Mailbox;
pub use crate::attachments::{Attachment, AttachmentPolicy, FileDescriptor};
pub use crate::config::{Limits, MIB, Timeouts};
pub use crate::delivery::Delivery;
pub use crate::errors::{Error, ErrorCategory, ErrorCode, ErrorDetails, Stage};
pub use crate::lifecycle::{LifecycleState, SendDeadlines, SendPermit, Transport};
pub use crate::message::{Header, HeaderName, HeaderValue, OutgoingMessage};
pub use crate::mime::{serialize, serialize_at};
pub use crate::ports::{DeliveryPort, DeliveryRequest};
pub use crate::results::{
    AmbiguityStage, DeliveryStatus, RecipientOutcome, RecipientStatus, SendOutcome, SmtpResponse,
};
pub use crate::transports::smtp::{
    SmtpConfig, SmtpCredentials, SmtpSecurity, SmtpSession, SmtpTransport,
};
pub use crate::validation::{Failures as ValidationFailures, ValidationStage, validate_in_order};

#[doc(hidden)]
pub mod __internal {
    //! Internal seams used by the integration tests. Not a supported API.

    pub use crate::transports::smtp::client::connection::Connection;
    pub use crate::transports::smtp::proto::capabilities::Capabilities;
    pub use crate::transports::smtp::proto::command::{AuthMechanism, Command};
    pub use crate::transports::smtp::proto::response::Reply;
    pub use crate::transports::smtp::session::Session;
}

/// Crate version, surfaced to the binding for diagnostics.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
