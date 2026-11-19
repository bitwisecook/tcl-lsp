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

//! `expr` bodies: whitespace compression and size-reducing rewrites of the expression tree.

use super::{MinifyEnv, minify_body};
use tcl_compiler::expr_ast::render_expr;
use tcl_compiler::{BinOp, ExprNode, UnaryOp, parse_expr_for_profile};

// expr whitespace compression

/// One token of an `expr` body for whitespace compression.
pub(super) enum ExprTok {
    /// A `[…]` command substitution (already minified).
    Cmd(String),
    /// Any other token (string, var, word, operator, punctuation).
    Other(String),
    /// A run of whitespace.
    Space,
}

/// Remove expression gaps only when the complete configured token sequence
/// remains unchanged. Required word boundaries and literal contents survive;
/// this step performs no AST shrinking.
pub(super) fn strip_expr_whitespace(text: &str, env: MinifyEnv<'_>, depth: u32) -> String {
    strip_expr_whitespace_with_script_compaction(text, env, depth, true)
}

pub(super) fn strip_expr_whitespace_with_script_compaction(
    text: &str,
    env: MinifyEnv<'_>,
    depth: u32,
    compact_scripts: bool,
) -> String {
    let mut syntax = tcl_syntax::expr::parser::ExprParseContext::for_profile(env.dialect);
    syntax.lexer_grammar = env.config.grammar_over(syntax.lexer_grammar);
    if matches!(
        tcl_syntax::expr::parser::parse_expr_with_syntax_context(text, &syntax),
        ExprNode::Raw { .. }
    ) {
        return text.to_owned();
    }
    let tokens = if compact_scripts {
        tokenise_expr(text, env, depth)
    } else {
        tokenise_expr_with_script_compaction(text, env, depth, false)
    };
    let rendered = tokens
        .iter()
        .filter_map(|token| match token {
            ExprTok::Space => None,
            ExprTok::Cmd(value) | ExprTok::Other(value) => Some(value.as_str()),
        })
        .collect::<Vec<_>>();
    let Some(first) = rendered.first() else {
        return text.to_owned();
    };
    let baseline = rendered.join(" ");
    let Some(expected) = expression_token_signature(&baseline, env, false) else {
        return text.to_owned();
    };
    if expression_token_signature(text, env, true)
        != expression_token_signature(&baseline, env, true)
    {
        return text.to_owned();
    }
    let mut output = (*first).to_owned();
    for pair in rendered.windows(2) {
        let (previous, current) = (pair[0], pair[1]);
        let joined = format!("{previous}{current}");
        let spaced = format!("{previous} {current}");
        let preserves = expression_token_signature(&joined, env, false)
            .zip(expression_token_signature(&spaced, env, false))
            .is_some_and(|(joined, spaced)| joined == spaced);
        if !preserves {
            output.push(' ');
        }
        output.push_str(current);
    }
    if expression_token_signature(&output, env, false) == Some(expected) {
        output
    } else {
        baseline
    }
}

fn configured_expression_tokens(
    text: &str,
    env: MinifyEnv<'_>,
) -> Option<Vec<tcl_lexer::ExprToken<Vec<u8>>>> {
    let (tokens, unknown) = tcl_lexer::tokenise_expr_bytes_checked_with_expression_grammar(
        text.as_bytes(),
        &env.config.grammar_over(env.dialect.grammar),
        env.dialect.expr_grammar_base,
        env.dialect.f5_core_expr_grammar(),
    );
    if unknown {
        return None;
    }
    tcl_lexer::expression_terms(text.as_bytes(), &tokens, env.config)?;
    Some(tokens)
}

/// Ordered lexical tokens only; rewritten child scripts retain their own
/// source purpose and AST transformations keep their independent contract.
pub(super) fn expression_token_signature(
    text: &str,
    env: MinifyEnv<'_>,
    omit_command_text: bool,
) -> Option<Vec<(tcl_lexer::ExprTokenType, Vec<u8>)>> {
    Some(
        configured_expression_tokens(text, env)?
            .into_iter()
            .filter(|token| !token.kind.is_skipped() && token.kind != tcl_lexer::ExprTokenType::Eof)
            .map(|token| {
                let text = if omit_command_text && token.kind == tcl_lexer::ExprTokenType::Command {
                    Vec::new()
                } else {
                    token.text
                };
                (token.kind, text)
            })
            .collect(),
    )
}

/// Compress and shrink an `expr` body: drop redundant parentheses, strip
/// whitespace, then try
/// AST transforms (De Morgan / comparison inversion / double
/// negation) and keep whichever is shorter.
pub(super) fn compress_expr(text: &str, env: MinifyEnv<'_>, depth: u32) -> String {
    let compressed = strip_expr_whitespace(&drop_redundant_parens(text, env), env, depth);
    let shrunk = shrink_expr_ast(&compressed, env, depth);
    if shrunk.len() < compressed.len() {
        shrunk
    } else {
        compressed
    }
}

/// The expression without the parentheses precedence makes redundant, parsed
/// and re-verified under the document's own expression grammar.
fn drop_redundant_parens(text: &str, env: MinifyEnv<'_>) -> String {
    original_expression(text, env)
        .and_then(|expression| expression.without_redundant_parens())
        .unwrap_or_else(|| text.to_owned())
}

/// `text` as an original expression under the document's own grammar.
pub(super) fn original_expression(
    text: &str,
    env: MinifyEnv<'_>,
) -> Option<tcl_syntax::expr::OriginalSourceExpression> {
    let mut syntax = tcl_syntax::expr::parser::ExprParseContext::for_profile(env.dialect);
    syntax.lexer_grammar = env.config.grammar_over(syntax.lexer_grammar);
    tcl_syntax::expr::OriginalSourceExpression::capture(text, &syntax, env.config)
}

/// AST-based expression shrinking.
pub(super) fn shrink_expr_ast(text: &str, env: MinifyEnv<'_>, depth: u32) -> String {
    let node = parse_expr_for_profile(text, Some(env.dialect));
    if matches!(node, ExprNode::Raw { .. }) {
        return text.to_owned();
    }
    let shrunk = shrink_node(&node);
    if shrunk == node {
        return text.to_owned();
    }
    let rendered = render_expr(&shrunk);
    strip_expr_whitespace(&rendered, env, depth)
}

/// The logical complement of a comparison / membership operator the minifier
/// may safely substitute, or `None` when there is none it can use.
///
/// The table itself is [`BinOp::inverse`] (`tcl_syntax::expr::operators`), so
/// the minifier cannot disagree with the shared evaluator about those 14 rows.
///
/// The four *ordered numeric* rows are refused outright. Their identity holds
/// only when neither operand can be NaN (`expr {!(NaN < 1)}` is 1 while
/// `expr {NaN >= 1}` is 0 — [`BinOp::inverse_needs_non_nan`]), and the minifier
/// rewrites source text with no type information whatsoever, so it can never
/// discharge that precondition for an operand like `$x`. `==`/`!=` are exact
/// complements even for NaN, and the string / membership operators never
/// compare numerically, so those rows stay available.
fn comparison_inversion(op: BinOp) -> Option<BinOp> {
    if op.inverse_needs_non_nan() {
        return None;
    }
    op.inverse()
}

/// Build a `!operand` node.
fn negate(operand: ExprNode) -> ExprNode {
    ExprNode::Unary {
        op: UnaryOp::Not,
        operand: Box::new(operand),
    }
}

/// Pick `candidate` over `original` when its rendering is shorter.
fn pick_shorter(candidate: ExprNode, original: &ExprNode) -> ExprNode {
    if render_expr(&candidate).len() < render_expr(original).len() {
        candidate
    } else {
        original.clone()
    }
}

/// Recursively try size-reducing transforms on an expression node.
fn shrink_node(node: &ExprNode) -> ExprNode {
    match node {
        ExprNode::Unary {
            op: UnaryOp::Not,
            operand,
        } => shrink_not(node, operand),
        ExprNode::Binary { op, left, right }
            if matches!(op, BinOp::Or | BinOp::WordOr) && both_negations(left, right) =>
        {
            // De Morgan reverse: !a || !b → !(a && b) (if shorter).
            let (a, b) = (unwrap_not(left), unwrap_not(right));
            let dual = if *op == BinOp::Or {
                BinOp::And
            } else {
                BinOp::WordAnd
            };
            let combined = negate(ExprNode::Binary {
                op: dual,
                left: Box::new(shrink_node(a)),
                right: Box::new(shrink_node(b)),
            });
            pick_shorter(combined, node)
        }
        ExprNode::Binary { op, left, right }
            if matches!(op, BinOp::And | BinOp::WordAnd) && both_negations(left, right) =>
        {
            // De Morgan reverse: !a && !b → !(a || b) (if shorter).
            let (a, b) = (unwrap_not(left), unwrap_not(right));
            let dual = if *op == BinOp::And {
                BinOp::Or
            } else {
                BinOp::WordOr
            };
            let combined = negate(ExprNode::Binary {
                op: dual,
                left: Box::new(shrink_node(a)),
                right: Box::new(shrink_node(b)),
            });
            pick_shorter(combined, node)
        }
        ExprNode::Binary { op, left, right } => {
            let new_left = shrink_node(left);
            let new_right = shrink_node(right);
            ExprNode::Binary {
                op: *op,
                left: Box::new(new_left),
                right: Box::new(new_right),
            }
        }
        ExprNode::Unary { op, operand } => ExprNode::Unary {
            op: *op,
            operand: Box::new(shrink_node(operand)),
        },
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => ExprNode::Ternary {
            condition: Box::new(shrink_node(condition)),
            true_branch: Box::new(shrink_node(true_branch)),
            false_branch: Box::new(shrink_node(false_branch)),
        },
        other => other.clone(),
    }
}

/// Whether both operands are `!`-negations.
fn both_negations(left: &ExprNode, right: &ExprNode) -> bool {
    matches!(
        left,
        ExprNode::Unary {
            op: UnaryOp::Not,
            ..
        }
    ) && matches!(
        right,
        ExprNode::Unary {
            op: UnaryOp::Not,
            ..
        }
    )
}

/// The operand of a `!`-negation (caller guarantees the shape).
fn unwrap_not(node: &ExprNode) -> &ExprNode {
    match node {
        ExprNode::Unary {
            op: UnaryOp::Not,
            operand,
        } => operand,
        _ => node,
    }
}

/// Handle the `!`-prefixed shrink cases (double negation,
/// comparison inversion, De Morgan forward), falling back to a
/// generic operand recurse.
fn shrink_not(node: &ExprNode, operand: &ExprNode) -> ExprNode {
    // NB: `!!x → x` is deliberately NOT folded. It is only sound when the whole
    // expression is consumed as a boolean (`if {!!$x}` ≡ `if {$x}`); in a value
    // context or as a subexpression operand it changes the result, since `!!x`
    // yields the 0/1 boolean coercion while `x` yields x's value (tclsh:
    // `expr {!!5}` → 1, `expr {5}` → 5). The minifier processes Expr-role
    // arguments without knowing whether the result is consumed as a boolean, so
    // the fold is unsafe here. The De Morgan rewrite below preserves the 0/1
    // result for any operand values and remains unconditional; the
    // comparison-inversion rewrite preserves it only for the operators
    // `comparison_inversion` still offers — the ordered numeric four (`<`,
    // `<=`, `>`, `>=`) are excluded there because their identity fails on a
    // NaN operand and the minifier cannot rule one out.
    if let ExprNode::Binary { op, left, right } = operand {
        // Comparison inversion: !($a == $b) → $a != $b.
        if let Some(inv) = comparison_inversion(*op) {
            let inverted = ExprNode::Binary {
                op: inv,
                left: Box::new(shrink_node(left)),
                right: Box::new(shrink_node(right)),
            };
            return pick_shorter(inverted, node);
        }
        // De Morgan forward.
        if matches!(op, BinOp::And | BinOp::WordAnd | BinOp::Or | BinOp::WordOr) {
            let neg_l = negate(shrink_node(left));
            let neg_r = negate(shrink_node(right));
            let dual = match op {
                BinOp::And => BinOp::Or,
                BinOp::WordAnd => BinOp::WordOr,
                BinOp::Or => BinOp::And,
                _ => BinOp::WordAnd,
            };
            let demorgan = ExprNode::Binary {
                op: dual,
                left: Box::new(shrink_node(&neg_l)),
                right: Box::new(shrink_node(&neg_r)),
            };
            return pick_shorter(demorgan, node);
        }
    }
    // Generic recurse into the operand.
    negate(shrink_node(operand))
}

/// Project the selected shared expression tokens and original command terms.
/// Unknown syntax keeps the entire original expression, including its gaps.
pub(super) fn tokenise_expr(text: &str, env: MinifyEnv<'_>, nest_depth: u32) -> Vec<ExprTok> {
    tokenise_expr_with_script_compaction(text, env, nest_depth, true)
}

fn tokenise_expr_with_script_compaction(
    text: &str,
    env: MinifyEnv<'_>,
    nest_depth: u32,
    compact_scripts: bool,
) -> Vec<ExprTok> {
    let project = || -> Option<Vec<ExprTok>> {
        let tokens = configured_expression_tokens(text, env)?;
        let terms = tcl_lexer::expression_terms(text.as_bytes(), &tokens, env.config)?;
        let mut output = Vec::new();
        for token in tokens {
            if token.kind == tcl_lexer::ExprTokenType::Eof {
                continue;
            }
            if token.kind.is_skipped() {
                output.push(ExprTok::Space);
                continue;
            }
            let end = token.end.checked_add(1)?;
            let original = text.get(token.start as usize..end as usize)?;
            if original.as_bytes() != token.text {
                return None;
            }
            if compact_scripts && token.kind == tcl_lexer::ExprTokenType::Command {
                let term = terms.iter().find(|term| {
                    term.kind == tcl_lexer::ExprTermKind::Command
                        && term.source == (token.start..end)
                })?;
                let body = text.get(term.value.start as usize..term.value.end as usize)?;
                output.push(ExprTok::Cmd(format!(
                    "[{}]",
                    minify_body(
                        body,
                        env,
                        nest_depth + 1,
                        tcl_lexer::SourceSegmentationPurpose::NestedScript
                    )
                )));
            } else {
                output.push(ExprTok::Other(original.to_owned()));
            }
        }
        Some(output)
    };
    project().unwrap_or_else(|| vec![ExprTok::Other(text.to_owned())])
}
