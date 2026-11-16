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

//! Whether the source spelling of an expression operand may drive XC advice.

use super::TranslationContext;
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::ir::Module;
use tcl_compiler::registry_invocation::source_structure::{
    OriginalRegistryWords, original_single_source_command_substitution,
};
use tcl_lexer::{NativeWord, Span, native_script_words_in};
use tcl_registry::CommandRegistry;
use tcl_syntax::expr::ast::ExprNode;

pub(super) fn source_xc_advice_allowed(
    words: &OriginalRegistryWords,
    analysis: &AnalysisResult,
) -> bool {
    let Some(input) = analysis.resolved_input.as_ref() else {
        return false;
    };
    words
        .with_source_schema(&input.context_registry(), |schema| {
            let descriptor = schema.authored_source_descriptors().command;
            matches!(
                tcl_registry::registry::spec_pack_of(descriptor),
                Some("irules" | "tcl")
            ) && descriptor.xc_translatable != Some(false)
        })
        .is_some_and(|allowed| allowed)
}

pub(super) struct SourceExpressionAdvice<'a> {
    pub(super) source: &'a str,
    pub(super) analysis: &'a AnalysisResult,
    pub(super) module: &'a Module,
    registry: &'a CommandRegistry,
    base: Option<u32>,
}

impl<'a> SourceExpressionAdvice<'a> {
    pub(super) fn new(context: &TranslationContext<'a>, base: Option<u32>) -> Self {
        Self {
            source: context.source,
            analysis: context.analysis,
            module: context.module,
            registry: context.registry,
            base,
        }
    }

    pub(super) fn original_operand(&self, span: Span) -> Option<NativeWord> {
        let mut plan =
            native_script_words_in(self.module.source.clone(), span, self.module.lexer_config)
                .ok()?;
        if plan.fatal_tail.is_some() || plan.commands.len() != 1 {
            return None;
        }
        let mut words = plan.commands.pop()?.words;
        if words.len() != 1 || words[0].span() != span || words[0].group().expand {
            return None;
        }
        words.pop()
    }

    pub(super) fn expression_operand(&self, node: &ExprNode) -> Option<NativeWord> {
        let ExprNode::Command { text, start, end } = node else {
            return None;
        };
        let base = self.base?;
        let span = tcl_syntax::expr::expression_span(
            self.source,
            base.checked_add(*start)?,
            base.checked_add(*end)?,
        )?;
        (self.source.get(span.as_range())? == text).then_some(())?;
        self.original_operand(span)
    }

    pub(super) fn command_for_operand(
        &self,
        operand: &NativeWord,
    ) -> Option<OriginalRegistryWords> {
        let selected = tcl_compiler::source_graph::registry_words_for_original_substitution(
            self.source,
            self.analysis,
            self.module,
            operand,
            self.registry,
        )?;
        source_xc_advice_allowed(&selected, self.analysis).then_some(selected)
    }

    pub(super) fn command(&self, node: &ExprNode) -> Option<OriginalRegistryWords> {
        self.command_for_operand(&self.expression_operand(node)?)
    }

    pub(super) fn parts(&self, node: &ExprNode) -> Option<Vec<String>> {
        let selected = self.command(node)?;
        let mut parts = vec![selected.command().to_owned()];
        for argument in selected.arguments() {
            parts.push(
                std::str::from_utf8(argument.literal_bytes()?)
                    .ok()?
                    .to_owned(),
            );
        }
        Some(parts)
    }

    pub(super) fn scalar_operand(&self, operand: &NativeWord) -> Option<String> {
        let mut selected = self.command_for_operand(operand)?;
        loop {
            if selected.arguments().is_empty() {
                return Some(selected.command().to_owned());
            }
            if selected.command() != "string" || selected.arguments().len() != 2 {
                return None;
            }
            let subcommand = selected.arguments()[0].literal_bytes()?;
            if !matches!(subcommand, b"tolower" | b"toupper") {
                return None;
            }
            selected = self.command_for_operand(selected.original_argument_word(1)?)?;
        }
    }

    pub(super) fn scalar(&self, node: &ExprNode) -> Option<String> {
        self.scalar_operand(&self.expression_operand(node)?)
    }

    pub(super) fn command_owners_available(&self, node: &ExprNode) -> bool {
        let mut nodes = vec![node];
        while let Some(node) = nodes.pop() {
            match node {
                ExprNode::Command { .. } => {
                    let Some(operand) = self.expression_operand(node) else {
                        return false;
                    };
                    let mut operands = vec![operand];
                    while let Some(operand) = operands.pop() {
                        if self.command_for_operand(&operand).is_none() {
                            return false;
                        }
                        let Some(original) = original_single_source_command_substitution(&operand)
                        else {
                            return false;
                        };
                        for word in &original.command().words {
                            for part in word.executable_parts().all_parts() {
                                if matches!(part.part, tcl_lexer::ExecutablePart::Command { .. }) {
                                    let Some(child) = self.original_operand(part.span) else {
                                        return false;
                                    };
                                    operands.push(child);
                                }
                            }
                        }
                    }
                }
                ExprNode::Binary { left, right, .. } => {
                    nodes.extend([left.as_ref(), right.as_ref()]);
                }
                ExprNode::Unary { operand, .. } => nodes.push(operand),
                ExprNode::Ternary {
                    condition,
                    true_branch,
                    false_branch,
                } => nodes.extend([
                    condition.as_ref(),
                    true_branch.as_ref(),
                    false_branch.as_ref(),
                ]),
                ExprNode::Call { args, .. } => nodes.extend(args),
                _ => {}
            }
        }
        true
    }

    pub(super) fn http_getter(&self, node: &ExprNode) -> Option<String> {
        self.scalar(node)
            .filter(|command| command.starts_with("HTTP::"))
    }
}
