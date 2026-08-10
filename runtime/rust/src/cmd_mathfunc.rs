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

/// Every math function name — registered as `::tcl::mathfunc::<name>`. Most
/// forward to the shared [`dispatch`]; `rand`/`srand` are handled inline
/// (interp state).
///
/// Derived from `tcl_syntax::expr::mathfunc::all()` (issue #983's
/// unification) rather than a hand-typed list — that list had gone stale,
/// missing the entire TIP 745 (Tcl 9.1) C99 batch even though `dispatch()`
/// (the function this loop wires every one of these names up to) already
/// implemented all of them: `::tcl::mathfunc::gamma` and its 20 siblings
/// were simply never registered as commands, an "invalid command name"
/// error rather than a working call.
fn mathfunc_names() -> Vec<&'static str> {
    tcl_syntax::expr::mathfunc::all()
        .into_iter()
        .map(|spec| spec.name)
        .collect()
}

/// Register `::tcl::mathfunc::*`.
pub fn install(interp: &mut Interp) {
    for name in mathfunc_names() {
        let mut full = b"::tcl::mathfunc::".to_vec();
        full.extend_from_slice(name.as_bytes());
        interp.register_builtin(&full, mathfunc);
    }
}

/// The shared implementation behind every `::tcl::mathfunc::NAME`. The function
/// is `argv[0]`'s simple tail (so one builtin serves all names); operands are
/// read off the tower as [`Num`] and dispatched.
pub(crate) fn mathfunc(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let name0 = obj_bytes(argv[0]);
    let tail = qualifier_segments(&name0)
        .last()
        .copied()
        .unwrap_or(&name0[..]);
    let Ok(fname) = core::str::from_utf8(tail) else {
        return interp.set_error(b"unknown math function");
    };

    // The interpreter registers one dispatch hook for the complete known
    // catalogue, but its public builtin surface is profile-selected by the
    // CommandRegistry. A user proc or alias replacing this entry never reaches
    // this handler, so version-gating the builtin preserves Tcl's open,
    // overridable `tcl::mathfunc` namespace.
    if tcl_registry::expr_surface::RuntimeExprSurface::for_tcl_version(interp.runtime_version())
        .builtin_math_function(fname)
        .is_none()
    {
        let mut message = b"invalid command name \"".to_vec();
        message.extend_from_slice(&name0);
        message.push(b'\"');
        let mut error_code = b"TCL LOOKUP COMMAND ".to_vec();
        error_code.extend_from_slice(&name0);
        return interp.error_with_code(&message, &error_code);
    }

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
/// A function's `(min, max)` argument count — derived from
/// `tcl_syntax::expr::mathfunc::spec` (issue #983's unification) rather than
/// a hand-typed match, which — like [`mathfunc_names`] — had gone stale for
/// the TIP 745 batch: several of those functions are 2- or 3-argument
/// (`copysign`, `dim`, `ldexp`, `nextafter`, `remainder` take 2; `fma` takes
/// 3), and the old fallback arm (`_ => (1, Some(1))`) would have wrongly
/// rejected a correct call to any of them once they were registered.
/// Unknown names fall back to `(1, Some(1))` too — unreachable for any name
/// [`mathfunc_names`] actually registers, since both read the same table.
fn arity(name: &str) -> (usize, Option<usize>) {
    let Some(spec) = tcl_syntax::expr::mathfunc::spec(name) else {
        return (1, Some(1));
    };
    (usize::from(spec.arity.min), spec.arity.max.map(usize::from))
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
    fn tcl84_expr_uses_the_fixed_math_table_not_a_command_wrapper() {
        leak_free(|i| {
            i.set_runtime_version(tcl_dialect::TclVersion::V8_4);

            // TP: the registry still supplies the 8.4 fixed builtin.
            assert_eq!(i.eval_str(b"expr {sqrt(4)}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"2.0");
            // FN: names outside that table do not take the later command-miss
            // path. C Tcl 8.4 reports this exact fixed-table diagnostic.
            assert_eq!(i.eval_str(b"expr {user_supplied(1)}"), Code::Error);
            assert_eq!(i.result_bytes(), b"unknown math function \"user_supplied\"");

            // A script can create an ordinary command with the same spelling,
            // but it changes neither the fixed builtin nor `expr`'s target.
            assert_eq!(
                i.eval_str(b"proc ::tcl::mathfunc::sqrt {x} { return 99 }"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"::tcl::mathfunc::sqrt 4"), Code::Ok);
            assert_eq!(i.result_bytes(), b"99");
            assert_eq!(i.eval_str(b"expr {sqrt(4)}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"2.0");

            // FP guard: Tcl 8.5's command-table mechanism makes that same
            // override active for `expr`.
            i.set_runtime_version(tcl_dialect::TclVersion::V8_5);
            assert_eq!(i.eval_str(b"expr {sqrt(4)}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"99");
        });
    }

    #[test]
    fn builtin_mathfunc_surface_tracks_the_registry_selected_release() {
        leak_free(|i| {
            i.set_runtime_version(tcl_dialect::TclVersion::V8_6);

            // FN: TIP 461's string-ordering operators are Tcl 9 grammar, not
            // a 8.x runtime comparison. The registry supplies both the floor
            // and C Tcl's BAREWORD error classification.
            assert_eq!(i.eval_str(b"expr {{a} lt {b}}"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"invalid bareword \"lt\"\nin expression \"{a} lt {b}\";\nshould be \"$lt\" or \"{lt}\" or \"lt(...)\" or ..."
            );
            assert_eq!(
                i.var_get(b"::errorCode").map(crate::interp::obj_bytes),
                Some(b"TCL PARSE EXPR BAREWORD".to_vec())
            );

            // FN: the 9.0 builtin is no longer supplied by the 8.6 surface.
            assert_eq!(i.eval_str(b"expr {isfinite(1.0)}"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"invalid command name \"tcl::mathfunc::isfinite\""
            );
            // The outermost eval publishes the error state to the Tcl global
            // before resetting its transient accumulator.
            assert_eq!(
                i.var_get(b"::errorCode").map(crate::interp::obj_bytes),
                Some(b"TCL LOOKUP COMMAND tcl::mathfunc::isfinite".to_vec())
            );

            // TP: older builtins retain their registry-backed handler.
            assert_eq!(i.eval_str(b"expr {sqrt(4)}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"2.0");

            // TP/FP: at the Tcl 9.0 boundary the registry admits both the
            // operator and the classification builtin; its normal false value
            // remains distinguishable from a version rejection.
            i.set_runtime_version(tcl_dialect::TclVersion::V9_0);
            assert_eq!(i.eval_str(b"expr {{a} lt {b}}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
            assert_eq!(i.eval_str(b"expr {isfinite(1.0 / 0.0)}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"0");

            // FP: a user definition in the open namespace remains callable;
            // the registry gates the builtin, not command indirection.
            i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            assert_eq!(
                i.eval_str(b"proc ::tcl::mathfunc::isfinite {x} { return custom }"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"expr {isfinite(1.0)}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"custom");
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

    /// The TIP 745 C99 batch is a Tcl 9.1 surface. The handler and its
    /// arity facts come from the shared registry-backed table, but the
    /// runtime must not expose the names while emulating Tcl 9.0.
    #[test]
    fn tip745_mathfunc_commands_follow_the_runtime_release_and_arity() {
        leak_free(|i| {
            // FN: Tcl 9.0.4 rejects the 9.1 command and `expr` spelling.
            // The lookup name is relative on the expression path, as in C Tcl.
            assert_eq!(i.eval_str(b"::tcl::mathfunc::gamma 5"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"invalid command name \"::tcl::mathfunc::gamma\""
            );
            assert_eq!(i.eval_str(b"expr {gamma(5)}"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"invalid command name \"tcl::mathfunc::gamma\""
            );

            i.set_runtime_version(tcl_dialect::TclVersion::V9_1);
            // TP: both unary and multi-argument 9.1 functions are real
            // commands, with their arities derived from the fact table.
            assert_eq!(i.eval_str(b"::tcl::mathfunc::gamma 5"), Code::Ok);
            assert_eq!(i.result_bytes(), b"24.0");
            assert_eq!(i.eval_str(b"::tcl::mathfunc::cbrt 27"), Code::Ok);
            assert_eq!(i.result_bytes(), b"3.0");
            assert_eq!(i.eval_str(b"::tcl::mathfunc::copysign 3 -1"), Code::Ok);
            assert_eq!(i.result_bytes(), b"-3.0");
            assert_eq!(i.eval_str(b"::tcl::mathfunc::fma 2 3 4"), Code::Ok);
            assert_eq!(i.result_bytes(), b"10.0");
            // Wrong arity on a 2-arg TIP 745 function is a real arity error,
            // not a silent `(1, Some(1))` fallback mis-check.
            assert_eq!(i.eval_str(b"::tcl::mathfunc::copysign 3"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"not enough arguments for math function \"copysign\""
            );
            assert_eq!(i.eval_str(b"::tcl::mathfunc::copysign 3 -1 2"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"too many arguments for math function \"copysign\""
            );
        });
    }
}
