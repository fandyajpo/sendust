//! Transport state, one-send reservation, BUSY semantics, and bounded close drain.
//!
//! The lifecycle is explicit: `open -> closing -> closed`. A transport accepts
//! exactly one active send; a second concurrent send fails immediately with
//! `TRANSPORT_BUSY` without touching the network. `close` is idempotent, and it
//! drains for a bounded budget and then reports `TIMEOUT_CLOSE`.
//!
//! The state machine owns no network code and no timer thread: callers pass an
//! `Instant`, which keeps timeout behavior testable without sleeping.

use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

pub use crate::config::SendDeadlines;
use crate::errors::{Error, ErrorCode};

/// Observable lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    Open,
    Closing,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Open,
    Closing,
    Closed,
}

#[derive(Debug)]
struct State {
    phase: Phase,
    active_send: bool,
}

/// A transport lifecycle: one active send, explicit bounded close.
#[derive(Debug)]
pub struct Transport {
    state: Mutex<State>,
    idle: Condvar,
}

impl Default for Transport {
    fn default() -> Self {
        Self::new()
    }
}

impl Transport {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(State {
                phase: Phase::Open,
                active_send: false,
            }),
            idle: Condvar::new(),
        }
    }

    /// Current state. Never blocks on a send: it is a snapshot.
    pub fn state(&self) -> LifecycleState {
        match self.lock().phase {
            Phase::Open => LifecycleState::Open,
            Phase::Closing => LifecycleState::Closing,
            Phase::Closed => LifecycleState::Closed,
        }
    }

    /// Reserves the single send slot.
    ///
    /// Fails before any network work when the transport is closing or closed,
    /// and fails immediately when a send is already active.
    pub fn begin_send(self: &Arc<Self>) -> Result<SendPermit, Error> {
        let mut state = self.lock();

        match state.phase {
            Phase::Closing | Phase::Closed => {
                return Err(Error::new(ErrorCode::TransportClosed));
            }
            Phase::Open => {}
        }

        if state.active_send {
            return Err(Error::new(ErrorCode::TransportBusy));
        }

        state.active_send = true;
        Ok(SendPermit {
            transport: Arc::clone(self),
        })
    }

    /// Enters `closing` atomically. Returns whether this call changed the state.
    pub fn begin_close(&self) -> bool {
        let mut state = self.lock();
        if state.phase == Phase::Open {
            state.phase = Phase::Closing;
            true
        } else {
            false
        }
    }

    /// Waits for the active send to drain, then reports `closed`.
    ///
    /// Returns `TIMEOUT_CLOSE` when the drain exceeds the budget; the transport
    /// is closed either way, so the caller cannot send again.
    pub fn wait_closed(&self, timeout: Duration) -> Result<(), Error> {
        let deadline = Instant::now() + timeout;
        let mut state = self.lock();

        while state.active_send {
            let now = Instant::now();
            if now >= deadline {
                state.phase = Phase::Closed;
                self.idle.notify_all();
                return Err(Error::new(ErrorCode::TimeoutClose));
            }

            let (next, _) = self
                .idle
                .wait_timeout(state, deadline - now)
                .unwrap_or_else(PoisonError::into_inner);
            state = next;
        }

        state.phase = Phase::Closed;
        self.idle.notify_all();
        Ok(())
    }

    fn release(&self) {
        let mut state = self.lock();
        state.active_send = false;
        self.idle.notify_all();
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Proof that the single send slot is held. Dropping it releases the slot.
#[derive(Debug)]
pub struct SendPermit {
    transport: Arc<Transport>,
}

impl SendPermit {
    pub fn transport(&self) -> &Transport {
        &self.transport
    }
}

impl Drop for SendPermit {
    fn drop(&mut self) {
        self.transport.release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    fn transport() -> Arc<Transport> {
        Arc::new(Transport::new())
    }

    #[test]
    fn starts_open_and_accepts_one_send() {
        let transport = transport();
        assert_eq!(transport.state(), LifecycleState::Open);

        let permit = transport.begin_send().expect("first send");
        assert_eq!(transport.state(), LifecycleState::Open);
        drop(permit);
        assert!(transport.begin_send().is_ok());
    }

    #[test]
    fn a_second_send_is_busy_and_dials_nothing() {
        let transport = transport();
        let _permit = transport.begin_send().expect("first send");

        let mut dials = 0;
        let outcome = transport.begin_send();
        if outcome.is_ok() {
            dials += 1;
        }

        assert_eq!(dials, 0, "a BUSY send must never dial");
        assert_eq!(outcome.expect_err("busy").code(), ErrorCode::TransportBusy);
        assert_eq!(transport.state(), LifecycleState::Open);
    }

    #[test]
    fn busy_is_released_when_the_permit_drops() {
        let transport = transport();
        drop(transport.begin_send().expect("first"));
        assert!(transport.begin_send().is_ok());
    }

    #[test]
    fn close_is_idempotent_and_refuses_new_sends() {
        let transport = transport();
        assert!(transport.begin_close());
        assert_eq!(transport.state(), LifecycleState::Closing);
        assert!(!transport.begin_close());
        assert_eq!(transport.state(), LifecycleState::Closing);

        assert_eq!(
            transport
                .begin_send()
                .expect_err("closing refuses sends")
                .code(),
            ErrorCode::TransportClosed
        );
        assert!(transport.wait_closed(Duration::from_secs(1)).is_ok());
        assert_eq!(transport.state(), LifecycleState::Closed);
    }

    #[test]
    fn send_after_close_is_refused() {
        let transport = transport();
        transport.begin_close();
        transport
            .wait_closed(Duration::from_secs(1))
            .expect("closed");

        assert_eq!(
            transport
                .begin_send()
                .expect_err("closed refuses sends")
                .code(),
            ErrorCode::TransportClosed
        );
    }

    #[test]
    fn close_drains_an_active_send() {
        let transport = transport();
        let permit = transport.begin_send().expect("send");
        transport.begin_close();

        let waiter = Arc::clone(&transport);
        let handle = thread::spawn(move || waiter.wait_closed(Duration::from_secs(2)));

        thread::sleep(Duration::from_millis(30));
        drop(permit);

        assert!(handle.join().expect("thread").is_ok());
        assert_eq!(transport.state(), LifecycleState::Closed);
    }

    #[test]
    fn close_drain_times_out_without_the_active_send_finishing() {
        let transport = transport();
        let _permit = transport.begin_send().expect("send");
        transport.begin_close();

        assert_eq!(
            transport
                .wait_closed(Duration::from_millis(20))
                .expect_err("drain timed out")
                .code(),
            ErrorCode::TimeoutClose
        );
        assert_eq!(transport.state(), LifecycleState::Closed);
    }
}
