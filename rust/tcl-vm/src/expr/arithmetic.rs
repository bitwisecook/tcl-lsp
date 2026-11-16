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

//! The numeric tower under every operator: operand coercion (`Num`), `i128`, arbitrary-precision and double arithmetic, and the operand errors.

use super::NumericContext;
use crate::error::TclError;
use crate::value::Value;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{Signed, ToPrimitive, Zero};
use tcl_syntax::expr::{BinOp, errors};
use tcl_syntax::number::Number;
use tcl_syntax::{number, number_tower};

/// A coerced numeric operand. `Big` carries an out-of-`i64` integer that still
/// fits `i128` — the fast integer tier; `Huge` is the arbitrary-precision tier
/// beyond it, so no integer magnitude wraps, errors, or
/// degrades to a lossy `double`.
pub(super) enum Num {
    Int(i64),
    Big(i128),
    Huge(BigInt),
    Dbl(f64),
}

pub(super) fn num_f(n: &Num) -> f64 {
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
    value_as_bigint_with_stages(v, None).ok().flatten()
}

pub(crate) fn value_as_bigint_with_stages(
    v: &Value,
    original: Option<NumericContext<'_>>,
) -> Result<Option<BigInt>, TclError> {
    let reached = v.as_int_with_stages(original);
    if let Some(original) = original {
        original.current_for_value(v)?;
    }
    match reached {
        Ok(n) => return Ok(Some(BigInt::from(n))),
        Err(error @ TclError::Host(_)) => return Err(error),
        Err(TclError::Guest(_)) => {}
    }
    let Some(text) = v.numeric_conversion_optional_text(original)? else {
        return Ok(None);
    };
    let parsed = match Value::numeric_conversion_parse(text.trim(), original) {
        Some(Number::Int(n)) => Some(BigInt::from(n)),
        Some(Number::Big {
            negative,
            radix,
            digits,
        }) => BigInt::parse_bytes(digits.as_bytes(), radix as u32)
            .map(|b| if negative { -b } else { b }),
        _ => None,
    };
    if let Some(original) = original {
        original.current_for_value(v)?;
    }
    Ok(parsed)
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
        // exponent (the domain error) and an exponent past the C limit.
        None if x.is_zero() => Err(zero_to_negative_power()),
        None => Err(TclError::new("exponent too large")),
    }
}

pub(super) fn to_num(v: &Value) -> Result<Num, TclError> {
    if let Some(value) = v.double_representation() {
        return Ok(Num::Dbl(value));
    }
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

/// C's `IllegalExprOperandType`, through the shared owner
/// ([`tcl_syntax::expr::errors`]): the *wording* is a release axis (9.0 names
/// the value and the side and has a list branch; 8.4-8.6 name neither and
/// have no list branch) while the `-errorcode ARITH DOMAIN <description>` is
/// invariant. Emitting the 9.0 wording at every `--tcl-version`, with no
/// `-errorcode` at all, would be wrong on both counts.
///
/// The release comes from the same ambient the numeric grammar already
/// follows ([`errors::ambient_release`]), because these errors are raised
/// from `arith`/`unary`, which bytecode opcodes call with no interpreter in
/// hand.
fn operand_type_err(v: &Value, side: errors::OperandSide, op: &str) -> TclError {
    operand_type_err_for_release(v, side, op, errors::ambient_release())
}

fn operand_type_err_for_release(
    v: &Value,
    side: errors::OperandSide,
    op: &str,
    release: tcl_dialect::TclVersion,
) -> TclError {
    let s = match v.try_to_str() {
        Ok(text) => text,
        Err(error) => return error.into(),
    };
    let desc = if is_list_operand(&s) {
        errors::OperandDesc::List
    } else {
        match number::parse_whole(s.trim()) {
            Some(Number::Double(_)) => errors::OperandDesc::FloatingPointValue,
            Some(Number::Nan { .. }) => errors::OperandDesc::NonNumericFloatingPointValue,
            _ => errors::OperandDesc::NonNumericString,
        }
    };
    TclError::with_error_code(
        errors::illegal_operand_message(desc, &s, side, op, release),
        errors::illegal_operand_error_code(desc, &s, release),
    )
}

/// The boolean-context error C raises for a value that is neither a number
/// nor a Tcl boolean word (`expected boolean value but got "x"`,
/// `-errorcode TCL VALUE NUMBER`) or for a NaN there
/// (`floating point value is Not a Number`, `TCL VALUE DOUBLE NAN`). Without
/// this, both would default to `NONE`.
#[cfg(test)]
pub(super) fn boolean_context_err(message: Vec<u8>) -> TclError {
    let code = if message == errors::NAN_MESSAGE.as_bytes() {
        errors::NAN_CODE
    } else {
        errors::BOOLEAN_OPERAND_CODE
    };
    TclError::with_error_code(message, code)
}

/// C `IllegalExprOperandType`'s list branch (`tclExecute.c:9089-9107`): a value
/// that is not a number but *is* a well-formed list of more than one element (or
/// a non-empty dict) is reported as `cannot use a list as …operand of "…"` with
/// errorCode `ARITH DOMAIN list`, rather than as a non-numeric string. Shared by
/// the unary and binary operand-error builders so the two agree.
fn is_list_operand(s: &str) -> bool {
    tcl_syntax::list::max_list_length(s) > 1 && tcl_syntax::list::split_list(s).is_ok()
}

pub(super) fn divzero() -> TclError {
    TclError::with_error_code("divide by zero", "ARITH DIVZERO {divide by zero}")
}

/// `0 ** negative` on either the integer or the float tier — C's
/// `EXPON_OF_ZERO` (`tclExecute.c`) is a **domain** error, `-errorcode ARITH
/// DOMAIN`, not a division by zero (tclsh 8.6.16/9.0.4 verified).
pub(super) fn zero_to_negative_power() -> TclError {
    TclError::with_error_code(
        "exponentiation of zero by negative power",
        "ARITH DOMAIN {exponentiation of zero by negative power}",
    )
}

/// The C `IllegalExprOperandType` message for a *unary* operator whose operand
/// cannot be used: `cannot use <desc> "<v>" as operand of "<op>"`. `<desc>` is
/// `floating-point value` (a double handed to `~`), `non-numeric floating-point
/// value` (NaN), `a list` (a multi-element list — phrased without quotes), or
/// `non-numeric string`. (`errorCode ARITH DOMAIN <desc>` is not threaded
/// here.)
pub(super) fn unary_operand_err(v: &Value, op: &str) -> TclError {
    operand_type_err(v, errors::OperandSide::Unary, op)
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
pub(super) fn promote(op: BinOp, x: i128, y: i128) -> Result<Value, TclError> {
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

pub(super) fn dbl_arith(op: BinOp, x: f64, y: f64) -> Result<Value, TclError> {
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
                return Err(zero_to_negative_power());
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
        return Err(TclError::with_error_code(
            errors::DOMAIN_MESSAGE,
            errors::DOMAIN_CODE,
        ));
    }
    Ok(Value::double(r))
}

/// The integer-only binary operators: a (valid) floating-point operand is itself
/// the error (`cannot use floating-point value "x" as <side> operand of "OP"`),
/// rather than routing to the double path.
pub(super) fn is_int_only(op: BinOp) -> bool {
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

pub(super) fn arith_numbers(
    op: BinOp,
    a: &Value,
    b: &Value,
    x: &Num,
    y: &Num,
) -> Result<Value, TclError> {
    // Fast integer path: both operands fit `i128` (promotes to bignum on overflow).
    if let (Some(xi), Some(yi)) = (num_i128(x), num_i128(y)) {
        return int_arith(op, xi, yi);
    }
    // Not both `i128`-fit. When *both* are integers (an operand already past
    // `i128`), stay exact via the arbitrary-precision path.
    if let (Some(xb), Some(yb)) = (num_as_bigint(x), num_as_bigint(y)) {
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
        dbl_arith(op, num_f(x), num_f(y))
    }
}
