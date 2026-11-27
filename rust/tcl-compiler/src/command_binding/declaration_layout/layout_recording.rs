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

//! Recording declaration layouts during the walk: receiver catalogues, suffixes and operands.

use super::super::{
    CommandAllocationSite, ModuleCommandBindings, SourceCommandBindings, SourceExecutionContext,
    SourceLookupSnapshot,
};
use super::variable_symbols::root_diagnostic_namespace;
use super::{
    DeclarationLayoutIssuer, DeclarationLayoutObservation, OriginalDiagnosticFrameEntry,
    OriginalLogicalDeclarationTargets, OriginalLogicalReceiverSourceBody,
    original_declaration_layouts,
};
use std::collections::BTreeMap;
use std::sync::Arc;

impl SourceCommandBindings {
    /// Called receivers retain a separate declaration preview for source roles.
    /// Only catalogue rows leave the collector; entered observations stay separate.
    pub(in super::super) fn retain_called_receiver_source_catalogues(
        &mut self,
        incoming: &ModuleCommandBindings,
        config: tcl_lexer::LexerConfig,
        registry: &tcl_registry::CommandRegistry,
    ) {
        let mut entries = BTreeMap::new();
        for (body, entry) in self.deferred_entry_states(incoming) {
            if !body.receiver_method
                || body.event.is_some()
                || body.future_frame.is_some()
                || body.statics.is_some()
                || !self
                    .called_implementations
                    .contains(&body.implementation_id())
            {
                continue;
            }
            let key = body.implementation_id();
            match entries.entry(key) {
                std::collections::btree_map::Entry::Vacant(slot) => {
                    slot.insert(Some((body, entry)));
                }
                std::collections::btree_map::Entry::Occupied(mut slot) => {
                    if slot.get().as_ref().is_some_and(|(previous, state)| {
                        previous != &body || !state.same_state(&entry)
                    }) {
                        slot.insert(None);
                    }
                }
            }
        }
        if entries.is_empty() {
            return;
        }
        let mut conditional = self.clone();
        conditional.active_calls.clear();
        conditional.active_variable_observers.clear();
        let mut declarations = std::collections::BTreeSet::new();
        for (body, entry) in entries.into_values().flatten() {
            let Some((declaration, _, _)) = &body.executed_script else {
                continue;
            };
            declarations.insert(declaration.clone());
            conditional.walk_deferred_entry(&body, &entry, config, registry);
        }
        for (site, observations) in &conditional.declaration_layouts {
            let Some(originals) = original_declaration_layouts(observations) else {
                continue;
            };
            for original in originals {
                let OriginalDiagnosticFrameEntry::DeclaredReceiver(body) = original.entry.as_ref()
                else {
                    continue;
                };
                if !declarations.contains(body.declaration()) {
                    continue;
                }
                let mut retained = original.clone();
                retained.issuer = DeclarationLayoutIssuer::ConditionalReceiverSource;
                let rows = self.declaration_layouts.entry(site.clone()).or_default();
                if !rows.contains(&retained) {
                    rows.push(retained);
                }
            }
        }
    }

    pub(in super::super) fn retain_logical_receiver_source_catalogues(
        &mut self,
        config: tcl_lexer::LexerConfig,
        registry: &tcl_registry::CommandRegistry,
    ) {
        let Some(input) = self.original_logical_source_name_advice_input().cloned() else {
            return;
        };
        let Some(origin) = self.source_origin().cloned() else {
            return;
        };
        if !input.has_logical_source_name_context()
            || input.lexer_config() != config
            || input
                .borrowed_context_registry()
                .commands()
                .snapshot()
                .semantic_key()
                != registry.snapshot().semantic_key()
        {
            return;
        }
        let Some(tape) = self.original_source_transition_advice_tape_for(
            input.borrowed_context_registry(),
            super::super::OriginalSourceCataloguePurpose::CompletePrefix,
        ) else {
            return;
        };
        let Some(commands) = tape.logical_receiver_source_commands(&origin, &input) else {
            return;
        };
        for (body, command) in commands {
            self.retain_one_logical_receiver_source_catalogue(
                body, &command, &origin, &input, config,
            );
        }
    }

    fn retain_one_logical_receiver_source_catalogue(
        &mut self,
        body: Arc<crate::registry_invocation::OriginalSourceDefinitionMemberScriptBody>,
        command: &Arc<crate::registry_invocation::OriginalSourceCommandProjection>,
        origin: &Arc<super::super::SourceOriginId>,
        input: &crate::analyser::ResolvedAnalysisInput,
        config: tcl_lexer::LexerConfig,
    ) {
        let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(
            parent,
        ) = body.original_parent_body().source_words().source()
        else {
            return;
        };
        if parent.logical_source_input() != Some(input)
            || !Arc::ptr_eq(&parent.site().source, origin)
            || !parent.matches_source(origin.source_image(), config)
            || !parent.matches_context(input.borrowed_context_registry())
        {
            return;
        }
        // Only the actual retained parent declaration supplies this
        // lineage. A fresh root state cannot stand in for a missing row.
        let Some(parents) = self
            .declaration_layouts
            .get(parent.site())
            .and_then(|rows| original_declaration_layouts(rows))
            .map(|rows| rows.cloned().collect::<Vec<_>>())
        else {
            return;
        };
        if parents.iter().any(|row| {
            row.config != config
                || row.snapshot.state.logical_source_name_advice_input() != Some(input)
                || !row.entry.owns_source(origin, parent.site().offset)
                || !row
                    .entry
                    .owns_original_context(&row.snapshot.state.source_variables)
                || row.snapshot.state.variable_frame != *row.entry.frame()
                || row.snapshot.state.current_source_origin.as_ref() != Some(origin)
                || origin
                    .original_lexical_command(&row.words, parent.site().offset, config)
                    .is_none_or(|original| original.native_words() != parent.original_words())
        }) {
            return;
        }
        let content = body.content_span();
        let Some(bytes) = origin.source_image().bytes().get(content.as_range()) else {
            return;
        };
        // This facet retains unchanged original body geometry. Transformed
        // values require their distinct source correspondence owner.
        if tcl_syntax::word_rules::original_static_word_unicode_value(body.original_container())
            .as_deref()
            != Some(bytes)
        {
            return;
        }
        let Some(source) = super::super::ExecutedScriptSource::contiguous_image(
            Arc::clone(origin),
            tcl_lexer::SourceImage::from_bytes(bytes, origin.source_image().channel()),
            content.start(),
        ) else {
            return;
        };
        let Some(member) = body.original_words().first() else {
            return;
        };
        let member_offset = member.span().start();
        let entry = Arc::new(OriginalDiagnosticFrameEntry::LogicalReceiver(Arc::new(
            OriginalLogicalReceiverSourceBody {
                body,
                input: input.clone(),
                source,
                frame: crate::var_resolve::VariableExecutionFrame::ReceiverMethod {
                    identity: super::super::source_activation_name(
                        Some(origin),
                        "logical-receiver-source-layout",
                        member_offset,
                    ),
                },
            },
        )));
        let site = CommandAllocationSite {
            source: Arc::clone(origin),
            offset: command.native_words()[0].span().start(),
        };
        let tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::from_image(origin.source_image()),
            config,
            command.command(),
        );
        let rows = self.declaration_layouts.entry(site).or_default();
        for parent in parents {
            let row = DeclarationLayoutObservation {
                issuer: DeclarationLayoutIssuer::ConditionalReceiverSource,
                entry: Arc::clone(&entry),
                // The held snapshot/namespace belong to the parent
                // declaration. LogicalReceiver cannot project them as a
                // receiver lookup namespace or entered naming context.
                snapshot: parent.snapshot,
                namespace: parent.namespace,
                config,
                words: Arc::from(tokens.words()),
            };
            if !rows.contains(&row) {
                rows.push(row);
            }
        }
    }

    /// Retain original diagnostic layouts after an abrupt source prefix.
    /// The actual abrupt table is borrowed unchanged. No arguments, dispatch,
    /// compiler visits or source effects are evaluated for this suffix.
    pub(in super::super) fn record_unentered_declaration_suffix(
        &mut self,
        image: &tcl_lexer::SourceImage,
        base: u32,
        segments: &[tcl_source::segment::SegmentedCommand],
        state: &ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) {
        let snapshot = Arc::new(SourceLookupSnapshot::in_realm(
            state.clone(),
            context.realm(),
        ));
        for segment in segments {
            if segment.is_partial {
                break;
            }
            let tokens =
                super::super::source_command_tokens_boxed(image, base, context.config, segment);
            self.record_declaration_operand_layout(state, context, segment.span.start(), &tokens);
            let Some(origin) = state.current_source_origin.as_ref() else {
                break;
            };
            let site = CommandAllocationSite {
                source: Arc::clone(origin),
                offset: segment.span.start(),
            };
            let Some(advice) = super::super::original_site_operand_layout_advice(
                &site,
                &tokens,
                &snapshot,
                &context.namespace_identity(),
                context.config,
            ) else {
                break;
            };
            let Some(selected) = crate::registry_invocation::declaration_invocation_flow(
                context.registry,
                &tokens,
                &advice,
            ) else {
                break;
            };
            // Without evaluating the command, a changed table or a substituted
            // operand cannot supply a lookup snapshot for its successors.
            if !selected.effects.lookup_stable
                || selected.effects.unknown_writes
                || !matches!(
                    selected.flow,
                    tcl_registry::script_body_flow::ScriptBodyFlow::None
                )
                || selected.effective.words.iter().any(|word| {
                    crate::registry_invocation::effective_invocation_word(
                        word,
                        advice.dialect().lexer_grammar.escapes,
                        advice.dialect().word_values,
                    )
                    .literal_bytes()
                    .is_none()
                })
            {
                break;
            }
        }
    }

    /// Capture before evaluating any operand, using the original declaration
    /// owner or its retained entered-body frame and the current source table.
    /// Earlier replacements remain visible.
    #[inline(never)]
    pub(in super::super) fn record_declaration_operand_layout(
        &mut self,
        state: &ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        offset: u32,
        words: &crate::ir::CommandTokens,
    ) {
        #[cfg(any(test, debug_assertions))]
        let _timing =
            super::super::root_owner_work::Timing::at(super::super::root_owner_work::Query::Layout);
        if state.source_variables.invocation_dialect.is_none() {
            return;
        }
        let Some(origin) = state.current_source_origin.as_ref() else {
            return;
        };
        let (entry, namespace) = if let Some(body) = self.conditional_body_entry_at(origin, offset)
        {
            if !self.original_body_owns_frame(&body, &state.variable_frame) {
                return;
            }
            (
                OriginalDiagnosticFrameEntry::Body(body),
                context.namespace_identity(),
            )
        } else if let Some(body) =
            super::super::declaration_preview::logical_body_entry::current_entry(
                state, context, offset,
            )
        {
            (
                OriginalDiagnosticFrameEntry::DeclaredProcedure(body),
                context.namespace_identity(),
            )
        } else if let Some(body) = self.declared_receiver_body_entry_at(origin, offset) {
            if body.preview_frame() != &state.variable_frame {
                return;
            }
            let namespace = body.declaration_namespace_context().clone();
            (
                OriginalDiagnosticFrameEntry::DeclaredReceiver(body),
                namespace,
            )
        } else if let Some(entry) =
            self.original_script_activation_at(state, context, origin, offset)
        {
            (entry, context.namespace_identity())
        } else {
            // A nested parser issues no frame. An immediately selected nested
            // original script (a command substitution, an `expr` operand's
            // `[…]`) still runs in this root frame and namespace, so it keeps
            // the root entry only for its own genuine original vector.
            // Conditional/future bodies and materialised text never qualify.
            if context.source_purpose() != tcl_lexer::SourceSegmentationPurpose::WholeInput
                && (context.body_entry() != super::super::SourceBodyEntryPurpose::Selected
                    || origin
                        .original_lexical_command(words.words(), offset, context.config)
                        .is_none())
            {
                return;
            }
            let Some(namespace) = root_diagnostic_namespace(state, &context.namespace_identity())
            else {
                return;
            };
            if self.root_origin.as_ref() != Some(origin) {
                return;
            }
            let Some(source) = super::super::ExecutedScriptSource::contiguous_image(
                Arc::clone(origin),
                origin.source_image().clone(),
                0,
            ) else {
                return;
            };
            (
                OriginalDiagnosticFrameEntry::RootScript {
                    source,
                    frame: state.variable_frame.clone(),
                    namespace: namespace.clone(),
                },
                namespace,
            )
        };
        let entry = Arc::new(entry);
        let observation = DeclarationLayoutObservation {
            issuer: if state.variable_frame == *entry.frame() {
                DeclarationLayoutIssuer::OriginalDeclaration
            } else {
                DeclarationLayoutIssuer::EnteredActivation
            },
            entry,
            snapshot: Arc::new(SourceLookupSnapshot::in_realm(
                state.clone(),
                context.realm(),
            )),
            namespace,
            config: context.config,
            words: Arc::from(words.words()),
        };
        let observations = self
            .declaration_layouts
            .entry(CommandAllocationSite {
                source: Arc::clone(origin),
                offset,
            })
            .or_default();
        if !observations.contains(&observation) {
            observations.push(observation);
        }
    }

    /// Retain only original declaration observations from a separate preview.
    /// Actual dispatch, reads, effects, coverage and compiler receipts stay in
    /// their independently entered inventories.
    pub(in super::super) fn merge_original_declaration_layouts(&mut self, preview: &Self) {
        for (site, observations) in &preview.declaration_layouts {
            let Some(originals) = original_declaration_layouts(observations) else {
                continue;
            };
            let retained = self.declaration_layouts.entry(site.clone()).or_default();
            for observation in originals {
                if !retained.contains(observation) {
                    retained.push(observation.clone());
                }
            }
        }
    }

    /// Seal only the actual original Logical declaration vector, after its
    /// ordinary token attachments. A reached point keeps its existing dispatch
    /// guards; an absent point retains unknown runtime lookup unchanged.
    pub(in super::super) fn stamp_original_logical_declaration_targets(
        &self,
        tokens: &mut crate::ir::CommandTokens,
    ) {
        // naming.compiler.original-positioned-callee-body-availability
        // docs/design/analysis/name-resolution-proofs/original-positioned-callee-body-availability.md
        let Some(binding) = tokens.source_binding.as_ref() else {
            return;
        };
        let Some(site) = binding.invocation_site() else {
            return;
        };
        if binding.lookup_state.is_some()
            || binding.original_logical_declaration_targets.is_some()
            || self.root_origin.as_ref() != Some(&site.source)
            || self.dispatch_points_at(site.offset).next().is_some()
            || binding != &self.invocation_at_source("", site.offset)
        {
            return;
        }
        let Some(advice) = binding.original_logical_declaration_layout_advice(tokens) else {
            return;
        };
        if !advice.closed_logical_source_lookup() {
            return;
        }
        let receipt = OriginalLogicalDeclarationTargets {
            original_binding: Box::new(binding.clone()),
            targets: advice.targets().to_vec(),
        };
        if let Some(binding) = tokens.source_binding.as_mut() {
            binding.original_logical_declaration_targets = Some(Arc::new(receipt));
        }
    }

    pub(in super::super) fn attach_declaration_operand_layout(
        &self,
        mut binding: super::super::SourceInvocationBinding,
        origin: Option<&Arc<super::super::SourceOriginId>>,
        offset: u32,
    ) -> super::super::SourceInvocationBinding {
        binding.declaration_layout_observations = origin.and_then(|origin| {
            self.declaration_layouts
                .get(&CommandAllocationSite {
                    source: Arc::clone(origin),
                    offset,
                })
                .map(|observations| Arc::from(observations.as_slice()))
        });
        binding.declaration_flow_inventory = binding
            .declaration_layout_observations
            .as_ref()
            .map(|_| self.current_declaration_flow_inventory());
        // Original geometry remains available even when no argv or dispatch
        // was reached. Its issuer is the immutable declaration observation;
        // targets, runtime reachability and compiler authority stay unchanged.
        if binding.dispatch_site.is_none() && binding.declaration_layout_observations.is_some() {
            binding.dispatch_site = origin.map(|source| CommandAllocationSite {
                source: Arc::clone(source),
                offset,
            });
        }
        binding
    }
}
