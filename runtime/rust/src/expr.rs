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

use std::rc::Rc;

use crate::bignum::{self, ArithError};
use crate::obj::{self, TclObj, TclObjType};
use tcl_syntax::expr::errors::{OperandDesc, OperandSide};
use tcl_syntax::expr::mathfunc::MathFuncError;
use tcl_syntax::expr::{eval, BinOp, ExprNode, ExprOps, NumericCompare, UnaryOp};

// ---------------------------------------------------------------------------
// `TCL_EXPR_TYPE` — the parsed-expression internal rep.
// ---------------------------------------------------------------------------

/// The expression cache's backing: the parsed AST, plus the emulated release
/// its registry validation was performed against.
///
/// The AST is behind an [`Rc`] so a reader takes an owning handle before
/// evaluating. Evaluation runs `[cmd]` substitutions, and one of those can
/// shimmer the very object the AST is cached on (a numeric read of an unshared
/// condition object, say); the strong reference makes that harmless instead of
/// a use-after-free.
struct CachedExpr {
    node: Rc<ExprNode>,
    version: tcl_dialect::TclVersion,
}

/// The `expr` type descriptor — a condition or operand's parsed, validated AST.
///
/// There is deliberately **no** `update_string_proc`: this rep is only ever
/// attached to an object that already carries its spelling, and nothing mutates
/// the AST, so the string rep stays the authority and is never regenerated from
/// the tree. [`obj::change_type`] keeps that spelling across the shimmer, and
/// any later string mutation frees this rep exactly like any other — which is
/// precisely the invalidation the cache needs.
pub static TCL_EXPR_TYPE: TclObjType = TclObjType {
    name: c"expr".as_ptr(),
    free_int_rep_proc: Some(expr_free),
    dup_int_rep_proc: Some(expr_dup),
    update_string_proc: None,
    set_from_any_proc: None,
};

extern "C" fn expr_free(obj: *mut TclObj) {
    let p = obj::internal_rep(obj) as usize as *mut CachedExpr;
    if p.is_null() {
        return;
    }
    // SAFETY: `obj` has the expr type, so its rep is the box `cache_expr` made.
    unsafe { drop(Box::from_raw(p)) };
}

extern "C" fn expr_dup(src: *mut TclObj, dup: *mut TclObj) {
    // SAFETY: `src` has the expr type; the copy shares the immutable AST.
    unsafe {
        let src_ref = &*(obj::internal_rep(src) as usize as *const CachedExpr);
        let boxed = Box::new(CachedExpr {
            node: Rc::clone(&src_ref.node),
            version: src_ref.version,
        });
        obj::change_type(dup, &TCL_EXPR_TYPE, Box::into_raw(boxed) as usize as u64);
    }
}

/// The AST cached on `obj`, if it was validated for `version`.
///
/// A release change (an embedder pinning another emulated Tcl) invalidates the
/// entry: the *parse* runs over the union grammar and is release-neutral, but
/// the registry validation that admitted it is not, so a cached tree is only
/// reusable under the release it was admitted for.
pub(crate) fn cached_expr(
    obj: *mut TclObj,
    version: tcl_dialect::TclVersion,
) -> Option<Rc<ExprNode>> {
    if !core::ptr::eq(obj::obj_type_ptr(obj), &TCL_EXPR_TYPE) {
        return None;
    }
    // SAFETY: the type check above proves the rep is a live `CachedExpr` box.
    let cached = unsafe { &*(obj::internal_rep(obj) as usize as *const CachedExpr) };
    (cached.version == version).then(|| Rc::clone(&cached.node))
}

/// Cache a parsed and validated AST on `obj` so the next evaluation of the same
/// condition object reuses it instead of re-lexing its text.
///
/// Only a **plain string** that already carries its spelling is shimmered. That
/// is the shape every literal condition word has, and refusing the rest keeps
/// the cache from destroying a list/dict/numeric rep another holder still wants
/// — the same "may we cache" reasoning as the numeric write-back, one rung up.
pub(crate) fn cache_expr(obj: *mut TclObj, version: tcl_dialect::TclVersion, node: &Rc<ExprNode>) {
    if !obj::obj_type_ptr(obj).is_null() || !obj::has_string_rep(obj) {
        return;
    }
    let boxed = Box::new(CachedExpr {
        node: Rc::clone(node),
        version,
    });
    obj::change_type(obj, &TCL_EXPR_TYPE, Box::into_raw(boxed) as usize as u64);
}

#[cfg(test)]
thread_local! {
    /// Test hook: how many expression *parses* have run since the last reset.
    /// The cache's whole job is to keep this from growing per evaluation.
    static EXPR_PARSE_COUNT: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
}

/// Test hook: record that the parser ran.
#[cfg(test)]
pub(crate) fn note_expr_parse() {
    EXPR_PARSE_COUNT.with(|c| c.set(c.get() + 1));
}

/// Test hook: reset the parse counter and read it back.
#[cfg(test)]
pub(crate) fn reset_expr_parse_count() {
    EXPR_PARSE_COUNT.with(|c| c.set(0));
}

/// Test hook: expression parses since [`reset_expr_parse_count`].
#[cfg(test)]
pub(crate) fn expr_parse_count() -> u64 {
    EXPR_PARSE_COUNT.with(core::cell::Cell::get)
}

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
        // `0 ** negative` is a *domain* error in C, not a division by zero
        // (tclsh 8.6/9.0: `-errorcode ARITH DOMAIN`).
        ArithError::ZeroToNegativePower => ExprError::with_code(
            b"exponentiation of zero by negative power",
            b"ARITH DOMAIN {exponentiation of zero by negative power}",
        ),
        // C's `TclExprFloatError` for a produced NaN (`tclExecute.c`).
        ArithError::NanResult => ExprError::with_code(
            b"domain error: argument not in valid range",
            b"ARITH DOMAIN {domain error: argument not in valid range}",
        ),
        ArithError::NegativeShift => ExprError::msg(b"negative shift argument"),
        ArithError::ExponentTooLarge => ExprError::msg(b"exponent too large"),
        ArithError::TooLargeToRepresent => ExprError::msg(b"integer value too large to represent"),
        ArithError::Alloc => ExprError::msg(b"out of memory"),
    }
}

/// C's `IllegalExprOperandType` (`tclExecute.c`), through the shared owner
/// [`tcl_syntax::expr::errors`]: the *wording* is a release axis (9.0 names
/// the value and the side, 8.4-8.6 name neither and have no list branch),
/// while the `-errorcode ARITH DOMAIN <description>` is invariant. Both were
/// hard-coded to 9.0's form with no `-errorcode` at all before #1581.
fn operand_type_err(desc: OperandDesc, value: &[u8], side: OperandSide, op: &[u8]) -> ExprError {
    let release = tcl_syntax::expr::errors::ambient_release();
    let message = tcl_syntax::expr::errors::illegal_operand_message(
        desc,
        &String::from_utf8_lossy(value),
        side,
        &String::from_utf8_lossy(op),
        release,
    );
    let code = tcl_syntax::expr::errors::illegal_operand_error_code(desc, release);
    ExprError::from_parts(message.into_bytes(), code.into_bytes())
}

/// How C describes `o` when an operator cannot use it: a NaN is a
/// "non-numeric floating-point value", a well-formed multi-element list is
/// 9.0's list branch, a double handed to an integer-only operator is a
/// "floating-point value", and anything else is a "non-numeric string".
fn operand_desc(o: *mut TclObj, float_operand: bool) -> OperandDesc {
    if float_operand {
        return OperandDesc::FloatingPointValue;
    }
    // `compare(o, o)` is `Unordered` exactly for a NaN (numeric but unusable).
    if matches!(bignum::compare(o, o), Some(NumericCompare::Unordered)) {
        return OperandDesc::NonNumericFloatingPointValue;
    }
    let bytes = obj::bytes_of(o);
    let text = String::from_utf8_lossy(&bytes);
    if tcl_syntax::list::max_list_length(&text) > 1 && tcl_syntax::list::split_list(&text).is_ok() {
        return OperandDesc::List;
    }
    OperandDesc::NonNumericString
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
    let (bad, side) = if left_bad {
        (lp, OperandSide::Left)
    } else {
        (rp, OperandSide::Right)
    };
    operand_type_err(
        operand_desc(bad, float),
        &obj::bytes_of(bad),
        side,
        binop_sym(op),
    )
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
pub fn dispatch_shared(name: &str, args: &[Owned]) -> Result<Owned, ExprError> {
    use tcl_syntax::expr::mathfunc::{try_dispatch_with_backend_int_width, IntWidth, NumValue};
    let nums: Option<Vec<NumValue<crate::bignum::TowerMp>>> = args
        .iter()
        .map(|o| crate::bignum::as_math_num(o.ptr()))
        .collect();
    let nums =
        nums.ok_or_else(|| ExprError::msg(b"argument to math function didn't have numeric value"))?;
    // The standalone evaluator has no interp to ask for a release, so it uses
    // the runtime's own target release (Tcl 9.0) for `int()`'s width; the
    // interp path resolves it from `Interp::runtime_version` in
    // `cmd_mathfunc`.
    match try_dispatch_with_backend_int_width(
        &name.to_ascii_lowercase(),
        &nums,
        IntWidth::Unbounded,
    ) {
        Ok(num) => Ok(Owned::fresh(crate::bignum::math_num_to_obj(num))),
        Err(MathFuncError::UnknownFunction) => {
            let mut m = b"unknown math function \"".to_vec();
            m.extend_from_slice(name.as_bytes());
            m.push(b'"');
            Err(ExprError::from_bytes(m))
        }
        Err(e) => Err(math_func_err(e)),
    }
}

/// A shared math-function refusal as this engine's error: C's verbatim
/// message and `-errorcode` (#1581). `Abstain` cannot occur here — the
/// runtime's backend has an arbitrary-precision rung and its release is
/// resolved — so it falls back to the generic domain error.
pub(crate) fn math_func_err(e: MathFuncError) -> ExprError {
    let message = e.message();
    if message.is_empty() {
        return ExprError::with_code(
            tcl_syntax::expr::errors::DOMAIN_MESSAGE.as_bytes(),
            tcl_syntax::expr::errors::DOMAIN_CODE.as_bytes(),
        );
    }
    ExprError::with_code(message.as_bytes(), e.error_code().as_bytes())
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
                ArithError::NonInteger => operand_type_err(
                    operand_desc(value.ptr(), true),
                    &obj::bytes_of(value.ptr()),
                    OperandSide::Unary,
                    sym,
                ),
                ArithError::NonNumeric => operand_type_err(
                    operand_desc(value.ptr(), false),
                    &obj::bytes_of(value.ptr()),
                    OperandSide::Unary,
                    sym,
                ),
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
                        operand_desc(value.ptr(), false),
                        &obj::bytes_of(value.ptr()),
                        OperandSide::Unary,
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
                Err(_) => Err(operand_type_err(
                    operand_desc(value.ptr(), false),
                    &obj::bytes_of(value.ptr()),
                    OperandSide::Unary,
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

/// Tcl boolean context (`Tcl_GetBooleanFromObj`) as an `expr` error: the
/// runtime's one typed-read owner ([`crate::typed_value::boolean`]) — the
/// shared boolean words by unique prefix, else any number against zero, a NaN
/// refused — with C's message and `-errorcode` carried into [`ExprError`].
pub(crate) fn to_bool(o: *mut TclObj) -> Result<bool, ExprError> {
    crate::typed_value::boolean(o).map_err(|e| ExprError::from_parts(e.message, e.code.to_vec()))
}

fn bool_obj(b: bool) -> Owned {
    Owned::fresh(obj::new_wide_int_obj(i64::from(b)))
}

/// Build an object from a literal token: a number through the shared grammar,
/// otherwise its original string spelling. Tcl preserves boolean literal text
/// (`expr {yes}` returns `yes`); coercion happens only in a boolean context.
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
    fn boolean_prefixes_share_the_canonical_converter() {
        for source in [
            "true", "tru", "t", "yes", "ye", "y", "false", "f", "no", "n", "off", "of",
        ] {
            assert_eq!(ok(source), source.as_bytes(), "{source}");
        }
        assert_eq!(ok("tru ? yes : no"), b"yes");
        assert_eq!(ok("!of"), b"1");
        assert!(ev("o", &[]).is_err(), "on/off share the prefix o");
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
