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
use tcl_syntax::expr::errors;
use tcl_syntax::expr::{BinOp, ExprOps, NumericCompare, UnaryOp};
use tcl_syntax::number::{self, Number};
use tcl_syntax::number_tower;

use crate::error::TclError;
use crate::interp::Vm;
use crate::value::Value;

/// Numeric authority for one reached VM activation. The simulation provider
/// is explicit and carries no authentic native getter or cache attestation.
#[derive(Clone, Copy)]
pub(crate) struct NumericContext<'a> {
    pub(crate) environment: Option<&'a dyn tcl_platform::NumericEnvironment>,
    pub(crate) dialect: tcl_registry::InvocationDialect,
    pub(crate) simulation:
        Option<tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation>,
}

impl From<tcl_registry::InvocationDialect> for NumericContext<'_> {
    fn from(dialect: tcl_registry::InvocationDialect) -> Self {
        Self {
            dialect,
            simulation: None,
            environment: None,
        }
    }
}

impl std::ops::Deref for NumericContext<'_> {
    type Target = tcl_registry::InvocationDialect;
    fn deref(&self) -> &Self::Target {
        &self.dialect
    }
}

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
    let text = v.try_to_str().ok()?;
    match number::parse_whole(text.trim()) {
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
        // exponent (the domain error) and an exponent past the C limit.
        None if x.is_zero() => Err(zero_to_negative_power()),
        None => Err(TclError::new("exponent too large")),
    }
}

fn to_num(v: &Value) -> Result<Num, TclError> {
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
        errors::illegal_operand_error_code(desc, release),
    )
}

/// The boolean-context error C raises for a value that is neither a number
/// nor a Tcl boolean word (`expected boolean value but got "x"`,
/// `-errorcode TCL VALUE NUMBER`) or for a NaN there
/// (`floating point value is Not a Number`, `TCL VALUE DOUBLE NAN`). Without
/// this, both would default to `NONE`.
fn boolean_context_err(message: Vec<u8>) -> TclError {
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

fn divzero() -> TclError {
    TclError::with_error_code("divide by zero", "ARITH DIVZERO {divide by zero}")
}

/// `0 ** negative` on either the integer or the float tier — C's
/// `EXPON_OF_ZERO` (`tclExecute.c`) is a **domain** error, `-errorcode ARITH
/// DOMAIN`, not a division by zero (tclsh 8.6.16/9.0.4 verified).
fn zero_to_negative_power() -> TclError {
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
fn unary_operand_err(v: &Value, op: &str) -> TclError {
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

fn arith_numbers(op: BinOp, a: &Value, b: &Value, x: &Num, y: &Num) -> Result<Value, TclError> {
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
    match number::parse_whole(v.try_to_str().ok()?.trim()) {
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
    // This standalone helper has Unicode-scalar string semantics. Native
    // execution passes its retained model through compare_in instead.
    let numeric = if string_comparison(op) {
        None
    } else {
        compare_values_numeric(a, b)
    };
    compare_with_numeric(
        op,
        a,
        b,
        numeric,
        Some(tcl_dialect::StringCharacterModel::UnicodeScalars),
    )
}

fn string_comparison(op: BinOp) -> bool {
    matches!(
        op,
        BinOp::StrEq | BinOp::StrNe | BinOp::StrLt | BinOp::StrLe | BinOp::StrGt | BinOp::StrGe
    )
}

fn compare_string_units(
    model: Option<tcl_dialect::StringCharacterModel>,
    a: &Value,
    b: &Value,
) -> Result<Ordering, TclError> {
    let model =
        model.ok_or(tcl_syntax::raw_string::NativeValueAccessRefusal::CharacterModelUnavailable)?;
    let counts = (
        a.native_character_count(model, None)?,
        b.native_character_count(model, None)?,
    );
    let left = tcl_syntax::raw_string::RawString::from_bytes(a.string_bytes());
    let right = tcl_syntax::raw_string::RawString::from_bytes(b.string_bytes());
    left.compare_character_units(model, &right, counts)
        .map_err(Into::into)
}

fn equal_string_bytes(
    model: Option<tcl_dialect::StringCharacterModel>,
    a: &Value,
    b: &Value,
) -> Result<bool, TclError> {
    model.ok_or(tcl_syntax::raw_string::NativeValueAccessRefusal::CharacterModelUnavailable)?;
    Ok(a.string_bytes() == b.string_bytes())
}

fn compare_with_numeric(
    op: BinOp,
    a: &Value,
    b: &Value,
    numeric: Option<NumericCompare>,
    model: Option<tcl_dialect::StringCharacterModel>,
) -> Result<bool, TclError> {
    use BinOp::{Eq, Ge, Gt, Le, Lt, Ne, StrEq, StrGe, StrGt, StrLe, StrLt, StrNe};
    use NumericCompare::{Ordered, Unordered};
    if matches!(op, StrEq | StrNe) {
        return equal_string_bytes(model, a, b).map(|equal| equal ^ (op == StrNe));
    }
    let outcome = match (string_comparison(op), numeric) {
        (false, Some(numeric)) => numeric,
        _ => Ordered(compare_string_units(model, a, b)?),
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
                v.try_to_str()
                    .ok()
                    .and_then(|text| number::parse_whole(text.trim())),
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

/// Convert one operand with the selected native grammar and integer tower.
fn native_num(dialect: NumericContext<'_>, value: &Value) -> Result<Num, TclError> {
    if let Some(simulation) = dialect.simulation {
        let number = simulation
            .parse_number(
                &value.string_bytes(),
                tcl_syntax::logical_numeric_simulation::LogicalNumericInputStage::Number,
            )
            .map_err(|_| TclError::new("non-numeric operand"))?;
        return logical_number(dialect, number);
    }

    if dialect.arithmetic() == Some(tcl_dialect::NativeArithmetic::Tcl84Wide)
        && !value.prepare_native_expression_integer84_with_environment(
            dialect.dialect,
            dialect.environment,
        )?
    {
        return Err(TclError::new("non-numeric operand"));
    }

    let input = dialect.scalar_numeric_input_policy().ok_or_else(|| {
        TclError::from(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable)
    })?;
    if input == number::NativeScalarNumericInputPolicy::NulTerminatedJim084
        && let Ok(integer) = value.native_int(input, dialect.numbers)
    {
        return Ok(Num::Int(integer));
    }
    if let Some(value) = value.integer_representation() {
        return Ok(Num::Int(value));
    }
    if let Some(value) = value.double_representation() {
        return Ok(Num::Dbl(value));
    }
    if dialect.arithmetic() == Some(tcl_dialect::NativeArithmetic::Tcl84Wide)
        && dialect.environment.is_some()
    {
        return match value.native_scalar_probe_with_environment(
            dialect.dialect,
            tcl_syntax::scalar_getter::NativeScalarGetterKind::Double,
            dialect.environment,
        )? {
            Ok(tcl_syntax::scalar_getter::NativeScalarGetterValue::Double(value)) => {
                Ok(Num::Dbl(value))
            }
            _ => Err(TclError::new("non-numeric operand")),
        };
    }

    let policy = dialect
        .arithmetic()
        .ok_or_else(|| TclError::new("native arithmetic policy is not selected"))?;
    if let Some(number @ Number::Big { .. }) = value.number_representation() {
        if policy != tcl_dialect::NativeArithmetic::TclBignum {
            return tcl_syntax::expr::wide::parsed_literal(policy, &number)
                .map(Num::Int)
                .map_err(|error| native_wide_error(dialect, policy, error));
        }
        let Number::Big {
            negative,
            radix,
            digits,
        } = number
        else {
            unreachable!("matched full integer cache")
        };
        let magnitude = BigInt::parse_bytes(digits.as_bytes(), radix as u32)
            .expect("validated native Big magnitude");
        let integer = if negative { -magnitude } else { magnitude };
        return Ok(integer
            .to_i128()
            .map_or_else(|| Num::Huge(integer), Num::Big));
    }
    let original = value.string_bytes();
    let text = match std::str::from_utf8(input.input_bytes(&original)) {
        Ok(text) => text,
        Err(_) if input == number::NativeScalarNumericInputPolicy::NulTerminatedJim084 => {
            return Err(TclError::new("non-numeric operand"));
        }
        Err(error) => {
            return Err(tcl_syntax::raw_string::UnicodeAccessError {
                valid_up_to: error.valid_up_to(),
                error_len: error.error_len(),
            }
            .into());
        }
    };
    parsed_native_number(dialect, value, policy, text)
}

fn parsed_native_number(
    dialect: NumericContext<'_>,
    value: &Value,
    policy: tcl_dialect::NativeArithmetic,
    text: &str,
) -> Result<Num, TclError> {
    match number::parse_whole_with(text, number::ParseFlags::for_syntax(dialect.numbers)) {
        Some(Number::Int(integer)) => {
            value.cache_integer_representation(integer);
            Ok(Num::Int(integer))
        }
        Some(Number::Double(double)) => {
            value.cache_double_representation(double);
            Ok(Num::Dbl(double))
        }
        Some(Number::Nan { .. }) => Ok(Num::Dbl(f64::NAN)),
        Some(Number::Big {
            negative,
            radix,
            digits,
        }) => {
            let magnitude = BigInt::parse_bytes(digits.as_bytes(), radix as u32)
                .ok_or_else(|| TclError::new("invalid integer literal"))?;
            let integer = if negative { -magnitude } else { magnitude };
            if policy == tcl_dialect::NativeArithmetic::TclBignum {
                return Ok(integer
                    .to_i128()
                    .map_or_else(|| Num::Huge(integer), Num::Big));
            }
            let integer = tcl_syntax::expr::wide::literal(policy, &integer)
                .map_err(|error| native_wide_error(dialect, policy, error))?;
            value.cache_integer_representation(integer);
            Ok(Num::Int(integer))
        }
        None => Err(TclError::new("non-numeric operand")),
    }
}

fn logical_number(dialect: NumericContext<'_>, number: Number) -> Result<Num, TclError> {
    match number {
        Number::Int(value) => Ok(Num::Int(value)),
        Number::Double(value) => Ok(Num::Dbl(value)),
        Number::Nan { .. } => Ok(Num::Dbl(f64::NAN)),
        integer @ Number::Big { .. } => {
            let policy = dialect
                .arithmetic()
                .ok_or_else(|| TclError::new("logical arithmetic policy is not selected"))?;
            tcl_syntax::expr::wide::parsed_literal(policy, &integer)
                .map(Num::Int)
                .map_err(|error| native_wide_error(dialect, policy, error))
        }
    }
}

/// Prepare a numeric value under an explicit reached invocation authority.
pub(crate) fn numeric_value_in(
    context: NumericContext<'_>,
    value: &Value,
) -> Result<Value, TclError> {
    native_num(context, value).map(native_num_value)
}

fn native_num_value(number: Num) -> Value {
    match number {
        Num::Int(value) => Value::int(value),
        Num::Big(value) => int_value(value),
        Num::Huge(value) => big_value(&value),
        Num::Dbl(value) => Value::double(value),
    }
}

/// Expression Boolean operators use `Tcl_NewLongObj` in C8.4, independently
/// of the operand's word-Boolean or wideInt primary.
pub(crate) fn native_boolean_result<'a>(
    context: impl Into<NumericContext<'a>>,
    value: bool,
) -> Value {
    let context = context.into();
    if let Some(cache) = context
        .native_scalar_getter_protocol()
        .and_then(|protocol| protocol.expression_integer_result84(i64::from(value), &[]))
    {
        Value::from_native_scalar_cache(cache, None, context.dialect)
            .expect("selected C84 Boolean result")
    } else {
        Value::bool(value)
    }
}

/// Eager logical opcode over the original two stack objects. C8.4 consumes
/// its retained normalisation header and reuses it only when truly unshared.
pub(crate) fn native_logical_in<'a>(
    context: impl Into<NumericContext<'a>>,
    left: Value,
    right: &Value,
    conjunction: bool,
) -> Result<Value, TclError> {
    let context = context.into();
    let a =
        native_boolean(context, &left).map_err(|error| boolean_operand_error(context, error))?;
    let b =
        native_boolean(context, right).map_err(|error| boolean_operand_error(context, error))?;
    let result = if conjunction { a && b } else { a || b };
    if context.simulation.is_none()
        && context
            .native_scalar_getter_protocol()
            .and_then(tcl_syntax::scalar_getter::NativeScalarGetterProtocol::tcl_version)
            == Some(tcl_dialect::TclVersion::V8_4)
        && !left.native_object_is_shared()
    {
        left.store_native_expression_long84(i64::from(result), context.dialect)?;
        Ok(left)
    } else {
        Ok(native_boolean_result(context, result))
    }
}

fn native_wide_error(
    dialect: NumericContext<'_>,
    policy: tcl_dialect::NativeArithmetic,
    error: tcl_syntax::expr::wide::WideError,
) -> TclError {
    use tcl_syntax::expr::wide::WideError;
    let error = match error {
        WideError::DivisionByZero if policy == tcl_dialect::NativeArithmetic::JimWide => {
            TclError::new("Division by zero")
        }
        WideError::DivisionByZero => divzero(),
        WideError::LiteralOverflow => TclError::with_error_code(
            "integer value too large to represent",
            "ARITH IOVERFLOW {integer value too large to represent}",
        ),
        WideError::ZeroToNegativePower if policy == tcl_dialect::NativeArithmetic::JimWide => {
            TclError::new("exponentiation of zero by negative power")
        }
        WideError::ZeroToNegativePower => zero_to_negative_power(),
        WideError::UndefinedNativeOperation => TclError::new("undefined native integer operation"),
        WideError::Unsupported => TclError::new("unsupported native integer operator"),
    };
    native_numeric_error84(dialect, error)
}

fn native_numeric_error84(dialect: NumericContext<'_>, error: TclError) -> TclError {
    if dialect
        .native_scalar_getter_protocol()
        .is_none_or(|protocol| protocol.tcl_version() != Some(tcl_dialect::TclVersion::V8_4))
    {
        return error;
    }
    let Some(materialization) = dialect.dialect.native_string_materialization(None) else {
        return tcl_syntax::value::ValueError::ScalarNumericInputUnavailable.into();
    };
    if let Some(completion) = error.guest_completion()
        && let Err(refusal) = completion
            .result
            .retain_native_string_representation(materialization)
    {
        return refusal.into();
    }
    error
}

/// Arithmetic used by both native expression evaluation and inline opcodes.
pub(crate) fn arith_in<'a>(
    dialect: impl Into<NumericContext<'a>>,
    op: BinOp,
    a: &Value,
    b: &Value,
) -> Result<Value, TclError> {
    let dialect = dialect.into();
    let policy = dialect
        .arithmetic()
        .ok_or_else(|| TclError::new("native arithmetic policy is not selected"))?;
    let x = native_operand(dialect, a, errors::OperandSide::Left, op)?;
    let y = native_operand(dialect, b, errors::OperandSide::Right, op)?;
    if is_int_only(op) && (matches!(x, Num::Dbl(_)) || matches!(y, Num::Dbl(_))) {
        let failed = if matches!(x, Num::Dbl(_)) { a } else { b };
        if let Some(error) = expression_operand_error(
            dialect,
            tcl_registry::native_numeric_error::NativeExpressionOperandStage::Integer,
            failed,
        ) {
            return Err(error);
        }
        return Err(selected_operand_error_code(
            dialect,
            if matches!(x, Num::Dbl(_)) {
                context_operand_error(dialect, a, errors::OperandSide::Left, op.as_str())
            } else {
                context_operand_error(dialect, b, errors::OperandSide::Right, op.as_str())
            },
        ));
    }
    if policy == tcl_dialect::NativeArithmetic::TclBignum {
        return arith_numbers(op, a, b, &x, &y);
    }
    if let (Num::Int(x), Num::Int(y)) = (&x, &y) {
        let integer = tcl_syntax::expr::wide::binary(policy, op, *x, *y)
            .map_err(|error| native_wide_error(dialect, policy, error))?;
        if let Some(cache) = dialect
            .native_scalar_getter_protocol()
            .and_then(|protocol| {
                protocol.expression_integer_result84(
                    integer,
                    &[a.native_scalar_cache(), b.native_scalar_cache()],
                )
            })
        {
            return Value::from_native_scalar_cache(cache, None, dialect.dialect)
                .map_err(Into::into);
        }
        return Ok(Value::int(integer));
    }
    dbl_arith(op, num_f(&x), num_f(&y))
}

fn expression_operand_error(
    dialect: NumericContext<'_>,
    stage: tcl_registry::native_numeric_error::NativeExpressionOperandStage,
    value: &Value,
) -> Option<TclError> {
    let presenter = dialect.expression_operand_error_presentation()?;
    Some(TclError::with_error_code(
        presenter.message(stage, &value.string_bytes()),
        presenter.error_code(),
    ))
}

/// Present an incompatible arithmetic operand using only the selected numeric
/// authority. Authored logical simulation does not borrow the physical engine's
/// error wording, primitive error state or cache transition.
fn context_operand_error(
    context: NumericContext<'_>,
    value: &Value,
    side: errors::OperandSide,
    operator: &str,
) -> TclError {
    let release = if let Some(simulation) = context.simulation {
        match simulation {
            tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation::Tcl84Core => {
                tcl_dialect::TclVersion::V8_4
            }
        }
    } else if let Some(release) = context
        .native_scalar_getter_protocol()
        .and_then(tcl_syntax::scalar_getter::NativeScalarGetterProtocol::tcl_version)
    {
        release
    } else {
        return TclError::from(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable);
    };
    let original = value.string_bytes();
    let parsed = if let Some(simulation) = context.simulation {
        simulation
            .parse_number(
                &original,
                tcl_syntax::logical_numeric_simulation::LogicalNumericInputStage::Number,
            )
            .ok()
    } else if let Some(double) = value.double_representation() {
        Some(Number::Double(double))
    } else {
        std::str::from_utf8(&original).ok().and_then(|text| {
            number::parse_whole_with(text, number::ParseFlags::for_syntax(context.numbers))
        })
    };
    let description = match parsed {
        Some(Number::Double(double)) if double.is_nan() => {
            errors::OperandDesc::NonNumericFloatingPointValue
        }
        Some(Number::Double(_)) => errors::OperandDesc::FloatingPointValue,
        Some(Number::Nan { .. }) => errors::OperandDesc::NonNumericFloatingPointValue,
        _ if release >= tcl_dialect::TclVersion::V9_0
            && tcl_syntax::list::split_list_bytes_in(
                &original,
                context.lexer_grammar.list_parse,
                context.lexer_grammar.escapes,
            )
            .is_ok_and(|elements| elements.len() > 1) =>
        {
            errors::OperandDesc::List
        }
        _ => errors::OperandDesc::NonNumericString,
    };
    let spelling =
        if release >= tcl_dialect::TclVersion::V9_0 && description != errors::OperandDesc::List {
            match value.try_to_str() {
                Ok(text) => text,
                Err(error) => return error.into(),
            }
        } else {
            std::rc::Rc::from("")
        };
    TclError::with_error_code(
        errors::illegal_operand_message(description, &spelling, side, operator, release),
        errors::illegal_operand_error_code(description, release),
    )
}

fn selected_operand_error_code(dialect: NumericContext<'_>, mut error: TclError) -> TclError {
    let policy = dialect.simulation.map_or_else(
        || dialect.expression_operand_error_code_policy(),
        |simulation| match simulation {
            tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation::Tcl84Core => {
                Some(tcl_registry::native_numeric_error::NativeExpressionOperandErrorCodePolicy::None)
            }
        },
    );
    if error.is_guest_error()
        && let Some(policy) = policy
    {
        let existing = error.error_code_bytes();
        error.set_error_code(
            policy.select_invalid_type_code(existing.as_deref().unwrap_or(b"NONE")),
        );
    }
    error
}

pub(crate) fn boolean_operand_error<'a>(
    dialect: impl Into<NumericContext<'a>>,
    error: TclError,
) -> TclError {
    let dialect = dialect.into();
    if !error.is_guest_error()
        || dialect.simulation.is_some()
        || dialect.expression_operand_error_presentation().is_some()
    {
        error
    } else {
        let error = boolean_context_err(
            error
                .message_bytes()
                .expect("guest operand failure")
                .to_vec(),
        );
        if error.error_code_bytes().as_deref() == Some(errors::NAN_CODE.as_bytes()) {
            error
        } else {
            selected_operand_error_code(dialect, error)
        }
    }
}

pub(crate) fn native_boolean<'a>(
    dialect: impl Into<NumericContext<'a>>,
    value: &Value,
) -> Result<bool, TclError> {
    let dialect = dialect.into();
    if let Some(simulation) = dialect.simulation {
        let stage = tcl_syntax::logical_numeric_simulation::LogicalBooleanInputStage::NumericTruth;
        if let Some(integer) = value.integer_representation() {
            return Ok(simulation
                .current_number_boolean(Number::Int(integer), stage)
                .value());
        }
        if let Some(double) = value.double_representation() {
            return Ok(simulation
                .current_number_boolean(Number::Double(double), stage)
                .value());
        }
        return simulation
            .parse_boolean(&value.string_bytes(), stage)
            .map(|value| value.value())
            .map_err(|_| {
                TclError::from(tcl_syntax::value::ValueError::NotBooleanBytes(
                    value.string_bytes().to_vec(),
                ))
            });
    }
    if dialect.arithmetic() == Some(tcl_dialect::NativeArithmetic::Tcl84Wide) {
        use tcl_syntax::scalar_getter::NativeScalarCache as Cache;
        match value.native_scalar_cache() {
            Some(Cache::WordBoolean(value)) => return Ok(value),
            Some(Cache::Tcl84Long(value) | Cache::Number(Number::Int(value))) => {
                return Ok(value != 0);
            }
            Some(Cache::Number(Number::Double(value))) => return Ok(value != 0.0),
            Some(Cache::Number(Number::Nan { .. })) => return Ok(true),
            _ => {}
        }
    }
    let input = dialect.scalar_numeric_input_policy().ok_or_else(|| {
        TclError::from(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable)
    })?;
    if let Some(presenter) = dialect.expression_operand_error_presentation() {
        if let Ok(number) = native_num(dialect, value) {
            return Ok(match number {
                Num::Int(number) => number != 0,
                Num::Dbl(number) => number != 0.0,
                Num::Big(number) => number != 0,
                Num::Huge(number) => !number.is_zero(),
            });
        }
        return native_boolean_word(dialect, value, input).map_err(|error| {
            if let Some(refusal) = error.native_access_refusal() {
                return refusal.into();
            }
            TclError::with_error_code(
                presenter.message(
                    tcl_registry::native_numeric_error::NativeExpressionOperandStage::Boolean,
                    &value.string_bytes(),
                ),
                presenter.error_code(),
            )
        });
    }
    if let Ok(number) = native_num(dialect, value) {
        return match number {
            Num::Int(number) => Ok(number != 0),
            Num::Dbl(number) if number.is_nan() => Err(TclError::with_error_code(
                errors::NAN_MESSAGE,
                errors::NAN_CODE,
            )),
            Num::Dbl(number) => Ok(number != 0.0),
            Num::Big(number) => Ok(number != 0),
            Num::Huge(number) => Ok(!number.is_zero()),
        };
    }
    native_boolean_word(dialect, value, input).map_err(|error| {
        if let Some(refusal) = error.native_access_refusal() {
            return refusal.into();
        }
        let message = match error {
            tcl_syntax::value::ValueError::NativeScalarGetter(error) => {
                error.eval_result_bytes(error.message_bytes()).to_vec()
            }
            error => error.message_bytes(),
        };
        selected_operand_error_code(dialect, boolean_context_err(message))
    })
}

fn native_boolean_word(
    dialect: NumericContext<'_>,
    value: &Value,
    input: number::NativeScalarNumericInputPolicy,
) -> Result<bool, tcl_syntax::value::ValueError> {
    if dialect
        .native_scalar_getter_protocol()
        .and_then(tcl_syntax::scalar_getter::NativeScalarGetterProtocol::tcl_version)
        .is_some()
    {
        return match value.native_scalar_getter(
            dialect.dialect,
            tcl_syntax::scalar_getter::NativeScalarGetterKind::Boolean,
        )? {
            tcl_syntax::scalar_getter::NativeScalarGetterValue::Boolean(boolean) => Ok(boolean),
            _ => Err(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable),
        };
    }
    value.native_bool(input, dialect.numbers)
}

fn native_operand(
    dialect: NumericContext<'_>,
    value: &Value,
    side: errors::OperandSide,
    op: BinOp,
) -> Result<Num, TclError> {
    native_num(dialect, value).map_err(|error| {
        if error.is_host() {
            return error;
        }
        let stage = if is_int_only(op) {
            tcl_registry::native_numeric_error::NativeExpressionOperandStage::Integer
        } else {
            tcl_registry::native_numeric_error::NativeExpressionOperandStage::FloatingPoint
        };
        let error = if let Some(error) = expression_operand_error(dialect, stage, value) {
            error
        } else if error
            .message_bytes()
            .is_ok_and(|message| message.as_ref() == b"integer value too large to represent")
        {
            return TclError::with_error_code(
                format!(
                    "can't use integer value too large to represent as operand of \"{}\"",
                    op.as_str()
                ),
                error.error_code_bytes().unwrap_or_default(),
            );
        } else {
            context_operand_error(dialect, value, side, op.as_str())
        };
        selected_operand_error_code(dialect, error)
    })
}

/// Unary arithmetic under the entered native command's selected policy.
pub(crate) fn unary_in<'a>(
    dialect: impl Into<NumericContext<'a>>,
    op: UnaryOp,
    value: &Value,
) -> Result<Value, TclError> {
    let dialect = dialect.into();
    let policy = dialect
        .arithmetic()
        .ok_or_else(|| TclError::new("native arithmetic policy is not selected"))?;
    if matches!(op, UnaryOp::Not | UnaryOp::WordNot) {
        return native_boolean(dialect, value).map(|truth| native_boolean_result(dialect, !truth));
    }
    let number = native_num(dialect, value).map_err(|error| {
        if error.is_host() {
            return error;
        }
        if let Some(presenter) = dialect.expression_operand_error_presentation()
            && matches!(op, UnaryOp::Pos | UnaryOp::Neg)
            && dialect
                .scalar_numeric_input_policy()
                .is_some_and(|input| value.native_bool(input, dialect.numbers).is_ok())
            && let Some(message) = presenter.non_numeric_unary_message(op)
        {
            return TclError::with_error_code(message, presenter.error_code());
        }
        let stage = if op == UnaryOp::BitNot {
            tcl_registry::native_numeric_error::NativeExpressionOperandStage::Integer
        } else {
            tcl_registry::native_numeric_error::NativeExpressionOperandStage::Boolean
        };
        expression_operand_error(dialect, stage, value).unwrap_or_else(|| {
            selected_operand_error_code(
                dialect,
                context_operand_error(dialect, value, errors::OperandSide::Unary, op.as_str()),
            )
        })
    })?;
    if op == UnaryOp::BitNot && matches!(number, Num::Dbl(_)) {
        return Err(expression_operand_error(
            dialect,
            tcl_registry::native_numeric_error::NativeExpressionOperandStage::Integer,
            value,
        )
        .unwrap_or_else(|| {
            selected_operand_error_code(
                dialect,
                context_operand_error(dialect, value, errors::OperandSide::Unary, op.as_str()),
            )
        }));
    }
    if policy == tcl_dialect::NativeArithmetic::TclBignum {
        return unary(op, &native_num_value(number));
    }
    match number {
        Num::Int(integer) => {
            let result = tcl_syntax::expr::wide::unary(policy, op, integer)
                .map_err(|error| native_wide_error(dialect, policy, error))?;
            if let Some(cache) = dialect
                .native_scalar_getter_protocol()
                .and_then(|protocol| {
                    protocol.expression_integer_result84(result, &[value.native_scalar_cache()])
                })
            {
                Value::from_native_scalar_cache(cache, None, dialect.dialect).map_err(Into::into)
            } else {
                Ok(Value::int(result))
            }
        }
        Num::Dbl(value) if op == UnaryOp::Neg => Ok(Value::double(-value)),
        Num::Dbl(value) if op == UnaryOp::Pos => Ok(Value::double(value)),
        _ => Err(expression_operand_error(
            dialect,
            tcl_registry::native_numeric_error::NativeExpressionOperandStage::Integer,
            value,
        )
        .unwrap_or_else(|| {
            selected_operand_error_code(
                dialect,
                context_operand_error(dialect, value, errors::OperandSide::Unary, op.as_str()),
            )
        })),
    }
}

fn native_literal(dialect: NumericContext<'_>, text: &str) -> Result<Value, TclError> {
    let policy = dialect
        .arithmetic()
        .ok_or_else(|| TclError::new("native arithmetic policy is not selected"))?;
    if policy == tcl_dialect::NativeArithmetic::TclBignum {
        return Ok(
            match number::parse_whole_with(text, number::ParseFlags::for_syntax(dialect.numbers)) {
                Some(Number::Int(value)) => Value::int(value),
                Some(Number::Double(value)) => Value::double(value),
                _ => Value::string(text),
            },
        );
    }
    match native_num(dialect, &Value::string(text)) {
        Ok(Num::Int(value)) => Ok(Value::int(value)),
        Ok(Num::Dbl(value)) if !value.is_nan() => Ok(Value::double(value)),
        Err(error)
            if error.message_bytes().is_ok_and(|message| {
                message.as_ref() == b"integer value too large to represent"
            }) =>
        {
            Err(error)
        }
        _ => Ok(Value::string(text)),
    }
}

fn normalize_expression_number84(
    dialect: NumericContext<'_>,
    value: Value,
    protocol: tcl_syntax::scalar_getter::NativeScalarGetterProtocol,
) -> Result<Value, TclError> {
    let integer = value.prepare_native_expression_integer84_with_environment(
        dialect.dialect,
        dialect.environment,
    )?;
    if integer
        && (value.native_scalar_cache().is_none()
            || matches!(
                value.native_scalar_cache(),
                Some(tcl_syntax::scalar_getter::NativeScalarCache::WordBoolean(_))
            ))
    {
        let _ = value.native_scalar_probe_with_environment(
            dialect.dialect,
            tcl_syntax::scalar_getter::NativeScalarGetterKind::Double,
            dialect.environment,
        )?;
    }
    let value = value.normalize_native_expression_number84(dialect.dialect)?;
    if let Some(double) = value.double_representation()
        && let Some(failure) = tcl_cmd_core::native_numeric::c84_nonfinite_error(
            protocol,
            double,
            dialect.environment,
        )?
    {
        let (message, code) = failure.diagnostic();
        return Err(native_numeric_error84(
            dialect,
            TclError::with_error_code(message, code),
        ));
    }
    Ok(value)
}

/// Result normalization is a separate native rule from bare literal conversion.
pub(crate) fn cvt_to_numeric_in<'a>(
    dialect: impl Into<NumericContext<'a>>,
    value: Value,
) -> Result<Value, TclError> {
    let dialect = dialect.into();
    let policy = dialect
        .arithmetic()
        .ok_or_else(|| TclError::new("native arithmetic policy is not selected"))?;
    if !policy.normalizes_expression_result() {
        return Ok(value);
    }
    if let Some(protocol) = dialect
        .native_scalar_getter_protocol()
        .filter(|protocol| protocol.tcl_version() == Some(tcl_dialect::TclVersion::V8_4))
    {
        return normalize_expression_number84(dialect, value, protocol);
    }
    if let Some(double) = value.double_representation() {
        if let Some(protocol) = dialect
            .native_scalar_getter_protocol()
            .filter(|protocol| protocol.tcl_version() == Some(tcl_dialect::TclVersion::V8_4))
        {
            if let Some(failure) = tcl_cmd_core::native_numeric::c84_nonfinite_error(
                protocol,
                double,
                dialect.environment,
            )? {
                let (message, code) = failure.diagnostic();
                return Err(native_numeric_error84(
                    dialect,
                    TclError::with_error_code(message, code),
                ));
            }
        } else if double.is_nan() {
            return Err(TclError::new("domain error: argument not in valid range"));
        }
        return Ok(value);
    }
    match native_num(dialect, &value) {
        Ok(Num::Int(integer)) => {
            if let Some(cache) = dialect
                .native_scalar_getter_protocol()
                .and_then(|protocol| {
                    protocol.expression_integer_result84(integer, &[value.native_scalar_cache()])
                })
            {
                Value::from_native_scalar_cache(cache, None, dialect.dialect).map_err(Into::into)
            } else {
                Ok(Value::int(integer))
            }
        }
        Ok(Num::Dbl(number)) => {
            if let Some(protocol) = dialect
                .native_scalar_getter_protocol()
                .filter(|protocol| protocol.tcl_version() == Some(tcl_dialect::TclVersion::V8_4))
                && let Some(failure) = tcl_cmd_core::native_numeric::c84_nonfinite_error(
                    protocol,
                    number,
                    dialect.environment,
                )?
            {
                let (message, code) = failure.diagnostic();
                return Err(native_numeric_error84(
                    dialect,
                    TclError::with_error_code(message, code),
                ));
            }
            if number.is_nan() {
                return Err(TclError::new("domain error: argument not in valid range"));
            }
            Ok(Value::double(number))
        }
        Ok(number) => Ok(native_num_value(number)),
        Err(error) if error.is_host() => Err(error),
        Err(_) => Ok(value),
    }
}

fn compare_values_numeric_in(
    dialect: NumericContext<'_>,
    a: &Value,
    b: &Value,
) -> Option<NumericCompare> {
    if dialect.arithmetic()? == tcl_dialect::NativeArithmetic::TclBignum {
        return compare_values_numeric(
            &native_num_value(native_num(dialect, a).ok()?),
            &native_num_value(native_num(dialect, b).ok()?),
        );
    }
    Some(
        match (native_num(dialect, a).ok()?, native_num(dialect, b).ok()?) {
            (Num::Int(a), Num::Int(b)) => NumericCompare::Ordered(a.cmp(&b)),
            (Num::Int(a), Num::Dbl(b)) => {
                NumericCompare::from_partial(number::compare_int_double(i128::from(a), b))
            }
            (Num::Dbl(a), Num::Int(b)) => NumericCompare::from_partial(
                number::compare_int_double(i128::from(b), a).map(Ordering::reverse),
            ),
            (Num::Dbl(a), Num::Dbl(b)) => NumericCompare::from_partial(a.partial_cmp(&b)),
            _ => return None,
        },
    )
}

/// Native comparisons preserve wide conversion before comparing numeric values.
pub(crate) fn compare_in<'a>(
    dialect: impl Into<NumericContext<'a>>,
    op: BinOp,
    a: &Value,
    b: &Value,
) -> Result<bool, TclError> {
    let dialect = dialect.into();
    let numeric = if string_comparison(op) {
        None
    } else {
        compare_values_numeric_in(dialect, a, b)
    };
    compare_with_numeric(op, a, b, numeric, dialect.characters)
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
        Contains, EndsWith, Matches, MatchesGlob, MatchesRegex, StartsWith, StrEquals, WordAnd,
        WordOr,
    };
    // `and`/`or` are boolean tests, not string ones — they never render an
    // operand.
    match op {
        WordAnd => return word_and(left, right).map(Value::bool),
        WordOr => return word_or(left, right).map(Value::bool),
        _ => {}
    }
    let (subject, operand) = (left.try_to_str()?, right.try_to_str()?);
    let truth = match op {
        Contains => subject.contains(&*operand),
        StartsWith => subject.starts_with(&*operand),
        EndsWith => subject.ends_with(&*operand),
        // `equals` is the word spelling of `eq`: always a string
        // comparison. The bare `matches` shares that answer, but not for
        // the same reason, and the difference is deliberately recorded
        // rather than hidden behind the shared arm: only the operator's
        // *presence* is measured
        // (`docs/design/f5/bigip-irule-parser-measurements.md` §4a
        // `e_matches`: `expr {"abc" matches "abc"}` → `1`), and that cell
        // is an exact-equality case, so equality is the one reading the
        // evidence actually exercises. §12 carries the discriminating
        // re-probe; until it runs the compiler deliberately refuses to
        // constant-fold `matches` (`tcl_compiler::tcl_expr_eval`), so no
        // unmeasured semantics is ever baked into a rewrite.
        StrEquals | Matches => compare(BinOp::StrEq, left, right)?,
        // Case-sensitive `string match` / `regexp` — the dialect operators have
        // no `-nocase` form.
        MatchesGlob => tcl_syntax::glob::string_match(&operand, &subject),
        // iRules embeds Tcl 8.4, whose ARE has no `\z`.
        MatchesRegex => crate::cmd_regexp::regexp_matches(
            &operand,
            &subject,
            false,
            tcl_dialect::TclVersion::V8_4,
        )
        .map_err(TclError::new)?,
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

/// A math-function command's error completion as an expr error, **keeping the
/// `-errorcode` it published**. Rebuilding the error from the message alone
/// on the dynamic `expr $e` path would report `ARITH DOMAIN` when
/// compiled as `expr {…}` but `NONE` when evaluated from a variable.
/// An [`ExprOps`] adapter that evaluates an expression AST against the VM
/// (resolving `$var` / `[cmd]` / math functions through it), reusing the shared
/// `tcl-syntax` expr walker.
pub struct ExprEval<'a> {
    /// The VM the expression resolves variables/commands against.
    pub vm: &'a mut Vm,
    /// Return options retained by this in-progress expression activation.
    /// A finished result is paired with these options by `finish`.
    options: Value,
    jim: Option<std::rc::Rc<tcl_syntax::expr::native_objects::JimExpressionObjects<Value>>>,
    safe: bool,
}

impl<'a> ExprEval<'a> {
    pub(crate) fn new(vm: &'a mut Vm) -> Self {
        Self {
            vm,
            options: Value::empty(),
            jim: None,
            safe: false,
        }
    }

    pub(crate) fn native(
        vm: &'a mut Vm,
        objects: std::rc::Rc<tcl_syntax::expr::native_objects::JimExpressionObjects<Value>>,
        safe: bool,
    ) -> Self {
        Self {
            vm,
            options: Value::empty(),
            jim: Some(objects),
            safe,
        }
    }

    pub(crate) fn with_jim_objects(
        vm: &'a mut Vm,
        objects: Option<std::rc::Rc<tcl_syntax::expr::native_objects::JimExpressionObjects<Value>>>,
    ) -> Self {
        Self {
            vm,
            options: Value::empty(),
            jim: objects,
            safe: false,
        }
    }

    fn original_term(
        &self,
        start: u32,
        end: Option<u32>,
    ) -> Result<Option<(tcl_lexer::ExprTermKind, Value)>, TclError> {
        let Some(objects) = &self.jim else {
            return Ok(None);
        };
        let (term, value) = objects.at(start, end).ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original Jim expression leaf extent",
            ),
        )?;
        Ok(Some((term.kind, value.clone())))
    }

    fn accept_completion(
        &mut self,
        completion: tcl_runtime_api::Completion<Value>,
    ) -> Result<Value, TclError> {
        if let Some(refusal) = self.vm.execution_refusal.clone() {
            return Err(TclError::from_execution_failure(refusal));
        }
        if completion.code != Code::Ok {
            return Err(TclError::from_completion(completion));
        }
        self.options = completion.options;
        Ok(completion.result)
    }

    pub(crate) fn finish(self, result: Value) -> tcl_runtime_api::Completion<Value> {
        tcl_runtime_api::Completion::new(Code::Ok, result, self.options)
    }
}

impl ExprOps for ExprEval<'_> {
    type Value = Value;
    type Error = TclError;

    fn literal_bytes_at(&mut self, text: &[u8], start: u32, end: u32) -> Result<Value, TclError> {
        if let Some((_, value)) = self.original_term(start, Some(end))? {
            return Ok(value);
        }
        self.literal_bytes(text)
    }

    fn string_bytes_at(
        &mut self,
        text: &[u8],
        substitutes: bool,
        start: u32,
        end: u32,
    ) -> Result<Value, TclError> {
        if let Some((kind, value)) = self.original_term(start, Some(end))? {
            if kind == tcl_lexer::ExprTermKind::String {
                return Ok(value);
            }
            if self.safe {
                return Err(TclError::new(b""));
            }
        }
        self.string_bytes(text, substitutes)
    }

    fn command_bytes_at(&mut self, text: &[u8], start: u32, end: u32) -> Result<Value, TclError> {
        if let Some((_, original)) = self.original_term(start, Some(end))? {
            if self.safe {
                return Err(TclError::new(b""));
            }
            let completion = self
                .vm
                .eval_value_at_level(self.vm.current_level(), &original);
            return self.accept_completion(completion);
        }
        self.command_bytes(text)
    }

    fn literal_bytes(&mut self, text: &[u8]) -> Result<Value, TclError> {
        let text = std::str::from_utf8(text).map_err(|_| {
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native numeric token grammar",
            )
        })?;
        self.literal(text)
    }

    fn string_bytes(&mut self, inner: &[u8], substitutes: bool) -> Result<Value, TclError> {
        if !substitutes {
            return Ok(Value::from_string_bytes(
                tcl_syntax::backslash::collapse_brace_continuations_for(
                    inner,
                    self.vm.native_invocation_dialect().word_values.brace,
                )
                .as_ref(),
            ));
        }
        let policy = self.vm.expression_quote_control().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native expression quote settlement",
            ),
        )?;
        let completion = crate::subst::subst_expression_string(
            self.vm,
            inner,
            crate::subst::SubstitutionControl::Expression(policy),
        )?;
        self.accept_completion(completion)
    }

    fn variable_reference_bytes_at(
        &mut self,
        reference: &[u8],
        start: u32,
    ) -> Result<Value, TclError> {
        if self.safe {
            if let Some((_, original)) = self.original_term(start, None)? {
                return Ok(original);
            }
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original Jim safe variable term",
            )
            .into());
        }
        crate::subst::subst_variable_reference_bytes(reference, self.vm)
    }

    fn command_bytes(&mut self, script: &[u8]) -> Result<Value, TclError> {
        let completion = self
            .vm
            .eval_source_image_at_internal(&tcl_lexer::SourceImage::native(script), None)?;
        self.accept_completion(completion)
    }

    fn call_bytes_at(
        &mut self,
        function: &[u8],
        args: Vec<Value>,
        start: u32,
    ) -> Result<Value, TclError> {
        let function = std::str::from_utf8(function).map_err(|_| {
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native math function name grammar",
            )
        })?;
        self.call_at(function, args, start)
    }

    fn literal(&mut self, text: &str) -> Result<Value, TclError> {
        let dialect = self.vm.native_invocation_dialect();
        native_literal(self.vm.numeric_context(), text)
            .map(|value| value.with_native_double_format(dialect))
    }

    fn string(&mut self, inner: &str, substitutes: bool) -> Result<Value, TclError> {
        // A `"…"` expr operand is a double-quoted word: substitute `$var` /
        // `[cmd]` / backslashes (the runtime-`expr` analogue of the compiler's
        // `emit_expr_string`), so `expr {"item $i"}` is `item 0`, not `item $i`.
        //
        // A `{…}` operand is literal but for its backslash-newlines, which
        // fold even inside braces. Substituting it fully ran the `[id 9]` in
        // `set e {{[id 9]}}; expr $e`, which tclsh 8.4.20 through 9.1b0 all
        // return as the text `[id 9]` (#2227).
        if !substitutes {
            return Ok(Value::string(
                self.vm
                    .native_invocation_dialect()
                    .word_values
                    .collapse_braced_word(inner)
                    .as_ref(),
            ));
        }
        let Some(policy) = self.vm.expression_quote_control() else {
            let _ = self
                .vm
                .refuse_host_command("native expression quote settlement is unavailable".into());
            return Err(TclError::from_execution_failure(
                self.vm
                    .execution_refusal
                    .clone()
                    .expect("quote refusal retained"),
            ));
        };
        let completion = crate::subst::subst_expression_string(
            self.vm,
            inner,
            crate::subst::SubstitutionControl::Expression(policy),
        )?;
        self.accept_completion(completion)
    }

    fn variable_reference(&mut self, reference: &str) -> Result<Value, TclError> {
        crate::subst::subst_word(reference, self.vm)
    }

    fn var(&mut self, name: &str) -> Result<Value, TclError> {
        match self.vm.read_var_traced(name) {
            Err(c) => Err(TclError::from_completion(c)),
            Ok(Some(value)) => Ok(value),
            Ok(None) => Err(TclError::new(format!(
                "can't read \"{name}\": no such variable"
            ))),
        }
    }

    fn command(&mut self, script: &str) -> Result<Value, TclError> {
        // A command substitution inside an expression yields the command's result
        // *only* when it completes normally; otherwise the completion must
        // propagate, not be taken as the value — otherwise `expr {[error msg]}`
        // would silently evaluate to the string "msg" (if-5.2). An error becomes a
        // plain `TclError`; a `break`/`continue`/`return` escaping the substitution
        // carries its code so the enclosing construct sees it.
        let c = self.vm.eval_source(script)?;
        self.accept_completion(c)
    }

    fn call(&mut self, function: &str, args: Vec<Value>) -> Result<Value, TclError> {
        let surface = tcl_registry::expr_surface::RuntimeExprSurface::for_profile(
            self.vm.native_execution_profile(),
        );
        match surface.math_function_call_target(function) {
            tcl_registry::expr_surface::MathFunctionCallTarget::FixedBuiltin(spec) => {
                // Tcl 8.4 predates TIP 232: builtin expressions use the fixed
                // function table selected by the registry, never a script
                // command that happens to share the `tcl::mathfunc` spelling.
                let c = self.vm.invoke_fixed_math_builtin(spec, &args);
                return self.accept_completion(c);
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
        self.accept_completion(c)
    }

    fn arith(&mut self, op: BinOp, l: Value, r: Value) -> Result<Value, TclError> {
        let dialect = self.vm.native_invocation_dialect();
        arith_in(self.vm.numeric_context(), op, &l, &r)
            .map(|value| value.with_native_double_format(dialect))
    }

    fn unary(&mut self, op: UnaryOp, v: Value) -> Result<Value, TclError> {
        let dialect = self.vm.native_invocation_dialect();
        unary_in(self.vm.numeric_context(), op, &v)
            .map(|value| value.with_native_double_format(dialect))
    }

    fn compare_numeric(&mut self, l: &Value, r: &Value) -> Option<NumericCompare> {
        compare_values_numeric_in(self.vm.numeric_context(), l, r)
    }

    fn compare_string(&mut self, l: &Value, r: &Value) -> Result<Ordering, TclError> {
        compare_string_units(self.vm.native_invocation_dialect().characters, l, r)
    }

    fn equal_string(&mut self, l: &Value, r: &Value) -> Result<bool, TclError> {
        equal_string_bytes(self.vm.native_invocation_dialect().characters, l, r)
    }

    fn in_list(&mut self, needle: &Value, list: &Value) -> Result<bool, TclError> {
        let dialect = self.vm.native_invocation_dialect();
        let items = list
            .native_list_elements(
                dialect.lexer_grammar.list_parse,
                dialect.lexer_grammar.escapes,
            )
            .map_err(|error| {
                TclError::from(tcl_syntax::value::ValueError::ListParse {
                    error,
                    source: list.string_bytes().to_vec(),
                })
            })?;
        let model = dialect.characters;
        for item in items.iter() {
            if equal_string_bytes(model, needle, item)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn to_bool(&mut self, v: &Value) -> Result<bool, TclError> {
        let context = self.vm.numeric_context();
        native_boolean(context, v).map_err(|error| boolean_operand_error(context, error))
    }

    fn bool_value(&mut self, b: bool) -> Value {
        native_boolean_result(self.vm.numeric_context(), b)
    }

    fn unsupported(&mut self, what: &str) -> TclError {
        TclError::new(what)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::Command;
    use crate::interp::{Vm, ok};
    use tcl_runtime_api::Completion;

    #[test]
    fn c84_reached_expression_matches_seven_original_cache_controls() {
        use tcl_syntax::scalar_getter::{
            NativeScalarCache as Cache, NativeScalarGetterKind as Getter,
        };
        let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4);
        let class = |value: &Value| match value.native_scalar_cache() {
            Some(Cache::Tcl84Long(_)) => "int",
            Some(Cache::Number(Number::Int(_))) => "wideInt",
            Some(Cache::Number(Number::Double(_))) => "double",
            None => "none",
            _ => panic!("original scalar class"),
        };
        let words = [b"2".as_slice(), b"2.0", b"2", b"2", b"2", b"0x10", b"010"];
        let mut compared = 0;
        for row in include_str!("../../tcl-syntax/tests/data/native_numeric_operand_conversions/expression84-reached.tsv").lines() {
            let fields = row.split('\t').collect::<Vec<_>>();
            let mode = fields[0].parse::<usize>().unwrap();
            let original = match mode {
                2 => Value::double(2.0),
                4 => Value::int(2),
                _ => Value::new_native_string_bytes(words[mode]),
            };
            if mode < 2 { original.native_scalar_getter(dialect, Getter::Double).unwrap(); }
            if mode == 3 { original.native_scalar_getter(dialect, Getter::Wide).unwrap(); }
            assert_eq!(class(&original), fields[1], "native before mode {mode}");
            assert_eq!(usize::from(original.resident_string_bytes().is_some()).to_string(), fields[2]);
            let one = Value::from_native_scalar_cache(Cache::Tcl84Long(1), None, dialect).unwrap();
            let result = arith_in(dialect, BinOp::Add, &original, &one).unwrap();
            assert_eq!(fields[3], "0");
            assert_eq!(class(&original), fields[4], "native reached mode {mode}");
            assert_eq!(usize::from(original.resident_string_bytes().is_some()).to_string(), fields[5]);
            assert_eq!(class(&result), fields[6], "native result mode {mode}");
            assert_eq!(usize::from(result.resident_string_bytes().is_some()).to_string(), fields[7]);
            let normalized = cvt_to_numeric_in(dialect, result).unwrap();
            assert_eq!(class(&normalized), fields[6], "native normalized result mode {mode}");
            assert_eq!(normalized.native_string_bytes(dialect.native_string_protocol().unwrap()).unwrap().as_ref(), fields[8].as_bytes());
            compared += 1;
        }
        assert_eq!(compared, 7);
    }

    fn arith(op: BinOp, a: &Value, b: &Value) -> Result<Value, TclError> {
        arith_in(
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0),
            op,
            a,
            b,
        )
    }

    fn arithmetic_profiles() -> Vec<&'static tcl_dialect::DialectProfile> {
        let mut profiles: Vec<_> = tcl_dialect::TclVersion::ALL
            .iter()
            .map(|version| {
                tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap()
            })
            .collect();
        profiles.push(tcl_registry::model::ingress::resolve_environment("jim").analyser_profile());
        profiles
    }

    fn native_case(
        profile: &'static tcl_dialect::DialectProfile,
        expression: &str,
        expected: &str,
    ) {
        for dynamic in [false, true] {
            let mut vm = Vm::new();
            vm.set_compiler(Box::new(
                tcl_compiler::compile_service::BytecodeCompileService::default(),
            ));
            vm.set_dialect_profile(profile);
            let script = if dynamic {
                format!("set native_expression {{{expression}}}; expr $native_expression")
            } else {
                format!("expr {{{expression}}}")
            };
            let result = vm.eval_source(&script).unwrap();
            assert_eq!(
                result.code,
                Code::Ok,
                "{}: {script}: {}",
                profile.display_name,
                result.result.to_str()
            );
            assert_eq!(
                &*result.result.to_str(),
                expected,
                "{}: {script}",
                profile.display_name
            );
        }
    }

    #[test]
    fn original_expression_bytes_keep_leaf_values_and_cached_source() {
        for profile in arithmetic_profiles() {
            let mut vm = Vm::new();
            vm.set_dialect_profile(profile);
            vm.set_var_bytes(b"\xff", Value::from_native_string_bytes(&b"7"[..]))
                .expect("original byte variable name");
            for (source, expected) in [
                (&b"\"\xff\0tail\""[..], &b"\xff\0tail"[..]),
                (&b"{\xff\0tail}"[..], &b"\xff\0tail"[..]),
                (&b"${\xff}"[..], &b"7"[..]),
                (&b"\"${\xff}\""[..], &b"7"[..]),
                (&b"0 && \"${\xffmissing}\""[..], &b"0"[..]),
            ] {
                let object = Value::from_native_string_bytes(source);
                for _ in 0..2 {
                    let node = vm
                        .prepare_expression_value(&object)
                        .expect("native original expression source");
                    let mut ops = ExprEval::new(&mut vm);
                    let value = tcl_syntax::expr::eval(&node, &mut ops)
                        .expect("native byte expression evaluation");
                    assert_eq!(
                        value.string_bytes().as_ref(),
                        expected,
                        "{}",
                        profile.display_name
                    );
                    assert_eq!(object.string_bytes().as_ref(), source);
                }
            }
        }
    }

    #[test]
    fn operand_failures_keep_logical_simulation_separate_from_the_host_engine() {
        let mut vm = Vm::new();
        vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
        let host = crate::environment::profile_for_dialect("tcl9.0");
        assert!(vm.set_native_engine_profile(host));
        let invalid = Value::from_string_bytes(b"bad\xff\0tail".as_slice());
        let refused = arith_in(vm.numeric_context(), BinOp::Add, &invalid, &Value::int(1))
            .expect_err("logical numeric provider is required");
        assert!(refused.is_host());
        assert!(vm.set_logical_numeric_provider(
            tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation::Tcl84Core,
        ));
        let context = vm.numeric_context();
        let error = arith_in(context, BinOp::Add, &invalid, &Value::int(1))
            .expect_err("authored logical operand rejection");
        assert_eq!(
            error.message_bytes().unwrap().as_ref(),
            b"can't use non-numeric string as operand of \"+\""
        );
        assert_eq!(
            error.error_code_bytes().as_deref(),
            Some(b"NONE".as_slice())
        );
        assert!(invalid.integer_representation().is_none());
        let boolean = boolean_operand_error(
            context,
            native_boolean(context, &invalid).expect_err("full logical boolean spelling"),
        );
        assert_eq!(
            boolean.message_bytes().unwrap().as_ref(),
            b"expected boolean value but got \"bad\xff\0tail\""
        );
        assert_eq!(
            boolean.error_code_bytes().as_deref(),
            Some(b"NONE".as_slice())
        );
        let float = unary_in(context, UnaryOp::BitNot, &Value::double(1.5))
            .expect_err("integer-only logical operator");
        assert_eq!(
            float.message_bytes().unwrap().as_ref(),
            b"can't use floating-point value as operand of \"~\""
        );
        assert_eq!(
            float.error_code_bytes().as_deref(),
            Some(b"NONE".as_slice())
        );
        let older_native =
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_5);
        for error in [
            arith_in(
                older_native,
                BinOp::BitAnd,
                &Value::double(1.5),
                &Value::int(1),
            )
            .expect_err("selected older native integer-only binary error"),
            unary_in(older_native, UnaryOp::BitNot, &Value::double(1.5))
                .expect_err("selected older native integer-only unary error"),
        ] {
            assert!(
                error
                    .message_bytes()
                    .unwrap()
                    .starts_with(b"can't use floating-point value as operand of")
            );
            assert_eq!(
                error.error_code_bytes().as_deref(),
                Some(b"ARITH DOMAIN {floating-point value}".as_slice())
            );
        }
        let native = tcl_registry::InvocationDialect::of_profile(host);
        let error = arith_in(native, BinOp::Add, &Value::string("bad"), &Value::int(1))
            .expect_err("actual host operand rejection");
        assert_eq!(
            error.message_bytes().unwrap().as_ref(),
            b"cannot use non-numeric string \"bad\" as left operand of \"+\""
        );
        assert_eq!(
            error.error_code_bytes().as_deref(),
            Some(b"ARITH DOMAIN {non-numeric string}".as_slice())
        );
    }

    #[test]
    fn reached_operand_error_codes_follow_the_actual_expression_release() {
        for version in [tcl_dialect::TclVersion::V8_4, tcl_dialect::TclVersion::V8_5] {
            let dialect = tcl_registry::InvocationDialect::for_version(version);
            let value = Value::string("invalid");
            let arithmetic = arith_in(dialect, BinOp::Add, &value, &Value::int(1))
                .expect_err("non-numeric operand");
            let boolean = boolean_operand_error(
                dialect,
                native_boolean(dialect, &value).expect_err("non-boolean operand"),
            );
            if version == tcl_dialect::TclVersion::V8_4 {
                assert_eq!(
                    arithmetic.error_code_bytes().as_deref(),
                    Some(b"NONE".as_slice())
                );
                assert_eq!(
                    boolean.error_code_bytes().as_deref(),
                    Some(b"NONE".as_slice())
                );
            } else {
                assert_eq!(
                    arithmetic.error_code_bytes().as_deref(),
                    Some(b"ARITH DOMAIN {non-numeric string}".as_slice())
                );
                assert_eq!(
                    boolean.error_code_bytes().as_deref(),
                    Some(b"TCL VALUE NUMBER".as_slice())
                );
            }
        }
    }

    /// Fixed expectations come from C8.4–9.1 and Jim. Each runs through
    /// bytecode arithmetic and through the dynamic expression parser.
    #[test]
    fn checked_equality_ordering_and_mathop_preserve_distinct_native_object_protocols() {
        let mut vm = Vm::new();
        vm.set_dialect_profile(
            tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
        );
        let dialect = vm.native_invocation_dialect();
        let raw = Value::from_string_bytes([0xff]);
        let unicode = Value::string("ÿ");
        assert!(!compare_in(dialect, BinOp::StrEq, &raw, &unicode).unwrap());
        assert!(compare_in(dialect, BinOp::Eq, &raw, &unicode).unwrap());
        let cached = Value::native_jim_string(&[0xc3, b'A', 0xc3, 0xa9], 2);
        let recounted = Value::from_string_bytes([0xc3, b'A', 0xc3, 0xa9]);
        assert!(compare_in(dialect, BinOp::StrEq, &cached, &recounted).unwrap());
        assert!(!compare_in(dialect, BinOp::Eq, &cached, &recounted).unwrap());
        let mut ops = ExprEval::new(&mut vm);
        let equal =
            tcl_cmd_core::mathop::eval(&mut ops, "eq", vec![cached.clone(), recounted.clone()])
                .unwrap_or_else(|_| panic!("known byte equality succeeds"));
        assert!(equal.as_bool().unwrap());
        let ordered = tcl_cmd_core::mathop::eval(&mut ops, "lt", vec![cached, recounted])
            .unwrap_or_else(|_| panic!("known cached ordering succeeds"));
        assert!(ordered.as_bool().unwrap());
        vm.set_dialect_profile(tcl_dialect::DialectProfile::find("tcl9.0").unwrap());
        let error =
            tcl_cmd_core::mathop::eval(&mut ExprEval::new(&mut vm), "lt", vec![raw, unicode])
                .unwrap_err();
        let tcl_cmd_core::mathop::MathopError::Op(error) = error else {
            panic!("checked comparison error")
        };
        assert!(error.is_host());
        assert!(
            compare_string_units(None, &Value::string("a"), &Value::string("b"))
                .unwrap_err()
                .native_access_refusal()
                .is_some()
        );
    }

    #[test]
    fn native_integer_arithmetic_uses_the_selected_engine_on_both_paths() {
        let cases = [
            (
                "9223372036854775807 * 2",
                "-2",
                "18446744073709551614",
                "-2",
            ),
            ("(9223372036854775807 + 1) < 0", "1", "0", "1"),
            ("-(-9223372036854775807 - 1) < 0", "1", "0", "1"),
            ("((-9223372036854775807 - 1) / -1) < 0", "1", "0", "1"),
            (
                "18446744073709551615 + 0",
                "-1",
                "18446744073709551615",
                "-1",
            ),
            ("1 << 64", "0", "18446744073709551616", "1"),
            ("8 >> 64", "0", "0", "8"),
            ("-1 >> 64", "-1", "-1", "-1"),
            ("-7 / 3", "-3", "-3", "-3"),
            ("7 / -3", "-3", "-3", "-3"),
            ("-7 % 3", "2", "2", "2"),
            ("7 % -3", "-2", "-2", "-2"),
        ];
        for profile in arithmetic_profiles() {
            for (expression, tcl84, modern, jim) in cases {
                let expected = match tcl_registry::InvocationDialect::of_profile(profile)
                    .arithmetic()
                    .unwrap()
                {
                    tcl_dialect::NativeArithmetic::Tcl84Wide => tcl84,
                    tcl_dialect::NativeArithmetic::TclBignum => modern,
                    tcl_dialect::NativeArithmetic::JimWide => jim,
                };
                native_case(profile, expression, expected);
            }
        }
    }

    #[test]
    fn native_literal_and_final_string_conversion_are_distinct() {
        for profile in arithmetic_profiles() {
            let policy = tcl_registry::InvocationDialect::of_profile(profile)
                .arithmetic()
                .unwrap();
            native_case(
                profile,
                "\"001\"",
                if policy.normalizes_expression_result() {
                    "1"
                } else {
                    "001"
                },
            );
            native_case(profile, "1e3", "1000.0");
            native_case(profile, "\"18446744073709551616\"", "18446744073709551616");
            if policy == tcl_dialect::NativeArithmetic::Tcl84Wide {
                let mut vm = Vm::new();
                vm.set_compiler(Box::new(
                    tcl_compiler::compile_service::BytecodeCompileService::default(),
                ));
                vm.set_dialect_profile(profile);
                let result = vm.eval_source("expr {18446744073709551616 + 0}").unwrap();
                assert_eq!(result.code, Code::Error);
                assert_eq!(
                    &*result.result.to_str(),
                    "integer value too large to represent"
                );
            } else {
                native_case(
                    profile,
                    "18446744073709551616 + 0",
                    if policy == tcl_dialect::NativeArithmetic::JimWide {
                        "-1"
                    } else {
                        "18446744073709551616"
                    },
                );
            }
        }
    }

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
            vm.eval_expr("user_supplied(1)")
                .unwrap_err()
                .message_unicode()
                .expect("Unicode fixture error")
                .to_string(),
            "unknown math function \"user_supplied\""
        );
        // Host registrations remain visible independently of absent stock commands.
        let replacement = vm.invoke_command("::tcl::mathfunc::sqrt", &[Value::int(4)]);
        assert_eq!(replacement.code, Code::Ok);
        assert_eq!(replacement.result.to_str().as_ref(), "99");
        // The default stock wrapper is absent before TIP 232.
        let mut stock = Vm::new();
        stock.set_runtime_version(tcl_dialect::TclVersion::V8_4);
        assert_eq!(
            stock
                .invoke_command("::tcl::mathfunc::sqrt", &[Value::int(4)])
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
                    .message_unicode()
                    .expect("Unicode fixture error")
                    .to_string()
                    .contains("invalid bareword"),
                "{source}"
            );
        }
        assert!(
            vm.eval_expr("NaN(1)x")
                .unwrap_err()
                .message_unicode()
                .expect("Unicode fixture error")
                .to_string()
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
        assert_eq!(
            div0.message_unicode()
                .expect("Unicode fixture error")
                .as_ref(),
            "divide by zero"
        );
        let mod0 = arith(BinOp::Mod, &Value::string(P200), &Value::int(0)).unwrap_err();
        assert_eq!(
            mod0.message_unicode()
                .expect("Unicode fixture error")
                .as_ref(),
            "divide by zero"
        );
    }

    /// The `**` and shift guards of the shared tower, with C's message text:
    /// the exponent limit is `MAX_EXPONENT` (2^28 - 1 — tclsh errors at
    /// `2**268435456` rather than compute a 33 MB number), a bignum
    /// exponent keeps the `0`/`±1` collapses, and a left-shift count past
    /// `INT_MAX` is C's overflow error rather than an astronomic attempt.
    #[test]
    fn pow_and_shift_guards_match_c() {
        let errmsg = |op: BinOp, a: &Value, b: &Value| {
            arith(op, a, b)
                .unwrap_err()
                .message_unicode()
                .expect("Unicode fixture error")
                .to_string()
        };
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
            e.message_unicode().expect("Unicode fixture error").as_ref(),
            "cannot use floating-point value \"1.5\" as right operand of \"%\""
        );
        let e = arith(BinOp::Mod, &Value::double(1.5), &s(p200)).unwrap_err();
        assert_eq!(
            e.message_unicode().expect("Unicode fixture error").as_ref(),
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

#[cfg(test)]
#[path = "expr_float_tests.rs"]
mod float_error_tests;
