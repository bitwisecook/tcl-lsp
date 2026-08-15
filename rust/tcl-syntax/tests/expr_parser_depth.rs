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

//! Depth coverage for the `tcl-syntax` `expr` stack — the EVALUATION half.
//!
//! Companion to `expr_parser.rs` and `expr_parser_coverage.rs`, which
//! drive the *parser* / *AST shape* (`parse_expr` → `ExprNode`). This file
//! instead exercises the two evaluation modules those parse-only tests do
//! not touch:
//!
//!   * [`tcl_syntax::expr::mathfunc`] — the shared `::tcl::mathfunc::*` math
//!     function dispatch (`sin`/`cos`/`atan2`/`int`/`round`/`isqrt`/… over the
//!     `Int`/`Float` numeric rung), and
//!   * [`tcl_syntax::expr::eval`] — the generic tree-walk over `ExprNode` driven
//!     by the [`ExprOps`] value seam (operator dispatch, short-circuit `&&`/`||`,
//!     `?:`, the numeric-vs-string comparison rule, `in`/`ni` membership, the
//!     `binary_other` dialect hook).
//!
//! To prove end-to-end EVALUATION RESULTS we build one real `ExprOps`
//! implementation here, [`Tower`], whose value type is `mathfunc::Num`
//! (`Int(i64)` / `Float(f64)`) and whose `call` seam routes through
//! `mathfunc::dispatch`. Parsing `expr {…}` and walking it with [`Tower`] gives
//! the value Tcl would compute, so every assertion is pinned to real `tclsh`
//! (8.6 + 9.0, via `scripts/dev/tclsh_check.sh`) ground truth, cited inline as
//! `// tclsh:`.
//!
//! C-Tcl proof — representative ground truth (8.6 == 9.0 unless noted):
//!   expr {7/2}            = 3        (int truncation toward zero in this rung)
//!   expr {-7/2}           = -4       (Tcl floor division — see note at the site)
//!   expr {7 % -3}         = -2       (Tcl floor modulo)
//!   expr {2**3**2}        = 512      (right-assoc Pow)
//!   expr {1+2*3}          = 7        (× binds tighter than +)
//!   expr {(2+3)*4}        = 20
//!   expr {abs(-5)+1}      = 6
//!   expr {min(5,abs(-2))} = 2
//!   expr {hypot(3,4)}     = 5.0
//!   expr {fmod(-7,3)}     = -1.0     (sign follows the dividend)
//!   expr {int(-0.9)}      = 0        (truncation toward zero)
//!   expr {round(-2.5)}    = -3       (ties away from zero)
//!   expr {"2" < "10"}     = 1        (both look numeric ⇒ numeric compare)
//!   expr {2 in {1 2 3}}   = 1
//! The IEEE predicate functions `isinf`/`isnan`/`isfinite`/`isnormal`/
//! `issubnormal`/`isunordered` are 9.0-ONLY (8.6 errors with `invalid command
//! name "tcl::mathfunc::is*"`); the Rust `mathfunc` models the latest dialect,
//! so those values are pinned to 9.0 and the 8.6 divergence is noted at the
//! site.

use core::cmp::Ordering;

use tcl_syntax::expr::mathfunc::{Num, dispatch};
use tcl_syntax::expr::{BinOp, ExprNode, ExprOps, NumericCompare, UnaryOp, eval, parse_expr};

// ===========================================================================
// Part A — `mathfunc::dispatch` directly.
//
// `dispatch(name, &[Num]) -> Option<Num>` is the public math-function seam.
// The parse-only ports never call it, and the in-source unit tests touch only
// a subset of the function table; here we hit every branch family and pin each
// result to tclsh.
// ===========================================================================

/// Float equality with a tolerance, for transcendental results whose last bit
/// is platform-libm-dependent.
fn approx(got: Num, want: f64) {
    match got {
        Num::Float(f) => assert!((f - want).abs() < 1e-9, "expected ~{want}, got {f}"),
        Num::Int(i) => panic!("expected Float(~{want}), got Int({i})"),
    }
}

/// Exact float equality, for results representable exactly (`2.0`, `5.0`,
/// `1024.0`, …). Compares bit patterns so the assertion is exact without
/// tripping clippy's `float_cmp`.
fn float_eq(got: Num, want: f64) {
    match got {
        Num::Float(f) => assert_eq!(f.to_bits(), want.to_bits(), "expected {want}, got {f}"),
        Num::Int(i) => panic!("expected Float({want}), got Int({i})"),
    }
}

#[test]
fn dispatch_unary_float_whole_table() {
    // tclsh: sin(0)=0.0 cos(0)=1.0 tan(0)=0.0 asin(0)=0.0 acos(1)=0.0
    //        atan(0)=0.0 sinh(0)=0.0 cosh(0)=1.0 tanh(0)=0.0
    //        exp(0)=1.0 log(1)=0.0 log10(1000)=3.0 sqrt(16)=4.0
    approx(dispatch("sin", &[Num::Int(0)]).unwrap(), 0.0);
    approx(dispatch("cos", &[Num::Int(0)]).unwrap(), 1.0);
    approx(dispatch("tan", &[Num::Int(0)]).unwrap(), 0.0);
    approx(dispatch("asin", &[Num::Int(0)]).unwrap(), 0.0);
    approx(dispatch("acos", &[Num::Int(1)]).unwrap(), 0.0);
    approx(dispatch("atan", &[Num::Int(0)]).unwrap(), 0.0);
    approx(dispatch("sinh", &[Num::Int(0)]).unwrap(), 0.0);
    approx(dispatch("cosh", &[Num::Int(0)]).unwrap(), 1.0);
    approx(dispatch("tanh", &[Num::Int(0)]).unwrap(), 0.0);
    approx(dispatch("exp", &[Num::Int(0)]).unwrap(), 1.0);
    approx(dispatch("log", &[Num::Int(1)]).unwrap(), 0.0);
    approx(dispatch("log10", &[Num::Int(1000)]).unwrap(), 3.0);
    // tclsh: sqrt(2)=1.4142135623730951
    approx(
        dispatch("sqrt", &[Num::Int(2)]).unwrap(),
        std::f64::consts::SQRT_2,
    );
}

#[test]
fn dispatch_unary_float_domain_and_arity() {
    // sqrt(-1) is a domain error: a NaN result from a non-NaN argument → None
    // (tclsh raises "domain error: argument not in valid range").
    assert_eq!(dispatch("sqrt", &[Num::Int(-1)]), None);
    // A NaN argument short-circuits to None before the libm call.
    assert_eq!(dispatch("sin", &[Num::Float(f64::NAN)]), None);
    // Wrong arity → None (unary functions take exactly one argument).
    assert_eq!(dispatch("cos", &[]), None);
    assert_eq!(dispatch("cos", &[Num::Int(0), Num::Int(0)]), None);
}

#[test]
fn dispatch_binary_float_whole_table() {
    // tclsh: atan2(0,1)=0.0 hypot(3,4)=5.0 fmod(7,3)=1.0 pow(2,10)=1024.0
    approx(dispatch("atan2", &[Num::Int(0), Num::Int(1)]).unwrap(), 0.0);
    float_eq(dispatch("hypot", &[Num::Int(3), Num::Int(4)]).unwrap(), 5.0);
    float_eq(dispatch("fmod", &[Num::Int(7), Num::Int(3)]).unwrap(), 1.0);
    float_eq(
        dispatch("pow", &[Num::Int(2), Num::Int(10)]).unwrap(),
        1024.0,
    );
    // fmod sign follows the DIVIDEND, like C fmod / Tcl.
    // tclsh: fmod(-7,3)=-1.0  fmod(7,-3)=1.0
    float_eq(
        dispatch("fmod", &[Num::Int(-7), Num::Int(3)]).unwrap(),
        -1.0,
    );
    float_eq(dispatch("fmod", &[Num::Int(7), Num::Int(-3)]).unwrap(), 1.0);
    // tclsh: fmod(7.5,2)=1.5
    float_eq(
        dispatch("fmod", &[Num::Float(7.5), Num::Int(2)]).unwrap(),
        1.5,
    );
}

#[test]
fn dispatch_binary_float_nan_and_arity() {
    // Either operand NaN → None (has_nan guard).
    assert_eq!(
        dispatch("atan2", &[Num::Float(f64::NAN), Num::Int(1)]),
        None
    );
    assert_eq!(dispatch("pow", &[Num::Int(1), Num::Float(f64::NAN)]), None);
    // Wrong arity → None.
    assert_eq!(dispatch("hypot", &[Num::Int(3)]), None);
    assert_eq!(
        dispatch("fmod", &[Num::Int(1), Num::Int(2), Num::Int(3)]),
        None
    );
}

#[test]
fn dispatch_min_max_int_and_float_rungs() {
    // All-int operands stay in the Int rung. tclsh: min(3,1,2)=1 max(3,1,2)=3
    assert_eq!(
        dispatch("min", &[Num::Int(3), Num::Int(1), Num::Int(2)]),
        Some(Num::Int(1))
    );
    assert_eq!(
        dispatch("max", &[Num::Int(3), Num::Int(1), Num::Int(2)]),
        Some(Num::Int(3))
    );
    // A mixed int/float set promotes to the Float rung.
    // tclsh: min(3,1.5)=1.5  max(3,1.5,9.2)=9.2
    float_eq(
        dispatch("min", &[Num::Int(3), Num::Float(1.5)]).unwrap(),
        1.5,
    );
    float_eq(
        dispatch("max", &[Num::Int(3), Num::Float(1.5), Num::Float(9.2)]).unwrap(),
        9.2,
    );
    // Single argument echoes itself. tclsh: min(42)=42 max(42)=42
    assert_eq!(dispatch("min", &[Num::Int(42)]), Some(Num::Int(42)));
    assert_eq!(dispatch("max", &[Num::Int(42)]), Some(Num::Int(42)));
    // Empty arg list → None; a NaN anywhere → None.
    assert_eq!(dispatch("max", &[]), None);
    assert_eq!(dispatch("min", &[Num::Int(1), Num::Float(f64::NAN)]), None);
}

#[test]
fn dispatch_type_conversions_int_and_float() {
    // abs: int and float, including the float branch. tclsh: abs(-7)=7 abs(-7.5)=7.5
    assert_eq!(dispatch("abs", &[Num::Int(-7)]), Some(Num::Int(7)));
    float_eq(dispatch("abs", &[Num::Float(-7.5)]).unwrap(), 7.5);
    // int/wide/entier truncate toward zero. tclsh: int(3.9)=3 int(-3.9)=-3 int(-0.9)=0
    assert_eq!(dispatch("int", &[Num::Float(3.9)]), Some(Num::Int(3)));
    assert_eq!(dispatch("wide", &[Num::Float(-3.9)]), Some(Num::Int(-3)));
    assert_eq!(dispatch("entier", &[Num::Float(-0.9)]), Some(Num::Int(0)));
    // An already-integer argument to int/round passes through the Int arm.
    assert_eq!(dispatch("int", &[Num::Int(5)]), Some(Num::Int(5)));
    assert_eq!(dispatch("round", &[Num::Int(5)]), Some(Num::Int(5)));
    // double widens. tclsh: double(5)=5.0
    float_eq(dispatch("double", &[Num::Int(5)]).unwrap(), 5.0);
    // bool is the truthiness predicate. tclsh: bool(5)=1 bool(0)=0
    assert_eq!(dispatch("bool", &[Num::Int(5)]), Some(Num::Int(1)));
    assert_eq!(dispatch("bool", &[Num::Float(0.0)]), Some(Num::Int(0)));
    // round ties away from zero. tclsh: round(2.5)=3 round(-2.5)=-3 round(2.4)=2
    assert_eq!(dispatch("round", &[Num::Float(2.5)]), Some(Num::Int(3)));
    assert_eq!(dispatch("round", &[Num::Float(-2.5)]), Some(Num::Int(-3)));
    assert_eq!(dispatch("round", &[Num::Float(2.4)]), Some(Num::Int(2)));
    // ceil / floor stay in the Float rung. tclsh: ceil(2.1)=3.0 floor(2.9)=2.0
    //   ceil(-2.1)=-2.0 floor(-2.1)=-3.0
    float_eq(dispatch("ceil", &[Num::Float(2.1)]).unwrap(), 3.0);
    float_eq(dispatch("floor", &[Num::Float(2.9)]).unwrap(), 2.0);
    float_eq(dispatch("ceil", &[Num::Float(-2.1)]).unwrap(), -2.0);
    float_eq(dispatch("floor", &[Num::Float(-2.1)]).unwrap(), -3.0);
    // isqrt is the integer square root. tclsh: isqrt(17)=4 isqrt(16)=4 isqrt(0)=0
    assert_eq!(dispatch("isqrt", &[Num::Int(17)]), Some(Num::Int(4)));
    assert_eq!(dispatch("isqrt", &[Num::Int(16)]), Some(Num::Int(4)));
    assert_eq!(dispatch("isqrt", &[Num::Int(0)]), Some(Num::Int(0)));
    // isqrt of a negative is undefined → None.
    assert_eq!(dispatch("isqrt", &[Num::Int(-1)]), None);
}

#[test]
fn dispatch_type_conversion_overflow_and_nan_guards() {
    // A float beyond the i64 window cannot be truncated/rounded → None.
    assert_eq!(dispatch("int", &[Num::Float(1.0e20)]), None);
    assert_eq!(dispatch("wide", &[Num::Float(1.0e20)]), None);
    assert_eq!(dispatch("entier", &[Num::Float(1.0e20)]), None);
    assert_eq!(dispatch("round", &[Num::Float(1.0e20)]), None);
    // NaN flows to None for the value-bearing conversions (the shared NaN
    // guard inside type_conv).
    assert_eq!(dispatch("abs", &[Num::Float(f64::NAN)]), None);
    assert_eq!(dispatch("int", &[Num::Float(f64::NAN)]), None);
    assert_eq!(dispatch("double", &[Num::Float(f64::NAN)]), None);
    assert_eq!(dispatch("bool", &[Num::Float(f64::NAN)]), None);
    assert_eq!(dispatch("ceil", &[Num::Float(f64::NAN)]), None);
    // Wrong arity for a unary conversion → None.
    assert_eq!(dispatch("abs", &[Num::Int(1), Num::Int(2)]), None);
}

#[test]
fn dispatch_ieee_predicates_9_0_only() {
    // These six are 9.0-ONLY functions. In tclsh8.6 they error with
    // `invalid command name "tcl::mathfunc::is*"`; tclsh9.0 evaluates them,
    // and the Rust mathfunc models the 9.0 dialect. Values pinned to 9.0.
    //
    // isinf / isnan / isfinite. tclsh9.0: isinf(Inf)=1 isinf(5)=0
    //   isnan(NaN)=1 isnan(5)=0  isfinite(1.0)=1 isfinite(Inf)=0 isfinite(5)=1
    assert_eq!(
        dispatch("isinf", &[Num::Float(f64::INFINITY)]),
        Some(Num::Int(1))
    );
    assert_eq!(dispatch("isinf", &[Num::Int(5)]), Some(Num::Int(0)));
    assert_eq!(
        dispatch("isnan", &[Num::Float(f64::NAN)]),
        Some(Num::Int(1))
    );
    assert_eq!(dispatch("isnan", &[Num::Int(5)]), Some(Num::Int(0)));
    assert_eq!(dispatch("isfinite", &[Num::Float(1.0)]), Some(Num::Int(1)));
    assert_eq!(
        dispatch("isfinite", &[Num::Float(f64::INFINITY)]),
        Some(Num::Int(0))
    );
    assert_eq!(dispatch("isfinite", &[Num::Int(5)]), Some(Num::Int(1)));
    // isnormal / issubnormal. tclsh9.0: isnormal(1.0)=1 issubnormal(1.0)=0.
    // An integer widens to a finite normal double.
    assert_eq!(dispatch("isnormal", &[Num::Float(1.0)]), Some(Num::Int(1)));
    assert_eq!(dispatch("isnormal", &[Num::Int(7)]), Some(Num::Int(1)));
    assert_eq!(dispatch("isnormal", &[Num::Float(0.0)]), Some(Num::Int(0)));
    assert_eq!(
        dispatch("issubnormal", &[Num::Float(1.0)]),
        Some(Num::Int(0))
    );
    // The smallest positive denormal is subnormal.
    assert_eq!(
        dispatch("issubnormal", &[Num::Float(f64::from_bits(1))]),
        Some(Num::Int(1))
    );
    // isunordered(x,y) is 1 iff either operand is NaN.
    // tclsh9.0: isunordered(NaN,1)=1  isunordered(1,2)=0
    assert_eq!(
        dispatch("isunordered", &[Num::Float(f64::NAN), Num::Int(1)]),
        Some(Num::Int(1))
    );
    assert_eq!(
        dispatch("isunordered", &[Num::Int(1), Num::Int(2)]),
        Some(Num::Int(0))
    );
    // isunordered wrong arity → None.
    assert_eq!(dispatch("isunordered", &[Num::Int(1)]), None);
}

#[test]
fn dispatch_unknown_function_is_none() {
    // An unknown name is never a math function → None (the const-folder
    // "give up" / runtime "fall through to the command table" contract).
    assert_eq!(dispatch("frobnicate", &[Num::Int(1)]), None);
    assert_eq!(dispatch("", &[]), None);
    // `rand`/`srand` are explicitly the caller's responsibility, so the shared
    // dispatch declines them too.
    assert_eq!(dispatch("rand", &[]), None);
    assert_eq!(dispatch("srand", &[Num::Int(1)]), None);
}

#[test]
fn num_as_f64_widens_int_rung() {
    // The public `Num::as_f64` widens the Int rung and passes Float through.
    // Bit-compare so the exact-value assertion does not trip clippy's float_cmp.
    assert_eq!(Num::Int(5).as_f64().to_bits(), 5.0_f64.to_bits());
    assert_eq!(Num::Float(2.5).as_f64().to_bits(), 2.5_f64.to_bits());
}

// ===========================================================================
// Part B — the shared `eval` walker driven by a real `ExprOps`.
//
// `Tower` is a full numeric evaluator over `mathfunc::Num`: literals parse to
// Int/Float, `$var` reads a small fixed environment, `[cmd]` is stubbed, and
// `call` routes to `mathfunc::dispatch`. Walking a parsed `expr {…}` with it
// yields the value tclsh computes, so each result below is C-Tcl-proven.
// ===========================================================================

/// Resolution failure / domain error from a value seam.
#[derive(Debug, PartialEq)]
struct EvalError(String);

/// The evaluator's value: a number on the [`Num`] rung, or a string (lists,
/// non-numeric quoted operands). Mirrors the int/double/string shape of a
/// `Tcl_Obj` closely enough to drive every walker branch — including `in`/`ni`
/// membership over a `{1 2 3}` braced list operand, which is not a number.
#[derive(Debug, Clone, PartialEq)]
enum Val {
    /// A numeric value (`Int` / `Float`).
    N(Num),
    /// A string value.
    S(String),
}

impl Val {
    /// The `f64` view of a numeric value (`None` for a string).
    fn as_f64(&self) -> Option<f64> {
        match self {
            Val::N(n) => Some(n.as_f64()),
            Val::S(_) => None,
        }
    }

    /// Canonical string rep, for string compares / membership.
    fn text(&self) -> String {
        match self {
            Val::N(Num::Int(i)) => i.to_string(),
            Val::N(Num::Float(f)) => f.to_string(),
            Val::N(Num::Big(_)) => unreachable!("default Num has no bignum backend"),
            Val::S(s) => s.clone(),
        }
    }

    /// Parse a literal/operand text into a value: int (incl. hex), then double,
    /// then the IEEE specials and booleans Tcl recognises, else a string.
    fn parse(text: &str) -> Val {
        if let Ok(i) = text.parse::<i64>() {
            return Val::N(Num::Int(i));
        }
        if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X"))
            && let Ok(i) = i64::from_str_radix(hex, 16)
        {
            return Val::N(Num::Int(i));
        }
        if let Ok(f) = text.parse::<f64>() {
            return Val::N(Num::Float(f));
        }
        match text.to_ascii_lowercase().as_str() {
            "inf" | "infinity" => Val::N(Num::Float(f64::INFINITY)),
            "nan" => Val::N(Num::Float(f64::NAN)),
            "true" | "yes" | "on" => Val::N(Num::Int(1)),
            "false" | "no" | "off" => Val::N(Num::Int(0)),
            _ => Val::S(text.to_string()),
        }
    }
}

/// A real `ExprOps` over [`Val`] (the `Int`/`Float`/`String` tower). The math
/// `call` seam routes through the shared [`dispatch`], so `expr {…}` walked
/// with this yields the value tclsh computes.
#[derive(Default)]
struct Tower {
    /// Record of every `[cmd]` script the walk evaluated, so short-circuit
    /// tests can assert the right operand was (not) reached.
    commands: Vec<String>,
}

impl Tower {
    fn drain_commands(&mut self) -> Vec<String> {
        std::mem::take(&mut self.commands)
    }
}

impl ExprOps for Tower {
    type Value = Val;
    type Error = EvalError;

    fn literal(&mut self, text: &str) -> Result<Val, EvalError> {
        Ok(Val::parse(text))
    }

    fn string(&mut self, inner: &str) -> Result<Val, EvalError> {
        // A quoted/braced operand shimmers to a number when it looks numeric,
        // else stays a string (so `{1 2 3}` is a list-bearing string value).
        Ok(Val::parse(inner))
    }

    fn var(&mut self, name: &str) -> Result<Val, EvalError> {
        // A tiny fixed environment mirroring the comments' tclsh `set`s.
        Ok(match name {
            "a" => Val::N(Num::Int(3)),
            "b" => Val::N(Num::Int(4)),
            "x" => Val::N(Num::Int(10)),
            "y" => Val::N(Num::Int(2)),
            "f" => Val::N(Num::Float(1.5)),
            other => return Err(EvalError(format!("no such var: {other}"))),
        })
    }

    fn command(&mut self, script: &str) -> Result<Val, EvalError> {
        self.commands.push(script.to_string());
        // A stand-in result so `[cmd] + 1`-style expressions evaluate.
        Ok(Val::N(Num::Int(100)))
    }

    fn call(&mut self, function: &str, args: Vec<Val>) -> Result<Val, EvalError> {
        // Lower the args onto the numeric rung, then route through the shared
        // math-function dispatch — the very seam the runtime/const-folder use.
        let mut nums = Vec::with_capacity(args.len());
        for a in &args {
            match a {
                Val::N(n) => nums.push(*n),
                Val::S(s) => return Err(EvalError(format!("non-numeric call arg: {s}"))),
            }
        }
        dispatch(function, &nums)
            .map(Val::N)
            .ok_or_else(|| EvalError(format!("bad call: {function}")))
    }

    fn arith(&mut self, op: BinOp, left: Val, right: Val) -> Result<Val, EvalError> {
        // Integer arithmetic stays in the Int rung; any Float operand promotes.
        // Division and modulo follow Tcl's FLOOR semantics for integers (the
        // sign of `%` follows the divisor), distinct from Rust's truncating
        // `/`/`%`.
        if let (Val::N(Num::Int(a)), Val::N(Num::Int(b))) = (&left, &right) {
            let (a, b) = (*a, *b);
            return Ok(Val::N(Num::Int(match op {
                BinOp::Add => a + b,
                BinOp::Sub => a - b,
                BinOp::Mul => a * b,
                BinOp::Div => floor_div(a, b)?,
                BinOp::Mod => floor_mod(a, b)?,
                BinOp::Pow => base_pow(a, b),
                BinOp::BitAnd => a & b,
                BinOp::BitOr => a | b,
                BinOp::BitXor => a ^ b,
                BinOp::LShift => a << b,
                BinOp::RShift => a >> b,
                _ => return Err(EvalError("non-arith op in arith".into())),
            })));
        }
        let (Some(a), Some(b)) = (left.as_f64(), right.as_f64()) else {
            return Err(EvalError("non-numeric arith operand".into()));
        };
        Ok(Val::N(Num::Float(match op {
            BinOp::Add => a + b,
            BinOp::Sub => a - b,
            BinOp::Mul => a * b,
            BinOp::Div => a / b,
            BinOp::Mod => a % b,
            BinOp::Pow => a.powf(b),
            _ => return Err(EvalError("non-float arith op".into())),
        })))
    }

    fn unary(&mut self, op: UnaryOp, value: Val) -> Result<Val, EvalError> {
        Ok(match (op, &value) {
            (UnaryOp::Neg, Val::N(Num::Int(n))) => Val::N(Num::Int(-n)),
            (UnaryOp::Neg, Val::N(Num::Float(f))) => Val::N(Num::Float(-f)),
            (UnaryOp::Pos, _) => value,
            (UnaryOp::BitNot, Val::N(Num::Int(n))) => Val::N(Num::Int(!n)),
            (UnaryOp::Not | UnaryOp::WordNot, _) => {
                let f = value
                    .as_f64()
                    .ok_or_else(|| EvalError("non-numeric unary".into()))?;
                Val::N(Num::Int(i64::from(f == 0.0)))
            }
            _ => return Err(EvalError("non-numeric unary".into())),
        })
    }

    fn binary_other(&mut self, op: BinOp, left: Val, right: Val) -> Result<Val, EvalError> {
        // Override (rather than the trait default) so the dialect operators
        // actually compute — exercises the non-default `binary_other` path.
        let b = match op {
            BinOp::StrEquals => left.text() == right.text(),
            BinOp::Contains => left.text().contains(&right.text()),
            _ => return Err(EvalError(format!("unsupported dialect op: {op:?}"))),
        };
        Ok(Val::N(Num::Int(i64::from(b))))
    }

    fn compare_numeric(&mut self, left: &Val, right: &Val) -> Option<NumericCompare> {
        // The `Some` arm of the numeric-vs-string rule: a numeric outcome when
        // BOTH operands are numeric (NaN → `Unordered`), else `None` (caller
        // falls back to string).
        match (left.as_f64(), right.as_f64()) {
            (Some(l), Some(r)) => Some(NumericCompare::from_partial(l.partial_cmp(&r))),
            _ => None,
        }
    }

    fn compare_string(&mut self, left: &Val, right: &Val) -> Ordering {
        left.text().cmp(&right.text())
    }

    fn in_list(&mut self, needle: &Val, list: &Val) -> Result<bool, EvalError> {
        let n = needle.text();
        Ok(list.text().split_whitespace().any(|e| e == n))
    }

    fn to_bool(&mut self, value: &Val) -> Result<bool, EvalError> {
        match value.as_f64() {
            Some(f) => Ok(f != 0.0),
            None => Err(EvalError("non-boolean value".into())),
        }
    }

    fn bool_value(&mut self, b: bool) -> Val {
        Val::N(Num::Int(i64::from(b)))
    }

    fn unsupported(&mut self, what: &str) -> EvalError {
        EvalError(format!("unsupported: {what}"))
    }
}

/// Tcl floor division for integers: the quotient rounds toward negative
/// infinity. tclsh: `-7/2` == -4, `7/2` == 3, `7/-2` == -4.
fn floor_div(a: i64, b: i64) -> Result<i64, EvalError> {
    if b == 0 {
        return Err(EvalError("divide by zero".into()));
    }
    // Rust's `/` truncates toward zero; adjust by one when the signs differ and
    // the division is inexact, to round toward negative infinity instead.
    let q = a / b;
    let r = a % b;
    Ok(if (r != 0) && ((r < 0) != (b < 0)) {
        q - 1
    } else {
        q
    })
}

/// Tcl floor modulo for integers: the remainder takes the SIGN OF THE DIVISOR.
/// tclsh: `7 % -3` == -2, `-7 % 3` == 2, `9 % 2` == 1.
fn floor_mod(a: i64, b: i64) -> Result<i64, EvalError> {
    if b == 0 {
        return Err(EvalError("divide by zero".into()));
    }
    let r = a % b;
    Ok(if (r != 0) && ((r < 0) != (b < 0)) {
        r + b
    } else {
        r
    })
}

/// Integer exponentiation (non-negative exponent) — our asserted exponents are
/// small and never overflow i64.
fn base_pow(base: i64, exp: i64) -> i64 {
    match u32::try_from(exp) {
        Ok(e) => base.pow(e),
        // A negative exponent is not an integer power in this rung.
        Err(_) => 0,
    }
}

/// Parse + evaluate `src`, reducing the result to a [`Num`] so the value-bearing
/// tests below can assert against `Num::Int`/`Num::Float` directly. A
/// string-typed result (only the membership/dialect paths produce booleans,
/// which are `Num::Int`) is an error here.
fn eval_tower(src: &str) -> Result<Num, EvalError> {
    let node = parse_expr(src, None);
    let mut ops = Tower::default();
    match eval(&node, &mut ops)? {
        Val::N(n) => Ok(n),
        Val::S(s) => Err(EvalError(format!("string result: {s}"))),
    }
}

#[test]
fn eval_integer_arithmetic_results() {
    // Each pinned to tclsh (8.6 == 9.0).
    // tclsh: 1+2*3=7  (2+3)*4=20  10-3-2=5  2*3%4=2
    assert_eq!(eval_tower("1 + 2 * 3").unwrap(), Num::Int(7));
    assert_eq!(eval_tower("(2 + 3) * 4").unwrap(), Num::Int(20));
    assert_eq!(eval_tower("10 - 3 - 2").unwrap(), Num::Int(5));
    assert_eq!(eval_tower("2 * 3 % 4").unwrap(), Num::Int(2));
    // tclsh: 2**3**2=512 (right-assoc)  2**10=1024
    assert_eq!(eval_tower("2 ** 3 ** 2").unwrap(), Num::Int(512));
    assert_eq!(eval_tower("2 ** 10").unwrap(), Num::Int(1024));
}

#[test]
fn eval_floor_division_and_modulo() {
    // The interesting Tcl-specific semantics: floor division + sign-of-divisor
    // modulo. tclsh: 7/2=3  -7/2=-4  7/-2=-4  7%-3=-2  -7%3=2  9%2=1
    assert_eq!(eval_tower("7 / 2").unwrap(), Num::Int(3));
    assert_eq!(eval_tower("-7 / 2").unwrap(), Num::Int(-4));
    assert_eq!(eval_tower("7 / -2").unwrap(), Num::Int(-4));
    assert_eq!(eval_tower("7 % -3").unwrap(), Num::Int(-2));
    assert_eq!(eval_tower("-7 % 3").unwrap(), Num::Int(2));
    assert_eq!(eval_tower("9 % 2").unwrap(), Num::Int(1));
}

#[test]
fn eval_bitwise_and_shift() {
    // tclsh: 6&3=2  4|1=5  5^1=4  1<<4=16  256>>2=64  ~0=-1
    assert_eq!(eval_tower("6 & 3").unwrap(), Num::Int(2));
    assert_eq!(eval_tower("4 | 1").unwrap(), Num::Int(5));
    assert_eq!(eval_tower("5 ^ 1").unwrap(), Num::Int(4));
    assert_eq!(eval_tower("1 << 4").unwrap(), Num::Int(16));
    assert_eq!(eval_tower("256 >> 2").unwrap(), Num::Int(64));
    assert_eq!(eval_tower("~0").unwrap(), Num::Int(-1));
    // Precedence across families. tclsh: 1|2&3=3  1^2|4=7
    assert_eq!(eval_tower("1 | 2 & 3").unwrap(), Num::Int(3));
    assert_eq!(eval_tower("1 ^ 2 | 4").unwrap(), Num::Int(7));
}

#[test]
fn eval_float_arithmetic_promotes() {
    // Any Float operand promotes the result to the Float rung.
    // tclsh: 1.5+2.5=4.0  10.0/4=2.5  7.5-2.0=5.5  1.5+1=2.5
    float_eq(eval_tower("1.5 + 2.5").unwrap(), 4.0);
    float_eq(eval_tower("10.0 / 4").unwrap(), 2.5);
    float_eq(eval_tower("7.5 - 2.0").unwrap(), 5.5);
    float_eq(eval_tower("1.5 + 1").unwrap(), 2.5);
    // tclsh: 10.0/3=3.3333333333333335
    approx(eval_tower("10.0 / 3").unwrap(), 10.0_f64 / 3.0);
}

#[test]
fn eval_unary_results() {
    // tclsh: -5=-5  +5=5  ~0=-1  !0=1  !1=0  !2.5=0  !0.0=1  -3.5=-3.5  --5=5
    assert_eq!(eval_tower("-5").unwrap(), Num::Int(-5));
    assert_eq!(eval_tower("+5").unwrap(), Num::Int(5));
    assert_eq!(eval_tower("~0").unwrap(), Num::Int(-1));
    assert_eq!(eval_tower("!0").unwrap(), Num::Int(1));
    assert_eq!(eval_tower("!1").unwrap(), Num::Int(0));
    assert_eq!(eval_tower("!2.5").unwrap(), Num::Int(0));
    assert_eq!(eval_tower("!0.0").unwrap(), Num::Int(1));
    float_eq(eval_tower("-3.5").unwrap(), -3.5);
    // Two prefix minuses (no decrement operator in Tcl). tclsh: --5=5
    assert_eq!(eval_tower("--5").unwrap(), Num::Int(5));
}

#[test]
fn eval_numeric_comparisons_both_rungs() {
    // Integer comparisons. tclsh: 2<10=1  3>2=1  2<=2=1  2>=3=0  10==10=1  1!=2=1
    assert_eq!(eval_tower("2 < 10").unwrap(), Num::Int(1));
    assert_eq!(eval_tower("3 > 2").unwrap(), Num::Int(1));
    assert_eq!(eval_tower("2 <= 2").unwrap(), Num::Int(1));
    assert_eq!(eval_tower("2 >= 3").unwrap(), Num::Int(0));
    assert_eq!(eval_tower("10 == 10").unwrap(), Num::Int(1));
    assert_eq!(eval_tower("1 != 2").unwrap(), Num::Int(1));
    // Cross-rung numeric compare (the `compare_numeric` Some path with a
    // Float operand). tclsh: 3.0==3=1  3.5<4=1  2.5>2=1  10==10.0=1
    assert_eq!(eval_tower("3.0 == 3").unwrap(), Num::Int(1));
    assert_eq!(eval_tower("3.5 < 4").unwrap(), Num::Int(1));
    assert_eq!(eval_tower("2.5 > 2").unwrap(), Num::Int(1));
    assert_eq!(eval_tower("10 == 10.0").unwrap(), Num::Int(1));
}

#[test]
fn eval_string_quoted_numeric_operands_compare_numerically() {
    // Tcl's numeric-vs-string rule: `"2"` and `"10"` both LOOK numeric, so the
    // `==`/`<` family compares them NUMERICALLY, not lexically.
    // tclsh: {"2" < "10"} == 1   (lexical would be 0)
    assert_eq!(eval_tower(r#""2" < "10""#).unwrap(), Num::Int(1));
    // tclsh: {"3" == "3"} == 1
    assert_eq!(eval_tower(r#""3" == "3""#).unwrap(), Num::Int(1));
}

#[test]
fn eval_membership_in_ni() {
    // tclsh: 2 in {1 2 3}=1  5 in {1 2 3}=0  5 ni {1 2 3}=1  2 ni {1 2 3}=0
    assert_eq!(eval_tower("2 in {1 2 3}").unwrap(), Num::Int(1));
    assert_eq!(eval_tower("5 in {1 2 3}").unwrap(), Num::Int(0));
    assert_eq!(eval_tower("5 ni {1 2 3}").unwrap(), Num::Int(1));
    assert_eq!(eval_tower("2 ni {1 2 3}").unwrap(), Num::Int(0));
}

#[test]
fn eval_ternary_selects_branch() {
    // tclsh: 1?20:30=20  0?20:30=30  1?2.5:3.5=2.5  0?2.5:3.5=3.5
    assert_eq!(eval_tower("1 ? 20 : 30").unwrap(), Num::Int(20));
    assert_eq!(eval_tower("0 ? 20 : 30").unwrap(), Num::Int(30));
    float_eq(eval_tower("1 ? 2.5 : 3.5").unwrap(), 2.5);
    float_eq(eval_tower("0 ? 2.5 : 3.5").unwrap(), 3.5);
    // A ternary whose condition is a comparison. tclsh: (3<4)?1:0 == 1
    assert_eq!(eval_tower("3 < 4 ? 1 : 0").unwrap(), Num::Int(1));
}

#[test]
fn eval_short_circuit_logical_skips_right() {
    // `&&` false-left: the right operand (a `[cmd]`) must NOT be evaluated.
    let node = parse_expr("0 && [boom]", None);
    let mut ops = Tower::default();
    assert_eq!(eval(&node, &mut ops).unwrap(), Val::N(Num::Int(0)));
    assert!(
        ops.drain_commands().is_empty(),
        "&& right side must be skipped"
    );

    // `||` true-left: the right operand must NOT be evaluated either.
    let node = parse_expr("1 || [boom]", None);
    let mut ops = Tower::default();
    assert_eq!(eval(&node, &mut ops).unwrap(), Val::N(Num::Int(1)));
    assert!(
        ops.drain_commands().is_empty(),
        "|| right side must be skipped"
    );

    // The evaluated paths still produce 0/1. tclsh: 1&&1=1 1&&0=0 0||1=1 0||0=0
    // 5&&3=1
    assert_eq!(eval_tower("1 && 1").unwrap(), Num::Int(1));
    assert_eq!(eval_tower("1 && 0").unwrap(), Num::Int(0));
    assert_eq!(eval_tower("0 || 1").unwrap(), Num::Int(1));
    assert_eq!(eval_tower("0 || 0").unwrap(), Num::Int(0));
    assert_eq!(eval_tower("5 && 3").unwrap(), Num::Int(1));
    // And-binds-tighter-than-or, value-checked. tclsh: 1||0&&0=1
    assert_eq!(eval_tower("1 || 0 && 0").unwrap(), Num::Int(1));
}

#[test]
fn eval_math_function_calls_end_to_end() {
    // The `Call` node routes through `mathfunc::dispatch`.
    // tclsh: abs(-5)=5  sqrt(16)=4.0  hypot(3,4)=5.0  max(1,2)=2  min(5,abs(-2))=2
    assert_eq!(eval_tower("abs(-5)").unwrap(), Num::Int(5));
    float_eq(eval_tower("sqrt(16)").unwrap(), 4.0);
    float_eq(eval_tower("hypot(3, 4)").unwrap(), 5.0);
    assert_eq!(eval_tower("max(1, 2)").unwrap(), Num::Int(2));
    // Nested call as an argument.
    assert_eq!(eval_tower("min(5, abs(-2))").unwrap(), Num::Int(2));
    // Call embedded in arithmetic. tclsh: abs(-5)+1=6  max(1,2)*3=6  int(3.9)+1=4
    assert_eq!(eval_tower("abs(-5) + 1").unwrap(), Num::Int(6));
    assert_eq!(eval_tower("max(1, 2) * 3").unwrap(), Num::Int(6));
    assert_eq!(eval_tower("int(3.9) + 1").unwrap(), Num::Int(4));
    // round ties away from zero, value-checked through the walker.
    // tclsh: round(-2.5)=-3
    assert_eq!(eval_tower("round(-2.5)").unwrap(), Num::Int(-3));
}

#[test]
fn eval_variables_and_command_substitution() {
    // `$var` reads the fixed environment; `[cmd]` is the stub (=100).
    // a=3 b=4 x=10 ⇒ tclsh-mirroring: $a + $b = 7, $x * 2 = 20.
    assert_eq!(eval_tower("$a + $b").unwrap(), Num::Int(7));
    assert_eq!(eval_tower("$x * 2").unwrap(), Num::Int(20));
    // A command substitution is evaluated through the `command` seam.
    let node = parse_expr("[llength $list] + 1", None);
    let mut ops = Tower::default();
    assert_eq!(eval(&node, &mut ops).unwrap(), Val::N(Num::Int(101)));
    assert_eq!(ops.drain_commands(), vec!["llength $list".to_string()]);
}

#[test]
fn eval_binary_other_override_is_reached() {
    // The overridden `binary_other` (NOT the trait default) computes the
    // dialect `equals` operator on two numeric operands. Parsed under the
    // f5-irules dialect so `equals` tokenises as an operator.
    let node = parse_expr("5 equals 5", Some("f5-irules"));
    let mut ops = Tower::default();
    assert_eq!(eval(&node, &mut ops).unwrap(), Val::N(Num::Int(1)));
    let node = parse_expr("5 equals 6", Some("f5-irules"));
    let mut ops = Tower::default();
    assert_eq!(eval(&node, &mut ops).unwrap(), Val::N(Num::Int(0)));
}

#[test]
fn eval_errors_propagate_from_seams() {
    // A `Raw` (unparseable) node surfaces `unsupported`.
    let node = ExprNode::Raw { text: "@#%".into() };
    let mut ops = Tower::default();
    assert_eq!(
        eval(&node, &mut ops).unwrap_err(),
        EvalError("unsupported: syntax error in expression".into())
    );
    // An unknown variable surfaces the `var` seam's error and aborts the walk.
    assert!(eval_tower("$undefined + 1").is_err());
    // A bad math call (unknown function) surfaces the `call` seam's error.
    assert!(eval_tower("frobnicate(1)").is_err());
    // Error in a nested operand short-circuits the whole evaluation.
    assert!(eval_tower("($undefined + 1) * 2").is_err());
    // Division by zero surfaces from the `arith` seam.
    assert!(eval_tower("1 / 0").is_err());
}
