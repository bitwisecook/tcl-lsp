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
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use core::cmp::Ordering;

use crate::bignum::{self, ArithError};
use crate::obj::{self, TclObj};
use tcl_syntax::expr::{eval, BinOp, ExprNode, ExprOps, NumericCompare, UnaryOp};

/// An expr-evaluation error: Tcl's verbatim message bytes plus an optional
/// `-errorcode` (a pre-formatted list, e.g. `ARITH DIVZERO {divide by zero}`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprError {
    pub msg: Vec<u8>,
    pub code: Option<Vec<u8>>,
}

impl ExprError {
    fn msg(s: &[u8]) -> ExprError {
        ExprError {
            msg: s.to_vec(),
            code: None,
        }
    }
    /// An error from owned message bytes (no `-errorcode`).
    pub fn from_bytes(m: Vec<u8>) -> ExprError {
        ExprError { msg: m, code: None }
    }
    /// An error from message bytes plus an optional `-errorcode` (an empty code
    /// is treated as none).
    pub fn from_parts(m: Vec<u8>, code: Vec<u8>) -> ExprError {
        ExprError {
            msg: m,
            code: (!code.is_empty()).then_some(code),
        }
    }
    /// An error with an explicit `-errorcode`.
    fn with_code(m: &[u8], code: &[u8]) -> ExprError {
        ExprError {
            msg: m.to_vec(),
            code: Some(code.to_vec()),
        }
    }
}

pub(crate) fn arith_err(e: ArithError) -> ExprError {
    match e {
        ArithError::NonNumeric => {
            ExprError::msg(b"can't use non-numeric string as operand of arithmetic")
        }
        ArithError::NonInteger => {
            ExprError::msg(b"can't use floating-point value as operand of bitwise op")
        }
        // C stamps the arithmetic `-errorcode`s (`tclExecute.c`).
        ArithError::DivideByZero => {
            ExprError::with_code(b"divide by zero", b"ARITH DIVZERO {divide by zero}")
        }
        // `0 ** negative` is a *domain* error, not a division by zero
        // (`tcl9.0.4/generic/tclExecute.c:8021`, `:7541`).
        ArithError::ZeroToNegativePower => ExprError::with_code(
            b"exponentiation of zero by negative power",
            b"ARITH DOMAIN {exponentiation of zero by negative power}",
        ),
        ArithError::NegativeShift => ExprError::msg(b"negative shift argument"),
        ArithError::ExponentTooLarge => ExprError::msg(b"exponent too large"),
        ArithError::TooLargeToRepresent => ExprError::msg(b"integer value too large to represent"),
        ArithError::Alloc => ExprError::msg(b"out of memory"),
    }
}

/// `cannot use DESC "VALUE" as SIDEoperand of "OP"` — C's
/// `IllegalExprOperandType` (`tclExecute.c`). `side` is `""` for a unary op,
/// `"left "`/`"right "` for a binary one; `desc` comes from
/// [`operand_desc`] / [`float_operand_desc`].
fn operand_type_err_desc(desc: &[u8], value: &[u8], side: &[u8], op: &[u8]) -> ExprError {
    let mut m = b"cannot use ".to_vec();
    m.extend_from_slice(desc);
    m.extend_from_slice(b" \"");
    m.extend_from_slice(value);
    m.extend_from_slice(b"\" as ");
    m.extend_from_slice(side);
    m.extend_from_slice(b"operand of \"");
    m.extend_from_slice(op);
    m.push(b'"');
    ExprError::from_bytes(m)
}

/// The two descriptors an *integer-only* operator picks between: a real double
/// is a `floating-point value`, anything else a `non-numeric string`.
fn float_operand_desc(float: bool) -> &'static [u8] {
    if float {
        b"floating-point value"
    } else {
        b"non-numeric string"
    }
}

/// C's `IllegalExprOperandType` descriptor for a value that cannot be used at
/// all: `NaN` is a **non-numeric floating-point value**, everything else a
/// `non-numeric string` (`tclExecute.c`). The bytecode VM classifies the same
/// way (`tcl_vm::expr::unary_operand_err`).
fn operand_desc(o: *mut TclObj) -> &'static [u8] {
    if matches!(
        bignum::compare(o, o),
        Some(tcl_syntax::expr::NumericCompare::Unordered)
    ) {
        b"non-numeric floating-point value"
    } else {
        b"non-numeric string"
    }
}

/// `cannot use {floating-point value|non-numeric string} "VALUE" as
/// SIDEoperand of "OP"` — the integer-only-operator spelling.
fn operand_type_err(float: bool, value: &[u8], side: &[u8], op: &[u8]) -> ExprError {
    operand_type_err_desc(float_operand_desc(float), value, side, op)
}

/// Map a binary operator to its source symbol (for operand-type errors).
fn binop_sym(op: BinOp) -> &'static [u8] {
    match op {
        BinOp::Add => b"+",
        BinOp::Sub => b"-",
        BinOp::Mul => b"*",
        BinOp::Div => b"/",
        BinOp::Mod => b"%",
        BinOp::Pow => b"**",
        BinOp::BitAnd => b"&",
        BinOp::BitOr => b"|",
        BinOp::BitXor => b"^",
        BinOp::LShift => b"<<",
        BinOp::RShift => b">>",
        _ => b"?",
    }
}

/// Build the operand-type error for a *binary* op: the offending operand is the
/// first one (left, then right) that is non-numeric (for `NonNumeric`) or a
/// float (for `NonInteger`). Other `ArithError`s keep their plain message.
fn binop_err(e: ArithError, op: BinOp, lp: *mut TclObj, rp: *mut TclObj) -> ExprError {
    let float = match e {
        ArithError::NonInteger => true,
        ArithError::NonNumeric => false,
        other => return arith_err(other),
    };
    // `NonInteger`: the float operand; `NonNumeric`: the non-numeric one.
    let left_bad = if float {
        bignum::is_numeric(lp) && !bignum::is_integer(lp)
    } else {
        !bignum::is_numeric(lp)
    };
    let (val, side): (Vec<u8>, &[u8]) = if left_bad {
        (obj::bytes_of(lp), b"left ")
    } else {
        (obj::bytes_of(rp), b"right ")
    };
    operand_type_err(float, &val, side, binop_sym(op))
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
    pub(crate) fn fresh(o: *mut TclObj) -> Owned {
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
    /// Substitute the raw contents of a `"…"` operand — `$var`, `${var}`,
    /// `[cmd]`, and backslashes — exactly as a double-quoted word (C's
    /// expr parser quotes the operand and substitutes it). The default treats
    /// the contents literally (the standalone evaluator has no interp).
    fn subst_string(&mut self, inner: &str) -> Result<Owned, ExprError> {
        Ok(Owned::fresh(obj::new_string_bytes(inner.as_bytes())))
    }
    /// Evaluate a `func(args…)` math-function call. The interp routes this
    /// through the command table (`::tcl::mathfunc::func`, so user overrides
    /// win — the A3 contract); the standalone evaluator falls back to the shared
    /// [`dispatch_shared`] built-in dispatch.
    fn call_function(&mut self, name: &str, args: &[Owned]) -> Result<Owned, ExprError>;
}

/// The shared built-in math-function dispatch over the tower
/// ([`tcl_syntax::expr::mathfunc`]) — the fallback when a function isn't an
/// overridable command. `args` are the already-evaluated operands.
pub fn dispatch_shared(
    name: &str,
    args: &[Owned],
    release: tcl_dialect::TclVersion,
) -> Result<Owned, ExprError> {
    use tcl_syntax::expr::mathfunc::{dispatch_with_backend, IntFuncWidth, NumValue};
    let nums: Option<Vec<NumValue<crate::bignum::TowerMp>>> = args
        .iter()
        .map(|o| crate::bignum::as_math_num(o.ptr()))
        .collect();
    let nums =
        nums.ok_or_else(|| ExprError::msg(b"argument to math function didn't have numeric value"))?;
    let int_func = IntFuncWidth::for_tcl_version(release);
    match dispatch_with_backend(&name.to_ascii_lowercase(), &nums, int_func) {
        Some(num) => Ok(Owned::fresh(crate::bignum::math_num_to_obj(num))),
        None => {
            let mut m = b"unknown math function \"".to_vec();
            m.extend_from_slice(name.as_bytes());
            m.push(b'"');
            Err(ExprError::from_bytes(m))
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
        self.ctx.subst_string(inner)
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
        Ok(Owned::fresh(res.map_err(|e| binop_err(e, op, lp, rp))?))
    }

    fn unary(&mut self, op: UnaryOp, value: Owned) -> Result<Owned, ExprError> {
        // A unary operand-type error names the value and the operator, with no
        // left/right qualifier (`as operand of "OP"`).
        let uerr = |e: ArithError, sym: &[u8]| -> ExprError {
            match e {
                ArithError::NonInteger => {
                    operand_type_err(true, &obj::bytes_of(value.ptr()), b"", sym)
                }
                ArithError::NonNumeric => {
                    operand_type_err(false, &obj::bytes_of(value.ptr()), b"", sym)
                }
                other => arith_err(other),
            }
        };
        match op {
            UnaryOp::Pos => {
                // Non-numeric AND NaN operands both raise for unary `+`
                // (tclsh: `expr {+NaN}` → "can't use non-numeric
                // floating-point value as operand").
                if !matches!(
                    bignum::compare(value.ptr(), value.ptr()),
                    Some(NumericCompare::Ordered(_))
                ) {
                    return Err(operand_type_err(
                        false,
                        &obj::bytes_of(value.ptr()),
                        b"",
                        b"+",
                    ));
                }
                Ok(value)
            }
            UnaryOp::Neg => Ok(Owned::fresh(
                bignum::neg(value.ptr()).map_err(|e| uerr(e, b"-"))?,
            )),
            UnaryOp::BitNot => Ok(Owned::fresh(
                bignum::bnot(value.ptr()).map_err(|e| uerr(e, b"~"))?,
            )),
            UnaryOp::Not => match to_bool(value.ptr()) {
                Ok(b) => Ok(bool_obj(!b)),
                // A `!` operand that is neither boolean nor numeric is an
                // operand-type error (not the generic "expected boolean").
                // A NaN operand gets its own descriptor — tclsh 9.0.4:
                // `cannot use non-numeric floating-point value "NaN" as
                // operand of "!"`.
                Err(_) => Err(operand_type_err_desc(
                    operand_desc(value.ptr()),
                    &obj::bytes_of(value.ptr()),
                    b"",
                    b"!",
                )),
            },
            UnaryOp::WordNot => Err(ExprError::msg(b"unsupported operator")),
        }
    }

    fn compare_numeric(&mut self, left: &Owned, right: &Owned) -> Option<NumericCompare> {
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
        ExprError::from_bytes(what.as_bytes().to_vec())
    }
}

/// Evaluate `node` over the tower, resolving `$var`/`[cmd]` via `ctx`.
pub fn eval_expr(node: &ExprNode, ctx: &mut dyn ExprCtx) -> Result<Owned, ExprError> {
    let mut ops = TowerOps { ctx };
    eval(node, &mut ops)
}

/// Drive `::tcl::mathop::<op>` over the tower: the shared `tcl_cmd_core::mathop`
/// fold/chain logic, each primitive going through this runtime's `ExprOps` (so
/// the same bignum behaviour as `expr`). `args` are already-evaluated operands;
/// `ctx`'s `$var`/`[cmd]` resolution is never invoked (a trivial ctx suffices).
pub fn eval_mathop(
    op: &str,
    args: Vec<Owned>,
    ctx: &mut dyn ExprCtx,
) -> Result<Owned, tcl_cmd_core::mathop::MathopError<ExprError>> {
    let mut ops = TowerOps { ctx };
    tcl_cmd_core::mathop::eval(&mut ops, op, args)
}

// ---- value helpers ---------------------------------------------------------

/// Tcl boolean coercion (`Tcl_GetBooleanFromObj`): a boolean word or any
/// non-zero number.
///
/// The word half is [`tcl_syntax::boolean::parse_boolean_word`], the
/// contract's owner: C's `ParseBoolean` (`tcl9.0.4/generic/tclObj.c:2133`)
/// accepts a boolean word by **unique case-insensitive prefix**, so `tru`,
/// `ye`, and `of` are boolean values. This function used to carry a private
/// six-spelling table with no prefix rule, which made `expr {$x ? 1 : 0}` with
/// `x` = `tru` an error here while tclsh and the bytecode VM returned 1
/// (issue #1425). It also trimmed the word, which C does not: `" true"` is
/// `expected boolean value but got " true"` on both reference releases.
pub(crate) fn to_bool(o: *mut TclObj) -> Result<bool, ExprError> {
    let bytes = obj::bytes_of(o);
    let s = core::str::from_utf8(&bytes).unwrap_or("");
    if let Some(b) = tcl_syntax::boolean::parse_boolean_word(s) {
        return Ok(b);
    }
    // Any number: non-zero ⇒ true. NaN is numeric but not a boolean —
    // tclsh: `expr {NaN ? 1 : 0}` → "floating point value is Not a Number".
    let zero = Owned::fresh(obj::new_wide_int_obj(0));
    match bignum::compare(o, zero.ptr()) {
        Some(NumericCompare::Ordered(ord)) => Ok(!ord.is_eq()),
        Some(NumericCompare::Unordered) => {
            Err(ExprError::msg(b"floating point value is Not a Number"))
        }
        None => {
            // The `got <here>` tail matches C: a multi-token well-formed list
            // reads as `a list`, anything else is quoted (truncated to 50 bytes),
            // so `expr {"a b" ? 1 : 0}` reports `… but got a list`, not `"a b"`.
            let mut m = b"expected boolean value but got ".to_vec();
            m.extend_from_slice(tcl_syntax::list::describe_bad_value(s).as_bytes());
            Err(ExprError::from_bytes(m))
        }
    }
}

fn bool_obj(b: bool) -> Owned {
    Owned::fresh(obj::new_wide_int_obj(i64::from(b)))
}

/// Build an object from a literal token: a number (via the shared grammar),
/// else a plain string.
///
/// A bareword boolean is **not** folded to `0`/`1` here. C keeps the word's
/// own string: `expr {true}` is `true` and `expr {true == 1}` is `0` on tclsh
/// 8.6.16 and 9.0.4, because `Tcl_GetBooleanFromObj` only ever runs when a
/// boolean *context* asks for it — which is [`to_bool`]'s job, over the shared
/// [`tcl_syntax::boolean`] acceptor. This function used to carry a third copy
/// of the six-spelling word table (issue #1425) and returned `1` for
/// `expr {true}`.
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
            dispatch_shared(name, args, tcl_dialect::TclVersion::V9_0)
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
        assert_eq!(
            ev("1 / 0", &[]),
            Err(ExprError::with_code(
                b"divide by zero",
                b"ARITH DIVZERO {divide by zero}"
            ))
        );
        assert!(ev("$missing + 1", &[]).is_err());
    }
}
