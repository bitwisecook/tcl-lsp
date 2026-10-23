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

//! Configured scalar/string equality source projections for refactor and report consumers.
//!
//! These pure syntax projections keep completed literal values separate from
//! original scalar references. Applicability, effects and edits have other owners.

use tcl_lexer::LexerConfig;

use super::datagroup::{ScalarVariableSourceSyntax, scalar_variable_source_syntax};

/// A selected scalar/string equality; numeric comparison has a separate value policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScalarStringEquality {
    pub(super) subject: ScalarVariableSourceSyntax,
    pub(super) value: String,
    pub(super) negated: bool,
}

impl ScalarStringEquality {
    /// Return the original scalar-reference syntax, completed literal value and negation.
    #[must_use]
    pub fn into_parts(self) -> (ScalarVariableSourceSyntax, String, bool) {
        (self.subject, self.value, self.negated)
    }
}

/// Classify one scalar/string equality under an independently selected expression context.
///
/// The configuration must be the lexical projection of that context. The result
/// describes source syntax only; it grants no read, lookup, frame or edit authority.
#[must_use]
pub fn scalar_string_equality(
    condition: &str,
    config: LexerConfig,
    context: &tcl_syntax::expr::parser::ExprParseContext,
) -> Option<ScalarStringEquality> {
    if context.lexer_grammar != config.grammar_over(context.lexer_grammar) {
        return None;
    }
    let parsed = tcl_syntax::expr::parser::parse_expr_with_syntax_context(condition, context);
    parsed_eq_test(&parsed, config, context)
}

fn parsed_eq_test(
    parsed: &tcl_syntax::expr::ExprNode,
    config: LexerConfig,
    context: &tcl_syntax::expr::parser::ExprParseContext,
) -> Option<ScalarStringEquality> {
    use tcl_syntax::expr::{BinOp, ExprNode, UnaryOp};
    let mut node = parsed;
    let mut negated = false;
    while let ExprNode::Unary {
        op: UnaryOp::Not,
        operand,
    } = node
    {
        negated = !negated;
        node = operand;
    }
    let ExprNode::Binary { op, left, right } = node else {
        return None;
    };
    let numeric_operator = match op {
        BinOp::StrEq => false,
        BinOp::StrNe => {
            negated = !negated;
            false
        }
        BinOp::Eq => true,
        BinOp::Ne => {
            negated = !negated;
            true
        }
        _ => return None,
    };
    let (subject, value) = scalar_and_fixed_string(left, right, config)
        .or_else(|| scalar_and_fixed_string(right, left, config))?;
    if numeric_operator
        && tcl_syntax::number::Numbers::Target(context.lexer_grammar.numbers)
            .is_number_in_any_release(&value)
    {
        return None;
    }
    Some(ScalarStringEquality {
        subject,
        value,
        negated,
    })
}

fn scalar_and_fixed_string(
    subject: &tcl_syntax::expr::ExprNode,
    value: &tcl_syntax::expr::ExprNode,
    config: LexerConfig,
) -> Option<(ScalarVariableSourceSyntax, String)> {
    use tcl_syntax::expr::ExprNode;
    let (ExprNode::Var { text: subject, .. } | ExprNode::String { text: subject, .. }) = subject
    else {
        return None;
    };
    let ExprNode::String { text: value, .. } = value else {
        return None;
    };
    Some((
        scalar_variable_source_syntax(subject, config)?,
        tcl_syntax::expr::fixed_string_operand(value)?.to_owned(),
    ))
}

/// Classify an actual OR tree of positive equalities against the same scalar.
///
/// Literal operator-looking bytes remain data. All leaves retain the same
/// configured equality policy as [`scalar_string_equality`].
#[must_use]
pub fn scalar_string_disjunction(
    condition: &str,
    config: LexerConfig,
    context: &tcl_syntax::expr::parser::ExprParseContext,
) -> Option<(ScalarVariableSourceSyntax, Vec<String>)> {
    use tcl_syntax::expr::{BinOp, ExprNode};
    if context.lexer_grammar != config.grammar_over(context.lexer_grammar) {
        return None;
    }
    let parsed = tcl_syntax::expr::parser::parse_expr_with_syntax_context(condition, context);
    let mut pending = vec![&parsed];
    let mut target_var: Option<ScalarVariableSourceSyntax> = None;
    let mut values = Vec::new();
    while let Some(node) = pending.pop() {
        if let ExprNode::Binary {
            op: BinOp::Or,
            left,
            right,
        } = node
        {
            pending.push(right);
            pending.push(left);
            continue;
        }
        let equality = parsed_eq_test(node, config, context)?;
        if equality.negated {
            return None;
        }
        match &target_var {
            None => target_var = Some(equality.subject),
            Some(previous) if previous.name() != equality.subject.name() => return None,
            _ => {}
        }
        values.push(equality.value);
    }
    (values.len() >= 2).then_some((target_var?, values))
}
