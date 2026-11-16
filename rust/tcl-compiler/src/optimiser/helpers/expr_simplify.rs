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
//! - [`try_fold_expr`] — constant-fold an expression text on the shared
//!   expression route, the one the lattice runs
//!   ([`crate::value_transfer::evaluate_expression_detached`]).
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

use tcl_lexer::ExprTokenType;

use crate::compilation_unit::FunctionUnit;
use crate::depth_guard::MAX_EXPR_NODE_DEPTH;
use crate::expr_ast::{BinOp, ExprNode, ExprOffset, render_expr};
use crate::expr_parser::parse_expr_for_profile;
use crate::sccp::BuiltinFoldInputs;
use crate::tcl_expr_eval::FoldPolicy;
use crate::tcl_expr_eval::leading_zero_is_octal;
use crate::types::{TclType, TypeKind, TypeLattice};
use tcl_registry::value_transfer::ExactValue;

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
    original_advice: std::collections::HashMap<tcl_lexer::Span, (HashSet<String>, HashSet<String>)>,
}

#[cfg(test)]
impl OperandTypes {
    /// A context proving `names` numeric but *not* integer (Double-typed).
    fn numeric_only(names: &[&str]) -> Self {
        Self {
            numeric: names.iter().map(|s| format!("${s}")).collect(),
            integer: HashSet::new(),
            ..Self::default()
        }
    }

    /// A context proving `names` integer (and hence numeric).
    fn integer(names: &[&str]) -> Self {
        let set: HashSet<String> = names.iter().map(|s| format!("${s}")).collect();
        Self {
            numeric: set.clone(),
            integer: set,
            ..Self::default()
        }
    }
}

/// A type context for the current function, or `None` when no type lattice is
/// available. Passed to the `*_typed` entry points so the operand-dropping
/// identities fire only on provably typed operands. `None` keeps the
/// unguarded, aggressive behaviour for callers (and tests) that have no type
/// lattice.
pub type NumericCtx<'a> = Option<&'a OperandTypes>;

/// Build conditional type consensus for original expression operands. Every
/// occurrence with the same written reference must have an exact retained read
/// and agree on the required type. These facts only select rewrite candidates;
/// the native execution-equivalence gate independently licenses an edit.
#[must_use]
pub fn operand_types(fu: &FunctionUnit) -> OperandTypes {
    use std::collections::HashMap;
    let mut operands: HashMap<String, (bool, bool)> = HashMap::new();
    for (&block, body) in &fu.cfg.blocks {
        for (index, statement) in body.statements.iter().enumerate() {
            let (expression, base) = match statement {
                crate::ir::Statement::AssignExpr {
                    expr, expr_base, ..
                }
                | crate::ir::Statement::ExprEval {
                    expr, expr_base, ..
                }
                | crate::ir::Statement::Return {
                    expr: Some(expr),
                    expr_base,
                    ..
                } => (expr, *expr_base),
                _ => continue,
            };
            record_operand_types(
                expression,
                base,
                crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index),
                fu,
                &mut operands,
            );
        }
        let (expression, base) = match body.terminator.as_ref() {
            Some(crate::cfg::Terminator::Branch {
                condition,
                condition_base,
                ..
            }) => (condition, *condition_base),
            Some(crate::cfg::Terminator::Return {
                expr: Some(expr),
                expr_base,
                ..
            }) => (expr, *expr_base),
            _ => continue,
        };
        record_operand_types(
            expression,
            base,
            crate::ssa::SsaSourceView::at_terminator(&fu.ssa, block),
            fu,
            &mut operands,
        );
    }
    let mut out = OperandTypes::default();
    for (text, (numeric, integer)) in operands {
        if numeric {
            out.numeric.insert(text.clone());
        }
        if integer {
            out.integer.insert(text);
        }
    }
    out
}

fn record_operand_types(
    expression: &ExprNode,
    base: Option<u32>,
    view: crate::ssa::SsaSourceView<'_>,
    fu: &FunctionUnit,
    operands: &mut std::collections::HashMap<String, (bool, bool)>,
) {
    for node in expression.variable_nodes() {
        let ExprNode::Var { text, .. } = node else {
            continue;
        };
        let lattice = view
            .read_expression_variable(node, base)
            .and_then(|read| fu.types.get(&(read.symbol, read.version?)));
        let numeric = lattice.is_some_and(lattice_is_numeric);
        let integer = lattice.is_some_and(lattice_is_integer);
        operands
            .entry(text.clone())
            .and_modify(|value| {
                value.0 &= numeric;
                value.1 &= integer;
            })
            .or_insert((numeric, integer));
    }
}

/// Retain separately scoped candidate types for original expression children.
/// The exact site map is consumed only by no-edit advice; normal rewriters keep
/// the represented-read type sets from `operand_types`.
pub fn operand_types_with_original_advice(
    fu: &FunctionUnit,
    registry: &tcl_registry::CommandRegistry,
) -> OperandTypes {
    let mut out = operand_types(fu);
    for (&block, body) in &fu.ssa.blocks {
        for index in 0..body.statements.len() {
            let Some(tokens) =
                crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index).source_tokens()
            else {
                continue;
            };
            let Some(binding) = tokens.source_binding.as_ref() else {
                continue;
            };
            let Some(reads) = binding.declaration_expression_reads(registry, tokens) else {
                continue;
            };
            let Some(dialect) = binding
                .declaration_operand_layout_advice(tokens)
                .map(|advice| advice.dialect())
            else {
                continue;
            };
            let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
            let mut calls = crate::word_subst::lifted_calls(Some(tokens), config);
            let Some(child) = calls.pop().and_then(|call| call.tokens) else {
                continue;
            };
            let Some(advice) =
                crate::registry_invocation::original_expression_operand_advice(registry, &child)
            else {
                continue;
            };
            let mut typed = std::collections::HashMap::<String, (bool, bool)>::new();
            for read in reads {
                let lattice = read
                    .diagnostic_version(&fu.ssa, block, index, registry)
                    .and_then(|key| fu.types.get(&key));
                let numeric = lattice.is_some_and(lattice_is_numeric);
                let integer = lattice.is_some_and(lattice_is_integer);
                typed
                    .entry(read.spelling().to_owned())
                    .and_modify(|facts| {
                        facts.0 &= numeric;
                        facts.1 &= integer;
                    })
                    .or_insert((numeric, integer));
            }
            let mut numeric = HashSet::new();
            let mut integer = HashSet::new();
            for (spelling, facts) in typed {
                if facts.0 {
                    numeric.insert(spelling.clone());
                }
                if facts.1 {
                    integer.insert(spelling);
                }
            }
            out.original_advice.insert(advice.span, (numeric, integer));
        }
    }
    out
}

impl OperandTypes {
    pub(crate) fn original_candidate_context(&self, site: tcl_lexer::Span) -> Option<Self> {
        let (numeric, integer) = self.original_advice.get(&site)?;
        Some(Self {
            numeric: numeric.clone(),
            integer: integer.clone(),
            ..Self::default()
        })
    }
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
/// With no type context (`None`) every node is assumed numeric.
fn node_provably_numeric(node: &ExprNode, numeric: NumericCtx<'_>) -> bool {
    let Some(ctx) = numeric else {
        return true;
    };
    match node {
        ExprNode::Literal { .. } => true,
        ExprNode::String { text, .. } => {
            fixed_operand_text(text).is_some_and(is_numeric_string_in_every_release)
        }
        ExprNode::Var { text, .. } => ctx.numeric.contains(text.as_str()),
        _ => false,
    }
}

/// Whether `node` is provably an *integer* for `expr` arithmetic — so folding
/// it to an integer literal, or dropping an integer-only operator around it,
/// matches Tcl's result and error behaviour. With no type context (`None`)
/// every node is assumed integer.
fn node_provably_integer(node: &ExprNode, numeric: NumericCtx<'_>) -> bool {
    let Some(ctx) = numeric else {
        return true;
    };
    match node {
        ExprNode::Literal { text, .. } => is_integer_string(text),
        ExprNode::String { text, .. } => fixed_operand_text(text).is_some_and(is_integer_string),
        ExprNode::Var { text, .. } => ctx.integer.contains(text.as_str()),
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
        ExprNode::Literal { text, .. } => !is_nan_string_in_any_release(text),
        ExprNode::String { text, .. } => {
            fixed_operand_text(text).is_some_and(|value| !is_nan_string_in_any_release(value))
        }
        ExprNode::Var { text, .. } => {
            numeric.is_some_and(|ctx| ctx.integer.contains(text.as_str()))
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
/// A string operand's value when the proofs here may read it: `None` for one
/// that substitutes, whose value is unknown (`"$x" == 1` is a numeric compare
/// when `x` is `1.0`, and `"$x"` may be `NaN`), and for a value that itself
/// holds a quote or brace, which [`strip_literal_delims`] would misread
/// (#2227, found in review).
fn fixed_operand_text(text: &str) -> Option<&str> {
    tcl_syntax::expr::fixed_string_operand(text).filter(|value| !value.contains(['"', '{', '}']))
}

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

/// The value semantics a rewrite under `folds` evaluates with: the target's
/// leading-zero rule, grammar and tower.
fn rewrite_policy(folds: BuiltinFoldInputs<'_>) -> FoldPolicy {
    FoldPolicy::for_profile(folds.dialect.and_then(leading_zero_is_octal), folds.dialect)
}

/// `node`'s value on the shared expression route
/// ([`crate::value_transfer::evaluate_expression_detached`]) under `folds`,
/// rendered as its source text: the value the lattice would prove, so a
/// rewrite never folds what the lattice declines.
fn fold_on_route(
    node: &ExprNode,
    constants: &std::collections::HashMap<String, ExactValue>,
    folds: BuiltinFoldInputs<'_>,
) -> Option<String> {
    let value = crate::value_transfer::evaluate_expression_detached(
        node,
        constants,
        folds,
        rewrite_policy(folds),
    )?;
    String::from_utf8(value.bytes).ok()
}

/// Attempt to fold `expr` to a Tcl literal value with no variable bound.
///
/// Returns `Some(folded_text)` when the shared expression route answers a
/// value (the one the lattice proves under the same target and trust) and
/// its rendering differs from `expr.trim()`. Returns `None` when the
/// expression reads a variable or a command substitution, or when the route
/// declines — a domain error, a math function the target lacks or the
/// module rebinds, a result the target's tower does not hold.
#[must_use]
pub fn try_fold_expr(expr: &str, folds: BuiltinFoldInputs<'_>) -> Option<String> {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return None;
    }
    let node = parse_expr_for_profile(trimmed, folds.dialect);
    if matches!(node, ExprNode::Raw { .. }) {
        return None;
    }
    let rendered = fold_on_route(&node, &std::collections::HashMap::new(), folds)?;
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
///   are bound; a string-valued var is left unbound and the fold bails.
///
/// Returns the folded literal, or `None` when the expression still depends
/// on an unresolved operand / command substitution, or the shared route
/// declines.
#[must_use]
pub fn try_fold_expr_with_constants<S: std::hash::BuildHasher>(
    expr: &str,
    constants: &std::collections::HashMap<String, String, S>,
    braced: bool,
    folds: BuiltinFoldInputs<'_>,
) -> Option<String> {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return None;
    }
    let node = parse_expr_for_profile(trimmed, folds.dialect);
    if matches!(node, ExprNode::Raw { .. }) {
        return None;
    }
    // A dialect is in hand here (it drove `parse_expr` above), so bind under
    // the release actually being compiled for rather than the ambient.
    let numbers = tcl_dialect::NumberSyntax::of_profile(Some(
        folds
            .dialect
            .unwrap_or(tcl_dialect::DialectProfile::plain_tcl()),
    ));
    let bound: std::collections::HashMap<String, ExactValue> = constants
        .iter()
        .filter(|(_, value)| braced || is_numeric_string_under(value, Some(numbers)))
        .map(|(name, value)| (name.clone(), ExactValue::from_literal(value)))
        .collect();
    let rendered = fold_on_route(&node, &bound, folds)?;
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
/// substituted verbatim; other values use the shared configured literal
/// quoted-content owner, including inert substitution and brace-container
/// spelling. Actual expression/source transformation authority is separate.
#[must_use]
pub fn substitute_expr_constants<S: std::hash::BuildHasher>(
    expr: &str,
    constants: &std::collections::HashMap<String, String, S>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> SubstitutionResult {
    let profile = dialect.unwrap_or_else(|| tcl_dialect::DialectProfile::plain_tcl());
    substitute_expr_constants_in_context(
        expr,
        constants,
        &tcl_syntax::expr::parser::ExprParseContext::for_profile(profile),
    )
}

/// Project scalar value constants using the supplied complete expression
/// grammar. This source emitter supplies no native object/read or execution
/// erasure proof. Jim and hosted grammars need no legacy catalogue profile.
#[must_use]
pub fn substitute_expr_constants_in_context<S: std::hash::BuildHasher>(
    expr: &str,
    constants: &std::collections::HashMap<String, String, S>,
    context: &tcl_syntax::expr::parser::ExprParseContext,
) -> SubstitutionResult {
    let config = tcl_lexer::LexerConfig::from_grammar(context.lexer_grammar);
    let (tokens, _) = tcl_lexer::tokenise_expr_checked_with_expression_grammar(
        expr,
        &context.lexer_grammar,
        context.expr_grammar_base,
        context.f5_word_grammar,
    );
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
            let name = expr
                .get(start..end_excl)
                .and_then(|written| scalar_expression_reference_name(written, config));
            if let Some((name, value)) =
                name.and_then(|name| constants.get(name).map(|value| (name, value)))
            {
                if is_numeric_literal(value) {
                    pieces.push(value.clone());
                } else {
                    let Some(fragment) =
                        tcl_syntax::backslash::literal_quoted_source_fragment_in_braced_script(
                            value, config,
                        )
                    else {
                        pieces.push(tok.text.clone());
                        cursor = end_excl;
                        continue;
                    };
                    let literal = format!("\"{fragment}\"");
                    if tcl_syntax::expr::OriginalSourceExpression::capture(
                        &literal, context, config,
                    )
                    .is_none()
                    {
                        pieces.push(tok.text.clone());
                        cursor = end_excl;
                        continue;
                    }
                    pieces.push(literal);
                }
                changed = true;
                substituted.insert(name.to_owned());
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

/// Scalar constants need a complete selected reference, not an analytical
/// array root. Current constant maps retain no original element-key receipt.
fn scalar_expression_reference_name(written: &str, config: tcl_lexer::LexerConfig) -> Option<&str> {
    // naming.expression.scalar-reference-constant-source-projection
    // docs/design/analysis/name-resolution-proofs/expression-scalar-reference-constant-source-projection.md
    let reference = tcl_lexer::word_parts::whole_var_ref(written.as_bytes(), config).ok()??;
    if reference.index.is_some()
        || tcl_syntax::naming::split_element_ref_bytes(reference.name).is_some()
    {
        return None;
    }
    let root =
        tcl_syntax::naming::variable_reference_root_bytes(written.as_bytes(), config).ok()??;
    std::str::from_utf8(root).ok()
}

/// Substitute constants only with actual retained-operand execution evidence.
/// The mathematical helper above has no native-object provenance: its newly
/// parsed literals must never be used to erase conversions of the original
/// retained references. Missing evidence leaves those references unchanged.
#[must_use]
pub fn substitute_expr_constants_for_execution<S: std::hash::BuildHasher>(
    expr: &str,
    constants: &std::collections::HashMap<String, String, S>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    operands: &crate::tcl_expr_eval::NativeOperandProofs,
) -> SubstitutionResult {
    let selected = dialect.map(tcl_registry::InvocationDialect::of_profile);
    let proved = constants
        .iter()
        .filter(|(name, _)| {
            operands
                .get(*name)
                .is_some_and(|proof| Some(proof.dialect) == selected)
        })
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect::<std::collections::HashMap<_, _>>();
    if proved.is_empty() {
        return SubstitutionResult {
            text: expr.to_owned(),
            changed: false,
            substituted: HashSet::new(),
        };
    }
    let original = parse_expr_for_profile(expr, dialect);
    let env = constants
        .iter()
        .map(|(name, value)| {
            (
                name.clone(),
                crate::tcl_expr_eval::EnvValue::Str(value.clone()),
            )
        })
        .collect();
    let policy = crate::tcl_expr_eval::FoldPolicy::for_profile(
        dialect.and_then(leading_zero_is_octal),
        dialect,
    );
    if crate::tcl_expr_eval::eval_tcl_expr_with_proved_operands(
        &original,
        &env,
        policy,
        &|_, _| None,
        operands,
    )
    .is_none()
    {
        return SubstitutionResult {
            text: expr.to_owned(),
            changed: false,
            substituted: HashSet::new(),
        };
    }
    substitute_expr_constants(expr, &proved, dialect)
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
#[cfg(test)]
pub fn instcombine_expr(expr: &str, bool_context: bool) -> (String, bool) {
    instcombine_expr_typed(expr, bool_context, None, None)
}

/// Whether `proposed` is `original` for every operand value: the same result,
/// or the same error. Two licences, each from tclsh 8.6.18 and 9.0.4 run over
/// integers, doubles, `NaN`, words, boolean words and the empty string:
///
/// * any operands: `!(L == R)` and `L != R` (and the `eq`/`ne`, `in`/`ni`
///   pairs and the 9.0 string orderings, each way round), `!!C` and `C` for a
///   comparison `C`, and `L == "w"`
///   with `L eq "w"` for a word `w` that is no number. `!($a && $b)` with
///   `!$a || !$b` is not one: a word operand raises `expected boolean value`
///   in the first and `can't use non-numeric string as operand of "!"` in the
///   second. Nor is `!($a < 1)` with `$a >= 1`, which differ for `NaN`.
/// * integer operands only: every variable is typed an integer and every
///   literal is one, so no operand can raise a conversion error, and the two
///   expressions agree on a grid of integer values, zero and negatives
///   included, so a dropped division or shift is caught.
#[must_use]
pub fn structural_rewrite_equivalence(
    original: &ExprNode,
    proposed: &str,
    numeric: NumericCtx<'_>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> bool {
    structural_equivalence(original, proposed, numeric, dialect, false)
}

/// [`structural_rewrite_equivalence`] for a branch condition, where only the
/// truth of the result is observed.
#[must_use]
pub fn structural_condition_equivalence(
    original: &ExprNode,
    proposed: &str,
    numeric: NumericCtx<'_>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> bool {
    structural_equivalence(original, proposed, numeric, dialect, true)
}

fn structural_equivalence(
    original: &ExprNode,
    proposed: &str,
    numeric: NumericCtx<'_>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    truth_only: bool,
) -> bool {
    let Some(profile) = dialect else {
        return false;
    };
    // Measured on C Tcl only. Jim's numeric comparison rewrites its operand's
    // string, and the F5 dialects have no interpreter to run here.
    if !matches!(
        tcl_syntax::expr::parser::ExprParseContext::for_profile(profile).native_syntax,
        tcl_syntax::expr::parser::NativeExprSyntax::Tcl(_)
    ) {
        return false;
    }
    let proposed = parse_expr_for_profile(proposed, dialect);
    if matches!(proposed, ExprNode::Raw { .. }) {
        return false;
    }
    equivalent_for_any_operand(original, &proposed, profile)
        || equivalent_for_integer_operands(original, &proposed, numeric, profile, truth_only)
}

fn equivalent_for_any_operand(
    original: &ExprNode,
    proposed: &ExprNode,
    profile: &'static tcl_dialect::DialectProfile,
) -> bool {
    use crate::expr_ast::UnaryOp;
    match (original, proposed) {
        (
            ExprNode::Unary {
                op: UnaryOp::Not,
                operand,
            },
            _,
        ) => {
            negates(operand, proposed)
                || matches!(
                    operand.as_ref(),
                    ExprNode::Unary { op: UnaryOp::Not, operand: inner }
                        if truth_valued(inner) && same_text(inner, proposed)
                )
        }
        (
            ExprNode::Binary { op, left, right },
            ExprNode::Binary {
                op: string_op,
                left: new_left,
                right: new_right,
            },
        ) if matches!(
            (op, string_op),
            (BinOp::Eq, BinOp::StrEq) | (BinOp::Ne, BinOp::StrNe)
        ) =>
        {
            same_text(left, new_left)
                && same_text(right, new_right)
                && (word_is_no_number(left, profile) || word_is_no_number(right, profile))
        }
        // `&&` and `||` never evaluate the right operand once the left one
        // decides, so no read or conversion of it is lost.
        (
            ExprNode::Binary {
                op: op @ (BinOp::And | BinOp::Or),
                left,
                ..
            },
            ExprNode::Literal { text, .. },
        ) => integer_literal(left, profile).is_some_and(|decider| match op {
            BinOp::And => decider == 0 && text == "0",
            _ => decider != 0 && text == "1",
        }),
        // A constant condition selects one arm; the other is never evaluated
        // and the selected operand is converted the way a lone operand is.
        (
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            },
            selected,
        ) => integer_literal(condition, profile).is_some_and(|decider| {
            same_text(
                if decider == 0 {
                    false_branch
                } else {
                    true_branch
                },
                selected,
            )
        }),
        _ => false,
    }
}

fn same_text(left: &ExprNode, right: &ExprNode) -> bool {
    render_expr(left) == render_expr(right)
}

/// A comparison, or `&&`/`||` over comparisons: evaluating it cannot raise a
/// conversion error and its result is `0` or `1`.
fn truth_valued(node: &ExprNode) -> bool {
    match node {
        ExprNode::Binary {
            op: BinOp::And | BinOp::Or,
            left,
            right,
        } => truth_valued(left) && truth_valued(right),
        ExprNode::Binary { op, .. } => matches!(
            op,
            BinOp::Eq
                | BinOp::Ne
                | BinOp::Lt
                | BinOp::Le
                | BinOp::Gt
                | BinOp::Ge
                | BinOp::StrEq
                | BinOp::StrNe
                | BinOp::In
                | BinOp::Ni
        ),
        _ => false,
    }
}

/// Whether `proposed` is `!positive` for every operand value: the written
/// negation itself, the complementary operator over the same operands, or De
/// Morgan over truth-valued sides (which keeps the short circuit).
fn negates(positive: &ExprNode, proposed: &ExprNode) -> bool {
    use crate::expr_ast::UnaryOp;
    if let ExprNode::Unary {
        op: UnaryOp::Not,
        operand,
    } = proposed
        && same_text(positive, operand)
    {
        return true;
    }
    let (
        ExprNode::Binary { op, left, right },
        ExprNode::Binary {
            op: negated,
            left: new_left,
            right: new_right,
        },
    ) = (positive, proposed)
    else {
        return false;
    };
    match (op, negated) {
        (BinOp::Eq, BinOp::Ne)
        | (BinOp::Ne, BinOp::Eq)
        | (BinOp::StrEq, BinOp::StrNe)
        | (BinOp::StrNe, BinOp::StrEq)
        | (BinOp::In, BinOp::Ni)
        | (BinOp::Ni, BinOp::In)
        | (BinOp::StrLt, BinOp::StrGe)
        | (BinOp::StrGe, BinOp::StrLt)
        | (BinOp::StrLe, BinOp::StrGt)
        | (BinOp::StrGt, BinOp::StrLe) => same_text(left, new_left) && same_text(right, new_right),
        (BinOp::And, BinOp::Or) | (BinOp::Or, BinOp::And) => {
            truth_valued(left)
                && truth_valued(right)
                && negates(left, new_left)
                && negates(right, new_right)
        }
        _ => false,
    }
}

fn integer_literal(node: &ExprNode, profile: &'static tcl_dialect::DialectProfile) -> Option<i64> {
    let ExprNode::Literal { text, .. } = node else {
        return None;
    };
    match tcl_syntax::number::parse_whole_with(
        text.trim(),
        tcl_syntax::number::ParseFlags::for_syntax(profile.grammar.numbers),
    )? {
        tcl_syntax::number::Number::Int(value) => Some(value),
        _ => None,
    }
}

/// A fixed string operand that no numeric conversion accepts, so `==` against
/// it compares strings.
fn word_is_no_number(node: &ExprNode, profile: &'static tcl_dialect::DialectProfile) -> bool {
    let ExprNode::String { text, .. } = node else {
        return false;
    };
    tcl_syntax::expr::fixed_string_operand(text).is_some_and(|word| {
        tcl_syntax::number::parse_whole_with(
            word.trim(),
            tcl_syntax::number::ParseFlags::for_syntax(profile.grammar.numbers),
        )
        .is_none()
    })
}

fn equivalent_for_integer_operands(
    original: &ExprNode,
    proposed: &ExprNode,
    numeric: NumericCtx<'_>,
    profile: &'static tcl_dialect::DialectProfile,
    truth_only: bool,
) -> bool {
    const GRID: [i64; 7] = [-3, -1, 0, 1, 2, 7, 1 << 40];
    let Some(types) = numeric else {
        return false;
    };
    let mut names: Vec<&str> = Vec::new();
    for tree in [original, proposed] {
        for node in tree.evaluation_nodes() {
            match node {
                ExprNode::Var { text, name, .. } => {
                    if !types.integer.contains(text)
                        || text.strip_prefix('$') != Some(name.as_str())
                    {
                        return false;
                    }
                    if !names.contains(&name.as_str()) {
                        names.push(name);
                    }
                }
                ExprNode::Literal { text, .. } => {
                    if !matches!(
                        tcl_syntax::number::parse_whole_with(
                            text.trim(),
                            tcl_syntax::number::ParseFlags::for_syntax(profile.grammar.numbers),
                        ),
                        Some(tcl_syntax::number::Number::Int(_))
                    ) {
                        return false;
                    }
                }
                ExprNode::Unary { .. } | ExprNode::Binary { .. } | ExprNode::Ternary { .. } => {}
                _ => return false,
            }
        }
    }
    if names.is_empty() || names.len() > 4 {
        return false;
    }
    let policy = FoldPolicy::for_profile(leading_zero_is_octal(profile), Some(profile));
    let value = |tree: &ExprNode, env: &crate::tcl_expr_eval::Env| {
        crate::tcl_expr_eval::analyse_tcl_expr_with_resolved_math_bindings(
            tree,
            env,
            policy,
            &|_, _| None,
            None,
        )
        .and_then(|evaluation| {
            crate::tcl_expr_eval::format_tcl_value_with_policy(&evaluation.value, policy)
        })
        .map(|text| {
            if truth_only {
                (text != "0").to_string()
            } else {
                text
            }
        })
    };
    let mut indices = vec![0usize; names.len()];
    loop {
        let env: crate::tcl_expr_eval::Env = names
            .iter()
            .zip(&indices)
            .map(|(name, &index)| {
                (
                    (*name).to_owned(),
                    crate::tcl_expr_eval::EnvValue::Int(GRID[index]),
                )
            })
            .collect();
        if value(original, &env) != value(proposed, &env) {
            return false;
        }
        let mut position = 0;
        loop {
            if position == indices.len() {
                return true;
            }
            indices[position] += 1;
            if indices[position] < GRID.len() {
                break;
            }
            indices[position] = 0;
            position += 1;
        }
    }
}

/// As [`instcombine_expr`], but with a numeric-type context so the
/// operand-dropping identities (`$x + 0` → `$x`, `$x * 0` → `0`, …) fire
/// only when the dropped operand is provably numeric. See [`NumericCtx`].
#[must_use]
pub fn instcombine_expr_typed(
    expr: &str,
    bool_context: bool,
    numeric: NumericCtx<'_>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> (String, bool) {
    let trimmed = expr.trim();
    let parsed = crate::expr_parser::parse_expr_for_profile(trimmed, dialect);
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
    // Native-stack safety net: this bottom-up rewriter recurses
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
    let untyped = OperandTypes::default();
    if let Some(rewritten) = reassociate_node(&lowered, numeric.unwrap_or(&untyped)) {
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
/// skipped.
///
/// Regrouping changes the order the chain's terms combine in, which only
/// exact arithmetic tolerates, so every term must be a variable `types`
/// proves integer. Over a double the rounding is order-dependent: `set x
/// 10000000000000000.0; expr {$x + 1 + 2}` prints `10000000000000002.0` under
/// tclsh 8.5 to 9.1 and `expr {$x + 3}` prints `10000000000000004.0`. A
/// closed subtree (`2 + 3 + $x` → `5 + $x`) folds elsewhere and needs no
/// proof.
fn reassociate_node(node: &ExprNode, types: &OperandTypes) -> Option<ExprNode> {
    // Every regrouped term is a variable proven integer.
    let exact = |terms: &[ExprNode]| {
        terms.iter().all(|term| {
            matches!(term, ExprNode::Var { text, .. } if types.integer.contains(text.as_str()))
        })
    };
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
            if !exact(&terms) {
                return fold_closed_left(*op, left, right, collect_add_terms);
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
            if !exact(&terms) {
                return fold_closed_left(*op, left, right, collect_mul_terms);
            }
            let built = build_mul_expr(&terms, constant);
            (render_expr(&built) != render_expr(node)).then_some(built)
        }
        _ => None,
    }
}

/// The closed-subtree fold a chain keeps when it may not regroup: a left
/// operand made of integer literals alone evaluates to its constant before
/// anything else in the chain runs, so folding it leaves the evaluation order
/// — and a double's rounding — as written (`2 + 3 + $x` → `5 + $x`).
fn fold_closed_left(
    op: BinOp,
    left: &ExprNode,
    right: &ExprNode,
    collect: fn(&ExprNode, &mut Vec<ExprNode>, u32) -> Option<i64>,
) -> Option<ExprNode> {
    if int_literal_value(left).is_some() {
        return None;
    }
    let mut terms = Vec::new();
    let constant = collect(left, &mut terms, 0)?;
    terms.is_empty().then(|| ExprNode::Binary {
        op,
        left: Box::new(make_int_literal(constant)),
        right: Box::new(right.clone()),
    })
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
    // Native-stack safety net: past the cap, stop flattening and
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
    // Native-stack safety net: past the cap, stop flattening and
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
#[cfg(test)]
pub fn try_strength_reduce_expr(expr: &str) -> (String, bool) {
    try_strength_reduce_expr_typed(expr, None, None)
}

/// As [`try_strength_reduce_expr`], but with a numeric-type context for the
/// operand-dropping identities. See [`NumericCtx`].
#[must_use]
pub fn try_strength_reduce_expr_typed(
    expr: &str,
    numeric: NumericCtx<'_>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> (String, bool) {
    let trimmed = expr.trim();
    let parsed = crate::expr_parser::parse_expr_for_profile(trimmed, dialect);
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
pub fn try_strlen_simplify_expr(
    expr: &str,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> (String, bool) {
    let trimmed = expr.trim();
    let parsed = crate::expr_parser::parse_expr_for_profile(trimmed, dialect);
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
pub fn try_eq_ne_string_compare_simplify_expr(
    expr: &str,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> (String, bool) {
    let trimmed = expr.trim();
    let parsed = crate::expr_parser::parse_expr_for_profile(trimmed, dialect);
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
    // `BinOp::inverse()` (tcl_syntax::expr::operators) is the single source
    // for which comparison inverts to which — a local match here drifts, and
    // one missing the TIP 461 string-ordering four (`lt`/`le`/`gt`/`ge`) or
    // list membership (`in`/`ni`) leaves `!(x lt y)` / `!(x in list)`
    // unsimplified even though the same total-order/negation identity holds
    // for them as for the numeric/string-eq forms already covered.
    //
    // The four *ordered numeric* rows (`<`/`<=`/`>`/`>=`) carry a NaN
    // precondition (`BinOp::inverse_needs_non_nan`): `expr {!(NaN < 1)}` is 1
    // but `expr {NaN >= 1}` is 0, so they fire only when both operands are
    // proved non-NaN. `==`/`!=` are exact complements even for NaN, and the
    // string / membership operators have no NaN rule, so those stay
    // unconditional.
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
    let (ExprNode::Var { text: l, .. }, ExprNode::Var { text: r, .. }) = (left, right) else {
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
        // opposite of every answer below. So they need $x proved non-NaN;
        // without the proof the expression stays as written.
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
/// type context both guards pass.
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
    match node {
        ExprNode::Binary { op, .. } => op.yields_boolean(),
        ExprNode::Unary { op, .. } => op.yields_boolean(),
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

/// Whether `left == right` (and `!=`) compares its operands as strings
/// whatever the other one holds, so `eq` / `ne` give the same answer.
///
/// Tcl compares numerically only when both operands are numbers, so it is
/// enough that one is a fixed string that is not a number under any release.
/// A boolean word counts as a string here: `==` never reads `true` as `1`
/// (`expr {"true" == 1}` is `0` on tclsh 8.4.20 through 9.1b0), while `nan`,
/// ` 1`, `0x10`, `08` (9.0) and `1_0` (9.0) are numbers. A substituting
/// operand proves nothing: `"$x" == 1` is numeric when `x` is `1.0`.
pub(crate) fn eq_ne_compares_as_strings(left: &ExprNode, right: &ExprNode) -> bool {
    let fixed_non_numeric = |node: &ExprNode| {
        matches!(node, ExprNode::String { text, .. }
            if fixed_operand_text(text).is_some_and(|value| !is_numeric_string_in_any_release(value)))
    };
    fixed_non_numeric(left) || fixed_non_numeric(right)
}

/// Whether `node` is provably **not** a number for `expr` — the dual of
/// [`node_provably_numeric`], used to gate the eq/ne string-compare
/// promotion (O120). Only a string literal whose stripped text is neither
/// numeric nor a boolean word qualifies; everything else (variables, command
/// substitutions, arithmetic) is conservatively rejected.
fn node_provably_non_numeric(node: &ExprNode) -> bool {
    matches!(node, ExprNode::String { text, .. }
        if fixed_operand_text(text).is_some_and(|value| !is_numeric_or_boolean_string(value)))
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
    // Public entry: the top of an expression tree is nesting depth 0; the
    // recursion cap lives in [`expr_has_command_subst_at`].
    expr_has_command_subst_at(node, 0)
}

fn expr_has_command_subst_at(node: &ExprNode, depth: u32) -> bool {
    // Native-stack safety net: past the cap, assume "yes, has a
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
        // A `"…"` operand substitutes, so a `[cmd]` in it runs: O110 rewrote
        // `expr {"[incr x]" && 0}` to `0`, losing the `incr` (#2227, found in
        // review).
        ExprNode::String { text, .. } => crate::word_subst::quoted_operand_body(text).is_some(),
        ExprNode::Literal { .. }
        | ExprNode::Var { .. }
        | ExprNode::Raw { .. }
        // A word, not an expression: it holds no command substitution.
        | ExprNode::CompiledWord { .. } => false,
    }
}

#[cfg(test)]
mod tests;
