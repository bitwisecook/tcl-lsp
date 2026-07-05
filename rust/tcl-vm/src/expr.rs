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
#![allow(clippy::cast_precision_loss)]

use std::cmp::Ordering;

use tcl_runtime_api::Code;
use tcl_syntax::expr::{BinOp, ExprOps, UnaryOp};
use tcl_syntax::number::{self, Number};

use crate::error::TclError;
use crate::interp::Vm;
use crate::value::Value;

/// A coerced numeric operand. `Big` carries an out-of-`i64` integer that still
/// fits `i128` — the VM's bounded stand-in for Tcl's arbitrary-precision
/// integers, so arithmetic promotes on overflow instead of wrapping.
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

fn ipow(mut base: i128, mut exp: i128) -> i128 {
    if exp < 0 {
        return match base {
            1 => 1,
            -1 => {
                if exp % 2 == 0 {
                    1
                } else {
                    -1
                }
            }
            _ => 0,
        };
    }
    let mut acc: i128 = 1;
    while exp > 0 {
        if exp & 1 == 1 {
            acc = acc.wrapping_mul(base);
        }
        exp >>= 1;
        if exp > 0 {
            base = base.wrapping_mul(base);
        }
    }
    acc
}

/// Integer arithmetic in `i128`, narrowing the result to a wide when it fits.
/// `i64`-range operands and results behave exactly as before; an `i64`-overflow
/// now promotes (e.g. `2**70`, `9223372036854775807 + 1`) instead of wrapping,
/// up to the `i128` range (the VM's bounded stand-in for Tcl's bignums — a
/// genuinely `i128`-overflowing result still wraps, lacking a wider rep).
fn int_arith(op: BinOp, x: i128, y: i128) -> Result<Value, TclError> {
    use BinOp::{Add, BitAnd, BitOr, BitXor, Div, LShift, Mod, Mul, Pow, RShift, Sub};
    let r = match op {
        Add => x.wrapping_add(y),
        Sub => x.wrapping_sub(y),
        Mul => x.wrapping_mul(y),
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
        Pow => ipow(x, y),
        LShift => {
            if (0..128).contains(&y) {
                x.wrapping_shl(u32::try_from(y).unwrap_or(0))
            } else if y >= 128 {
                0
            } else {
                return Err(TclError::new("negative shift argument"));
            }
        }
        RShift => {
            if (0..128).contains(&y) {
                x >> u32::try_from(y).unwrap_or(0)
            } else if y >= 128 {
                if x < 0 { -1 } else { 0 }
            } else {
                return Err(TclError::new("negative shift argument"));
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
    match (num_i128(x), num_i128(y)) {
        // Both operands are integers (in-range or i128-promoted): integer path.
        (Some(xi), Some(yi)) => int_arith(op, xi, yi),
        // An integer-only operator with a (valid) float operand: that operand is
        // the error. Name the offending side (the left operand wins if both are
        // floats, matching C's left-to-right operand check).
        _ if is_int_only(op) => {
            if num_i128(x).is_none() {
                Err(float_operand_err(a, "left", op))
            } else {
                Err(float_operand_err(b, "right", op))
            }
        }
        _ => dbl_arith(op, num_f(x), num_f(y)),
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
            Ok(Num::Dbl(f)) => Ok(Value::double(-f)),
            Err(_) => Err(unary_operand_err(v, "-")),
        },
        Pos => match to_num(v) {
            Ok(Num::Int(n)) => Ok(Value::int(n)),
            Ok(Num::Big(b)) => Ok(int_value(b)),
            Ok(Num::Dbl(f)) => Ok(Value::double(f)),
            Err(_) => Err(unary_operand_err(v, "+")),
        },
        // `~` needs an integer; a double is a "floating-point value" operand
        // error, a non-number a "non-numeric string" one.
        BitNot => match to_num(v) {
            Ok(Num::Int(n)) => Ok(Value::int(!n)),
            Ok(Num::Big(b)) => Ok(int_value(!b)),
            Ok(Num::Dbl(_)) | Err(_) => Err(unary_operand_err(v, "~")),
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
        let name = format!("tcl::mathfunc::{function}");
        match self.vm.dispatch_builtin(&name, &args) {
            Some(c) if c.code.is_ok() => Ok(c.result),
            Some(c) => Err(TclError::new(c.result.to_str().to_string())),
            None => Err(TclError::new(format!(
                "unknown math function \"{function}\""
            ))),
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
