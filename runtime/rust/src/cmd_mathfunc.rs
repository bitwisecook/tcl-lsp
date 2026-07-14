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

//! `::tcl::mathfunc::*` — the math functions as **real, overridable commands**
//! (T1.5, the registry-backed convergence).
//!
//! In C Tcl 9 (`tclBasic.c`) `expr`'s function calls dispatch as commands in the
//! `::tcl::mathfunc` namespace, so they are overridable/renamable. This registers
//! one builtin per function name; each forwards to the **shared**
//! [`tcl_syntax::expr::mathfunc::dispatch`] (the same dispatch the compiler
//! const-folds with, and `expr`'s fallback). `expr`'s function-call path
//! (`interp.eval_math_call`) resolves `::tcl::mathfunc::NAME` through the command
//! table first, so a user `proc ::tcl::mathfunc::foo` or a `rename` is honoured —
//! verified against tclsh 9.0.
//!
//! Needs the numeric tower (`as_math_num`), so it tracks the `have_tommath` cfg
//! like `expr` itself. `rand`/`srand` carry PRNG state on the interp, so they
//! are handled here directly rather than via the pure shared dispatch.

use tcl_syntax::expr::mathfunc::{dispatch, Num};
use tcl_syntax::naming::qualifier_segments;

use crate::interp::{obj_bytes, Code, Interp};
use crate::obj::{self, TclObj};

/// Every math function — registered as `::tcl::mathfunc::<name>`. Most forward
/// to the shared [`dispatch`]; `rand`/`srand` are handled inline (interp state).
const MATHFUNCS: &[&str] = &[
    "abs",
    "acos",
    "asin",
    "atan",
    "atan2",
    "bool",
    "ceil",
    "cos",
    "cosh",
    "double",
    "entier",
    "exp",
    "floor",
    "fmod",
    "hypot",
    "int",
    "isqrt",
    "isfinite",
    "isinf",
    "isnan",
    "isnormal",
    "issubnormal",
    "isunordered",
    "log",
    "log10",
    "max",
    "min",
    "pow",
    "rand",
    "round",
    "sin",
    "sinh",
    "sqrt",
    "srand",
    "tan",
    "tanh",
    "wide",
];

/// Register `::tcl::mathfunc::*`.
pub fn install(interp: &mut Interp) {
    for name in MATHFUNCS {
        let mut full = b"::tcl::mathfunc::".to_vec();
        full.extend_from_slice(name.as_bytes());
        interp.register_builtin(&full, mathfunc);
    }
}

/// The shared implementation behind every `::tcl::mathfunc::NAME`. The function
/// is `argv[0]`'s simple tail (so one builtin serves all names); operands are
/// read off the tower as [`Num`] and dispatched.
fn mathfunc(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let name0 = obj_bytes(argv[0]);
    let tail = qualifier_segments(&name0)
        .last()
        .copied()
        .unwrap_or(&name0[..]);
    let Ok(fname) = core::str::from_utf8(tail) else {
        return interp.set_error(b"unknown math function");
    };

    // Argument-count check first (C reports this before operand conversion), with
    // `-errorcode TCL WRONGARGS`.
    let lname = fname.to_ascii_lowercase();
    let (amin, amax) = arity(&lname);
    let n = argv.len() - 1;
    if n < amin {
        return wrong_func_args(interp, b"not enough arguments for math function \"", fname);
    }
    if amax.is_some_and(|m| n > m) {
        return wrong_func_args(interp, b"too many arguments for math function \"", fname);
    }

    // `rand`/`srand` carry PRNG state on the interp, so they bypass the pure
    // shared dispatch (C's `ExprRandFunc`/`ExprSrandFunc`).
    match lname.as_str() {
        "rand" => {
            interp.set_result(obj::new_double_obj(interp.rand_next()));
            return Code::Ok;
        }
        "srand" => {
            // The seed is the operand's low 64 bits (C's `TclGetWideBitsFromObj`);
            // a non-integer operand is an error.
            if !crate::bignum::is_integer(argv[1]) {
                return interp.set_error(b"argument to math function didn't have numeric value");
            }
            let seed = crate::bignum::truncate_to_wide(argv[1]);
            interp.set_result(obj::new_double_obj(interp.srand(seed)));
            return Code::Ok;
        }
        _ => {}
    }

    // `wide`/`int`/`entier` on an *integer* operand work on the tower directly,
    // not via the f64-limited shared dispatch: `wide` wraps a too-big integer to
    // a 64-bit signed value (C's truncation), and `int`/`entier` keep the
    // (possibly bignum) integer exactly. (A *float* operand falls through to the
    // shared dispatch.)
    if matches!(lname.as_str(), "wide" | "int" | "entier") && crate::bignum::is_integer(argv[1]) {
        if lname == "wide" {
            interp.set_result(obj::new_wide_int_obj(crate::bignum::truncate_to_wide(
                argv[1],
            )));
        } else {
            interp.set_result(argv[1]); // integer is its own int/entier
        }
        return Code::Ok;
    }

    // Operands → Num (object-preserving: a bignum/double keeps its rep).
    let nums: Option<Vec<Num>> = argv[1..]
        .iter()
        .map(|&o| crate::bignum::as_math_num(o))
        .collect();
    let Some(nums) = nums else {
        return interp.set_error(b"argument to math function didn't have numeric value");
    };

    // `set_result` adopts a fresh rc-0 obj (retains it; no extra drop needed).
    match dispatch(&lname, &nums) {
        Some(Num::Int(i)) => {
            interp.set_result(obj::new_wide_int_obj(i));
            Code::Ok
        }
        Some(Num::Float(f)) => {
            interp.set_result(obj::new_double_obj(f));
            Code::Ok
        }
        // A registered name with the right arity reaching `None` is a domain
        // error (e.g. `sqrt(-1)`), with `-errorcode ARITH DOMAIN`.
        None => interp.error_with_code(
            b"domain error: argument not in valid range",
            b"ARITH DOMAIN {domain error: argument not in valid range}",
        ),
    }
}

/// `(min, max)` argument count for a math function (`max` = `None` ⇒ unbounded).
/// Every name reaching the builtin is registered, so unknowns can't occur; the
/// default is the unary `(1, 1)`.
fn arity(name: &str) -> (usize, Option<usize>) {
    match name {
        "min" | "max" => (1, None),
        "atan2" | "hypot" | "fmod" | "pow" | "isunordered" => (2, Some(2)),
        "rand" => (0, Some(0)),
        _ => (1, Some(1)),
    }
}

/// `{not enough|too many} arguments for math function "NAME"` with
/// `-errorcode TCL WRONGARGS`.
fn wrong_func_args(interp: &mut Interp, prefix: &[u8], fname: &str) -> Code {
    let mut m = prefix.to_vec();
    m.extend_from_slice(fname.as_bytes());
    m.push(b'"');
    interp.error_with_code(&m, b"TCL WRONGARGS")
}

#[cfg(test)]
mod tests {
    use crate::counters;
    use crate::interp::{Code, Interp};

    fn leak_free(body: impl FnOnce(&mut Interp)) {
        counters::reset();
        {
            let mut interp = Interp::new();
            body(&mut interp);
        }
        assert_eq!(
            counters::finalize(),
            0,
            "residual: {} objs, {} bufs",
            counters::live_objs(),
            counters::live_bufs()
        );
        assert_eq!(counters::double_free_count(), 0);
    }

    #[test]
    fn mathfunc_callable_as_command() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"::tcl::mathfunc::sqrt 4"), Code::Ok);
            assert_eq!(i.result_bytes(), b"2.0");
            assert_eq!(i.eval_str(b"::tcl::mathfunc::abs -7"), Code::Ok);
            assert_eq!(i.result_bytes(), b"7");
            assert_eq!(i.eval_str(b"::tcl::mathfunc::max 1 9 3"), Code::Ok);
            assert_eq!(i.result_bytes(), b"9");
        });
    }

    #[test]
    fn expr_routes_through_the_command_table() {
        leak_free(|i| {
            // `expr` still works (the builtin forwards to the shared dispatch) …
            assert_eq!(i.eval_str(b"expr {sqrt(4)}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"2.0");
            // … and resolves the function *through the command table*: renaming
            // the command away breaks `expr` with C's invalid-command error.
            assert_eq!(i.eval_str(b"rename ::tcl::mathfunc::sqrt {}"), Code::Ok);
            assert_eq!(i.eval_str(b"expr {sqrt(4)}"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"invalid command name \"tcl::mathfunc::sqrt\""
            );
        });
    }

    #[test]
    fn override_via_alias_wins_in_expr() {
        // A redirect of `::tcl::mathfunc::sqrt` → `::tcl::mathfunc::abs` is honoured
        // by `expr` (proves the override hook without needing procs).
        leak_free(|i| {
            assert_eq!(
                i.eval_str(b"interp alias {} ::tcl::mathfunc::sqrt {} ::tcl::mathfunc::abs"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"expr {sqrt(-4)}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"4"); // abs(-4), not sqrt
        });
    }

    /// `expr` resolves a function `f` as the *relative* name
    /// `tcl::mathfunc::f` from the CURRENT namespace, so a namespace-local
    /// `::ns::tcl::mathfunc::f` shadows the global function inside `::ns`
    /// (tclsh 8.6.16 / 9.0.4-pinned; the mathfunc rows in the shared
    /// conformance vector table pin the command-call shape).
    #[test]
    fn namespace_local_mathfunc_shadows_in_expr() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(b"proc ::tcl::mathfunc::pf {x} { return 10 }"),
                Code::Ok
            );
            assert_eq!(
                i.eval_str(b"namespace eval ::nsa::tcl::mathfunc {}"),
                Code::Ok
            );
            assert_eq!(
                i.eval_str(b"proc ::nsa::tcl::mathfunc::pf {x} { return 20 }"),
                Code::Ok
            );
            assert_eq!(
                i.eval_str(b"namespace eval ::nsa { expr {pf(1)} }"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"20");
            // From any namespace without a local shadow, global wins.
            assert_eq!(
                i.eval_str(b"namespace eval ::nsb { expr {pf(1)} }"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"10");
            assert_eq!(i.eval_str(b"expr {pf(1)}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"10");
        });
    }

    #[test]
    fn unknown_function_reports_invalid_command() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"expr {frobnicate(1)}"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"invalid command name \"tcl::mathfunc::frobnicate\""
            );
        });
    }

    #[test]
    fn domain_error_surfaces() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"::tcl::mathfunc::sqrt -1"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"domain error: argument not in valid range"
            );
        });
    }
}
