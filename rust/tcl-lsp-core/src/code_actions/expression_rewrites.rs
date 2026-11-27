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

//! Rewrites of a selected expression: De Morgan and comparison inversion.

use super::{ActionKind, CodeAction};
use crate::definition::LspRange;
use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::LineIndex;
use tcl_syntax::expr::ast::{BinOp, ExprNode, UnaryOp, render_expr};
use tcl_syntax::expr::substitution::OriginalSourceExpression;

// Expression rewrites: De Morgan + invert comparison.

pub(super) fn expr_rewrite_actions(
    source: &str,
    range: LspRange,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Vec<CodeAction> {
    use tcl_compiler::registry_invocation::source_structure::OriginalExpressionOperatorRewriteKind;
    // Single-line, non-empty selection only.
    if range.start_line != range.end_line || range.start_character >= range.end_character {
        return Vec::new();
    }
    let start = crate::definition::byte_offset_at(
        line_index,
        source,
        range.start_line,
        range.start_character,
    );
    let finish =
        crate::definition::byte_offset_at(line_index, source, range.end_line, range.end_character);
    let Some(operand) = crate::expr_context::source_expression_operand_at(source, analysis, start)
    else {
        return Vec::new();
    };
    let Some(extent) = operand.content_span() else {
        return Vec::new();
    };
    if start < extent.start() || finish > extent.end() {
        return Vec::new();
    }
    let Some(expression) = operand.original_expression(source, analysis) else {
        return Vec::new();
    };
    let selected = (start - extent.start()) as usize..(finish - extent.start()) as usize;
    let Some(node) = selected_node(&expression, selected.clone()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let edit_range = LspRange {
        start_line: range.start_line,
        start_character: range.start_character,
        end_line: range.end_line,
        end_character: range.end_character,
    };
    if let Some(rewritten) = demorgan_transform(node) {
        let rewritten = if selected == (0..expression.source().len()) {
            rewritten
        } else {
            format!("({rewritten})")
        };
        out.push((
            CodeAction {
                title: "Apply De Morgan's law".to_string(),
                edits: vec![crate::rename::TextEdit {
                    range: edit_range,
                    new_text: rewritten,
                }],
                kind: ActionKind::RefactorRewrite,
                command: None,
                data_group_definition: None,
                disabled: None,
            },
            OriginalExpressionOperatorRewriteKind::DeMorgan,
        ));
    }
    if let Some(rewritten) = invert_comparison(&expression, node, selected) {
        out.push((
            CodeAction {
                title: "Invert comparison".to_string(),
                edits: vec![crate::rename::TextEdit {
                    range: edit_range,
                    new_text: rewritten,
                }],
                kind: ActionKind::RefactorRewrite,
                command: None,
                data_group_definition: None,
                disabled: None,
            },
            OriginalExpressionOperatorRewriteKind::InvertComparison,
        ));
    }
    if !analysis.allows_retained_logical_declaration_advice() {
        for (action, kind) in &mut out {
            if start == extent.start()
                && finish == extent.end()
                && original_operator_rewrite_allowed(
                    source,
                    analysis,
                    extent,
                    &action.edits[0].new_text,
                    *kind,
                )
            {
                continue;
            }
            action.edits.clear();
            action.disabled = Some("missing-expression-operator-rewrite-permission: Original expression roles provide source geometry; Native or hosted operator rewrites require an independent selected evaluation and replacement contract".to_owned());
        }
    }
    out.into_iter().map(|(action, _)| action).collect()
}

fn original_operator_rewrite_allowed(
    source: &str,
    analysis: &AnalysisResult,
    extent: tcl_lexer::Span,
    candidate: &str,
    kind: tcl_compiler::registry_invocation::source_structure::OriginalExpressionOperatorRewriteKind,
) -> bool {
    let Some(config) = analysis.body_lexer_config else {
        return false;
    };
    let commands = tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
    let [command] = commands.as_slice() else {
        return false;
    };
    let Some(words) = crate::original_invocation::source_registry_words(source, analysis, command)
    else {
        return false;
    };
    let Some(roles) = words.roles.as_ref() else {
        return false;
    };
    roles.iter().any(|&(argument, role)| {
        role == tcl_registry::ArgRole::Expr
            && tcl_compiler::registry_invocation::source_structure::original_expression_operator_rewrite(
                source, analysis, command, argument, candidate, kind,
            ).is_some_and(|proof| proof.original_operand().content_span().ok() == Some(extent))
    })
}

/// Select an actual parsed subtree. Parsing the highlighted bytes alone would
/// mistake matching text inside a quoted literal for a comparison.
fn selected_node(
    expression: &OriginalSourceExpression,
    selection: std::ops::Range<usize>,
) -> Option<&ExprNode> {
    if selection == (0..expression.source().len()) {
        return Some(expression.root());
    }
    let expression_text = expression.source().get(selection.clone())?;
    let subexpression = OriginalSourceExpression::capture(
        expression_text,
        expression.context(),
        expression.lexer_config(),
    )?;
    let rendered = render_expr(subexpression.root());
    let mut matching = expression
        .root()
        .evaluation_nodes()
        .into_iter()
        .filter(|node| {
            node.lexical_source_extent()
                .and_then(|(a, b)| {
                    tcl_syntax::expr::substitution::expression_span(expression.source(), a, b)
                })
                .is_some_and(|span| span.as_range() == selection)
                && render_expr(node) == rendered
        });
    let node = matching.next()?;
    matching.next().is_none().then_some(node)
}

fn invert_comparison(
    expression: &OriginalSourceExpression,
    node: &ExprNode,
    selection: std::ops::Range<usize>,
) -> Option<String> {
    let ExprNode::Binary { op, .. } = node else {
        return None;
    };
    let inverse = op.inverse()?;
    let operator = expression.binary_operator_span(node)?.as_range();
    let mut replacement = expression.source().get(selection.clone())?.to_owned();
    replacement.replace_range(
        operator.start.checked_sub(selection.start)?..operator.end.checked_sub(selection.start)?,
        inverse.as_str(),
    );
    Some(replacement)
}

fn opposite_connective(op: BinOp) -> Option<BinOp> {
    Some(match op {
        BinOp::And => BinOp::Or,
        BinOp::Or => BinOp::And,
        BinOp::WordAnd => BinOp::WordOr,
        BinOp::WordOr => BinOp::WordAnd,
        _ => return None,
    })
}

fn negate_node(operand: &ExprNode, op: UnaryOp) -> ExprNode {
    if let ExprNode::Unary { op: inner, operand } = operand
        && inner.yields_boolean()
    {
        *operand.clone()
    } else {
        ExprNode::Unary {
            op,
            operand: Box::new(operand.clone()),
        }
    }
}

fn demorgan_transform(node: &ExprNode) -> Option<String> {
    let rewritten = match node {
        ExprNode::Unary { op: not, operand } if not.yields_boolean() => {
            let ExprNode::Binary { op, left, right } = operand.as_ref() else {
                return None;
            };
            ExprNode::Binary {
                op: opposite_connective(*op)?,
                left: Box::new(negate_node(left, *not)),
                right: Box::new(negate_node(right, *not)),
            }
        }
        ExprNode::Binary { op, left, right } => {
            let (
                ExprNode::Unary {
                    op: first,
                    operand: l,
                },
                ExprNode::Unary {
                    op: second,
                    operand: r,
                },
            ) = (left.as_ref(), right.as_ref())
            else {
                return None;
            };
            if !first.yields_boolean() || first != second {
                return None;
            }
            ExprNode::Unary {
                op: *first,
                operand: Box::new(ExprNode::Binary {
                    op: opposite_connective(*op)?,
                    left: l.clone(),
                    right: r.clone(),
                }),
            }
        }
        _ => return None,
    };
    Some(render_expr(&rewritten))
}
