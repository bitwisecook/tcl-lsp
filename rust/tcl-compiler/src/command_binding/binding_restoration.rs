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

//! Restoring retained bindings onto a script and stamping source proofs onto its command words.

use super::command_identity::CommandAllocationSite;
use super::{
    OriginalSourceAnalysisOwner, SourceCommandBindings, SourceInvocationBinding, compiler_inventory,
};
use crate::ir::Script;
use std::collections::BTreeSet;
use std::sync::Arc;

impl SourceCommandBindings {
    /// Restore carriers from their selected original source instance.
    /// Native compiler obligations must match that boundary in full and stay
    /// unresolved. Phase-region dependencies require a separate relocation
    /// contract and are declined here instead of retaining stale provenance.
    pub fn restore_script_bindings(&self, script: &mut Script) -> bool {
        let mut valid = true;
        crate::ir::for_each_script_mut(script, &mut |script| {
            let Some(source) = self.restored_script_source(script) else {
                valid = false;
                return;
            };
            let Some(namespace) = script
                .namespace_context
                .as_deref()
                .cloned()
                .or_else(|| self.executed_script_namespace_context(&source))
            else {
                valid = false;
                return;
            };
            let Some(bindings) = self.selected_source_in_context(&source, &namespace) else {
                valid = false;
                return;
            };
            // Restoring source bindings does not resolve a compiler entry.
            // Retain only the complete obligation already recorded at this
            // original boundary; a cache cannot supply or repair its source,
            // required provider, failure contexts or dependency generations.
            let failure = bindings.native_compilation_failure_at(source.base());
            let provider_required = bindings.native_compilation_provider_required_at(source.base())
                || failure.is_some();
            let original_obligation = match script.native_compilation_admission.as_deref() {
                Some(admission) => {
                    admission.provider_required == provider_required
                        && admission.failure.as_deref() == failure
                        && admission.source.as_ref() == Some(&source)
                        && bindings.native_compilation_source_at(source.base()) == Some(&source)
                }
                None => !provider_required,
            };
            if script.native_compilation_failure.as_deref() != failure || !original_obligation {
                valid = false;
                return;
            }
            script.namespace_context = Some(Box::new(namespace));
            {
                script.implicit_math_invocations =
                    bindings.implicit_math_invocations_for_script(&source);
                script.expression_preparations =
                    bindings.expression_preparations_for_script(&source);
                script.executed_source = Some(Arc::new(source));
            }
            for site in script.command_binding_sites.iter_mut() {
                let proof = bindings.invocation_at_source(&site.binding.name, site.span.start());
                if !proof.proved_execution_target().is_some_and(|target| {
                    target.registry_backed
                        && tcl_registry::command_descriptor_labels_match(
                            &target.command,
                            &site.binding.identity,
                        )
                }) {
                    valid = false;
                }
                site.known_namespaces = Some(proof.known_namespaces);
                site.variable_frame = Some(proof.variable_frame);
                site.variable_context = Some(proof.variable_context);
                site.existing_namespace_cells = Some(proof.existing_namespace_cells);
                if let Some(tokens) = &mut site.source_tokens {
                    tokens.source_binding =
                        Some(bindings.invocation_at_source("", site.span.start()));
                    bindings.stamp_original_logical_declaration_targets(tokens);
                    tokens.variable_accesses =
                        bindings.variable_accesses_during_expression_invocation(site.span.start());
                    bindings.restore_original_nested_bindings(tokens);
                }
            }
            for statement in &mut script.statements {
                let span = statement.span();
                let offset = span.start();
                let Some(tokens) = statement.tokens_mut() else {
                    continue;
                };
                if tokens.evaluated_body.is_some() {
                    valid = false;
                }
                tokens.source_binding = Some(bindings.invocation_at_source("", offset));
                bindings.stamp_original_logical_declaration_targets(tokens);
                tokens.variable_accesses =
                    bindings.variable_accesses_during_expression_invocation(offset);
                bindings.restore_original_nested_bindings(tokens);
            }
        });
        valid
    }

    /// Issue a current source-syntax attachment from this collector's actual
    /// compiler visits, independently of an offset-zero cached body's owner.
    /// This does not supply Native admission or alter the refreshed raw facts.
    pub(super) fn restored_child_analysis_owner(
        &self,
        offset: u32,
        binding: &SourceInvocationBinding,
    ) -> Option<Arc<OriginalSourceAnalysisOwner>> {
        let owner = compiler_inventory::trace_compiler_syntax_option(
            "restore.owner",
            Some(offset),
            self.original_analysis_owner.as_ref(),
        )?;
        let site = compiler_inventory::trace_compiler_syntax_option(
            "restore.site",
            Some(offset),
            binding.invocation_site(),
        )?;
        let input = compiler_inventory::trace_compiler_syntax_option(
            "restore.input",
            Some(offset),
            owner.metadata.source_analysis_input(),
        )?;
        let registry = input.borrowed_context_registry().commands();
        let (_, original) = compiler_inventory::trace_compiler_syntax_option(
            "restore.recorded",
            Some(offset),
            binding.original_recorded_command(),
        )?;
        if compiler_inventory::trace_compiler_syntax_flag(
            "restore.site-offset",
            Some(offset),
            site.offset != offset,
        ) || compiler_inventory::trace_compiler_syntax_flag(
            "restore.site-owner",
            Some(offset),
            !Arc::ptr_eq(&site.source, &owner.source),
        ) || compiler_inventory::trace_compiler_syntax_flag(
            "restore.head-offset",
            Some(offset),
            compiler_inventory::trace_compiler_syntax_option(
                "restore.head",
                Some(offset),
                original.words().first(),
            )?
            .source()
            .span
            .start()
                != offset,
        ) || compiler_inventory::trace_compiler_syntax_flag(
            "restore.synthetic",
            Some(offset),
            original.synthetic.is_some(),
        ) || compiler_inventory::trace_compiler_syntax_flag(
            "restore.word-vector",
            Some(offset),
            !original.original_words_match_argv_text(),
        ) || compiler_inventory::trace_compiler_syntax_flag(
            "restore.logical",
            Some(offset),
            input.has_logical_source_name_context(),
        ) || compiler_inventory::trace_compiler_syntax_flag(
            "restore.hosted",
            Some(offset),
            input.has_hosted_source_name_context(),
        ) || compiler_inventory::trace_compiler_syntax_flag(
            "restore.config",
            Some(offset),
            binding.original_lexer_config_for_tokens(&original) != Some(input.lexer_config()),
        ) || compiler_inventory::trace_compiler_syntax_flag(
            "restore.source-image",
            Some(offset),
            !self.matches_original_source_image(owner.source.source_image(), input.lexer_config()),
        ) || compiler_inventory::trace_compiler_syntax_flag(
            "restore.raw-binding",
            Some(offset),
            binding != &self.invocation_at_source("", offset),
        ) {
            return None;
        }
        compiler_inventory::trace_compiler_syntax_option(
            "restore.visits",
            Some(offset),
            self.original_compiler_syntax_visits_for_owner(
                &original,
                input,
                registry,
                owner,
                Some(offset),
            ),
        )?;
        Some(Arc::clone(owner))
    }

    /// Refresh only the retained child inventory, then re-seal an original
    /// Logical declaration through its existing full source/vector issuer.
    /// Unmatched children keep the ordinary raw query; no offset is added.
    pub(super) fn restore_original_nested_bindings(&self, tokens: &mut crate::ir::CommandTokens) {
        // naming.compiler.original-positioned-callee-body-availability
        // docs/design/analysis/name-resolution-proofs/original-positioned-callee-body-availability.md
        for (offset, binding) in tokens.nested_bindings_mut() {
            *binding = self.invocation_at_source("", *offset);
        }
        let Some(config) = self.lexer_config else {
            return;
        };
        let Some(parent) = tokens.source_binding.as_ref() else {
            return;
        };
        let Some(site) = parent.invocation_site() else {
            return;
        };
        let mut original_parent = parent.clone();
        original_parent.original_logical_declaration_targets = None;
        if parent.original_lexer_config_for_tokens(tokens) != Some(config)
            || self.root_origin.as_ref() != Some(&site.source)
            || original_parent != self.invocation_at_source("", site.offset)
        {
            return;
        }
        let Some(calls) = crate::word_subst::checked_lifted_calls(tokens, config) else {
            return;
        };
        for index in 0..tokens.nested_bindings.len() {
            let offset = tokens.nested_bindings[index].0;
            if tokens
                .nested_bindings
                .iter()
                .filter(|(site, _)| *site == offset)
                .count()
                != 1
            {
                continue;
            }
            if tokens.words().iter().any(|word| {
                word.source().span.start() <= offset && offset <= word.source().span.end()
            }) {
                let owner =
                    self.restored_child_analysis_owner(offset, &tokens.nested_bindings[index].1);
                tokens.nested_bindings_mut()[index]
                    .1
                    .original_analysis_owner = owner;
            }
            let mut matching = calls.iter().filter_map(|call| {
                let child = call.tokens.as_ref()?;
                child
                    .words()
                    .first()
                    .is_some_and(|head| head.source().span.start() == offset)
                    .then_some(child)
            });
            let Some(original) = matching.next() else {
                continue;
            };
            if matching.any(|other| other.words() != original.words()) {
                continue;
            }
            let mut child = original.clone();
            child.source_binding = Some(tokens.nested_bindings[index].1.clone());
            self.stamp_original_logical_declaration_targets(&mut child);
            if let Some(binding) = child.source_binding {
                tokens.nested_bindings_mut()[index].1 = binding;
            }
        }
    }

    pub(super) fn compiled_invocation_offsets_in_words(
        &self,
        words: &[crate::ir::WordExpr],
        offset: u32,
    ) -> Vec<u32> {
        let Some(root) = self.root_origin.as_ref() else {
            return Vec::new();
        };
        let mut selected = BTreeSet::new();
        for word in words {
            let span = word.source().span;
            if span.start() > span.end() {
                continue;
            }
            let start = CommandAllocationSite {
                source: Arc::clone(root),
                offset: span.start(),
            };
            let end = CommandAllocationSite {
                source: Arc::clone(root),
                offset: span.end(),
            };
            selected.extend(
                self.compiler_invocations
                    .range(start..=end)
                    .filter(|(site, _)| {
                        self.root_origin.as_ref() == Some(&site.source)
                            && site.offset != offset
                            && words.iter().any(|word| {
                                word.source().span.start() <= site.offset
                                    && site.offset <= word.source().span.end()
                            })
                    })
                    .map(|(site, _)| site.offset),
            );
        }
        selected.into_iter().collect()
    }

    /// Attach exact retained source proofs to original command words. Missing
    /// reached sites stay explicitly unknown; this never constructs entry facts.
    pub fn stamp_original_tokens(&self, tokens: &mut crate::ir::CommandTokens) {
        self.stamp_original_tokens_with_bindings(tokens, |offset| {
            self.invocation_at_source("", offset)
        });
    }

    /// Retain the same original offset inventory and attachments while an
    /// immutable owner supplies its complete derived point projection.
    /// Mutable builders use the ordinary query and retain no projection memo.
    pub(crate) fn stamp_original_tokens_with_bindings(
        &self,
        tokens: &mut crate::ir::CommandTokens,
        mut binding_at: impl FnMut(u32) -> SourceInvocationBinding,
    ) {
        // Implementation contract: naming.source.original-invocation-projection-memo
        // docs/design/analysis/name-resolution-proofs/original-invocation-projection-memo.md
        let Some(head) = tokens.words().first() else {
            return;
        };
        let offset = head.source().span.start();
        tokens.variable_accesses = self.variable_accesses_during_expression_invocation(offset);
        tokens.nested_bindings = Arc::default();
        let config = self.lexer_config.unwrap_or_default();
        let argument_syntax = self.original_argument_syntax_for_tokens(tokens, config);
        for lifted in crate::word_subst::lifted_calls(Some(tokens), config) {
            if let Some(mut nested) = lifted.tokens
                && let Some(head) = nested.words().first()
            {
                let offset = head.source().span.start();
                nested.source_binding = Some(binding_at(offset));
                self.stamp_original_logical_declaration_targets(&mut nested);
                let syntax = argument_syntax
                    .as_ref()
                    .and_then(|syntax| syntax.child(&nested));
                if let Some(binding) = &mut nested.source_binding {
                    binding.original_argument_syntax = syntax.map(Arc::new);
                }
                if let Some(binding) = nested.source_binding {
                    tokens.nested_bindings_mut().push((offset, binding));
                }
            }
        }
        let mut retained = tokens
            .nested_bindings
            .iter()
            .map(|(offset, _)| *offset)
            .collect::<BTreeSet<_>>();
        let nested_offsets = tokens
            .words()
            .iter()
            .flat_map(|word| {
                let span = &word.source().span;
                self.dispatch_points
                    .range(span.start()..=span.end())
                    .map(|(offset, _)| *offset)
            })
            .filter(|nested| *nested != offset)
            .collect::<BTreeSet<_>>();
        let compiled_offsets = self.compiled_invocation_offsets_in_words(tokens.words(), offset);
        for nested in nested_offsets.into_iter().chain(compiled_offsets) {
            if retained.insert(nested) {
                tokens
                    .nested_bindings_mut()
                    .push((nested, binding_at(nested)));
            }
        }
        tokens.source_binding = Some(binding_at(offset));
        self.stamp_original_logical_declaration_targets(tokens);
        for binding in tokens.source_binding.iter_mut().chain(
            Arc::make_mut(&mut tokens.nested_bindings)
                .iter_mut()
                .map(|(_, binding)| binding),
        ) {
            binding
                .original_analysis_owner
                .clone_from(&self.original_analysis_owner);
            binding.original_source_operand_attachment = None;
        }
        if let Some(binding) = &mut tokens.source_binding {
            binding.original_argument_syntax = argument_syntax.map(Arc::new);
        }
        // This attachment authenticates the actual current declaration inventory;
        // a cloned analysis with withdrawn rows cannot reuse a derived cache.
        let attachment = self.original_declaration_source_operand_attachment(tokens);
        if let Some(binding) = &mut tokens.source_binding {
            binding.original_source_operand_attachment = attachment;
        }
    }
}
