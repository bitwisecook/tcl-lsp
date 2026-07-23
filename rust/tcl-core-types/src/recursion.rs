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

//! The shared native-stack safety-net primitive for every recursive-descent
//! walker in the workspace — [`RecursionLimit`] + [`RecursionGuard`]
//! (issue #996).
//!
//! Every stage of the pipeline that walks nested Tcl structures (the
//! compiler's `Script`/`Statement` IR, the optimiser passes, the WASM
//! codegen backend, the formatter, the minifier, the iRules reference
//! walker, the bytecode VM's runtime-command fallback, the standalone WASM
//! Tcl runtime's execution engine, …) does it by native recursive descent:
//! one Rust stack-frame group per source-nesting level. Nothing in Rust
//! stops that from becoming an uncatchable process abort (`SIGABRT`) on
//! sufficiently deep input — unlike a Tcl-level `proc`-call stack (bounded
//! by `interp recursionlimit`, a catchable error). See
//! `docs/design/compiler/recursive-descent-depth-limits.md` for the full
//! investigation, the empirical crash-threshold measurements behind each
//! call site's chosen limit, and why the limits differ so much between
//! walkers (from 16 up to 256, depending on each recursion's actual
//! per-level native-stack cost and its ambient stack budget).
//!
//! This module intentionally centralises only the *bookkeeping* — compare,
//! increment, decrement, once, tested thoroughly — not the "what happens
//! when the limit trips" behaviour, which stays domain-specific by design:
//! an analyser walker emits a diagnostic and stops descending, a lowering
//! pass emits a barrier statement, a codegen backend falls back to a
//! whole-construct eval, a formatter/minifier leaves the excess nesting
//! untouched, an interpreter raises a catchable "too many nested
//! evaluations" error. Forcing those into one shared callback would either
//! be a leaky abstraction or lose domain-specific correctness; the
//! `RecursionLimit`/`RecursionGuard` split below deliberately stops short of
//! that.
//!
//! Two call-site shapes exist in practice, both built on the same
//! `depth > limit` comparison ([`RecursionLimit::exceeded`]):
//!
//! - **An explicit `depth: u32` parameter threaded through recursive
//!   calls** (the common case: the analyser, the lowering pass, the
//!   optimiser passes, the codegen structured-walk, the iRules walker, the
//!   formatter, the minifier). Each call already carries a `depth` — the
//!   nesting level of the node it's about to process — so the guard is a
//!   direct call to [`RecursionLimit::exceeded`] at the top of the
//!   function, no state to own.
//! - **A counter stored on a long-lived struct** (the bytecode VM's
//!   `cmd_control.rs` fallback, the standalone WASM Tcl runtime's
//!   `eval_script_mode`), incremented before a nested call and decremented
//!   after. [`RecursionGuard`] wraps that pattern in RAII: it can never be
//!   forgotten on an early-return path (or a panic unwind) the way a manual
//!   increment/decrement pair can.

/// A recursive-descent walker's maximum nesting depth. Wraps a plain `u32`
/// so every guarded walker's limit constant shares one type — searching for
/// `RecursionLimit` finds every one of them — rather than each declaring its
/// own bare `const MAX_X_DEPTH: u32`.
///
/// See the module docs for the two call-site shapes this supports and why
/// the "what happens when it trips" behaviour deliberately stays outside
/// this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RecursionLimit(pub u32);

impl RecursionLimit {
    /// Whether `depth` — the nesting level of the node/call about to run,
    /// *including* it — has exceeded this limit.
    ///
    /// For the explicit-depth-parameter shape, pass the depth of the call
    /// about to execute directly (`RecursionLimit(256).exceeded(depth)` at
    /// the top of the recursive function, before doing any work).
    ///
    /// For the struct-field-counter shape, this is what
    /// [`RecursionGuard::enter`] calls internally with `*counter + 1` (the
    /// value the counter is about to become) — algebraically identical to
    /// the more common "reject when the counter has already reached the
    /// limit" (`*counter >= limit.0`) phrasing, since `(counter + 1) >
    /// limit ⟺ counter >= limit` for integers. Prefer [`RecursionGuard`]
    /// over calling this directly for that shape; it also keeps the
    /// counter itself correctly balanced.
    #[must_use]
    pub const fn exceeded(self, depth: u32) -> bool {
        depth > self.0
    }
}

/// An RAII guard over a `&mut u32` depth counter stored on a long-lived
/// struct — see the module docs' second call-site shape.
///
/// [`RecursionGuard::enter`] checks the counter against a [`RecursionLimit`]
/// and, if still within bounds, increments it and returns a guard that
/// decrements it again on drop — including on an early return via `?`, an
/// early `break`/`return` in the caller, or an unexpected panic unwind.
/// Manually paired increment/decrement calls (the pattern this replaces)
/// have no such guarantee: every early-return path has to remember the
/// decrement, and a caller that forgets one leaves the counter permanently
/// too high, eventually refusing legitimate recursion it should have
/// allowed.
pub struct RecursionGuard<'a> {
    counter: &'a mut u32,
}

impl<'a> RecursionGuard<'a> {
    /// Enter one level of recursion tracked by `counter`, checked against
    /// `limit`. On success, `*counter` is incremented and the returned
    /// guard decrements it again when dropped. On failure (the limit is
    /// already reached), `*counter` is left unchanged and the limit is
    /// returned so the caller can build its own domain-specific error from
    /// it.
    pub fn enter(counter: &'a mut u32, limit: RecursionLimit) -> Result<Self, RecursionLimit> {
        if limit.exceeded(*counter + 1) {
            return Err(limit);
        }
        *counter += 1;
        Ok(Self { counter })
    }

    /// The live depth this guard (and any guard nested inside it) counts
    /// against — `*counter` at the moment of the call, i.e. including this
    /// guard's own `enter`.
    #[must_use]
    pub fn depth(&self) -> u32 {
        *self.counter
    }
}

impl Drop for RecursionGuard<'_> {
    fn drop(&mut self) {
        *self.counter = self.counter.saturating_sub(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exceeded_is_strictly_greater_than() {
        let limit = RecursionLimit(256);
        assert!(!limit.exceeded(0));
        assert!(!limit.exceeded(255));
        assert!(!limit.exceeded(256), "depth == limit is still allowed");
        assert!(limit.exceeded(257), "one past the limit trips it");
        assert!(limit.exceeded(1000));
    }

    #[test]
    fn zero_limit_rejects_any_depth() {
        let limit = RecursionLimit(0);
        assert!(!limit.exceeded(0));
        assert!(limit.exceeded(1));
    }

    #[test]
    fn guard_enter_succeeds_up_to_the_limit() {
        // limit = 3: three successful `enter` calls (counter 0 -> 1 -> 2 -> 3),
        // matching "RecursionLimit(N) allows up to N levels" for the
        // counter-based shape.
        let limit = RecursionLimit(3);
        let mut counter = 0u32;
        {
            let g1 = RecursionGuard::enter(&mut counter, limit).expect("1st enter");
            assert_eq!(g1.depth(), 1);
        }
        // The first guard's drop already ran (scope closed above); counter
        // is back to 0. Re-enter to depth 1, 2, 3, confirming each level.
        let g1 = RecursionGuard::enter(&mut counter, limit).expect("re-enter to depth 1");
        assert_eq!(*g1.counter, 1);
        let g2 = RecursionGuard::enter(g1.counter, limit).expect("enter to depth 2");
        assert_eq!(*g2.counter, 2);
        let g3 = RecursionGuard::enter(g2.counter, limit).expect("enter to depth 3");
        assert_eq!(*g3.counter, 3);
    }

    #[test]
    fn guard_enter_rejects_past_the_limit_without_mutating_the_counter() {
        let limit = RecursionLimit(2);
        let mut counter = 2u32; // already at the limit
        let before = counter;
        let err = RecursionGuard::enter(&mut counter, limit)
            .err()
            .expect("entering past the limit must fail");
        assert_eq!(err, limit);
        assert_eq!(
            counter, before,
            "a rejected enter must not increment the counter"
        );
    }

    #[test]
    fn guard_drop_decrements_the_counter() {
        let limit = RecursionLimit(10);
        let mut counter = 0u32;
        {
            let g = RecursionGuard::enter(&mut counter, limit).unwrap();
            assert_eq!(g.depth(), 1);
        }
        assert_eq!(counter, 0, "the counter must return to 0 once the guard drops");
    }

    #[test]
    fn nested_guards_drop_in_lifo_order() {
        let limit = RecursionLimit(10);
        let mut counter = 0u32;
        let g1 = RecursionGuard::enter(&mut counter, limit).unwrap();
        assert_eq!(*g1.counter, 1);
        {
            let g2 = RecursionGuard::enter(g1.counter, limit).unwrap();
            assert_eq!(*g2.counter, 2);
            {
                let g3 = RecursionGuard::enter(g2.counter, limit).unwrap();
                assert_eq!(*g3.counter, 3);
            }
            // g3 dropped.
            assert_eq!(*g2.counter, 2);
        }
        // g2 dropped.
        assert_eq!(*g1.counter, 1);
        drop(g1);
        assert_eq!(counter, 0);
    }

    /// A guard created inside a function that returns early via `?` still
    /// decrements the counter — the exact bug class a manual
    /// increment/decrement pair is prone to (issue #996's `tcl_vm` fix
    /// needed careful manual auditing of every early-return path for
    /// exactly this).
    #[test]
    fn guard_decrements_on_early_return_via_question_mark() {
        fn maybe_fails(counter: &mut u32, limit: RecursionLimit, fail: bool) -> Result<(), ()> {
            let _guard = RecursionGuard::enter(counter, limit).map_err(|_| ())?;
            if fail {
                return Err(()); // early return — the guard must still drop.
            }
            Ok(())
        }

        let limit = RecursionLimit(5);
        let mut counter = 0u32;
        assert!(maybe_fails(&mut counter, limit, true).is_err());
        assert_eq!(
            counter, 0,
            "the guard must decrement even on the early-return path"
        );
        assert!(maybe_fails(&mut counter, limit, false).is_ok());
        assert_eq!(counter, 0);
    }

    // No panic-unwind test here: this crate is `#![no_std]` and forbids
    // `unsafe`, and `std::panic::catch_unwind` needs both. `Drop` running
    // during unwind is a Rust language guarantee, not something this
    // crate's own tests need to re-verify independently; the early-return
    // test above already covers the application-specific property (every
    // non-panic early-exit path balances the counter correctly).

    #[test]
    fn guard_depth_reports_the_current_counter_value() {
        let limit = RecursionLimit(10);
        let mut counter = 0u32;
        let g1 = RecursionGuard::enter(&mut counter, limit).unwrap();
        assert_eq!(g1.depth(), 1);
        let g2 = RecursionGuard::enter(g1.counter, limit).unwrap();
        assert_eq!(g2.depth(), 2);
    }
}
