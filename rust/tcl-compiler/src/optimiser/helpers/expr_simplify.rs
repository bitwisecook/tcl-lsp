//! Expression-simplification helpers (C30e + C30e4–C30e7).
//!
//! Ported from `core/compiler/optimiser/_expr_simplify.py`.  The
//! Python module is a 1100-line toolkit of AST-level expression
//! rewriters consumed by the propagation, branch-folding, and
//! pattern-recognition passes.  Every helper used by those
//! consumers is now landed:
//!
//! - [`try_fold_expr`] — constant-fold an expression text via
//!   [`eval_tcl_expr`].
//! - [`try_unwrap_expr_in_expr`] — unwrap a redundant
//!   `[expr {…}]` in expression context (`O115`).
//! - [`substitute_expr_constants`] — replace `$var` references
//!   with their SCCP-proved literals.
//! - [`instcombine_expr`] — bottom-up fixpoint of local
//!   instcombine rewrites (capped at 16 iterations).
//! - [`try_strength_reduce_expr`] — `x+0` / `x-0` / `x*0` /
//!   `x*1` / `x/1` / `x**2` / `x % 2^k` strength-reduction set.
//! - [`try_strlen_simplify_expr`] — `[string length OP] == 0`
//!   → `OP eq ""`.
//! - [`try_eq_ne_string_compare_simplify_expr`] — `==`/`!=`
//!   against an `ExprNode::String` literal → `eq`/`ne`.
//!
//! Sub-strip history: C30e1 `try_fold_expr` · C30e2
//! `try_unwrap_expr_in_expr` · C30e3 `substitute_expr_constants`
//! · C30e4 `instcombine_expr` · C30e5 `try_strength_reduce_expr`
//! · C30e6 `try_strlen_simplify_expr` · C30e7
//! `try_eq_ne_string_compare_simplify_expr`.  The four AST
//! rewriters are wired into
//! `branch_folding::propagate_into_branches`:
//! `substitute_expr_constants` runs first to build a working
//! text, then the AST rewriters are probed in priority order —
//! `strength_reduce` → `strlen` → `streq` → `instcombine`.  The
//! first rewriter that changes text wins its diagnostic code
//! (`O113` / `O117` / `O120` / `O110`); `O100` ("Propagate
//! constants into branch expression") only fires when
//! substitution changed the text *and* none of the AST
//! rewriters did.

use std::collections::HashSet;

use tcl_lexer::{ExprTokenType, tokenise_expr};

use crate::compilation_unit::FunctionUnit;
use crate::expr_ast::{BinOp, ExprNode, ExprOffset, render_expr};
use crate::expr_parser::parse_expr;
use crate::naming::normalise_var_name;
use crate::tcl_expr_eval::{Env, eval_tcl_expr, format_tcl_value};
use crate::types::{TclType, TypeKind, TypeLattice};

/// A set of variable names proven numeric for the current function, or
/// `None` when no type context is available. Passed to the `*_typed`
/// entry points so the operand-dropping identities fire only on provably
/// numeric operands (mirrors Python's `_is_provably_numeric_expr_node`
/// gate). `None` keeps the historical aggressive behaviour for callers
/// (and tests) that have no type lattice.
pub type NumericCtx<'a> = Option<&'a HashSet<String>>;

/// Build the set of variable names whose **every** SSA version is a known
/// numeric type (Int / Double / Numeric / Boolean). A name absent here is
/// treated as not provably numeric, so a `$x + 0` / `$x * 0` identity is
/// kept — matching Python, which proves numericity per use from the type
/// lattice. Using the function-level join (all versions must agree) is a
/// sound over-approximation of the per-use check.
#[must_use]
pub fn numeric_var_names(fu: &FunctionUnit) -> HashSet<String> {
    use std::collections::HashMap;
    // name → (all-versions-numeric-so-far).
    let mut acc: HashMap<&str, bool> = HashMap::new();
    for ((name, _ver), lattice) in &fu.types {
        let is_num = lattice_is_numeric(lattice);
        acc.entry(name.as_str())
            .and_modify(|v| *v = *v && is_num)
            .or_insert(is_num);
    }
    acc.into_iter()
        .filter(|(_, ok)| *ok)
        .map(|(n, _)| n.to_owned())
        .collect()
}

/// Whether a type-lattice element is a known numeric Tcl type. Mirrors
/// Python's `_NUMERIC_TCL_TYPES` membership.
fn lattice_is_numeric(t: &TypeLattice) -> bool {
    t.kind == TypeKind::Known
        && matches!(
            t.tcl_type,
            Some(TclType::Int | TclType::Double | TclType::Numeric | TclType::Boolean)
        )
}

/// Whether `node` is provably numeric for `expr` arithmetic — so dropping it
/// from an identity rewrite cannot hide Tcl's numeric-coercion error.
/// Mirrors `_is_provably_numeric_expr_node`. With no type context (`None`)
/// every node is assumed numeric, preserving the legacy behaviour for
/// callers without a lattice.
fn node_provably_numeric(node: &ExprNode, numeric: NumericCtx<'_>) -> bool {
    let Some(names) = numeric else {
        return true;
    };
    match node {
        ExprNode::Literal { .. } => true,
        ExprNode::String { text, .. } => is_numeric_string(text),
        ExprNode::Var { name, .. } => names.contains(name.as_str()),
        _ => false,
    }
}

/// Whether the (delimiter-stripped) text of an `expr` string literal parses
/// as an integer or float — the SCCP-inlined-constant case.
fn is_numeric_string(text: &str) -> bool {
    let t = text
        .trim()
        .trim_start_matches(['"', '{'])
        .trim_end_matches(['"', '}'])
        .trim();
    !t.is_empty() && (t.parse::<i64>().is_ok() || t.parse::<f64>().is_ok())
}

// Landed: try_fold_expr (O101 — fold constant expression)

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

/// Fold `expr` to a literal under a set of known constant variable values.
///
/// `constants` maps variable names (no `$`) to their literal text. `braced`
/// distinguishes the substitution model of the enclosing `expr` argument:
///
/// * **braced** (`expr {$a == $b}`) — `expr` resolves the `$var` references
///   itself, so a string-valued constant is a valid string operand; every
///   constant is bound.
/// * **quoted / bare** (`expr "$a == $b"`, `expr $a==$b`) — Tcl substitutes
///   the variable *values* textually before parsing, so a non-numeric value
///   becomes an invalid bareword (a runtime error). Only numeric constants
///   are bound; a string-valued var is left unbound and the fold bails,
///   matching the SCCP `[expr …]` fold (`sccp::env_from_uses_numeric`).
///
/// Returns the folded literal, or `None` when the expression still depends
/// on an unresolved operand / command substitution.
#[must_use]
pub fn try_fold_expr_with_constants<S: std::hash::BuildHasher>(
    expr: &str,
    constants: &std::collections::HashMap<String, String, S>,
    braced: bool,
    dialect: Option<&str>,
) -> Option<String> {
    use crate::tcl_expr_eval::EnvValue;
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return None;
    }
    let node = parse_expr(trimmed, dialect);
    if matches!(node, ExprNode::Raw { .. }) {
        return None;
    }
    let mut env = Env::new();
    for (name, value) in constants {
        if braced || is_numeric_string(value) {
            env.insert(name.clone(), EnvValue::Str(value.clone()));
        }
    }
    let value = eval_tcl_expr(&node, &env)?;
    let rendered = format_tcl_value(value);
    if rendered == trimmed {
        return None;
    }
    Some(rendered)
}

// Landed: try_unwrap_expr_in_expr (O115 — redundant nested expr)

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

// Landed: substitute_expr_constants (O100 — constant propagation)

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

// Landed: instcombine / strength-reduce / strlen / streq

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
    instcombine_expr_typed(expr, bool_context, None)
}

/// As [`instcombine_expr`], but with a numeric-type context so the
/// operand-dropping identities (`$x + 0` → `$x`, `$x * 0` → `0`, …) fire
/// only when the dropped operand is provably numeric. See [`NumericCtx`].
#[must_use]
pub fn instcombine_expr_typed(
    expr: &str,
    bool_context: bool,
    numeric: NumericCtx<'_>,
) -> (String, bool) {
    let trimmed = expr.trim();
    let parsed = parse_expr(trimmed, None);
    if matches!(parsed, ExprNode::Raw { .. }) || expr_has_command_subst(&parsed) {
        return (expr.to_owned(), false);
    }
    let simplified = simplify_to_fixpoint(&parsed, bool_context, numeric);
    let rendered = render_expr(&simplified);
    // SYNC-MAY31-1e (#498): suppress O110 noise. When the canonical
    // re-render differs from the input only in whitespace (e.g.
    // `$x<0` → `$x < 0`), it is a spacing preference, not a real
    // finding — report no change. Structural rewrites (paren removal,
    // identity folds, operand reordering, …) all alter non-whitespace
    // characters and still register as changed. Mirrors Python's
    // `_strip_ws` guard in `compiler/optimiser/_propagation.py`.
    let changed = strip_ws(&rendered) != strip_ws(trimmed);
    (rendered, changed)
}

/// Return `expr` with all whitespace removed. Port of Python's
/// `_strip_ws` ([`_propagation._strip_ws`]) — the whitespace-insensitive
/// comparison key for the O110 noise guard.
fn strip_ws(expr: &str) -> String {
    expr.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Apply one pass of local simplifications to `node`, returning
/// the rewritten subtree. Used as the step function in
/// [`simplify_to_fixpoint`].
fn simplify_node_once(node: &ExprNode, bool_context: bool, numeric: NumericCtx<'_>) -> ExprNode {
    use crate::expr_ast::UnaryOp;
    // First, recurse into children — bottom-up rewriting. The boolean
    // context propagates only where the operand's *value* is consumed as a
    // truth value: the operands of `&&`/`||`/`!` and a ternary condition.
    // Comparison/arithmetic operands consume the operand's full value, so
    // they reset the context to `false`. Mirrors the per-position
    // `bool_context` threading in Python's `_simplify_expr_node`.
    let lowered = match node {
        ExprNode::Binary { op, left, right } => {
            let child_bool = matches!(op, BinOp::And | BinOp::Or | BinOp::WordAnd | BinOp::WordOr);
            ExprNode::Binary {
                op: *op,
                left: Box::new(simplify_node_once(left, child_bool, numeric)),
                right: Box::new(simplify_node_once(right, child_bool, numeric)),
            }
        }
        ExprNode::Unary { op, operand } => {
            let child_bool = matches!(op, UnaryOp::Not | UnaryOp::WordNot);
            ExprNode::Unary {
                op: *op,
                operand: Box::new(simplify_node_once(operand, child_bool, numeric)),
            }
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => ExprNode::Ternary {
            condition: Box::new(simplify_node_once(condition, true, numeric)),
            true_branch: Box::new(simplify_node_once(true_branch, bool_context, numeric)),
            false_branch: Box::new(simplify_node_once(false_branch, bool_context, numeric)),
        },
        other => other.clone(),
    };

    // Apply local rewrites at this level in priority order.
    if let Some(rewritten) = strength_reduce_node(&lowered, bool_context, numeric) {
        return rewritten;
    }
    if let Some(rewritten) = streq_promote_node(&lowered) {
        return rewritten;
    }
    if let Some(rewritten) = reassociate_node(&lowered) {
        return rewritten;
    }
    lowered
}

/// **O110 constant reassociation.** Combine the integer-literal constants
/// across a left-associative `+`/`-` (resp. `*`) chain while keeping every
/// non-constant term: `$a + 1 + 2` → `$a + 3`, `$a + 3 - 1` → `$a + 2`,
/// `$a * 2 * 3` → `$a * 6`. Mirrors `_collect_add_terms` / `_collect_mul_terms`
/// + `_build_*_expr` in `compiler/optimiser/_expr_simplify.py`.
///
/// Fires only when one operand is itself an additive (resp. multiplicative)
/// chain — a pure operand reorder (`1 + $a` → `$a + 1`) is suppressed as
/// noise. All non-constant terms are preserved, so numeric-coercion error
/// semantics are unchanged. The two term-dropping cases that *would* need a
/// provably-numeric guard (annihilating `* 0`, dropping a lone `* 1`) are
/// skipped — Python proves numericity from the SSA type lattice, which this
/// AST-level pass cannot, so it conservatively leaves them be.
fn reassociate_node(node: &ExprNode) -> Option<ExprNode> {
    let ExprNode::Binary { op, left, right } = node else {
        return None;
    };
    match op {
        BinOp::Add | BinOp::Sub => {
            if !is_additive(left) && !is_additive(right) {
                return None;
            }
            let mut terms = Vec::new();
            let constant = collect_add_terms(node, &mut terms)?;
            if constant == i64::MIN {
                return None; // `-constant` would overflow in the builder
            }
            let built = build_add_expr(&terms, constant);
            (render_expr(&built) != render_expr(node)).then_some(built)
        }
        BinOp::Mul => {
            if !is_mul(left) && !is_mul(right) {
                return None;
            }
            let mut terms = Vec::new();
            let constant = collect_mul_terms(node, &mut terms)?;
            // Conservative: don't drop terms without a numeric proof.
            if constant == 0 || (constant == 1 && terms.len() == 1) {
                return None;
            }
            let built = build_mul_expr(&terms, constant);
            (render_expr(&built) != render_expr(node)).then_some(built)
        }
        _ => None,
    }
}

fn is_additive(n: &ExprNode) -> bool {
    matches!(
        n,
        ExprNode::Binary {
            op: BinOp::Add | BinOp::Sub,
            ..
        }
    )
}

fn is_mul(n: &ExprNode) -> bool {
    matches!(n, ExprNode::Binary { op: BinOp::Mul, .. })
}

/// Flatten an `+`/`-` chain: accumulate the literal constant and push every
/// non-literal term onto `terms`. A `-` is followed only when its RHS is an
/// integer literal (otherwise the whole node is an opaque term — Python does
/// not negate a non-literal subtrahend here). `None` on integer overflow.
fn collect_add_terms(node: &ExprNode, terms: &mut Vec<ExprNode>) -> Option<i64> {
    if let ExprNode::Binary { op, left, right } = node {
        match op {
            BinOp::Add => {
                let l = collect_add_terms(left, terms)?;
                let r = collect_add_terms(right, terms)?;
                return l.checked_add(r);
            }
            BinOp::Sub => {
                if let Some(rhs) = int_literal_value(right) {
                    let l = collect_add_terms(left, terms)?;
                    return l.checked_sub(rhs);
                }
            }
            _ => {}
        }
    }
    if let Some(v) = int_literal_value(node) {
        return Some(v);
    }
    terms.push(node.clone());
    Some(0)
}

fn build_add_expr(terms: &[ExprNode], constant: i64) -> ExprNode {
    let Some((first, rest)) = terms.split_first() else {
        return make_int_literal(constant);
    };
    let mut result = first.clone();
    for term in rest {
        result = ExprNode::Binary {
            op: BinOp::Add,
            left: Box::new(result),
            right: Box::new(term.clone()),
        };
    }
    match constant.cmp(&0) {
        std::cmp::Ordering::Greater => ExprNode::Binary {
            op: BinOp::Add,
            left: Box::new(result),
            right: Box::new(make_int_literal(constant)),
        },
        std::cmp::Ordering::Less => ExprNode::Binary {
            op: BinOp::Sub,
            left: Box::new(result),
            right: Box::new(make_int_literal(-constant)),
        },
        std::cmp::Ordering::Equal => result,
    }
}

/// Flatten a `*` chain: multiply the literal constants, push non-literals.
/// `None` on integer overflow.
fn collect_mul_terms(node: &ExprNode, terms: &mut Vec<ExprNode>) -> Option<i64> {
    if let ExprNode::Binary {
        op: BinOp::Mul,
        left,
        right,
    } = node
    {
        let l = collect_mul_terms(left, terms)?;
        let r = collect_mul_terms(right, terms)?;
        return l.checked_mul(r);
    }
    if let Some(v) = int_literal_value(node) {
        return Some(v);
    }
    terms.push(node.clone());
    Some(1)
}

fn build_mul_expr(terms: &[ExprNode], constant: i64) -> ExprNode {
    if constant == 0 {
        return make_int_literal(0);
    }
    let Some((first, rest)) = terms.split_first() else {
        return make_int_literal(constant);
    };
    let mut result = first.clone();
    for term in rest {
        result = ExprNode::Binary {
            op: BinOp::Mul,
            left: Box::new(result),
            right: Box::new(term.clone()),
        };
    }
    if constant != 1 {
        result = ExprNode::Binary {
            op: BinOp::Mul,
            left: Box::new(result),
            right: Box::new(make_int_literal(constant)),
        };
    }
    result
}

/// Run [`simplify_node_once`] until the AST stops changing.
fn simplify_to_fixpoint(node: &ExprNode, bool_context: bool, numeric: NumericCtx<'_>) -> ExprNode {
    let mut cur = node.clone();
    for _ in 0..16 {
        let next = simplify_node_once(&cur, bool_context, numeric);
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
    try_strength_reduce_expr_typed(expr, None)
}

/// As [`try_strength_reduce_expr`], but with a numeric-type context for the
/// operand-dropping identities. See [`NumericCtx`].
#[must_use]
pub fn try_strength_reduce_expr_typed(expr: &str, numeric: NumericCtx<'_>) -> (String, bool) {
    let trimmed = expr.trim();
    let parsed = parse_expr(trimmed, None);
    if matches!(parsed, ExprNode::Raw { .. }) || expr_has_command_subst(&parsed) {
        return (expr.to_owned(), false);
    }
    // A standalone strength-reduce has no enclosing boolean context — the
    // expression's full value is consumed.
    let Some(rewritten) = strength_reduce_node(&parsed, false, numeric) else {
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

// AST-level rewriters (private helpers)

/// One pass of strength reduction. Returns `None` when no
/// rewrite applies. Conservative — only obviously-safe rewrites
/// (no overflow / divide-by-zero concerns).
fn strength_reduce_node(
    node: &ExprNode,
    bool_context: bool,
    numeric: NumericCtx<'_>,
) -> Option<ExprNode> {
    match node {
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => reduce_ternary(condition, true_branch, false_branch),
        ExprNode::Unary { op, operand } => reduce_unary(*op, operand, bool_context, numeric),
        ExprNode::Binary { op, left, right } => {
            reduce_binary(*op, left, right, bool_context, numeric)
        }
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
fn reduce_unary(
    op: crate::expr_ast::UnaryOp,
    operand: &ExprNode,
    bool_context: bool,
    numeric: NumericCtx<'_>,
) -> Option<ExprNode> {
    use crate::expr_ast::UnaryOp;

    // `+x` → `x` (arithmetic identity) — drops the unary `+`, so `x` must be
    // provably numeric or the coercion error (`expr {+$s}`) would be lost.
    if matches!(op, UnaryOp::Pos) && node_provably_numeric(operand, numeric) {
        return Some(operand.clone());
    }

    // `!!x` → `x` / `not not x` → `x` — only sound when `x` already yields a
    // `0`/`1` boolean or the result is consumed in a boolean context;
    // otherwise the double-negation is the very normalisation that turns a
    // non-`0`/`1` value into `0`/`1` (`expr {!!2}` is `1`). Mirrors Python's
    // gated `!!x` collapse.
    if matches!(op, UnaryOp::Not | UnaryOp::WordNot)
        && let ExprNode::Unary {
            op: inner_op,
            operand: inner_operand,
        } = operand
        && matches!(inner_op, UnaryOp::Not | UnaryOp::WordNot)
        && (bool_context || is_boolean_expr(inner_operand))
    {
        return Some((**inner_operand).clone());
    }

    // `~~x` → `x` — drops both bitwise negations, so `x` must be provably
    // numeric or the coercion error would be lost.
    if matches!(op, UnaryOp::BitNot)
        && let ExprNode::Unary {
            op: UnaryOp::BitNot,
            operand: inner_operand,
        } = operand
        && node_provably_numeric(inner_operand, numeric)
    {
        return Some((**inner_operand).clone());
    }

    // `!(x <cmp> y)` → inverted comparison, and DeMorgan for `!(a && b)`.
    if matches!(op, UnaryOp::Not | UnaryOp::WordNot)
        && let ExprNode::Binary {
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
fn reduce_binary(
    op: BinOp,
    left: &ExprNode,
    right: &ExprNode,
    bool_context: bool,
    numeric: NumericCtx<'_>,
) -> Option<ExprNode> {
    // Self-comparison tautologies for pure variable references.
    if let Some(result) = reduce_self_comparison(op, left, right) {
        return Some(result);
    }

    let lit_right = int_literal_value(right);
    let lit_left = int_literal_value(left);

    reduce_arith_identity(op, left, right, lit_left, lit_right, numeric)
        .or_else(|| reduce_pow(op, left, right, lit_right, numeric))
        .or_else(|| reduce_mod(op, left, lit_right, numeric))
        .or_else(|| reduce_shift(op, left, lit_right, numeric))
        .or_else(|| reduce_bitwise(op, left, right, lit_left, lit_right, numeric))
        .or_else(|| reduce_logical(op, left, right, lit_left, lit_right, bool_context))
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
///
/// Every rewrite here removes an arithmetic operation, so the surviving
/// expression must still raise Tcl's numeric-coercion error iff the
/// original would: each is gated on the *non-literal* operand being
/// provably numeric (mirrors Python's `_numeric` guard). Without a type
/// context the guard passes (legacy aggressive behaviour).
fn reduce_arith_identity(
    op: BinOp,
    left: &ExprNode,
    right: &ExprNode,
    lit_left: Option<i64>,
    lit_right: Option<i64>,
    numeric: NumericCtx<'_>,
) -> Option<ExprNode> {
    let num = |n: &ExprNode| node_provably_numeric(n, numeric);
    match op {
        // x + 0 → x, 0 + x → x.
        BinOp::Add => {
            if lit_right == Some(0) && num(left) {
                return Some(left.clone());
            }
            if lit_left == Some(0) && num(right) {
                return Some(right.clone());
            }
            None
        }
        // x - 0 → x.
        BinOp::Sub if lit_right == Some(0) && num(left) => Some(left.clone()),
        // x * 1 → x, 1 * x → x, x * 0 / 0 * x → 0.
        BinOp::Mul => {
            if lit_right == Some(1) && num(left) {
                return Some(left.clone());
            }
            if lit_left == Some(1) && num(right) {
                return Some(right.clone());
            }
            // Annihilation drops the whole non-literal operand.
            if lit_right == Some(0) && num(left) {
                return Some(make_int_literal(0));
            }
            if lit_left == Some(0) && num(right) {
                return Some(make_int_literal(0));
            }
            None
        }
        // x / 1 → x.
        BinOp::Div if lit_right == Some(1) && num(left) => Some(left.clone()),
        _ => None,
    }
}

/// `x ** 0 → 1`, `x ** 1 → x`, `x ** 2 → x * x` for integer literal exponents.
///
/// `** 0` / `** 1` drop the `x ** …` operation, so `x` must be provably
/// numeric. `** 2 → x * x` keeps `x` as an operand on both sides, preserving
/// error semantics without a guard.
fn reduce_pow(
    op: BinOp,
    left: &ExprNode,
    _right: &ExprNode,
    lit_right: Option<i64>,
    numeric: NumericCtx<'_>,
) -> Option<ExprNode> {
    if !matches!(op, BinOp::Pow) {
        return None;
    }
    match lit_right? {
        0 if node_provably_numeric(left, numeric) => Some(make_int_literal(1)),
        1 if node_provably_numeric(left, numeric) => Some(left.clone()),
        2 => Some(ExprNode::Binary {
            op: BinOp::Mul,
            left: Box::new(left.clone()),
            right: Box::new(left.clone()),
        }),
        _ => None,
    }
}

/// `x % 1 → 0` (absorbing) and `x % pow2 → x & (pow2 - 1)`.
///
/// `% 1 → 0` drops `x`, so it needs `x` numeric. The pow2 strength-reduction
/// keeps `x` (the `&` coerces too), so it preserves error semantics.
fn reduce_mod(
    op: BinOp,
    left: &ExprNode,
    lit_right: Option<i64>,
    numeric: NumericCtx<'_>,
) -> Option<ExprNode> {
    if !matches!(op, BinOp::Mod) {
        return None;
    }
    let n = lit_right?;
    if n == 1 && node_provably_numeric(left, numeric) {
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

/// `x << 0 → x`, `x >> 0 → x`. Drops the shift, so `x` must be numeric.
fn reduce_shift(
    op: BinOp,
    left: &ExprNode,
    lit_right: Option<i64>,
    numeric: NumericCtx<'_>,
) -> Option<ExprNode> {
    if !matches!(op, BinOp::LShift | BinOp::RShift) {
        return None;
    }
    if lit_right == Some(0) && node_provably_numeric(left, numeric) {
        Some(left.clone())
    } else {
        None
    }
}

/// Bitwise identities / annihilators: `x & 0 → 0`, `x | 0 → x`, `x ^ 0 → x`.
/// Each drops or strips an operand's coercion, so the non-literal operand
/// must be provably numeric.
fn reduce_bitwise(
    op: BinOp,
    left: &ExprNode,
    right: &ExprNode,
    lit_left: Option<i64>,
    lit_right: Option<i64>,
    numeric: NumericCtx<'_>,
) -> Option<ExprNode> {
    let num = |n: &ExprNode| node_provably_numeric(n, numeric);
    match op {
        BinOp::BitAnd => {
            if lit_right == Some(0) && num(left) {
                return Some(make_int_literal(0));
            }
            if lit_left == Some(0) && num(right) {
                return Some(make_int_literal(0));
            }
            None
        }
        BinOp::BitOr | BinOp::BitXor => {
            if lit_right == Some(0) && num(left) {
                Some(left.clone())
            } else if lit_left == Some(0) && num(right) {
                Some(right.clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Logical reductions for `&&` / `||` (O110).
///
/// Absorbing cases collapse to a constant boolean: `x && 0 → 0`,
/// `x || 1 → 1`. The identity cases (`x && 1`, `x || 0`) are subtler —
/// Tcl's `&&`/`||` return the normalised boolean (`0`/`1`), **not** the
/// operand value (`expr {2 && 1}` is `1`, not `2`), so the operand can be
/// returned bare only when its result is already a `0`/`1` boolean or is
/// consumed in a boolean context (where truthiness suffices). Otherwise it
/// is wrapped as `!!x` to preserve the `0`/`1` normalisation. Mirrors the
/// `&&`/`||` identity arms of Python's `_simplify_expr_node`.
fn reduce_logical(
    op: BinOp,
    left: &ExprNode,
    right: &ExprNode,
    lit_left: Option<i64>,
    lit_right: Option<i64>,
    bool_context: bool,
) -> Option<ExprNode> {
    // `x && y` where y is a non-`1` literal keeps absorbing behaviour;
    // the `WordAnd`/`WordOr` spellings share the same semantics.
    match op {
        BinOp::And | BinOp::WordAnd => {
            if lit_right == Some(0) || lit_left == Some(0) {
                return Some(make_int_literal(0));
            }
            if lit_right == Some(1) {
                return Some(normalise_bool(left, bool_context));
            }
            if lit_left == Some(1) {
                return Some(normalise_bool(right, bool_context));
            }
            None
        }
        BinOp::Or | BinOp::WordOr => {
            if lit_right == Some(1) || lit_left == Some(1) {
                return Some(make_int_literal(1));
            }
            if lit_right == Some(0) {
                return Some(normalise_bool(left, bool_context));
            }
            if lit_left == Some(0) {
                return Some(normalise_bool(right, bool_context));
            }
            None
        }
        _ => None,
    }
}

/// Return `node` bare when it already yields a `0`/`1` boolean (or its
/// truthiness is all the surrounding `bool_context` needs); otherwise wrap
/// it as `!!node` so the logical operator's normalised result is preserved.
/// Mirrors Python's `_boolify` / `_is_boolean_expr` gate.
fn normalise_bool(node: &ExprNode, bool_context: bool) -> ExprNode {
    if bool_context || is_boolean_expr(node) {
        node.clone()
    } else {
        boolify(node)
    }
}

/// Wrap `node` in `!!node` to canonicalise it to a `0`/`1` result.
fn boolify(node: &ExprNode) -> ExprNode {
    use crate::expr_ast::UnaryOp;
    ExprNode::Unary {
        op: UnaryOp::Not,
        operand: Box::new(ExprNode::Unary {
            op: UnaryOp::Not,
            operand: Box::new(node.clone()),
        }),
    }
}

/// Whether `node` is known to produce a boolean (`0`/`1`) result — a
/// comparison / logical operator, a logical negation, or a boolean
/// literal. Mirrors Python's `_is_boolean_expr` / `_BOOLEAN_OPS`.
fn is_boolean_expr(node: &ExprNode) -> bool {
    use crate::expr_ast::UnaryOp;
    match node {
        ExprNode::Binary { op, .. } => matches!(
            op,
            BinOp::And
                | BinOp::Or
                | BinOp::WordAnd
                | BinOp::WordOr
                | BinOp::Eq
                | BinOp::Ne
                | BinOp::Lt
                | BinOp::Le
                | BinOp::Gt
                | BinOp::Ge
                | BinOp::StrEq
                | BinOp::StrNe
                | BinOp::StrLt
                | BinOp::StrLe
                | BinOp::StrGt
                | BinOp::StrGe
                | BinOp::In
                | BinOp::Ni
                | BinOp::Contains
                | BinOp::StartsWith
                | BinOp::EndsWith
                | BinOp::StrEquals
                | BinOp::MatchesGlob
                | BinOp::MatchesRegex
        ),
        ExprNode::Unary { op, .. } => matches!(op, UnaryOp::Not | UnaryOp::WordNot),
        ExprNode::Literal { text, .. } => {
            let t = text.trim();
            t == "0"
                || t == "1"
                || matches!(
                    t.to_ascii_lowercase().as_str(),
                    "true" | "false" | "yes" | "no" | "on" | "off"
                )
        }
        _ => false,
    }
}

/// One pass of eq/ne-promotion: rewrite `==` / `!=` to `eq` / `ne`
/// **only when at least one operand is provably non-numeric**.
///
/// D5-O120 soundness gate. Tcl's `==`/`!=` parse *both* operands as a
/// number first and only fall through to a string compare when at least
/// one parse fails — so the promotion is sound iff one operand can never
/// be a number. A bare string literal is not enough: `$x == "1"` must
/// stay numeric (`"1"` parses as the integer 1), or the rewrite flips the
/// result when `$x` is numeric. We require a string literal whose
/// delimiter-stripped text does **not** parse as a number, mirroring
/// Python's `_is_provably_non_numeric_expr_node`. The variable-with-SCCP-
/// CONST refinement Python also accepts needs lattice values not threaded
/// here, so it is conservatively skipped (a missed rewrite, never an
/// unsound one).
fn streq_promote_node(node: &ExprNode) -> Option<ExprNode> {
    let ExprNode::Binary { op, left, right } = node else {
        return None;
    };
    let new_op = match op {
        BinOp::Eq => BinOp::StrEq,
        BinOp::Ne => BinOp::StrNe,
        _ => return None,
    };
    if !node_provably_non_numeric(left) && !node_provably_non_numeric(right) {
        return None;
    }
    Some(ExprNode::Binary {
        op: new_op,
        left: left.clone(),
        right: right.clone(),
    })
}

/// Whether `node` is provably **not** a number for `expr` — the dual of
/// [`node_provably_numeric`], used to gate the eq/ne string-compare
/// promotion (O120). Only a string literal whose stripped text is neither
/// numeric nor a boolean word qualifies; everything else (variables, command
/// substitutions, arithmetic) is conservatively rejected. Mirrors
/// `_is_provably_non_numeric_expr_node` minus the SCCP-CONST variable case.
fn node_provably_non_numeric(node: &ExprNode) -> bool {
    matches!(node, ExprNode::String { text, .. } if !is_numeric_or_boolean_string(text))
}

/// Whether the (delimiter-stripped) text parses as a number **or** is one of
/// Tcl's boolean words (`true`/`false`/`yes`/`no`/`on`/`off`,
/// case-insensitive). Mirrors Python's `_is_numeric_string_value`, which `==`
/// promotion treats as "could still be a number" and so refuses to promote.
/// Kept separate from [`is_numeric_string`] (used by the arithmetic-identity
/// gate, where a boolean word is *not* a valid arithmetic operand).
fn is_numeric_or_boolean_string(text: &str) -> bool {
    if is_numeric_string(text) {
        return true;
    }
    let stripped = text
        .trim()
        .trim_start_matches(['"', '{'])
        .trim_end_matches(['"', '}'])
        .trim();
    matches!(
        stripped.to_ascii_lowercase().as_str(),
        "true" | "false" | "yes" | "no" | "on" | "off"
    )
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

    #[test]
    fn arith_identity_numeric_guard() {
        // No type context (`None`) → legacy aggressive behaviour: drop.
        for e in [
            "$x * 0", "$x + 0", "$x * 1", "$x - 0", "$x / 1", "$x % 1", "$x << 0",
        ] {
            let (_, changed) = instcombine_expr(e, false);
            assert!(changed, "None ctx must still simplify {e:?}");
        }
        // With a type context that does NOT prove `$x` numeric → keep the
        // operand (dropping it would hide a coercion error). Mirrors Python.
        let empty: HashSet<String> = HashSet::new();
        for e in ["$x * 0", "$x + 0", "$x * 1", "$x % 1", "$x << 0"] {
            let (out, changed) = instcombine_expr_typed(e, false, Some(&empty));
            assert!(!changed, "non-numeric `$x` must keep {e:?}, got {out:?}");
        }
        // With `$x` proven numeric → the identity fires again, matching Python.
        let mut numeric = HashSet::new();
        numeric.insert("x".to_owned());
        let (out, changed) = instcombine_expr_typed("$x * 0", false, Some(&numeric));
        assert!(
            changed && out.trim() == "0",
            "numeric `$x * 0` → 0, got {out:?}"
        );
        let (out, changed) = instcombine_expr_typed("$x + 0", false, Some(&numeric));
        assert!(
            changed && out.trim() == "$x",
            "numeric `$x + 0` → $x, got {out:?}"
        );
    }

    #[test]
    fn numeric_string_literal_is_provably_numeric() {
        // An SCCP-inlined numeric constant arrives as a string literal.
        assert!(is_numeric_string("42"));
        assert!(is_numeric_string("\"3.5\""));
        assert!(!is_numeric_string("\"abc\""));
        assert!(!is_numeric_string(""));
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

    #[test]
    fn streq_promotion_with_numeric_string_literal_is_unsound_noop() {
        // D5-O120: `$x == "1"` must stay numeric. `"1"` parses as a
        // number, so Tcl runs the numeric compare; promoting to `eq`
        // would flip the result when `$x` is numeric (e.g. `1.0`).
        // `"1"`/`"3.5"` are numeric; `"yes"` is a Tcl boolean word that
        // `==` still treats as number-ish — none may promote.
        for input in ["$x == \"1\"", "$x != \"3.5\"", "$x == \"yes\""] {
            let (out, changed) = try_eq_ne_string_compare_simplify_expr(input);
            assert!(!changed, "{input:?} should not promote to eq/ne");
            assert_eq!(out, input);
        }
    }

    #[test]
    fn streq_promotion_with_nonnumeric_string_literal() {
        // At least one operand is a provably non-numeric string literal,
        // so the string-compare path is guaranteed — promotion is sound.
        for input in ["$x == \"foo\"", "\"a\" != $y", "$x == \"\""] {
            let (_out, changed) = try_eq_ne_string_compare_simplify_expr(input);
            assert!(changed, "{input:?} should promote to eq/ne");
        }
    }

    // -- instcombine_expr (composite) --------------------------------------

    #[test]
    fn o110_logical_identity_boolifies_outside_bool_context() {
        // `$x && 1` at expression-value position must normalise to `0`/`1`,
        // not the operand value — so it becomes `!!$x`, never bare `$x`.
        let (out, changed) = instcombine_expr("$x && 1", false);
        assert!(changed);
        assert_eq!(out.trim(), "!!$x");
        let (out, changed) = instcombine_expr("$x || 0", false);
        assert!(changed);
        assert_eq!(out.trim(), "!!$x");
    }

    #[test]
    fn o110_logical_identity_drops_in_bool_context() {
        // In a boolean context the truthiness is all that is consumed, so
        // `$x && 1` collapses to bare `$x`.
        let (out, changed) = instcombine_expr("$x && 1", true);
        assert!(changed);
        assert_eq!(out.trim(), "$x");
    }

    #[test]
    fn o110_logical_identity_keeps_boolean_operand_bare() {
        // `($a < $b) && 1` — the operand is already a `0`/`1` comparison,
        // so it stays bare even outside a boolean context.
        let (out, changed) = instcombine_expr("$a < $b && 1", false);
        assert!(changed);
        assert_eq!(out.trim(), "$a < $b");
    }

    #[test]
    fn o110_logical_absorbing_still_folds() {
        // The absorbing cases must keep folding to a constant.
        assert_eq!(instcombine_expr("$x && 0", false).0.trim(), "0");
        assert_eq!(instcombine_expr("$x || 1", false).0.trim(), "1");
    }

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

    #[test]
    fn o110_reassociates_constant_chains() {
        // Additive and multiplicative constant reassociation (O110).
        for (input, want) in [
            ("$a + 1 + 2", "$a + 3"),
            ("$a * 2 * 3", "$a * 6"),
            ("$a + 3 - 1", "$a + 2"),
            ("$a - 1 - 2", "$a - 3"),
            ("5 + $a - 2", "$a + 3"),
            ("$a + $b + 1 + 2", "$a + $b + 3"),
            ("$a + 1 - 1", "$a"),
        ] {
            let (out, changed) = instcombine_expr(input, false);
            assert!(changed, "expected a rewrite for {input:?}");
            assert_eq!(out.trim(), want, "for {input:?}");
        }
    }

    #[test]
    fn o110_skips_pure_reorder_and_unsafe_drops() {
        // No additive chain to flatten → the reassociation does not fire
        // (a bare `1 + $a` reorder is left to other passes / suppressed).
        assert!(
            reassociate_node(&parse_expr("1 + $a", None)).is_none(),
            "reassociation must not fire on a non-chain reorder",
        );
        // `* 0` annihilation across a chain needs a numeric proof this
        // AST-level pass cannot make, so the reassociation abstains (the
        // result, if any, comes from the separate identity pass, not here).
        let parsed = parse_expr("$a * 0 * 3", None);
        assert!(
            reassociate_node(&parsed).is_none(),
            "reassociation must not annihilate `* 0` without a numeric proof",
        );
    }

    #[test]
    fn instcombine_whitespace_only_rerender_is_not_a_change() {
        // SYNC-MAY31-1e (#498): a re-render differing from the input
        // only in whitespace (`$x<0` → `$x < 0`) is canonical-spacing
        // noise, not an O110 finding. The guard strips all whitespace
        // before comparing, so the spacing-only re-render reports no
        // change.
        let (out, changed) = instcombine_expr("$x<0", false);
        assert!(
            !changed,
            "whitespace-only re-render must not count as changed (got {out:?})",
        );
        // The narrow no-op case (input already canonical) also reports
        // no change — unchanged from before the guard.
        let (_, unchanged) = instcombine_expr("$x + $y", false);
        assert!(!unchanged);
        // A real structural change (identity fold) still fires.
        let (reduced, structural) = instcombine_expr("$x * 1", false);
        assert!(
            structural,
            "identity fold `$x * 1` → `$x` must still register as changed",
        );
        assert_eq!(reduced.trim(), "$x");
    }

    #[test]
    fn strip_ws_removes_all_whitespace() {
        assert_eq!(strip_ws("$x < 0"), "$x<0");
        assert_eq!(strip_ws("  a  b\tc\n"), "abc");
        assert_eq!(strip_ws("nospace"), "nospace");
    }
}
