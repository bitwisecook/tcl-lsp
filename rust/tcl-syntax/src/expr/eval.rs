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

    /// Numeric three-way comparison, or `None` when an operand is non-numeric
    /// (the walker then falls back to [`ExprOps::compare_string`] — the Tcl
    /// `==`/`<`… "numeric when both look numeric, else string" rule).
    fn compare_numeric(&mut self, left: &Self::Value, right: &Self::Value) -> Option<Ordering>;
    /// String comparison (for `eq`/`ne`/`lt`… and the `==` string fallback).
    fn compare_string(&mut self, left: &Self::Value, right: &Self::Value) -> Ordering;
    /// `needle in list` membership (string equality of elements).
    fn in_list(&mut self, needle: &Self::Value, list: &Self::Value)
        -> Result<bool, Self::Error>;

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
        ExprNode::Var { name, .. } => ops.var(name),
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
        // `==`/`!=`/`<`… numeric when both look numeric, else string.
        BinOp::Eq => num_or_str(ops, &l, &r).is_eq(),
        BinOp::Ne => !num_or_str(ops, &l, &r).is_eq(),
        BinOp::Lt => num_or_str(ops, &l, &r).is_lt(),
        BinOp::Le => num_or_str(ops, &l, &r).is_le(),
        BinOp::Gt => num_or_str(ops, &l, &r).is_gt(),
        BinOp::Ge => num_or_str(ops, &l, &r).is_ge(),
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
        // `&&`/`||` handled above; iRules dialect string ops are consumer-
        // specific and not part of the shared core.
        _ => return Err(ops.unsupported("operator")),
    };
    Ok(ops.bool_value(b))
}

/// The numeric-or-string comparison rule: numeric when both operands compare
/// numerically, else a string comparison.
fn num_or_str<O: ExprOps>(ops: &mut O, l: &O::Value, r: &O::Value) -> Ordering {
    match ops.compare_numeric(l, r) {
        Some(ord) => ord,
        None => ops.compare_string(l, r),
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
