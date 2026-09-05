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
//! Every row here goes through `expr`, which exists only when the numeric
//! tower does: a build whose libtommath cross-compile was unavailable
//! registers no `expr` at all, so the whole file is gated on the tower
//! rather than asserting the semantics of a command that configuration does
//! not have. CI always builds with libtommath.
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

#![cfg(have_tommath)]

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

// ---------------------------------------------------------------------------
// #1382 — `entier`/`int`/`wide`/`round` on a float outside the wide range.
// The shared arms now widen through the tower's bignum rung, so the runtime
// answers what tclsh answers instead of raising `ARITH DOMAIN`.
// ---------------------------------------------------------------------------

/// tclsh 8.6.16/9.0.4: TIP 237 makes `entier()` unbounded, so `entier(1e300)`
/// is the exact 301-digit value of the double `1e300` — not `10^300`, and not
/// a domain error (which is what the runtime used to raise).
#[test]
fn entier_of_a_beyond_wide_float_is_the_exact_integer() {
    expr_is("entier(1e300)", E1E300);
    expr_is("entier(-1e300)", &format!("-{E1E300}"));
    expr_is("entier(1e20)", "100000000000000000000");
    expr_is("entier(-1e20)", "-100000000000000000000");
    expr_is("entier(1.9)", "1");
    expr_is("entier(-1.9)", "-1");
    expr_is("entier(2**64+1)", "18446744073709551617");
}

/// `round()` is unbounded in both releases too (tclsh: `round(1e300)` is the
/// same 301-digit integer, `round(1e20)` the same 21-digit one), and it is
/// half away from zero on the *exact* operand rather than `floor(d + 0.5)`,
/// which rounds twice: tclsh 8.6.16/9.0.4 answer `round(0.49999999999999994)`
/// with `0` while `floor(0.49999999999999994 + 0.5)` is `1.0`.
#[test]
fn round_of_a_beyond_wide_float_is_the_exact_integer() {
    expr_is("round(1e300)", E1E300);
    expr_is("round(-1e300)", &format!("-{E1E300}"));
    expr_is("round(1e20)", "100000000000000000000");
    expr_is("round(0.5)", "1");
    expr_is("round(1.5)", "2");
    expr_is("round(2.5)", "3");
    expr_is("round(-2.5)", "-3");
    expr_is("round(0.49999999999999994)", "0");
    expr_is("round(-0.49999999999999994)", "0");
    expr_is("round(4503599627370497.0)", "4503599627370497");
    expr_is("round(-4503599627370497.0)", "-4503599627370497");
    expr_is("round(4503599627370497.5)", "4503599627370498");
    expr_is("round(2251799813685248.5)", "2251799813685249");
}

/// `wide()` truncates then takes the low 64 bits, in every release
/// (tclsh 8.6/9.0: `wide(1e20)` is `7766279631452241920`, `wide(1e300)` is
/// `0` because 10^300's exact double value is divisible by 2^64).
#[test]
fn wide_windows_the_exact_truncation() {
    expr_is("wide(1e20)", "7766279631452241920");
    expr_is("wide(-1e20)", "-7766279631452241920");
    expr_is("wide(1e19)", "-8446744073709551616");
    expr_is("wide(1e300)", "0");
    expr_is("wide(2**64+1)", "1");
}

/// `isqrt()` on a beyond-wide float keeps its exact root as well (tclsh:
/// `isqrt(1e300)` is a 151-digit integer).
#[test]
fn isqrt_of_a_beyond_wide_float_is_exact() {
    expr_is("isqrt(1e300)", ISQRT_1E300);
}

/// `int()` is the one release-split conversion. tclsh9.0.4 binds it to the
/// unbounded `ExprIntFunc`; tclsh8.6.16 keeps the 64-bit window. The runtime
/// selects on its own `runtime_version`, through the shared `IntWidth` axis,
/// so the two engines cannot disagree about which release windows.
#[test]
fn int_follows_the_interps_release() {
    // Default runtime release is 9.0.
    expr_is("int(1e20)", "100000000000000000000");
    expr_is("int(-1e20)", "-100000000000000000000");
    expr_is("int(1e300)", E1E300);
    expr_is("int(2**64+1)", "18446744073709551617");

    let mut interp = Interp::new();
    interp.set_runtime_version(tcl_dialect::TclVersion::V8_6);
    for (body, want) in [
        ("int(1e20)", "7766279631452241920"),
        ("int(-1e20)", "-7766279631452241920"),
        ("int(1e300)", "0"),
        ("int(2**64+1)", "1"),
        // `wide`/`entier` are release-invariant.
        ("wide(1e20)", "7766279631452241920"),
        ("entier(1e20)", "100000000000000000000"),
    ] {
        assert_eq!(
            interp.eval_str(format!("expr {{{body}}}").as_bytes()),
            Code::Ok,
            "expr {{{body}}} at 8.6"
        );
        assert_eq!(
            String::from_utf8_lossy(&interp.result_bytes()),
            want,
            "expr {{{body}}} at 8.6"
        );
    }
}

/// The integer fast path keeps the operand's own object, so an integer's
/// string rep survives the conversion (tclsh 9.0.4:
/// `::tcl::mathfunc::entier 0x10` is `0x10`, not `16`). The release-split
/// `int` still windows a beyond-wide operand at 8.6, which is why the fast
/// path had to learn the axis rather than always returning the operand.
#[test]
fn an_integer_operand_keeps_its_own_string_rep() {
    let (code, result, _) = run("::tcl::mathfunc::entier 0x10");
    assert_eq!(code, Code::Ok);
    assert_eq!(result, "0x10");
    let (code, result, _) = run("::tcl::mathfunc::int 0x10");
    assert_eq!(code, Code::Ok);
    assert_eq!(result, "0x10");
}

/// The exact integer value of the double `1e300` (301 digits) — tclsh
/// `expr {entier(1e300)}`.
const E1E300: &str = "1000000000000000052504760255204420248704468581108159154915854115511802457988908195786371375080447864043704443832883878176942523235360430575644792184786706982848387200926575803737830233794788090059368953234970799945081119038967640880074652742780142494579258788820056842838115669472196386865459400540160";

/// tclsh `expr {isqrt(1e300)}` (151 digits).
const ISQRT_1E300: &str = "1000000000000000026252380127602209779758503108492371458359424883684651414333812736380124287612629691547944630047071980611862607399628869272326975124240";

// ---------------------------------------------------------------------------
// #1432 — `rand`/`srand`. The generator (Park-Miller step, seed nudge, and C's
// reciprocal-multiply scaling) is now `tcl_syntax::expr::rand`; only the seed
// storage and the nondeterministic first-seed policy stay per engine.
// ---------------------------------------------------------------------------

/// `srand(251)` is the smallest seed in the dense family where C's
/// `seed * (1.0/RAND_IM)` and a true `seed / RAND_IM` differ by one ulp, which
/// Tcl's shortest-round-trip formatting makes visible. tclsh 8.6.16/9.0.4
/// print the reciprocal-multiply value.
#[test]
fn srand_reproduces_cs_reciprocal_multiply_scaling() {
    expr_is("srand(251)", "0.001964418684115828");
    expr_is("srand(1)", "7.826369259425611e-6");
    expr_is("srand(0)", "0.24257829889775176");
    // `-1 & 0x7FFFFFFF` is `IM`, one of the generator's two fixed points, so
    // both seeds land on the same nudged stream.
    expr_is("srand(2147483647)", "0.7574217011022483");
    expr_is("srand(-1)", "0.7574217011022483");
    // C reads the operand's low 64 bits (`TclGetWideBitsFromObj`).
    expr_is("srand(2**64+7)", "5.4784584815979276e-5");
}

/// A whole seeded stream, not just its first draw: the 145th draw after
/// `srand(1)` is the first index at which the two scalings disagree, so this
/// row is the drift gate for the family. tclsh 8.6.16/9.0.4:
/// `0.9833050970841688`.
#[test]
fn the_145th_draw_after_srand_1_matches_the_oracle() {
    let (code, result, _) = run("expr {srand(1)}\n\
         set v {}\n\
         for {set i 2} {$i <= 145} {incr i} { set v [expr {rand()}] }\n\
         set v");
    assert_eq!(code, Code::Ok);
    assert_eq!(result, "0.9833050970841688");
}

/// C reads `srand`'s operand with `TclGetWideBitsFromObj`, which takes an
/// integer of any width and **refuses** a double rather than truncating it
/// (tclsh8.6.16: `expected integer but got "1.5"`, `-errorcode TCL VALUE
/// INTEGER`; tclsh9.0.4 raises with an empty message because C passes a NULL
/// interp there). Both engines use 8.6's wording so they agree with each
/// other; 9.0's empty-message quirk is left to #1581.
#[test]
fn srand_refuses_a_non_integer_operand() {
    expr_err(
        "srand(1.5)",
        "expected integer but got \"1.5\"",
        "TCL VALUE INTEGER",
    );
    expr_err(
        "srand(\"abc\")",
        "expected integer but got \"abc\"",
        "TCL VALUE NUMBER",
    );
}

// ---------------------------------------------------------------------------
// #1581 — the expr/mathfunc error taxonomy: IOVERFLOW / NaN codes, the
// boolean-context codes, and the release axis for `IllegalExprOperandType`.
// ---------------------------------------------------------------------------

const IOVERFLOW: &str = "integer value too large to represent";
const IOVERFLOW_CODE: &str = "ARITH IOVERFLOW {integer value too large to represent}";
const NAN_MSG: &str = "floating point value is Not a Number";
const NAN_CODE: &str = "TCL VALUE DOUBLE NAN";

/// tclsh 8.6.16/9.0.4: an infinity reaching an integer conversion is
/// `ARITH IOVERFLOW`, not the generic `ARITH DOMAIN` the runtime used to
/// report.
#[test]
fn an_infinity_in_an_integer_conversion_is_ioverflow() {
    for body in [
        "entier(Inf)",
        "int(Inf)",
        "wide(Inf)",
        "round(Inf)",
        "entier(-Inf)",
        "isqrt(Inf)",
    ] {
        expr_err(body, IOVERFLOW, IOVERFLOW_CODE);
    }
    // FP guard: the conversions that *can* answer with an infinity still do.
    expr_is("abs(-Inf)", "Inf");
    expr_is("double(Inf)", "Inf");
    expr_is("floor(Inf)", "Inf");
}

/// tclsh 8.6.16/9.0.4: a NaN operand is `TCL VALUE DOUBLE NAN`.
#[test]
fn a_nan_operand_carries_the_double_nan_code() {
    for body in [
        "entier(NaN)",
        "int(NaN)",
        "round(NaN)",
        "abs(NaN)",
        "double(NaN)",
        "ceil(NaN)",
        "isqrt(NaN)",
    ] {
        expr_err(body, NAN_MSG, NAN_CODE);
    }
}

/// A genuine domain error keeps its own class, and `isqrt` of a negative
/// keeps C's specialised message with the ordinary domain code (tclsh
/// verified).
#[test]
fn domain_errors_keep_their_own_class() {
    expr_err(
        "sqrt(-1)",
        "domain error: argument not in valid range",
        "ARITH DOMAIN {domain error: argument not in valid range}",
    );
    expr_err(
        "isqrt(-1)",
        "square root of negative argument",
        "ARITH DOMAIN {domain error: argument not in valid range}",
    );
}

/// Boolean context: tclsh 8.6.16/9.0.4 stamp `TCL VALUE NUMBER` on
/// `expected boolean value but got "…"` and `TCL VALUE DOUBLE NAN` on a NaN
/// there. Both were `NONE` before #1581.
#[test]
fn boolean_context_errors_carry_their_codes() {
    for body in [
        "\"abc\" ? 1 : 2",
        "\"abc\" && 1",
        "1 && \"abc\"",
        "\"abc\" || 0",
    ] {
        expr_err(
            body,
            "expected boolean value but got \"abc\"",
            "TCL VALUE NUMBER",
        );
    }
    expr_err("\"NaN\" ? 1 : 2", NAN_MSG, NAN_CODE);
}

/// `IllegalExprOperandType` at Tcl 9.0: the value and the side are named, the
/// multi-element list has its own branch, and every one carries
/// `ARITH DOMAIN <description>`.
#[test]
fn operand_type_errors_use_the_9_0_wording_and_code() {
    expr_err(
        "!\"abc\"",
        "cannot use non-numeric string \"abc\" as operand of \"!\"",
        "ARITH DOMAIN {non-numeric string}",
    );
    expr_err(
        "~1.5",
        "cannot use floating-point value \"1.5\" as operand of \"~\"",
        "ARITH DOMAIN {floating-point value}",
    );
    expr_err(
        "\"abc\" + 1",
        "cannot use non-numeric string \"abc\" as left operand of \"+\"",
        "ARITH DOMAIN {non-numeric string}",
    );
    expr_err(
        "1 + \"abc\"",
        "cannot use non-numeric string \"abc\" as right operand of \"+\"",
        "ARITH DOMAIN {non-numeric string}",
    );
    expr_err(
        "2 & 1.5",
        "cannot use floating-point value \"1.5\" as right operand of \"&\"",
        "ARITH DOMAIN {floating-point value}",
    );
    expr_err(
        "\"a b\" + 1",
        "cannot use a list as left operand of \"+\"",
        "ARITH DOMAIN list",
    );
    // (`expr {NaN + 1}` is a separate, pre-existing gap: the runtime's tower
    // accepts a *typed* double NaN as an arithmetic operand and answers `NaN`
    // where tclsh raises the operand-type error. That is an operand-acceptance
    // bug, not a taxonomy one, and is left outside this lane.)
}

/// The same errors at Tcl 8.6: no value, no side, and no list branch — the
/// release axis #1581 asks for. Measured on tclsh8.6.16.
#[test]
fn operand_type_errors_use_the_8_6_wording_at_8_6() {
    let mut interp = Interp::new();
    interp.set_runtime_version(tcl_dialect::TclVersion::V8_6);
    for (body, want) in [
        (
            "!\"abc\"",
            "can't use non-numeric string as operand of \"!\"",
        ),
        ("~1.5", "can't use floating-point value as operand of \"~\""),
        (
            "\"abc\" + 1",
            "can't use non-numeric string as operand of \"+\"",
        ),
        (
            "1 + \"abc\"",
            "can't use non-numeric string as operand of \"+\"",
        ),
        (
            "2 & 1.5",
            "can't use floating-point value as operand of \"&\"",
        ),
        // 8.6 has no list branch at all.
        (
            "\"a b\" + 1",
            "can't use non-numeric string as operand of \"+\"",
        ),
    ] {
        assert_eq!(
            interp.eval_str(format!("expr {{{body}}}").as_bytes()),
            Code::Error,
            "expr {{{body}}} at 8.6"
        );
        assert_eq!(
            String::from_utf8_lossy(&interp.result_bytes()),
            want,
            "expr {{{body}}} at 8.6"
        );
    }
}

// ---------------------------------------------------------------------------
// #1425 — boolean context: the shared `tcl_syntax::boolean` words, by unique
// prefix, in every context the runtime evaluates.
// ---------------------------------------------------------------------------

/// tclsh 8.6.16/9.0.4: every unique prefix of a boolean word is accepted in
/// `expr`'s `?:`, in `if`, in `while`, and in `dict filter … script` — the
/// issue's own repro (`set x tru; expr {$x ? "T" : "F"}` is `T`).
#[test]
fn boolean_prefixes_are_accepted_in_every_boolean_context() {
    let (code, result, _) = run("set x tru; expr {$x ? \"T\" : \"F\"}");
    assert_eq!((code, result.as_str()), (Code::Ok, "T"));
    for (word, want) in [
        ("t", "yes"),
        ("tr", "yes"),
        ("tru", "yes"),
        ("y", "yes"),
        ("ye", "yes"),
        ("on", "yes"),
        ("f", "no"),
        ("fa", "no"),
        ("fal", "no"),
        ("n", "no"),
        ("of", "no"),
        ("off", "no"),
    ] {
        let (code, result, _) = run(&format!("set x {word}; if {{$x}} {{list yes}} {{list no}}"));
        assert_eq!((code, result.as_str()), (Code::Ok, want), "if {{{word}}}");
        let (code, result, _) = run(&format!(
            "set x {word}; set n 0; while {{$x}} {{incr n; set x 0}}; set n"
        ));
        let laps = if want == "yes" { "1" } else { "0" };
        assert_eq!(
            (code, result.as_str()),
            (Code::Ok, laps),
            "while {{{word}}}"
        );
    }
    let (code, result, _) = run("set x tru; dict filter {a 1 b 2} script {k v} {set x}");
    assert_eq!((code, result.as_str()), (Code::Ok, "a 1 b 2"));
}

/// tclsh 8.6.16/9.0.4: `o` is shared by `on` and `off`, so it is refused —
/// with the same message and `TCL VALUE NUMBER` in every boolean context —
/// and a NaN is `TCL VALUE DOUBLE NAN`. A multi-element list is described as
/// `a list` at 9.0 (8.6.16 quotes it, `"a b"`; that wording axis is #1581's,
/// which `describe_bad_value` does not yet carry, so only the 9.0 row is
/// pinned).
#[test]
fn the_ambiguous_prefix_and_nan_are_refused_in_every_boolean_context() {
    for script in [
        "set x o; if {$x} {}",
        "set x o; while {$x} {}",
        "set x o; expr {$x ? 1 : 0}",
        "set x o; dict filter {a 1} script {k v} {set x}",
    ] {
        let (code, result, error_code) = run(script);
        assert_eq!(code, Code::Error, "{script}");
        assert_eq!(result, "expected boolean value but got \"o\"", "{script}");
        assert_eq!(error_code, "TCL VALUE NUMBER", "{script}");
    }
    let (code, result, error_code) = run("set x NaN; if {$x} {}");
    assert_eq!(
        (code, result.as_str(), error_code.as_str()),
        (
            Code::Error,
            "floating point value is Not a Number",
            "TCL VALUE DOUBLE NAN"
        )
    );
    let (code, result, error_code) = run("set x {a b}; if {$x} {}");
    assert_eq!(
        (code, result.as_str(), error_code.as_str()),
        (
            Code::Error,
            "expected boolean value but got a list",
            "TCL VALUE NUMBER"
        )
    );
}

/// A boolean word is coerced only where a boolean is actually needed: as a
/// whole expression it keeps its own spelling (tclsh: `expr {tru}` is `tru`,
/// not `1`), while `!` and `&&` — which do want a boolean — read it through
/// the same acceptor as `if`.
#[test]
fn a_boolean_word_keeps_its_spelling_outside_a_boolean_context() {
    expr_is("tru", "tru");
    expr_is("yes", "yes");
    expr_is("!of", "1");
    expr_is("tru && 1", "1");
}
