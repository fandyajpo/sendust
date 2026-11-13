//! Narrow internal outbound delivery contract implemented by transport adapters.
//!
//! Delivery orchestration composes a message and hands the final bytes to this
//! port. The SMTP adapter is the only MVP implementation; adding a transport
//! means implementing this port, not changing domain or application modules.

use std::future::Future;
use std::pin::Pin;

use crate::errors::Error;
use crate::message::OutgoingMessage;
use crate::results::SendOutcome;

/// One composed message ready to submit, with its exact wire bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryRequest {
    pub message: OutgoingMessage,
    pub wire: Vec<u8>,
}

/// Boxed future returned by a transport adapter.
pub type DeliveryFuture<'a> = Pin<Box<dyn Future<Output = Result<SendOutcome, Error>> + Send + 'a>>;

/// The single outbound seam between application orchestration and a transport.
///
/// Implementations submit exactly one message per call and never retry.
pub trait DeliveryPort {
    fn deliver<'a>(&'a mut self, request: DeliveryRequest) -> DeliveryFuture<'a>;
}
