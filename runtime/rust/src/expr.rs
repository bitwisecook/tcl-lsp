//! The runtime's `expr` value operations — a thin [`tcl_syntax::expr::ExprOps`]
//! implementation over the numeric tower ([`crate::bignum`]). The evaluation
//! *walk* (operator dispatch, short-circuit `&&`/`||`, `?:`, the
//! numeric-vs-string comparison rule, `eq`/`ne`/`in`/`ni`) is **shared** with the
//! compiler via `tcl-syntax` (the same way the lexer/parser are shared); only the
//! value-type-specific bits live here — the tower arithmetic, `Tcl_Obj`
//! construction, `$var`/`[cmd]` resolution, and boolean coercion.
//!
//! `$var`/`[cmd]` resolve through caller closures (the interp wires its frame +
//! eval machinery; tests use mocks). Refcounts are managed by the [`Owned`] RAII
//! guard so every early return in the shared walk releases cleanly.

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use core::cmp::Ordering;

use crate::bignum::{self, ArithError};
use crate::obj::{self, TclObj};
use tcl_syntax::expr::{eval, BinOp, ExprNode, ExprOps, UnaryOp};

/// An expr-evaluation error carrying Tcl's verbatim message bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprError(pub Vec<u8>);

impl ExprError {
    fn msg(s: &[u8]) -> ExprError {
        ExprError(s.to_vec())
    }
}

fn arith_err(e: ArithError) -> ExprError {
    ExprError::msg(match e {
        ArithError::NonNumeric => b"can't use non-numeric string as operand of arithmetic",
        ArithError::NonInteger => b"can't use floating-point value as operand of bitwise op",
        ArithError::DivideByZero => b"divide by zero",
        ArithError::NegativeShift => b"negative shift argument",
        ArithError::ExponentTooLarge => b"exponent too large",
        ArithError::Alloc => b"out of memory",
    })
}

/// An owned object reference (`rc +1`) that releases on drop — the discipline
/// that keeps the shared recursive walk leak-/double-free-safe across early
/// returns.
pub struct Owned(*mut TclObj);

impl Owned {
    /// Take an owning `+1` on a live object (e.g. a variable's store value).
    pub fn retain(o: *mut TclObj) -> Owned {
        // SAFETY: `o` is a live object.
        unsafe { obj::incr_ref_count(o) };
        Owned(o)
    }

    /// Adopt a freshly-minted (`rc 0`) object, taking it to `rc 1`.
    fn fresh(o: *mut TclObj) -> Owned {
        // SAFETY: `o` is a fresh object from a constructor / tower op.
        unsafe { obj::incr_ref_count(o) };
        Owned(o)
    }

    #[inline]
    fn ptr(&self) -> *mut TclObj {
        self.0
    }

    /// The borrowed object pointer (the `+1` stays with this `Owned`). Callers
    /// that retain it (e.g. `Tcl_SetObjResult`, which takes its own `+1`) read
    /// through this and let the `Owned` drop its reference normally.
    #[inline]
    #[must_use]
    pub fn as_ptr(&self) -> *mut TclObj {
        self.0
    }

    /// Hand the `+1` to the caller without releasing it here.
    pub fn into_raw(self) -> *mut TclObj {
        let o = self.0;
        core::mem::forget(self);
        o
    }
}

impl Drop for Owned {
    fn drop(&mut self) {
        // SAFETY: `self.0` is the object we hold a `+1` on.
        unsafe { obj::decr_ref_count(self.0) };
    }
}

/// The evaluation context the tower `ExprOps` resolves `$var`/`[cmd]` through —
/// one `&mut` borrow (the interp implements this; tests use a mock). A single
/// trait (vs two closures) avoids double-borrowing the interp for var-read +
/// command-eval.
pub trait ExprCtx {
    /// Resolve a `$name` reference to an owned value, or `Err` (`can't read …`).
    fn read_var(&mut self, name: &str) -> Result<Owned, ExprError>;
    /// Evaluate a `[script]` (brackets stripped) to an owned result.
    fn eval_command(&mut self, script: &str) -> Result<Owned, ExprError>;
    /// Evaluate a `func(args…)` math-function call. The interp routes this
    /// through the command table (`::tcl::mathfunc::func`, so user overrides
    /// win — the A3 contract); the standalone evaluator falls back to the shared
    /// [`dispatch_shared`] built-in dispatch.
    fn call_function(&mut self, name: &str, args: &[Owned]) -> Result<Owned, ExprError>;
}

/// The shared built-in math-function dispatch over the tower
/// ([`tcl_syntax::expr::mathfunc`]) — the fallback when a function isn't an
/// overridable command. `args` are the already-evaluated operands.
pub fn dispatch_shared(name: &str, args: &[Owned]) -> Result<Owned, ExprError> {
    use tcl_syntax::expr::mathfunc::{dispatch, Num};
    let nums: Option<Vec<Num>> = args
        .iter()
        .map(|o| crate::bignum::as_math_num(o.ptr()))
        .collect();
    let nums =
        nums.ok_or_else(|| ExprError::msg(b"argument to math function didn't have numeric value"))?;
    match dispatch(&name.to_ascii_lowercase(), &nums) {
        Some(Num::Int(i)) => Ok(Owned::fresh(obj::new_wide_int_obj(i))),
        Some(Num::Float(f)) => Ok(Owned::fresh(obj::new_double_obj(f))),
        None => {
            let mut m = b"unknown math function \"".to_vec();
            m.extend_from_slice(name.as_bytes());
            m.push(b'"');
            Err(ExprError(m))
        }
    }
}

/// The tower [`ExprOps`] over an [`ExprCtx`].
struct TowerOps<'a> {
    ctx: &'a mut dyn ExprCtx,
}

impl ExprOps for TowerOps<'_> {
    type Value = Owned;
    type Error = ExprError;

    fn literal(&mut self, text: &str) -> Result<Owned, ExprError> {
        Ok(make_literal(text))
    }
    fn string(&mut self, inner: &str) -> Result<Owned, ExprError> {
        Ok(Owned::fresh(obj::new_string_bytes(inner.as_bytes())))
    }
    fn var(&mut self, name: &str) -> Result<Owned, ExprError> {
        self.ctx.read_var(name)
    }
    fn command(&mut self, script: &str) -> Result<Owned, ExprError> {
        self.ctx.eval_command(script)
    }
    fn call(&mut self, function: &str, args: Vec<Owned>) -> Result<Owned, ExprError> {
        // Resolve `func(…)` through the context: the interp routes it to the
        // command table (`::tcl::mathfunc::func`, so overrides/renames win — A3),
        // falling back to the shared built-in dispatch for the standalone case.
        self.ctx.call_function(function, &args)
    }

    fn arith(&mut self, op: BinOp, left: Owned, right: Owned) -> Result<Owned, ExprError> {
        let (lp, rp) = (left.ptr(), right.ptr());
        let res = match op {
            BinOp::Add => bignum::add(lp, rp),
            BinOp::Sub => bignum::sub(lp, rp),
            BinOp::Mul => bignum::mul(lp, rp),
            BinOp::Div => bignum::div(lp, rp),
            BinOp::Mod => bignum::mod_(lp, rp),
            BinOp::Pow => bignum::pow(lp, rp),
            BinOp::BitAnd => bignum::band(lp, rp),
            BinOp::BitOr => bignum::bor(lp, rp),
            BinOp::BitXor => bignum::bxor(lp, rp),
            BinOp::LShift => bignum::shl(lp, rp),
            BinOp::RShift => bignum::shr(lp, rp),
            _ => return Err(ExprError::msg(b"unsupported operator")),
        };
        // `left`/`right` stay alive until here, then release.
        Ok(Owned::fresh(res.map_err(arith_err)?))
    }

    fn unary(&mut self, op: UnaryOp, value: Owned) -> Result<Owned, ExprError> {
        match op {
            UnaryOp::Pos => {
                if bignum::compare(value.ptr(), value.ptr()).is_none() {
                    return Err(arith_err(ArithError::NonNumeric));
                }
                Ok(value)
            }
            UnaryOp::Neg => Ok(Owned::fresh(bignum::neg(value.ptr()).map_err(arith_err)?)),
            UnaryOp::BitNot => Ok(Owned::fresh(bignum::bnot(value.ptr()).map_err(arith_err)?)),
            UnaryOp::Not => {
                let b = to_bool(value.ptr())?;
                Ok(bool_obj(!b))
            }
            UnaryOp::WordNot => Err(ExprError::msg(b"unsupported operator")),
        }
    }

    fn compare_numeric(&mut self, left: &Owned, right: &Owned) -> Option<Ordering> {
        bignum::compare(left.ptr(), right.ptr())
    }
    fn compare_string(&mut self, left: &Owned, right: &Owned) -> Ordering {
        obj::bytes_of(left.ptr()).cmp(&obj::bytes_of(right.ptr()))
    }
    fn in_list(&mut self, needle: &Owned, list: &Owned) -> Result<bool, ExprError> {
        let hay = obj::bytes_of(list.ptr());
        let s = core::str::from_utf8(&hay).map_err(|_| ExprError::msg(b"invalid list"))?;
        let elems = tcl_syntax::list::split_list(s).map_err(|_| ExprError::msg(b"invalid list"))?;
        let n = obj::bytes_of(needle.ptr());
        Ok(elems.iter().any(|e| e.as_bytes() == n.as_slice()))
    }

    fn to_bool(&mut self, value: &Owned) -> Result<bool, ExprError> {
        to_bool(value.ptr())
    }
    fn bool_value(&mut self, b: bool) -> Owned {
        bool_obj(b)
    }
    fn unsupported(&mut self, what: &str) -> ExprError {
        ExprError(what.as_bytes().to_vec())
    }
}

/// Evaluate `node` over the tower, resolving `$var`/`[cmd]` via `ctx`.
pub fn eval_expr(node: &ExprNode, ctx: &mut dyn ExprCtx) -> Result<Owned, ExprError> {
    let mut ops = TowerOps { ctx };
    eval(node, &mut ops)
}

// ---- value helpers ---------------------------------------------------------

/// Tcl boolean coercion (`Tcl_GetBoolean`): the keywords or any non-zero number.
fn to_bool(o: *mut TclObj) -> Result<bool, ExprError> {
    let bytes = obj::bytes_of(o);
    let s = core::str::from_utf8(&bytes).unwrap_or("");
    match s.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => return Ok(true),
        "0" | "false" | "no" | "off" => return Ok(false),
        _ => {}
    }
    // Any number: non-zero ⇒ true.
    let zero = Owned::fresh(obj::new_wide_int_obj(0));
    match bignum::compare(o, zero.ptr()) {
        Some(ord) => Ok(!ord.is_eq()),
        None => {
            let mut m = b"expected boolean value but got \"".to_vec();
            m.extend_from_slice(&bytes);
            m.push(b'"');
            Err(ExprError(m))
        }
    }
}

fn bool_obj(b: bool) -> Owned {
    Owned::fresh(obj::new_wide_int_obj(i64::from(b)))
}

/// Build an object from a literal token: a number (via the shared grammar), a
/// boolean keyword, else a plain string.
fn make_literal(text: &str) -> Owned {
    use tcl_syntax::number::{parse_whole, Number};
    if let Some(n) = parse_whole(text) {
        return Owned::fresh(match n {
            Number::Int(v) => obj::new_wide_int_obj(v),
            Number::Double(d) => obj::new_double_obj(d),
            Number::Big {
                negative,
                radix,
                digits,
            } => bignum::from_big_digits(negative, radix, &digits),
            Number::Nan { .. } => obj::new_double_obj(f64::NAN),
        });
    }
    match text.to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" => return bool_obj(true),
        "false" | "no" | "off" => return bool_obj(false),
        _ => {}
    }
    Owned::fresh(obj::new_string_bytes(text.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_syntax::expr::parser::parse_expr;

    /// A mock context: a `$var` table; `[cmd]` is unsupported in these tests.
    struct MockCtx(std::collections::HashMap<String, i64>);
    impl ExprCtx for MockCtx {
        fn read_var(&mut self, name: &str) -> Result<Owned, ExprError> {
            self.0
                .get(name)
                .map(|&v| Owned::fresh(obj::new_wide_int_obj(v)))
                .ok_or_else(|| ExprError::msg(b"can't read var"))
        }
        fn eval_command(&mut self, _script: &str) -> Result<Owned, ExprError> {
            Err(ExprError::msg(b"no commands"))
        }
        fn call_function(&mut self, name: &str, args: &[Owned]) -> Result<Owned, ExprError> {
            // No command table in the mock — use the shared built-in dispatch.
            dispatch_shared(name, args)
        }
    }

    fn ev(src: &str, vars: &[(&str, i64)]) -> Result<Vec<u8>, ExprError> {
        crate::counters::reset();
        let node = parse_expr(src, None);
        let mut ctx = MockCtx(vars.iter().map(|&(k, v)| (k.to_string(), v)).collect());
        let r = eval_expr(&node, &mut ctx)?;
        let out = obj::bytes_of(r.ptr());
        drop(r);
        assert_eq!(crate::counters::finalize(), 0, "leak");
        Ok(out)
    }

    fn ok(src: &str) -> Vec<u8> {
        ev(src, &[]).expect("eval")
    }

    #[test]
    fn arithmetic_and_precedence() {
        assert_eq!(ok("1 + 2 * 3"), b"7");
        assert_eq!(ok("(1 + 2) * 3"), b"9");
        assert_eq!(ok("2 ** 10"), b"1024");
        assert_eq!(ok("2 ** 64"), b"18446744073709551616"); // bignum
        assert_eq!(ok("7 / 2"), b"3");
        assert_eq!(ok("-7 / 2"), b"-4"); // floor
        assert_eq!(ok("7 % 3"), b"1");
        assert_eq!(ok("1 + 2.5"), b"3.5"); // double promotion
    }

    #[test]
    fn bitwise_and_shifts() {
        assert_eq!(ok("0xff & 0x0f"), b"15");
        assert_eq!(ok("12 | 3"), b"15");
        assert_eq!(ok("5 ^ 3"), b"6");
        assert_eq!(ok("~5"), b"-6");
        assert_eq!(ok("1 << 4"), b"16");
        assert_eq!(ok("256 >> 4"), b"16");
    }

    #[test]
    fn comparisons_numeric_and_string() {
        assert_eq!(ok("3 < 5"), b"1");
        assert_eq!(ok("5 <= 5"), b"1");
        assert_eq!(ok("3 == 3"), b"1");
        assert_eq!(ok("3 != 4"), b"1");
        assert_eq!(ok(r#""abc" eq "abc""#), b"1");
        assert_eq!(ok(r#""abc" ne "abd""#), b"1");
    }

    #[test]
    fn short_circuit_and_ternary() {
        assert_eq!(ok("1 && 0"), b"0");
        assert_eq!(ok("0 || 1"), b"1");
        assert_eq!(ok("!0"), b"1");
        assert_eq!(ok("1 ? 42 : 99"), b"42");
        assert_eq!(ok("0 ? 42 : 99"), b"99");
    }

    #[test]
    fn variables_and_membership() {
        assert_eq!(ev("$x + $y", &[("x", 10), ("y", 32)]).unwrap(), b"42");
        assert_eq!(ev("$x * 2", &[("x", 21)]).unwrap(), b"42");
        assert_eq!(ok("3 in {1 2 3 4}"), b"1");
        assert_eq!(ok("9 ni {1 2 3 4}"), b"1");
    }

    #[test]
    fn math_functions() {
        // the shared tcl_syntax::expr::mathfunc dispatch, over the tower
        assert_eq!(ok("sqrt(4)"), b"2.0");
        assert_eq!(ok("max(1, 9, 3)"), b"9");
        assert_eq!(ok("min(5, 2)"), b"2");
        assert_eq!(ok("abs(-7)"), b"7");
        assert_eq!(ok("int(3.9)"), b"3");
        assert_eq!(ok("pow(2, 10)"), b"1024.0");
        // unknown function / domain error surface as errors
        assert!(ev("frobnicate(1)", &[]).is_err());
        assert!(ev("sqrt(-1)", &[]).is_err());
    }

    #[test]
    fn errors() {
        assert_eq!(ev("1 / 0", &[]), Err(ExprError::msg(b"divide by zero")));
        assert!(ev("$missing + 1", &[]).is_err());
    }
}
