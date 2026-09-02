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

//! Oracle-pinned coverage for the r3-numeric-tower lane (#1428, #1382,
//! #1432, #1581) on the `runtime/rust` engine.
//!
//! Every expected value, message and `-errorcode` below was read verbatim
//! out of `tclsh9.0` (9.0.4) — and, where the two releases agree, also
//! `tclsh8.6` (8.6.16) — with the sheet
//!
//! ```tcl
//! catch {expr {...}} m o; list $m [dict get $o -errorcode]
//! ```
//!
//! so a reader can re-derive any expectation in a real shell without this
//! harness. The matching pins for the bytecode VM live in
//! `rust/tcl-vm/tests/numeric_tower_e2e.rs`; the two files assert the same
//! rows so the engines cannot drift.

use tcl_runtime::interp::{Code, Interp};

/// Evaluate `script`, returning `(result-string, errorCode)`. `errorCode` is
/// read back from `::errorCode`, which is what
/// `catch … opts; dict get $opts -errorcode` observes in real Tcl.
fn run(script: &str) -> (Code, String, String) {
    let mut interp = Interp::new();
    let code = interp.eval_str(script.as_bytes());
    let result = String::from_utf8_lossy(&interp.result_bytes()).into_owned();
    let error_code = if code == Code::Error {
        interp.eval_str(b"set ::errorCode");
        String::from_utf8_lossy(&interp.result_bytes()).into_owned()
    } else {
        String::new()
    };
    (code, result, error_code)
}

/// `expr $body` must succeed with exactly `expected`.
#[track_caller]
fn expr_is(body: &str, expected: &str) {
    let (code, result, _) = run(&format!("expr {{{body}}}"));
    assert_eq!(code, Code::Ok, "expr {{{body}}} → {result}");
    assert_eq!(result, expected, "expr {{{body}}}");
}

/// `expr $body` must fail with exactly `message` and `-errorcode` `code`.
#[track_caller]
fn expr_err(body: &str, message: &str, code: &str) {
    let (status, result, error_code) = run(&format!("expr {{{body}}}"));
    assert_eq!(status, Code::Error, "expr {{{body}}} → {result}");
    assert_eq!(result, message, "expr {{{body}}} message");
    assert_eq!(error_code, code, "expr {{{body}}} errorCode");
}

// ---------------------------------------------------------------------------
// #1428 — `**`, `<<`, `>>`, `/` and `%` route their integer tier through
// `tcl_syntax::number_tower`, so `0 ** -1` is C's domain error (not a
// division by zero) and the 2^28 exponent ceiling refuses instead of
// allocating.
// ---------------------------------------------------------------------------

/// tclsh 8.6.16/9.0.4: `exponentiation of zero by negative power`,
/// `-errorcode ARITH DOMAIN {exponentiation of zero by negative power}` —
/// **not** `divide by zero` / `ARITH DIVZERO`.
#[test]
fn zero_to_a_negative_power_is_a_domain_error_not_divide_by_zero() {
    const MSG: &str = "exponentiation of zero by negative power";
    const CODE: &str = "ARITH DOMAIN {exponentiation of zero by negative power}";
    for body in [
        "0 ** -1",
        "0 ** -2",
        "0 ** -100000000000000000000",
        // The float tier collapses identically (tclsh: every one of these is
        // the same domain error, never `Inf`).
        "0.0 ** -1",
        "0 ** -1.0",
        "0.0 ** -1.0",
    ] {
        expr_err(body, MSG, CODE);
    }
}

/// A real division by zero keeps its own class: tclsh `divide by zero`,
/// `-errorcode ARITH DIVZERO {divide by zero}`. The FP guard for the test
/// above — the two refusals must not be merged back together.
#[test]
fn divide_by_zero_keeps_its_own_class() {
    for body in ["1 / 0", "1 % 0", "(2**70) / 0", "(2**70) % 0"] {
        expr_err(body, "divide by zero", "ARITH DIVZERO {divide by zero}");
    }
}

/// tclsh 8.6.16/9.0.4: an exponent at or past 2^28 is `exponent too large`
/// (`-errorcode NONE`) — refused instantly, never computed. Before #1428 the
/// runtime attempted the allocation (a multi-hundred-megabit result), so this
/// test also guards the resource-exhaustion vector: it would time out, not
/// merely fail.
#[test]
fn the_exponent_ceiling_refuses_instead_of_allocating() {
    for body in ["2 ** 268435456", "3 ** 268435456", "(2**70) ** 268435456"] {
        expr_err(body, "exponent too large", "NONE");
    }
}

/// The ceiling does **not** swallow the collapsing bases: tclsh computes
/// `0`/`±1` powers at any exponent, and floors a non-collapsing base with a
/// negative exponent to `0`.
#[test]
fn the_collapsing_bases_still_answer_at_any_exponent() {
    expr_is("0 ** 268435456", "0");
    expr_is("1 ** 268435456", "1");
    expr_is("-1 ** 268435456", "1");
    expr_is("(-1) ** 268435457", "-1");
    expr_is("(-1) ** 100000000000000000001", "-1");
    expr_is("(-1) ** 100000000000000000000", "1");
    expr_is("2 ** -100000000000000000000", "0");
    expr_is("7 ** -3", "0");
    expr_is("0 ** 0", "1");
    expr_is("0.0 ** 0", "1.0");
}

/// Floor division and modulus (sign follows the divisor) across the wide and
/// beyond-wide tiers — the rows the tower's conformance corpus pins, now
/// reached by production `expr`.
#[test]
fn floor_division_and_modulus_match_the_oracle() {
    expr_is("-7 / 2", "-4");
    expr_is("-7 % 2", "1");
    expr_is("7 % -2", "-1");
    expr_is("(2**64) % 7", "2");
    expr_is("(2**64) / -3", "-6148914691236517206");
    // i64::MIN / -1 promotes rather than overflowing.
    expr_is("(-9223372036854775807 - 1) / -1", "9223372036854775808");
}

/// Shift edges: the sign collapse past the operand width, the negative-count
/// refusal, the `<<` count overflow, and C's zero-base short circuit.
#[test]
fn shift_edges_match_the_oracle() {
    expr_is("2 >> 100000000000000000000", "0");
    expr_is("-2 >> 100000000000000000000", "-1");
    expr_is("-1 >> 100000000000000000000", "-1");
    expr_is("0 >> 100000000000000000000", "0");
    expr_is("-8 >> 1", "-4");
    expr_is("1 << 100", "1267650600228229401496703205376");
    // C checks the zero base before the count, so these are 0, not errors.
    expr_is("0 << 100000000000000000000", "0");
    expr_is("0 << 2147483648", "0");
    expr_err("1 << -1", "negative shift argument", "NONE");
    expr_err("1 >> -1", "negative shift argument", "NONE");
    expr_err(
        "1 << 100000000000000000000",
        "integer value too large to represent",
        "NONE",
    );
    expr_err(
        "1 << 2147483648",
        "integer value too large to represent",
        "NONE",
    );
}

/// tclsh reports the *left operand* before it looks at the shift count:
/// `expr {1.5 >> -1}` is the operand-type error, not `negative shift
/// argument`.
#[test]
fn a_float_left_operand_beats_a_negative_shift_count() {
    let (code, result, _) = run("expr {1.5 >> -1}");
    assert_eq!(code, Code::Error);
    assert_eq!(
        result,
        "cannot use floating-point value \"1.5\" as left operand of \">>\""
    );
}
