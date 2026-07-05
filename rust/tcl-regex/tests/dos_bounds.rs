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

//! Resource-bound regression tests for the three `DoS` classes a code review
//! reproduced against this engine:
//!
//! 1. **Parser recursion → stack overflow** — a pattern of `(`×4000 drove the
//!    recursive-descent `parse`/`parsebranch`/`parseqatom` cycle into a SIGABRT
//!    stack overflow. The parser now caps nesting depth and reports a clean
//!    `REG_ETOOBIG` compile error instead.
//! 2. **Backreference `ReDoS`** — `(a+)+\1$` against a run of "a"s sent the
//!    backtracking matcher exponential (~4s on 20 chars). A shared step budget
//!    now bounds the search; it bails (reporting no match) rather than hanging.
//! 3. **Reach-core blow-up** — plain `a*` over a long input is O(n²) and `(a*)*`
//!    cubic in the set-simulation core. The same budget bounds that core's total
//!    work, so these inputs return promptly instead of locking up.
//!
//! Each test simply asserts the engine *returns* — quickly. We do not pin the
//! exact match outcome of the bailing cases (a tripped guard legitimately yields
//! "no match"); the point is that no call hangs or aborts. The harness runs each
//! case on a worker thread with a wall-clock deadline so a regression that
//! reintroduces the hang fails loudly instead of stalling the suite.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use tcl_regex::ErrorCode;
use tcl_regex::Regex;
use tcl_regex::defs::REG_ADVANCED;

/// Run `f` on a worker thread, failing the test if it does not finish within
/// `secs`. A blown guard would hang here, so the deadline converts "engine hung"
/// into a deterministic test failure rather than a stuck process.
fn within<F>(secs: u64, label: &str, f: F)
where
    F: FnOnce() + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    // Give the worker the same 8 MB the main thread gets by default: the depth
    // guard is sized for a normal stack, and Rust's spawned threads otherwise
    // default to a smaller (2 MB) stack that would not represent real use.
    let handle = thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            f();
            // Ignore send errors: if the receiver already timed out and went
            // away, there is nothing to report to.
            let _ = tx.send(());
        })
        .expect("spawn worker thread");
    match rx.recv_timeout(Duration::from_secs(secs)) {
        Ok(()) => {
            handle.join().expect("worker thread panicked");
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {
            panic!("{label}: did not finish within {secs}s (guard failed to trip)")
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            // The worker dropped its sender without signalling — it panicked or
            // aborted; surface that as a test failure too.
            handle.join().expect("worker thread panicked");
            panic!("{label}: worker exited without completing");
        }
    }
}

/// Codepoints for a `&str`.
fn cps(s: &str) -> Vec<u32> {
    s.chars().map(|c| c as u32).collect()
}

#[test]
fn parser_deep_nesting_does_not_overflow() {
    // `(`×4000 used to overflow the stack (SIGABRT). The depth guard must
    // turn this into a normal compile error, fast.
    within(5, "deep-nesting compile", || {
        let pattern = cps(&"(".repeat(4000));
        let result = Regex::compile(&pattern, REG_ADVANCED);
        // It must *return* a resource-exhaustion compile error rather than abort
        // the process: the depth guard trips at the cap (before the unbalanced
        // `)` would be noticed), so the first error reported is ETOOBIG.
        assert_eq!(
            result.err(),
            Some(ErrorCode::Etoobig),
            "deeply nested pattern should bail with REG_ETOOBIG, not abort or match"
        );
    });
}

#[test]
fn backref_redos_is_bounded() {
    // `(a+)+\1$` on a run of "a"s is the textbook catastrophic-backtracking
    // case. The step budget must make it bail quickly. We only require that exec
    // *returns* (Some or None both acceptable — a tripped guard yields None).
    within(5, "backref ReDoS exec", || {
        let re = Regex::compile_str("(a+)+\\1$", REG_ADVANCED).expect("compiles");
        let subject = cps(&"a".repeat(30));
        let _ = re.exec(&subject, 0, 0);
    });
}

#[test]
fn reach_linear_star_is_bounded() {
    // plain `a*` over a long input exercises the O(n^2) reach scan. With
    // the fuel guard it must finish promptly regardless of outcome.
    within(5, "a* large-input exec", || {
        let re = Regex::compile_str("a*", REG_ADVANCED).expect("compiles");
        let subject = cps(&"a".repeat(20000));
        let got = re.exec(&subject, 0, 0);
        // `a*` always matches (at least the empty string at position 0), so a
        // result is expected here even though the budget may curtail the scan.
        assert!(got.is_some(), "a* should match somewhere");
    });
}

#[test]
fn reach_nested_star_is_bounded() {
    // `(a*)*` is cubic in the reach core. The shared budget must bound it.
    within(5, "(a*)* exec", || {
        let re = Regex::compile_str("(a*)*", REG_ADVANCED).expect("compiles");
        let subject = cps(&"a".repeat(2000));
        let _ = re.exec(&subject, 0, 0);
    });
}
