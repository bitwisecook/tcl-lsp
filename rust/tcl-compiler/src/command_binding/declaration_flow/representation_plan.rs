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

//! Conditional representation plans: naming scopes, body contexts, targets, loops and captures.

use super::super::{CommandAllocationSite, ExecutedScriptMapping, ExecutedScriptSource};
use super::{
    Builder, ConditionalRepresentationBodyContext, ConditionalRepresentationBodyInputs,
    ConditionalRepresentationLoopPlan, ConditionalRepresentationNaming,
    ConditionalRepresentationPlan, ConditionalRepresentationSubstitution,
    ConditionalRepresentationTarget, DeclarationFlowFrame, DeclarationReadPointScope,
    representation,
};
use crate::ir::{CommandTokens, WordExpr};
use std::collections::VecDeque;
use std::sync::Arc;
use tcl_registry::script_body_flow::ScriptBodyFlow;

impl Builder<'_> {
    /// Naming geometry at this exact original root command. Later commands
    /// retain their own source and namespace applicability.
    pub(super) fn representation_root_naming_scope(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
        conditional_flow: Option<&crate::registry_invocation::DeclarationInvocationFlow>,
    ) -> Option<(
        crate::signature_scan::scope::SignatureNamespaceScope,
        tcl_syntax::naming::NamePolicyProtocol,
    )> {
        use super::super::declaration_layout::OriginalDiagnosticFrameEntry;
        let DeclarationFlowFrame::Existing(
            entry @ OriginalDiagnosticFrameEntry::RootScript {
                source,
                frame,
                namespace,
            },
        ) = self.entry
        else {
            return None;
        };
        let seed = self.inventory.source_seed.as_ref()?;
        let owner = seed.analysis_owner.as_ref()?;
        let input = seed.baseline.metadata_context.source_analysis_input()?;
        let metadata = crate::registry_invocation::InvocationMetadataContext::for_source_input(
            self.registry,
            input,
            self.config,
            Some(input.unit_profile()),
        )?;
        if *frame != crate::var_resolve::VariableExecutionFrame::Global
            || source.mapping != (ExecutedScriptMapping::Contiguous { base: 0 })
            || source.text != *source.origin.source_image()
            || !Arc::ptr_eq(&source.origin, &seed.origin)
            || !Arc::ptr_eq(&source.origin, &owner.source)
            || !Arc::ptr_eq(&site.source, &source.origin)
            || seed.config != self.config
            || owner.config != self.config
            || input.lexer_config() != self.config
            || owner.metadata != seed.baseline.metadata_context
            || owner.metadata.source_analysis_input() != Some(input)
            || seed.logical_input.is_some()
            || !seed.matches_registry(self.registry)
            || tokens.synthetic.is_some()
            || !tokens.original_words_match_argv_text()
        {
            return None;
        }
        let related = self.representation_naming_is_related(tokens, site, conditional_flow)?;
        let rows = super::super::declaration_layout::original_declaration_layouts(
            self.inventory.layouts.get(site)?,
        )?;
        let mut selected = None;
        for row in rows {
            let current_entry = if related {
                matches!(
                    row.entry.as_ref(),
                    OriginalDiagnosticFrameEntry::ScriptActivation { .. }
                )
            } else {
                row.entry.as_ref() == entry
            };
            if !current_entry
                || !Arc::ptr_eq(&row.entry.source().origin, &source.origin)
                || row.config != self.config
                || row.namespace != *namespace
                || row.snapshot.state.variable_frame != *frame
                || row.words.as_ref() != tokens.words()
                || !row
                    .snapshot
                    .state
                    .current_source_origin
                    .as_ref()
                    .is_some_and(|origin| Arc::ptr_eq(origin, &source.origin))
                || row
                    .snapshot
                    .state
                    .retained_original_source_purpose(metadata, self.registry)
                    != Some(super::super::OriginalInvocationSourcePurpose::AuthoredSimulation)
                || crate::registry_invocation::original_native_compiler_words(
                    source.origin.source_image(),
                    &row.words,
                    site.offset,
                    self.config,
                )
                .is_none()
            {
                return None;
            }
            let policy = row
                .snapshot
                .state
                .baseline
                .execution_name_policy?
                .native_recipe()?;
            if policy.authority() != tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation {
                return None;
            }
            let scope = row
                .snapshot
                .state
                .original_namespace_geometry(namespace, policy)?;
            let current = (scope, policy);
            if selected.as_ref().is_some_and(|prior| prior != &current) {
                return None;
            }
            selected = Some(current);
        }
        selected
    }

    fn representation_naming_is_related(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
        conditional_flow: Option<&crate::registry_invocation::DeclarationInvocationFlow>,
    ) -> Option<bool> {
        Some(match self.source_read_point_scope(tokens, site)? {
            DeclarationReadPointScope::Observed(_) => false,
            DeclarationReadPointScope::ConditionalRootRelated { .. } => {
                let flow = conditional_flow?;
                if self.source_read_selection(tokens, site).as_ref() != Some(flow) {
                    return None;
                }
                true
            }
            _ => return None,
        })
    }

    /// Revalidate a conditional body against its actual immediate source
    /// parent. The returned context is a premise, not the child's snapshot.
    pub(super) fn conditional_representation_body_scope(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
        selected: &crate::registry_invocation::DeclarationInvocationFlow,
    ) -> Option<Arc<ConditionalRepresentationBodyContext>> {
        let held = self.current_representation_body.as_ref()?;
        let seed = self.inventory.source_seed.as_ref()?;
        let owner = seed.analysis_owner.as_ref()?;
        let input = seed.baseline.metadata_context.source_analysis_input()?;
        if !selected.representation_no_known_counterevidence
            || !Arc::ptr_eq(owner, &held.owner)
            || !Arc::ptr_eq(&site.source, &held.site.source)
            || !Arc::ptr_eq(&site.source, &seed.origin)
            || input != &held.input
            || input.lexer_config() != self.config
            || !seed.matches_registry(self.registry)
            || !held.binder.direct_phase_preserves_lookup()
            || held.count.iterations() < 2
            || !Self::conditional_substitution_matches(held, tokens, site, self.config)
        {
            return None;
        }
        let DeclarationReadPointScope::ConditionalRootRelated { root, .. } =
            self.source_read_point_scope(tokens, site)?
        else {
            return None;
        };
        if !Arc::ptr_eq(&root.source().origin, &held.site.source)
            || self.source_read_selection(tokens, site).as_ref() != Some(selected)
        {
            return None;
        }
        let rows = super::super::declaration_layout::original_declaration_layouts(
            self.inventory.layouts.get(site)?,
        )?;
        if rows.into_iter().any(|row| {
            row.snapshot
                .state
                .baseline
                .execution_name_policy
                .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                != Some(held.policy)
        }) {
            return None;
        }
        let catalogue = self
            .inventory
            .source_read_catalogue(self.registry, self.purpose)?;
        catalogue.scope_with_ancestry(site, tokens, self.config, |advice, context| {
            self.conditional_representation_immediate_parent(held, advice, context)
        })?;
        Some(Arc::clone(held))
    }

    pub(super) fn conditional_substitution_matches(
        held: &ConditionalRepresentationBodyContext,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        let Some(substitution) = &held.substitution else {
            return true;
        };
        let original = &substitution.original;
        let mut expected_parent = held.clone();
        expected_parent.substitution = None;
        substitution.parent.as_ref() == &expected_parent
            && Arc::ptr_eq(&substitution.parent.site.source, &held.site.source)
            && Arc::ptr_eq(&substitution.parent_site.source, &site.source)
            && substitution.parent.substitution.is_none()
            && Self::conditional_substitution_parent_matches(
                &substitution.parent,
                &substitution.parent_site,
                &substitution.parent_words,
                config,
            )
            && tokens.synthetic.is_none()
            && tokens.original_words_match_argv_text()
            && substitution
                .parent_words
                .head_source()
                .and_then(|head| head.word())
                .is_some_and(|head| {
                    head.span().start() == substitution.parent_site.offset
                        && head.image() == site.source.source_image()
                        && head.config() == config
                })
            && substitution
                .parent_words
                .original_argument_word(substitution.operand)
                == Some(original.original_operand())
            && original.command().span.start() == site.offset
            && crate::registry_invocation::original_native_compiler_words(
                site.source.source_image(),
                tokens.words(),
                site.offset,
                config,
            )
            .as_deref()
                == Some(original.command().words.as_slice())
    }

    fn conditional_substitution_parent_matches(
        held: &ConditionalRepresentationBodyContext,
        site: &CommandAllocationSite,
        words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        use crate::registry_invocation::source_structure::OriginalRegistrySource;
        let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
            return false;
        };
        let original = advice.original_words();
        Arc::ptr_eq(&held.owner.source, &site.source)
            && Arc::ptr_eq(&advice.site().source, &site.source)
            && advice.site().offset == site.offset
            && advice.logical_source_input().is_none()
            && advice.matches_source(site.source.source_image(), config)
            && advice.matches_context(held.input.borrowed_context_registry())
            && words.matches_source(site.source.source_image(), config)
            && crate::registry_invocation::source_cut_declaration_origins(words)
            && original.len() == words.arguments().len() + 1
            && original.first() == words.head_source().and_then(|head| head.word())
            && original
                .iter()
                .skip(1)
                .enumerate()
                .all(|(index, word)| words.original_argument_word(index) == Some(word))
    }

    fn conditional_representation_immediate_parent(
        &self,
        held: &ConditionalRepresentationBodyContext,
        advice: &super::super::OriginalSourceCommandTransitionAdvice,
        context: &tcl_registry::model::ContextRegistry,
    ) -> bool {
        use super::super::source_transition_advice::OriginalSourceCatalogueBody;
        use crate::registry_invocation::source_structure::OriginalRegistrySource;
        let check = || -> Option<()> {
            if !std::ptr::eq(held.input.borrowed_context_registry(), context) {
                return None;
            }
            let OriginalSourceCatalogueBody::Script(body) =
                advice.original_catalogue_source_body()?
            else {
                return None;
            };
            if body.content_span() != held.body.content_span()
                || body.original_container() != held.body.original_container()
                || !body.matches_source(held.site.source.source_image(), self.config)
                || !body.inherits_immediate_caller_source_context(context)
                || !body
                    .source_words()
                    .source_script_bodies(context)
                    .contains(body.as_ref())
            {
                return None;
            }
            let parent_words = body.source_words();
            let OriginalRegistrySource::SourceTransitions(parent) = parent_words.source() else {
                return None;
            };
            if !Arc::ptr_eq(&parent.site().source, &held.site.source)
                || parent.site().offset != held.site.offset
                || !parent.matches_context(context)
                || !parent.matches_source(held.site.source.source_image(), self.config)
                || parent_words
                    .original_source_head_correspondence(&held.words, context)
                    .is_none()
            {
                return None;
            }
            let original = parent.original_words();
            if original.len() != held.words.arguments().len() + 1
                || original.first() != held.words.head_source().and_then(|head| head.word())
                || original
                    .iter()
                    .skip(1)
                    .enumerate()
                    .any(|(index, word)| held.words.original_argument_word(index) != Some(word))
                || parent_words.origins().get(1..) != held.words.origins().get(1..)
            {
                return None;
            }
            Some(())
        };
        check().is_some()
    }

    pub(super) fn representation_plan(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
        selected: &crate::registry_invocation::DeclarationInvocationFlow,
    ) -> Option<ConditionalRepresentationPlan> {
        use tcl_registry::value_transfer::{AnalysisContext, InvocationLayout};
        if !Self::representation_is_applicable(tokens, selected) {
            return None;
        }
        let naming = self.representation_plan_naming(tokens, site, selected)?;
        let words = selected.representation_words.as_ref()?;
        if !Self::representation_words_match_tokens(tokens, words) {
            return None;
        }
        let ConditionalRepresentationNaming {
            namespace,
            policy,
            body_context,
        } = naming;
        if !namespace.is_root() {
            return None;
        }
        let seed = self.inventory.source_seed.as_ref()?;
        let input = seed.baseline.metadata_context.source_analysis_input()?;
        let context = seed.source_context(self.registry)?;
        let source = Arc::new(words.clone());
        let captures = Self::representation_captures(tokens, words, &namespace, policy)?;
        let argument_words = Self::representation_schema_words(words);
        let texts = Self::representation_argument_texts(tokens);
        let (relation, prior_target, target, coercion, list_append, list_index) = words
            .with_source_schema(&context, |schema| {
                let view = crate::value_transfer::view_of(
                    schema,
                    &texts,
                    &argument_words,
                    InvocationLayout::Source,
                );
                let analysis = AnalysisContext {
                    script_encoding: input.script_encoding(),
                    ..AnalysisContext::detached(Some(input.unit_profile()))
                };
                let structure = crate::value_transfer::StructureInputs::new(view, &analysis);
                let relation =
                    schema.semantics.value.semantics().and_then(|semantics| {
                        semantics.conditional_source_representation(&structure)
                    });
                let list_append = schema.conditional_source_nonempty_list_append(&structure);
                if relation.is_some() && list_append.is_some() {
                    return None;
                }
                if (selected.effects.unknown_reads || !selected.effects.lookup_stable)
                    && !Self::representation_append_direct_phase(
                        schema,
                        selected,
                        words,
                        list_append,
                    )
                {
                    return None;
                }
                let prior_target = Self::conditional_prior_target(
                    schema.semantics.value.semantics(),
                    &structure,
                    relation,
                    words,
                );
                let target = Self::representation_schema_target(
                    schema,
                    words,
                    &namespace,
                    policy,
                    relation,
                    list_append,
                )
                .ok()?;
                let (coercion, list_index) = Self::representation_source_index(schema, &structure);
                Some((
                    relation,
                    prior_target,
                    target,
                    coercion,
                    list_append,
                    list_index,
                ))
            })??;
        (relation.is_some() || coercion.is_some() || list_append.is_some()).then_some(
            ConditionalRepresentationPlan {
                source,
                body_context,
                captures,
                relation,
                prior_target,
                target,
                coercion,
                list_append,
                list_index,
            },
        )
    }

    fn representation_schema_words(
        words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
    ) -> Vec<tcl_registry::InvocationWord<'_>> {
        words
            .arguments()
            .iter()
            .map(|word| {
                crate::registry_invocation::source_structure::source_schema_word(
                    word.as_registry_word(),
                )
            })
            .collect()
    }

    fn representation_argument_texts(tokens: &CommandTokens) -> Vec<&str> {
        tokens
            .argv_texts
            .iter()
            .skip(1)
            .map(String::as_str)
            .collect()
    }

    fn representation_schema_target(
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
        namespace: &crate::signature_scan::scope::SignatureNamespaceScope,
        policy: tcl_syntax::naming::NamePolicyProtocol,
        relation: Option<tcl_registry::value_transfer::ConditionalSourceRepresentation>,
        append: Option<tcl_registry::value_transfer::ConditionalSourceListAppend>,
    ) -> Result<Option<ConditionalRepresentationTarget>, ()> {
        let target = Self::conditional_representation_target(relation).or_else(|| {
            append.map(tcl_registry::value_transfer::ConditionalSourceListAppend::target)
        });
        match target {
            Some(target) => {
                Self::representation_source_target(schema, words, namespace, policy, target)
                    .map(Some)
                    .ok_or(())
            }
            None => Ok(None),
        }
    }

    fn representation_source_target(
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
        namespace: &crate::signature_scan::scope::SignatureNamespaceScope,
        policy: tcl_syntax::naming::NamePolicyProtocol,
        target: tcl_registry::value_transfer::TargetId,
    ) -> Option<ConditionalRepresentationTarget> {
        use crate::signature_scan::variable_symbol::{
            OriginalVariableSymbolPurpose, OriginalVariableSymbolReceiver,
            SignatureSourceVariableSymbol,
        };
        let name = words.original_argument_value_input(target.0.0)?;
        if name.policy() != policy
            || tcl_syntax::naming::split_element_ref_bytes(name.bytes()).is_some()
        {
            return None;
        }
        let form = schema.authored_source_variable_receiver_operand_form(target.0.0)?;
        let symbol = SignatureSourceVariableSymbol::from_original_input(
            name,
            namespace,
            OriginalVariableSymbolPurpose::NamespaceStorage,
            OriginalVariableSymbolReceiver::Operand(form),
        )?;
        Some((target, symbol))
    }

    fn representation_source_index(
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        inputs: &dyn tcl_registry::value_transfer::AnalysisInputs,
    ) -> (
        Option<tcl_registry::representation::ConditionalSourceListIndexCoercion>,
        Option<tcl_registry::value_transfer::ConditionalSourceLiteralListIndex>,
    ) {
        let index = schema.conditional_source_single_literal_list_index(inputs);
        let coercion = schema
            .semantics
            .representation_effect
            .conditional_source_list_index_coercion(
                schema.words.arguments(),
                schema.semantics.argument_offset,
            )
            .filter(|coercion| {
                index
                    .as_ref()
                    .is_some_and(|index| index.container_operand().0 == coercion.argument())
            });
        let index = index.filter(|index| {
            coercion.is_some_and(|coercion| index.container_operand().0 == coercion.argument())
        });
        (coercion, index)
    }

    fn representation_words_match_tokens(
        tokens: &CommandTokens,
        words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
    ) -> bool {
        words.arguments().len() + 1 == tokens.words().len()
            && tokens.argv_texts.len() == tokens.words().len()
            && crate::registry_invocation::source_cut_declaration_origins(words)
    }

    fn representation_plan_naming(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
        selected: &crate::registry_invocation::DeclarationInvocationFlow,
    ) -> Option<ConditionalRepresentationNaming> {
        if selected.representation_quiet
            && let Some((namespace, policy)) =
                self.representation_root_naming_scope(tokens, site, Some(selected))
        {
            return Some(ConditionalRepresentationNaming {
                namespace,
                policy,
                body_context: None,
            });
        }
        let context = self.conditional_representation_body_scope(tokens, site, selected)?;
        Some(ConditionalRepresentationNaming {
            namespace: context.namespace.clone(),
            policy: context.policy,
            body_context: Some(context),
        })
    }

    fn conditional_representation_target(
        relation: Option<tcl_registry::value_transfer::ConditionalSourceRepresentation>,
    ) -> Option<tcl_registry::value_transfer::TargetId> {
        use tcl_registry::value_transfer::ConditionalSourceRepresentation;
        match relation {
            Some(ConditionalSourceRepresentation::IntegerResult { target }) => target,
            Some(ConditionalSourceRepresentation::ForwardValue { target, .. }) => Some(target),
            None => None,
        }
    }

    fn conditional_prior_target(
        semantics: Option<&dyn tcl_registry::value_transfer::CommandSemantics>,
        inputs: &dyn tcl_registry::value_transfer::AnalysisInputs,
        relation: Option<tcl_registry::value_transfer::ConditionalSourceRepresentation>,
        words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
    ) -> Option<representation::PriorTargetUpdate> {
        use tcl_registry::value_transfer::{ConditionalSourceRepresentation, PlanAnswer};
        let semantics = semantics?;
        let update = semantics.conditional_source_target_update(inputs)?;
        let PlanAnswer::CellReadModifyWrite { target, .. } = semantics.structure(inputs) else {
            return None;
        };
        if update.target() != target
            || relation
                != Some(ConditionalSourceRepresentation::IntegerResult {
                    target: Some(target),
                })
        {
            return None;
        }
        Some(representation::PriorTargetUpdate {
            relation: update,
            read_span: words.original_argument_word(target.0.0)?.span(),
        })
    }

    pub(super) fn representation_loop_plan(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
        selected: &crate::registry_invocation::DeclarationInvocationFlow,
    ) -> Option<ConditionalRepresentationLoopPlan> {
        use crate::signature_scan::variable_symbol::{
            OriginalVariableSymbolPurpose, OriginalVariableSymbolReceiver,
            SignatureSourceVariableSymbol,
        };
        let ScriptBodyFlow::Loop {
            initial,
            repeated,
            continued,
            conditions,
        } = &selected.flow
        else {
            return None;
        };
        let words = selected.representation_words.as_ref()?;
        let body = words.arguments().len().checked_sub(1)?;
        if !initial.is_empty()
            || !continued.is_empty()
            || !conditions.is_empty()
            || repeated.as_slice() != [body]
            || words.arguments().len() != 3
            || !matches!(tokens.words().last(), Some(WordExpr::BracedLiteral { .. }))
            || tokens.words().len() != words.arguments().len() + 1
            || tokens.argv_texts.len() != tokens.words().len()
            || !tokens.original_words_match_argv_text()
            || tokens.synthetic.is_some()
            || !crate::registry_invocation::source_cut_declaration_origins(words)
            || !selected.representation_quiet
            || selected.effects.unknown_writes
            || selected.effects.frame_reachable()
            || !selected.writes.is_empty()
            || !selected.reads.is_empty()
            || !selected.name_queries.is_empty()
            || !selected.aliases.is_empty()
            || !selected.caller_frame_aliases.is_empty()
            || !selected.removes.is_empty()
            || !selected.possible_output_writes.is_empty()
        {
            return None;
        }
        let (namespace, policy) =
            self.representation_root_naming_scope(tokens, site, Some(selected))?;
        if !namespace.is_root() {
            return None;
        }
        let context = self
            .inventory
            .source_seed
            .as_ref()?
            .source_context(self.registry)?;
        let (count, binder, binder_receipt) = words.with_source_schema(&context, |schema| {
            let args = schema.words.arguments();
            let facts = schema.facts();
            let count = facts.conditional_source_finite_iteration(args)?;
            if count.iterations() < 2 {
                return None;
            }
            let binder = schema.conditional_source_single_loop_binding()?;
            if !binder.direct_phase_preserves_lookup()
                || !crate::registry_invocation::source_advice_has_no_declared_lookup_mutation(
                    &facts,
                )
            {
                return None;
            }
            let input = words.original_argument_value_input(binder.operand())?;
            if input.policy() != policy
                || input.bytes() != binder.name()
                || tcl_syntax::naming::split_element_ref_bytes(input.bytes()).is_some()
                || selected.loop_bindings.len() != 1
                || selected.loop_bindings[0].as_bytes() != binder.name()
            {
                return None;
            }
            let symbol = SignatureSourceVariableSymbol::from_original_input(
                input,
                &namespace,
                OriginalVariableSymbolPurpose::NamespaceStorage,
                OriginalVariableSymbolReceiver::Operand(binder.receiver_form()),
            )?;
            Some((count, symbol, binder))
        })??;
        let source = Arc::new(words.clone());
        let body_context =
            self.representation_loop_body_context(ConditionalRepresentationBodyInputs {
                site,
                words: &source,
                body_argument: body,
                context: &context,
                namespace,
                policy,
                binder: binder_receipt,
                count,
            });
        Some(ConditionalRepresentationLoopPlan {
            source,
            body_context,
            count,
            binder,
            span: tokens.words().first()?.source().span,
        })
    }

    fn representation_loop_body_context(
        &self,
        request: ConditionalRepresentationBodyInputs<'_>,
    ) -> Option<Arc<ConditionalRepresentationBodyContext>> {
        let ConditionalRepresentationBodyInputs {
            site,
            words,
            body_argument,
            context,
            namespace,
            policy,
            binder,
            count,
        } = request;
        let seed = self.inventory.source_seed.as_ref()?;
        let owner = seed.analysis_owner.as_ref()?;
        let input = seed.baseline.metadata_context.source_analysis_input()?;
        if !Arc::ptr_eq(&site.source, &owner.source)
            || !Arc::ptr_eq(&site.source, &seed.origin)
            || input.lexer_config() != self.config
            || !std::ptr::eq(input.borrowed_context_registry(), context)
            || !seed.matches_registry(self.registry)
        {
            return None;
        }
        let container = words.original_argument_word(body_argument)?;
        let mut bodies = words
            .source_script_bodies_for(
                context,
                crate::registry_invocation::OriginalSourceScriptPurpose::PotentialEvaluation,
            )
            .into_iter()
            .filter(|body| {
                body.original_container() == container
                    && body.matches_source(site.source.source_image(), self.config)
                    && body.inherits_immediate_caller_source_context(context)
            });
        let body = bodies.next()?;
        if bodies.next().is_some() {
            return None;
        }
        Some(Arc::new(ConditionalRepresentationBodyContext {
            owner: Arc::clone(owner),
            input: input.clone(),
            site: site.clone(),
            words: Arc::clone(words),
            body,
            namespace,
            policy,
            binder,
            count,
            substitution: None,
        }))
    }

    fn representation_is_applicable(
        tokens: &CommandTokens,
        selected: &crate::registry_invocation::DeclarationInvocationFlow,
    ) -> bool {
        !selected.effects.unknown_writes
            && selected.aliases.is_empty()
            && selected.caller_frame_aliases.is_empty()
            && selected.removes.is_empty()
            && selected.possible_output_writes.is_empty()
            && matches!(selected.flow, ScriptBodyFlow::None)
            && tokens.synthetic.is_none()
            && tokens.original_words_match_argv_text()
    }

    fn representation_append_direct_phase(
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        selected: &crate::registry_invocation::DeclarationInvocationFlow,
        words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
        append: Option<tcl_registry::value_transfer::ConditionalSourceListAppend>,
    ) -> bool {
        let Some(append) = append else { return false };
        let phase = append.conditional_direct_phase();
        let values = phase.evaluated_value_operands();
        let target = phase.current_target();
        let Some(name) = words.original_argument_value_input(target.0.0) else {
            return false;
        };
        let Ok(name) = std::str::from_utf8(name.bytes()) else {
            return false;
        };
        let facts = schema.facts();
        phase.conditionally_preserves_lookup()
            && target == append.target()
            && target.0.0 < values.start
            && values.end == words.arguments().len()
            && values.len() == append.appended_operand_count()
            && !selected.effects.frame_reachable()
            && selected.reads.len() == 1
            && selected.reads[0] == name
            && selected.writes.len() == 1
            && selected.writes[0] == name
            && selected.name_queries.is_empty()
            && crate::registry_invocation::source_advice_has_no_declared_lookup_mutation(&facts)
    }

    fn representation_captures(
        tokens: &CommandTokens,
        words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
        namespace: &crate::signature_scan::scope::SignatureNamespaceScope,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<VecDeque<representation::CaptureSource>> {
        use crate::signature_scan::scope::SignatureSourceNameInput;
        use crate::signature_scan::variable_symbol::{
            OriginalVariableSymbolPurpose, OriginalVariableSymbolReceiver,
            SignatureSourceVariableSymbol,
        };
        let mut captures = VecDeque::with_capacity(words.arguments().len());
        for (index, expression) in tokens.words().iter().skip(1).enumerate() {
            let word = words.original_argument_word(index)?;
            let capture = if expression.sole_variable_substitution().is_some() {
                let reference =
                    tcl_lexer::word_parts::whole_var_ref(word.bytes(), word.config()).ok()??;
                if reference.index.is_some()
                    || reference.name.contains(&0)
                    || tcl_syntax::naming::split_element_ref_bytes(reference.name).is_some()
                {
                    return None;
                }
                let root = crate::signature_scan::variable_name::SignatureSourceVariableRoot::from_original_word(
                    word, word.span(),
                    tcl_syntax::word_rules::WordValueRules::from_config(&word.config()), policy,
                )?;
                if root.is_separate_array_root() {
                    return None;
                }
                let name = SignatureSourceNameInput::OriginalVariableRoot(root);
                let symbol = SignatureSourceVariableSymbol::from_original_input(
                    &name,
                    namespace,
                    OriginalVariableSymbolPurpose::NamespaceStorage,
                    OriginalVariableSymbolReceiver::LexicalRoot,
                )?;
                representation::CaptureSource::Variable {
                    symbol,
                    span: word.span(),
                }
            } else if expression.sole_command_substitution().is_some() {
                representation::CaptureSource::LastResult
            } else if matches!(expression, WordExpr::Template { rejected: None, .. }) {
                let bytes = tcl_syntax::word_rules::original_static_word_source_bytes(word)?;
                representation::CaptureSource::Literal { bytes }
            } else if matches!(
                expression,
                WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. }
            ) {
                tcl_syntax::word_rules::original_static_word_source_bytes(word)
                    .map_or(representation::CaptureSource::Unknown, |bytes| {
                        representation::CaptureSource::Literal { bytes }
                    })
            } else {
                return None;
            };
            captures.push_back(capture);
        }
        Some(captures)
    }

    pub(super) fn conditional_substitution_context(
        &self,
        plan: &ConditionalRepresentationPlan,
        site: &CommandAllocationSite,
        tokens: &CommandTokens,
        operand: usize,
        source: &ExecutedScriptSource,
    ) -> Option<Arc<ConditionalRepresentationBodyContext>> {
        use crate::registry_invocation::source_structure::original_single_source_command_substitution;
        use tcl_lexer::word_parts::ExecutablePart;
        let parent = plan.body_context.as_ref()?;
        if parent.substitution.is_some()
            || !self
                .current_representation_body
                .as_ref()
                .is_some_and(|held| Arc::ptr_eq(held, parent))
            || !Arc::ptr_eq(&parent.site.source, &site.source)
            || !Arc::ptr_eq(&source.origin, &site.source)
            || !Self::conditional_substitution_parent_matches(
                parent,
                site,
                &plan.source,
                self.config,
            )
            || tokens
                .words()
                .get(operand + 1)?
                .sole_command_substitution()
                .is_none()
        {
            return None;
        }
        let word = plan.source.original_argument_word(operand)?;
        if word.group().expand
            || word.config() != self.config
            || word.image() != site.source.source_image()
            || !plan
                .source
                .matches_source(site.source.source_image(), self.config)
        {
            return None;
        }
        let parent_words = crate::registry_invocation::original_native_compiler_words(
            site.source.source_image(),
            tokens.words(),
            site.offset,
            self.config,
        )?;
        if parent_words.first() != plan.source.head_source().and_then(|head| head.word())
            || parent_words.len() != plan.source.arguments().len() + 1
            || parent_words
                .iter()
                .skip(1)
                .enumerate()
                .any(|(index, candidate)| {
                    plan.source.original_argument_word(index) != Some(candidate)
                })
            || parent_words.get(operand + 1) != Some(word)
        {
            return None;
        }
        let original = original_single_source_command_substitution(word)?;
        let arena = word.executable_parts();
        let [part] = arena.list(arena.root()) else {
            return None;
        };
        let ExecutablePart::Command { body } = &part.part else {
            return None;
        };
        if source.mapping != (ExecutedScriptMapping::Contiguous { base: body.start() })
            || u32::try_from(source.text.len()).ok()? != body.end().checked_sub(body.start())?
            || site.source.source_image().bytes().get(body.as_range()) != Some(source.text.bytes())
        {
            return None;
        }
        let mut context = parent.as_ref().clone();
        context.substitution = Some(ConditionalRepresentationSubstitution {
            parent: Arc::clone(parent),
            parent_site: site.clone(),
            parent_words: Arc::clone(&plan.source),
            operand,
            original,
        });
        Some(Arc::new(context))
    }
}
