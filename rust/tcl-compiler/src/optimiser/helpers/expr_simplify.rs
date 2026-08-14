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

//! Expression-simplification helpers.
//!
//! A toolkit of AST-level expression rewriters consumed by the
//! propagation, branch-folding, and pattern-recognition passes:
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
//! The four AST rewriters are wired into
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
use crate::depth_guard::MAX_EXPR_NODE_DEPTH;
use crate::expr_ast::{BinOp, ExprNode, ExprOffset, render_expr};
use crate::expr_parser::parse_expr;
use crate::naming::normalise_var_name;
use crate::tcl_expr_eval::{
    Env, eval_tcl_expr_with_octal_and_dialect, format_tcl_value, leading_zero_is_octal,
};
use crate::types::{TclType, TypeKind, TypeLattice};

/// Operand type facts for the current function: which variable names are
/// provably *numeric* (Int / Double / Numeric / Boolean) and which are provably
/// *integer* (Int). The integer set is a subset of the numeric set.
///
/// The two are distinct because they gate different rewrites. An identity that
/// keeps its operand under an operator that also accepts doubles (`$x + 0`,
/// `$x * 1`) only needs the operand *numeric*. But an identity that either
/// folds to an **integer literal** by dropping an operand (`$x * 0 → 0`,
/// `$x ** 0 → 1`, `$x - $x → 0`) or removes an **integer-only** operator
/// (`<<`, `>>`, `&`, `|`, `^`, `%`) needs the operand *integer*: for a
/// double operand Tcl yields a double (`1.5 * 0` → `0.0`, not `0`) or raises
/// "can't use floating-point value as operand" — either way the integer-result
/// / operator-drop rewrite would be wrong.
#[derive(Debug, Default, Clone)]
pub struct OperandTypes {
    numeric: HashSet<String>,
    integer: HashSet<String>,
}

#[cfg(test)]
impl OperandTypes {
    /// A context proving `names` numeric but *not* integer (Double-typed).
    fn numeric_only(names: &[&str]) -> Self {
        Self {
            numeric: names.iter().map(|s| (*s).to_owned()).collect(),
            integer: HashSet::new(),
        }
    }

    /// A context proving `names` integer (and hence numeric).
    fn integer(names: &[&str]) -> Self {
        let set: HashSet<String> = names.iter().map(|s| (*s).to_owned()).collect();
        Self {
            numeric: set.clone(),
            integer: set,
        }
    }
}

/// A type context for the current function, or `None` when no type lattice is
/// available. Passed to the `*_typed` entry points so the operand-dropping
/// identities fire only on provably typed operands. `None` keeps the historical
/// aggressive behaviour for callers (and tests) that have no type lattice.
pub type NumericCtx<'a> = Option<&'a OperandTypes>;

/// Build the [`OperandTypes`] for `fu`: a name is numeric (resp. integer) when
/// **every** SSA version of it is a known numeric (resp. integer) type. A name
/// absent from a set is treated as not provably that type, so the corresponding
/// identity is kept. Using the function-level join (all versions must agree) is
/// a sound over-approximation of the proper per-use check.
#[must_use]
pub fn operand_types(fu: &FunctionUnit) -> OperandTypes {
    use std::collections::HashMap;
    // symbol → (all-versions-numeric, all-versions-integer).
    let mut acc: HashMap<crate::ssa::Symbol, (bool, bool)> = HashMap::new();
    for ((sym, _ver), lattice) in fu.types.iter() {
        let is_num = lattice_is_numeric(lattice);
        let is_int = lattice_is_integer(lattice);
        acc.entry(*sym)
            .and_modify(|v| {
                v.0 = v.0 && is_num;
                v.1 = v.1 && is_int;
            })
            .or_insert((is_num, is_int));
    }
    let mut out = OperandTypes::default();
    for (sym, (is_num, is_int)) in acc {
        let name = fu.ssa.var_name(sym).to_owned();
        if is_num {
            out.numeric.insert(name.clone());
        }
        if is_int {
            out.integer.insert(name);
        }
    }
    out
}

/// Whether a type-lattice element is a known numeric Tcl type.
fn lattice_is_numeric(t: &TypeLattice) -> bool {
    t.kind() == TypeKind::Known
        && matches!(
            t.tcl_type(),
            Some(TclType::Int | TclType::Double | TclType::Numeric | TclType::Boolean)
        )
}

/// Whether a type-lattice element is a known *integer* Tcl type. `Double` and
/// the catch-all `Numeric` (which may be a double) are excluded; `Boolean` is
/// excluded too, since a boolean-typed operand may hold the word `true`, which
/// is not an integer in `expr` (`expr {true << 0}` errors).
fn lattice_is_integer(t: &TypeLattice) -> bool {
    t.kind() == TypeKind::Known && matches!(t.tcl_type(), Some(TclType::Int))
}

/// Whether `node` is provably numeric for `expr` arithmetic — so dropping it
/// from an identity rewrite cannot hide Tcl's numeric-coercion error.
/// With no type context (`None`) every node is assumed numeric, preserving
/// the legacy behaviour for callers without a lattice.
fn node_provably_numeric(node: &ExprNode, numeric: NumericCtx<'_>) -> bool {
    let Some(ctx) = numeric else {
        return true;
    };
    match node {
        ExprNode::Literal { .. } => true,
        ExprNode::String { text, .. } => is_numeric_string_in_every_release(text),
        ExprNode::Var { name, .. } => ctx.numeric.contains(name.as_str()),
        _ => false,
    }
}

/// Whether `node` is provably an *integer* for `expr` arithmetic — so folding
/// it to an integer literal, or dropping an integer-only operator around it,
/// matches Tcl's result and error behaviour. With no type context (`None`)
/// every node is assumed integer, preserving the legacy aggressive behaviour.
fn node_provably_integer(node: &ExprNode, numeric: NumericCtx<'_>) -> bool {
    let Some(ctx) = numeric else {
        return true;
    };
    match node {
        ExprNode::Literal { text, .. } | ExprNode::String { text, .. } => is_integer_string(text),
        ExprNode::Var { name, .. } => ctx.integer.contains(name.as_str()),
        _ => false,
    }
}

/// Whether `node`'s value provably cannot be the IEEE NaN — the precondition
/// of [`BinOp::inverse`]'s ordered-comparison rows and of the `$x == $x`
/// self-comparison folds.
///
/// Tcl follows C's rule (`tclExecute.c`): with a NaN operand `!=` is true and
/// every other comparison is false. That breaks both `!(a < b) == (a >= b)`
/// and `$x == $x == 1`, so those rewrites need this proof.
///
/// Proof sources, deliberately narrow:
///
/// * a literal or string whose text is not a NaN spelling under **any** release
///   — either a non-NaN number, or not a number at all (a plain string can no
///   more be NaN than it can be `inf`);
/// * a variable the type lattice proves `Int`. *Numeric* is not enough: the
///   numeric set also holds `Double` and the catch-all `Numeric`, and a double
///   may perfectly well be NaN.
///
/// Everything else — a command substitution, an arithmetic subexpression, any
/// variable with no type context at all — is unproven, so the caller abstains.
/// Unlike [`node_provably_numeric`] / [`node_provably_integer`] this does *not*
/// fall back to "assume the best" when the context is `None`: a caller with no
/// lattice has proved nothing, and the rewrites this gates are unsound without
/// the proof.
fn node_cannot_be_nan(node: &ExprNode, numeric: NumericCtx<'_>) -> bool {
    match node {
        ExprNode::Literal { text, .. } | ExprNode::String { text, .. } => {
            !is_nan_string_in_any_release(text)
        }
        ExprNode::Var { name, .. } => {
            numeric.is_some_and(|ctx| ctx.integer.contains(name.as_str()))
        }
        _ => false,
    }
}

/// Whether the (delimiter-stripped) text spells NaN under **any** release —
/// the sound direction for a gate that *refuses* a rewrite because a value
/// "could still be NaN", mirroring [`is_numeric_string_in_any_release`].
fn is_nan_string_in_any_release(text: &str) -> bool {
    use tcl_syntax::number::{Number, ParseFlags};
    tcl_syntax::number::NumberSyntax::any(|n| {
        matches!(
            tcl_syntax::number::parse_whole_with(
                strip_literal_delims(text),
                ParseFlags::for_syntax(n),
            ),
            Some(Number::Nan { .. })
        )
    })
}

/// Strip the surrounding `"…"` / `{…}` delimiters (if any) from an `expr`
/// literal or a propagated constant's value text, for the numeric
/// classifiers below.
fn strip_literal_delims(text: &str) -> &str {
    text.trim()
        .trim_start_matches(['"', '{'])
        .trim_end_matches(['"', '}'])
        .trim()
}

/// Whether the (delimiter-stripped) text of an `expr` string literal or a
/// propagated constant's value parses as a Tcl number — the SCCP-inlined-
/// constant case. Backed by the shared `tcl_syntax::number` grammar (the
/// same one the const-folder and [`is_integer_string`] use) rather than
/// Rust's own `str::parse`, which rejects Tcl's `0x`/`0o`/`0b` prefixes and
/// `_` digit separators — a hex/octal/binary constant is a real Tcl number
/// and must classify as one here, or two callers (the O120 eq/ne string-
/// compare promotion gate and the O100/O101 constant-substitution binder)
/// wrongly treat it as "provably not a number".
/// [`is_numeric_string`] under one release's grammar, or — with `numbers`
/// [`None`] — under whichever grammar this process was built for.
fn is_numeric_string_under(text: &str, numbers: Option<tcl_syntax::number::NumberSyntax>) -> bool {
    use tcl_syntax::number::{Number, ParseFlags};
    let t = strip_literal_delims(text);
    let flags = numbers.map_or_else(ParseFlags::default, ParseFlags::for_syntax);
    !t.is_empty()
        && matches!(
            tcl_syntax::number::parse_whole_with(t, flags),
            Some(Number::Int(_) | Number::Big { .. } | Number::Double(_) | Number::Nan { .. })
        )
}

/// Whether `text` is a number under **every** release — the sound reading of
/// "provably numeric" for an optimiser gate that has no target in hand.
///
/// A release-dependent spelling is not *provable*: `08` is a number from 9.0 and
/// an invalid octal before it, so a rewrite that requires a numeric operand must
/// not fire on it. Requiring unanimity keeps the gate correct whichever release
/// the program is eventually built for.
fn is_numeric_string_in_every_release(text: &str) -> bool {
    tcl_syntax::number::NumberSyntax::every(|n| is_numeric_string_under(text, Some(n)))
}

/// Whether `text` is a number under **any** release — the sound reading for a
/// gate that *refuses* an optimisation because a value "could still be a
/// number". Here the permissive direction is the safe one: a spelling numeric on
/// even one release must block the rewrite.
fn is_numeric_string_in_any_release(text: &str) -> bool {
    tcl_syntax::number::NumberSyntax::any(|n| is_numeric_string_under(text, Some(n)))
}

/// Whether the (delimiter-stripped) text of an `expr` literal parses as a Tcl
/// *integer* (decimal / `0x` / `0o` / `0b`, any magnitude — a bignum is still
/// an integer). A float literal (`1.5`) is not. Uses the shared number grammar
/// so it agrees with the const-folder.
fn is_integer_string(text: &str) -> bool {
    use tcl_syntax::number::{Number, ParseFlags};
    // Unanimous across releases, for the same reason as
    // [`is_numeric_string_in_every_release`]: this gates rewrites that need a
    // genuine integer operand, and a release-dependent spelling is not proof.
    tcl_syntax::number::NumberSyntax::every(|n| {
        matches!(
            tcl_syntax::number::parse_whole_with(
                strip_literal_delims(text),
                ParseFlags::for_syntax(n),
            ),
            Some(Number::Int(_) | Number::Big { .. })
        )
    })
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
    let value = eval_tcl_expr_with_octal_and_dialect(
        &node,
        &env,
        dialect.and_then(leading_zero_is_octal),
        dialect,
    )?;
    let rendered = format_tcl_value(&value);
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
        // A dialect is in hand here (it drove `parse_expr` above), so bind under
        // the release actually being compiled for rather than the ambient.
        if braced
            || is_numeric_string_under(
                value,
                Some(tcl_dialect::NumberSyntax::of_profile(Some(
                    tcl_dialect::DialectProfile::by_opt_name(dialect),
                ))),
            )
        {
            env.insert(name.clone(), EnvValue::Str(value.clone()));
        }
    }
    let value = eval_tcl_expr_with_octal_and_dialect(
        &node,
        &env,
        dialect.and_then(leading_zero_is_octal),
        dialect,
    )?;
    let rendered = format_tcl_value(&value);
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

/// Whether `text` is safe to inline as a bare (unquoted) token when
/// substituting a constant into `expr` text. Delegates to
/// [`is_numeric_string`] — the same Tcl-number grammar, not a locally
/// re-implemented Rust-`str::parse` approximation.
fn is_numeric_literal(text: &str) -> bool {
    is_numeric_string_in_every_release(text)
}

// instcombine / strength-reduce / strlen / streq

/// InstCombine-style fixpoint simplification of an
/// expression text.
///
/// Parses `expr`, runs the AST simplifier until fixpoint, then
/// renders the result back to text. Returns `(new_text, changed)`
/// where `changed` indicates whether the rendered output differs
/// from `expr.trim()`. Unparseable inputs and expressions
/// containing command substitutions are returned unchanged.
///
/// The fixpoint composes all the simplifiers —
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
    // Suppress O110 noise. When the canonical re-render differs from the
    // input only in whitespace (e.g. `$x<0` → `$x < 0`), it is a spacing
    // preference, not a real finding — report no change. Structural
    // rewrites (paren removal, identity folds, operand reordering, …) all
    // alter non-whitespace characters and still register as changed.
    let changed = strip_ws(&rendered) != strip_ws(trimmed);
    (rendered, changed)
}

/// Return `expr` with all whitespace removed — the whitespace-insensitive
/// comparison key for the O110 noise guard.
fn strip_ws(expr: &str) -> String {
    expr.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Apply one pass of local simplifications to `node`, returning
/// the rewritten subtree. Used as the step function in
/// [`simplify_to_fixpoint`].
fn simplify_node_once(
    node: &ExprNode,
    bool_context: bool,
    numeric: NumericCtx<'_>,
    depth: u32,
) -> ExprNode {
    use crate::expr_ast::UnaryOp;
    // Native-stack safety net (issue #996): this bottom-up rewriter recurses
    // once per `ExprNode` level. Past the cap, pass the node through
    // unchanged (no rewrite) rather than recurse — a safe no-op for a
    // simplifier, and the same shape it returns for any node it can't rewrite.
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return node.clone();
    }
    // First, recurse into children — bottom-up rewriting. The boolean
    // context propagates only where the operand's *value* is consumed as a
    // truth value: the operands of `&&`/`||`/`!` and a ternary condition.
    // Comparison/arithmetic operands consume the operand's full value, so
    // they reset the context to `false`.
    let lowered = match node {
        ExprNode::Binary { op, left, right } => {
            let child_bool = matches!(op, BinOp::And | BinOp::Or | BinOp::WordAnd | BinOp::WordOr);
            ExprNode::Binary {
                op: *op,
                left: Box::new(simplify_node_once(left, child_bool, numeric, depth + 1)),
                right: Box::new(simplify_node_once(right, child_bool, numeric, depth + 1)),
            }
        }
        ExprNode::Unary { op, operand } => {
            let child_bool = matches!(op, UnaryOp::Not | UnaryOp::WordNot);
            ExprNode::Unary {
                op: *op,
                operand: Box::new(simplify_node_once(operand, child_bool, numeric, depth + 1)),
            }
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => ExprNode::Ternary {
            condition: Box::new(simplify_node_once(condition, true, numeric, depth + 1)),
            true_branch: Box::new(simplify_node_once(
                true_branch,
                bool_context,
                numeric,
                depth + 1,
            )),
            false_branch: Box::new(simplify_node_once(
                false_branch,
                bool_context,
                numeric,
                depth + 1,
            )),
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
/// `$a * 2 * 3` → `$a * 6`.
///
/// Fires only when one operand is itself an additive (resp. multiplicative)
/// chain — a pure operand reorder (`1 + $a` → `$a + 1`) is suppressed as
/// noise. All non-constant terms are preserved, so numeric-coercion error
/// semantics are unchanged. The term-dropping cases that *would* need a
/// provably-numeric guard — annihilating `* 0`, dropping a lone `* 1`, and a
/// lone additive term whose constant cancels to zero (`$a + 5 - 5`) — are
/// skipped: proving numericity needs the SSA type lattice, which this
/// AST-level pass cannot consult, so it conservatively leaves them be.
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
            let constant = collect_add_terms(node, &mut terms, 0)?;
            if constant == i64::MIN {
                return None; // `-constant` would overflow in the builder
            }
            // Conservative: a lone term whose additive constant cancels to zero
            // (`$a + 5 - 5`) would emit `$a` BARE — stripping the numeric-
            // coercion error `$a` must raise when non-numeric (`expr {$a}`
            // returns the string; `expr {$a + 5 - 5}` errors). Proving `$a`
            // integer needs the SSA type lattice this AST pass can't consult,
            // so mirror the multiplicative guard (`$a * 1`) and leave it be..
            if constant == 0 && terms.len() == 1 {
                return None;
            }
            let built = build_add_expr(&terms, constant);
            (render_expr(&built) != render_expr(node)).then_some(built)
        }
        BinOp::Mul => {
            if !is_mul(left) && !is_mul(right) {
                return None;
            }
            let mut terms = Vec::new();
            let constant = collect_mul_terms(node, &mut terms, 0)?;
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
/// integer literal (otherwise the whole node is an opaque term — a
/// non-literal subtrahend is not negated here). `None` on integer overflow.
fn collect_add_terms(node: &ExprNode, terms: &mut Vec<ExprNode>, depth: u32) -> Option<i64> {
    // Native-stack safety net (issue #996): past the cap, stop flattening and
    // treat the whole remaining subtree as one opaque term contributing the
    // additive identity — the same handling as any non-chain leaf, so the
    // reassociation stays sound (all non-constant terms preserved).
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        terms.push(node.clone());
        return Some(0);
    }
    if let ExprNode::Binary { op, left, right } = node {
        match op {
            BinOp::Add => {
                let l = collect_add_terms(left, terms, depth + 1)?;
                let r = collect_add_terms(right, terms, depth + 1)?;
                return l.checked_add(r);
            }
            BinOp::Sub => {
                if let Some(rhs) = int_literal_value(right) {
                    let l = collect_add_terms(left, terms, depth + 1)?;
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
fn collect_mul_terms(node: &ExprNode, terms: &mut Vec<ExprNode>, depth: u32) -> Option<i64> {
    // Native-stack safety net (issue #996): past the cap, stop flattening and
    // treat the remaining subtree as one opaque term contributing the
    // multiplicative identity — same handling as any non-chain leaf.
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        terms.push(node.clone());
        return Some(1);
    }
    if let ExprNode::Binary {
        op: BinOp::Mul,
        left,
        right,
    } = node
    {
        let l = collect_mul_terms(left, terms, depth + 1)?;
        let r = collect_mul_terms(right, terms, depth + 1)?;
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
        let next = simplify_node_once(&cur, bool_context, numeric, 0);
        if render_expr(&next) == render_expr(&cur) {
            return next;
        }
        cur = next;
    }
    cur
}

/// Strength-reduce a single expression text.
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

/// Simplify `[string length $s] == 0` → `$s eq ""` and
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

/// Promote numeric `==` / `!=` against a quoted-string
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
    // non-`0`/`1` value into `0`/`1` (`expr {!!2}` is `1`).
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
    // `BinOp::inverse()` (tcl_syntax::expr::operators, issue #983's
    // unification) is the single source for which comparison inverts to
    // which — used to be a local 8-arm match missing the TIP 461
    // string-ordering four (`lt`/`le`/`gt`/`ge`) and list membership
    // (`in`/`ni`), so `!(x lt y)`/`!(x in list)` never simplified even
    // though the same total-order/negation identity holds for them as for
    // the numeric/string-eq forms already covered.
    //
    // The four *ordered numeric* rows (`<`/`<=`/`>`/`>=`) carry a NaN
    // precondition (`BinOp::inverse_needs_non_nan`): `expr {!(NaN < 1)}` is 1
    // but `expr {NaN >= 1}` is 0, so they fire only when both operands are
    // proved non-NaN. `==`/`!=` are exact complements even for NaN, and the
    // string / membership operators have no NaN rule, so those stay
    // unconditional (issue #1437).
    if matches!(op, UnaryOp::Not | UnaryOp::WordNot)
        && let ExprNode::Binary {
            op: inner_op,
            left,
            right,
        } = operand
    {
        let nan_safe = !inner_op.inverse_needs_non_nan()
            || (node_cannot_be_nan(left, numeric) && node_cannot_be_nan(right, numeric));
        if nan_safe && let Some(new_op) = inner_op.inverse() {
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
    if let Some(result) = reduce_self_comparison(op, left, right, numeric) {
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
///
/// The *reflexive* rows (`==`, `<=`, `>=` → 1 and `!=` → 0) additionally need
/// `$x` proved non-NaN — see [`node_cannot_be_nan`].
fn reduce_self_comparison(
    op: BinOp,
    left: &ExprNode,
    right: &ExprNode,
    numeric: NumericCtx<'_>,
) -> Option<ExprNode> {
    let (ExprNode::Var { name: l, .. }, ExprNode::Var { name: r, .. }) = (left, right) else {
        return None;
    };
    if l != r {
        return None;
    }
    let k = match op {
        // Comparison operators have a string fallback in `expr` (`$x == $x`,
        // `$x < $x`, `$x eq $x` never raise), so these fold for any defined $x.
        // `$x < $x` / `$x > $x` are false for *every* value, NaN included (a
        // NaN operand makes every ordered comparison false), and string
        // identity has no NaN notion at all.
        BinOp::Lt | BinOp::Gt | BinOp::StrNe => 0,
        BinOp::StrEq => 1,
        // The reflexive folds are the ones NaN breaks: `expr {NaN == NaN}` is
        // 0, `expr {NaN <= NaN}` is 0, and `expr {NaN != NaN}` is 1 — the
        // opposite of every answer below. So they need $x proved non-NaN
        // (issue #1437); without the proof the expression stays as written.
        BinOp::Eq | BinOp::Le | BinOp::Ge if node_cannot_be_nan(left, numeric) => 1,
        BinOp::Ne if node_cannot_be_nan(left, numeric) => 0,
        // `$x - $x` / `$x ^ $x` fold to the *integer* literal 0, so `$x` must
        // be a provable integer, not merely numeric: for a double `$x`,
        // `expr {1.5 - 1.5}` is `0.0` (not `0`), and `^` is integer-only
        // (`expr {1.5 ^ 1.5}` errors). Folding to `0` in either case would be
        // wrong. A non-numeric `$x` still errors, which the
        // integer gate also (conservatively) declines to fold.
        BinOp::Sub | BinOp::BitXor if node_provably_integer(left, numeric) => 0,
        _ => return None,
    };
    Some(make_int_literal(k))
}

/// `+/-/*//` arithmetic identities and annihilators.
///
/// Every rewrite here removes an arithmetic operation, so the surviving
/// expression must still raise Tcl's numeric-coercion error iff the original
/// would. Identities that **keep** the operand under a double-accepting
/// operator (`x + 0`, `x - 0`, `x * 1`, `x / 1`) are gated on the operand being
/// provably *numeric*. The `x * 0 → 0` annihilator folds to an **integer**
/// literal, so it is gated on the operand being provably *integer*: for a
/// double `$x`, `expr {1.5 * 0}` is `0.0`, not `0`. Without a
/// type context both guards pass (legacy aggressive behaviour).
fn reduce_arith_identity(
    op: BinOp,
    left: &ExprNode,
    right: &ExprNode,
    lit_left: Option<i64>,
    lit_right: Option<i64>,
    numeric: NumericCtx<'_>,
) -> Option<ExprNode> {
    let num = |n: &ExprNode| node_provably_numeric(n, numeric);
    let int = |n: &ExprNode| node_provably_integer(n, numeric);
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
            // Annihilation folds to the integer literal 0 — needs integer, not
            // just numeric (a double operand yields `0.0`).
            if lit_right == Some(0) && int(left) {
                return Some(make_int_literal(0));
            }
            if lit_left == Some(0) && int(right) {
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
/// `** 0` folds to the integer literal 1, so `x` must be provably *integer*
/// (a double base yields `1.0`). `** 1 → x` keeps `x` under an operator that
/// accepts doubles, so *numeric* suffices. `** 2 → x * x` keeps `x` on both
/// sides, preserving error semantics without a guard.
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
        // `x ** 0 → 1` folds to the integer literal 1 — needs integer, not just
        // numeric (`expr {1.5 ** 0}` is `1.0`). `x ** 1 → x` keeps `x`, and
        // `**` accepts doubles, so numeric suffices there.
        0 if node_provably_integer(left, numeric) => Some(make_int_literal(1)),
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
/// `%` is integer-only, so `% 1 → 0` drops `x` *and* folds to an integer: `x`
/// must be provably *integer* (`expr {1.5 % 1}` errors — folding to `0` would
/// hide it). The pow2 strength-reduction keeps `x` under `&`
/// (also integer-only), so a double `x` errors either way — error semantics
/// preserved without a guard.
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
    if n == 1 && node_provably_integer(left, numeric) {
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

/// `x << 0 → x`, `x >> 0 → x`. The shift is integer-only, so `x` must be
/// provably *integer*: for a double `x`, `expr {1.5 << 0}` errors, while `$x`
/// alone returns the double — dropping the shift would hide the error.
fn reduce_shift(
    op: BinOp,
    left: &ExprNode,
    lit_right: Option<i64>,
    numeric: NumericCtx<'_>,
) -> Option<ExprNode> {
    if !matches!(op, BinOp::LShift | BinOp::RShift) {
        return None;
    }
    if lit_right == Some(0) && node_provably_integer(left, numeric) {
        Some(left.clone())
    } else {
        None
    }
}

/// Bitwise identities / annihilators: `x & 0 → 0`, `x | 0 → x`, `x ^ 0 → x`.
/// `&`/`|`/`^` are integer-only, so each rewrite (folding to `0`, or stripping
/// the operator around `x`) needs the non-literal operand provably *integer* —
/// a double operand is a Tcl error the rewrite must not hide.
fn reduce_bitwise(
    op: BinOp,
    left: &ExprNode,
    right: &ExprNode,
    lit_left: Option<i64>,
    lit_right: Option<i64>,
    numeric: NumericCtx<'_>,
) -> Option<ExprNode> {
    let int = |n: &ExprNode| node_provably_integer(n, numeric);
    match op {
        BinOp::BitAnd => {
            if lit_right == Some(0) && int(left) {
                return Some(make_int_literal(0));
            }
            if lit_left == Some(0) && int(right) {
                return Some(make_int_literal(0));
            }
            None
        }
        BinOp::BitOr | BinOp::BitXor => {
            if lit_right == Some(0) && int(left) {
                Some(left.clone())
            } else if lit_left == Some(0) && int(right) {
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
/// is wrapped as `!!x` to preserve the `0`/`1` normalisation.
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
/// literal.
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
            // Full spellings only, deliberately conservative for rewrite
            // safety — the canonical vocabulary, not a private word list.
            t == "0" || t == "1" || tcl_syntax::boolean::is_boolean_full_word(t)
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
/// delimiter-stripped text does **not** parse as a number to prove an
/// operand non-numeric. The variable-with-SCCP-CONST refinement needs
/// lattice values not threaded here, so it is conservatively skipped (a
/// missed rewrite, never an unsound one).
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
/// substitutions, arithmetic) is conservatively rejected.
fn node_provably_non_numeric(node: &ExprNode) -> bool {
    matches!(node, ExprNode::String { text, .. } if !is_numeric_or_boolean_string(text))
}

/// Whether the (delimiter-stripped) text parses as a number **or** is one of
/// Tcl's boolean words (`true`/`false`/`yes`/`no`/`on`/`off`,
/// case-insensitive). The `==` promotion treats such a value as "could still
/// be a number" and so refuses to promote.
/// Kept separate from [`is_numeric_string`] (used by the arithmetic-identity
/// gate, where a boolean word is *not* a valid arithmetic operand).
fn is_numeric_or_boolean_string(text: &str) -> bool {
    // The permissive direction: this gate refuses a promotion when the value
    // could still be a number on some release.
    if is_numeric_string_in_any_release(text) {
        return true;
    }
    let stripped = text
        .trim()
        .trim_start_matches(['"', '{'])
        .trim_end_matches(['"', '}'])
        .trim();
    tcl_syntax::boolean::is_boolean_full_word(stripped)
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
/// subtree.
#[must_use]
pub fn expr_has_command_subst(node: &ExprNode) -> bool {
    // Public entry: the top of an expression tree is nesting depth 0 (issue
    // #996 — the recursion cap lives in [`expr_has_command_subst_at`]).
    expr_has_command_subst_at(node, 0)
}

fn expr_has_command_subst_at(node: &ExprNode, depth: u32) -> bool {
    // Native-stack safety net (issue #996): past the cap, assume "yes, has a
    // command substitution" — the conservative direction, since callers use
    // this to *suppress* an optimisation when a command sub is present, so a
    // false `true` only forgoes a rewrite, never enables an unsound one.
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return true;
    }
    match node {
        ExprNode::Command { .. } => true,
        ExprNode::Binary { left, right, .. } => {
            expr_has_command_subst_at(left, depth + 1)
                || expr_has_command_subst_at(right, depth + 1)
        }
        ExprNode::Unary { operand, .. } => expr_has_command_subst_at(operand, depth + 1),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            expr_has_command_subst_at(condition, depth + 1)
                || expr_has_command_subst_at(true_branch, depth + 1)
                || expr_has_command_subst_at(false_branch, depth + 1)
        }
        ExprNode::Call { args, .. } => args.iter().any(|a| expr_has_command_subst_at(a, depth + 1)),
        ExprNode::Literal { .. }
        | ExprNode::Var { .. }
        | ExprNode::Raw { .. }
        | ExprNode::String { .. } => false,
    }
}

/// Return `true` when `node` invokes a Tcl math function (`abs(...)`,
/// `max(...)`, …) whose name is shadowed by a user-defined
/// `::tcl::mathfunc::<name>` proc anywhere in the module.
///
/// Math functions are not `CommandSpec`s — they live in the shared
/// `tcl_syntax::expr::mathfunc` dispatch table the const-folder and the
/// runtime both consume — so there is no registry purity/redefinition fact
/// to consult the way [`crate::command_binding::ModuleCommandMutations`]
/// covers ordinary commands. The module's own `proc` definitions are the
/// only source of truth: real Tcl compiles `abs(x)` to a `tcl::mathfunc::abs`
/// command invocation and only falls back to the C builtin when nothing
/// shadows it, so folding `abs(-5)` to `5` is unsound whenever
/// `::tcl::mathfunc::abs` has been (re)defined.
#[must_use]
pub fn expr_uses_shadowed_mathfunc<S: std::hash::BuildHasher>(
    node: &ExprNode,
    procedures: &std::collections::HashMap<String, crate::ir::Procedure, S>,
) -> bool {
    // Public entry: the top of an expression tree is nesting depth 0 (issue
    // #996 — the recursion cap lives in [`expr_uses_shadowed_mathfunc_at`]).
    expr_uses_shadowed_mathfunc_at(node, procedures, 0)
}

fn expr_uses_shadowed_mathfunc_at<S: std::hash::BuildHasher>(
    node: &ExprNode,
    procedures: &std::collections::HashMap<String, crate::ir::Procedure, S>,
    depth: u32,
) -> bool {
    // Native-stack safety net (issue #996): past the cap, assume "yes, uses a
    // shadowed mathfunc" — the conservative direction, since callers use this
    // to *suppress* constant folding when a mathfunc may be shadowed, so a
    // false `true` only forgoes a fold, never performs an unsound one.
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return true;
    }
    match node {
        ExprNode::Call { function, args, .. } => {
            let key = format!("::tcl::mathfunc::{}", function.to_ascii_lowercase());
            procedures.contains_key(&key)
                || args
                    .iter()
                    .any(|a| expr_uses_shadowed_mathfunc_at(a, procedures, depth + 1))
        }
        ExprNode::Binary { left, right, .. } => {
            expr_uses_shadowed_mathfunc_at(left, procedures, depth + 1)
                || expr_uses_shadowed_mathfunc_at(right, procedures, depth + 1)
        }
        ExprNode::Unary { operand, .. } => {
            expr_uses_shadowed_mathfunc_at(operand, procedures, depth + 1)
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            expr_uses_shadowed_mathfunc_at(condition, procedures, depth + 1)
                || expr_uses_shadowed_mathfunc_at(true_branch, procedures, depth + 1)
                || expr_uses_shadowed_mathfunc_at(false_branch, procedures, depth + 1)
        }
        ExprNode::Literal { .. }
        | ExprNode::Var { .. }
        | ExprNode::Raw { .. }
        | ExprNode::String { .. }
        | ExprNode::Command { .. } => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // try_fold_expr

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

    /// Regression coverage for issue #996: `simplify_node_once`,
    /// `collect_add_terms`, `collect_mul_terms`, `expr_has_command_subst` and
    /// `expr_uses_shadowed_mathfunc` each recurse once per `ExprNode` level
    /// with no depth cap before this fix. The Pratt parser caps *its* output
    /// at 256 levels, but a tree built directly is unbounded and empirically
    /// overflowed the native stack (SIGABRT) in the low thousands of levels
    /// on a 2 MiB thread (`cargo test`'s default). 3000 is comfortably past
    /// that crash range and past `MAX_EXPR_NODE_DEPTH` (256); the assertion is
    /// that each walker returns at all.
    #[test]
    fn deeply_nested_expr_simplify_walks_survive() {
        use crate::expr_ast::UnaryOp;

        fn var_x() -> ExprNode {
            ExprNode::Var {
                text: "$x".into(),
                name: "x".into(),
                start: 0,
                end: 2,
            }
        }

        // A 3000-deep nested-unary `!` tree — drives `simplify_node_once`
        // (its `reduce_unary`/`reassociate_node` helpers never render or eval
        // this Unary shape, so this isolates the walker's own recursion) and
        // the two boolean predicates.
        let mut unary = var_x();
        for _ in 0..3000 {
            unary = ExprNode::Unary {
                op: UnaryOp::Not,
                operand: Box::new(unary),
            };
        }
        let _ = simplify_node_once(&unary, false, None, 0);
        assert!(expr_has_command_subst(&ExprNode::Command {
            text: "[x]".into(),
            start: 0,
            end: 3,
        }));
        // Deep tree with no command subst / no mathfunc → the walkers descend
        // fully (capped) and answer `false` for realistic input; the point is
        // they do not overflow.
        let _ = expr_has_command_subst(&unary);
        let procs: std::collections::HashMap<String, crate::ir::Procedure> =
            std::collections::HashMap::new();
        let _ = expr_uses_shadowed_mathfunc(&unary, &procs);

        // A 3000-deep left-nested `+` chain drives `collect_add_terms`.
        let mut add = var_x();
        for _ in 0..3000 {
            add = ExprNode::Binary {
                op: BinOp::Add,
                left: Box::new(add),
                right: Box::new(make_int_literal(1)),
            };
        }
        let mut terms = Vec::new();
        let _ = collect_add_terms(&add, &mut terms, 0);

        // A 3000-deep left-nested `*` chain drives `collect_mul_terms`.
        let mut mul = var_x();
        for _ in 0..3000 {
            mul = ExprNode::Binary {
                op: BinOp::Mul,
                left: Box::new(mul),
                right: Box::new(make_int_literal(2)),
            };
        }
        let mut mterms = Vec::new();
        let _ = collect_mul_terms(&mul, &mut mterms, 0);
    }

    // try_unwrap_expr_in_expr

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

    // substitute_expr_constants

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

    // expr_has_command_subst

    #[test]
    fn command_subst_detection() {
        let expr = parse_expr("[foo] + 1", None);
        assert!(expr_has_command_subst(&expr));
        let expr = parse_expr("$x + 1", None);
        assert!(!expr_has_command_subst(&expr));
        let expr = parse_expr("1 + 2", None);
        assert!(!expr_has_command_subst(&expr));
    }

    // try_strength_reduce_expr

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
        // operand (dropping it would hide a coercion error).
        let empty = OperandTypes::default();
        for e in ["$x * 0", "$x + 0", "$x * 1", "$x % 1", "$x << 0"] {
            let (out, changed) = instcombine_expr_typed(e, false, Some(&empty));
            assert!(!changed, "non-numeric `$x` must keep {e:?}, got {out:?}");
        }
        // With `$x` proven numeric (but not integer) → identities that KEEP the
        // operand fire (`$x + 0` → `$x`), but the integer-result annihilator
        // (`$x * 0` → `0`) does NOT (a double `$x` yields `0.0`).
        let numeric = OperandTypes::numeric_only(&["x"]);
        let (out, changed) = instcombine_expr_typed("$x + 0", false, Some(&numeric));
        assert!(
            changed && out.trim() == "$x",
            "numeric `$x + 0` → $x, got {out:?}"
        );
        let (out, changed) = instcombine_expr_typed("$x * 0", false, Some(&numeric));
        assert!(
            !changed,
            "numeric-but-not-integer `$x * 0` must NOT fold to 0, got {out:?}"
        );
        // With `$x` proven integer → the integer-result / integer-only rewrites
        // fire.
        let integer = OperandTypes::integer(&["x"]);
        let (out, changed) = instcombine_expr_typed("$x * 0", false, Some(&integer));
        assert!(
            changed && out.trim() == "0",
            "integer `$x * 0` → 0, got {out:?}"
        );
    }

    #[test]
    fn integer_only_ops_need_integer_not_just_numeric() {
        // `<<`/`>>`/`&`/`|`/`^`/`%`/`**0`/`*0`/`$x-$x` are wrong
        // for a Double-typed operand — they either yield a double where an
        // integer literal is produced, or are integer-only ops Tcl rejects on a
        // float. A numeric-but-not-integer context must decline them all.
        let double = OperandTypes::numeric_only(&["x"]);
        for e in [
            "$x << 0", "$x >> 0", "$x & 0", "$x | 0", "$x ^ 0", "$x % 1", "$x ** 0", "$x * 0",
            "$x - $x", "$x ^ $x",
        ] {
            let (out, changed) = instcombine_expr_typed(e, false, Some(&double));
            assert!(
                !changed,
                "double `$x`: {e:?} must not be rewritten, got {out:?}"
            );
        }
        // An integer context fires them (spot-check a representative set).
        let integer = OperandTypes::integer(&["x"]);
        for (e, want) in [
            ("$x << 0", "$x"),
            ("$x & 0", "0"),
            ("$x % 1", "0"),
            ("$x ** 0", "1"),
            ("$x - $x", "0"),
        ] {
            let (out, changed) = instcombine_expr_typed(e, false, Some(&integer));
            assert!(
                changed && out.trim() == want,
                "integer `$x`: {e:?} → {want:?}, got {out:?}"
            );
        }
    }

    /// Issue #1437: `!($x < 1)` → `$x >= 1` is wrong when `$x` may be NaN
    /// (`expr {!(NaN < 1)}` is 1, `expr {NaN >= 1}` is 0), so the four ordered
    /// comparisons invert only on operands proved NaN-free.
    #[test]
    fn ordered_comparison_inversion_needs_non_nan_operands() {
        // No proof available — with no type context at all, and with a context
        // that knows nothing about `$x`.
        let empty = OperandTypes::default();
        for e in ["!($x < 1)", "!($x <= 1)", "!($x > 1)", "!($x >= 1)"] {
            let (out, changed) = instcombine_expr(e, false);
            assert!(!changed, "untyped `$x`: {e:?} must not invert, got {out:?}");
            let (out, changed) = instcombine_expr_typed(e, false, Some(&empty));
            assert!(
                !changed,
                "unproven `$x`: {e:?} must not invert, got {out:?}"
            );
        }
        // A provably-integer `$x` cannot be NaN, so the inversion fires.
        let integer = OperandTypes::integer(&["x"]);
        for (e, want) in [
            ("!($x < 1)", "$x >= 1"),
            ("!($x <= 1)", "$x > 1"),
            ("!($x > 1)", "$x <= 1"),
            ("!($x >= 1)", "$x < 1"),
        ] {
            let (out, changed) = instcombine_expr_typed(e, false, Some(&integer));
            assert!(
                changed && out.trim() == want,
                "integer `$x`: {e:?} → {want:?}, got {out:?}"
            );
        }
        // A `Double`-typed operand is numeric but may still be NaN.
        let double = OperandTypes::numeric_only(&["x"]);
        let (out, changed) = instcombine_expr_typed("!($x < 1)", false, Some(&double));
        assert!(!changed, "double `$x`: must not invert, got {out:?}");
        // A NaN *literal* is no proof either, in any spelling.
        for e in ["!(NaN < 1)", "!(nan < 1)", "!(1 < NaN)"] {
            let (out, changed) = instcombine_expr_typed(e, false, Some(&empty));
            assert!(!changed, "{e:?} must not invert, got {out:?}");
        }
        // Two ordinary numeric literals are proof enough.
        let (out, changed) = instcombine_expr_typed("!(2 < 1)", false, Some(&empty));
        assert!(changed && out.trim() == "2 >= 1", "got {out:?}");
    }

    /// `==`/`!=` are exact complements even for NaN, and the string / membership
    /// operators never compare numerically — those rows keep inverting with no
    /// type facts at all (issue #1437).
    #[test]
    fn equality_and_string_comparison_inversion_stays_unconditional() {
        let empty = OperandTypes::default();
        for (e, want) in [
            ("!($x == $y)", "$x != $y"),
            ("!($x != $y)", "$x == $y"),
            ("!($x eq $y)", "$x ne $y"),
            ("!($x ne $y)", "$x eq $y"),
            ("!($x lt $y)", "$x ge $y"),
            ("!($x in $y)", "$x ni $y"),
            ("!($x ni $y)", "$x in $y"),
        ] {
            let (out, changed) = instcombine_expr_typed(e, false, Some(&empty));
            assert!(
                changed && out.trim() == want,
                "{e:?} → {want:?}, got {out:?}"
            );
        }
    }

    /// Issue #1437: `$x == $x` is 0 and `$x != $x` is 1 for a NaN `$x`, so the
    /// reflexive folds need the same non-NaN proof. `$x < $x` / `$x > $x` are
    /// false for every value including NaN, and `eq`/`ne` have no NaN notion, so
    /// those keep folding unconditionally.
    #[test]
    fn reflexive_self_comparison_folds_need_non_nan() {
        let empty = OperandTypes::default();
        for e in ["$x == $x", "$x != $x", "$x <= $x", "$x >= $x"] {
            let (out, changed) = instcombine_expr(e, false);
            assert!(!changed, "untyped `$x`: {e:?} must not fold, got {out:?}");
            let (out, changed) = instcombine_expr_typed(e, false, Some(&empty));
            assert!(!changed, "unproven `$x`: {e:?} must not fold, got {out:?}");
        }
        // Always-false and string-identity rows fold with no proof.
        for (e, want) in [
            ("$x < $x", "0"),
            ("$x > $x", "0"),
            ("$x eq $x", "1"),
            ("$x ne $x", "0"),
        ] {
            let (out, changed) = instcombine_expr_typed(e, false, Some(&empty));
            assert!(
                changed && out.trim() == want,
                "{e:?} → {want:?}, got {out:?}"
            );
        }
        // A provably-integer `$x` unlocks the reflexive rows.
        let integer = OperandTypes::integer(&["x"]);
        for (e, want) in [
            ("$x == $x", "1"),
            ("$x != $x", "0"),
            ("$x <= $x", "1"),
            ("$x >= $x", "1"),
        ] {
            let (out, changed) = instcombine_expr_typed(e, false, Some(&integer));
            assert!(
                changed && out.trim() == want,
                "integer `$x`: {e:?} → {want:?}, got {out:?}"
            );
        }
        // A `Double` is numeric but not NaN-free.
        let double = OperandTypes::numeric_only(&["x"]);
        let (out, changed) = instcombine_expr_typed("$x == $x", false, Some(&double));
        assert!(!changed, "double `$x == $x` must not fold, got {out:?}");
    }

    /// The NaN gate's proof sources, direct.
    #[test]
    fn node_cannot_be_nan_proof_sources() {
        let integer = OperandTypes::integer(&["i"]);
        let double = OperandTypes::numeric_only(&["d"]);
        let node = |src: &str| parse_expr(src, None);
        assert!(node_cannot_be_nan(&node("1"), Some(&integer)));
        assert!(node_cannot_be_nan(&node("1.5"), Some(&integer)));
        assert!(node_cannot_be_nan(&node("\"abc\""), Some(&integer)));
        assert!(node_cannot_be_nan(&node("$i"), Some(&integer)));
        assert!(!node_cannot_be_nan(&node("$d"), Some(&double)));
        assert!(!node_cannot_be_nan(&node("$i"), None));
        assert!(!node_cannot_be_nan(&node("NaN"), Some(&integer)));
        assert!(!node_cannot_be_nan(&node("[f]"), Some(&integer)));
        assert!(is_nan_string_in_any_release("NaN"));
        assert!(is_nan_string_in_any_release("-nan"));
        assert!(!is_nan_string_in_any_release("42"));
        assert!(!is_nan_string_in_any_release("banana"));
    }

    #[test]
    fn numeric_string_literal_is_provably_numeric() {
        // An SCCP-inlined numeric constant arrives as a string literal.
        assert!(is_numeric_string_in_every_release("42"));
        assert!(is_numeric_string_in_every_release("\"3.5\""));
        assert!(!is_numeric_string_in_every_release("\"abc\""));
        assert!(!is_numeric_string_in_every_release(""));
    }

    /// The gate reads through the shared Tcl grammar, not Rust's — `0x1a` is a
    /// number here where `str::parse::<i64>/<f64>` rejects it outright.
    ///
    /// It asks the *provable* question, though, so only spellings every release
    /// agrees on qualify. `0x1a` is hex in all of them; `0o17`/`0b101` arrive in
    /// 8.5 and `1_000` in 9.0, so none of those three is provably a number
    /// without a target in hand — and this predicate gates rewrites that need a
    /// genuine numeric operand. The permissive question has its own predicate,
    /// [`is_numeric_string_in_any_release`], asserted below.
    #[test]
    fn is_numeric_string_recognises_non_decimal_tcl_numbers() {
        // Universal in every release — and rejected by Rust's own parsers.
        for text in ["0x1a", "\"0x1a\""] {
            assert!(
                is_numeric_string_in_every_release(text),
                "{text:?} should be provably numeric"
            );
        }
        // Real Tcl numbers, but only from 8.5 / 9.0 — not provable without a
        // target, and each *is* numeric under the release that has it.
        for (text, first) in [
            ("0o17", tcl_syntax::number::NumberSyntax::Tcl85),
            ("0b101", tcl_syntax::number::NumberSyntax::Tcl85),
            ("1_000", tcl_syntax::number::NumberSyntax::Tcl90),
        ] {
            assert!(
                !is_numeric_string_in_every_release(text),
                "{text:?} is not provable across releases"
            );
            assert!(
                is_numeric_string_under(text, Some(first)),
                "{text:?} is numeric under {first:?}"
            );
            assert!(
                is_numeric_string_in_any_release(text),
                "{text:?} is numeric on some release, so it blocks a promotion"
            );
        }
        // TN control: still rejects genuine non-numbers, both ways.
        for text in ["hello", "0xzz"] {
            assert!(!is_numeric_string_in_every_release(text), "{text:?}");
            assert!(!is_numeric_string_in_any_release(text), "{text:?}");
        }
    }

    #[test]
    fn is_numeric_literal_agrees_with_is_numeric_string() {
        // is_numeric_literal (the substitute_expr_constants bare-vs-quoted
        // gate) now delegates to the same grammar, so a hex constant
        // inlines bare instead of being needlessly quoted.
        assert!(is_numeric_literal("0x1a"));
        assert!(!is_numeric_literal("hello"));
    }

    #[test]
    fn hex_string_literal_is_not_wrongly_promoted_to_streq() {
        // FP guard: `"0x1a" == $y` must NOT promote to `eq` (O120) — 0x1a
        // is a genuine Tcl number (tclsh: `expr {"0x1a" == 26}` -> 1, a
        // numeric compare), so treating it as "provably non-numeric" would
        // silently turn a numeric comparison into a string comparison.
        // Before the fix, `is_numeric_string_in_every_release("0x1a")` was false, so
        // `node_provably_non_numeric` wrongly returned true for this
        // literal and the eq/ne promotion fired.
        let (out, changed) = try_eq_ne_string_compare_simplify_expr("\"0x1a\" == $y");
        assert!(
            !changed,
            "hex literal must not be promoted to string eq, got {out:?}"
        );
    }

    // try_strlen_simplify_expr

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

    // try_eq_ne_string_compare_simplify_expr

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

    // instcombine_expr (composite)

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
            // `$a + $b + 1 - 1` still folds the constant: two terms survive, so
            // `$a + $b` keeps a `+` that coerces both operands.
            ("$a + $b + 1 - 1", "$a + $b"),
        ] {
            let (out, changed) = instcombine_expr(input, false);
            assert!(changed, "expected a rewrite for {input:?}");
            assert_eq!(out.trim(), want, "for {input:?}");
        }
    }

    #[test]
    fn o110_additive_lone_term_cancel_to_zero_abstains() {
        // `$a + 1 - 1` cancels to a lone bare `$a`, dropping the
        // numeric-coercion error `$a + 1 - 1` raises for a non-numeric `$a`.
        // Mirror the multiplicative `$a * 1` guard and abstain (the AST pass
        // has no numeric proof).
        assert!(
            reassociate_node(&parse_expr("$a + 1 - 1", None)).is_none(),
            "lone additive term cancelling to zero must not fold to a bare term",
        );
        let (out, changed) = instcombine_expr("$a + 1 - 1", false);
        assert!(!changed, "`$a + 1 - 1` must be left unchanged, got {out:?}");
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
        // A re-render differing from the input
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
