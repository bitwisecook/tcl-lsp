//! Tcl expression evaluator (compile-time constant folding).
//!
//! The **tree-walk is shared** with the runtime: [`eval_tcl_expr`] drives the
//! one [`tcl_syntax::expr::eval()`] over the AST and supplies this const-folder's
//! value ops via [`FoldOps`] (an `ExprOps` impl) — the same way the lexer/parser
//! are shared. Only the value-type-specific bits (the `i64`/`f64`/`Str`
//! [`FoldValue`], the operator helpers below, env-var resolution) live here.
//! `None` means "can't fold" — a variable not in the environment, a command
//! substitution, a domain error, or a value past a wide — and callers fall
//! through to the runtime form.
//!
//! Semantics follow C Tcl 9.0.2 (`tclExecute.c`, `tclBasic.c`):
//!
//! - Integer division floors toward negative infinity.
//! - Integer modulo: sign follows divisor.
//! - Exponentiation: special rules for `|base| ≤ 1` and negative
//!   exponents.
//! - Comparisons always return `Int(0)` or `Int(1)`; `eq`/`ne`/`lt`… compare the
//!   operands' raw text (so `5.00 eq 5.0` → 0).
//! - `round()` ties away from zero (not Python/Rust banker's rounding).
//!
//! Originally ported from `core/compiler/tcl_expr_eval.py` (C22); the iRules
//! word operators (`contains`/`starts_with`/`matches_glob`/`matches_regex`/
//! `equals`/`in`/`ni`) fold via [`tcl_syntax::expr::ExprOps::binary_other`].

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
///
/// Mirrors C Tcl's `INST_EXPON` limit (`exponent < 2^28`).  Set to
/// `(1 << 28) - 1` by upstream commit ``342d4c7a`` (PR #331);
/// previously a tighter `10_000` cap that diverged from C Tcl.
const MAX_EXPONENT: i64 = (1 << 28) - 1;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Evaluate an expression AST against `env`. Returns `None` when the
/// expression depends on runtime state or triggers a domain error.
///
/// The tree-walk is the **shared** [`tcl_syntax::expr::eval()`] (the same one the
/// runtime evaluates with); this const-folder supplies only its value ops via
/// [`FoldOps`]. A `None` result means "can't fold".
#[must_use]
pub fn eval_tcl_expr(node: &ExprNode, env: &Env) -> Option<TclValue> {
    let mut ops = FoldOps { env };
    // The final value must reduce to a number (a bare string like `expr {"x"}`
    // doesn't fold) — `to_number` maps a `Str` result through `parse_literal`.
    tcl_syntax::expr::eval(node, &mut ops).ok()?.to_number()
}

// ---------------------------------------------------------------------------
// FoldOps — the const-folder's value ops for the shared expr walk
// ---------------------------------------------------------------------------

/// A const-fold value. `Str` keeps the operand's **raw text** and is parsed
/// lazily per context (numeric ops via [`parse_literal`]; string ops use it
/// verbatim) — exactly the old `eval`-vs-`eval_as_string` split, so the raw-text
/// string-compare behaviour (`5.00 eq 5.0` → 0, #519) is preserved.
#[derive(Clone)]
enum FoldValue {
    Int(i64),
    Float(f64),
    Str(String),
}

impl FoldValue {
    /// Interpret as a number, or `None` when the text isn't numeric.
    fn to_number(&self) -> Option<TclValue> {
        match self {
            FoldValue::Int(i) => Some(TclValue::Int(*i)),
            FoldValue::Float(f) => Some(TclValue::Float(*f)),
            FoldValue::Str(s) => parse_literal(s),
        }
    }
    /// Render as a string: raw for `Str`, canonical for numbers.
    fn to_string_val(&self) -> String {
        match self {
            FoldValue::Str(s) => s.clone(),
            FoldValue::Int(i) => format_tcl_value(TclValue::Int(*i)),
            FoldValue::Float(f) => format_tcl_value(TclValue::Float(*f)),
        }
    }
    fn from_tcl(v: TclValue) -> FoldValue {
        match v {
            TclValue::Int(i) => FoldValue::Int(i),
            TclValue::Float(f) => FoldValue::Float(f),
        }
    }
}

/// The const-folder's [`ExprOps`](tcl_syntax::expr::ExprOps). `Error = ()` is the
/// "can't fold" signal (mapped to the public `Option`); `$var` resolves from the
/// `env`, `[cmd]`/`Raw` are opaque.
struct FoldOps<'a> {
    env: &'a Env,
}

impl tcl_syntax::expr::ExprOps for FoldOps<'_> {
    type Value = FoldValue;
    type Error = ();

    fn literal(&mut self, text: &str) -> Result<FoldValue, ()> {
        Ok(FoldValue::Str(text.to_owned()))
    }
    fn string(&mut self, inner: &str) -> Result<FoldValue, ()> {
        Ok(FoldValue::Str(inner.to_owned()))
    }
    fn var(&mut self, name: &str) -> Result<FoldValue, ()> {
        match self.env.get(name) {
            Some(EnvValue::Int(i)) => Ok(FoldValue::Int(*i)),
            Some(EnvValue::Float(f)) => Ok(FoldValue::Float(*f)),
            Some(EnvValue::Str(s)) => Ok(FoldValue::Str(s.clone())),
            None => Err(()), // unbound → can't fold
        }
    }
    fn command(&mut self, _script: &str) -> Result<FoldValue, ()> {
        Err(()) // command substitution is opaque at compile time
    }
    fn call(&mut self, function: &str, args: Vec<FoldValue>) -> Result<FoldValue, ()> {
        use tcl_syntax::expr::mathfunc::{dispatch, Num};
        let name = function.to_ascii_lowercase();
        if matches!(name.as_str(), "rand" | "srand") {
            return Err(()); // non-deterministic
        }
        // Math functions are the shared `tcl_syntax::expr::mathfunc` (the same
        // dispatch the runtime evaluates). Map `TclValue` → `Num` → result.
        let nums: Option<Vec<Num>> = args
            .iter()
            .map(|v| {
                v.to_number().map(|t| match t {
                    TclValue::Int(i) => Num::Int(i),
                    TclValue::Float(f) => Num::Float(f),
                })
            })
            .collect();
        match dispatch(&name, &nums.ok_or(())?).ok_or(())? {
            Num::Int(i) => Ok(FoldValue::Int(i)),
            Num::Float(f) => Ok(FoldValue::Float(f)),
        }
    }

    fn arith(&mut self, op: BinOp, left: FoldValue, right: FoldValue) -> Result<FoldValue, ()> {
        let a = left.to_number().ok_or(())?;
        let b = right.to_number().ok_or(())?;
        apply_binary(op, a, b).map(FoldValue::from_tcl).ok_or(())
    }
    fn unary(&mut self, op: UnaryOp, value: FoldValue) -> Result<FoldValue, ()> {
        match op {
            UnaryOp::Not | UnaryOp::WordNot => {
                let truthy = value.to_number().ok_or(())?.is_truthy();
                Ok(FoldValue::Int(i64::from(!truthy)))
            }
            UnaryOp::Pos => value.to_number().map(FoldValue::from_tcl).ok_or(()),
            UnaryOp::Neg => match value.to_number().ok_or(())? {
                TclValue::Int(i) => i.checked_neg().map(FoldValue::Int).ok_or(()),
                TclValue::Float(f) => Ok(FoldValue::Float(-f)),
            },
            UnaryOp::BitNot => match value.to_number().ok_or(())? {
                TclValue::Int(i) => Ok(FoldValue::Int(!i)),
                TclValue::Float(_) => Err(()),
            },
        }
    }

    fn compare_numeric(
        &mut self,
        left: &FoldValue,
        right: &FoldValue,
    ) -> Option<std::cmp::Ordering> {
        Some(numeric_cmp(left.to_number()?, right.to_number()?))
    }
    fn compare_string(&mut self, left: &FoldValue, right: &FoldValue) -> std::cmp::Ordering {
        left.to_string_val().cmp(&right.to_string_val())
    }
    fn in_list(&mut self, needle: &FoldValue, list: &FoldValue) -> Result<bool, ()> {
        let n = needle.to_string_val();
        Ok(split_tcl_list(&list.to_string_val()).contains(&n))
    }

    fn to_bool(&mut self, value: &FoldValue) -> Result<bool, ()> {
        Ok(value.to_number().ok_or(())?.is_truthy())
    }
    fn bool_value(&mut self, b: bool) -> FoldValue {
        FoldValue::Int(i64::from(b))
    }
    fn unsupported(&mut self, _what: &str) {}

    /// The iRules dialect string operators (`contains`/`starts_with`/`equals`/
    /// `matches_glob`/`matches_regex`/…) — apply to the operands as strings.
    fn binary_other(
        &mut self,
        op: BinOp,
        left: FoldValue,
        right: FoldValue,
    ) -> Result<FoldValue, ()> {
        apply_irules_string_op(op, &left.to_string_val(), &right.to_string_val())
            .map(FoldValue::from_tcl)
            .ok_or(())
    }
}

/// Render a `TclValue` as a Tcl source literal. Matches Tcl's
/// `Tcl_GetStringFromObj` output for numbers.
#[must_use]
pub fn format_tcl_value(v: TclValue) -> String {
    match v {
        TclValue::Int(i) => i.to_string(),
        // The shared canonical double formatter (also the runtime's `double`
        // string rep): integer-valued floats get `.0`, plus `Inf`/`NaN`.
        TclValue::Float(f) => tcl_syntax::number::format_double(f),
    }
}

// ---------------------------------------------------------------------------
// Core dispatch
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Math function calls
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Literals and variables
// ---------------------------------------------------------------------------

/// Parse a numeric/boolean literal. Supports `0x`/`0o`/`0b` prefixes,
/// Tcl-style leading-zero decimals (e.g. `0005`), floats, and the
/// Tcl boolean spellings.
#[must_use]
pub fn parse_literal(text: &str) -> Option<TclValue> {
    use tcl_syntax::number::Number;
    // Boolean keywords (`Tcl_GetBoolean`, not part of the number grammar).
    match text.to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" => return Some(TclValue::Int(1)),
        "false" | "no" | "off" => return Some(TclValue::Int(0)),
        _ => {}
    }
    // The numeric grammar is the shared `tcl_syntax::number` (the same
    // `TclParseNumber` port the runtime const-folds with): `0x`/`0o`/`0b`,
    // leading-zero decimal, `_` separators, `Inf`/`NaN`. The const-folder's
    // value is `i64`/`f64`, so a magnitude past a wide (a `Big`) can't be
    // folded — return `None` (give up), matching the old i64-overflow behaviour.
    match tcl_syntax::number::parse_whole(text)? {
        Number::Int(v) => Some(TclValue::Int(v)),
        Number::Double(d) => Some(TclValue::Float(d)),
        Number::Nan { .. } => Some(TclValue::Float(f64::NAN)),
        Number::Big { .. } => None,
    }
}

// ---------------------------------------------------------------------------
// Binary operators
// ---------------------------------------------------------------------------

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
        BinOp::Lt => Some(TclValue::Int(i64::from(
            numeric_cmp(a, b) == std::cmp::Ordering::Less,
        ))),
        BinOp::Le => Some(TclValue::Int(i64::from(
            numeric_cmp(a, b) != std::cmp::Ordering::Greater,
        ))),
        BinOp::Gt => Some(TclValue::Int(i64::from(
            numeric_cmp(a, b) == std::cmp::Ordering::Greater,
        ))),
        BinOp::Ge => Some(TclValue::Int(i64::from(
            numeric_cmp(a, b) != std::cmp::Ordering::Less,
        ))),

        // String-comparison ops (eq/ne/lt/le/gt/ge) are routed through
        // `apply_string_compare` from `eval_binary` before they reach here
        // (they need string, not numeric, operands), as are the
        // short-circuit and iRules string ops.
        BinOp::StrEq
        | BinOp::StrNe
        | BinOp::StrLt
        | BinOp::StrLe
        | BinOp::StrGt
        | BinOp::StrGe
        | BinOp::And
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

// ---------------------------------------------------------------------------
// iRules string ops (C22i1/i2)
// ---------------------------------------------------------------------------

/// Split a simple Tcl list string into elements.
///
/// Handles space-separated words and brace-grouped elements.
/// Does not handle the full Tcl list-quoting grammar (backslash
/// continuations, nested braces within quoted strings) but covers
/// the constant inputs seen by `in` / `ni` at compile time.
pub(crate) fn split_tcl_list(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r') {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        if bytes[i] == b'{' {
            let mut level = 1i32;
            i += 1;
            let start = i;
            while i < bytes.len() && level > 0 {
                match bytes[i] {
                    b'{' => level += 1,
                    b'}' => level -= 1,
                    _ => {}
                }
                i += 1;
            }
            out.push(text[start..i - 1].to_owned());
        } else if bytes[i] == b'"' {
            i += 1;
            let start = i;
            while i < bytes.len() && bytes[i] != b'"' {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    i += 2;
                    continue;
                }
                i += 1;
            }
            out.push(text[start..i].to_owned());
            if i < bytes.len() {
                i += 1;
            }
        } else {
            let start = i;
            while i < bytes.len() && !matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r') {
                i += 1;
            }
            out.push(text[start..i].to_owned());
        }
    }
    out
}

/// Apply an iRules string operator to two rendered string operands.
fn apply_irules_string_op(op: BinOp, left: &str, right: &str) -> Option<TclValue> {
    let res = match op {
        BinOp::Contains => left.contains(right),
        BinOp::StartsWith => left.starts_with(right),
        BinOp::EndsWith => left.ends_with(right),
        BinOp::StrEquals => left == right,
        BinOp::MatchesGlob => tcl_syntax::glob::string_match(right, left),
        BinOp::In => split_tcl_list(right).iter().any(|e| e == left),
        BinOp::Ni => !split_tcl_list(right).iter().any(|e| e == left),
        // `matches_regex` is deliberately *not* constant-folded (along
        // with any other / unsupported operator): the Rust `regex` crate
        // is not Tcl's ARE engine — classes, anchors, word boundaries,
        // embedded options and greediness all differ — so folding here
        // could disagree with runtime.  Decline to evaluate and defer to
        // the runtime regex engine.
        _ => return None,
    };
    Some(TclValue::Int(i64::from(res)))
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

    /// Parse + evaluate using the iRules dialect, which enables
    /// `contains`/`starts_with`/`ends_with`/`equals`/`matches_glob`/
    /// `matches_regex`/`in`/`ni` word operators.
    fn eval_irules(expr: &str) -> Option<TclValue> {
        let env = Env::new();
        eval_tcl_expr(&parse_expr(expr, Some("f5-irules")), &env)
    }

    fn eval_irules_env(expr: &str, env: &Env) -> Option<TclValue> {
        eval_tcl_expr(&parse_expr(expr, Some("f5-irules")), env)
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
    fn string_comparisons_fold_string_operands() {
        // SYNC-JUN02b-5 (#519): `eq`/`ne`/`lt`/`gt`/`le`/`ge` compare
        // operands AS strings, so string-only operands now fold
        // (previously `eval` parsed them numerically, returning None).
        assert_eq!(eval_str("\"x\" eq \"y\""), Some(TclValue::Int(0)));
        assert_eq!(eval_str("\"x\" ne \"y\""), Some(TclValue::Int(1)));
        assert_eq!(eval_str("\"x\" eq \"x\""), Some(TclValue::Int(1)));
        // Lexical ordering on string operands.
        assert_eq!(eval_str("\"abc\" lt \"abd\""), Some(TclValue::Int(1)));
        assert_eq!(eval_str("\"b\" gt \"a\""), Some(TclValue::Int(1)));
        // `5 eq 5.0` compares the strings "5" vs "5.0" (→ 0), matching
        // C Tcl 9 — a regression guard for the numeric-looking case.
        assert_eq!(eval_str("5 eq 5.0"), Some(TclValue::Int(0)));
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

    // -- C22i3: matches_regex is never constant-folded --

    #[test]
    fn irules_matches_regex_is_not_folded() {
        // `matches_regex` is deferred to the runtime ARE engine rather
        // than folded via the Rust `regex` crate (whose syntax/semantics
        // differ from Tcl ARE), so the constant evaluator returns None
        // for every pattern — even ones the Rust engine could match.
        for expr in [
            r#""hello world" matches_regex "world""#,
            r#""hello" matches_regex "^bye""#,
            r#""abc123" matches_regex "^[a-z]+[0-9]+$""#,
            r#""apple" matches_regex "apple|orange""#,
            r#""prefix-world-suffix" matches_regex "world""#,
            r#""abc" matches_regex "(?=a)abc""#,
            r#""abc" matches_regex "[unterminated""#,
        ] {
            assert_eq!(eval_irules(expr), None, "{expr}");
        }
    }

    // -- C22i1: simple iRules string ops --

    #[test]
    fn irules_contains() {
        assert_eq!(
            eval_irules(r#""hello world" contains "world""#),
            Some(TclValue::Int(1))
        );
        assert_eq!(
            eval_irules(r#""hello" contains "bye""#),
            Some(TclValue::Int(0))
        );
    }

    #[test]
    fn irules_starts_with() {
        assert_eq!(
            eval_irules(r#""foobar" starts_with "foo""#),
            Some(TclValue::Int(1))
        );
        assert_eq!(
            eval_irules(r#""foobar" starts_with "bar""#),
            Some(TclValue::Int(0))
        );
    }

    #[test]
    fn irules_ends_with() {
        assert_eq!(
            eval_irules(r#""foobar" ends_with "bar""#),
            Some(TclValue::Int(1))
        );
        assert_eq!(
            eval_irules(r#""foobar" ends_with "foo""#),
            Some(TclValue::Int(0))
        );
    }

    #[test]
    fn irules_str_equals() {
        assert_eq!(eval_irules(r#""abc" equals "abc""#), Some(TclValue::Int(1)));
        assert_eq!(eval_irules(r#""abc" equals "xyz""#), Some(TclValue::Int(0)));
    }

    #[test]
    fn irules_string_op_with_bound_variable() {
        let mut env = Env::new();
        env.insert("name".into(), EnvValue::Str("production".into()));
        assert_eq!(
            eval_irules_env(r#"$name contains "prod""#, &env),
            Some(TclValue::Int(1))
        );
    }

    // -- C22i2: matches_glob + in/ni --

    #[test]
    fn irules_matches_glob_star() {
        assert_eq!(
            eval_irules(r#""hello world" matches_glob "hello*""#),
            Some(TclValue::Int(1))
        );
        assert_eq!(
            eval_irules(r#""hello world" matches_glob "*world""#),
            Some(TclValue::Int(1))
        );
        assert_eq!(
            eval_irules(r#""hello world" matches_glob "*lo w*""#),
            Some(TclValue::Int(1))
        );
    }

    #[test]
    fn irules_matches_glob_question_and_class() {
        assert_eq!(
            eval_irules(r#""abc" matches_glob "a?c""#),
            Some(TclValue::Int(1))
        );
        assert_eq!(
            eval_irules(r#""abc" matches_glob "a[bxy]c""#),
            Some(TclValue::Int(1))
        );
        assert_eq!(
            eval_irules(r#""axc" matches_glob "a[bxy]c""#),
            Some(TclValue::Int(1))
        );
        assert_eq!(
            eval_irules(r#""azc" matches_glob "a[bxy]c""#),
            Some(TclValue::Int(0))
        );
    }

    #[test]
    fn irules_matches_glob_rejects_on_mismatch() {
        assert_eq!(
            eval_irules(r#""hello" matches_glob "world""#),
            Some(TclValue::Int(0))
        );
    }

    #[test]
    fn irules_in_list_membership() {
        assert_eq!(eval_irules(r#""b" in "a b c""#), Some(TclValue::Int(1)));
        assert_eq!(eval_irules(r#""d" in "a b c""#), Some(TclValue::Int(0)));
        // Braced element grouping.
        assert_eq!(
            eval_irules(r#""b c" in "{a b} {b c} d""#),
            Some(TclValue::Int(1))
        );
    }

    #[test]
    fn irules_ni_negated_membership() {
        assert_eq!(eval_irules(r#""d" ni "a b c""#), Some(TclValue::Int(1)));
        assert_eq!(eval_irules(r#""b" ni "a b c""#), Some(TclValue::Int(0)));
    }

    #[test]
    fn split_tcl_list_handles_braces_and_quotes() {
        assert_eq!(split_tcl_list("a b c"), vec!["a", "b", "c"]);
        assert_eq!(
            split_tcl_list("{hello world} foo"),
            vec!["hello world", "foo"]
        );
        assert_eq!(split_tcl_list(""), Vec::<String>::new());
    }

    #[test]
    fn matches_glob_folds_through_shared_string_match() {
        // `matches_glob` (iRules dialect) now folds through the shared
        // `tcl_syntax::glob` (one `string match` dialect); see that module for
        // the full grammar.
        assert_eq!(
            eval_irules(r#""abc" matches_glob "a*c""#),
            Some(TclValue::Int(1))
        );
        assert_eq!(
            eval_irules(r#""ab" matches_glob "a*c""#),
            Some(TclValue::Int(0))
        );
        assert_eq!(
            eval_irules(r#""abc" matches_glob "a?c""#),
            Some(TclValue::Int(1))
        );
        assert_eq!(
            eval_irules(r#""abc" matches_glob "a?d""#),
            Some(TclValue::Int(0))
        );
    }

    // (string-delimiter stripping now lives in the shared `tcl_syntax::expr`
    // walk — `strip_delims` — and is exercised by its tests.)

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
