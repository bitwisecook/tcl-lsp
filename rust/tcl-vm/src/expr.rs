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
use num_traits::{Signed, ToPrimitive, Zero};
use tcl_runtime_api::Code;
use tcl_syntax::expr::{BinOp, ExprOps, UnaryOp};
use tcl_syntax::number::{self, Number};

use crate::error::TclError;
use crate::interp::Vm;
use crate::value::Value;

/// A coerced numeric operand. `Big` carries an out-of-`i64` integer that still
/// fits `i128` — the fast integer tier; an operand or result beyond `i128`
/// promotes to the arbitrary-precision [`BigInt`] path (`RUST_ISSUE_011`) rather
/// than wrapping.
#[derive(Clone, Copy)]
enum Num {
    Int(i64),
    Big(i128),
    Dbl(f64),
}

fn num_f(n: Num) -> f64 {
    match n {
        Num::Int(i) => i as f64,
        Num::Big(i) => i as f64,
        Num::Dbl(f) => f,
    }
}

/// The integer (`i128`) view of a non-float operand, for the integer arithmetic
/// path. `None` for a float (which routes to `dbl_arith`).
fn num_i128(n: Num) -> Option<i128> {
    match n {
        Num::Int(i) => Some(i128::from(i)),
        Num::Big(i) => Some(i),
        Num::Dbl(_) => None,
    }
}

/// Wrap an `i128` arithmetic result as a value: a plain wide when it fits,
/// otherwise the decimal string (the VM has no wider integer rep).
pub(crate) fn int_value(r: i128) -> Value {
    i64::try_from(r).map_or_else(|_| Value::string(r.to_string()), Value::int)
}

/// Wrap an arbitrary-precision integer result as a value, narrowing to a wide
/// when it fits (so `2+2` via the bignum path is still `Value::int(4)`), else its
/// canonical decimal string (`RUST_ISSUE_011`).
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
/// operand already exceeds `i128`. Div/mod are floor (sign of divisor), matching
/// Tcl; bit ops use two's-complement (`num-bigint`'s semantics = Tcl's).
fn big_arith(op: BinOp, x: &BigInt, y: &BigInt) -> Result<Value, TclError> {
    use BinOp::{Add, BitAnd, BitOr, BitXor, Div, LShift, Mod, Mul, Pow, RShift, Sub};
    let r = match op {
        Add => x + y,
        Sub => x - y,
        Mul => x * y,
        Div => {
            if y.is_zero() {
                return Err(divzero());
            }
            x.div_floor(y)
        }
        Mod => {
            if y.is_zero() {
                return Err(divzero());
            }
            x.mod_floor(y)
        }
        Pow => return big_pow(x, y),
        LShift | RShift => {
            let Some(shift) = y.to_usize() else {
                if y.is_negative() {
                    return Err(TclError::new("negative shift argument"));
                }
                // A shift wider than any representable result: `<<` overflows to
                // an enormous value (unrepresentable — but such a huge shift is a
                // pathological program), `>>` collapses to the sign.
                return Ok(big_value(
                    &(if matches!(op, RShift) && x.is_negative() {
                        BigInt::from(-1)
                    } else {
                        BigInt::zero()
                    }),
                ));
            };
            if matches!(op, LShift) {
                x << shift
            } else {
                x >> shift
            }
        }
        BitAnd => x & y,
        BitOr => x | y,
        BitXor => x ^ y,
        _ => return Err(TclError::new("unsupported integer operator")),
    };
    Ok(big_value(&r))
}

/// `x ** y` in arbitrary precision. A negative exponent on an integer base is `0`
/// except for `±1` (C's `TclExecute` integer-power rule); a huge exponent that
/// can't fit `u32` is only reachable with `|base| <= 1` (else the result is
/// astronomically large), so those collapse to the base-driven value.
fn big_pow(x: &BigInt, y: &BigInt) -> Result<Value, TclError> {
    if y.is_negative() {
        let r = if x == &BigInt::from(1) {
            1
        } else if x == &BigInt::from(-1) {
            if y.is_even() { 1 } else { -1 }
        } else if x.is_zero() {
            return Err(TclError::new("exponentiation of zero by negative power"));
        } else {
            0
        };
        return Ok(Value::int(r));
    }
    if let Some(exp) = y.to_u32() {
        return Ok(big_value(&x.pow(exp)));
    }
    // Exponent beyond `u32`: only `|x| <= 1` yields a representable result.
    let r = if x == &BigInt::from(1) || x.is_zero() {
        x.clone()
    } else if x == &BigInt::from(-1) {
        BigInt::from(if y.is_even() { 1 } else { -1 })
    } else {
        return Err(TclError::new("exponent too large"));
    };
    Ok(big_value(&r))
}

fn to_num(v: &Value) -> Result<Num, TclError> {
    if let Ok(n) = v.as_int() {
        return Ok(Num::Int(n));
    }
    if let Some(b) = v.as_i128() {
        return Ok(Num::Big(b));
    }
    match v.as_double() {
        Ok(f) => Ok(Num::Dbl(f)),
        Err(_) => Err(TclError::new(format!(
            "can't use non-numeric string \"{}\" as operand of arithmetic",
            v.to_str()
        ))),
    }
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
    // decimal string, rather than degrading to a lossy `double` (`RUST_ISSUE_011`).
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
    // `nan` (and `±NaN`) is a valid floating-point *value* that simply cannot be
    // an arithmetic operand — C words that differently from a non-number string.
    let s = v.to_str();
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

fn divzero() -> TclError {
    TclError::new("divide by zero")
}

/// The C `IllegalExprOperandType` message for a *unary* operator whose operand
/// cannot be used: `cannot use <desc> "<v>" as operand of "<op>"`. `<desc>` is
/// `floating-point value` (a double handed to `~`), `non-numeric floating-point
/// value` (NaN), `a list` (a multi-element list — phrased without quotes), or
/// `non-numeric string`. (`errorCode ARITH DOMAIN <desc>` is not threaded here;
/// the VM does not yet set arith error codes — same as `divide by zero`.)
fn unary_operand_err(v: &Value, op: &str) -> TclError {
    let s = v.to_str();
    if tcl_syntax::list::max_list_length(&s) > 1 && tcl_syntax::list::split_list(&s).is_ok() {
        return TclError::new(format!("cannot use a list as operand of \"{op}\""));
    }
    let desc = match number::parse_whole(s.trim()) {
        Some(Number::Double(_)) => "floating-point value",
        Some(Number::Nan { .. }) => "non-numeric floating-point value",
        _ => "non-numeric string",
    };
    TclError::new(format!("cannot use {desc} \"{s}\" as operand of \"{op}\""))
}

/// Floored integer division (Tcl `/`: rounds toward negative infinity).
fn fdiv(x: i128, y: i128) -> i128 {
    let q = x.wrapping_div(y);
    let r = x.wrapping_rem(y);
    if r != 0 && ((r < 0) != (y < 0)) {
        q - 1
    } else {
        q
    }
}

/// Floored integer modulo (Tcl `%`: result takes the sign of the divisor).
fn fmod_i(x: i128, y: i128) -> i128 {
    let r = x.wrapping_rem(y);
    if r != 0 && ((r < 0) != (y < 0)) {
        r + y
    } else {
        r
    }
}

/// Promote an `i128` integer operation that overflowed (or can't stay bounded)
/// to the arbitrary-precision path (`RUST_ISSUE_011`).
fn promote(op: BinOp, x: i128, y: i128) -> Result<Value, TclError> {
    big_arith(op, &BigInt::from(x), &BigInt::from(y))
}

/// Integer arithmetic in `i128`, narrowing the result to a wide when it fits and
/// **promoting to arbitrary precision on overflow** rather than wrapping: `2**70`
/// / `9223372036854775807 + 1` fit `i128`; `2**200` / a product past `2^127`
/// promote to a bignum, matching tclsh (`RUST_ISSUE_011`). `i64`-range operands
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
        Pow => x.powf(y),
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
    if let (Some(xi), Some(yi)) = (num_i128(x), num_i128(y)) {
        return int_arith(op, xi, yi);
    }
    // Not both `i128`-fit. When *both* are integers (an operand already past
    // `i128`), stay exact via the arbitrary-precision path (`RUST_ISSUE_011`).
    if let (Some(xb), Some(yb)) = (value_as_bigint(a), value_as_bigint(b)) {
        return big_arith(op, &xb, &yb);
    }
    // At least one operand is a genuine float (or non-number).
    if is_int_only(op) {
        // An integer-only operator with a float operand: that operand is the
        // error (a huge-integer operand took the bignum path above). Name the
        // offending side (left wins if both are floats, matching C's order).
        if value_as_bigint(a).is_none() {
            Err(float_operand_err(a, "left", op))
        } else {
            Err(float_operand_err(b, "right", op))
        }
    } else {
        dbl_arith(op, num_f(x), num_f(y))
    }
}

fn num_cmp(x: Num, y: Num) -> Ordering {
    match (num_i128(x), num_i128(y)) {
        (Some(a), Some(b)) => a.cmp(&b),
        _ => num_f(x).partial_cmp(&num_f(y)).unwrap_or(Ordering::Equal),
    }
}

/// Apply a comparison operator, returning the boolean result. Numeric when both
/// operands look numeric (or always-string for the `STR_*` variants), else a
/// string comparison — Tcl's `==`/`<`… rule.
pub fn compare(op: BinOp, a: &Value, b: &Value) -> Result<bool, TclError> {
    use BinOp::{Eq, Ge, Gt, Le, Lt, Ne, StrEq, StrGe, StrGt, StrLe, StrLt, StrNe};
    let str_ord = || (*a.to_str()).cmp(&b.to_str());
    let ord = match op {
        StrEq | StrNe | StrLt | StrLe | StrGt | StrGe => str_ord(),
        _ => match (to_num(a), to_num(b)) {
            // When an operand is an integer past `i128`, compare exactly as
            // bignums — `num_cmp`'s `f64` fallback would lose precision
            // (`RUST_ISSUE_011`); the common `i128`-fit case stays on `num_cmp`.
            (Ok(x), Ok(y)) if num_i128(x).is_none() || num_i128(y).is_none() => {
                match (value_as_bigint(a), value_as_bigint(b)) {
                    (Some(xb), Some(yb)) => xb.cmp(&yb),
                    _ => num_cmp(x, y),
                }
            }
            (Ok(x), Ok(y)) => num_cmp(x, y),
            _ => str_ord(),
        },
    };
    Ok(match op {
        Eq | StrEq => ord.is_eq(),
        Ne | StrNe => ord.is_ne(),
        Lt | StrLt => ord.is_lt(),
        Le | StrLe => ord.is_le(),
        Gt | StrGt => ord.is_gt(),
        Ge | StrGe => ord.is_ge(),
        _ => return Err(TclError::new("unsupported comparison operator")),
    })
}

/// Apply a unary operator.
pub fn unary(op: UnaryOp, v: &Value) -> Result<Value, TclError> {
    use UnaryOp::{BitNot, Neg, Not, Pos, WordNot};
    match op {
        // `to_num` promotes an out-of-wide literal to `Big`, so `-2^63` (and the
        // rest of the i128 range) negates correctly: `int_value` narrows
        // `-9223372036854775808` back to the most-negative wide. A non-numeric
        // operand is the C operand-type error (`as operand of "-"`).
        Neg => match to_num(v) {
            Ok(Num::Int(n)) => Ok(Value::int(n.wrapping_neg())),
            Ok(Num::Big(b)) => Ok(int_value(b.wrapping_neg())),
            // A huge integer (past `i128`) coerces to `Num::Dbl`; negate it
            // exactly as a bignum, not as a lossy float (`RUST_ISSUE_011`).
            Ok(Num::Dbl(f)) => {
                Ok(value_as_bigint(v).map_or_else(|| Value::double(-f), |b| big_value(&-b)))
            }
            Err(_) => Err(unary_operand_err(v, "-")),
        },
        Pos => match to_num(v) {
            Ok(Num::Int(n)) => Ok(Value::int(n)),
            Ok(Num::Big(b)) => Ok(int_value(b)),
            Ok(Num::Dbl(f)) => {
                Ok(value_as_bigint(v).map_or_else(|| Value::double(f), |b| big_value(&b)))
            }
            Err(_) => Err(unary_operand_err(v, "+")),
        },
        // `~` needs an integer; a double is a "floating-point value" operand
        // error, a non-number a "non-numeric string" one.
        BitNot => match to_num(v) {
            Ok(Num::Int(n)) => Ok(Value::int(!n)),
            Ok(Num::Big(b)) => Ok(int_value(!b)),
            // A huge integer bit-complements exactly (`~x == -x-1`); a real float
            // is the operand error.
            Ok(Num::Dbl(_)) => value_as_bigint(v)
                .map_or_else(|| Err(unary_operand_err(v, "~")), |b| Ok(big_value(&!b))),
            Err(_) => Err(unary_operand_err(v, "~")),
        },
        // `!` accepts any boolean (incl. numbers and the boolean words); a NaN or
        // non-numeric non-boolean is the operand error (not "expected boolean").
        Not => {
            if matches!(
                number::parse_whole(v.to_str().trim()),
                Some(Number::Nan { .. })
            ) {
                return Err(unary_operand_err(v, "!"));
            }
            match v.as_bool() {
                Ok(b) => Ok(Value::bool(!b)),
                Err(_) => Err(unary_operand_err(v, "!")),
            }
        }
        WordNot => Err(TclError::new("unsupported operator")),
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
        // TIP 232: a math function is an ordinary command, so a user
        // `proc tcl::mathfunc::f {…} {…}` (the canonical custom-function
        // mechanism) must dispatch exactly like a builtin one — full command
        // dispatch, not builtin-only. The name stays *relative*, so
        // resolution is current-namespace-first (a namespace-local
        // `tcl::mathfunc::f` shadows the global; tclsh-pinned by the
        // mathfunc conformance vectors).
        let name = format!("tcl::mathfunc::{function}");
        if self.vm.lookup_command(&name).is_none() {
            // C reports the command miss, not a special math-function error
            // (tclsh 8.6.16 / 9.0.4: `expr {frobnicate(1)}` →
            // `invalid command name "tcl::mathfunc::frobnicate"`).
            return Err(TclError::new(format!(
                "invalid command name \"tcl::mathfunc::{function}\""
            )));
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

    fn compare_numeric(&mut self, l: &Value, r: &Value) -> Option<Ordering> {
        match (to_num(l), to_num(r)) {
            (Ok(x), Ok(y)) => Some(num_cmp(x, y)),
            _ => None,
        }
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

    /// The arbitrary-precision integer tower (`RUST_ISSUE_011`): operations that
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
        assert!(arith(BinOp::Pow, &Value::int(0), &Value::int(-1)).is_err());
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
}
