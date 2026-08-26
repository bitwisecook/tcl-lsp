// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The one syscall Rust's `std` does not expose on wasip1: `poll_oneoff`.
//!
//! `std::io::Stdin::read` blocks the process, and wasip1 gives the guest no
//! second thread to keep the async runtime turning while it does. This module
//! is the readiness primitive the driver waits on instead: one `poll_oneoff`
//! carrying *both* a `fd_read` subscription on stdin and a monotonic clock
//! subscription, so the call returns as soon as the client writes — and no
//! later than the deadline, which is what keeps the runtime's timers moving
//! while nothing is arriving. [`crate::driver`] has the full derivation.
//!
//! # Unsafe
//!
//! The whole crate's `unsafe` budget is [`poll`]'s single call. `wasi 0.11`
//! exposes `poll_oneoff` as a raw pointer pair because the ABI takes an input
//! array and an output array; the safe wrapper below owns both allocations, so
//! the pointers are valid and correctly sized by construction. Nothing else
//! here — and nothing anywhere else in the crate — needs it, which is why this
//! module is the only place `unsafe_code` is allowed.

#![allow(unsafe_code)]

use std::time::Duration;

/// Which subscription woke a [`poll`].
const STDIN: u64 = 1;
const CLOCK: u64 = 2;

/// Standard input. Fixed by the WASI ABI, not by a preopen.
const STDIN_FD: u32 = 0;

/// What [`wait_for_stdin`] observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Readiness {
    /// Stdin has bytes to read — or has reached end-of-file, which reports the
    /// same way and is told apart by a `read` returning zero.
    Readable,
    /// The deadline passed with stdin quiet.
    TimedOut,
    /// The host could not poll stdin at all. The caller must fall back to
    /// sleeping and probing, rather than spinning on a subscription that will
    /// never fire. See [`crate::driver`]'s degradation note.
    Unsupported,
}

/// How many times a transient `poll_oneoff` failure is retried before the call
/// simply reports "nothing arrived".
///
/// Small on purpose. Retrying restarts the *relative* clock subscription, so a
/// host that fails every call would otherwise stretch one slice without bound;
/// after this many attempts [`Readiness::TimedOut`] is both true enough and
/// safe, because the driver loops and asks again.
const MAX_POLL_RETRIES: u8 = 4;

/// Whether this errno means the host will *never* poll stdin, as opposed to
/// having failed this one call.
///
/// The two mistakes are not symmetric, and that sets the bar. A permanent
/// refusal misread as transient *hangs the session*: every [`wait_for_stdin`]
/// exhausts the retries and reports [`Readiness::TimedOut`], the driver never
/// latches its degraded mode, and the blocking read that would have worked is
/// never reached — `initialize` simply never answers. A transient misread as
/// permanent only costs the good path for the rest of the session. So the test
/// is whether an errno can describe a *moment*: if it cannot, it is permanent.
///
/// Permanent, and why none of them can describe a moment:
///
/// - `ENOTSUP` — the host does not implement `fd_read` subscriptions.
/// - `ENOSYS` — it does not implement `poll_oneoff` at all.
/// - `EINVAL` — it rejects the subscription as malformed. Every call sends the
///   same two subscriptions, so a rejection is a fact about the host.
/// - `EBADF` — it does not regard stdin as a pollable descriptor.
/// - `ENOTCAPABLE` — preview1's rights check: stdin carries `fd_read` (so the
///   degraded path's reads work) but not the right this poll needs. Rights can
///   only be dropped over a process's life, never granted, so the next call
///   cannot do better than this one.
/// - `EPERM` / `EACCES` — the host refuses the operation by policy rather than
///   by rights. Policy is fixed at instantiation for the same reason.
/// - `ENOTSOCK` — a host whose readiness polling covers sockets only. Stdin
///   will not become a socket.
///
/// Transient, and retried: `EINTR` above all, plus `EAGAIN` ("ask again" in so
/// many words), `ENOMEM` / `ENOBUFS` (pressure that eases), `EIO`, `ECANCELED`
/// and `ETIMEDOUT` — and, deliberately, every errno not named here at all,
/// because an unknown refusal must fall on the side whose mistake is the
/// recoverable one.
fn is_permanent(errno: wasi::Errno) -> bool {
    matches!(
        errno,
        wasi::ERRNO_NOTSUP
            | wasi::ERRNO_NOSYS
            | wasi::ERRNO_INVAL
            | wasi::ERRNO_BADF
            | wasi::ERRNO_NOTCAPABLE
            | wasi::ERRNO_PERM
            | wasi::ERRNO_ACCES
            | wasi::ERRNO_NOTSOCK
    )
}

/// Block until stdin is readable or `deadline` elapses, whichever comes first.
///
/// A zero `deadline` makes this a non-blocking readiness probe.
pub fn wait_for_stdin(deadline: Duration) -> Readiness {
    let nanos = u64::try_from(deadline.as_nanos()).unwrap_or(u64::MAX);
    let subscriptions = [subscribe_stdin(), subscribe_clock(nanos)];
    judge(|| poll(&subscriptions))
}

/// The retry-and-classify policy, over whatever `attempt` reports.
///
/// Separate from [`wait_for_stdin`] because the policy is the part that has to
/// be right on hosts we cannot obtain: a caller can hand it the refusals a real
/// `poll_oneoff` would have to be broken to produce.
fn judge(mut attempt: impl FnMut() -> Result<Vec<wasi::Event>, wasi::Errno>) -> Readiness {
    for _ in 0..MAX_POLL_RETRIES {
        // A failure of the call itself and an errno reported *on* the stdin
        // subscription mean the same thing and are judged the same way: only a
        // permanent refusal tells the driver to stop asking.
        let events = match attempt() {
            Ok(events) => events,
            Err(errno) if is_permanent(errno) => return Readiness::Unsupported,
            Err(_) => continue,
        };
        let mut readable = false;
        let mut transient = false;
        for event in &events {
            if event.userdata == STDIN {
                if event.error == wasi::ERRNO_SUCCESS {
                    readable = true;
                } else if is_permanent(event.error) {
                    return Readiness::Unsupported;
                } else {
                    transient = true;
                }
            }
        }
        if readable {
            return Readiness::Readable;
        }
        if !transient {
            return Readiness::TimedOut;
        }
    }
    Readiness::TimedOut
}

/// Sleep for `duration` using a clock-only `poll_oneoff`.
///
/// The fallback wait for a host whose `poll_oneoff` refuses `fd_read` on
/// stdin. Deliberately *not* `std::thread::sleep`: that is the same call
/// underneath, and naming it here keeps the degraded path's one syscall
/// visible next to the primary one.
pub fn sleep(duration: Duration) {
    let nanos = u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX);
    let _ = poll(&[subscribe_clock(nanos)]);
}

/// A `fd_read` subscription on stdin.
fn subscribe_stdin() -> wasi::Subscription {
    wasi::Subscription {
        userdata: STDIN,
        u: wasi::SubscriptionU {
            tag: wasi::EVENTTYPE_FD_READ.raw(),
            u: wasi::SubscriptionUU {
                fd_read: wasi::SubscriptionFdReadwrite {
                    file_descriptor: STDIN_FD,
                },
            },
        },
    }
}

/// A relative monotonic-clock subscription, `nanos` from now.
fn subscribe_clock(nanos: u64) -> wasi::Subscription {
    wasi::Subscription {
        userdata: CLOCK,
        u: wasi::SubscriptionU {
            tag: wasi::EVENTTYPE_CLOCK.raw(),
            u: wasi::SubscriptionUU {
                clock: wasi::SubscriptionClock {
                    id: wasi::CLOCKID_MONOTONIC,
                    timeout: nanos,
                    // Zero precision asks for the host's best effort; wasmtime
                    // treats it as "no coarsening".
                    precision: 0,
                    // No `SUBCLOCKFLAGS_SUBSCRIPTION_CLOCK_ABSTIME`: the
                    // timeout is relative to the moment of the call.
                    flags: 0,
                },
            },
        },
    }
}

/// The safe wrapper around `poll_oneoff`.
fn poll(subscriptions: &[wasi::Subscription]) -> Result<Vec<wasi::Event>, wasi::Errno> {
    let mut events: Vec<wasi::Event> = Vec::with_capacity(subscriptions.len());
    // SAFETY: `subscriptions` is a live slice, so its pointer is valid for
    // `subscriptions.len()` reads. `events` was just allocated with capacity
    // for exactly that many `Event`s, so its pointer is valid for that many
    // writes and the two regions cannot overlap. `poll_oneoff` returns how many
    // events it initialised, which is at most the subscription count, so the
    // `set_len` below only covers elements the host wrote.
    let written = unsafe {
        wasi::poll_oneoff(
            subscriptions.as_ptr(),
            events.as_mut_ptr(),
            subscriptions.len(),
        )
    }?;
    debug_assert!(written <= subscriptions.len());
    // SAFETY: as above — `written` elements were initialised by the host.
    unsafe { events.set_len(written.min(subscriptions.len())) };
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::{CLOCK, MAX_POLL_RETRIES, Readiness, STDIN, is_permanent, judge};

    /// Every errno the host can attach to the stdin subscription, or fail the
    /// whole call with, that means "never".
    const PERMANENT: [wasi::Errno; 8] = [
        wasi::ERRNO_NOTSUP,
        wasi::ERRNO_NOSYS,
        wasi::ERRNO_INVAL,
        wasi::ERRNO_BADF,
        wasi::ERRNO_NOTCAPABLE,
        wasi::ERRNO_PERM,
        wasi::ERRNO_ACCES,
        wasi::ERRNO_NOTSOCK,
    ];

    /// Refusals that describe a moment, plus the unknown errno that stands in
    /// for everything the enumeration does not name.
    const TRANSIENT: [wasi::Errno; 8] = [
        wasi::ERRNO_INTR,
        wasi::ERRNO_AGAIN,
        wasi::ERRNO_NOMEM,
        wasi::ERRNO_NOBUFS,
        wasi::ERRNO_IO,
        wasi::ERRNO_CANCELED,
        wasi::ERRNO_TIMEDOUT,
        wasi::ERRNO_BUSY,
    ];

    fn stdin_event(error: wasi::Errno) -> wasi::Event {
        wasi::Event {
            userdata: STDIN,
            error,
            type_: wasi::EVENTTYPE_FD_READ,
            fd_readwrite: wasi::EventFdReadwrite {
                nbytes: 0,
                flags: 0,
            },
        }
    }

    fn clock_event() -> wasi::Event {
        wasi::Event {
            userdata: CLOCK,
            error: wasi::ERRNO_SUCCESS,
            type_: wasi::EVENTTYPE_CLOCK,
            fd_readwrite: wasi::EventFdReadwrite {
                nbytes: 0,
                flags: 0,
            },
        }
    }

    #[test]
    fn the_permanent_set_is_exactly_the_refusals_that_cannot_change() {
        for errno in PERMANENT {
            assert!(
                is_permanent(errno),
                "{} names a host that will never poll stdin",
                errno.name()
            );
        }
    }

    #[test]
    fn transient_failures_and_unknown_errnos_are_retried() {
        for errno in TRANSIENT {
            assert!(
                !is_permanent(errno),
                "{} describes one call, not the host",
                errno.name()
            );
        }
        assert!(!is_permanent(wasi::ERRNO_SUCCESS));
    }

    /// The finding this classification exists for.
    ///
    /// A host that grants `fd_read` on stdin but not the polling right refuses
    /// with `ENOTCAPABLE`. Classified transient, it would spend every call's
    /// retries and report [`Readiness::TimedOut`] forever, so the driver would
    /// never latch its degraded mode and never read at all.
    #[test]
    fn a_notcapable_refusal_reports_unsupported_without_retrying() {
        let mut calls = 0_u8;
        let readiness = judge(|| {
            calls += 1;
            Err(wasi::ERRNO_NOTCAPABLE)
        });
        assert_eq!(readiness, Readiness::Unsupported);
        assert_eq!(calls, 1, "a permanent refusal is not worth asking twice");
    }

    /// The same judgement when the refusal rides on the event rather than on
    /// the call.
    #[test]
    fn a_notcapable_event_on_stdin_reports_unsupported() {
        let readiness = judge(|| Ok(vec![stdin_event(wasi::ERRNO_NOTCAPABLE), clock_event()]));
        assert_eq!(readiness, Readiness::Unsupported);
    }

    /// The retry budget is spent, and then the call reports "nothing arrived"
    /// rather than a refusal — the driver loops and asks again.
    #[test]
    fn an_always_interrupted_poll_exhausts_the_retries_and_times_out() {
        let mut calls = 0_u8;
        let readiness = judge(|| {
            calls += 1;
            Err(wasi::ERRNO_INTR)
        });
        assert_eq!(readiness, Readiness::TimedOut);
        assert_eq!(calls, MAX_POLL_RETRIES);
    }

    /// A transient errno reported *on* the stdin subscription spends the same
    /// budget as a failed call.
    #[test]
    fn a_transient_event_error_exhausts_the_retries_and_times_out() {
        let mut calls = 0_u8;
        let readiness = judge(|| {
            calls += 1;
            Ok(vec![stdin_event(wasi::ERRNO_AGAIN)])
        });
        assert_eq!(readiness, Readiness::TimedOut);
        assert_eq!(calls, MAX_POLL_RETRIES);
    }

    #[test]
    fn an_interruption_that_clears_reports_what_the_next_poll_saw() {
        let mut calls = 0_u8;
        let readiness = judge(|| {
            calls += 1;
            if calls == 1 {
                Err(wasi::ERRNO_INTR)
            } else {
                Ok(vec![stdin_event(wasi::ERRNO_SUCCESS)])
            }
        });
        assert_eq!(readiness, Readiness::Readable);
        assert_eq!(calls, 2);
    }

    #[test]
    fn a_readable_stdin_event_reports_readable() {
        let readiness = judge(|| Ok(vec![stdin_event(wasi::ERRNO_SUCCESS)]));
        assert_eq!(readiness, Readiness::Readable);
    }

    /// The deadline firing is an answer, not a failure: one clock event and no
    /// stdin event ends the call immediately.
    #[test]
    fn a_clock_only_wake_times_out_without_retrying() {
        let mut calls = 0_u8;
        let readiness = judge(|| {
            calls += 1;
            Ok(vec![clock_event()])
        });
        assert_eq!(readiness, Readiness::TimedOut);
        assert_eq!(calls, 1);
    }
}
