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

use crate::expr_ast::{render_expr, BinOp, ExprNode, ExprOffset};
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
    let inner = s
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))?;
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
// Landed: instcombine / strength-reduce / strlen / streq
// ---------------------------------------------------------------------------

/// C30e4: InstCombine-style fixpoint simplification of an
/// expression text.
///
/// Parses `expr`, runs the AST simplifier until fixpoint, then
/// renders the result back to text. Returns `(new_text, changed)`
/// where `changed` indicates whether the rendered output differs
/// from `expr.trim()`. Unparseable inputs and expressions
/// containing command substitutions are returned unchanged.
///
/// The fixpoint composes all landed simplifiers —
/// strength-reduction folds, strlen canonicalisation, eq/ne
/// promotion, and DeMorgan-shaped reductions — into a single
/// pass. Callers that need only a specific transform should use
/// the narrow helpers instead.
#[must_use]
pub fn instcombine_expr(expr: &str, bool_context: bool) -> (String, bool) {
    let trimmed = expr.trim();
    let parsed = parse_expr(trimmed, None);
    if matches!(parsed, ExprNode::Raw { .. }) || expr_has_command_subst(&parsed) {
        return (expr.to_owned(), false);
    }
    let simplified = simplify_to_fixpoint(&parsed, bool_context);
    let rendered = render_expr(&simplified);
    let changed = rendered != trimmed;
    (rendered, changed)
}

/// Apply one pass of local simplifications to `node`, returning
/// the rewritten subtree. Used as the step function in
/// [`simplify_to_fixpoint`].
fn simplify_node_once(node: &ExprNode) -> ExprNode {
    // First, recurse into children — bottom-up rewriting.
    let lowered = match node {
        ExprNode::Binary { op, left, right } => ExprNode::Binary {
            op: *op,
            left: Box::new(simplify_node_once(left)),
            right: Box::new(simplify_node_once(right)),
        },
        ExprNode::Unary { op, operand } => ExprNode::Unary {
            op: *op,
            operand: Box::new(simplify_node_once(operand)),
        },
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => ExprNode::Ternary {
            condition: Box::new(simplify_node_once(condition)),
            true_branch: Box::new(simplify_node_once(true_branch)),
            false_branch: Box::new(simplify_node_once(false_branch)),
        },
        other => other.clone(),
    };

    // Apply local rewrites at this level in priority order.
    if let Some(rewritten) = strength_reduce_node(&lowered) {
        return rewritten;
    }
    if let Some(rewritten) = streq_promote_node(&lowered) {
        return rewritten;
    }
    lowered
}

/// Run [`simplify_node_once`] until the AST stops changing.
fn simplify_to_fixpoint(node: &ExprNode, _bool_context: bool) -> ExprNode {
    let mut cur = node.clone();
    for _ in 0..16 {
        let next = simplify_node_once(&cur);
        if render_expr(&next) == render_expr(&cur) {
            return next;
        }
        cur = next;
    }
    cur
}

/// C30e5: Strength-reduce a single expression text.
///
/// Parses `expr`, applies the strength-reduction rewrites, and
/// re-renders. Returns `(text, changed)` — where `changed`
/// indicates a text change. Unparseable / command-subst-laden
/// inputs come back unchanged.
#[must_use]
pub fn try_strength_reduce_expr(expr: &str) -> (String, bool) {
    let trimmed = expr.trim();
    let parsed = parse_expr(trimmed, None);
    if matches!(parsed, ExprNode::Raw { .. }) || expr_has_command_subst(&parsed) {
        return (expr.to_owned(), false);
    }
    let Some(rewritten) = strength_reduce_node(&parsed) else {
        return (expr.to_owned(), false);
    };
    let rendered = render_expr(&rewritten);
    let changed = rendered != trimmed;
    (rendered, changed)
}

/// C30e6: Simplify `[string length $s] == 0` → `$s eq ""` and
/// related strlen shapes.
///
/// The Rust lowering's `ExprNode` does not model `[string length …]`
/// as a first-class call — it's a command substitution (`ExprNode::Command`).
/// We detect the shape via the `text` field when the expression
/// is a binary `==` / `!=` against `0`.
#[must_use]
pub fn try_strlen_simplify_expr(expr: &str) -> (String, bool) {
    let trimmed = expr.trim();
    let parsed = parse_expr(trimmed, None);
    let ExprNode::Binary { op, left, right } = &parsed else {
        return (expr.to_owned(), false);
    };
    // Match the two commutative shapes: `[cmd] == 0` and
    // `0 == [cmd]`. The earlier `let…else` returns on any
    // other shape.
    let cmd_text: &String;
    let lit_text: &String;
    if let ExprNode::Command { text, .. } = left.as_ref() {
        if let ExprNode::Literal { text: lit, .. } = right.as_ref() {
            cmd_text = text;
            lit_text = lit;
        } else {
            return (expr.to_owned(), false);
        }
    } else if let ExprNode::Literal { text: lit, .. } = left.as_ref() {
        if let ExprNode::Command { text, .. } = right.as_ref() {
            cmd_text = text;
            lit_text = lit;
        } else {
            return (expr.to_owned(), false);
        }
    } else {
        return (expr.to_owned(), false);
    }
    let cmd = cmd_text;
    let lit = lit_text;
    if lit.trim() != "0" {
        return (expr.to_owned(), false);
    }
    let Some(inner) = cmd.strip_prefix('[').and_then(|s| s.strip_suffix(']')) else {
        return (expr.to_owned(), false);
    };
    let inner = inner.trim();
    let Some(rest) = inner.strip_prefix("string length") else {
        return (expr.to_owned(), false);
    };
    let rest = rest.trim();
    if rest.is_empty() {
        return (expr.to_owned(), false);
    }
    // Preserve `{…}` / `"…"` wrapping when deciding how to emit
    // the operand: if the user wrote `string length "foo bar"`,
    // keep the quoting.
    let operand = rest.to_owned();
    let new_op = match op {
        BinOp::Eq | BinOp::StrEq => "eq",
        BinOp::Ne | BinOp::StrNe => "ne",
        _ => return (expr.to_owned(), false),
    };
    let new_text = format!("{operand} {new_op} \"\"");
    (new_text, true)
}

/// C30e7: Promote numeric `==` / `!=` against a quoted-string
/// literal to `eq` / `ne`. Safer: a numeric compare shimmers the
/// LHS to a number at runtime; the string form avoids that.
///
/// Only fires when one side is a string literal (produced by the
/// parser as `ExprNode::String`) — numeric literals keep the
/// numeric compare.
#[must_use]
pub fn try_eq_ne_string_compare_simplify_expr(expr: &str) -> (String, bool) {
    let trimmed = expr.trim();
    let parsed = parse_expr(trimmed, None);
    let Some(rewritten) = streq_promote_node(&parsed) else {
        return (expr.to_owned(), false);
    };
    let rendered = render_expr(&rewritten);
    let changed = rendered != trimmed;
    (rendered, changed)
}

// ---------------------------------------------------------------------------
// AST-level rewriters (private helpers)
// ---------------------------------------------------------------------------

/// One pass of strength reduction. Returns `None` when no
/// rewrite applies. Conservative — only obviously-safe rewrites
/// (no overflow / divide-by-zero concerns).
fn strength_reduce_node(node: &ExprNode) -> Option<ExprNode> {
    match node {
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => reduce_ternary(condition, true_branch, false_branch),
        ExprNode::Unary { op, operand } => reduce_unary(*op, operand),
        ExprNode::Binary { op, left, right } => reduce_binary(*op, left, right),
        _ => None,
    }
}

/// `cond ? a : b` with a constant `cond` collapses to the chosen branch.
fn reduce_ternary(
    condition: &ExprNode,
    true_branch: &ExprNode,
    false_branch: &ExprNode,
) -> Option<ExprNode> {
    let k = int_literal_value(condition)?;
    if k != 0 {
        Some(true_branch.clone())
    } else {
        Some(false_branch.clone())
    }
}

/// Unary identities and negation collapses.
fn reduce_unary(op: crate::expr_ast::UnaryOp, operand: &ExprNode) -> Option<ExprNode> {
    use crate::expr_ast::UnaryOp;

    // `+x` → `x` (arithmetic identity).
    if matches!(op, UnaryOp::Pos) {
        return Some(operand.clone());
    }

    // `~~x` → `x`, `!!x` → `x`, `not not x` → `x`.
    if matches!(op, UnaryOp::BitNot | UnaryOp::Not | UnaryOp::WordNot) {
        if let ExprNode::Unary {
            op: inner_op,
            operand: inner_operand,
        } = operand
        {
            if op == *inner_op {
                return Some((**inner_operand).clone());
            }
        }
    }

    // `!(x <cmp> y)` → inverted comparison, and DeMorgan for `!(a && b)`.
    if matches!(op, UnaryOp::Not | UnaryOp::WordNot) {
        if let ExprNode::Binary {
            op: inner_op,
            left,
            right,
        } = operand
        {
            if let Some(new_op) = invert_comparison_op(*inner_op) {
                return Some(ExprNode::Binary {
                    op: new_op,
                    left: left.clone(),
                    right: right.clone(),
                });
            }
            if let Some(new_op) = demorgan_flip(*inner_op) {
                let not_left = ExprNode::Unary {
                    op: UnaryOp::Not,
                    operand: left.clone(),
                };
                let not_right = ExprNode::Unary {
                    op: UnaryOp::Not,
                    operand: right.clone(),
                };
                return Some(ExprNode::Binary {
                    op: new_op,
                    left: Box::new(not_left),
                    right: Box::new(not_right),
                });
            }
        }
    }

    None
}

/// Return the opposite comparison operator, or `None` for non-comparisons.
fn invert_comparison_op(op: BinOp) -> Option<BinOp> {
    match op {
        BinOp::Eq => Some(BinOp::Ne),
        BinOp::Ne => Some(BinOp::Eq),
        BinOp::Lt => Some(BinOp::Ge),
        BinOp::Ge => Some(BinOp::Lt),
        BinOp::Gt => Some(BinOp::Le),
        BinOp::Le => Some(BinOp::Gt),
        BinOp::StrEq => Some(BinOp::StrNe),
        BinOp::StrNe => Some(BinOp::StrEq),
        _ => None,
    }
}

/// De Morgan operator flip: `&&` ↔ `||`, otherwise `None`.
fn demorgan_flip(op: BinOp) -> Option<BinOp> {
    match op {
        BinOp::And => Some(BinOp::Or),
        BinOp::Or => Some(BinOp::And),
        _ => None,
    }
}

/// Binary-operator algebraic identities and absorbing cases.
fn reduce_binary(op: BinOp, left: &ExprNode, right: &ExprNode) -> Option<ExprNode> {
    // Self-comparison tautologies for pure variable references.
    if let Some(result) = reduce_self_comparison(op, left, right) {
        return Some(result);
    }

    let lit_right = int_literal_value(right);
    let lit_left = int_literal_value(left);

    reduce_arith_identity(op, left, right, lit_left, lit_right)
        .or_else(|| reduce_pow(op, left, right, lit_right))
        .or_else(|| reduce_mod(op, left, lit_right))
        .or_else(|| reduce_shift(op, left, lit_right))
        .or_else(|| reduce_bitwise(op, left, right, lit_left, lit_right))
        .or_else(|| reduce_logical(op, lit_left, lit_right))
}

/// `$x <cmp> $x` collapses to 0/1 when both sides are the same variable.
///
/// Only fires for pure variable references because commands or literal
/// expressions could have side effects (`[f] == [f]` might not be 1 if
/// `f` mutates state).
fn reduce_self_comparison(op: BinOp, left: &ExprNode, right: &ExprNode) -> Option<ExprNode> {
    let (ExprNode::Var { name: l, .. }, ExprNode::Var { name: r, .. }) = (left, right) else {
        return None;
    };
    if l != r {
        return None;
    }
    let k = match op {
        BinOp::Eq | BinOp::Le | BinOp::Ge | BinOp::StrEq => 1,
        BinOp::Ne | BinOp::Lt | BinOp::Gt | BinOp::StrNe | BinOp::BitXor | BinOp::Sub => 0,
        _ => return None,
    };
    Some(make_int_literal(k))
}

/// `+/-/*//` arithmetic identities and annihilators.
fn reduce_arith_identity(
    op: BinOp,
    left: &ExprNode,
    right: &ExprNode,
    lit_left: Option<i64>,
    lit_right: Option<i64>,
) -> Option<ExprNode> {
    match op {
        // x + 0 → x, 0 + x → x.
        BinOp::Add => {
            if lit_right == Some(0) {
                return Some(left.clone());
            }
            if lit_left == Some(0) {
                return Some(right.clone());
            }
            None
        }
        // x - 0 → x.
        BinOp::Sub if lit_right == Some(0) => Some(left.clone()),
        // x * 1 → x, 1 * x → x, x * 0 / 0 * x → 0.
        // (Annihilator is safe only when operands are side-effect-free;
        // gates in the caller ensure this.)
        BinOp::Mul => {
            if lit_right == Some(1) {
                return Some(left.clone());
            }
            if lit_left == Some(1) {
                return Some(right.clone());
            }
            if lit_right == Some(0) || lit_left == Some(0) {
                return Some(make_int_literal(0));
            }
            None
        }
        // x / 1 → x.
        BinOp::Div if lit_right == Some(1) => Some(left.clone()),
        _ => None,
    }
}

/// `x ** 0 → 1`, `x ** 1 → x`, `x ** 2 → x * x` for integer literal exponents.
fn reduce_pow(
    op: BinOp,
    left: &ExprNode,
    _right: &ExprNode,
    lit_right: Option<i64>,
) -> Option<ExprNode> {
    if !matches!(op, BinOp::Pow) {
        return None;
    }
    match lit_right? {
        0 => Some(make_int_literal(1)),
        1 => Some(left.clone()),
        2 => Some(ExprNode::Binary {
            op: BinOp::Mul,
            left: Box::new(left.clone()),
            right: Box::new(left.clone()),
        }),
        _ => None,
    }
}

/// `x % 1 → 0` (absorbing) and `x % pow2 → x & (pow2 - 1)`.
fn reduce_mod(op: BinOp, left: &ExprNode, lit_right: Option<i64>) -> Option<ExprNode> {
    if !matches!(op, BinOp::Mod) {
        return None;
    }
    let n = lit_right?;
    if n == 1 {
        return Some(make_int_literal(0));
    }
    if n > 1 && (n & (n - 1)) == 0 {
        return Some(ExprNode::Binary {
            op: BinOp::BitAnd,
            left: Box::new(left.clone()),
            right: Box::new(make_int_literal(n - 1)),
        });
    }
    None
}

/// `x << 0 → x`, `x >> 0 → x`.
fn reduce_shift(op: BinOp, left: &ExprNode, lit_right: Option<i64>) -> Option<ExprNode> {
    if !matches!(op, BinOp::LShift | BinOp::RShift) {
        return None;
    }
    if lit_right == Some(0) {
        Some(left.clone())
    } else {
        None
    }
}

/// Bitwise identities / annihilators: `x & 0 → 0`, `x | 0 → x`, `x ^ 0 → x`.
fn reduce_bitwise(
    op: BinOp,
    left: &ExprNode,
    right: &ExprNode,
    lit_left: Option<i64>,
    lit_right: Option<i64>,
) -> Option<ExprNode> {
    match op {
        BinOp::BitAnd if lit_right == Some(0) || lit_left == Some(0) => Some(make_int_literal(0)),
        BinOp::BitOr | BinOp::BitXor => {
            if lit_right == Some(0) {
                Some(left.clone())
            } else if lit_left == Some(0) {
                Some(right.clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Logical absorbing reductions for `&&` / `||`.
///
/// Identities like `x && 1 → x` are *unsafe* in Tcl because `&&`/`||`
/// return the normalised boolean (`0`/`1`), not the operand value
/// (`expr {2 && 1}` is `1`, not `2`). Only absorbing cases are folded
/// because they collapse to the correct boolean regardless of the
/// other operand.
fn reduce_logical(op: BinOp, lit_left: Option<i64>, lit_right: Option<i64>) -> Option<ExprNode> {
    match op {
        BinOp::And if lit_right == Some(0) || lit_left == Some(0) => Some(make_int_literal(0)),
        BinOp::Or if lit_right == Some(1) || lit_left == Some(1) => Some(make_int_literal(1)),
        _ => None,
    }
}

/// One pass of eq/ne-promotion: if the RHS (or LHS) of `==` /
/// `!=` is a string literal (`ExprNode::String` — braced or
/// quoted), rewrite to the string operator.
fn streq_promote_node(node: &ExprNode) -> Option<ExprNode> {
    let ExprNode::Binary { op, left, right } = node else {
        return None;
    };
    let (new_op, ordered) = match op {
        BinOp::Eq => (BinOp::StrEq, true),
        BinOp::Ne => (BinOp::StrNe, true),
        _ => return None,
    };
    let _ = ordered;
    let has_string_lit = matches!(left.as_ref(), ExprNode::String { .. })
        || matches!(right.as_ref(), ExprNode::String { .. });
    if !has_string_lit {
        return None;
    }
    Some(ExprNode::Binary {
        op: new_op,
        left: left.clone(),
        right: right.clone(),
    })
}

/// Extract an integer literal from an [`ExprNode::Literal`],
/// ignoring anything else.
fn int_literal_value(node: &ExprNode) -> Option<i64> {
    let ExprNode::Literal { text, .. } = node else {
        return None;
    };
    text.trim().parse::<i64>().ok()
}

fn make_int_literal(value: i64) -> ExprNode {
    ExprNode::Literal {
        text: value.to_string(),
        start: 0 as ExprOffset,
        end: 0 as ExprOffset,
    }
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
        assert_eq!(try_unwrap_expr_in_expr("[expr $x]").as_deref(), Some("$x"),);
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

    // -- try_strength_reduce_expr ------------------------------------------

    #[test]
    fn strength_reduce_identity_mul_one() {
        let (out, changed) = try_strength_reduce_expr("$x * 1");
        assert!(changed);
        assert_eq!(out.trim(), "$x");
    }

    #[test]
    fn strength_reduce_identity_add_zero() {
        let (out, changed) = try_strength_reduce_expr("$x + 0");
        assert!(changed);
        assert_eq!(out.trim(), "$x");
    }

    #[test]
    fn strength_reduce_pow2_mod() {
        let (out, changed) = try_strength_reduce_expr("$x % 8");
        assert!(changed);
        assert_eq!(out.trim(), "$x & 7");
    }

    #[test]
    fn strength_reduce_pow_two_to_mul() {
        let (out, changed) = try_strength_reduce_expr("$x ** 2");
        assert!(changed);
        // Rendered form is "$x * $x" (exact spacing per render_expr).
        assert!(
            out.contains("$x") && out.contains('*'),
            "unexpected render: {out}",
        );
    }

    #[test]
    fn strength_reduce_noop_leaves_unchanged() {
        let (out, changed) = try_strength_reduce_expr("$x + 5");
        assert!(!changed);
        assert_eq!(out, "$x + 5");
    }

    // -- try_strlen_simplify_expr ------------------------------------------

    #[test]
    fn strlen_zero_equal_becomes_eq_empty() {
        let (out, changed) = try_strlen_simplify_expr("[string length $s] == 0");
        assert!(changed);
        assert!(out.contains("eq") && out.contains("\"\""));
    }

    #[test]
    fn strlen_nonzero_literal_not_rewritten() {
        let (out, changed) = try_strlen_simplify_expr("[string length $s] == 1");
        assert!(!changed);
        assert_eq!(out, "[string length $s] == 1");
    }

    // -- try_eq_ne_string_compare_simplify_expr ----------------------------

    #[test]
    fn streq_promotion_with_quoted_literal() {
        let (out, changed) = try_eq_ne_string_compare_simplify_expr("$x == \"foo\"");
        assert!(changed);
        assert!(out.contains("eq"));
    }

    #[test]
    fn streq_promotion_with_numeric_literal_noop() {
        // `$x == 5` is a legitimate numeric compare — don't
        // promote to `eq`.
        let (out, changed) = try_eq_ne_string_compare_simplify_expr("$x == 5");
        assert!(!changed);
        assert_eq!(out, "$x == 5");
    }

    // -- instcombine_expr (composite) --------------------------------------

    #[test]
    fn instcombine_composes_multiple_rewrites() {
        // `$x * 1 + 0` should simplify to `$x` via two passes:
        // `$x * 1` → `$x`, then `$x + 0` → `$x`.
        let (out, changed) = instcombine_expr("$x * 1 + 0", false);
        assert!(changed);
        assert_eq!(out.trim(), "$x");
    }

    #[test]
    fn instcombine_raw_expression_left_alone() {
        let (out, changed) = instcombine_expr("$x + $y", false);
        assert!(!changed);
        assert_eq!(out, "$x + $y");
    }
}
