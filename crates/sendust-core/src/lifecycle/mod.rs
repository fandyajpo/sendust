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

use crate::config::Timeouts;
use crate::errors::{Error, ErrorCode, ErrorDetails, Stage};

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

/// Per-send deadlines derived from a [`Timeouts`] value and one start instant.
///
/// A single total deadline governs the send, so independent timers cannot
/// overrun it. `operation` bounds each command/response wait; `connect` bounds
/// DNS, TCP, greeting, EHLO, and TLS as one outer boundary.
#[derive(Debug, Clone, Copy)]
pub struct SendDeadlines {
    started: Instant,
    connect_by: Instant,
    total_by: Instant,
    operation: Duration,
}

impl SendDeadlines {
    pub fn start(timeouts: Timeouts, now: Instant) -> Self {
        Self {
            started: now,
            connect_by: now + timeouts.connect(),
            total_by: now + timeouts.total(),
            operation: timeouts.operation(),
        }
    }

    pub fn connect_by(&self) -> Instant {
        self.connect_by
    }

    pub fn total_by(&self) -> Instant {
        self.total_by
    }

    pub fn elapsed(&self, now: Instant) -> Duration {
        now.saturating_duration_since(self.started)
    }

    pub fn remaining_total(&self, now: Instant) -> Duration {
        self.total_by.saturating_duration_since(now)
    }

    pub fn connect_expired(&self, now: Instant) -> bool {
        now >= self.connect_by || now >= self.total_by
    }

    pub fn operation_expired(&self, now: Instant) -> bool {
        now >= self.operation_deadline(now) || now >= self.total_by
    }

    /// Deadline for a single command: the operation budget, capped by the total.
    pub fn operation_deadline(&self, now: Instant) -> Instant {
        std::cmp::min(now + self.operation, self.total_by)
    }

    /// Honest timeout classification: connect only before the session is up.
    pub fn timeout_error(&self, now: Instant, connected: bool, stage: Stage) -> Error {
        let code = if !connected && now >= self.connect_by {
            ErrorCode::TimeoutConnect
        } else if now >= self.total_by {
            ErrorCode::TimeoutTotal
        } else {
            ErrorCode::TimeoutOperation
        };

        Error::new(code).with_details(
            ErrorDetails::default()
                .stage(stage)
                .elapsed_ms(u64::try_from(self.elapsed(now).as_millis()).unwrap_or(u64::MAX)),
        )
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

    #[test]
    fn deadlines_use_the_approved_budgets() {
        let now = Instant::now();
        let deadlines = SendDeadlines::start(Timeouts::default(), now);

        assert_eq!(deadlines.connect_by(), now + Duration::from_secs(10));
        assert_eq!(deadlines.total_by(), now + Duration::from_secs(120));
        assert!(
            !deadlines.connect_expired(now + Duration::from_secs(10) - Duration::from_millis(1))
        );
        assert!(deadlines.connect_expired(now + Duration::from_secs(10)));
    }

    #[test]
    fn operation_deadline_is_capped_by_the_total() {
        let now = Instant::now();
        let deadlines = SendDeadlines::start(Timeouts::default(), now);

        assert_eq!(
            deadlines.operation_deadline(now),
            now + Duration::from_secs(30)
        );

        let late = now + Duration::from_secs(100);
        assert_eq!(
            deadlines.operation_deadline(late),
            now + Duration::from_secs(120)
        );
        assert!(deadlines.operation_expired(now + Duration::from_secs(120)));
    }

    #[test]
    fn remaining_total_never_underflows() {
        let now = Instant::now();
        let deadlines = SendDeadlines::start(Timeouts::default(), now);
        assert_eq!(deadlines.remaining_total(now), Duration::from_secs(120));
        assert_eq!(
            deadlines.remaining_total(now + Duration::from_secs(200)),
            Duration::ZERO
        );
    }

    #[test]
    fn timeout_classification_is_honest_about_the_connect_phase() {
        let now = Instant::now();
        let deadlines = SendDeadlines::start(Timeouts::default(), now);

        let connect =
            deadlines.timeout_error(now + Duration::from_secs(10), false, Stage::ConnectionSetup);
        assert_eq!(connect.code(), ErrorCode::TimeoutConnect);
        assert_eq!(
            connect.details().operation_stage(),
            Some(Stage::ConnectionSetup)
        );

        let total = deadlines.timeout_error(now + Duration::from_secs(120), true, Stage::DataBody);
        assert_eq!(total.code(), ErrorCode::TimeoutTotal);

        let operation = deadlines.timeout_error(now + Duration::from_secs(40), true, Stage::Mail);
        assert_eq!(operation.code(), ErrorCode::TimeoutOperation);
    }
}
