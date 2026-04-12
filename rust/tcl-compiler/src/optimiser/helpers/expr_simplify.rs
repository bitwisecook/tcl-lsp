//! Expression-simplification helpers (C30e, partial).
//!
//! Ported from `core/compiler/optimiser/_expr_simplify.py`. The
//! Python module is a 1100-line toolkit of AST-level expression
//! rewriters consumed by the propagation, branch-folding, and
//! pattern-recognition passes. This strip lands the three
//! helpers every one of those consumers actually depends on to
//! produce any rewrite at all:
//!
//! - [`try_fold_expr`] — constant-fold an expression text via
//!   [`eval_tcl_expr`].
//! - [`try_unwrap_expr_in_expr`] — unwrap a redundant
//!   `[expr {…}]` in expression context (`O115`).
//! - [`substitute_expr_constants`] — replace `$var` references
//!   with their SCCP-proved literals.
//!
//! The deeper AST rewrites (`InstCombine`, strength reduction,
//! strlen simplification, `eq`/`ne` → `eq`/`ne` string-compare
//! promotion, pattern-match conversion, `DeMorgan`, etc.) are
//! stubbed and will land as follow-up sub-strips. Each stub returns
//! `(expr.to_owned(), false)` — "no change" — so downstream
//! passes using the stable signature keep working and their
//! diagnostic codes will fire once the real rewrite is
//! populated.
//!
//! The sub-strips are sized independently:
//!
//! - **C30e1** — `try_fold_expr` (landed).
//! - **C30e2** — `try_unwrap_expr_in_expr` (landed).
//! - **C30e3** — `substitute_expr_constants` (landed).
//! - **C30e4** — `instcombine_expr` fixpoint + `simplify_to_fixpoint`.
//! - **C30e5** — `try_strength_reduce_expr`.
//! - **C30e6** — `try_strlen_simplify_expr`.
//! - **C30e7** — `try_eq_ne_string_compare_simplify_expr`.

use std::collections::HashSet;

use tcl_lexer::{tokenise_expr, ExprTokenType};

use crate::expr_ast::ExprNode;
use crate::expr_parser::parse_expr;
use crate::naming::normalise_var_name;
use crate::tcl_expr_eval::{eval_tcl_expr, format_tcl_value, Env};

// ---------------------------------------------------------------------------
// Landed: try_fold_expr (O101 — fold constant expression)
// ---------------------------------------------------------------------------

/// Attempt to fold `expr` to a Tcl literal value by evaluating it
/// with an empty environment.
///
/// Returns `Some(folded_text)` when every variable-free sub-
/// expression collapses to a value and the rendered literal
/// differs from `expr.trim()`. Returns `None` when the
/// expression depends on a variable not in the env, a command
/// substitution, or any domain error (match `eval_tcl_expr`'s
/// conservative "give up, use runtime form" contract).
#[must_use]
pub fn try_fold_expr(expr: &str, dialect: Option<&str>) -> Option<String> {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return None;
    }
    let node = parse_expr(trimmed, dialect);
    if matches!(node, ExprNode::Raw { .. }) {
        return None;
    }
    let env = Env::new();
    let value = eval_tcl_expr(&node, &env)?;
    let rendered = format_tcl_value(value);
    if rendered == trimmed {
        return None;
    }
    Some(rendered)
}

// ---------------------------------------------------------------------------
// Landed: try_unwrap_expr_in_expr (O115 — redundant nested expr)
// ---------------------------------------------------------------------------

/// Detect and strip a redundant `[expr {…}]` wrapper in
/// expression context. Returns the inner expression text when the
/// whole input is of the form `[expr {body}]` or `[expr body]`;
/// otherwise `None`.
///
/// The unwrap is sound because in *expression* context Tcl already
/// re-evaluates the text via `expr`, so the inner `[expr {…}]`
/// runs the evaluator twice on the same body. Branch folding
/// reports this as **O115**.
#[must_use]
pub fn try_unwrap_expr_in_expr(expr_text: &str) -> Option<String> {
    let s = expr_text.trim();
    let inner = s.strip_prefix('[').and_then(|rest| rest.strip_suffix(']'))?;
    let inner = inner.trim();
    let rest = inner.strip_prefix("expr")?;
    // Must have a word boundary after "expr".
    let rest_first = rest.chars().next()?;
    if !(rest_first.is_whitespace() || rest_first == '{' || rest_first == '"') {
        return None;
    }
    let rest = rest.trim_start();
    // Body can be braced, quoted, or bare.
    let body = if let Some(inside) = rest.strip_prefix('{').and_then(|r| r.strip_suffix('}')) {
        inside.trim().to_owned()
    } else if let Some(inside) = rest.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
        inside.trim().to_owned()
    } else {
        rest.trim().to_owned()
    };
    if body.is_empty() || body == s {
        return None;
    }
    Some(body)
}

// ---------------------------------------------------------------------------
// Landed: substitute_expr_constants (O100 — constant propagation)
// ---------------------------------------------------------------------------

/// Outcome of [`substitute_expr_constants`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubstitutionResult {
    /// The rewritten expression text.
    pub text: String,
    /// `true` when at least one variable was substituted.
    pub changed: bool,
    /// Normalised names of the variables that were substituted
    /// (useful for downstream passes building DSE worklists).
    pub substituted: HashSet<String>,
}

/// Replace each `$var` / `${var}` token in `expr` with its
/// literal value from `constants`, quoting non-numeric values to
/// keep the resulting text valid as an `expr` body. Returns the
/// new text plus book-keeping.
///
/// Numeric literals (parseable as integer or float) are
/// substituted verbatim; anything else is wrapped in `"…"` with
/// backslashes and quotes escaped.
#[must_use]
pub fn substitute_expr_constants<S: std::hash::BuildHasher>(
    expr: &str,
    constants: &std::collections::HashMap<String, String, S>,
    dialect: Option<&str>,
) -> SubstitutionResult {
    let tokens = tokenise_expr(expr, dialect);
    let mut pieces: Vec<String> = Vec::new();
    let mut cursor: usize = 0;
    let mut changed = false;
    let mut substituted: HashSet<String> = HashSet::new();

    for tok in tokens {
        let start = tok.start as usize;
        let end_incl = tok.end as usize;
        let end_excl = end_incl.saturating_add(1).min(expr.len());

        if start > cursor && start <= expr.len() {
            pieces.push(expr[cursor..start].to_owned());
        }

        if tok.kind == ExprTokenType::Variable {
            let name = normalise_var_name(&tok.text).to_owned();
            if let Some(value) = constants.get(&name) {
                if is_numeric_literal(value) {
                    pieces.push(value.clone());
                } else {
                    let escaped = value.replace('\\', r"\\").replace('"', "\\\"");
                    pieces.push(format!("\"{escaped}\""));
                }
                changed = true;
                substituted.insert(name);
            } else {
                pieces.push(tok.text.clone());
            }
        } else {
            pieces.push(tok.text.clone());
        }

        cursor = end_excl;
    }

    if cursor < expr.len() {
        pieces.push(expr[cursor..].to_owned());
    }

    SubstitutionResult {
        text: pieces.concat(),
        changed,
        substituted,
    }
}

fn is_numeric_literal(text: &str) -> bool {
    text.trim().parse::<i64>().is_ok() || text.trim().parse::<f64>().is_ok()
}

// ---------------------------------------------------------------------------
// Deferred: returns `(expr, false)` — "no change" — until the
// corresponding C30e sub-strip lands. Downstream passes
// (`propagation`, `branch_folding::optimise_branch_proc_calls`,
// `pattern_recognition`) call these through the stable
// signature and will start producing their respective O-codes as
// soon as each stub is replaced.
// ---------------------------------------------------------------------------

/// C30e4 (deferred). InstCombine-style fixpoint simplification.
///
/// Returns `(expr.to_owned(), false)` — callers should not
/// depend on any rewrite firing from this helper until the
/// corresponding sub-strip lands.
#[must_use]
pub fn instcombine_expr(expr: &str, _bool_context: bool) -> (String, bool) {
    (expr.to_owned(), false)
}

/// C30e5 (deferred). Strength reduction: `x * 1 → x`,
/// `x + 0 → x`, `x ** 2 → x * x`, `x % pow2 → x & (pow2 - 1)`,
/// etc. Returns `(expr.to_owned(), false)` until the sub-strip
/// lands.
#[must_use]
pub fn try_strength_reduce_expr(expr: &str) -> (String, bool) {
    (expr.to_owned(), false)
}

/// C30e6 (deferred). Simplify `[string length $s] == 0` to
/// `$s eq ""` and similar strlen forms. Returns `(expr.to_owned(),
/// false)` until the sub-strip lands.
#[must_use]
pub fn try_strlen_simplify_expr(expr: &str) -> (String, bool) {
    (expr.to_owned(), false)
}

/// C30e7 (deferred). Promote numeric `==` / `!=` against string
/// literals to `eq` / `ne` when the left-hand side is known to be
/// a string. Returns `(expr.to_owned(), false)` until the
/// sub-strip lands.
#[must_use]
pub fn try_eq_ne_string_compare_simplify_expr(expr: &str) -> (String, bool) {
    (expr.to_owned(), false)
}

/// Return `true` when `node` contains any command-substitution
/// subtree. Matches `_expr_has_command_subst`.
#[must_use]
pub fn expr_has_command_subst(node: &ExprNode) -> bool {
    match node {
        ExprNode::Command { .. } => true,
        ExprNode::Binary { left, right, .. } => {
            expr_has_command_subst(left) || expr_has_command_subst(right)
        }
        ExprNode::Unary { operand, .. } => expr_has_command_subst(operand),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            expr_has_command_subst(condition)
                || expr_has_command_subst(true_branch)
                || expr_has_command_subst(false_branch)
        }
        ExprNode::Call { args, .. } => args.iter().any(expr_has_command_subst),
        ExprNode::Literal { .. }
        | ExprNode::Var { .. }
        | ExprNode::Raw { .. }
        | ExprNode::String { .. } => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- try_fold_expr ------------------------------------------------------

    #[test]
    fn fold_integer_arithmetic() {
        assert_eq!(try_fold_expr("1 + 2", None).as_deref(), Some("3"));
        assert_eq!(try_fold_expr("10 * 5", None).as_deref(), Some("50"));
        assert_eq!(try_fold_expr("100 / 4", None).as_deref(), Some("25"));
    }

    #[test]
    fn fold_comparison_to_bool_literal() {
        assert_eq!(try_fold_expr("1 < 2", None).as_deref(), Some("1"));
        assert_eq!(try_fold_expr("3 == 3", None).as_deref(), Some("1"));
        assert_eq!(try_fold_expr("5 > 10", None).as_deref(), Some("0"));
    }

    #[test]
    fn fold_returns_none_for_var_expressions() {
        assert!(try_fold_expr("$x + 1", None).is_none());
        assert!(try_fold_expr("[cmd]", None).is_none());
    }

    #[test]
    fn fold_returns_none_when_already_literal() {
        // "42" folds to "42" — no change, None.
        assert!(try_fold_expr("42", None).is_none());
    }

    #[test]
    fn fold_empty_expression() {
        assert!(try_fold_expr("", None).is_none());
        assert!(try_fold_expr("   ", None).is_none());
    }

    // -- try_unwrap_expr_in_expr --------------------------------------------

    #[test]
    fn unwrap_braced_body() {
        assert_eq!(
            try_unwrap_expr_in_expr("[expr {$x + 1}]").as_deref(),
            Some("$x + 1"),
        );
    }

    #[test]
    fn unwrap_quoted_body() {
        assert_eq!(
            try_unwrap_expr_in_expr(r#"[expr "$x + 1"]"#).as_deref(),
            Some("$x + 1"),
        );
    }

    #[test]
    fn unwrap_bare_body() {
        assert_eq!(
            try_unwrap_expr_in_expr("[expr $x]").as_deref(),
            Some("$x"),
        );
    }

    #[test]
    fn unwrap_rejects_non_expr_wrapping() {
        assert!(try_unwrap_expr_in_expr("[foo bar]").is_none());
        assert!(try_unwrap_expr_in_expr("$x + 1").is_none());
        assert!(try_unwrap_expr_in_expr("[express]").is_none());
    }

    #[test]
    fn unwrap_rejects_empty_body() {
        assert!(try_unwrap_expr_in_expr("[expr {}]").is_none());
    }

    // -- substitute_expr_constants ------------------------------------------

    fn consts(pairs: &[(&str, &str)]) -> std::collections::HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    #[test]
    fn substitute_numeric_var_verbatim() {
        let c = consts(&[("x", "42")]);
        let out = substitute_expr_constants("$x + 1", &c, None);
        assert_eq!(out.text, "42 + 1");
        assert!(out.changed);
        assert!(out.substituted.contains("x"));
    }

    #[test]
    fn substitute_string_var_is_quoted() {
        let c = consts(&[("s", "hello")]);
        let out = substitute_expr_constants("$s", &c, None);
        assert_eq!(out.text, "\"hello\"");
        assert!(out.changed);
    }

    #[test]
    fn substitute_unknown_var_is_untouched() {
        let c = consts(&[("x", "1")]);
        let out = substitute_expr_constants("$y + 1", &c, None);
        assert!(!out.changed);
        assert!(out.substituted.is_empty());
    }

    #[test]
    fn substitute_braced_var_reference() {
        let c = consts(&[("x", "7")]);
        let out = substitute_expr_constants("${x} * 2", &c, None);
        assert_eq!(out.text, "7 * 2");
        assert!(out.changed);
    }

    #[test]
    fn substitute_escapes_backslash_and_quote() {
        let c = consts(&[("s", "he said \"hi\\there\"")]);
        let out = substitute_expr_constants("$s", &c, None);
        assert_eq!(out.text, r#""he said \"hi\\there\"""#);
    }

    // -- expr_has_command_subst --------------------------------------------

    #[test]
    fn command_subst_detection() {
        let expr = parse_expr("[foo] + 1", None);
        assert!(expr_has_command_subst(&expr));
        let expr = parse_expr("$x + 1", None);
        assert!(!expr_has_command_subst(&expr));
        let expr = parse_expr("1 + 2", None);
        assert!(!expr_has_command_subst(&expr));
    }

    // -- stubs just keep their signature -----------------------------------

    #[test]
    fn stubs_return_no_change() {
        assert_eq!(instcombine_expr("$x + 0", false), ("$x + 0".into(), false));
        assert_eq!(
            try_strength_reduce_expr("$x * 1"),
            ("$x * 1".into(), false),
        );
        assert_eq!(
            try_strlen_simplify_expr("[string length $s] == 0"),
            ("[string length $s] == 0".into(), false),
        );
        assert_eq!(
            try_eq_ne_string_compare_simplify_expr("$x == \"foo\""),
            ("$x == \"foo\"".into(), false),
        );
    }
}
