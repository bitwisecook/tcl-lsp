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

//! Original expression operands: the context and advice of a selected word.

use super::InvocationWordOrigin;
use super::source_model_operations::resolve_original_declared_layout;
use crate::ir::CommandTokens;
use tcl_registry::CommandRegistry;
use tcl_source::words::WordExpr;

/// An unchanged original expression operand for conditional diagnostics only.
/// Neither a normal handler nor successful operand evaluation is asserted.
pub(crate) struct OriginalExpressionOperandAdvice {
    pub(crate) expression: crate::expr_ast::ExprNode,
    pub(crate) dialect: tcl_native_model::InvocationDialect,
    pub(crate) evaluation: tcl_registry::conditional_expression::ConditionalExpressionEvaluation,
    pub(crate) pool: tcl_registry::conditional_expression::ConditionalExpressionPoolState,
    pub(crate) expression_base: u32,
    pub(crate) expression_text: String,
    pub(crate) lookup_closed: bool,
    pub(crate) frame: crate::var_resolve::VariableExecutionFrame,
    pub(crate) namespace_key: crate::command_binding::SourceNamespaceKey,
    pub(crate) span: tcl_lexer::Span,
}

/// Original declaration/compiler lookup candidates must agree on the expression
/// operation and the same written operand. Runtime uncertainty remains intact.
pub(crate) fn original_expression_operand_advice(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> Option<OriginalExpressionOperandAdvice> {
    original_expression_operand_advice_selected_word(registry, tokens, None)
}

/// Conditional checked expression topology for one actual written operand.
/// The selected descriptor owns its Expression role; source geometry alone
/// supplies neither this role nor an evaluation or successful handler.
pub(crate) fn original_expression_operand_advice_for_word(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
    written: usize,
) -> Option<OriginalExpressionOperandAdvice> {
    original_expression_operand_advice_selected_word(registry, tokens, Some(written))
}

/// Independently selected conditional expression grammar at one unchanged
/// original written operand. This is a parser purpose, never Normal or edits.
pub(crate) struct OriginalExpressionContextAdvice {
    pub(crate) parser: tcl_syntax::expr::parser::ExprParseContext,
    pub(crate) dialect: tcl_native_model::InvocationDialect,
    pub(crate) written: usize,
    pub(crate) lookup_closed: bool,
    pool: tcl_registry::conditional_expression::ConditionalExpressionPoolState,
    pub(super) frame: crate::var_resolve::VariableExecutionFrame,
    pub(super) namespace_key: crate::command_binding::SourceNamespaceKey,
}

/// Literal native bytes and logical source text borrow the same original
/// candidate/role/ordinal/configuration selector; neither reselects a parser.
pub(crate) fn original_literal_expression_context_advice(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> Option<OriginalExpressionContextAdvice> {
    original_expression_context_advice_selected_word(registry, tokens, None)
}

fn original_expression_context_advice_selected_word(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
    written: Option<usize>,
) -> Option<OriginalExpressionContextAdvice> {
    let binding = tokens.source_binding.as_ref()?;
    // Availability comes from this unchanged original point. Supplied missing
    // or stale ownership cannot be replaced by a catalogue/profile context.
    let context = binding
        .original_source_metadata_for_advice(tokens, registry)
        .ok()??;
    let advice = binding.original_conditional_operand_layout_advice(tokens, registry)?;
    let logical_source = advice.original_source_purpose(context, registry)?
        == crate::command_binding::OriginalInvocationSourcePurpose::Logical;
    if advice.has_opaque_handler_alternatives() {
        return None;
    }
    let selected = resolve_original_declared_layout(registry, context, tokens, &advice)?;
    // These are the selected descriptor's conditional expression slots. They
    // do not certify successful argument evaluation or an entered handler.
    let expressions =
        selected.with_metadata_schema(registry, context, advice.realm(), |schema| {
            if logical_source {
                schema.authored_logical_source_expression_arguments()
            } else {
                schema.authored_source_expression_arguments()
            }
        })?;
    let effective_index = if let Some(written) = written {
        let mut positions = expressions.arguments.iter().filter_map(|argument| {
            let effective_index = argument.checked_add(1)?;
            matches!(selected.effective.origins.get(effective_index),
                Some(InvocationWordOrigin::Written(index)) if *index == written)
            .then_some(effective_index)
        });
        let first = positions.next()?;
        if positions.next().is_some() {
            return None;
        }
        first
    } else {
        // The whole expression command's single source operand remains a
        // distinct purpose from one condition of a structured command.
        if selected.facts.successful_handler
            != Some(tcl_registry::native_compilation::SuccessfulHandlerSpec::ExpressionArguments)
            || selected.effective.words.len() != 2
            || expressions.arguments != [0]
        {
            return None;
        }
        1
    };
    let InvocationWordOrigin::Written(original) =
        *selected.effective.origins.get(effective_index)?
    else {
        return None;
    };
    let config = binding.original_lexer_config_for_tokens(tokens)?;
    let dialect = advice.dialect();
    let mut parser = dialect.expression_parse_context(None);
    parser.lexer_grammar = config.grammar_over(parser.lexer_grammar);
    Some(OriginalExpressionContextAdvice {
        parser,
        dialect,
        written: original,
        lookup_closed: if logical_source {
            advice.closed_logical_source_lookup()
        } else {
            advice.closed_lookup()
        },
        pool: advice.expression_pool_state(),
        frame: advice.frame().clone(),
        namespace_key: advice.namespace().clone(),
    })
}

fn original_expression_operand_advice_selected_word(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
    written: Option<usize>,
) -> Option<OriginalExpressionOperandAdvice> {
    let context = original_expression_context_advice_selected_word(registry, tokens, written)?;
    let binding = tokens.source_binding.as_ref()?;
    let index = context.written;
    let (WordExpr::BracedLiteral { text, source } | WordExpr::Literal { text, source }) =
        tokens.words().get(index)?
    else {
        return None;
    };
    if source.provenance != tcl_source::words::Provenance::Source {
        return None;
    }
    let base = crate::lowering_hooks::word_content_base(
        *tokens.argv.get(index)?,
        *tokens.single_token_word.get(index)?,
        text,
    )?;
    let end = base.checked_add(u32::try_from(text.len()).ok()?)?;
    let site = binding.invocation_site()?;
    if site
        .source
        .source_image()
        .bytes()
        .get(base as usize..end as usize)
        != Some(text.as_bytes())
    {
        return None;
    }
    let parser = context.parser;
    let evaluation =
        tcl_registry::conditional_expression::ConditionalExpressionEvaluation::prepare(
            text, &parser,
        )?;
    let expression = evaluation.tree().clone();
    Some(OriginalExpressionOperandAdvice {
        expression,
        dialect: context.dialect,
        evaluation,
        pool: context.pool,
        expression_base: base,
        expression_text: text.clone(),
        lookup_closed: context.lookup_closed,
        frame: context.frame,
        namespace_key: context.namespace_key,
        span: tcl_lexer::Span::new(
            tokens.words().first()?.source().span.start(),
            source.span.end(),
        ),
    })
}
