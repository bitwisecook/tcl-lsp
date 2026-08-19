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

//! Expression and arithmetic semantics.
//!
//! The numeric/comparison/unary logic is single-sourced here and used by *both*
//! the inline arithmetic opcodes (`ADD`/`LT`/`UMINUS`/…) and `EXPR_STK` (via the
//! [`ExprEval`] adapter over [`tcl_syntax::expr::ExprOps`]), so the VM shares
//! the expr tower with the const-folder and the runtime.
// Integer→double coercion in the shared arithmetic is intentional (Tcl `expr`
// promotes to double), so the precision loss is the defined behaviour.
#![allow(clippy::cast_precision_loss)]

use std::cmp::Ordering;

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{FromPrimitive, Signed, ToPrimitive, Zero};
use tcl_runtime_api::Code;
use tcl_syntax::expr::{BinOp, ExprOps, NumericCompare, UnaryOp};
use tcl_syntax::number::{self, Number};
use tcl_syntax::number_tower;

use crate::error::TclError;
use crate::interp::Vm;
use crate::value::Value;

/// A coerced numeric operand. `Big` carries an out-of-`i64` integer that still
/// fits `i128` — the fast integer tier; `Huge` is the arbitrary-precision tier
/// beyond it, so no integer magnitude wraps, errors, or
/// degrades to a lossy `double`.
enum Num {
    Int(i64),
    Big(i128),
    Huge(BigInt),
    Dbl(f64),
}

fn num_f(n: &Num) -> f64 {
    match n {
        Num::Int(i) => *i as f64,
        Num::Big(i) => *i as f64,
        // Float contagion converts a bignum operand rather than refusing it —
        // C's `Tcl_GetNumberFromObj` + `TclBignumToDouble` rounding.
        Num::Huge(b) => big_to_f64(b),
        Num::Dbl(f) => *f,
    }
}

/// The integer (`i128`) view of a fast-tier operand, for the integer arithmetic
/// path. `None` for a float (which routes to `dbl_arith`) and for an integer
/// past `i128` (which routes to `big_arith`).
fn num_i128(n: &Num) -> Option<i128> {
    match n {
        Num::Int(i) => Some(i128::from(*i)),
        Num::Big(i) => Some(*i),
        Num::Huge(_) | Num::Dbl(_) => None,
    }
}

/// The exact bignum view of an integer operand (`None` for a float), for the
/// arbitrary-precision path when the `i128` fast tier can't hold both operands.
fn num_as_bigint(n: &Num) -> Option<BigInt> {
    match n {
        Num::Int(i) => Some(BigInt::from(*i)),
        Num::Big(i) => Some(BigInt::from(*i)),
        Num::Huge(b) => Some(b.clone()),
        Num::Dbl(_) => None,
    }
}

/// The nearest `f64` to an arbitrary-precision integer — C's
/// `TclBignumToDouble`: round-to-nearest-even over the *full* value, overflowing
/// to `±Inf` past the double range. (`num-bigint`'s `to_f64` truncates to the
/// top 64 bits before rounding, which mis-rounds a tie whose deciding bits sit
/// below the truncation — e.g. `2**127 + 2**74 + 1` must round up.)
pub(crate) fn big_to_f64(b: &BigInt) -> f64 {
    let mag = b.magnitude();
    let bits = mag.bits();
    let f = if bits <= 64 {
        // Every bit is present in one word: the primitive u64→f64 cast is
        // exactly the round-to-nearest-even C performs.
        mag.to_u64().map_or(0.0, |m| m as f64)
    } else if bits > 1024 {
        // Past `DBL_MAX_EXP` no finite double exists; C returns ±HUGE_VAL.
        f64::INFINITY
    } else {
        // Keep the top 64 bits. The dropped remainder matters only when the
        // kept part sits exactly on a rounding tie (its low 11 bits are one
        // half-ulp), where any dropped bit must break the tie upward.
        let excess = bits - 64;
        let mut top = (mag >> excess).to_u64().unwrap_or(u64::MAX);
        let dropped_nonzero = mag.trailing_zeros().is_some_and(|tz| tz < excess);
        if top & 0x7FF == 0x400 && dropped_nonzero {
            top += 1;
        }
        // `excess` ≤ 960 here, so the exponent fits `i32` and the power-of-two
        // scale is exact.
        (top as f64) * 2f64.powi(i32::try_from(excess).unwrap_or(i32::MAX))
    };
    if b.is_negative() { -f } else { f }
}

/// Wrap an `i128` arithmetic result as a value: a plain wide when it fits,
/// otherwise the decimal string (the VM has no wider integer rep).
pub(crate) fn int_value(r: i128) -> Value {
    i64::try_from(r).map_or_else(|_| Value::string(r.to_string()), Value::int)
}

/// Wrap an arbitrary-precision integer result as a value, narrowing to a wide
/// when it fits (so `2+2` via the bignum path is still `Value::int(4)`), else its
/// canonical decimal string.
pub(crate) fn big_value(r: &BigInt) -> Value {
    r.to_i64()
        .map_or_else(|| Value::string(r.to_string()), Value::int)
}

/// The exact arbitrary-precision integer value of `v`, or `None` when `v` is not
/// an integer (a float / non-number). Covers magnitudes beyond `i128` that
/// [`Value::as_i128`] can't, by reading the parsed `Big` literal. Shared with
/// `value_ops::int_add` so `incr`/`dict incr` promote to bignum too.
pub(crate) fn value_as_bigint(v: &Value) -> Option<BigInt> {
    if let Ok(n) = v.as_int() {
        return Some(BigInt::from(n));
    }
    match number::parse_whole(v.to_str().trim()) {
        Some(Number::Int(n)) => Some(BigInt::from(n)),
        Some(Number::Big {
            negative,
            radix,
            digits,
        }) => {
            let b = BigInt::parse_bytes(digits.as_bytes(), radix as u32)?;
            Some(if negative { -b } else { b })
        }
        _ => None,
    }
}

/// Integer arithmetic in arbitrary precision — the promotion target of the
/// `i128` fast path ([`int_arith`]) on overflow, and the direct path when an
/// operand already exceeds `i128`. The value semantics — floor div/mod, the
/// `**` collapses, the shift edge rules, two's-complement bit ops — are the
/// shared tower's ([`number_tower`]); this adopter adds the error surfaces
/// (message text) and the count/exponent narrowing.
fn big_arith(op: BinOp, x: &BigInt, y: &BigInt) -> Result<Value, TclError> {
    use BinOp::{Add, BitAnd, BitOr, BitXor, Div, LShift, Mod, Mul, Pow, RShift, Sub};
    let r = match op {
        Add => x + y,
        Sub => x - y,
        Mul => x * y,
        Div => number_tower::int_div(x, y).ok_or_else(divzero)?,
        Mod => number_tower::int_mod(x, y).ok_or_else(divzero)?,
        Pow => return big_pow(x, y),
        LShift => {
            if y.is_negative() {
                return Err(TclError::new("negative shift argument"));
            }
            // C checks the zero base before the count: `0 << huge` is 0,
            // not the count-overflow error.
            if num_traits::Zero::is_zero(x) {
                return Ok(Value::int(0));
            }
            // C's left-shift count must fit an `int` (`mp_mul_2d`): past
            // `INT_MAX` it raises the overflow error rather than attempt an
            // astronomic result (tclsh: `2 << 2**32` errors).
            let count = y
                .to_u32()
                .filter(|&c| i32::try_from(c).is_ok())
                .ok_or_else(|| TclError::new("integer value too large to represent"))?;
            number_tower::BigIntOps::shl(x, count)
        }
        RShift => {
            // A count past `i64` collapses exactly like any count past the
            // operand width, so fold it to a same-sign stand-in for the tower.
            let count = y
                .to_i64()
                .unwrap_or(if y.is_negative() { -1 } else { i64::MAX });
            number_tower::int_shr(x, count)
                .ok_or_else(|| TclError::new("negative shift argument"))?
        }
        BitAnd => x & y,
        BitOr => x | y,
        BitXor => x ^ y,
        _ => return Err(TclError::new("unsupported integer operator")),
    };
    Ok(big_value(&r))
}

/// `x ** y` in arbitrary precision, via the tower's
/// [`int_pow`](number_tower::int_pow). An exponent past `i64` folds to an
/// equal-sign, equal-parity stand-in: beyond [`number_tower::MAX_EXPONENT`]
/// only the `0`/`±1` collapses remain computable, and those depend on nothing
/// but the exponent's sign and parity (tclsh: `2 ** 10**20` is "exponent too
/// large" while `(-1) ** 10**20` is `1`).
fn big_pow(x: &BigInt, y: &BigInt) -> Result<Value, TclError> {
    let exponent = y.to_i64().unwrap_or(match (y.is_negative(), y.is_even()) {
        (true, true) => -2,
        (true, false) => -1,
        (false, true) => number_tower::MAX_EXPONENT + 1,
        (false, false) => number_tower::MAX_EXPONENT + 2,
    });
    match number_tower::int_pow(x, exponent) {
        Some(r) => Ok(big_value(&r)),
        // `int_pow` declines exactly two cases: a zero base with a negative
        // exponent (the domain error) and an exponent past the C limit. The
        // first carries `-errorcode ARITH DOMAIN` (`tcl9.0.4`
        // `tclExecute.c:7541`, `:8021`); the second sets no code at all
        // (`GENERAL_ARITHMETIC_ERROR`, `tclExecute.c:8380`) — tclsh 8.6.16 and
        // 9.0.4 both report `errorCode` `NONE` there.
        None if x.is_zero() => Err(TclError::with_error_code(
            ZERO_POWER_MSG,
            format!("ARITH DOMAIN {{{ZERO_POWER_MSG}}}"),
        )),
        None => Err(TclError::new("exponent too large")),
    }
}

/// C's message for a zero base raised to a negative power — a *domain* error,
/// not a division by zero (`tcl9.0.4/generic/tclExecute.c:8021`).
pub(crate) const ZERO_POWER_MSG: &str = "exponentiation of zero by negative power";

fn to_num(v: &Value) -> Result<Num, TclError> {
    if let Ok(n) = v.as_int() {
        return Ok(Num::Int(n));
    }
    if let Some(b) = v.as_i128() {
        return Ok(Num::Big(b));
    }
    if let Ok(f) = v.as_double() {
        return Ok(Num::Dbl(f));
    }
    // `as_double` declines an integer past `i128` (keeping the tower exact); it
    // is still a numeric operand — the arbitrary-precision tier.
    if let Some(b) = value_as_bigint(v) {
        return Ok(Num::Huge(b));
    }
    Err(TclError::new(format!(
        "can't use non-numeric string \"{}\" as operand of arithmetic",
        v.to_str()
    )))
}

/// Regenerate the canonical numeric string rep of an `expr` result
/// (C `INST_TRY_CVT_TO_NUMERIC`): a numeric value is rebuilt from its parsed
/// form so its string matches the *number* rather than how it was written
/// (`1e3` → `1000.0`, `0xff` → `255`, `0001` → `1`, `5.` → `5.0`); a bare `NaN`
/// is a domain error (a NaN is not a usable result, only `isnan`'s input); and
/// anything non-numeric (`expr {1 ? "big" : "x"}`) passes through unchanged.
pub(crate) fn cvt_to_numeric(v: Value) -> Result<Value, TclError> {
    if let Ok(n) = v.as_int() {
        return Ok(Value::int(n));
    }
    if let Some(b) = v.as_i128() {
        return Ok(int_value(b));
    }
    // An integer past `i128` keeps its exact (bignum) value, canonicalised to its
    // decimal string, rather than degrading to a lossy `double`.
    if let Some(b) = value_as_bigint(&v) {
        return Ok(big_value(&b));
    }
    if let Ok(d) = v.as_double() {
        return Ok(Value::double(d));
    }
    // `as_double` rejects a bare `NaN`; as a whole-expression result it is a
    // domain error, not a value that survives.
    if matches!(
        number::parse_whole(v.to_str().trim()),
        Some(Number::Nan { .. })
    ) {
        return Err(TclError::new("domain error: argument not in valid range"));
    }
    Ok(v)
}

/// Like [`to_num`] but reports the C Tcl 9 operand-specific message
/// (`cannot use non-numeric string "x" as left operand of "+"`). `side` is
/// `"left"` / `"right"`.
fn to_num_operand(v: &Value, side: &str, op: BinOp) -> Result<Num, TclError> {
    if let Ok(n) = v.as_int() {
        return Ok(Num::Int(n));
    }
    if let Some(b) = v.as_i128() {
        return Ok(Num::Big(b));
    }
    if let Ok(f) = v.as_double() {
        return Ok(Num::Dbl(f));
    }
    // An integer past `i128` is a numeric operand (the arbitrary-precision
    // tier), not an operand-type error.
    if let Some(b) = value_as_bigint(v) {
        return Ok(Num::Huge(b));
    }
    // A non-number that *is* a multi-element list gets C's distinct wording,
    // which `IllegalExprOperandType` applies to binary operators exactly as to
    // unary ones — `ord` is `"left "`/`"right "` there.
    let s = v.to_str();
    if is_list_operand(&s) {
        return Err(TclError::new(format!(
            "cannot use a list as {side} operand of \"{}\"",
            op.as_str()
        )));
    }
    // `nan` (and `±NaN`) is a valid floating-point *value* that simply cannot be
    // an arithmetic operand — C words that differently from a non-number string.
    let kind = if matches!(number::parse_whole(s.trim()), Some(Number::Nan { .. })) {
        "floating-point value"
    } else {
        "string"
    };
    Err(TclError::new(format!(
        "cannot use non-numeric {kind} \"{s}\" as {side} operand of \"{}\"",
        op.as_str()
    )))
}

/// C `IllegalExprOperandType`'s list branch (`tclExecute.c:9089-9107`): a value
/// that is not a number but *is* a well-formed list of more than one element (or
/// a non-empty dict) is reported as `cannot use a list as …operand of "…"` with
/// errorCode `ARITH DOMAIN list`, rather than as a non-numeric string. Shared by
/// the unary and binary operand-error builders so the two agree.
fn is_list_operand(s: &str) -> bool {
    tcl_syntax::list::max_list_length(s) > 1 && tcl_syntax::list::split_list(s).is_ok()
}

fn divzero() -> TclError {
    TclError::with_error_code(DIVZERO_MSG, format!("ARITH DIVZERO {{{DIVZERO_MSG}}}"))
}

/// C's integer divide-by-zero message and `-errorcode`
/// (`tcl9.0.4/generic/tclExecute.c`'s `divideByZero` label).
const DIVZERO_MSG: &str = "divide by zero";

/// The C `IllegalExprOperandType` message for a *unary* operator whose operand
/// cannot be used: `cannot use <desc> "<v>" as operand of "<op>"`. `<desc>` is
/// `floating-point value` (a double handed to `~`), `non-numeric floating-point
/// value` (NaN), `a list` (a multi-element list — phrased without quotes), or
/// `non-numeric string`. (`errorCode ARITH DOMAIN <desc>` is not threaded here;
/// the VM does not yet set arith error codes — same as `divide by zero`.)
fn unary_operand_err(v: &Value, op: &str) -> TclError {
    let s = v.to_str();
    if is_list_operand(&s) {
        return TclError::new(format!("cannot use a list as operand of \"{op}\""));
    }
    let desc = match number::parse_whole(s.trim()) {
        Some(Number::Double(_)) => "floating-point value",
        Some(Number::Nan { .. }) => "non-numeric floating-point value",
        _ => "non-numeric string",
    };
    TclError::new(format!("cannot use {desc} \"{s}\" as operand of \"{op}\""))
}

/// Floored integer division (Tcl `/`: rounds toward negative infinity) — the
/// `i128` fast-tier mirror of `tcl_syntax::number_tower::int_div`. The orphan
/// rule forbids implementing the tower's `BigIntOps` for a primitive in this
/// crate, so the fast tier hand-rolls the same floor rule; the bignum tier
/// ([`big_arith`]) calls the tower directly.
fn fdiv(x: i128, y: i128) -> i128 {
    let q = x.wrapping_div(y);
    let r = x.wrapping_rem(y);
    if r != 0 && ((r < 0) != (y < 0)) {
        q - 1
    } else {
        q
    }
}

/// Floored integer modulo (Tcl `%`: result takes the sign of the divisor) —
/// the `i128` fast-tier mirror of `tcl_syntax::number_tower::int_mod`, kept
/// hand-rolled for the same orphan-rule reason as [`fdiv`].
fn fmod_i(x: i128, y: i128) -> i128 {
    let r = x.wrapping_rem(y);
    if r != 0 && ((r < 0) != (y < 0)) {
        r + y
    } else {
        r
    }
}

/// Promote an `i128` integer operation that overflowed (or can't stay bounded)
/// to the arbitrary-precision path.
fn promote(op: BinOp, x: i128, y: i128) -> Result<Value, TclError> {
    big_arith(op, &BigInt::from(x), &BigInt::from(y))
}

/// Integer arithmetic in `i128`, narrowing the result to a wide when it fits and
/// **promoting to arbitrary precision on overflow** rather than wrapping: `2**70`
/// / `9223372036854775807 + 1` fit `i128`; `2**200` / a product past `2^127`
/// promote to a bignum, matching tclsh. `i64`-range operands
/// and results are unchanged.
fn int_arith(op: BinOp, x: i128, y: i128) -> Result<Value, TclError> {
    use BinOp::{Add, BitAnd, BitOr, BitXor, Div, LShift, Mod, Mul, Pow, RShift, Sub};
    let r = match op {
        Add => match x.checked_add(y) {
            Some(r) => r,
            None => return promote(op, x, y),
        },
        Sub => match x.checked_sub(y) {
            Some(r) => r,
            None => return promote(op, x, y),
        },
        Mul => match x.checked_mul(y) {
            Some(r) => r,
            None => return promote(op, x, y),
        },
        Div => {
            if y == 0 {
                return Err(divzero());
            }
            fdiv(x, y)
        }
        Mod => {
            if y == 0 {
                return Err(divzero());
            }
            fmod_i(x, y)
        }
        // Powers grow fast; let the bignum path compute then narrow if it fits.
        Pow => return promote(op, x, y),
        LShift => {
            if y < 0 {
                return Err(TclError::new("negative shift argument"));
            }
            // Fast path only when the shift stays within `i128`; else promote.
            match u32::try_from(y)
                .ok()
                .filter(|&s| s < 127)
                .and_then(|s| x.checked_shl(s).filter(|r| r >> s == x))
            {
                Some(r) => r,
                None => return promote(op, x, y),
            }
        }
        RShift => {
            if y < 0 {
                return Err(TclError::new("negative shift argument"));
            }
            if y >= 128 {
                if x < 0 { -1 } else { 0 }
            } else {
                x >> u32::try_from(y).unwrap_or(0)
            }
        }
        BitAnd => x & y,
        BitOr => x | y,
        BitXor => x ^ y,
        _ => return Err(TclError::new("unsupported integer operator")),
    };
    Ok(int_value(r))
}

fn dbl_arith(op: BinOp, x: f64, y: f64) -> Result<Value, TclError> {
    use BinOp::{Add, Div, Mul, Pow, Sub};
    let r = match op {
        Add => x + y,
        Sub => x - y,
        Mul => x * y,
        Div => x / y,
        Pow => {
            // C raises the domain error before computing (`tclExecute.c`
            // EXPONENT_OF_ZERO): `0.0 ** -1` is an error, never Inf.
            if x == 0.0 && y < 0.0 {
                return Err(TclError::with_error_code(
                    ZERO_POWER_MSG,
                    format!("ARITH DOMAIN {{{ZERO_POWER_MSG}}}"),
                ));
            }
            x.powf(y)
        }
        _ => {
            return Err(TclError::new(
                "can't use floating-point value as operand of this operator",
            ));
        }
    };
    // A double op that produces NaN from non-NaN operands (`0.0/0.0`,
    // `Inf - Inf`, `Inf/Inf`) is a domain error in C (`tclExecute.c` checks the
    // result via `TclExprFloatError`), not a silent `NaN`. A NaN operand can't
    // reach here — `to_num_operand` rejects it with its own message — so an
    // operand check is belt-and-braces. (The `ARITH DOMAIN` errorCode is not
    // threaded, as noted for the operand errors above.)
    if r.is_nan() && !x.is_nan() && !y.is_nan() {
        return Err(TclError::new("domain error: argument not in valid range"));
    }
    Ok(Value::double(r))
}

/// The integer-only binary operators: a (valid) floating-point operand is itself
/// the error (`cannot use floating-point value "x" as <side> operand of "OP"`),
/// rather than routing to the double path.
fn is_int_only(op: BinOp) -> bool {
    use BinOp::{BitAnd, BitOr, BitXor, LShift, Mod, RShift};
    matches!(op, BitAnd | BitOr | BitXor | LShift | RShift | Mod)
}

fn float_operand_err(v: &Value, side: &str, op: BinOp) -> TclError {
    TclError::new(format!(
        "cannot use floating-point value \"{}\" as {side} operand of \"{}\"",
        v.to_str(),
        op.as_str()
    ))
}

/// Apply an arithmetic / bitwise / shift binary operator to two values.
pub fn arith(op: BinOp, a: &Value, b: &Value) -> Result<Value, TclError> {
    let x = to_num_operand(a, "left", op)?;
    let y = to_num_operand(b, "right", op)?;
    // Fast integer path: both operands fit `i128` (promotes to bignum on overflow).
    if let (Some(xi), Some(yi)) = (num_i128(&x), num_i128(&y)) {
        return int_arith(op, xi, yi);
    }
    // Not both `i128`-fit. When *both* are integers (an operand already past
    // `i128`), stay exact via the arbitrary-precision path.
    if let (Some(xb), Some(yb)) = (num_as_bigint(&x), num_as_bigint(&y)) {
        return big_arith(op, &xb, &yb);
    }
    // At least one operand is a genuine float.
    if is_int_only(op) {
        // An integer-only operator with a float operand: that operand is the
        // error (a huge-integer operand took the bignum path above). Name the
        // offending side (left wins if both are floats, matching C's order).
        if matches!(x, Num::Dbl(_)) {
            Err(float_operand_err(a, "left", op))
        } else {
            Err(float_operand_err(b, "right", op))
        }
    } else {
        // Float contagion: the integer operand — bignum included — converts to
        // its nearest double (`num_f`), then the operation is pure `f64`
        // (tclsh: `2**200 + 1.5` is `1.6069380442589903e+60`, not an error).
        dbl_arith(op, num_f(&x), num_f(&y))
    }
}

/// Exact bignum-vs-double comparison for integers past `i128`: compare integer
/// parts as bignums, then let a non-zero fraction break a tie. NaN is
/// unordered.
fn cmp_bigint_double(w: &BigInt, d: f64) -> NumericCompare {
    use NumericCompare::{Ordered, Unordered};
    if d.is_nan() {
        return Unordered;
    }
    if d == f64::INFINITY {
        return Ordered(Ordering::Less);
    }
    if d == f64::NEG_INFINITY {
        return Ordered(Ordering::Greater);
    }
    let truncated = d.trunc();
    // `from_f64` on a finite integral double is exact; `None` is unreachable
    // for the finite values that get here, but degrade to "unordered" (which
    // callers treat conservatively) rather than panic.
    let Some(int_part) = BigInt::from_f64(truncated) else {
        return Unordered;
    };
    Ordered(match w.cmp(&int_part) {
        Ordering::Equal => {
            let fraction = d - truncated;
            if fraction > 0.0 {
                Ordering::Less
            } else if fraction < 0.0 {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        }
        unequal => unequal,
    })
}

/// A comparison operand classified across the full numeric tower. Distinct
/// from [`Num`] (the arithmetic coercion) because comparison must keep
/// integers past `i128` exact and must classify NaN as a *number* — both of
/// which the arithmetic path deliberately rejects.
enum CmpNum {
    /// Any integer that fits `i128` (wides included).
    Int(i128),
    /// An integer past `i128`, exact.
    Big(BigInt),
    /// A double — possibly NaN.
    Dbl(f64),
}

/// Classify a comparison operand, or `None` for a non-numeric value.
fn cmp_num_of(v: &Value) -> Option<CmpNum> {
    if let Ok(n) = v.as_int() {
        return Some(CmpNum::Int(i128::from(n)));
    }
    if let Some(b) = v.as_i128() {
        return Some(CmpNum::Int(b));
    }
    // Typed doubles (including a typed NaN) and ordinary numeric strings.
    if let Ok(d) = v.as_double() {
        return Some(CmpNum::Dbl(d));
    }
    // `as_double` rejects integers past `i128` and the NaN spellings; both
    // are numeric for comparison purposes (Tcl's NaN rule).
    match number::parse_whole(v.to_str().trim()) {
        Some(Number::Nan { .. }) => Some(CmpNum::Dbl(f64::NAN)),
        Some(Number::Big { .. }) => value_as_bigint(v).map(CmpNum::Big),
        _ => None,
    }
}

/// The one numeric-comparison path over *values*: `None` when either operand is
/// non-numeric (the caller falls back to a string comparison), otherwise the
/// exact outcome across the whole tower — wide/`i128` exactly, integer-vs-double
/// exactly, integers past `i128` as bignums, NaN unordered.
/// Shared by [`compare`] and the [`ExprEval`] adapter so the two cannot drift.
fn compare_values_numeric(a: &Value, b: &Value) -> Option<NumericCompare> {
    use NumericCompare::{Ordered, Unordered};
    let reversed = |outcome: NumericCompare| match outcome {
        Ordered(ord) => Ordered(ord.reverse()),
        Unordered => Unordered,
    };
    Some(match (cmp_num_of(a)?, cmp_num_of(b)?) {
        (CmpNum::Int(p), CmpNum::Int(q)) => Ordered(p.cmp(&q)),
        (CmpNum::Int(p), CmpNum::Dbl(d)) => {
            NumericCompare::from_partial(number::compare_int_double(p, d))
        }
        (CmpNum::Dbl(d), CmpNum::Int(q)) => reversed(NumericCompare::from_partial(
            number::compare_int_double(q, d),
        )),
        (CmpNum::Dbl(p), CmpNum::Dbl(q)) => NumericCompare::from_partial(p.partial_cmp(&q)),
        (CmpNum::Big(p), CmpNum::Big(q)) => Ordered(p.cmp(&q)),
        (CmpNum::Big(p), CmpNum::Int(q)) => Ordered(p.cmp(&BigInt::from(q))),
        (CmpNum::Int(p), CmpNum::Big(q)) => Ordered(BigInt::from(p).cmp(&q)),
        (CmpNum::Big(p), CmpNum::Dbl(d)) => cmp_bigint_double(&p, d),
        (CmpNum::Dbl(d), CmpNum::Big(q)) => reversed(cmp_bigint_double(&q, d)),
    })
}

/// Apply a comparison operator, returning the boolean result. Numeric when both
/// operands look numeric (or always-string for the `STR_*` variants), else a
/// string comparison — Tcl's `==`/`<`… rule.
pub fn compare(op: BinOp, a: &Value, b: &Value) -> Result<bool, TclError> {
    use BinOp::{Eq, Ge, Gt, Le, Lt, Ne, StrEq, StrGe, StrGt, StrLe, StrLt, StrNe};
    use NumericCompare::{Ordered, Unordered};
    let str_ord = || Ordered((*a.to_str()).cmp(&b.to_str()));
    let outcome = match op {
        StrEq | StrNe | StrLt | StrLe | StrGt | StrGe => str_ord(),
        // Numeric when both operands are numbers, else string — with a NaN
        // operand unordered: unequal to everything, ordered before/after
        // nothing (C Tcl's "NaN arg" rule in tclExecute.c).
        _ => compare_values_numeric(a, b).unwrap_or_else(str_ord),
    };
    Ok(match (op, outcome) {
        (Ne | StrNe, Unordered) => true,
        (_, Unordered) => false,
        (Eq | StrEq, Ordered(ord)) => ord.is_eq(),
        (Ne | StrNe, Ordered(ord)) => ord.is_ne(),
        (Lt | StrLt, Ordered(ord)) => ord.is_lt(),
        (Le | StrLe, Ordered(ord)) => ord.is_le(),
        (Gt | StrGt, Ordered(ord)) => ord.is_gt(),
        (Ge | StrGe, Ordered(ord)) => ord.is_ge(),
        _ => return Err(TclError::new("unsupported comparison operator")),
    })
}

/// Apply a unary operator.
pub fn unary(op: UnaryOp, v: &Value) -> Result<Value, TclError> {
    use UnaryOp::{BitNot, Neg, Not, Pos, WordNot};
    match op {
        // `to_num` promotes an out-of-wide literal to `Big` (then `Huge`), so
        // `-2^63` — and any magnitude beyond — negates exactly: `int_value` /
        // `big_value` narrow back to a wide when the result re-fits. A
        // non-numeric operand is the C operand-type error (`as operand of "-"`).
        Neg => match to_num(v) {
            // `-i64::MIN` does not fit a wide — promote through the i128
            // tier exactly (a *computed* MIN reaches this arm as `Int`).
            Ok(Num::Int(n)) => Ok(int_value(-i128::from(n))),
            Ok(Num::Big(b)) => Ok(int_value(b.wrapping_neg())),
            // An integer past `i128` negates exactly in arbitrary precision,
            // never as a lossy float.
            Ok(Num::Huge(b)) => Ok(big_value(&-b)),
            Ok(Num::Dbl(f)) => Ok(Value::double(-f)),
            Err(_) => Err(unary_operand_err(v, "-")),
        },
        Pos => match to_num(v) {
            Ok(Num::Int(n)) => Ok(Value::int(n)),
            Ok(Num::Big(b)) => Ok(int_value(b)),
            Ok(Num::Huge(b)) => Ok(big_value(&b)),
            Ok(Num::Dbl(f)) => Ok(Value::double(f)),
            Err(_) => Err(unary_operand_err(v, "+")),
        },
        // `~` needs an integer; a double is a "floating-point value" operand
        // error, a non-number a "non-numeric string" one.
        BitNot => match to_num(v) {
            Ok(Num::Int(n)) => Ok(Value::int(!n)),
            Ok(Num::Big(b)) => Ok(int_value(!b)),
            // An integer past `i128` bit-complements exactly (`~x == -x-1`).
            Ok(Num::Huge(b)) => Ok(big_value(&!b)),
            Ok(Num::Dbl(_)) | Err(_) => Err(unary_operand_err(v, "~")),
        },
        // `!` accepts any boolean (incl. numbers and the boolean words); a NaN or
        // non-numeric non-boolean is the operand error (not "expected boolean").
        // The iRules dialect's `not` is the word spelling of the same operator
        // (`Op::IRULE_WORD_NOT`), so it shares the coercion and only differs in
        // the spelling it reports for a bad operand.
        Not | WordNot => {
            let spelling = if matches!(op, WordNot) { "not" } else { "!" };
            if matches!(
                number::parse_whole(v.to_str().trim()),
                Some(Number::Nan { .. })
            ) {
                return Err(unary_operand_err(v, spelling));
            }
            match v.as_bool() {
                Ok(b) => Ok(Value::bool(!b)),
                Err(_) => Err(unary_operand_err(v, spelling)),
            }
        }
    }
}

/// Apply an iRules-dialect binary operator to two already-evaluated operands
/// (`left` is the subject, `right` the needle/pattern/right-hand side — the
/// order `Op::from_binop` codegen pushes them in).
///
/// One home for the dialect operator semantics, shared by the `IRULE_*` opcodes
/// and available to any tree-walking consumer that gains dialect support (the
/// shared walker routes them to `ExprOps::binary_other`). The string tests reuse
/// the same helpers the equivalent core commands do — `string match`'s glob
/// matcher, the `regexp` ARE engine, and this module's string comparison — so
/// the dialect operators cannot drift from `[string match]` / `[regexp]` /
/// `[string equal]`.
pub(crate) fn irule_binary(op: BinOp, left: &Value, right: &Value) -> Result<Value, TclError> {
    use BinOp::{
        Contains, EndsWith, MatchesGlob, MatchesRegex, StartsWith, StrEquals, WordAnd, WordOr,
    };
    // `and`/`or` are boolean tests, not string ones — they never render an
    // operand.
    match op {
        WordAnd => return word_and(left, right).map(Value::bool),
        WordOr => return word_or(left, right).map(Value::bool),
        _ => {}
    }
    let (subject, operand) = (left.to_str(), right.to_str());
    let truth = match op {
        Contains => subject.contains(&*operand),
        StartsWith => subject.starts_with(&*operand),
        EndsWith => subject.ends_with(&*operand),
        // `equals` is the word spelling of `eq`: always a string comparison.
        StrEquals => compare(BinOp::StrEq, left, right)?,
        // Case-sensitive `string match` / `regexp` — the dialect operators have
        // no `-nocase` form.
        MatchesGlob => tcl_syntax::glob::string_match(&operand, &subject),
        MatchesRegex => {
            crate::cmd_regexp::regexp_matches(&operand, &subject, false).map_err(TclError::new)?
        }
        _ => return Err(TclError::new("unsupported operator")),
    };
    Ok(Value::bool(truth))
}

/// The iRules `and` word operator over two already-evaluated operands.
///
/// The shared tree-walk coerces the right operand only when the left does not
/// already decide the result; these reproduce that with both operands in hand
/// (as the opcode form has them), so a non-boolean right operand is an error in
/// exactly the cases the walker treats as one.
fn word_and(left: &Value, right: &Value) -> Result<bool, TclError> {
    if left.as_bool()? {
        right.as_bool()
    } else {
        Ok(false)
    }
}

/// The iRules `or` word operator — see [`word_and`].
fn word_or(left: &Value, right: &Value) -> Result<bool, TclError> {
    if left.as_bool()? {
        Ok(true)
    } else {
        right.as_bool()
    }
}

/// An [`ExprOps`] adapter that evaluates an expression AST against the VM
/// (resolving `$var` / `[cmd]` / math functions through it), reusing the shared
/// `tcl-syntax` expr walker.
pub struct ExprEval<'a> {
    /// The VM the expression resolves variables/commands against.
    pub vm: &'a mut Vm,
}

impl ExprOps for ExprEval<'_> {
    type Value = Value;
    type Error = TclError;

    fn literal(&mut self, text: &str) -> Result<Value, TclError> {
        Ok(match number::parse_whole(text) {
            Some(Number::Int(n)) => Value::int(n),
            Some(Number::Double(f)) => Value::double(f),
            _ => Value::string(text),
        })
    }

    fn string(&mut self, inner: &str) -> Result<Value, TclError> {
        // A `"…"` expr operand is a double-quoted word: substitute `$var` /
        // `[cmd]` / backslashes (the runtime-`expr` analogue of the compiler's
        // `emit_expr_string`), so `expr {"item $i"}` is `item 0`, not `item $i`.
        let s = crate::subst::subst_command(self.vm, inner, true, true, true)?;
        Ok(Value::string(s))
    }

    fn var(&mut self, name: &str) -> Result<Value, TclError> {
        if let Err(c) = self.vm.fire_var_traces(name, "read") {
            return Err(TclError::new(c.result.to_str().to_string()));
        }
        self.vm
            .get_var(name)
            .ok_or_else(|| TclError::new(format!("can't read \"{name}\": no such variable")))
    }

    fn command(&mut self, script: &str) -> Result<Value, TclError> {
        // A command substitution inside an expression yields the command's result
        // *only* when it completes normally; otherwise the completion must
        // propagate, not be taken as the value — otherwise `expr {[error msg]}`
        // would silently evaluate to the string "msg" (if-5.2). An error becomes a
        // plain `TclError`; a `break`/`continue`/`return` escaping the substitution
        // carries its code so the enclosing construct sees it.
        let c = self.vm.eval_source(script)?;
        match c.code {
            Code::Ok => Ok(c.result),
            Code::Error => Err(TclError::new(c.result.to_str().to_string())),
            code => Err(TclError::with_code(c.result.to_str().to_string(), code)),
        }
    }

    fn call(&mut self, function: &str, args: Vec<Value>) -> Result<Value, TclError> {
        let surface = tcl_registry::expr_surface::RuntimeExprSurface::for_tcl_version(
            self.vm.runtime_version(),
        );
        match surface.math_function_call_target(function) {
            tcl_registry::expr_surface::MathFunctionCallTarget::FixedBuiltin(spec) => {
                // Tcl 8.4 predates TIP 232: builtin expressions use the fixed
                // function table selected by the registry, never a script
                // command that happens to share the `tcl::mathfunc` spelling.
                let c = self.vm.invoke_fixed_math_builtin(spec, &args);
                return match c.code {
                    Code::Ok => Ok(c.result),
                    Code::Error => Err(TclError::new(c.result.to_str().to_string())),
                    code => Err(TclError::with_code(c.result.to_str().to_string(), code)),
                };
            }
            tcl_registry::expr_surface::MathFunctionCallTarget::FixedTableMiss => {
                return Err(TclError::new(format!(
                    "unknown math function \"{function}\""
                )));
            }
            tcl_registry::expr_surface::MathFunctionCallTarget::CommandTable => {}
        }

        // TIP 232: a math function is an ordinary command, so a user
        // `proc tcl::mathfunc::f {…} {…}` (the canonical custom-function
        // mechanism) must dispatch exactly like a builtin one — full command
        // dispatch, not builtin-only. The name stays *relative*, so
        // resolution is current-namespace-first (a namespace-local
        // `tcl::mathfunc::f` shadows the global; tclsh-pinned by the
        // mathfunc conformance vectors).
        let name = tcl_registry::mathfunc::qualified_name(function)
            .trim_start_matches("::")
            .to_owned();
        if self.vm.lookup_command(&name).is_none() {
            // C reports the command miss, not a special math-function error
            // (tclsh 8.6.16 / 9.0.4: `expr {frobnicate(1)}` →
            // `invalid command name "tcl::mathfunc::frobnicate"`).
            return Err(TclError::with_error_code(
                format!("invalid command name \"{name}\""),
                format!("TCL LOOKUP COMMAND {name}"),
            ));
        }
        let c = self.vm.invoke_command(&name, &args);
        match c.code {
            Code::Ok => Ok(c.result),
            Code::Error => Err(TclError::new(c.result.to_str().to_string())),
            code => Err(TclError::with_code(c.result.to_str().to_string(), code)),
        }
    }

    fn arith(&mut self, op: BinOp, l: Value, r: Value) -> Result<Value, TclError> {
        arith(op, &l, &r)
    }

    fn unary(&mut self, op: UnaryOp, v: Value) -> Result<Value, TclError> {
        unary(op, &v)
    }

    fn compare_numeric(&mut self, l: &Value, r: &Value) -> Option<NumericCompare> {
        compare_values_numeric(l, r)
    }

    fn compare_string(&mut self, l: &Value, r: &Value) -> Ordering {
        (*l.to_str()).cmp(&r.to_str())
    }

    fn in_list(&mut self, needle: &Value, list: &Value) -> Result<bool, TclError> {
        let items = list.as_list()?;
        let n = needle.to_str();
        Ok(items.iter().any(|v| *v.to_str() == *n))
    }

    fn to_bool(&mut self, v: &Value) -> Result<bool, TclError> {
        v.as_bool()
    }

    fn bool_value(&mut self, b: bool) -> Value {
        Value::bool(b)
    }

    fn unsupported(&mut self, what: &str) -> TclError {
        TclError::new(what.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::Command;
    use crate::interp::{Vm, ok};
    use tcl_runtime_api::Completion;

    #[test]
    fn tcl84_expr_uses_the_registry_selected_fixed_math_table() {
        fn replacement(_vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
            ok(Value::int(99))
        }

        let mut vm = Vm::new();
        // Replace the normal command-table entry after construction. The
        // preserved fixed table is intentionally not changed: Tcl 8.4's expr
        // dispatch did not use a Tcl command name at all.
        vm.register_command("tcl::mathfunc::sqrt", Command::Builtin(replacement));
        vm.set_runtime_version(tcl_dialect::TclVersion::V8_4);

        // TP: a fixed-table builtin remains executable.
        assert_eq!(vm.eval_expr("sqrt(4)").unwrap().to_str().as_ref(), "2.0");
        // FN: an absent fixed-table entry has C Tcl's distinct diagnostic.
        assert_eq!(
            vm.eval_expr("user_supplied(1)").unwrap_err().message,
            "unknown math function \"user_supplied\""
        );
        // TN: the default command wrapper itself is unavailable before TIP 232.
        assert_eq!(
            vm.invoke_command("::tcl::mathfunc::sqrt", &[Value::int(4)])
                .code,
            Code::Error
        );

        vm.set_runtime_version(tcl_dialect::TclVersion::V8_5);
        // FP guard: the same replacement becomes the open command-table target
        // once TIP 232 is enabled, rather than retaining the 8.4 shortcut.
        assert_eq!(vm.eval_expr("sqrt(4)").unwrap().to_str().as_ref(), "99");
    }

    /// These boundaries are established below the VM by the shared expression
    /// numeral scanner. The parse success and error shape are direct tclsh
    /// 8.6.17 / 9.0.3 oracle rows; if a radix run or completed NaN payload
    /// fuses with its suffix, `eval_expr` reaches a different AST or misses
    /// C's bareword. The VM's comparison coercion is intentionally not the
    /// lexeme-boundary assertion here.
    #[test]
    fn shared_expr_numeral_boundaries_reach_the_vm() {
        let mut vm = Vm::new();
        vm.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        assert!(vm.eval_expr("0b1ne 1").is_ok());
        assert!(vm.eval_expr("0xfne 1").is_ok());
        assert!(vm.eval_expr("0xffin {255}").is_ok());

        vm.set_runtime_version(tcl_dialect::TclVersion::V9_0);
        assert!(vm.eval_expr("0xfge 15").is_ok());
        assert!(vm.eval_expr("0d9lt 10").is_ok());
        for source in ["0 + 1_eq", "0 + 12x"] {
            assert!(
                vm.eval_expr(source)
                    .unwrap_err()
                    .message
                    .contains("invalid bareword"),
                "{source}"
            );
        }
        assert!(
            vm.eval_expr("NaN(1)x")
                .unwrap_err()
                .message
                .starts_with("invalid bareword \"x\"")
        );
    }

    #[test]
    fn int_div_mod_floored() {
        // Tcl: -7 / 2 == -4, -7 % 2 == 1
        assert_eq!(
            arith(BinOp::Div, &Value::int(-7), &Value::int(2))
                .unwrap()
                .as_int()
                .unwrap(),
            -4
        );
        assert_eq!(
            arith(BinOp::Mod, &Value::int(-7), &Value::int(2))
                .unwrap()
                .as_int()
                .unwrap(),
            1
        );
    }

    /// The arbitrary-precision integer tower: operations that
    /// overflow `i128` (or whose operands already exceed it) stay exact and match
    /// tclsh 9.0.4, instead of wrapping or degrading to a lossy `double`.
    #[test]
    fn bignum_tower_stays_exact() {
        let big = |op: BinOp, a: &str, b: &str| {
            arith(op, &Value::string(a), &Value::string(b))
                .unwrap()
                .to_str()
                .to_string()
        };
        // (2^64)^2 = 2^128 — a product past i128 promotes to an exact bignum.
        assert_eq!(
            big(BinOp::Mul, "18446744073709551616", "18446744073709551616"),
            "340282366920938463463374607431768211456"
        );
        // Pow, floor div/mod and shifts stay exact.
        assert_eq!(
            arith(BinOp::Pow, &Value::int(2), &Value::int(200))
                .unwrap()
                .to_str()
                .to_string(),
            "1606938044258990275541962092341162602522202993782792835301376"
        );
        assert_eq!(
            big(BinOp::Div, "1000000000000000000000000000000", "3"),
            "333333333333333333333333333333"
        );
        assert_eq!(big(BinOp::Mod, "1000000000000000000000000000000", "7"), "1");
        assert_eq!(
            arith(BinOp::LShift, &Value::int(1), &Value::int(100))
                .unwrap()
                .to_str()
                .to_string(),
            "1267650600228229401496703205376"
        );
        // i64/i128-overflowing sums promote instead of wrapping.
        assert_eq!(
            arith(BinOp::Add, &Value::int(i64::MAX), &Value::int(1))
                .unwrap()
                .to_str()
                .to_string(),
            "9223372036854775808"
        );
        // The shared-tower oracle rows (tclsh 8.6/9.0): floor div/mod at and
        // beyond the wide boundary, the shift width-collapse, and `1 << 63`
        // crossing into bignum.
        assert_eq!(big(BinOp::Mod, "18446744073709551616", "7"), "2");
        assert_eq!(
            big(BinOp::Div, "18446744073709551616", "-3"),
            "-6148914691236517206"
        );
        assert_eq!(big(BinOp::RShift, "18446744073709551616", "64"), "1");
        assert_eq!(big(BinOp::RShift, "-18446744073709551616", "200"), "-1");
        assert_eq!(
            arith(BinOp::LShift, &Value::int(1), &Value::int(63))
                .unwrap()
                .to_str()
                .to_string(),
            "9223372036854775808"
        );
        // Exact bignum comparison — an `f64` would collapse `2^100` and `2^100+1`.
        let (a, b) = (
            "1267650600228229401496703205376",
            "1267650600228229401496703205377",
        );
        assert!(compare(BinOp::Lt, &Value::string(a), &Value::string(b)).unwrap());
        assert!(!compare(BinOp::Eq, &Value::string(a), &Value::string(b)).unwrap());
        // Unary negate / bit-not of a bignum stay exact; `0 ** -n` errors like C.
        assert_eq!(
            unary(UnaryOp::Neg, &Value::string(a))
                .unwrap()
                .to_str()
                .to_string(),
            format!("-{a}")
        );
        assert_eq!(
            unary(UnaryOp::BitNot, &Value::string(a))
                .unwrap()
                .to_str()
                .to_string(),
            format!("-{b}")
        );
        assert_eq!(
            unary(UnaryOp::BitNot, &Value::string("18446744073709551616"))
                .unwrap()
                .to_str()
                .to_string(),
            "-18446744073709551617"
        );
        assert!(arith(BinOp::Pow, &Value::int(0), &Value::int(-1)).is_err());
    }

    /// Operands already past `i128` are numeric (`Num::Huge`), not the
    /// non-numeric-string operand error: they reach the arbitrary-precision
    /// path in `arith`/`unary` and stay exact. All rows pinned to tclsh
    /// 8.6.14 (`2**200` is the 61-digit 16069…376).
    #[test]
    fn operands_past_i128_reach_the_bignum_path() {
        const P200: &str = "1606938044258990275541962092341162602522202993782792835301376";
        let big = |op: BinOp, a: &str, b: &str| {
            arith(op, &Value::string(a), &Value::string(b))
                .unwrap()
                .to_str()
                .to_string()
        };
        // tclsh: 2**200 + 1 (the divergence that motivated the gap fix).
        assert_eq!(
            big(BinOp::Add, P200, "1"),
            "1606938044258990275541962092341162602522202993782792835301377"
        );
        // tclsh: floor div/mod, shifts and bit ops over a beyond-i128 operand.
        assert_eq!(big(BinOp::Mod, P200, "7"), "4");
        assert_eq!(
            big(BinOp::Div, P200, "-3"),
            "-535646014752996758513987364113720867507400997927597611767126"
        );
        assert_eq!(big(BinOp::RShift, P200, "137"), "9223372036854775808");
        assert_eq!(
            big(BinOp::RShift, &format!("-{P200}"), "500"),
            "-1",
            "sign collapse past the width"
        );
        assert_eq!(big(BinOp::BitAnd, P200, "255"), "0");
        assert_eq!(
            big(BinOp::BitOr, P200, "1"),
            "1606938044258990275541962092341162602522202993782792835301377"
        );
        // Unary over a beyond-i128 operand: exact negate / complement / pass.
        assert_eq!(
            unary(UnaryOp::Neg, &Value::string(P200))
                .unwrap()
                .to_str()
                .to_string(),
            format!("-{P200}")
        );
        assert_eq!(
            unary(UnaryOp::BitNot, &Value::string(P200))
                .unwrap()
                .to_str()
                .to_string(),
            "-1606938044258990275541962092341162602522202993782792835301377"
        );
        assert_eq!(
            unary(UnaryOp::Pos, &Value::string(P200))
                .unwrap()
                .to_str()
                .to_string(),
            P200
        );
        // Error surfaces on the bignum path keep C's message text.
        let div0 = arith(BinOp::Div, &Value::string(P200), &Value::int(0)).unwrap_err();
        assert_eq!(div0.message, "divide by zero");
        let mod0 = arith(BinOp::Mod, &Value::string(P200), &Value::int(0)).unwrap_err();
        assert_eq!(mod0.message, "divide by zero");
    }

    /// The `**` and shift guards of the shared tower, with C's message text:
    /// the exponent limit is `MAX_EXPONENT` (2^28 - 1 — tclsh errors at
    /// `2**268435456` where the old code computed a 33 MB number), a bignum
    /// exponent keeps the `0`/`±1` collapses, and a left-shift count past
    /// `INT_MAX` is C's overflow error rather than an astronomic attempt.
    #[test]
    fn pow_and_shift_guards_match_c() {
        let errmsg = |op: BinOp, a: &Value, b: &Value| arith(op, a, b).unwrap_err().message;
        // tclsh: expr {2**268435456} → exponent too large (268435455 is legal).
        assert_eq!(
            errmsg(BinOp::Pow, &Value::int(2), &Value::int(268_435_456)),
            "exponent too large"
        );
        assert_eq!(
            errmsg(BinOp::Pow, &Value::int(2), &Value::int(5_000_000_000)),
            "exponent too large"
        );
        // tclsh: a beyond-i64 exponent still collapses for |base| <= 1 —
        // parity included — and errors for any other base.
        let huge = "99999999999999999999";
        let pow = |a: i64, b: &str| {
            arith(BinOp::Pow, &Value::int(a), &Value::string(b))
                .unwrap()
                .to_str()
                .to_string()
        };
        assert_eq!(pow(1, huge), "1");
        assert_eq!(pow(-1, huge), "-1"); // odd exponent
        assert_eq!(pow(-1, "99999999999999999998"), "1"); // even exponent
        assert_eq!(pow(0, huge), "0");
        assert_eq!(
            errmsg(BinOp::Pow, &Value::int(2), &Value::string(huge)),
            "exponent too large"
        );
        assert_eq!(
            errmsg(
                BinOp::Pow,
                &Value::int(0),
                &Value::string(format!("-{huge}"))
            ),
            "exponentiation of zero by negative power"
        );
        // tclsh: expr {2 << 4294967296} → integer value too large to
        // represent (count must fit C's int); negative counts keep their
        // error on both tiers.
        assert_eq!(
            errmsg(BinOp::LShift, &Value::int(2), &Value::int(4_294_967_296)),
            "integer value too large to represent"
        );
        assert_eq!(
            errmsg(
                BinOp::LShift,
                &Value::int(2),
                &Value::string("99999999999999999999")
            ),
            "integer value too large to represent"
        );
        assert_eq!(
            errmsg(
                BinOp::RShift,
                &Value::string("18446744073709551616"),
                &Value::int(-1)
            ),
            "negative shift argument"
        );
        // tclsh: a beyond-wide right-shift count collapses to the sign.
        let big = |op: BinOp, a: &str, b: &str| {
            arith(op, &Value::string(a), &Value::string(b))
                .unwrap()
                .to_str()
                .to_string()
        };
        assert_eq!(big(BinOp::RShift, "2", huge), "0");
        assert_eq!(big(BinOp::RShift, "-2", huge), "-1");
    }

    /// Float contagion over bignum operands: C converts the integer with
    /// `TclBignumToDouble` (round-to-nearest-even over the full value) and
    /// computes in `f64` — it does not refuse. Rows pinned to tclsh 8.6.14,
    /// including the tie whose deciding bit sits below the top 64 (2^127 +
    /// 2^74 + 1 must round *up*; a bare 64-bit truncation rounds it down).
    #[test]
    fn bignum_float_contagion_matches_c() {
        let s = |t: &str| Value::string(t);
        let f = |op: BinOp, a: &Value, b: &Value| arith(op, a, b).unwrap().to_str().to_string();
        // tclsh: expr {1.5 + 18446744073709551616} — the *value* is exactly
        // 2^64 (tclsh: entier of it is 18446744073709551616). tclsh 8.6
        // prints it "1.844674407370955e+19", which does not round-trip (it
        // re-parses as 2^64 - 2048); this toolchain's `format_double` is
        // pinned to the shortest *round-tripping* form.
        assert_eq!(
            f(BinOp::Add, &Value::double(1.5), &s("18446744073709551616")),
            "1.8446744073709552e+19"
        );
        // tclsh: expr {2**200 + 1.5} → 1.6069380442589903e+60
        let p200 = "1606938044258990275541962092341162602522202993782792835301376";
        assert_eq!(
            f(BinOp::Add, &s(p200), &Value::double(1.5)),
            "1.6069380442589903e+60"
        );
        // Round-to-nearest-even ties: 2^127+2^74 is exactly halfway (stays
        // even → 2^127), 2^127+2^74+1 is just above it (rounds up).
        assert_eq!(
            f(
                BinOp::Mul,
                &s("170141183460469250621153235194464960512"),
                &Value::double(1.0)
            ),
            "1.7014118346046923e+38"
        );
        assert_eq!(
            f(
                BinOp::Mul,
                &s("170141183460469250621153235194464960513"),
                &Value::double(1.0)
            ),
            "1.7014118346046927e+38"
        );
        // Past the double range the conversion overflows to Inf, as C's
        // `TclBignumToDouble` does (tclsh: expr {1.0 + 10**309} → Inf).
        let p1100 = arith(BinOp::Pow, &Value::int(2), &Value::int(1100)).unwrap();
        assert_eq!(f(BinOp::Add, &p1100, &Value::double(1.0)), "Inf");
        // An integer-only operator still rejects the *float* operand — the
        // bignum side is fine (tclsh 9 wording, sided).
        let e = arith(BinOp::Mod, &s(p200), &Value::double(1.5)).unwrap_err();
        assert_eq!(
            e.message,
            "cannot use floating-point value \"1.5\" as right operand of \"%\""
        );
        let e = arith(BinOp::Mod, &Value::double(1.5), &s(p200)).unwrap_err();
        assert_eq!(
            e.message,
            "cannot use floating-point value \"1.5\" as left operand of \"%\""
        );
    }

    #[test]
    fn mixed_promotes_to_double() {
        assert_eq!(
            &*arith(BinOp::Mul, &Value::int(2), &Value::double(1.5))
                .unwrap()
                .to_str(),
            "3.0"
        );
    }

    #[test]
    fn compare_numeric_then_string() {
        assert!(compare(BinOp::Lt, &Value::string("9"), &Value::string("10")).unwrap());
        assert!(!compare(BinOp::Lt, &Value::string("apple"), &Value::string("Apple")).unwrap());
    }

    #[test]
    fn compare_int_vs_double_is_exact() {
        // tclsh: expr {9007199254740993 == 9007199254740992.0} → 0 — a
        // both-as-f64 comparison would call them equal.
        let big = Value::string("9007199254740993");
        let dbl = Value::double(9_007_199_254_740_992.0);
        assert!(!compare(BinOp::Eq, &big, &dbl).unwrap());
        assert!(compare(BinOp::Gt, &big, &dbl).unwrap());
        assert!(compare(BinOp::Lt, &dbl, &big).unwrap());
        // Integers past i128 (the `CmpNum::Big` tier) against a double —
        // the bignum-vs-double arm: 2¹³⁰ vs 1.5×2¹³⁰-ish.
        let huge = Value::string("1361129467683753853853498429727072845824"); // 2^130
        assert!(compare(BinOp::Gt, &huge, &Value::double(1e39)).unwrap());
        assert!(compare(BinOp::Lt, &huge, &Value::double(1.4e39)).unwrap());
        // Equal-integer-part case in the bignum arm: a double that is exactly
        // 2¹³⁰ (powers of two are representable) compares Equal to the exact
        // decimal spelling of 2¹³⁰.
        assert!(compare(BinOp::Eq, &huge, &Value::double(2f64.powi(130))).unwrap());
    }

    #[test]
    fn compare_nan_is_unordered() {
        // tclsh: `set x NaN` — `$x != 1` → 1; `==`/`<`/`<=`/`>`/`>=` → 0;
        // `$x == $x` → 0. Applies to string-spelled and typed NaN alike.
        for nan in [Value::string("NaN"), Value::double(f64::NAN)] {
            assert!(compare(BinOp::Ne, &nan, &Value::int(1)).unwrap());
            assert!(!compare(BinOp::Eq, &nan, &Value::int(1)).unwrap());
            assert!(!compare(BinOp::Lt, &nan, &Value::int(1)).unwrap());
            assert!(!compare(BinOp::Le, &nan, &Value::int(1)).unwrap());
            assert!(!compare(BinOp::Gt, &nan, &Value::int(1)).unwrap());
            assert!(!compare(BinOp::Ge, &nan, &Value::int(1)).unwrap());
            assert!(!compare(BinOp::Eq, &nan, &nan).unwrap());
            assert!(compare(BinOp::Ne, &nan, &nan).unwrap());
        }
        // The string comparators still see the spelling: `NaN eq NaN` → 1.
        let nan = Value::string("NaN");
        assert!(compare(BinOp::StrEq, &nan, &nan).unwrap());
    }
}
