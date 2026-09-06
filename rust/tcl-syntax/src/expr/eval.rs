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

//! The shared `expr` evaluator — the one tree-walk over the [`ExprNode`] AST,
//! generic over a value type via the [`ExprOps`] trait. This is the evaluation
//! parallel of sharing the lexer/parser: the *structure* of `expr` semantics
//! (operator dispatch, short-circuit `&&`/`||`, `?:`, the numeric-vs-string
//! comparison rule, `eq`/`ne` always-string, `in`/`ni` membership) lives **once**
//! here, and each consumer plugs in its value operations:
//!
//! - the **runtime** over the full numeric tower (`i64`/bignum/`double` on
//!   `Tcl_Obj`),
//! - the **compiler's const-folder** over its `TclValue` (which bails — returns
//!   its "can't fold" — on anything it doesn't model).
//!
//! Only the value-type-specific bits (the actual arithmetic, value construction,
//! `$var`/`[cmd]` resolution, boolean coercion) are per consumer; the grammar of
//! evaluation is not re-derived.

use core::cmp::Ordering;

use super::ast::{BinOp, ExprNode, UnaryOp};

/// Outcome of a numeric comparison between two number-classified operands.
///
/// C Tcl's `==`/`<`/… are only *mostly* a three-way ordering: a NaN operand is
/// still a **number** (so the string-comparison fallback must not apply) but
/// orders against nothing — `tclExecute.c`'s rule is "NaN arg: NaN != to
/// everything, other compares are false". [`eval`] maps [`Self::Unordered`]
/// accordingly; a plain `Option<Ordering>` cannot carry that third state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericCompare {
    /// Both operands are ordered numbers.
    Ordered(Ordering),
    /// At least one operand is NaN.
    Unordered,
}

impl NumericCompare {
    /// Wrap an [`Ordering`], or [`Self::Unordered`] for `None` — the shape
    /// `f64::partial_cmp` hands back.
    #[must_use]
    pub fn from_partial(ord: Option<Ordering>) -> Self {
        ord.map_or(Self::Unordered, Self::Ordered)
    }
}

/// The value operations an `expr` consumer supplies. The shared [`eval`] walker
/// drives these; it owns the dispatch, short-circuit, ternary, and the
/// numeric-vs-string comparison rule.
pub trait ExprOps {
    /// The consumer's value type (a `Tcl_Obj` pointer, a const-fold value, …).
    type Value;
    /// The consumer's error type.
    type Error;

    /// A numeric/boolean literal token (`42`, `0xff`, `1.5`, `true`).
    fn literal(&mut self, text: &str) -> Result<Self::Value, Self::Error>;
    /// A quoted/braced string operand (delimiters already stripped).
    fn string(&mut self, inner: &str) -> Result<Self::Value, Self::Error>;
    /// Resolve a `$name` reference.
    fn var(&mut self, name: &str) -> Result<Self::Value, Self::Error>;
    /// Evaluate a `[script]` (brackets already stripped).
    fn command(&mut self, script: &str) -> Result<Self::Value, Self::Error>;
    /// A `func(args…)` math-function call (dispatched through `::tcl::mathfunc`).
    fn call(&mut self, function: &str, args: Vec<Self::Value>) -> Result<Self::Value, Self::Error>;

    /// An arithmetic / bitwise / shift binary op (`op` is one of the arithmetic
    /// set — `Add`..`RShift`); the consumer applies its tower/fold semantics.
    fn arith(
        &mut self,
        op: BinOp,
        left: Self::Value,
        right: Self::Value,
    ) -> Result<Self::Value, Self::Error>;
    /// A unary op (`-`/`+`/`~`/`!`).
    fn unary(&mut self, op: UnaryOp, value: Self::Value) -> Result<Self::Value, Self::Error>;

    /// A binary op the shared core does not handle — the dialect operators
    /// (`contains`/`starts_with`/`matches_glob`/…). Standard consumers leave the
    /// default (`unsupported`); a dialect-aware consumer (the compiler's
    /// const-folder) overrides this. Operands are already evaluated.
    fn binary_other(
        &mut self,
        _op: BinOp,
        _left: Self::Value,
        _right: Self::Value,
    ) -> Result<Self::Value, Self::Error> {
        Err(self.unsupported("operator"))
    }

    /// Numeric comparison, or `None` when an operand is non-numeric (the
    /// walker then falls back to [`ExprOps::compare_string`] — the Tcl
    /// `==`/`<`… "numeric when both look numeric, else string" rule). A NaN
    /// operand is numeric but unordered: return
    /// `Some(NumericCompare::Unordered)`, **not** `None` — Tcl does not
    /// string-compare NaN, it applies the "`!=` true, everything else false"
    /// rule (which the walker owns).
    fn compare_numeric(
        &mut self,
        left: &Self::Value,
        right: &Self::Value,
    ) -> Option<NumericCompare>;
    /// String comparison (for `eq`/`ne`/`lt`… and the `==` string fallback).
    fn compare_string(&mut self, left: &Self::Value, right: &Self::Value) -> Ordering;
    /// `needle in list` membership (string equality of elements).
    fn in_list(&mut self, needle: &Self::Value, list: &Self::Value) -> Result<bool, Self::Error>;

    /// Tcl boolean coercion (`Tcl_GetBoolean`) for conditions / `&&`/`||`/`!`.
    fn to_bool(&mut self, value: &Self::Value) -> Result<bool, Self::Error>;
    /// Construct a boolean result value (`0`/`1`).
    fn bool_value(&mut self, b: bool) -> Self::Value;

    /// Build the error for an unsupported construct / `Raw` (unparseable) node.
    fn unsupported(&mut self, what: &str) -> Self::Error;
}

/// Evaluate `node` against the consumer's [`ExprOps`].
pub fn eval<O: ExprOps>(node: &ExprNode, ops: &mut O) -> Result<O::Value, O::Error> {
    match node {
        ExprNode::Literal { text, .. } => ops.literal(text),
        ExprNode::String { text, .. } => ops.string(strip_delims(text)),
        // The value as-is — there are no delimiters to strip, which is the
        // whole reason this operand exists: `strip_delims` on a value that
        // merely looks braced (`switch -- "{abc}"`) takes a layer that was
        // never source.
        //
        // A braced word's value is literal, so it always evaluates. An
        // unbraced one is a constant only while nothing is left for the word
        // substitution to do; with a `${…}` or `[…]` still live the value is
        // not known here and the evaluator must decline rather than fold the
        // unsubstituted text. That is the same marker test the codegen side
        // applies (`push_word_value`).
        ExprNode::CompiledWord { text, braced } => {
            if *braced || !(text.contains("${") || text.contains('[')) {
                ops.string(text)
            } else {
                Err(ops.unsupported("compiled word with a live substitution"))
            }
        }
        // Pass the index-preserving reference (`arr(idx)`), not the base name,
        // so an evaluator can read array elements; the base `name` field stays
        // for analysis (`.vars_with_grammar(...)`).
        ExprNode::Var { text, .. } => ops.var(crate::naming::var_reference(text)),
        ExprNode::Command { text, .. } => ops.command(strip_brackets(text)),
        ExprNode::Unary { op, operand } => {
            let v = eval(operand, ops)?;
            ops.unary(*op, v)
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            let c = eval(condition, ops)?;
            if ops.to_bool(&c)? {
                eval(true_branch, ops)
            } else {
                eval(false_branch, ops)
            }
        }
        ExprNode::Call { function, args, .. } => {
            let mut vals = Vec::with_capacity(args.len());
            for a in args {
                vals.push(eval(a, ops)?);
            }
            ops.call(function, vals)
        }
        ExprNode::Binary { op, left, right } => eval_binary(*op, left, right, ops),
        ExprNode::Raw { .. } => Err(ops.unsupported("syntax error in expression")),
    }
}

fn eval_binary<O: ExprOps>(
    op: BinOp,
    left: &ExprNode,
    right: &ExprNode,
    ops: &mut O,
) -> Result<O::Value, O::Error> {
    // Short-circuit logical operators: the right operand is evaluated lazily.
    match op {
        BinOp::And | BinOp::WordAnd => {
            let l = eval(left, ops)?;
            let lb = ops.to_bool(&l)?;
            let r = if lb {
                let rv = eval(right, ops)?;
                ops.to_bool(&rv)?
            } else {
                false
            };
            return Ok(ops.bool_value(lb && r));
        }
        BinOp::Or | BinOp::WordOr => {
            let l = eval(left, ops)?;
            let lb = ops.to_bool(&l)?;
            let r = if lb {
                true
            } else {
                let rv = eval(right, ops)?;
                ops.to_bool(&rv)?
            };
            return Ok(ops.bool_value(lb || r));
        }
        _ => {}
    }

    let l = eval(left, ops)?;
    let r = eval(right, ops)?;

    // Arithmetic / bitwise / shift → the consumer's value ops.
    if matches!(
        op,
        BinOp::Add
            | BinOp::Sub
            | BinOp::Mul
            | BinOp::Div
            | BinOp::Mod
            | BinOp::Pow
            | BinOp::BitAnd
            | BinOp::BitOr
            | BinOp::BitXor
            | BinOp::LShift
            | BinOp::RShift
    ) {
        return ops.arith(op, l, r);
    }

    // Otherwise it's a comparison / membership → a boolean. Compute the boolean
    // first (releasing the `ops` borrow) before constructing the result value.
    let b = match op {
        // `==`/`!=`/`<`… numeric when both look numeric, else string; a NaN
        // operand is unordered — unequal to everything, ordered before/after
        // nothing (`tclExecute.c`: "NaN arg: NaN != to everything, other
        // compares are false").
        BinOp::Eq => matches!(num_or_str(ops, &l, &r), NumericCompare::Ordered(o) if o.is_eq()),
        BinOp::Ne => !matches!(num_or_str(ops, &l, &r), NumericCompare::Ordered(o) if o.is_eq()),
        BinOp::Lt => matches!(num_or_str(ops, &l, &r), NumericCompare::Ordered(o) if o.is_lt()),
        BinOp::Le => matches!(num_or_str(ops, &l, &r), NumericCompare::Ordered(o) if o.is_le()),
        BinOp::Gt => matches!(num_or_str(ops, &l, &r), NumericCompare::Ordered(o) if o.is_gt()),
        BinOp::Ge => matches!(num_or_str(ops, &l, &r), NumericCompare::Ordered(o) if o.is_ge()),
        // `eq`/`ne`/`lt`… always string-compare.
        BinOp::StrEq | BinOp::StrEquals => ops.compare_string(&l, &r).is_eq(),
        BinOp::StrNe => !ops.compare_string(&l, &r).is_eq(),
        BinOp::StrLt => ops.compare_string(&l, &r).is_lt(),
        BinOp::StrLe => ops.compare_string(&l, &r).is_le(),
        BinOp::StrGt => ops.compare_string(&l, &r).is_gt(),
        BinOp::StrGe => ops.compare_string(&l, &r).is_ge(),
        // List membership.
        BinOp::In => ops.in_list(&l, &r)?,
        BinOp::Ni => !ops.in_list(&l, &r)?,
        // `&&`/`||` handled above; the remaining (dialect) operators go to the
        // consumer's `binary_other` hook — which returns a value directly, so
        // short-circuit out of the boolean path here.
        _ => return ops.binary_other(op, l, r),
    };
    Ok(ops.bool_value(b))
}

/// The numeric-or-string comparison rule: numeric when both operands compare
/// numerically (possibly unordered, for NaN), else a string comparison.
fn num_or_str<O: ExprOps>(ops: &mut O, l: &O::Value, r: &O::Value) -> NumericCompare {
    match ops.compare_numeric(l, r) {
        Some(outcome) => outcome,
        None => NumericCompare::Ordered(ops.compare_string(l, r)),
    }
}

/// Strip one layer of `{}`/`""` delimiters from a string-literal token.
fn strip_delims(text: &str) -> &str {
    let b = text.as_bytes();
    if b.len() >= 2
        && ((b[0] == b'{' && b[b.len() - 1] == b'}') || (b[0] == b'"' && b[b.len() - 1] == b'"'))
    {
        &text[1..text.len() - 1]
    } else {
        text
    }
}

/// Strip the `[`/`]` brackets from a command-substitution token.
fn strip_brackets(text: &str) -> &str {
    text.strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::parser::parse_expr;

    /// A minimal two-rung value: integers compare numerically, everything else
    /// string-compares — enough to drive every branch of the shared walker.
    #[derive(Debug, Clone, PartialEq)]
    enum V {
        Num(i64),
        Str(String),
    }
    impl V {
        fn as_string(&self) -> String {
            match self {
                V::Num(n) => n.to_string(),
                V::Str(s) => s.clone(),
            }
        }
    }

    /// Records which seam methods fired so the tests can assert dispatch, not
    /// just the final value.
    #[derive(Default)]
    struct Ops {
        commands: Vec<String>,
        calls: Vec<String>,
    }

    impl ExprOps for Ops {
        type Value = V;
        type Error = String;

        fn literal(&mut self, text: &str) -> Result<V, String> {
            text.parse::<i64>()
                .map(V::Num)
                .or_else(|_| Ok(V::Str(text.to_string())))
        }
        fn string(&mut self, inner: &str) -> Result<V, String> {
            Ok(V::Str(inner.to_string()))
        }
        fn var(&mut self, name: &str) -> Result<V, String> {
            // `x` → 10, `arr(idx)` echoes its reference text, else 0.
            Ok(match name {
                "x" => V::Num(10),
                "y" => V::Num(0),
                other => V::Str(other.to_string()),
            })
        }
        fn command(&mut self, script: &str) -> Result<V, String> {
            self.commands.push(script.to_string());
            Ok(V::Num(7))
        }
        fn call(&mut self, function: &str, args: Vec<V>) -> Result<V, String> {
            self.calls.push(format!("{function}/{}", args.len()));
            // `max` of two numbers; otherwise echo the arg count.
            if function == "max"
                && args.len() == 2
                && let (V::Num(a), V::Num(b)) = (&args[0], &args[1])
            {
                return Ok(V::Num((*a).max(*b)));
            }
            Ok(V::Num(i64::try_from(args.len()).unwrap_or(i64::MAX)))
        }
        fn arith(&mut self, op: BinOp, left: V, right: V) -> Result<V, String> {
            let (V::Num(a), V::Num(b)) = (&left, &right) else {
                return Err(self.unsupported("non-numeric arith"));
            };
            let (a, b) = (*a, *b);
            Ok(V::Num(match op {
                BinOp::Add => a + b,
                BinOp::Sub => a - b,
                BinOp::Mul => a * b,
                BinOp::Div => a / b,
                BinOp::Mod => a % b,
                BinOp::Pow => a.pow(u32::try_from(b).unwrap_or(0)),
                BinOp::BitAnd => a & b,
                BinOp::BitOr => a | b,
                BinOp::BitXor => a ^ b,
                BinOp::LShift => a << b,
                BinOp::RShift => a >> b,
                _other => return Err(self.unsupported("bad arith op")),
            }))
        }
        fn unary(&mut self, op: UnaryOp, value: V) -> Result<V, String> {
            let V::Num(n) = value else {
                return Err(self.unsupported("non-numeric unary"));
            };
            Ok(match op {
                UnaryOp::Neg => V::Num(-n),
                UnaryOp::Pos => V::Num(n),
                UnaryOp::BitNot => V::Num(!n),
                UnaryOp::Not | UnaryOp::WordNot => V::Num(i64::from(n == 0)),
            })
        }
        fn compare_numeric(&mut self, left: &V, right: &V) -> Option<NumericCompare> {
            match (left, right) {
                (V::Num(a), V::Num(b)) => Some(NumericCompare::Ordered(a.cmp(b))),
                _ => None,
            }
        }
        fn compare_string(&mut self, left: &V, right: &V) -> Ordering {
            left.as_string().cmp(&right.as_string())
        }
        fn in_list(&mut self, needle: &V, list: &V) -> Result<bool, String> {
            let n = needle.as_string();
            Ok(list.as_string().split_whitespace().any(|e| e == n))
        }
        fn to_bool(&mut self, value: &V) -> Result<bool, String> {
            Ok(match value {
                V::Num(n) => *n != 0,
                V::Str(s) => s == "1" || s == "true",
            })
        }
        fn bool_value(&mut self, b: bool) -> V {
            V::Num(i64::from(b))
        }
        fn unsupported(&mut self, what: &str) -> String {
            format!("unsupported: {what}")
        }
        // NB: `binary_other` intentionally left as the trait default so the
        // default (`Err(unsupported)`) body is exercised.
    }

    fn eval_str(src: &str) -> Result<V, String> {
        let node = parse_expr(src, None);
        let mut ops = Ops::default();
        eval(&node, &mut ops)
    }

    // The `Ops` mock's numeric/comparison/membership results below were
    // cross-checked against `tclsh8.6` and `tclsh9.0` so the walker's dispatch
    // is exercised against C-Tcl-accurate expectations:
    //   expr {9/2}==4  {2**3}==8  {5^1}==4  {16>>2}==4  {6&3}==2  {4|1}==5
    //   {1<<3}==8  {9%2}==1  {-5}==-5  {~0}==-1  {!0}==1  {"abc"<"abd"}==1
    //   {2 in {1 2 3}}==1  {5 ni {1 2 3}}==1  {1?20:30}==20  {0?20:30}==30
    //   {max(3,9)}==9. NB `lt`/`le`/`gt`/`ge` are 9.0-only operators (error in
    //   8.6); the Rust parser models them under the default/latest dialect.

    #[test]
    fn literal_string_var_command() {
        assert_eq!(eval_str("42").unwrap(), V::Num(42));
        assert_eq!(eval_str("{hello}").unwrap(), V::Str("hello".into()));
        assert_eq!(eval_str("\"hi\"").unwrap(), V::Str("hi".into()));
        assert_eq!(eval_str("$x").unwrap(), V::Num(10));
        // Command substitution routes through `command` (brackets stripped).
        let node = parse_expr("[foo bar]", None);
        let mut ops = Ops::default();
        assert_eq!(eval(&node, &mut ops).unwrap(), V::Num(7));
        assert_eq!(ops.commands, vec!["foo bar".to_string()]);
    }

    #[test]
    fn unary_ops() {
        assert_eq!(eval_str("-5").unwrap(), V::Num(-5));
        assert_eq!(eval_str("+5").unwrap(), V::Num(5));
        assert_eq!(eval_str("~0").unwrap(), V::Num(-1));
        assert_eq!(eval_str("!0").unwrap(), V::Num(1));
        assert_eq!(eval_str("!1").unwrap(), V::Num(0));
    }

    #[test]
    fn ternary_both_branches() {
        assert_eq!(eval_str("1 ? 20 : 30").unwrap(), V::Num(20));
        assert_eq!(eval_str("0 ? 20 : 30").unwrap(), V::Num(30));
    }

    #[test]
    fn call_dispatch() {
        assert_eq!(eval_str("max(3, 9)").unwrap(), V::Num(9));
        let node = parse_expr("max(1, 2)", None);
        let mut ops = Ops::default();
        eval(&node, &mut ops).unwrap();
        assert_eq!(ops.calls, vec!["max/2".to_string()]);
    }

    #[test]
    fn arithmetic_ops() {
        for (src, want) in [
            ("1 + 2", 3),
            ("5 - 3", 2),
            ("4 * 3", 12),
            ("9 / 2", 4),
            ("9 % 2", 1),
            ("2 ** 3", 8),
            ("6 & 3", 2),
            ("4 | 1", 5),
            ("5 ^ 1", 4),
            ("1 << 3", 8),
            ("16 >> 2", 4),
        ] {
            assert_eq!(eval_str(src).unwrap(), V::Num(want), "{src}");
        }
    }

    #[test]
    fn numeric_comparisons() {
        assert_eq!(eval_str("1 == 1").unwrap(), V::Num(1));
        assert_eq!(eval_str("1 != 2").unwrap(), V::Num(1));
        assert_eq!(eval_str("1 < 2").unwrap(), V::Num(1));
        assert_eq!(eval_str("2 <= 2").unwrap(), V::Num(1));
        assert_eq!(eval_str("3 > 2").unwrap(), V::Num(1));
        assert_eq!(eval_str("2 >= 3").unwrap(), V::Num(0));
    }

    #[test]
    fn string_fallback_comparisons() {
        // Both operands non-numeric → `num_or_str` falls back to compare_string.
        assert_eq!(eval_str("\"abc\" == \"abc\"").unwrap(), V::Num(1));
        assert_eq!(eval_str("\"abc\" < \"abd\"").unwrap(), V::Num(1));
        // `eq`/`ne`/`lt`… always string-compare.
        assert_eq!(eval_str("\"a\" eq \"a\"").unwrap(), V::Num(1));
        assert_eq!(eval_str("\"a\" ne \"b\"").unwrap(), V::Num(1));
        assert_eq!(eval_str("\"a\" lt \"b\"").unwrap(), V::Num(1));
        assert_eq!(eval_str("\"a\" le \"a\"").unwrap(), V::Num(1));
        assert_eq!(eval_str("\"b\" gt \"a\"").unwrap(), V::Num(1));
        assert_eq!(eval_str("\"b\" ge \"b\"").unwrap(), V::Num(1));
    }

    #[test]
    fn membership_ops() {
        assert_eq!(eval_str("2 in {1 2 3}").unwrap(), V::Num(1));
        assert_eq!(eval_str("5 in {1 2 3}").unwrap(), V::Num(0));
        assert_eq!(eval_str("5 ni {1 2 3}").unwrap(), V::Num(1));
        assert_eq!(eval_str("2 ni {1 2 3}").unwrap(), V::Num(0));
    }

    #[test]
    fn short_circuit_logical() {
        // `&&`: when left is false the right is NOT evaluated (records nothing).
        let node = parse_expr("0 && [boom]", None);
        let mut ops = Ops::default();
        assert_eq!(eval(&node, &mut ops).unwrap(), V::Num(0));
        assert!(ops.commands.is_empty(), "right side must be skipped");
        // `&&` true path evaluates the right operand.
        assert_eq!(eval_str("1 && 1").unwrap(), V::Num(1));
        assert_eq!(eval_str("1 && 0").unwrap(), V::Num(0));
        // `||`: when left is true the right is NOT evaluated.
        let node = parse_expr("1 || [boom]", None);
        let mut ops = Ops::default();
        assert_eq!(eval(&node, &mut ops).unwrap(), V::Num(1));
        assert!(ops.commands.is_empty(), "right side must be skipped");
        // `||` false path evaluates the right operand.
        assert_eq!(eval_str("0 || 1").unwrap(), V::Num(1));
        assert_eq!(eval_str("0 || 0").unwrap(), V::Num(0));
    }

    #[test]
    fn dialect_operator_routes_to_binary_other_default() {
        // A dialect operator (`Contains`) the shared core doesn't handle hits the
        // trait-default `binary_other`, which returns `unsupported`.
        let node = ExprNode::Binary {
            op: BinOp::Contains,
            left: Box::new(parse_expr("1", None)),
            right: Box::new(parse_expr("2", None)),
        };
        let mut ops = Ops::default();
        let err = eval(&node, &mut ops).unwrap_err();
        assert_eq!(err, "unsupported: operator");
    }

    #[test]
    fn raw_node_is_unsupported() {
        let node = ExprNode::Raw {
            text: "@#%".to_string(),
        };
        let mut ops = Ops::default();
        let err = eval(&node, &mut ops).unwrap_err();
        assert_eq!(err, "unsupported: syntax error in expression");
    }

    #[test]
    fn error_propagates_from_operand_seams() {
        // A non-numeric arithmetic operand surfaces the consumer's error
        // (the `?` propagation through eval_binary's arith arm).
        assert!(eval_str("\"abc\" + 1").is_err());
        // Non-numeric unary operand likewise propagates.
        assert!(eval_str("-\"abc\"").is_err());
        // Error from a nested operand short-circuits the whole walk.
        assert!(eval_str("(\"x\" + 1) * 2").is_err());
    }

    #[test]
    fn strip_delims_variants() {
        assert_eq!(strip_delims("{abc}"), "abc");
        assert_eq!(strip_delims("\"abc\""), "abc");
        assert_eq!(strip_delims("abc"), "abc");
        assert_eq!(strip_delims("x"), "x"); // too short to strip
    }

    #[test]
    fn strip_brackets_variants() {
        assert_eq!(strip_brackets("[cmd]"), "cmd");
        assert_eq!(strip_brackets("cmd"), "cmd");
    }
}
