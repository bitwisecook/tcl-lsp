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

//! Constant-folder integration tests over parsed `[expr]` bodies.
//!
//! These exercise the compiler's **compile-time constant folder** — the
//! [`eval_tcl_expr`] tree-walk over a parsed `[expr]` body
//! ([`tcl_compiler::expr_parser::parse_expr`]). Because that folder drives
//! O101 expr-folding, every disagreement with real C Tcl is a **miscompile**,
//! so each expected value below is pinned to `tclsh8.6`/`tclsh9.0`
//! (`scripts/dev/tclsh_check.sh 'expr {<expr>}'`) and the value is cited
//! inline next to the assertion. This mirrors the canonical in-crate templates
//! `const_fold_integers_match_tclsh` /
//! `const_fold_floats_and_logicals_match_tclsh` /
//! `const_fold_integer_division_floors_toward_negative_infinity` in
//! `src/tcl_expr_eval.rs`, and reuses the same `eval_str` / `eval_str_env` /
//! `eval_irules` / `eval_irules_env` helper shape.
//!
//! Notes on deliberate folder behaviours worth calling out:
//!
//! * **`i64`-width folder.** The const-folder's value is `i64`/`f64`; a magnitude
//!   past a wide is a Tcl `Big`, which "can't fold" → `None` (documented in the
//!   module: "a value past a wide … callers fall through to the runtime form").
//!   So `10 ** 19` (= 10000000000000000000 in tclsh, but > `i64::MAX`) folds to
//!   `None` here and the runtime form takes over.
//!   `10 ** 18` (≤ `i64::MAX`) still folds. See
//!   [`exponentiation_big_results_past_wide_decline_to_fold`].
//! * **Dialect-aware leading zero.** The 8.x-vs-9.0 octal rule is resolved via
//!   the explicit [`eval_tcl_expr_in_dialect`] entry point, and applies to
//!   arithmetic/unary/math-function operands as well as comparisons. A bare
//!   leading-zero operand under an *unknown* dialect folds only when the
//!   octal and decimal readings agree (`07` is `7` either way) and declines
//!   (`None`) when they disagree (`010`: octal 8 vs decimal 10) rather than
//!   guessing.
//! * **`format_tcl_value`** takes a `TclValue`, so free-float cases are written
//!   `format_tcl_value(&TclValue::Float(3.14))`.
//!
//! No test is `#[ignore]`d; every const-fold result here matches tclsh.

// Decimal literals such as `3.14` / `6.28` are the values Tcl yields for those
// expression *strings*, not the math constants PI / TAU — `approx_constant`
// would mis-flag them, so it is disabled for this expression-folding port.
#![allow(clippy::approx_constant)]

use tcl_compiler::expr_parser::parse_expr;
use tcl_compiler::tcl_expr_eval::{eval_tcl_expr, eval_tcl_expr_in_dialect};
use tcl_compiler::{Env, EnvValue, TclValue, format_tcl_value};

// -- helpers (mirroring `src/tcl_expr_eval.rs`'s `#[cfg(test)] mod tests`) --

/// Fold a default-dialect expression over an empty environment.
fn eval_str(expr: &str) -> Option<TclValue> {
    let env = Env::new();
    eval_tcl_expr(&parse_expr(expr, None), &env)
}

/// Fold a default-dialect expression over a caller-supplied environment.
fn eval_str_env(expr: &str, env: &Env) -> Option<TclValue> {
    eval_tcl_expr(&parse_expr(expr, None), env)
}

/// Fold under the iRules dialect (enables `contains`/`starts_with`/`ends_with`/
/// `equals`/`matches_glob`/`matches_regex`/`in`/`ni`/`and`/`or`/`not`). Must
/// use the dialect-threading evaluator, not the bare [`eval_tcl_expr`] — the
/// word operators parse under any dialect gate, but only actually *fold*
/// when the evaluator's own iRules dialect flag is set (issue #983/#985's
/// defence-in-depth fix), which only [`eval_tcl_expr_in_dialect`] does.
fn eval_irules(expr: &str) -> Option<TclValue> {
    let env = Env::new();
    eval_tcl_expr_in_dialect(
        &parse_expr(expr, Some("f5-irules")),
        &env,
        tcl_dialect::DialectProfile::irules(),
    )
}

/// iRules-dialect fold over a caller-supplied environment.
fn eval_irules_env(expr: &str, env: &Env) -> Option<TclValue> {
    eval_tcl_expr_in_dialect(
        &parse_expr(expr, Some("f5-irules")),
        env,
        tcl_dialect::DialectProfile::irules(),
    )
}

/// Fold under a named dialect (resolves the 8.x-octal vs 9.0-decimal
/// leading-zero rule for comparison operands).
fn eval_dialect(expr: &str, dialect: &str) -> Option<TclValue> {
    eval_tcl_expr_in_dialect(
        &parse_expr(expr, None),
        &Env::new(),
        tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
    )
}

/// Convenience constructors keeping assertions terse.
fn int(i: i64) -> TclValue {
    TclValue::Int(i)
}
fn float(f: f64) -> TclValue {
    TclValue::Float(f)
}

/// Bind a string variable (the only `EnvValue` the iRules string-op cases and
/// the auto-coercion tests need).
fn env_str(name: &str, value: &str) -> Env {
    let mut env = Env::new();
    env.insert(name.to_owned(), EnvValue::Str(value.to_owned()));
    env
}

// =========================================================================
// Literals
// =========================================================================

#[test]
fn literal_integers_in_all_radixes() {
    // tclsh: 42=42  -7=-7  0xFF=255  0o17=15  0b1010=10  0x20=32  0o15=13
    assert_eq!(eval_str("42"), Some(int(42)));
    assert_eq!(eval_str("-7"), Some(int(-7)));
    assert_eq!(eval_str("0xFF"), Some(int(255)));
    assert_eq!(eval_str("0o17"), Some(int(15)));
    assert_eq!(eval_str("0b1010"), Some(int(10)));
    assert_eq!(eval_str("0x20"), Some(int(32)));
    assert_eq!(eval_str("0o15"), Some(int(13)));
    assert_eq!(eval_str("-0xff"), Some(int(-255)));
}

#[test]
fn literal_floats_and_scientific() {
    // tclsh: 3.14=3.14  1e10=10000000000.0  2.0e2=200.0  2.0e15=2000000000000000.0
    assert_eq!(eval_str("3.14"), Some(float(3.14)));
    assert_eq!(eval_str("1e10"), Some(float(1e10)));
    assert_eq!(eval_str("2.0e2"), Some(float(200.0)));
    assert_eq!(eval_str("2.0e15"), Some(float(2.0e15)));
}

#[test]
fn literal_boolean_words_fold_to_0_or_1() {
    // tclsh boolean context: true/yes/on=1, false/no/off=0.
    assert_eq!(eval_str("true"), Some(int(1)));
    assert_eq!(eval_str("false"), Some(int(0)));
    assert_eq!(eval_str("yes"), Some(int(1)));
    assert_eq!(eval_str("no"), Some(int(0)));
    assert_eq!(eval_str("on"), Some(int(1)));
    assert_eq!(eval_str("off"), Some(int(0)));
}

// =========================================================================
// Arithmetic
// =========================================================================

#[test]
fn arithmetic_integer_basic() {
    // tclsh: 1+2=3  10-3=7  6*7=42  0xff+0x3=258  4+-2=2  -0xf2 - -0x3=-239
    //        0xff*0x3=765  4*-2=-8
    assert_eq!(eval_str("1 + 2"), Some(int(3)));
    assert_eq!(eval_str("10 - 3"), Some(int(7)));
    assert_eq!(eval_str("6 * 7"), Some(int(42)));
    assert_eq!(eval_str("0xff + 0x3"), Some(int(258)));
    assert_eq!(eval_str("4 + -2"), Some(int(2)));
    assert_eq!(eval_str("-0xf2 - -0x3"), Some(int(-239)));
    assert_eq!(eval_str("0xff * 0x3"), Some(int(765)));
    assert_eq!(eval_str("4 * -2"), Some(int(-8)));
}

#[test]
fn arithmetic_float_promotion() {
    // tclsh: 1.0+2=3.0  2+2.5=4.5  2.5+2=4.5  2-2.5=-0.5
    assert_eq!(eval_str("1.0 + 2"), Some(float(3.0)));
    assert_eq!(eval_str("2 + 2.5"), Some(float(4.5)));
    assert_eq!(eval_str("2.5 + 2"), Some(float(4.5)));
    assert_eq!(eval_str("2 - 2.5"), Some(float(-0.5)));
}

#[test]
fn integer_division_floors_toward_negative_infinity() {
    // tclsh: 7/2=3  -7/2=-4 (floor, not truncate)  7/-2=-4  36/12=3
    //        27/4=6  -1/2=-1
    assert_eq!(eval_str("7 / 2"), Some(int(3)));
    assert_eq!(eval_str("-7 / 2"), Some(int(-4)));
    assert_eq!(eval_str("7 / -2"), Some(int(-4)));
    assert_eq!(eval_str("36 / 12"), Some(int(3)));
    assert_eq!(eval_str("27 / 4"), Some(int(6)));
    assert_eq!(eval_str("-1 / 2"), Some(int(-1)));
}

#[test]
fn float_division() {
    // tclsh: 7.0/2=3.5  36.0/12.0=3.0  27/4.0=6.75  2/2.5=0.8
    assert_eq!(eval_str("7.0 / 2"), Some(float(3.5)));
    assert_eq!(eval_str("36.0 / 12.0"), Some(float(3.0)));
    assert_eq!(eval_str("27 / 4.0"), Some(float(6.75)));
    assert_eq!(eval_str("2 / 2.5"), Some(float(0.8)));
}

#[test]
fn modulo_sign_follows_divisor() {
    // tclsh: 7%3=1  -7%3=2  7%-3=-2  0xff%2=1  7891%0o123=6  -0xf2%-0x3=-2
    //        36%5=1  -36%5=4  36%-5=-4  -36%-5=-1
    assert_eq!(eval_str("7 % 3"), Some(int(1)));
    assert_eq!(eval_str("-7 % 3"), Some(int(2)));
    assert_eq!(eval_str("7 % -3"), Some(int(-2)));
    assert_eq!(eval_str("0xff % 2"), Some(int(1)));
    assert_eq!(eval_str("7891 % 0o123"), Some(int(6)));
    assert_eq!(eval_str("-0xf2 % -0x3"), Some(int(-2)));
    assert_eq!(eval_str("36 % 5"), Some(int(1)));
    assert_eq!(eval_str("-36 % 5"), Some(int(4)));
    assert_eq!(eval_str("36 % -5"), Some(int(-4)));
    assert_eq!(eval_str("-36 % -5"), Some(int(-1)));
}

#[test]
fn division_and_modulo_by_zero_decline() {
    // tclsh: integer division/modulo by zero are "divide by zero" errors →
    // not foldable (None). A float division by zero with a non-zero
    // numerator is a real IEEE value (Inf), not an error — tclsh:
    // `expr {1.0/0}` -> Inf; only 0.0/0.0 itself is a domain error.
    assert_eq!(eval_str("1 / 0"), None);
    assert_eq!(eval_str("1 % 0"), None);
    assert_eq!(eval_str("1.0 / 0"), Some(float(f64::INFINITY)));
    assert_eq!(eval_str("0.0 / 0"), None);
}

// =========================================================================
// Exponentiation
// =========================================================================

#[test]
fn exponentiation_integer_and_float() {
    // tclsh: 2**10=1024  4**2=16  0xff**2=65025  18**07=612220032
    //        0xff**0x3=16581375  5**0=1  5**1=5  0**1=0  0**0=1
    //        2.0**3=8.0  +0o00123 is just octal 83 (unrelated, but pow checks)
    assert_eq!(eval_str("2 ** 10"), Some(int(1024)));
    assert_eq!(eval_str("4 ** 2"), Some(int(16)));
    assert_eq!(eval_str("0xff ** 2"), Some(int(65025)));
    // `07` is unambiguous under a dialect-blind fold: it reads as 7 under
    // both the octal rule (a valid octal digit) and the decimal rule, so
    // this still folds even without dialect info (unlike `010`, whose
    // octal/decimal readings disagree — see `dialect_blind_arithmetic_
    // declines_on_bare_leading_zero` in src/tcl_expr_eval.rs).
    assert_eq!(eval_str("18 ** 07"), Some(int(612_220_032)));
    assert_eq!(eval_str("0xff ** 0x3"), Some(int(16_581_375)));
    assert_eq!(eval_str("5 ** 0"), Some(int(1)));
    assert_eq!(eval_str("5 ** 1"), Some(int(5)));
    assert_eq!(eval_str("0 ** 1"), Some(int(0)));
    assert_eq!(eval_str("0 ** 0"), Some(int(1)));
    assert_eq!(eval_str("2.0 ** 3"), Some(float(8.0)));
}

#[test]
fn exponentiation_negative_exponent_integer_rules() {
    // tclsh integer `**`: |base|>1 with negative exp → 0; ±1 special-cased.
    //   2**-1=0  2**-2=0  (-1)**3=-1  (-1)**4=1  (-1)**-1=-1  1**-5=1
    //   1**1234567=1  0**-1 → "exponent ... negative" error → None
    assert_eq!(eval_str("2 ** -1"), Some(int(0)));
    assert_eq!(eval_str("2 ** -2"), Some(int(0)));
    assert_eq!(eval_str("(-1) ** 3"), Some(int(-1)));
    assert_eq!(eval_str("(-1) ** 4"), Some(int(1)));
    assert_eq!(eval_str("(-1) ** -1"), Some(int(-1)));
    assert_eq!(eval_str("1 ** -5"), Some(int(1)));
    assert_eq!(eval_str("1 ** 1234567"), Some(int(1)));
    assert_eq!(eval_str("0 ** -1"), None);
}

#[test]
fn exponentiation_float_negative_exponent() {
    // tclsh: 2.0**-1=0.5
    assert_eq!(eval_str("2.0 ** -1"), Some(float(0.5)));
}

#[test]
fn exponentiation_big_results_past_wide_stay_exact() {
    // The folder is bignum-exact (type-tracking P4): a result past a wide
    // promotes to the exact bignum tclsh computes — never wrapped, never
    // declined, never a rounded double.
    assert_eq!(eval_str("10 ** 17"), Some(int(100_000_000_000_000_000)));
    assert_eq!(eval_str("10 ** 18"), Some(int(1_000_000_000_000_000_000)));
    assert_eq!(
        eval_str("10 ** 19"),
        Some(TclValue::Big("10000000000000000000".parse().unwrap()))
    );
}

#[test]
fn exponentiation_huge_exponent_guard() {
    // Guard at exponent == 2**28 (268435456), matching C Tcl's INST_EXPON limit.
    //   tclsh proof: `expr {2 ** 268435455 == 2 ** 268435455}` → 1, so the
    //   one-below value is a *valid* (huge bignum) op. In the i64 folder the
    //   guarded case is None; the below-limit case is also a `Big` so it too
    //   declines here — but for a different reason (past a wide, not guarded).
    assert_eq!(eval_str("2 ** 268435456"), None); // at the limit → guarded
}

// =========================================================================
// Comparisons
// =========================================================================

#[test]
fn comparisons_return_int_0_or_1() {
    // tclsh: 3<5=1  5<3=0  3<=3=1  5>3=1  3>=5=0  5==5=1  5==6=0  5!=6=1
    assert_eq!(eval_str("3 < 5"), Some(int(1)));
    assert_eq!(eval_str("5 < 3"), Some(int(0)));
    assert_eq!(eval_str("3 <= 3"), Some(int(1)));
    assert_eq!(eval_str("5 > 3"), Some(int(1)));
    assert_eq!(eval_str("3 >= 5"), Some(int(0)));
    assert_eq!(eval_str("5 == 5"), Some(int(1)));
    assert_eq!(eval_str("5 == 6"), Some(int(0)));
    assert_eq!(eval_str("5 != 6"), Some(int(1)));
}

#[test]
fn comparisons_hex_and_shift_operands() {
    // tclsh: 0xff>=+0x3=1  -0xf2<0x3=1  6>>1>=3=1 (chained: (6>>1)=3, 3>=3)
    assert_eq!(eval_str("0xff >= +0x3"), Some(int(1)));
    assert_eq!(eval_str("-0xf2 < 0x3"), Some(int(1)));
    assert_eq!(eval_str("6 >> 1 >= 3"), Some(int(1)));
}

#[test]
fn comparisons_float() {
    // tclsh: 3.1>2.1=1  2.1>2.1=0  1.1<2.1=1  3.1>=2.2=1  2.345>=2.345=1
    //        1.1>=2.2=0  2.1<=2.1=1  2.2==2.2=1  3.2!=2.2=1
    assert_eq!(eval_str("3.1 > 2.1"), Some(int(1)));
    assert_eq!(eval_str("2.1 > 2.1"), Some(int(0)));
    assert_eq!(eval_str("1.1 < 2.1"), Some(int(1)));
    assert_eq!(eval_str("3.1 >= 2.2"), Some(int(1)));
    assert_eq!(eval_str("2.345 >= 2.345"), Some(int(1)));
    assert_eq!(eval_str("1.1 >= 2.2"), Some(int(0)));
    assert_eq!(eval_str("2.1 <= 2.1"), Some(int(1)));
    assert_eq!(eval_str("2.2 == 2.2"), Some(int(1)));
    assert_eq!(eval_str("3.2 != 2.2"), Some(int(1)));
}

#[test]
fn comparisons_mixed_int_float() {
    // tclsh: 5==5.0=1  2>2.5=0  2.5>2=1  2<2.5=1  2>=2.5=0  2<=2.5=1
    //        2==2.5=0  2!=2.5=1
    assert_eq!(eval_str("5 == 5.0"), Some(int(1)));
    assert_eq!(eval_str("2 > 2.5"), Some(int(0)));
    assert_eq!(eval_str("2.5 > 2"), Some(int(1)));
    assert_eq!(eval_str("2 < 2.5"), Some(int(1)));
    assert_eq!(eval_str("2 >= 2.5"), Some(int(0)));
    assert_eq!(eval_str("2 <= 2.5"), Some(int(1)));
    assert_eq!(eval_str("2 == 2.5"), Some(int(0)));
    assert_eq!(eval_str("2 != 2.5"), Some(int(1)));
}

#[test]
fn comparisons_word_eq_ne_on_numbers() {
    // tclsh: `5 eq 5`=1, `5 ne 6`=1 (string compare of canonical reps).
    assert_eq!(eval_str("5 eq 5"), Some(int(1)));
    assert_eq!(eval_str("5 ne 6"), Some(int(1)));
}

// =========================================================================
// Logical
// =========================================================================

#[test]
fn logical_and_or_not() {
    // tclsh: 1&&1=1  1&&0=0  0||1=1  0||0=0  !0=1  !1=0  !5=0
    assert_eq!(eval_str("1 && 1"), Some(int(1)));
    assert_eq!(eval_str("1 && 0"), Some(int(0)));
    assert_eq!(eval_str("0 || 1"), Some(int(1)));
    assert_eq!(eval_str("0 || 0"), Some(int(0)));
    assert_eq!(eval_str("!0"), Some(int(1)));
    assert_eq!(eval_str("!1"), Some(int(0)));
    assert_eq!(eval_str("!5"), Some(int(0)));
}

#[test]
fn logical_short_circuit_skips_unfoldable_rhs() {
    // tclsh: `0 && [cmd]`=0 and `1 || [cmd]`=1 — the unfoldable command subst on
    // the dead side must not block folding (short-circuit).
    assert_eq!(eval_str("0 && [cmd]"), Some(int(0)));
    assert_eq!(eval_str("1 || [cmd]"), Some(int(1)));
}

#[test]
fn logical_on_floats_and_chains() {
    // tclsh: 1.3&&3.3=1  0.0&&1.3=0  0.0||1.3=1  3.0||0.0=1
    //        0||0||1=1  1&&1&&2=1  !0.0=1  !27=0  !2.1=0
    assert_eq!(eval_str("1.3 && 3.3"), Some(int(1)));
    assert_eq!(eval_str("0.0 && 1.3"), Some(int(0)));
    assert_eq!(eval_str("0.0 || 1.3"), Some(int(1)));
    assert_eq!(eval_str("3.0 || 0.0"), Some(int(1)));
    assert_eq!(eval_str("0 || 0 || 1"), Some(int(1)));
    assert_eq!(eval_str("1 && 1 && 2"), Some(int(1)));
    assert_eq!(eval_str("!0.0"), Some(int(1)));
    assert_eq!(eval_str("!27"), Some(int(0)));
    assert_eq!(eval_str("!2.1"), Some(int(0)));
}

// =========================================================================
// Bitwise & shifts
// =========================================================================

#[test]
fn bitwise_and_or_xor_not() {
    // tclsh: 0xFF&0x0F=15  0xF0|0x0F=255  0xFF^0x0F=240  ~0=-1
    //        7&0x13=3  0xf2&0x53=82  -1&-7=-7  7|0x13=23  0|7=7
    //        7^0x13=20  3^0x10=19  0^7=7  -1^7=-8  ~4=-5  ~0xff00ff=-16711936
    assert_eq!(eval_str("0xFF & 0x0F"), Some(int(0x0F)));
    assert_eq!(eval_str("0xF0 | 0x0F"), Some(int(0xFF)));
    assert_eq!(eval_str("0xFF ^ 0x0F"), Some(int(0xF0)));
    assert_eq!(eval_str("~0"), Some(int(-1)));
    assert_eq!(eval_str("7 & 0x13"), Some(int(3)));
    assert_eq!(eval_str("0xf2 & 0x53"), Some(int(82)));
    assert_eq!(eval_str("-1 & -7"), Some(int(-7)));
    assert_eq!(eval_str("7 | 0x13"), Some(int(23)));
    assert_eq!(eval_str("0 | 7"), Some(int(7)));
    assert_eq!(eval_str("7 ^ 0x13"), Some(int(20)));
    assert_eq!(eval_str("3 ^ 0x10"), Some(int(19)));
    assert_eq!(eval_str("0 ^ 7"), Some(int(7)));
    assert_eq!(eval_str("-1 ^ 7"), Some(int(-8)));
    assert_eq!(eval_str("~4"), Some(int(-5)));
    assert_eq!(eval_str("~0xff00ff"), Some(int(-16_711_936)));
}

#[test]
fn shifts_left_and_right() {
    // tclsh: 1<<4=16  16>>2=4  1<<3=8  -0xf2<<0x3=-1936  0xff>>2=63
    //        -1>>2=-1  5>>32=0
    assert_eq!(eval_str("1 << 4"), Some(int(16)));
    assert_eq!(eval_str("16 >> 2"), Some(int(4)));
    assert_eq!(eval_str("1 << 3"), Some(int(8)));
    assert_eq!(eval_str("-0xf2 << 0x3"), Some(int(-1936)));
    assert_eq!(eval_str("0xff >> 2"), Some(int(63)));
    assert_eq!(eval_str("-1 >> 2"), Some(int(-1)));
    assert_eq!(eval_str("5 >> 32"), Some(int(0)));
}

#[test]
fn bitwise_and_shift_require_integers() {
    // tclsh: bitwise/shift on a float is a "can't use ... floating-point" error,
    // so the folder declines (None).
    assert_eq!(eval_str("1.0 & 2"), None);
    assert_eq!(eval_str("1.0 | 2"), None);
    assert_eq!(eval_str("1.0 << 2"), None);
}

// =========================================================================
// Ternary
// =========================================================================

#[test]
fn ternary_selects_branch_and_short_circuits() {
    // tclsh: 1?42:99=42  0?42:99=99  and the untaken arm (a command subst) is
    // never evaluated, so `1 ? 42 : [cmd]`=42 and `0 ? [cmd] : 99`=99.
    assert_eq!(eval_str("1 ? 42 : 99"), Some(int(42)));
    assert_eq!(eval_str("0 ? 42 : 99"), Some(int(99)));
    assert_eq!(eval_str("1 ? 42 : [cmd]"), Some(int(42)));
    assert_eq!(eval_str("0 ? [cmd] : 99"), Some(int(99)));
}

#[test]
fn ternary_nested_and_with_operators() {
    // tclsh: 1?2?3:4:0=3  0?2?3:4:0=0  3>2?44:66=44  2>3?44:66=66  1||0?3:4=3
    assert_eq!(eval_str("1 ? 2 ? 3 : 4 : 0"), Some(int(3)));
    assert_eq!(eval_str("0 ? 2 ? 3 : 4 : 0"), Some(int(0)));
    assert_eq!(eval_str("3 > 2 ? 44 : 66"), Some(int(44)));
    assert_eq!(eval_str("2 > 3 ? 44 : 66"), Some(int(66)));
    assert_eq!(eval_str("1 || 0 ? 3 : 4"), Some(int(3)));
}

// =========================================================================
// Unary
// =========================================================================

#[test]
fn unary_sign_chains_and_octal() {
    // tclsh: +36=36  -4=-4  --5=5  +--++36=36  +0o00123=83  -4.2=-4.2
    //        +--+-62.0=-62.0
    assert_eq!(eval_str("+36"), Some(int(36)));
    assert_eq!(eval_str("-4"), Some(int(-4)));
    assert_eq!(eval_str("--5"), Some(int(5)));
    assert_eq!(eval_str("+--++36"), Some(int(36)));
    assert_eq!(eval_str("+0o00123"), Some(int(83)));
    assert_eq!(eval_str("-4.2"), Some(float(-4.2)));
    assert_eq!(eval_str("+--+-62.0"), Some(float(-62.0)));
}

#[test]
fn unary_logical_and_bitwise_not() {
    // tclsh: !2=0  !0=1  ~3=-4
    assert_eq!(eval_str("!2"), Some(int(0)));
    assert_eq!(eval_str("!0"), Some(int(1)));
    assert_eq!(eval_str("~3"), Some(int(-4)));
}

// =========================================================================
// Operator precedence
// =========================================================================

#[test]
fn precedence_unary_vs_binary() {
    // tclsh: -~3=4 (-(~3)=-(-4))  -!3=0 (-(!3)=-(0))  -~0=1 (-(~0)=-(-1))
    assert_eq!(eval_str("-~3"), Some(int(4)));
    assert_eq!(eval_str("-!3"), Some(int(0)));
    assert_eq!(eval_str("-~0"), Some(int(1)));
}

#[test]
fn precedence_arithmetic_left_assoc() {
    // tclsh: 2*4/6=1  24/6*3=12  24/6/2=2  -2+4=2  6-3-2=1
    //        2*3+4=10  8/2+4=8  8%3+4=6
    assert_eq!(eval_str("2 * 4 / 6"), Some(int(1)));
    assert_eq!(eval_str("24 / 6 * 3"), Some(int(12)));
    assert_eq!(eval_str("24 / 6 / 2"), Some(int(2)));
    assert_eq!(eval_str("-2 + 4"), Some(int(2)));
    assert_eq!(eval_str("6 - 3 - 2"), Some(int(1)));
    assert_eq!(eval_str("2 * 3 + 4"), Some(int(10)));
    assert_eq!(eval_str("8 / 2 + 4"), Some(int(8)));
    assert_eq!(eval_str("8 % 3 + 4"), Some(int(6)));
}

#[test]
fn precedence_shifts_relative_to_arithmetic() {
    // tclsh: 7+1>>2=2  7+1<<2=32  7>>3-2=3  7<<3-2=14
    assert_eq!(eval_str("7 + 1 >> 2"), Some(int(2)));
    assert_eq!(eval_str("7 + 1 << 2"), Some(int(32)));
    assert_eq!(eval_str("7 >> 3 - 2"), Some(int(3)));
    assert_eq!(eval_str("7 << 3 - 2"), Some(int(14)));
}

#[test]
fn precedence_bitwise_and_logical() {
    // tclsh: 7&3^0x10=19  7^0x10|3=23  0&&1||1=1  1||1&&0=1
    assert_eq!(eval_str("7 & 3 ^ 0x10"), Some(int(19)));
    assert_eq!(eval_str("7 ^ 0x10 | 3"), Some(int(23)));
    assert_eq!(eval_str("0 && 1 || 1"), Some(int(1)));
    assert_eq!(eval_str("1 || 1 && 0"), Some(int(1)));
}

#[test]
fn precedence_chained_comparisons_left_to_right() {
    // tclsh chains left-to-right; `>` binds tighter than `==`:
    //   2<3<4=1 ((2<3)=1, 1<4)  4>3>=2=0 ((4>3)=1, 1>=2)
    //   1==4>3=1 (1 == (4>3) = 1==1)
    assert_eq!(eval_str("2 < 3 < 4"), Some(int(1)));
    assert_eq!(eval_str("4 > 3 >= 2"), Some(int(0)));
    assert_eq!(eval_str("1 == 4 > 3"), Some(int(1)));
}

#[test]
fn parenthesised_and_complex_expressions() {
    // tclsh: (1+2)*3=9  ((1+2)*(3+4))=21  2+3*4-1=13
    assert_eq!(eval_str("(1 + 2) * 3"), Some(int(9)));
    assert_eq!(eval_str("((1 + 2) * (3 + 4))"), Some(int(21)));
    assert_eq!(eval_str("2 + 3 * 4 - 1"), Some(int(13)));
}

// =========================================================================
// Math functions
// =========================================================================

#[test]
fn math_abs_int_and_float() {
    // tclsh: abs(-5)=5  abs(-5.0)=5.0  abs(-2147483648)=2147483648
    //        abs(-0)=0  abs(-0.0)=0.0  abs(-42)=42
    assert_eq!(eval_str("abs(-5)"), Some(int(5)));
    assert_eq!(eval_str("abs(-5.0)"), Some(float(5.0)));
    assert_eq!(eval_str("abs(-2147483648)"), Some(int(2_147_483_648)));
    assert_eq!(eval_str("abs(-0)"), Some(int(0)));
    assert_eq!(eval_str("abs(-0.0)"), Some(float(0.0)));
    assert_eq!(eval_str("abs(-42)"), Some(int(42)));
}

#[test]
fn math_int_double_bool_conversions() {
    // tclsh: int(3.7)=3  int(-3.7)=-3  int(5)=5  double(5)=5.0  double(27)=27.0
    //        bool(0)=0  bool(42)=1  bool(-1+1)=0  bool(0+1)=1  entier(3.7)=3
    //        wide(42)=42
    assert_eq!(eval_str("int(3.7)"), Some(int(3)));
    assert_eq!(eval_str("int(-3.7)"), Some(int(-3)));
    assert_eq!(eval_str("int(5)"), Some(int(5)));
    assert_eq!(eval_str("double(5)"), Some(float(5.0)));
    assert_eq!(eval_str("double(27)"), Some(float(27.0)));
    assert_eq!(eval_str("bool(0)"), Some(int(0)));
    assert_eq!(eval_str("bool(42)"), Some(int(1)));
    assert_eq!(eval_str("bool(-1 + 1)"), Some(int(0)));
    assert_eq!(eval_str("bool(0 + 1)"), Some(int(1)));
    assert_eq!(eval_str("entier(3.7)"), Some(int(3)));
    assert_eq!(eval_str("wide(42)"), Some(int(42)));
}

#[test]
fn math_round_ties_away_from_zero() {
    // tclsh round (ties away from zero, NOT banker's):
    //   round(2.5)=3  round(3.5)=4  round(-2.5)=-3  round(0.5)=1  round(1.5)=2
    //   round(-0.5)=-1  round(-1.5)=-2  round(3.0)=3
    assert_eq!(eval_str("round(2.5)"), Some(int(3)));
    assert_eq!(eval_str("round(3.5)"), Some(int(4)));
    assert_eq!(eval_str("round(-2.5)"), Some(int(-3)));
    assert_eq!(eval_str("round(0.5)"), Some(int(1)));
    assert_eq!(eval_str("round(1.5)"), Some(int(2)));
    assert_eq!(eval_str("round(-0.5)"), Some(int(-1)));
    assert_eq!(eval_str("round(-1.5)"), Some(int(-2)));
    assert_eq!(eval_str("round(3.0)"), Some(int(3)));
}

#[test]
fn math_ceil_and_floor_return_floats() {
    // tclsh: ceil(2.1)=2.0  floor(2.9)=2.0 (both return doubles).
    assert_eq!(eval_str("ceil(2.1)"), Some(float(3.0)));
    assert_eq!(eval_str("floor(2.9)"), Some(float(2.0)));
}

#[test]
fn math_sqrt_isqrt_min_max() {
    // tclsh: sqrt(16)=4.0  sqrt(-1)→domain error→None  isqrt(16)=4
    //        min(3,1,2)=1  max(3,1,2)=3  min(5,3,7)=3  max(5,3,7)=7
    assert_eq!(eval_str("sqrt(16)"), Some(float(4.0)));
    assert_eq!(eval_str("sqrt(-1)"), None);
    assert_eq!(eval_str("isqrt(16)"), Some(int(4)));
    assert_eq!(eval_str("min(3, 1, 2)"), Some(int(1)));
    assert_eq!(eval_str("max(3, 1, 2)"), Some(int(3)));
    assert_eq!(eval_str("min(5, 3, 7)"), Some(int(3)));
    assert_eq!(eval_str("max(5, 3, 7)"), Some(int(7)));
}

#[test]
fn math_pow_log_exp_and_friends() {
    // tclsh: pow(2,3)=8.0  hypot(3,4)=5.0  log(1)=0.0  log10(100)=2.0
    //        fmod(7.5,3.0)=1.5  atan2(0,1)=0.0
    assert_eq!(eval_str("pow(2, 3)"), Some(float(8.0)));
    assert_eq!(eval_str("hypot(3, 4)"), Some(float(5.0)));
    assert_eq!(eval_str("log(1)"), Some(float(0.0)));
    assert_eq!(eval_str("log10(100)"), Some(float(2.0)));
    assert_eq!(eval_str("fmod(7.5, 3.0)"), Some(float(1.5)));
    assert_eq!(eval_str("atan2(0, 1)"), Some(float(0.0)));
}

#[test]
fn math_trig_exact_zero_and_one_results() {
    // tclsh exact-representable results: sin(0)=0.0  cos(0)=1.0  tan(0)=0.0
    //   atan(0)=0.0  asin(0)=0.0  sinh(0)=0.0  cosh(0)=1.0  tanh(0)=0.0
    assert_eq!(eval_str("sin(0)"), Some(float(0.0)));
    assert_eq!(eval_str("cos(0)"), Some(float(1.0)));
    assert_eq!(eval_str("tan(0)"), Some(float(0.0)));
    assert_eq!(eval_str("atan(0)"), Some(float(0.0)));
    assert_eq!(eval_str("asin(0)"), Some(float(0.0)));
    assert_eq!(eval_str("sinh(0)"), Some(float(0.0)));
    assert_eq!(eval_str("cosh(0)"), Some(float(1.0)));
    assert_eq!(eval_str("tanh(0)"), Some(float(0.0)));
}

#[test]
fn math_trig_approx_results() {
    // tclsh: exp(1)=e≈2.71828…  acos(0)=π/2≈1.5707963… — compare within tol.
    match eval_str("exp(1)") {
        Some(TclValue::Float(f)) => assert!((f - std::f64::consts::E).abs() < 1e-10),
        other => panic!("exp(1) folded to {other:?}, expected a float"),
    }
    match eval_str("acos(0)") {
        Some(TclValue::Float(f)) => assert!((f - std::f64::consts::FRAC_PI_2).abs() < 1e-10),
        other => panic!("acos(0) folded to {other:?}, expected a float"),
    }
}

#[test]
fn math_classification_predicates() {
    // tclsh: isinf(1)=0  isnan(1)=0  isfinite(42)=1  isfinite(3.14)=1
    assert_eq!(eval_str("isinf(1)"), Some(int(0)));
    assert_eq!(eval_str("isnan(1)"), Some(int(0)));
    assert_eq!(eval_str("isfinite(42)"), Some(int(1)));
    assert_eq!(eval_str("isfinite(3.14)"), Some(int(1)));
}

#[test]
fn math_rand_is_not_foldable() {
    // Non-deterministic — the folder must never constant-fold rand().
    assert_eq!(eval_str("rand()"), None);
}

#[test]
fn math_function_folding_combined_expressions() {
    // tclsh: 3.14*2=6.28  1.5+2.5=4.0  abs(-5)+3=8  int(3.7)*2=6  round(2.5)+1=4
    assert_eq!(eval_str("3.14 * 2"), Some(float(6.28)));
    assert_eq!(eval_str("1.5 + 2.5"), Some(float(4.0)));
    assert_eq!(eval_str("abs(-5) + 3"), Some(int(8)));
    assert_eq!(eval_str("int(3.7) * 2"), Some(int(6)));
    assert_eq!(eval_str("round(2.5) + 1"), Some(int(4)));
}

// =========================================================================
// Variables
// =========================================================================

#[test]
fn variable_resolution_and_coercion() {
    // A bound int folds; a numeric *string* binding coerces; a non-numeric
    // string binding declines; an unbound variable declines.
    let mut env = Env::new();
    env.insert("a".into(), EnvValue::Int(5));
    assert_eq!(eval_str_env("$a + 1", &env), Some(int(6))); // a=5 → 6

    let mut env2 = Env::new();
    env2.insert("a".into(), EnvValue::Int(3));
    env2.insert("b".into(), EnvValue::Int(4));
    assert_eq!(eval_str_env("$a + $b", &env2), Some(int(7))); // 3+4

    // String binding "5" is numeric → coerces to 6 in `+`.
    assert_eq!(eval_str_env("$a + 1", &env_str("a", "5")), Some(int(6)));
    // Non-numeric string → can't fold `+`.
    assert_eq!(eval_str_env("$a + 1", &env_str("a", "hello")), None);
    // Unbound variable → can't fold.
    assert_eq!(eval_str("$a + 1"), None);
}

// =========================================================================
// Unevaluable
// =========================================================================

#[test]
fn unevaluable_command_substitution_and_empty() {
    // A command substitution is opaque at compile time, and an empty/unparseable
    // expression has no value → both decline.
    assert_eq!(eval_str("[clock seconds]"), None);
    assert_eq!(eval_str(""), None);
    assert_eq!(eval_str("1 + [foo]"), None);
}

// =========================================================================
// format_tcl_value
// =========================================================================

#[test]
fn format_tcl_value_integers() {
    assert_eq!(format_tcl_value(&TclValue::Int(42)), "42");
    assert_eq!(format_tcl_value(&TclValue::Int(-7)), "-7");
    assert_eq!(format_tcl_value(&TclValue::Int(0)), "0");
}

#[test]
fn format_tcl_value_floats() {
    // Whole-valued doubles keep a trailing `.0`; fractional ones round-trip.
    assert_eq!(format_tcl_value(&TclValue::Float(3.0)), "3.0");
    assert_eq!(format_tcl_value(&TclValue::Float(3.14)), "3.14");
    assert_eq!(format_tcl_value(&TclValue::Float(42.0)), "42.0");
    assert_eq!(format_tcl_value(&TclValue::Float(-1.0)), "-1.0");
}

#[test]
fn format_tcl_value_signed_zero() {
    // IEEE-754 sign bit survives: Tcl renders -0.0 as "-0.0".
    assert_eq!(format_tcl_value(&TclValue::Float(0.0)), "0.0");
    assert_eq!(format_tcl_value(&TclValue::Float(-0.0)), "-0.0");
    // -1.0 * 0.0 == -0.0 by IEEE-754.
    let neg_zero = -0.0_f64;
    assert!(neg_zero.is_sign_negative());
    assert_eq!(format_tcl_value(&TclValue::Float(neg_zero)), "-0.0");
}

#[test]
fn format_tcl_value_specials_and_large() {
    // tclsh 9.0.3: Inf/-Inf/NaN (capitalised), and 1e308 stays scientific
    // (not a huge integer string).
    assert_eq!(format_tcl_value(&TclValue::Float(f64::INFINITY)), "Inf");
    assert_eq!(
        format_tcl_value(&TclValue::Float(f64::NEG_INFINITY)),
        "-Inf"
    );
    assert_eq!(format_tcl_value(&TclValue::Float(f64::NAN)), "NaN");
    assert_eq!(format_tcl_value(&TclValue::Float(1e308)), "1e+308");
    assert_eq!(format_tcl_value(&TclValue::Float(-1e308)), "-1e+308");
}

// =========================================================================
// String comparison folding
//   `eq`/`ne`/`lt`/`gt`/`le`/`ge` compare operands AS STRINGS.
// =========================================================================

#[test]
fn word_string_compare_eq_ne() {
    // tclsh: "x" eq "x"=1  "x" eq "y"=0  "abc" eq "abc"=1  "x" ne "y"=1
    //        "x" ne "x"=0
    assert_eq!(eval_str(r#""x" eq "x""#), Some(int(1)));
    assert_eq!(eval_str(r#""x" eq "y""#), Some(int(0)));
    assert_eq!(eval_str(r#""abc" eq "abc""#), Some(int(1)));
    assert_eq!(eval_str(r#""x" ne "y""#), Some(int(1)));
    assert_eq!(eval_str(r#""x" ne "x""#), Some(int(0)));
}

#[test]
fn word_string_compare_ordering() {
    // tclsh: "a" lt "b"=1  "b" gt "a"=1  "a" le "a"=1  "b" ge "b"=1
    assert_eq!(eval_str(r#""a" lt "b""#), Some(int(1)));
    assert_eq!(eval_str(r#""b" gt "a""#), Some(int(1)));
    assert_eq!(eval_str(r#""a" le "a""#), Some(int(1)));
    assert_eq!(eval_str(r#""b" ge "b""#), Some(int(1)));
}

#[test]
fn word_eq_is_string_compare_not_numeric() {
    // tclsh: `eq` compares the *strings*, so 5 eq 5.0 → 0 ("5" vs "5.0"),
    // even though they're numerically equal. 5 eq 5=1, `5 eq "5"`=1.
    assert_eq!(eval_str("5 eq 5"), Some(int(1)));
    assert_eq!(eval_str("5 eq 5.0"), Some(int(0)));
    assert_eq!(eval_str(r#"5 eq "5""#), Some(int(1)));
}

// =========================================================================
// Polymorphic equality folding
//   `==`/`!=` are numeric when *both* operands are numbers, else string.
// =========================================================================

#[test]
fn polymorphic_equality_string_operands_fold() {
    // tclsh: "foo"=="foo"=1  "foo"=="bar"=0  "foo"!="bar"=1  "foo"!="foo"=0
    assert_eq!(eval_str(r#""foo" == "foo""#), Some(int(1)));
    assert_eq!(eval_str(r#""foo" == "bar""#), Some(int(0)));
    assert_eq!(eval_str(r#""foo" != "bar""#), Some(int(1)));
    assert_eq!(eval_str(r#""foo" != "foo""#), Some(int(0)));
}

#[test]
fn polymorphic_equality_mixed_operand_with_var() {
    // tclsh string compare via a bound variable: $x=="foo" etc.
    assert_eq!(
        eval_str_env(r#"$x == "foo""#, &env_str("x", "foo")),
        Some(int(1))
    );
    assert_eq!(
        eval_str_env(r#"$x == "foo""#, &env_str("x", "bar")),
        Some(int(0))
    );
    assert_eq!(
        eval_str_env(r#"$x != "foo""#, &env_str("x", "bar")),
        Some(int(1))
    );
}

#[test]
fn polymorphic_equality_both_numeric_uses_numeric_compare() {
    // tclsh: 5==5=1  5!=6=1  5==5.0=1  "5"==5=1  5=="5.0"=1
    assert_eq!(eval_str("5 == 5"), Some(int(1)));
    assert_eq!(eval_str("5 != 6"), Some(int(1)));
    assert_eq!(eval_str("5 == 5.0"), Some(int(1)));
    assert_eq!(eval_str(r#""5" == 5"#), Some(int(1)));
    assert_eq!(eval_str(r#"5 == "5.0""#), Some(int(1)));
}

#[test]
fn polymorphic_equality_boolean_words_compare_as_strings() {
    // tclsh: bareword booleans are NOT numbers for `==`/`!=`, so they string
    // compare: true==1=0  1==true=0  true==true=1  false==0=0.
    assert_eq!(eval_str(r#""true" == "1""#), Some(int(0)));
    assert_eq!(eval_str("true == 1"), Some(int(0)));
    assert_eq!(eval_str(r#"1 == "true""#), Some(int(0)));
    assert_eq!(eval_str(r#""true" == "true""#), Some(int(1)));
    assert_eq!(eval_str("false == 0"), Some(int(0)));
}

#[test]
fn polymorphic_equality_not_yet_folds_for_unbound_operand() {
    // An unbound variable leaves the comparison unevaluable.
    assert_eq!(eval_str(r#"$x == "foo""#), None);
}

#[test]
fn polymorphic_equality_leading_zero_is_dialect_aware() {
    // A bare leading-zero integer is octal in Tcl 8.x but decimal in 9.0
    // (TIP 472); the fold follows the dialect (via `eval_tcl_expr_in_dialect`):
    //   tcl8.6: "08"=="8"=0 ("08" invalid octal→string), "010"=="10"=0
    //           (octal 8 ≠ 10), "010"=="8"=1 (octal 010 == 8).
    //   tcl9.0: "08"=="8"=1 (decimal), "010"=="10"=1 (decimal).
    assert_eq!(eval_dialect(r#""08" == "8""#, "tcl8.6"), Some(int(0)));
    assert_eq!(eval_dialect(r#""010" == "10""#, "tcl8.6"), Some(int(0)));
    assert_eq!(eval_dialect(r#""010" == "8""#, "tcl8.6"), Some(int(1)));
    assert_eq!(eval_dialect(r#""08" == "8""#, "tcl9.0"), Some(int(1)));
    assert_eq!(eval_dialect(r#""010" == "10""#, "tcl9.0"), Some(int(1)));

    // Default (unknown) dialect: a leading-zero operand is ambiguous → decline.
    assert_eq!(eval_str(r#""08" == "8""#), None);
}

// =========================================================================
// iRules `contains`
// =========================================================================

#[test]
fn irules_contains() {
    // Substring test (case-sensitive). tclsh-string semantics:
    //   "hello world" contains "world"=1, "xyz"=0, "" (empty needle)=1,
    //   "" contains "a"=0, "Hello" contains "hello"=0 (case), "abc" contains "abc"=1.
    assert_eq!(
        eval_irules(r#""hello world" contains "world""#),
        Some(int(1))
    );
    assert_eq!(eval_irules(r#""hello world" contains "xyz""#), Some(int(0)));
    assert_eq!(eval_irules(r#""hello" contains """#), Some(int(1)));
    assert_eq!(eval_irules(r#""" contains "a""#), Some(int(0)));
    assert_eq!(eval_irules(r#""Hello" contains "hello""#), Some(int(0)));
    assert_eq!(eval_irules(r#""abc" contains "abc""#), Some(int(1)));
}

#[test]
fn irules_contains_with_variable() {
    assert_eq!(
        eval_irules_env(r#"$uri contains "/admin""#, &env_str("uri", "/admin/login")),
        Some(int(1))
    );
    assert_eq!(
        eval_irules_env(r#"$uri contains "/admin""#, &env_str("uri", "/public")),
        Some(int(0))
    );
}

// =========================================================================
// iRules `starts_with` / `ends_with`
// =========================================================================

#[test]
fn irules_starts_with() {
    assert_eq!(
        eval_irules(r#""hello world" starts_with "hello""#),
        Some(int(1))
    );
    assert_eq!(
        eval_irules(r#""hello world" starts_with "world""#),
        Some(int(0))
    );
    assert_eq!(eval_irules(r#""hello" starts_with """#), Some(int(1)));
    assert_eq!(eval_irules(r#""abc" starts_with "abc""#), Some(int(1)));
    assert_eq!(eval_irules(r#""ab" starts_with "abc""#), Some(int(0)));
    assert_eq!(
        eval_irules_env(
            r#"$path starts_with "/api/""#,
            &env_str("path", "/api/v2/users")
        ),
        Some(int(1))
    );
}

#[test]
fn irules_ends_with() {
    assert_eq!(eval_irules(r#""hello.txt" ends_with ".txt""#), Some(int(1)));
    assert_eq!(eval_irules(r#""hello.txt" ends_with ".jpg""#), Some(int(0)));
    assert_eq!(eval_irules(r#""hello" ends_with """#), Some(int(1)));
    assert_eq!(eval_irules(r#""abc" ends_with "abc""#), Some(int(1)));
    assert_eq!(eval_irules(r#""bc" ends_with "abc""#), Some(int(0)));
    assert_eq!(
        eval_irules_env(
            r#"$host ends_with ".example.com""#,
            &env_str("host", "www.example.com")
        ),
        Some(int(1))
    );
}

// =========================================================================
// iRules `equals`
// =========================================================================

#[test]
fn irules_equals() {
    assert_eq!(eval_irules(r#""hello" equals "hello""#), Some(int(1)));
    assert_eq!(eval_irules(r#""hello" equals "world""#), Some(int(0)));
    assert_eq!(eval_irules(r#""Hello" equals "hello""#), Some(int(0))); // case-sensitive
    assert_eq!(eval_irules(r#""" equals """#), Some(int(1)));
    assert_eq!(
        eval_irules_env(r#"$method equals "GET""#, &env_str("method", "GET")),
        Some(int(1))
    );
    assert_eq!(
        eval_irules_env(r#"$method equals "GET""#, &env_str("method", "POST")),
        Some(int(0))
    );
}

// =========================================================================
// iRules `matches_glob`
// =========================================================================

#[test]
fn irules_matches_glob() {
    // `string match` glob semantics: *, ?, [ranges].
    assert_eq!(
        eval_irules(r#""hello.txt" matches_glob "*.txt""#),
        Some(int(1))
    );
    assert_eq!(
        eval_irules(r#""hello.txt" matches_glob "*.jpg""#),
        Some(int(0))
    );
    assert_eq!(eval_irules(r#""cat" matches_glob "c?t""#), Some(int(1)));
    assert_eq!(eval_irules(r#""cart" matches_glob "c?t""#), Some(int(0)));
    assert_eq!(eval_irules(r#""hello" matches_glob "hello""#), Some(int(1)));
    assert_eq!(
        eval_irules(r#""/api/v2/users" matches_glob "/api/*/users""#),
        Some(int(1))
    );
    assert_eq!(eval_irules(r#""a1" matches_glob "a[0-9]""#), Some(int(1)));
    assert_eq!(eval_irules(r#""ax" matches_glob "a[0-9]""#), Some(int(0)));
    assert_eq!(
        eval_irules_env(
            r#"$uri matches_glob "/images/*""#,
            &env_str("uri", "/images/logo.png")
        ),
        Some(int(1))
    );
}

// =========================================================================
// iRules `matches_regex`
//   Deliberately NEVER constant-folded (Rust `regex` != Tcl ARE) → always None.
// =========================================================================

#[test]
fn irules_matches_regex_is_never_folded() {
    // The folder defers ALL `matches_regex` to the runtime ARE engine (the Rust
    // `regex` crate's syntax/semantics differ from Tcl ARE), so every pattern —
    // matching, non-matching, or invalid — folds to None. The in-crate
    // `irules_matches_regex_is_not_folded` test is the canonical statement of
    // this behaviour.
    for expr in [
        r#""hello123" matches_regex "[a-z]+[0-9]+""#,
        r#""hello" matches_regex "^[0-9]+$""#,
        r#""abc123def" matches_regex "[0-9]+""#,
        r#""hello" matches_regex "^hello$""#,
        r#""hello world" matches_regex "^hello$""#,
        r#""hello" matches_regex "[invalid""#,
        r#""anything" matches_regex ".*""#,
    ] {
        assert_eq!(eval_irules(expr), None, "{expr}");
    }
    // Variable operand likewise declines.
    assert_eq!(
        eval_irules_env(
            r#"$uri matches_regex "^/api/v[0-9]+/""#,
            &env_str("uri", "/api/v3/users")
        ),
        None
    );
}

// =========================================================================
// iRules `in` / `ni`
// =========================================================================

#[test]
fn irules_in_list_membership() {
    assert_eq!(
        eval_irules(r#""apple" in "apple banana cherry""#),
        Some(int(1))
    );
    assert_eq!(
        eval_irules(r#""grape" in "apple banana cherry""#),
        Some(int(0))
    );
    assert_eq!(eval_irules(r#""apple" in "apple""#), Some(int(1)));
    assert_eq!(eval_irules(r#""apple" in """#), Some(int(0)));
    // Braced grouping: {hello world} is one element.
    assert_eq!(
        eval_irules(r#""hello world" in "{hello world} simple""#),
        Some(int(1))
    );
    // A prefix is not a list element.
    assert_eq!(eval_irules(r#""app" in "apple banana""#), Some(int(0)));
    assert_eq!(
        eval_irules_env(
            r#"$method in "GET HEAD OPTIONS""#,
            &env_str("method", "GET")
        ),
        Some(int(1))
    );
    assert_eq!(
        eval_irules_env(
            r#"$method in "GET HEAD OPTIONS""#,
            &env_str("method", "POST")
        ),
        Some(int(0))
    );
}

#[test]
fn irules_ni_negated_membership() {
    assert_eq!(
        eval_irules(r#""grape" ni "apple banana cherry""#),
        Some(int(1))
    );
    assert_eq!(
        eval_irules(r#""apple" ni "apple banana cherry""#),
        Some(int(0))
    );
    assert_eq!(eval_irules(r#""apple" ni """#), Some(int(1)));
    assert_eq!(
        eval_irules_env(r#"$method ni "GET HEAD""#, &env_str("method", "POST")),
        Some(int(1))
    );
    assert_eq!(
        eval_irules_env(r#"$method ni "GET HEAD""#, &env_str("method", "GET")),
        Some(int(0))
    );
}

// =========================================================================
// iRules word logical (`and`/`or`/`not`)
// =========================================================================

#[test]
fn irules_word_logical_and_or_not() {
    assert_eq!(eval_irules("1 and 1"), Some(int(1)));
    assert_eq!(eval_irules("1 and 0"), Some(int(0)));
    assert_eq!(eval_irules("0 and 1"), Some(int(0)));
    assert_eq!(eval_irules("1 or 0"), Some(int(1)));
    assert_eq!(eval_irules("0 or 1"), Some(int(1)));
    assert_eq!(eval_irules("0 or 0"), Some(int(0)));
    assert_eq!(eval_irules("not 1"), Some(int(0)));
    assert_eq!(eval_irules("not 0"), Some(int(1)));
}

#[test]
fn irules_word_logical_short_circuits() {
    // `0 and [cmd]`=0 and `1 or [cmd]`=1 — the unfoldable RHS is skipped.
    assert_eq!(eval_irules("0 and [some_cmd]"), Some(int(0)));
    assert_eq!(eval_irules("1 or [some_cmd]"), Some(int(1)));
}

// =========================================================================
// iRules combined expressions
// =========================================================================

#[test]
fn irules_combined_string_ops_with_logical() {
    assert_eq!(
        eval_irules(
            r#""http://example.com/api" contains "api" and "http://example.com/api" starts_with "http""#
        ),
        Some(int(1))
    );
    assert_eq!(eval_irules(r#"not ("hello" contains "xyz")"#), Some(int(1)));
    assert_eq!(
        eval_irules(r#""test.jpg" ends_with ".jpg" or "test.jpg" ends_with ".png""#),
        Some(int(1))
    );
    assert_eq!(
        eval_irules(r#""test.gif" ends_with ".jpg" or "test.gif" ends_with ".png""#),
        Some(int(0))
    );
    assert_eq!(
        eval_irules(r#""hello" contains "ell" ? 42 : 0"#),
        Some(int(42))
    );
}

#[test]
fn irules_combined_with_variables() {
    let mut env = Env::new();
    env.insert("method".into(), EnvValue::Str("GET".into()));
    env.insert("uri".into(), EnvValue::Str("/api/data".into()));
    assert_eq!(
        eval_irules_env(r#"$method in "GET HEAD" and $uri starts_with "/api""#, &env),
        Some(int(1))
    );

    let mut env2 = Env::new();
    env2.insert("host".into(), EnvValue::Str("www.example.com".into()));
    env2.insert("domain".into(), EnvValue::Str("example".into()));
    assert_eq!(
        eval_irules_env("$host contains $domain", &env2),
        Some(int(1))
    );

    let mut env3 = Env::new();
    env3.insert("x".into(), EnvValue::Int(15));
    env3.insert("uri".into(), EnvValue::Str("/admin/panel".into()));
    assert_eq!(
        eval_irules_env(r#"($x > 10) and ($uri contains "/admin")"#, &env3),
        Some(int(1))
    );
}
