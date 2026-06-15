//! Expression and arithmetic semantics.
//!
//! The numeric/comparison/unary logic is single-sourced here and used by *both*
//! the inline arithmetic opcodes (`ADD`/`LT`/`UMINUS`/…) and `EXPR_STK` (via the
//! [`ExprEval`] adapter over [`tcl_syntax::expr::ExprOps`]), so the VM shares
//! the expr tower with the const-folder and the runtime.
#![allow(clippy::cast_precision_loss)]

use std::cmp::Ordering;

use tcl_syntax::expr::{BinOp, ExprOps, UnaryOp};
use tcl_syntax::number::{self, Number};

use crate::error::TclError;
use crate::interp::Vm;
use crate::value::Value;

/// A coerced numeric operand.
#[derive(Clone, Copy)]
enum Num {
    Int(i64),
    Dbl(f64),
}

fn num_f(n: Num) -> f64 {
    match n {
        Num::Int(i) => i as f64,
        Num::Dbl(f) => f,
    }
}

fn to_num(v: &Value) -> Result<Num, TclError> {
    if let Ok(n) = v.as_int() {
        return Ok(Num::Int(n));
    }
    match v.as_double() {
        Ok(f) => Ok(Num::Dbl(f)),
        Err(_) => Err(TclError::new(format!(
            "can't use non-numeric string \"{}\" as operand of arithmetic",
            v.to_str()
        ))),
    }
}

fn divzero() -> TclError {
    TclError::new("divide by zero")
}

/// Floored integer division (Tcl `/`: rounds toward negative infinity).
fn fdiv(x: i64, y: i64) -> i64 {
    let q = x.wrapping_div(y);
    let r = x.wrapping_rem(y);
    if r != 0 && ((r < 0) != (y < 0)) {
        q - 1
    } else {
        q
    }
}

/// Floored integer modulo (Tcl `%`: result takes the sign of the divisor).
fn fmod_i(x: i64, y: i64) -> i64 {
    let r = x.wrapping_rem(y);
    if r != 0 && ((r < 0) != (y < 0)) {
        r + y
    } else {
        r
    }
}

fn ipow(mut base: i64, mut exp: i64) -> i64 {
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
    let mut acc: i64 = 1;
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

fn int_arith(op: BinOp, x: i64, y: i64) -> Result<Value, TclError> {
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
            if (0..64).contains(&y) {
                x.wrapping_shl(u32::try_from(y).unwrap_or(0))
            } else if y >= 64 {
                0
            } else {
                return Err(TclError::new("negative shift count"));
            }
        }
        RShift => {
            if (0..64).contains(&y) {
                x >> u32::try_from(y).unwrap_or(0)
            } else if y >= 64 {
                if x < 0 { -1 } else { 0 }
            } else {
                return Err(TclError::new("negative shift count"));
            }
        }
        BitAnd => x & y,
        BitOr => x | y,
        BitXor => x ^ y,
        _ => return Err(TclError::new("unsupported integer operator")),
    };
    Ok(Value::int(r))
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
    Ok(Value::double(r))
}

/// Apply an arithmetic / bitwise / shift binary operator to two values.
pub fn arith(op: BinOp, a: &Value, b: &Value) -> Result<Value, TclError> {
    match (to_num(a)?, to_num(b)?) {
        (Num::Int(x), Num::Int(y)) => int_arith(op, x, y),
        (x, y) => dbl_arith(op, num_f(x), num_f(y)),
    }
}

fn num_cmp(x: Num, y: Num) -> Ordering {
    match (x, y) {
        (Num::Int(a), Num::Int(b)) => a.cmp(&b),
        (a, b) => num_f(a).partial_cmp(&num_f(b)).unwrap_or(Ordering::Equal),
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
        Neg => Ok(match to_num(v)? {
            Num::Int(n) => Value::int(n.wrapping_neg()),
            Num::Dbl(f) => Value::double(-f),
        }),
        Pos => Ok(match to_num(v)? {
            Num::Int(n) => Value::int(n),
            Num::Dbl(f) => Value::double(f),
        }),
        BitNot => Ok(Value::int(!v.as_int()?)),
        Not => Ok(Value::bool(!v.as_bool()?)),
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
        Ok(Value::string(inner))
    }

    fn var(&mut self, name: &str) -> Result<Value, TclError> {
        self.vm
            .get_var(name)
            .ok_or_else(|| TclError::new(format!("can't read \"{name}\": no such variable")))
    }

    fn command(&mut self, script: &str) -> Result<Value, TclError> {
        self.vm.eval_source(script).map(|c| c.result)
    }

    fn call(&mut self, function: &str, _args: Vec<Value>) -> Result<Value, TclError> {
        Err(TclError::new(format!(
            "unknown math function \"{function}\""
        )))
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
