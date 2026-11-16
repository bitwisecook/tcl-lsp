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

//! Expression operands of a body: their commands, substitutions and traced roles.

use super::super::{AdviceGraph, AdviceNamingPolicy, OriginalSourceCommandTransitionAdvice};
use super::{BodyWalk, OriginalSourceExpressionCommand, boxed_region_words};
#[cfg(test)]
use crate::command_binding::source_transition_advice::body_effects::trace_body_inventory;
use std::sync::Arc;
use tcl_lexer::Span;

impl BodyWalk<'_, '_> {
    pub(super) fn expressions(
        &mut self,
        graph: &mut AdviceGraph,
        advice: &OriginalSourceCommandTransitionAdvice,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        depth: usize,
    ) -> Option<()> {
        self.expression_commands(graph, advice, schema, depth, false)
    }

    #[cfg(test)]
    fn trace_expression_roles(
        advice: &OriginalSourceCommandTransitionAdvice,
        complete: bool,
        roles: &[(u8, tcl_registry::ArgRole)],
    ) {
        trace_body_inventory(
            "expressions.roles",
            advice
                .original_words()
                .first()
                .map(|word| word.span().start()),
            complete,
            roles.len(),
        );
    }

    #[cfg(test)]
    fn trace_expression_value(
        argument: &super::super::SourceAdviceWord,
        expression_span: Span,
        source: &str,
    ) {
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
            trace_body_inventory(
                "expressions.original_value",
                Some(expression_span.start()),
                argument.value.as_deref() == Some(source.as_bytes()),
                source.len(),
            );
        }
    }

    #[cfg(test)]
    fn trace_expression_parts(
        expression_span: Span,
        substitutions: Option<&tcl_syntax::expr::LiveExpressionSubstitutions>,
    ) {
        trace_body_inventory(
            "expressions.checked",
            Some(expression_span.start()),
            substitutions.is_some(),
            substitutions.map_or(0, |parts| parts.commands.len()),
        );
    }

    #[cfg(test)]
    fn trace_expression_child(start: u32, end: u32, walked: Option<()>) {
        trace_body_inventory(
            "expressions.child",
            Some(start),
            walked.is_some(),
            (end - start) as usize,
        );
    }

    fn body_expression_context(&self) -> tcl_syntax::expr::parser::ExprParseContext {
        let mut context = self
            .context
            .dialect
            .expression_parse_context(self.context.context.commands().profile());
        context.lexer_grammar = self.context.config.grammar_over(context.lexer_grammar);
        context
    }

    pub(super) fn expression_commands(
        &mut self,
        graph: &mut AdviceGraph,
        advice: &OriginalSourceCommandTransitionAdvice,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        depth: usize,
        catalogue: bool,
    ) -> Option<()> {
        let (roles, complete) = self.context.authored_roles(schema);
        #[cfg(test)]
        Self::trace_expression_roles(advice, complete, &roles);
        if !complete {
            // A normal-source case owns only a subject and literal case list;
            // argv substitutions were inspected separately, never as Expr.
            let words = boxed_region_words(self.context, advice)?;
            self.normal_case_regions(&words, schema)?;
            return Some(());
        }
        for (ordinal, role) in roles {
            if role != tcl_registry::ArgRole::Expr {
                continue;
            }
            let ordinal = schema
                .facts()
                .argument_offset
                .checked_add(usize::from(ordinal))?;
            let argument = advice.arguments().get(ordinal)?;
            if !matches!(
                argument.origin,
                crate::registry_invocation::InvocationWordOrigin::Written(_)
            ) {
                return None;
            }
            let expression_span = argument.original.content_span().ok()?;
            let source = argument
                .original
                .image()
                .try_text()
                .ok()?
                .get(expression_span.as_range())?;
            // Cooked values and incomplete expressions cannot lend their
            // original positions to the checked expression/script bridge.
            #[cfg(test)]
            Self::trace_expression_value(argument, expression_span, source);
            if argument.value.as_deref() != Some(source.as_bytes()) {
                return None;
            }
            let context = self.body_expression_context();
            let substitutions = self.checked_body_expression_substitutions(graph, source, &context);
            #[cfg(test)]
            Self::trace_expression_parts(expression_span, substitutions.as_ref());
            let substitutions = substitutions?;
            // A condition of a selected first-match chain runs once, before any
            // clause body, so its command children are ordered by threading each
            // one's possible effects into the next. Variable reads (traces),
            // unwalked math-function calls and repeated or fixture-timed
            // conditions still leave the parent operation itself uncertain.
            let ordered = schema.semantics.clause_grammar.is_some_and(
                tcl_registry::ClauseGrammarSpec::evaluates_conditions_once_before_bodies,
            ) && tcl_syntax::expr::OriginalSourceExpression::capture(
                source,
                &context,
                self.context.config,
            )
            .is_some_and(|expression| expression.function_calls().is_empty());
            if !substitutions.variables.is_empty()
                || (!ordered && !substitutions.commands.is_empty())
            {
                graph.record_uncertainty(advice.original_words());
            }
            let original = (!ordered).then(|| graph.clone());
            for command in substitutions.commands {
                // The shared expression owner supplies whole brackets. Lazy
                // branches are lexical possibilities, never entered scripts.
                let start = expression_span
                    .start()
                    .checked_add(command.start())?
                    .checked_add(1)?;
                let end = expression_span
                    .start()
                    .checked_add(command.end())?
                    .checked_sub(1)?;
                let threaded;
                let base = if let Some(original) = &original {
                    original
                } else {
                    threaded = graph.clone();
                    &threaded
                };
                let mut branch = base.clone();
                let parent_catalogue = self.catalogue_body.clone();
                if catalogue {
                    let owner = OriginalSourceExpressionCommand {
                        parent: Arc::new(advice.clone()),
                        ordinal,
                        original: argument.original.clone(),
                        command: Span::new(start, end),
                    };
                    if !owner
                        .matches_source(self.context.origin.source_image(), self.context.config)
                        || !owner.matches_context(self.context.context)
                    {
                        return None;
                    }
                    self.catalogue_body = Some(
                        super::super::OriginalSourceCatalogueBody::Expression(Arc::new(owner)),
                    );
                }
                let walked = self.script(&mut branch, Span::new(start, end), depth.checked_add(1)?);
                self.catalogue_body = parent_catalogue;
                #[cfg(test)]
                Self::trace_expression_child(start, end, walked);
                walked?;
                Self::retain_possible_effects(graph, base, branch, advice.original_words());
            }
        }
        Some(())
    }

    fn checked_body_expression_substitutions(
        &self,
        graph: &AdviceGraph,
        source: &str,
        context: &tcl_syntax::expr::parser::ExprParseContext,
    ) -> Option<tcl_syntax::expr::LiveExpressionSubstitutions> {
        match self.context.policy {
            AdviceNamingPolicy::Logical(input) if input.has_logical_source_name_context() => {
                tcl_syntax::expr::checked_source_expression_substitutions(source, context)
            }
            AdviceNamingPolicy::Logical(_) => None,
            AdviceNamingPolicy::Native(_) => {
                if graph.c_source_namespace.is_some() && !graph.native_baseline {
                    tcl_syntax::expr::checked_source_expression_substitutions(source, context)
                } else {
                    tcl_syntax::expr::checked_expression_substitutions(source, context)
                }
            }
        }
    }
}
