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

//! Resource-bound regression tests for the `DoS` classes a code review (and,
//! for #4, a later systematic sweep for issue #996) reproduced against this
//! engine:
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
//! 4. **Dissection/backtrack recursion → stack overflow** (issue #996) —
//!    matching a repeat quantifier against a long subject recurses once per
//!    matched iteration in both `Matcher::dissect_repeat` (the POSIX
//!    dissection phase run after every repeat match, not just backreference
//!    ones) and `Bt::m_star`/`Bt::m_backref` (the backreference backtracking
//!    path). `MATCH_FUEL` bounds neither as a *depth* budget — it is a
//!    *work* budget, and for `Bt` in particular each recursive step
//!    costs a fixed 1 unit regardless of native stack use, so depth can run
//!    far ahead of any work-based bail-out. Both now cap their recursion
//!    depth (`MAX_DISSECT_DEPTH` / `MAX_BT_DEPTH` in `exec.rs`) and degrade
//!    gracefully — an approximate nested-capture span, or a clean "no match"
//!    along that backtracking branch — instead of aborting the process.
//!
//! Each test simply asserts the engine *returns* — quickly. We do not pin the
//! exact match outcome of the bailing cases (a tripped guard legitimately yields
//! "no match"); the point is that no call hangs or aborts. The harness runs each
//! case on a worker thread with a wall-clock deadline so a regression that
//! reintroduces the hang fails loudly instead of stalling the suite. The #4
//! tests additionally run on the default (un-spawned) test thread — `cargo
//! test`'s per-test default 2 MiB stack — since that is the exact ambient
//! stack the depth caps below were calibrated against.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use tcl_regex::ErrorCode;
use tcl_regex::Regex;
use tcl_regex::Span;
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

/// Regression coverage for issue #996: `Matcher::dissect_repeat`'s `min == 0`
/// branch recurses once per matched iteration of a repeated sub-pattern, with
/// no depth cap before this fix — reachable from the completely ordinary
/// `a*` (no backreference needed). Empirically (with the fuel budget
/// temporarily raised to isolate the depth effect from `MATCH_FUEL`
/// exhaustion), unguarded input overflowed the native stack (SIGABRT)
/// between depth 2400 and 2420 on a 2 MiB thread (`cargo test`'s per-test
/// default). 10,000 is comfortably past both that crash range and
/// `MAX_DISSECT_DEPTH` (256); the assertion is that `exec` returns at all,
/// not what it returns — though for plain `a*` it always matches, so we
/// also sanity-check that.
#[test]
fn deeply_nested_dissect_repeat_survives() {
    let re = Regex::compile_str("a*", REG_ADVANCED).expect("compiles");
    let subject = cps(&"a".repeat(10_000));
    let got = re.exec(&subject, 0, 0);
    assert!(got.is_some(), "a* should match somewhere");
}

/// Regression coverage for issue #996: `Bt::m_star` (the backtracking
/// matcher's repeat handling, used only when the pattern contains a
/// backreference) recurses once per matched iteration of a repeated
/// sub-pattern, with no depth cap before this fix. Empirically, unguarded
/// input overflowed the native stack (SIGABRT) between depth 2200 and 2300
/// on a 2 MiB thread (`cargo test`'s per-test default). 10,000 is
/// comfortably past both that crash range and `MAX_BT_DEPTH` (256); the
/// assertion is that `exec` returns at all, not what it returns.
#[test]
fn deeply_nested_backtrack_repeat_survives() {
    let re = Regex::compile_str("a*(b)\\1", REG_ADVANCED).expect("compiles");
    let mut s = "a".repeat(10_000);
    s.push_str("bb");
    let subject = cps(&s);
    let _ = re.exec(&subject, 0, 0);
}

/// Regression coverage for issue #996: `Bt::m_backref` (a quantified
/// backreference, e.g. `\1*`) recurses once per repetition of the
/// backreference, with no depth cap before this fix. Empirically, unguarded
/// input overflowed the native stack (SIGABRT) between depth 2400 and 2500
/// on a 2 MiB thread (`cargo test`'s per-test default). 10,000 is
/// comfortably past both that crash range and `MAX_BT_DEPTH` (256); the
/// assertion is that `exec` returns at all, not what it returns.
#[test]
fn deeply_nested_backref_repeat_survives() {
    let re = Regex::compile_str("(x)\\1*", REG_ADVANCED).expect("compiles");
    let subject = cps(&"x".repeat(10_001));
    let _ = re.exec(&subject, 0, 0);
}

/// A capture nested inside a repeat, well under `MAX_DISSECT_DEPTH` (256),
/// still records the correct *final-iteration* span — the depth guard must
/// not fire, and dissection must be byte-for-byte unaffected, at realistic
/// nesting depths.
#[test]
fn moderately_nested_capture_in_repeat_is_unaffected() {
    let re = Regex::compile_str("(x)*", REG_ADVANCED).expect("compiles");
    let subject = cps(&"x".repeat(200));
    let got = re.exec(&subject, 0, 0).expect("(x)* matches 200 x's");
    assert_eq!(
        got[0],
        Some(Span { start: 0, end: 200 }),
        "whole match spans the subject"
    );
    assert_eq!(
        got[1],
        Some(Span {
            start: 199,
            end: 200
        }),
        "capture 1 records the last (200th) iteration, one character wide"
    );
}

/// A quantified backreference repeated well under `MAX_BT_DEPTH` (256) still
/// matches — the depth guard must not fire at realistic repeat counts.
#[test]
fn moderately_nested_backref_repeat_is_unaffected() {
    let re = Regex::compile_str("(x)\\1*", REG_ADVANCED).expect("compiles");
    let subject = cps(&"x".repeat(101));
    let got = re.exec(&subject, 0, 0);
    assert_eq!(
        got.map(|c| c[0]),
        Some(Some(Span { start: 0, end: 101 })),
        "moderate backref repeat should still match the whole subject"
    );
}
