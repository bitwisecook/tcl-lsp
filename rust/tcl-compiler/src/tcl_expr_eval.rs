//! Tcl expression evaluator (compile-time constant folding).
//!
//! Walks the [`ExprNode`] AST produced by [`crate::expr_parser::parse_expr`]
//! and evaluates constant expressions, returning `Some(TclValue)` when
//! the whole expression is determined at compile time. Anything that
//! touches a variable not in the environment, a command substitution,
//! a domain error (division by zero, negative base to non-integer
//! exponent, …), or an unsupported feature (iRules string operators,
//! most math functions) yields `None` — callers treat that as "give
//! up, emit the runtime form".
//!
//! Semantics follow C Tcl 9.0.2 (`tclExecute.c`, `tclBasic.c`):
//!
//! - Integer division floors toward negative infinity.
//! - Integer modulo: sign follows divisor.
//! - Exponentiation: special rules for `|base| ≤ 1` and negative
//!   exponents.
//! - Comparisons always return `Int(0)` or `Int(1)`.
//! - `round()` ties away from zero (not Python/Rust banker's rounding).
//!
//! Ported from `core/compiler/tcl_expr_eval.py` (C22). This is a
//! focused subset: iRules-specific word operators (`contains`,
//! `starts_with`, `matches_glob`, `matches_regex`, `in`, `ni`) and
//! most math-function dispatches are deferred to follow-up work —
//! they return `None` and callers fall through to the runtime path.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]

use std::collections::HashMap;

use crate::expr_ast::{BinOp, ExprNode, UnaryOp};

/// Result of evaluating a constant Tcl expression.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TclValue {
    /// Integer value (Tcl booleans are represented as `Int(0)` / `Int(1)`).
    Int(i64),
    /// IEEE-754 double.
    Float(f64),
}

impl TclValue {
    /// Return the raw float representation (converting integer → float
    /// when necessary). Used by arithmetic that promotes mixed operands.
    #[must_use]
    pub fn as_f64(self) -> f64 {
        match self {
            Self::Int(i) => i as f64,
            Self::Float(f) => f,
        }
    }

    /// True when the value is non-zero (Tcl truthiness).
    #[must_use]
    pub fn is_truthy(self) -> bool {
        match self {
            Self::Int(i) => i != 0,
            Self::Float(f) => f != 0.0,
        }
    }
}

/// Environment value kind — what callers can bind a variable to.
#[derive(Debug, Clone)]
pub enum EnvValue {
    /// Integer binding.
    Int(i64),
    /// Float binding.
    Float(f64),
    /// String binding — decoded as a literal on read.
    Str(String),
}

/// Variable environment for evaluation.
pub type Env = HashMap<String, EnvValue>;

/// Maximum exponent for integer `**` — guards against pathological
/// inputs like `2 ** 999_999_999`.
const MAX_EXPONENT: i64 = 10_000;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Evaluate an expression AST against `env`. Returns `None` when the
/// expression depends on runtime state or triggers a domain error.
#[must_use]
pub fn eval_tcl_expr(node: &ExprNode, env: &Env) -> Option<TclValue> {
    eval(node, env)
}

/// Render a `TclValue` as a Tcl source literal. Matches Tcl's
/// `Tcl_GetStringFromObj` output for numbers.
#[must_use]
pub fn format_tcl_value(v: TclValue) -> String {
    match v {
        TclValue::Int(i) => i.to_string(),
        TclValue::Float(f) => {
            if f.is_nan() {
                "NaN".into()
            } else if f.is_infinite() {
                if f.is_sign_negative() {
                    "-Inf".into()
                } else {
                    "Inf".into()
                }
            } else if f.fract() == 0.0 && f.abs() < 1e16 {
                // Append `.0` for integer-valued floats so they round-trip
                // as floats rather than being reparsed as integers.
                format!("{}.0", f as i64)
            } else {
                format!("{f}")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Core dispatch
// ---------------------------------------------------------------------------

fn eval(node: &ExprNode, env: &Env) -> Option<TclValue> {
    match node {
        ExprNode::Literal { text, .. } => parse_literal(text),
        ExprNode::Var { name, .. } => resolve_var(name, env),
        ExprNode::Binary { op, left, right } => eval_binary(*op, left, right, env),
        ExprNode::Unary { op, operand } => eval_unary(*op, operand, env),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            let cv = eval(condition, env)?;
            if cv.is_truthy() {
                eval(true_branch, env)
            } else {
                eval(false_branch, env)
            }
        }
        ExprNode::Call { function, args, .. } => eval_call(function, args, env),
        // Command substitutions, quoted strings, and raw fallbacks
        // are opaque at compile time.
        ExprNode::Command { .. } | ExprNode::String { .. } | ExprNode::Raw { .. } => None,
    }
}

// ---------------------------------------------------------------------------
// Math function calls
// ---------------------------------------------------------------------------

/// Dispatch a math-function call with already-evaluated operand
/// nodes. Returns `None` for unknown functions, non-deterministic
/// ones (`rand`, `srand`), and any call whose argument count or
/// value doesn't match the function's expected shape.
fn eval_call(function: &str, args: &[ExprNode], env: &Env) -> Option<TclValue> {
    let name = function.to_ascii_lowercase();
    if matches!(name.as_str(), "rand" | "srand") {
        return None;
    }
    let mut vals = Vec::with_capacity(args.len());
    for arg in args {
        vals.push(eval(arg, env)?);
    }
    dispatch_math(&name, &vals)
}

#[allow(clippy::too_many_lines)]
fn dispatch_math(name: &str, vals: &[TclValue]) -> Option<TclValue> {
    // Helpers for common argument shapes.
    let one = || -> Option<TclValue> {
        if vals.len() == 1 {
            Some(vals[0])
        } else {
            None
        }
    };
    let unary_float = |f: fn(f64) -> f64| -> Option<TclValue> {
        let v = one()?;
        let r = f(v.as_f64());
        if r.is_nan() && !v.as_f64().is_nan() {
            None // domain error (e.g. log(-1))
        } else {
            Some(TclValue::Float(r))
        }
    };
    let binary_float = |f: fn(f64, f64) -> f64| -> Option<TclValue> {
        if vals.len() != 2 {
            return None;
        }
        let r = f(vals[0].as_f64(), vals[1].as_f64());
        if r.is_nan() {
            None
        } else {
            Some(TclValue::Float(r))
        }
    };

    match name {
        // Type conversion.
        "abs" => match one()? {
            TclValue::Int(i) => i.checked_abs().map(TclValue::Int),
            TclValue::Float(f) => Some(TclValue::Float(f.abs())),
        },
        "int" | "entier" | "wide" => match one()? {
            TclValue::Int(i) => Some(TclValue::Int(i)),
            TclValue::Float(f) => {
                if f.is_finite() {
                    Some(TclValue::Int(f as i64))
                } else {
                    None
                }
            }
        },
        "double" => Some(TclValue::Float(one()?.as_f64())),
        "bool" => Some(TclValue::Int(i64::from(one()?.is_truthy()))),

        // Rounding — Tcl `round` ties away from zero; `ceil` / `floor`
        // return doubles, matching C Tcl.
        "round" => match one()? {
            TclValue::Int(i) => Some(TclValue::Int(i)),
            TclValue::Float(f) => {
                if !f.is_finite() {
                    None
                } else if f >= 0.0 {
                    Some(TclValue::Int((f + 0.5).floor() as i64))
                } else {
                    Some(TclValue::Int((f - 0.5).ceil() as i64))
                }
            }
        },
        "ceil" => {
            let v = one()?;
            Some(TclValue::Float(v.as_f64().ceil()))
        }
        "floor" => {
            let v = one()?;
            Some(TclValue::Float(v.as_f64().floor()))
        }

        // Variadic min/max — never shrink width.
        "min" | "max" => {
            if vals.is_empty() {
                return None;
            }
            let all_int = vals.iter().all(|v| matches!(v, TclValue::Int(_)));
            if all_int {
                let ints: Vec<i64> = vals
                    .iter()
                    .map(|v| match v {
                        TclValue::Int(i) => *i,
                        TclValue::Float(_) => unreachable!(),
                    })
                    .collect();
                let r = if name == "min" {
                    *ints.iter().min().unwrap()
                } else {
                    *ints.iter().max().unwrap()
                };
                Some(TclValue::Int(r))
            } else {
                let mut best = vals[0].as_f64();
                for v in &vals[1..] {
                    let f = v.as_f64();
                    let take = if name == "min" { f < best } else { f > best };
                    if take {
                        best = f;
                    }
                }
                Some(TclValue::Float(best))
            }
        }

        // Integer sqrt.
        "isqrt" => match one()? {
            TclValue::Int(i) if i >= 0 => Some(TclValue::Int((i as f64).sqrt() as i64)),
            _ => None,
        },

        // Classification (returns int 0/1).
        "isinf" => Some(TclValue::Int(i64::from(matches!(one()?, TclValue::Float(f) if f.is_infinite())))),
        "isnan" => Some(TclValue::Int(i64::from(matches!(one()?, TclValue::Float(f) if f.is_nan())))),
        "isfinite" => match one()? {
            TclValue::Int(_) => Some(TclValue::Int(1)),
            TclValue::Float(f) => Some(TclValue::Int(i64::from(f.is_finite()))),
        },

        // Unary float.
        "sqrt" => unary_float(f64::sqrt),
        "exp" => unary_float(f64::exp),
        "log" => unary_float(f64::ln),
        "log10" => unary_float(f64::log10),
        "sin" => unary_float(f64::sin),
        "cos" => unary_float(f64::cos),
        "tan" => unary_float(f64::tan),
        "asin" => unary_float(f64::asin),
        "acos" => unary_float(f64::acos),
        "atan" => unary_float(f64::atan),
        "sinh" => unary_float(f64::sinh),
        "cosh" => unary_float(f64::cosh),
        "tanh" => unary_float(f64::tanh),

        // Binary float.
        "atan2" => binary_float(f64::atan2),
        "hypot" => binary_float(f64::hypot),
        "fmod" => binary_float(|a, b| a % b),
        "pow" => binary_float(f64::powf),

        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Literals and variables
// ---------------------------------------------------------------------------

/// Parse a numeric/boolean literal. Supports `0x`/`0o`/`0b` prefixes,
/// Tcl-style leading-zero decimals (e.g. `0005`), floats, and the
/// Tcl boolean spellings.
#[must_use]
pub fn parse_literal(text: &str) -> Option<TclValue> {
    let low = text.to_ascii_lowercase();
    if matches!(low.as_str(), "true" | "yes" | "on") {
        return Some(TclValue::Int(1));
    }
    if matches!(low.as_str(), "false" | "no" | "off") {
        return Some(TclValue::Int(0));
    }
    // Hex / octal / binary with prefix.
    if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        if let Ok(i) = i64::from_str_radix(hex, 16) {
            return Some(TclValue::Int(i));
        }
    }
    if let Some(oct) = text.strip_prefix("0o").or_else(|| text.strip_prefix("0O")) {
        if let Ok(i) = i64::from_str_radix(oct, 8) {
            return Some(TclValue::Int(i));
        }
    }
    if let Some(bin) = text.strip_prefix("0b").or_else(|| text.strip_prefix("0B")) {
        if let Ok(i) = i64::from_str_radix(bin, 2) {
            return Some(TclValue::Int(i));
        }
    }
    // Plain decimal (including leading-zero integers — Tcl 9.0 accepts
    // `0005` as decimal, whereas Rust's from_str does too).
    if let Ok(i) = text.parse::<i64>() {
        return Some(TclValue::Int(i));
    }
    if let Ok(f) = text.parse::<f64>() {
        return Some(TclValue::Float(f));
    }
    None
}

fn resolve_var(name: &str, env: &Env) -> Option<TclValue> {
    match env.get(name)? {
        EnvValue::Int(i) => Some(TclValue::Int(*i)),
        EnvValue::Float(f) => Some(TclValue::Float(*f)),
        EnvValue::Str(s) => parse_literal(s),
    }
}

// ---------------------------------------------------------------------------
// Binary operators
// ---------------------------------------------------------------------------

fn eval_binary(op: BinOp, left: &ExprNode, right: &ExprNode, env: &Env) -> Option<TclValue> {
    // Short-circuit logical operators.
    if matches!(op, BinOp::And | BinOp::WordAnd) {
        let lv = eval(left, env)?;
        if !lv.is_truthy() {
            return Some(TclValue::Int(0));
        }
        let rv = eval(right, env)?;
        return Some(TclValue::Int(i64::from(rv.is_truthy())));
    }
    if matches!(op, BinOp::Or | BinOp::WordOr) {
        let lv = eval(left, env)?;
        if lv.is_truthy() {
            return Some(TclValue::Int(1));
        }
        let rv = eval(right, env)?;
        return Some(TclValue::Int(i64::from(rv.is_truthy())));
    }

    // iRules string operators and list-membership are deferred —
    // callers fall through to runtime emission.
    if matches!(
        op,
        BinOp::Contains
            | BinOp::StartsWith
            | BinOp::EndsWith
            | BinOp::StrEquals
            | BinOp::MatchesGlob
            | BinOp::MatchesRegex
            | BinOp::In
            | BinOp::Ni
    ) {
        return None;
    }

    let lv = eval(left, env)?;
    let rv = eval(right, env)?;
    apply_binary(op, lv, rv)
}

#[allow(clippy::too_many_lines)]
fn apply_binary(op: BinOp, a: TclValue, b: TclValue) -> Option<TclValue> {
    match op {
        // Arithmetic.
        BinOp::Add => Some(arith(a, b, i64::checked_add, |x, y| x + y)?),
        BinOp::Sub => Some(arith(a, b, i64::checked_sub, |x, y| x - y)?),
        BinOp::Mul => Some(arith(a, b, i64::checked_mul, |x, y| x * y)?),
        BinOp::Div => tcl_div(a, b),
        BinOp::Mod => tcl_mod(a, b),
        BinOp::Pow => tcl_pow(a, b),

        // Shifts and bitwise — integer only.
        BinOp::LShift => match (a, b) {
            (TclValue::Int(x), TclValue::Int(y)) if (0..=64).contains(&y) => {
                x.checked_shl(y as u32).map(TclValue::Int)
            }
            _ => None,
        },
        BinOp::RShift => match (a, b) {
            (TclValue::Int(x), TclValue::Int(y)) if y >= 0 => {
                if y > 64 {
                    Some(TclValue::Int(if x >= 0 { 0 } else { -1 }))
                } else {
                    Some(TclValue::Int(x >> y))
                }
            }
            _ => None,
        },
        BinOp::BitAnd => match (a, b) {
            (TclValue::Int(x), TclValue::Int(y)) => Some(TclValue::Int(x & y)),
            _ => None,
        },
        BinOp::BitOr => match (a, b) {
            (TclValue::Int(x), TclValue::Int(y)) => Some(TclValue::Int(x | y)),
            _ => None,
        },
        BinOp::BitXor => match (a, b) {
            (TclValue::Int(x), TclValue::Int(y)) => Some(TclValue::Int(x ^ y)),
            _ => None,
        },

        // Numeric comparison — always returns Int(0) or Int(1).
        BinOp::Eq => Some(TclValue::Int(i64::from(numeric_eq(a, b)))),
        BinOp::Ne => Some(TclValue::Int(i64::from(!numeric_eq(a, b)))),
        BinOp::Lt => Some(TclValue::Int(i64::from(numeric_cmp(a, b) == std::cmp::Ordering::Less))),
        BinOp::Le => Some(TclValue::Int(i64::from(numeric_cmp(a, b) != std::cmp::Ordering::Greater))),
        BinOp::Gt => Some(TclValue::Int(i64::from(numeric_cmp(a, b) == std::cmp::Ordering::Greater))),
        BinOp::Ge => Some(TclValue::Int(i64::from(numeric_cmp(a, b) != std::cmp::Ordering::Less))),

        // String comparison — render both sides via format_tcl_value
        // and compare lexicographically.
        BinOp::StrEq => Some(TclValue::Int(i64::from(format_tcl_value(a) == format_tcl_value(b)))),
        BinOp::StrNe => Some(TclValue::Int(i64::from(format_tcl_value(a) != format_tcl_value(b)))),
        BinOp::StrLt => Some(TclValue::Int(i64::from(format_tcl_value(a) < format_tcl_value(b)))),
        BinOp::StrLe => Some(TclValue::Int(i64::from(format_tcl_value(a) <= format_tcl_value(b)))),
        BinOp::StrGt => Some(TclValue::Int(i64::from(format_tcl_value(a) > format_tcl_value(b)))),
        BinOp::StrGe => Some(TclValue::Int(i64::from(format_tcl_value(a) >= format_tcl_value(b)))),

        // Short-circuit and iRules string ops are handled in eval_binary.
        BinOp::And
        | BinOp::Or
        | BinOp::WordAnd
        | BinOp::WordOr
        | BinOp::Contains
        | BinOp::StartsWith
        | BinOp::EndsWith
        | BinOp::StrEquals
        | BinOp::MatchesGlob
        | BinOp::MatchesRegex
        | BinOp::In
        | BinOp::Ni => None,
    }
}

fn arith<F, G>(a: TclValue, b: TclValue, int_op: F, float_op: G) -> Option<TclValue>
where
    F: FnOnce(i64, i64) -> Option<i64>,
    G: FnOnce(f64, f64) -> f64,
{
    match (a, b) {
        (TclValue::Int(x), TclValue::Int(y)) => int_op(x, y).map(TclValue::Int),
        _ => Some(TclValue::Float(float_op(a.as_f64(), b.as_f64()))),
    }
}

fn numeric_eq(a: TclValue, b: TclValue) -> bool {
    match (a, b) {
        (TclValue::Int(x), TclValue::Int(y)) => x == y,
        _ => (a.as_f64() - b.as_f64()).abs() == 0.0,
    }
}

fn numeric_cmp(a: TclValue, b: TclValue) -> std::cmp::Ordering {
    match (a, b) {
        (TclValue::Int(x), TclValue::Int(y)) => x.cmp(&y),
        _ => a
            .as_f64()
            .partial_cmp(&b.as_f64())
            .unwrap_or(std::cmp::Ordering::Equal),
    }
}

fn tcl_div(a: TclValue, b: TclValue) -> Option<TclValue> {
    match (a, b) {
        (TclValue::Int(_), TclValue::Int(0)) => None,
        (TclValue::Int(x), TclValue::Int(y)) => {
            // Floor division: r and y must have the same sign,
            // otherwise subtract 1 from the truncated quotient.
            let q = x.checked_div(y)?;
            let r = x.checked_rem(y)?;
            if r != 0 && (r.signum() != y.signum()) {
                Some(TclValue::Int(q.checked_sub(1)?))
            } else {
                Some(TclValue::Int(q))
            }
        }
        _ => {
            let fa = a.as_f64();
            let fb = b.as_f64();
            if fb == 0.0 {
                return None;
            }
            Some(TclValue::Float(fa / fb))
        }
    }
}

fn tcl_mod(a: TclValue, b: TclValue) -> Option<TclValue> {
    match (a, b) {
        (TclValue::Int(_), TclValue::Int(0)) => None,
        (TclValue::Int(x), TclValue::Int(y)) => {
            // Sign follows divisor.
            let r = x.checked_rem(y)?;
            if r != 0 && (r.signum() != y.signum()) {
                Some(TclValue::Int(r.checked_add(y)?))
            } else {
                Some(TclValue::Int(r))
            }
        }
        _ => None, // Tcl 9.0 rejects floats for `%`.
    }
}

fn tcl_pow(a: TclValue, b: TclValue) -> Option<TclValue> {
    if matches!(a, TclValue::Float(_)) || matches!(b, TclValue::Float(_)) {
        let fa = a.as_f64();
        let fb = b.as_f64();
        if fa == 0.0 && fb < 0.0 {
            return None;
        }
        if fa < 0.0 && (!fb.is_finite() || fb.fract() != 0.0) {
            return None;
        }
        let r = fa.powf(fb);
        if r.is_nan() {
            return None;
        }
        return Some(TclValue::Float(r));
    }
    // Integer path.
    let (TclValue::Int(x), TclValue::Int(y)) = (a, b) else {
        return None;
    };
    if y == 0 {
        return Some(TclValue::Int(1));
    }
    if y == 1 {
        return Some(TclValue::Int(x));
    }
    if x == 0 {
        return if y < 0 { None } else { Some(TclValue::Int(0)) };
    }
    if x == 1 {
        return Some(TclValue::Int(1));
    }
    if x == -1 {
        return Some(TclValue::Int(if y % 2 == 0 { 1 } else { -1 }));
    }
    if y < 0 {
        return Some(TclValue::Int(0));
    }
    if y > MAX_EXPONENT {
        return None;
    }
    // Overflow-checked integer power.
    let mut base = x;
    let mut exp = y;
    let mut acc: i64 = 1;
    while exp > 0 {
        if exp & 1 == 1 {
            acc = acc.checked_mul(base)?;
        }
        exp >>= 1;
        if exp > 0 {
            base = base.checked_mul(base)?;
        }
    }
    Some(TclValue::Int(acc))
}

// ---------------------------------------------------------------------------
// Unary operators
// ---------------------------------------------------------------------------

fn eval_unary(op: UnaryOp, operand: &ExprNode, env: &Env) -> Option<TclValue> {
    let v = eval(operand, env)?;
    match op {
        UnaryOp::Neg => match v {
            TclValue::Int(i) => i.checked_neg().map(TclValue::Int),
            TclValue::Float(f) => Some(TclValue::Float(-f)),
        },
        UnaryOp::Pos => Some(v),
        UnaryOp::Not | UnaryOp::WordNot => Some(TclValue::Int(i64::from(!v.is_truthy()))),
        UnaryOp::BitNot => match v {
            TclValue::Int(i) => Some(TclValue::Int(!i)),
            TclValue::Float(_) => None,
        },
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr_parser::parse_expr;

    fn eval_str(expr: &str) -> Option<TclValue> {
        let env = Env::new();
        eval_tcl_expr(&parse_expr(expr, None), &env)
    }

    fn eval_str_env(expr: &str, env: &Env) -> Option<TclValue> {
        eval_tcl_expr(&parse_expr(expr, None), env)
    }

    #[test]
    fn literal_int() {
        assert_eq!(eval_str("42"), Some(TclValue::Int(42)));
        assert_eq!(eval_str("0"), Some(TclValue::Int(0)));
        assert_eq!(eval_str("0x1a"), Some(TclValue::Int(26)));
        assert_eq!(eval_str("0b101"), Some(TclValue::Int(5)));
        assert_eq!(eval_str("0o17"), Some(TclValue::Int(15)));
    }

    #[test]
    fn literal_float() {
        assert_eq!(eval_str("1.5"), Some(TclValue::Float(1.5)));
        assert_eq!(eval_str("2e3"), Some(TclValue::Float(2000.0)));
    }

    #[test]
    fn literal_bool() {
        assert_eq!(parse_literal("true"), Some(TclValue::Int(1)));
        assert_eq!(parse_literal("yes"), Some(TclValue::Int(1)));
        assert_eq!(parse_literal("false"), Some(TclValue::Int(0)));
        assert_eq!(parse_literal("off"), Some(TclValue::Int(0)));
    }

    #[test]
    fn arithmetic_int() {
        assert_eq!(eval_str("1 + 2"), Some(TclValue::Int(3)));
        assert_eq!(eval_str("5 - 3"), Some(TclValue::Int(2)));
        assert_eq!(eval_str("4 * 5"), Some(TclValue::Int(20)));
        assert_eq!(eval_str("10 / 3"), Some(TclValue::Int(3)));
        assert_eq!(eval_str("10 % 3"), Some(TclValue::Int(1)));
    }

    #[test]
    fn arithmetic_int_floor_division_negative() {
        // Tcl / Python: floor toward -inf.
        assert_eq!(eval_str("-7 / 2"), Some(TclValue::Int(-4)));
        assert_eq!(eval_str("7 / -2"), Some(TclValue::Int(-4)));
        // Modulo sign follows divisor.
        assert_eq!(eval_str("-7 % 2"), Some(TclValue::Int(1)));
        assert_eq!(eval_str("7 % -2"), Some(TclValue::Int(-1)));
    }

    #[test]
    fn arithmetic_float_promotion() {
        assert_eq!(eval_str("1 + 2.5"), Some(TclValue::Float(3.5)));
        assert_eq!(eval_str("10.0 / 4"), Some(TclValue::Float(2.5)));
    }

    #[test]
    fn division_by_zero() {
        assert_eq!(eval_str("1 / 0"), None);
        assert_eq!(eval_str("1 % 0"), None);
        assert_eq!(eval_str("1.0 / 0.0"), None);
    }

    #[test]
    fn pow_integer() {
        assert_eq!(eval_str("2 ** 10"), Some(TclValue::Int(1024)));
        assert_eq!(eval_str("2 ** 0"), Some(TclValue::Int(1)));
        assert_eq!(eval_str("0 ** 5"), Some(TclValue::Int(0)));
        assert_eq!(eval_str("(-1) ** 3"), Some(TclValue::Int(-1)));
        assert_eq!(eval_str("(-1) ** 4"), Some(TclValue::Int(1)));
        // |base| > 1 with negative exp → 0 (Tcl integer rules).
        assert_eq!(eval_str("2 ** -5"), Some(TclValue::Int(0)));
        // 0 ** negative → error.
        assert_eq!(eval_str("0 ** -1"), None);
    }

    #[test]
    fn comparisons_return_0_or_1() {
        assert_eq!(eval_str("1 < 2"), Some(TclValue::Int(1)));
        assert_eq!(eval_str("2 < 1"), Some(TclValue::Int(0)));
        assert_eq!(eval_str("3 == 3"), Some(TclValue::Int(1)));
        assert_eq!(eval_str("3 != 3"), Some(TclValue::Int(0)));
    }

    #[test]
    fn string_comparisons() {
        assert_eq!(eval_str("1 eq 1"), Some(TclValue::Int(1)));
        assert_eq!(eval_str("1 ne 2"), Some(TclValue::Int(1)));
    }

    #[test]
    fn short_circuit_and() {
        // Second operand is an unbound variable — short-circuit must
        // avoid evaluating it when the first operand is falsy.
        let env = Env::new();
        assert_eq!(eval_str_env("0 && $undef", &env), Some(TclValue::Int(0)));
    }

    #[test]
    fn short_circuit_or() {
        let env = Env::new();
        assert_eq!(eval_str_env("1 || $undef", &env), Some(TclValue::Int(1)));
    }

    #[test]
    fn ternary_selects_correct_branch() {
        assert_eq!(eval_str("1 ? 10 : 20"), Some(TclValue::Int(10)));
        assert_eq!(eval_str("0 ? 10 : 20"), Some(TclValue::Int(20)));
    }

    #[test]
    fn unary_operators() {
        assert_eq!(eval_str("-5"), Some(TclValue::Int(-5)));
        assert_eq!(eval_str("+5"), Some(TclValue::Int(5)));
        assert_eq!(eval_str("!0"), Some(TclValue::Int(1)));
        assert_eq!(eval_str("!1"), Some(TclValue::Int(0)));
        assert_eq!(eval_str("~0"), Some(TclValue::Int(-1)));
    }

    #[test]
    fn bitwise_operators() {
        assert_eq!(eval_str("0xff & 0x0f"), Some(TclValue::Int(0x0f)));
        assert_eq!(eval_str("0xf0 | 0x0f"), Some(TclValue::Int(0xff)));
        assert_eq!(eval_str("0xff ^ 0x0f"), Some(TclValue::Int(0xf0)));
    }

    #[test]
    fn shifts() {
        assert_eq!(eval_str("1 << 4"), Some(TclValue::Int(16)));
        assert_eq!(eval_str("16 >> 2"), Some(TclValue::Int(4)));
        // Negative shift count is undefined.
        assert_eq!(eval_str("1 << -1"), None);
    }

    #[test]
    fn variable_resolution_from_env() {
        let mut env = Env::new();
        env.insert("x".into(), EnvValue::Int(42));
        assert_eq!(eval_str_env("$x + 8", &env), Some(TclValue::Int(50)));
    }

    #[test]
    fn unbound_variable_is_none() {
        let env = Env::new();
        assert_eq!(eval_str_env("$undef + 1", &env), None);
    }

    #[test]
    fn command_substitution_is_none() {
        // Raw text `[foo]` is parsed as an ExprCommand, which is opaque.
        assert_eq!(eval_str("[clock seconds] + 1"), None);
    }

    #[test]
    fn format_tcl_value_int_and_float() {
        assert_eq!(format_tcl_value(TclValue::Int(42)), "42");
        assert_eq!(format_tcl_value(TclValue::Int(-7)), "-7");
        assert_eq!(format_tcl_value(TclValue::Float(1.5)), "1.5");
        // Integer-valued floats render with trailing .0.
        assert_eq!(format_tcl_value(TclValue::Float(3.0)), "3.0");
    }

    #[test]
    fn overflow_returns_none() {
        // 10 ** 100 overflows i64.
        assert_eq!(eval_str("10 ** 100"), None);
    }

    #[test]
    fn irules_string_ops_return_none() {
        // Deferred — callers fall through to runtime.
        assert_eq!(eval_str("\"abc\" contains \"b\""), None);
    }

    // -- Math function dispatch --

    #[test]
    fn math_abs_int_and_float() {
        assert_eq!(eval_str("abs(-5)"), Some(TclValue::Int(5)));
        assert_eq!(eval_str("abs(-1.5)"), Some(TclValue::Float(1.5)));
    }

    #[test]
    fn math_int_conversion_truncates() {
        assert_eq!(eval_str("int(3.7)"), Some(TclValue::Int(3)));
        assert_eq!(eval_str("int(-3.7)"), Some(TclValue::Int(-3)));
        assert_eq!(eval_str("entier(2.9)"), Some(TclValue::Int(2)));
        assert_eq!(eval_str("wide(1)"), Some(TclValue::Int(1)));
    }

    #[test]
    fn math_double_promotes_ints() {
        assert_eq!(eval_str("double(3)"), Some(TclValue::Float(3.0)));
    }

    #[test]
    fn math_bool_normalises_to_01() {
        assert_eq!(eval_str("bool(42)"), Some(TclValue::Int(1)));
        assert_eq!(eval_str("bool(0)"), Some(TclValue::Int(0)));
        assert_eq!(eval_str("bool(0.0)"), Some(TclValue::Int(0)));
    }

    #[test]
    fn math_round_ties_away_from_zero() {
        // Tcl round: 0.5 → 1, -0.5 → -1 (NOT banker's rounding).
        assert_eq!(eval_str("round(0.5)"), Some(TclValue::Int(1)));
        assert_eq!(eval_str("round(-0.5)"), Some(TclValue::Int(-1)));
        assert_eq!(eval_str("round(1.5)"), Some(TclValue::Int(2)));
        assert_eq!(eval_str("round(-1.5)"), Some(TclValue::Int(-2)));
        assert_eq!(eval_str("round(2.5)"), Some(TclValue::Int(3)));
    }

    #[test]
    fn math_ceil_and_floor_return_floats() {
        assert_eq!(eval_str("ceil(1.2)"), Some(TclValue::Float(2.0)));
        assert_eq!(eval_str("floor(1.8)"), Some(TclValue::Float(1.0)));
    }

    #[test]
    fn math_min_max_preserve_int_width() {
        assert_eq!(eval_str("min(3, 1, 2)"), Some(TclValue::Int(1)));
        assert_eq!(eval_str("max(3, 1, 2)"), Some(TclValue::Int(3)));
        // Mixed int/float → float result.
        assert_eq!(eval_str("min(1, 2.5)"), Some(TclValue::Float(1.0)));
    }

    #[test]
    fn math_sqrt_and_pow() {
        assert_eq!(eval_str("sqrt(16)"), Some(TclValue::Float(4.0)));
        assert_eq!(eval_str("pow(2, 10)"), Some(TclValue::Float(1024.0)));
    }

    #[test]
    fn math_sqrt_negative_is_domain_error() {
        assert_eq!(eval_str("sqrt(-1)"), None);
    }

    #[test]
    fn math_log_zero_and_negative_domain_error() {
        assert_eq!(eval_str("log(-1)"), None);
        // log(0) → -inf, treated as success (Tcl returns -inf too).
        let v = eval_str("log(0)");
        assert!(matches!(v, Some(TclValue::Float(f)) if f.is_infinite()));
    }

    #[test]
    fn math_atan2_and_hypot() {
        // atan2(0, 1) = 0, hypot(3, 4) = 5
        assert_eq!(eval_str("atan2(0, 1)"), Some(TclValue::Float(0.0)));
        assert_eq!(eval_str("hypot(3, 4)"), Some(TclValue::Float(5.0)));
    }

    #[test]
    fn math_trig_approx() {
        // sin(0) == 0.
        assert!(matches!(
            eval_str("sin(0)"),
            Some(TclValue::Float(f)) if f == 0.0
        ));
    }

    #[test]
    fn math_classification() {
        assert_eq!(eval_str("isinf(1)"), Some(TclValue::Int(0)));
        assert_eq!(eval_str("isnan(1.0)"), Some(TclValue::Int(0)));
        assert_eq!(eval_str("isfinite(1.0)"), Some(TclValue::Int(1)));
    }

    #[test]
    fn math_isqrt_integer_only() {
        assert_eq!(eval_str("isqrt(16)"), Some(TclValue::Int(4)));
        assert_eq!(eval_str("isqrt(17)"), Some(TclValue::Int(4)));
        // Float arg → None (mirrors Python behaviour).
        assert_eq!(eval_str("isqrt(4.0)"), None);
    }

    #[test]
    fn math_rand_and_srand_always_none() {
        // Non-deterministic — callers must not constant-fold.
        assert_eq!(eval_str("rand()"), None);
        assert_eq!(eval_str("srand(42)"), None);
    }

    #[test]
    fn math_unknown_function_is_none() {
        assert_eq!(eval_str("thereisnosuchfn(1)"), None);
    }
}
