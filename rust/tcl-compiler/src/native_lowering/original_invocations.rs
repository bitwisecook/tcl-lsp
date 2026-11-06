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

//! Original source carriers for `NativeLowering` descriptor queries.

use crate::ir::{CommandTokens, Module, Script, WordExpr};
use crate::registry_invocation::InvocationMetadataContext;
use tcl_registry::CommandRegistry;

/// Immutable source carriers, without dispatch or compiler admission.
#[derive(Default)]
pub(super) struct OriginalInvocations {
    commands: Vec<CommandTokens>,
    substitutions: Vec<CommandTokens>,
}

impl OriginalInvocations {
    pub(super) fn for_module(
        module: &Module,
        registry: &CommandRegistry,
        source: &str,
    ) -> Option<Self> {
        (module.source.bytes() == source.as_bytes()).then_some(())?;
        let retained = module.retained_source_bindings.as_deref()?;
        retained.positioned_for_module(module, registry)?;
        let metadata = InvocationMetadataContext::for_module(registry, module)?;
        metadata.source_analysis_input()?;
        let mut inventory = Self::default();
        let mut pending: Vec<&Script> = vec![&module.top_level];
        for (id, procedure) in module.original_source_procedure_bodies() {
            if retained.matches_original_procedure_summary(*id, procedure) {
                pending.push(&procedure.body);
            }
        }
        while let Some(script) = pending.pop() {
            for statement in &script.statements {
                pending.extend(statement.child_scripts());
                let Some(tokens) = script.retained_source_tokens_for_statement(statement) else {
                    continue;
                };
                if !retained.owns_original_analysis_attachment(tokens) {
                    continue;
                }
                let Some(binding) = tokens.source_binding.as_ref() else {
                    continue;
                };
                let Some((command, original)) = binding.original_recorded_command() else {
                    continue;
                };
                if command.span != statement.span() || original.words() != tokens.words() {
                    continue;
                }
                inventory.commands.push(tokens.clone());
                if let Some(calls) = crate::word_subst::checked_original_argument_lifted_calls(
                    tokens, registry, metadata,
                ) {
                    inventory
                        .substitutions
                        .extend(calls.into_iter().filter_map(|call| call.tokens));
                }
            }
        }
        Some(inventory)
    }

    pub(super) fn command(
        &self,
        invoke: &crate::executable_ir::GenericInvoke,
    ) -> Option<&CommandTokens> {
        unanimous(self.commands.iter().filter(|tokens| {
            tokens.words() == invoke.original_words
                && tokens.source_binding == invoke.source_binding
                && tokens
                    .source_binding
                    .as_ref()
                    .and_then(super::super::command_binding::SourceInvocationBinding::original_recorded_command)
                    .is_some_and(|(command, _)| command.span == invoke.source.span)
        }))
    }

    pub(super) fn substitution(&self, words: &[WordExpr]) -> Option<&CommandTokens> {
        unanimous(
            self.substitutions
                .iter()
                .filter(|tokens| tokens.words() == words),
        )
    }
}

fn unanimous<'a>(mut tokens: impl Iterator<Item = &'a CommandTokens>) -> Option<&'a CommandTokens> {
    let first = tokens.next()?;
    tokens.all(|other| other == first).then_some(first)
}
