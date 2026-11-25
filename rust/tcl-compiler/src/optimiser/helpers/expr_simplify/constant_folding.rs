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

//! Folding an expression to a literal, unwrapping a redundant `[expr {…}]` and substituting known constants.

use super::numeric_proofs::{is_numeric_string_in_every_release, is_numeric_string_under};
use crate::expr_ast::ExprNode;
use crate::expr_parser::parse_expr_for_profile;
use crate::sccp::BuiltinFoldInputs;
use crate::tcl_expr_eval::{FoldPolicy, leading_zero_is_octal};
use std::collections::HashSet;
use tcl_lexer::ExprTokenType;
use tcl_registry::value_transfer::ExactValue;

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
                crate::tcl_expr_eval::EnvValue::text(value.as_str()),
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
pub(super) fn is_numeric_literal(text: &str) -> bool {
    is_numeric_string_in_every_release(text)
}
