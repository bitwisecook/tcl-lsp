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

//! Nested native expressions selected through invocation proofs, with their verbatim source anchors.

use super::lifted_calls;
use crate::ir::CommandTokens;
use tcl_lexer::Span;
use tcl_source::words::WordExpr;

/// A proved nested native expression with its verbatim source anchor.
#[derive(Debug, Clone)]
pub struct LiftedSourceExpression {
    /// Parsed expression selected by the shared invocation owner.
    pub expression: crate::expr_ast::ExprNode,
    /// Extent of the enclosing substitution, for diagnostics.
    pub span: Span,
    /// First byte of the original expression text; transformed text has no anchor.
    pub expression_base: Option<u32>,
    /// Exact concatenated expression bytes and piecewise original read sites.
    pub executed_source: Option<crate::command_binding::ExecutedExpressionSource>,
}

/// Select an expression from proved invocation facts, retaining its exact
/// argument source. Dynamic concatenation cannot be projected into authored
/// expression offsets and yields no positioned expression proof.
#[must_use]
pub(crate) fn source_expression_from_invocation(
    invocation: &crate::registry_invocation::ResolvedStatementInvocation,
    profile: Option<&tcl_dialect::DialectProfile>,
    span: Span,
    parent: Option<tcl_names::command::CommandAllocationSite>,
) -> Option<LiftedSourceExpression> {
    source_expression_from_invocation_parsed(invocation, span, parent, |text| {
        tcl_syntax::expr::parser::parse_expr_for_profile(text, profile)
    })
}

pub(crate) fn source_expression_from_invocation_with_syntax_context(
    invocation: &crate::registry_invocation::ResolvedStatementInvocation,
    parser: tcl_syntax::expr::parser::ExprParseContext,
    span: Span,
    parent: Option<tcl_names::command::CommandAllocationSite>,
) -> Option<LiftedSourceExpression> {
    source_expression_from_invocation_parsed(invocation, span, parent, |text| {
        tcl_syntax::expr::parser::parse_expr_with_syntax_context(text, &parser)
    })
}

pub(crate) fn source_expression_from_invocation_parsed(
    invocation: &crate::registry_invocation::ResolvedStatementInvocation,
    span: Span,
    parent: Option<tcl_names::command::CommandAllocationSite>,
    parse: impl Fn(&str) -> crate::expr_ast::ExprNode,
) -> Option<LiftedSourceExpression> {
    if invocation.facts.operation
        != tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::Expr,
        )
    {
        return None;
    }
    if let Some(parent) = parent {
        let values = (0..invocation.arguments.len())
            .map(|index| invocation.argument_literal(index))
            .collect::<Option<Vec<_>>>()?;
        let executed_source = crate::command_binding::ExecutedExpressionSource::from_arguments(
            parent,
            invocation.effective.words.get(1..)?,
            &values,
            invocation.dialect?,
        )?;
        return Some(LiftedSourceExpression {
            expression: parse(&executed_source.text),
            span,
            expression_base: None,
            executed_source: Some(executed_source),
        });
    }
    if invocation.arguments.len() != 1 {
        return None;
    }
    let word = invocation.effective.words.get(1)?;
    let (WordExpr::BracedLiteral { text, .. } | WordExpr::Literal { text, .. }) = word else {
        return None;
    };
    if invocation.argument_literal(0)?.as_str() != text {
        return None;
    }
    Some(LiftedSourceExpression {
        expression: parse(text),
        span,
        expression_base: None,
        executed_source: None,
    })
}

/// Representation expression reads with complete supplied metadata and grammar.
/// Neither input supplies execution, source positions or a native cache by itself.
pub(crate) fn representation_expression_with_metadata_context(
    tokens: &CommandTokens,
    registry: &tcl_registry::CommandRegistry,
    context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    parser: tcl_syntax::expr::parser::ExprParseContext,
    span: Span,
) -> Option<LiftedSourceExpression> {
    let invocation =
        crate::registry_invocation::normal_representation_invocation_with_metadata_context(
            registry, context, tokens,
        )?;
    invocation.source_expression_with_syntax_context(parser, span, expression_parent(tokens))
}

/// Nested normal representation reads using the same actual parent context.
pub(crate) fn lifted_representation_expressions_with_metadata_context(
    tokens: Option<&CommandTokens>,
    registry: &tcl_registry::CommandRegistry,
    context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    config: tcl_lexer::LexerConfig,
    parser: tcl_syntax::expr::parser::ExprParseContext,
) -> Vec<LiftedSourceExpression> {
    lifted_calls(tokens, config)
        .into_iter()
        .filter_map(|lifted| {
            representation_expression_with_metadata_context(
                lifted.tokens.as_ref()?,
                registry,
                context,
                parser,
                lifted.span,
            )
        })
        .collect()
}

fn expression_parent(tokens: &CommandTokens) -> Option<tcl_names::command::CommandAllocationSite> {
    Some(tcl_names::command::CommandAllocationSite {
        source: std::sync::Arc::clone(tokens.source_binding.as_ref()?.source_origin()?),
        offset: tokens.argv.first()?.start(),
    })
}

/// Nested native expressions selected through actual retained dispatch proofs.
/// Source anchors belong to their effective argument words, including aliases;
/// unresolved commands and computed expression text provide no guessed proof.
#[must_use]
pub fn lifted_source_expressions(
    tokens: Option<&CommandTokens>,
    registry: &tcl_registry::CommandRegistry,
) -> Vec<LiftedSourceExpression> {
    selected_lifted_expressions(tokens, registry)
}

/// Original nested expressions under the supplied full metadata and grammar.
/// Missing or foreign availability cannot borrow the registry's nominal profile.
#[must_use]
pub(crate) fn lifted_source_expressions_with_metadata_context(
    tokens: Option<&CommandTokens>,
    registry: &tcl_registry::CommandRegistry,
    context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    config: tcl_lexer::LexerConfig,
    parser: tcl_syntax::expr::parser::ExprParseContext,
) -> Vec<LiftedSourceExpression> {
    let Some(context) = context.filter(|context| context.matches_registry(registry)) else {
        return Vec::new();
    };
    lifted_calls(tokens, config)
        .into_iter()
        .filter_map(|lifted| {
            let tokens = lifted.tokens.as_ref()?;
            let invocation =
                crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
                    registry,
                    Some(context),
                    tokens,
                )?;
            source_expression_from_invocation_with_syntax_context(
                &invocation,
                parser,
                lifted.span,
                expression_parent(tokens),
            )
        })
        .collect()
}

fn selected_lifted_expressions(
    tokens: Option<&CommandTokens>,
    registry: &tcl_registry::CommandRegistry,
) -> Vec<LiftedSourceExpression> {
    let context = registry
        .profile()
        .map(tcl_registry::model::semantic::SemanticContext::for_profile);
    lifted_calls(
        tokens,
        tcl_lexer::LexerConfig::for_profile(registry.profile()),
    )
    .into_iter()
    .filter_map(|lifted| {
        let tokens = lifted.tokens.as_ref()?;
        let invocation =
            crate::registry_invocation::resolved_tokens_invocation(registry, context, tokens)?;
        source_expression_from_invocation(
            &invocation,
            registry.profile(),
            lifted.span,
            expression_parent(tokens),
        )
    })
    .collect()
}
