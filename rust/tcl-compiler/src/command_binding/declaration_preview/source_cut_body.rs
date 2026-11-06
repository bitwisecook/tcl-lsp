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

//! Conditional source-cut body layouts, without dispatch or entered-body facts.

use super::super::{
    OriginalCompletedCommandWorld, OriginalSourceAnalysisOwner,
    OriginalSourceCutBodyEntryProjection, SourceInvocationBinding,
};
use super::{SourceCommandBindings, original_declaration_layouts};
use crate::analyser::ResolvedAnalysisInput;
use crate::ir::{CommandTokens, WordExpr};
use crate::registry_invocation::OriginalSourceScriptBody;
use std::collections::BTreeMap;
use std::sync::Arc;

pub(crate) struct OriginalSourceCutBodyRows {
    owner: Arc<OriginalSourceAnalysisOwner>,
    commands: BTreeMap<u32, (Arc<[WordExpr]>, SourceInvocationBinding)>,
}

impl OriginalSourceCutBodyRows {
    pub(crate) fn capture(
        bindings: &SourceCommandBindings,
        header: &CommandTokens,
        body: &OriginalSourceScriptBody,
        projection: &OriginalSourceCutBodyEntryProjection,
        input: &ResolvedAnalysisInput,
    ) -> Option<Self> {
        let owner = bindings.original_analysis_owner.as_ref()?;
        let site = header.source_binding.as_ref()?.invocation_site()?;
        let registry = input.borrowed_context_registry().commands();
        if !projection.matches_analysis_owner(owner)
            || !bindings.owns_original_analysis_attachment(header)
            || !Arc::ptr_eq(&owner.source, &site.source)
            || bindings.lexer_config != Some(input.lexer_config())
            || !body.matches_source(site.source.source_image(), input.lexer_config())
            || !body.matches_context(input.borrowed_context_registry())
        {
            return None;
        }
        let rows = original_declaration_layouts(bindings.declaration_layouts.get(site)?)?;
        let mut agreed = None;
        let mut snapshot = None;
        for row in rows {
            let candidate = super::declared_body(site, header, row, registry)?;
            Self::matches_body(&candidate, body, site)?;
            Self::world_owns_declaration(&candidate, projection.completed_world())?;
            let incoming = projection.snapshot(row, site, &candidate.namespace)?;
            if agreed.as_ref().is_some_and(|old| old != &candidate)
                || snapshot.as_ref().is_some_and(|old| old != &incoming)
            {
                return None;
            }
            agreed = Some(candidate);
            snapshot = Some(incoming);
        }
        let mut derived = SourceCommandBindings {
            root_origin: Some(Arc::clone(&owner.source)),
            lexer_config: Some(input.lexer_config()),
            original_analysis_owner: Some(Arc::clone(owner)),
            ..SourceCommandBindings::default()
        };
        derived.retain_declared_body_layouts_from_snapshot(
            agreed?,
            &snapshot?,
            input.lexer_config(),
            registry,
        );
        Self::from_complete_rows(&derived, body, owner, input)
    }

    fn matches_body(
        candidate: &super::DeclaredProcedureBody,
        body: &OriginalSourceScriptBody,
        site: &super::CommandAllocationSite,
    ) -> Option<()> {
        let content = body.content_span();
        if candidate.declaration != *site
            || !Arc::ptr_eq(&candidate.source.origin, &site.source)
            || candidate.source.base() != content.start()
            || candidate.source.text.bytes()
                != body
                    .original_container()
                    .image()
                    .bytes()
                    .get(content.as_range())?
            || candidate
                .authored_original_body_word
                .as_ref()
                .is_some_and(|word| word != body.original_container())
        {
            return None;
        }
        Some(())
    }

    fn world_owns_declaration(
        body: &super::DeclaredProcedureBody,
        world: &OriginalCompletedCommandWorld,
    ) -> Option<()> {
        let mut actual = world.declarations().filter_map(|publication| {
            let definition = publication.definition()?;
            let allocation = definition.allocation();
            (definition.kind() == super::super::SourceCommandDefinitionKind::Procedure
                && Arc::ptr_eq(&allocation.site.source, &body.declaration.source)
                && allocation.site.offset == body.declaration.offset
                && allocation.namespace == body.namespace)
                .then_some(allocation)
        });
        actual.next()?;
        actual.next().is_none().then_some(())
    }

    fn from_complete_rows(
        derived: &SourceCommandBindings,
        body: &OriginalSourceScriptBody,
        owner: &Arc<OriginalSourceAnalysisOwner>,
        input: &ResolvedAnalysisInput,
    ) -> Option<Self> {
        let context = input.borrowed_context_registry();
        let originals = body.original_commands(context)?;
        let flow = derived.current_declaration_flow_inventory();
        let mut commands = BTreeMap::new();
        for original in originals {
            let site = super::CommandAllocationSite {
                source: Arc::clone(&owner.source),
                offset: original.words.first()?.span().start(),
            };
            let observations = derived.declaration_layouts.get(&site)?;
            let rows = original_declaration_layouts(observations)?;
            for row in rows {
                let actual =
                    site.source
                        .original_lexical_command(&row.words, site.offset, row.config)?;
                if row.config != input.lexer_config()
                    || actual.native_words() != original.words.as_slice()
                {
                    return None;
                }
            }
            let first = original_declaration_layouts(observations)?.next()?;
            let binding = SourceInvocationBinding {
                unknown: true,
                dispatch_site: Some(site.clone()),
                original_analysis_owner: Some(Arc::clone(owner)),
                declaration_layout_observations: Some(Arc::from(observations.as_slice())),
                declaration_flow_inventory: Some(Arc::clone(&flow)),
                lookup_namespace: first.namespace.advisory_key()?,
                lookup_namespace_key: first.namespace.clone(),
                variable_context: Arc::clone(&first.snapshot.state.source_variables),
                variable_frame: first.snapshot.state.variable_frame.clone(),
                ..SourceInvocationBinding::default()
            };
            if commands
                .insert(site.offset, (Arc::clone(&first.words), binding))
                .is_some()
            {
                return None;
            }
        }
        Some(Self {
            owner: Arc::clone(owner),
            commands,
        })
    }

    pub(crate) fn source_reference(
        &self,
        tokens: &CommandTokens,
    ) -> Option<super::super::SourceCommandReference> {
        if tokens.synthetic.is_some() || !tokens.original_words_match_argv_text() {
            return None;
        }
        let offset = tokens.words().first()?.source().span.start();
        let (words, template) = self.commands.get(&offset)?;
        let binding = tokens.source_binding.as_ref()?;
        let mut expected = template.clone();
        expected
            .original_argument_syntax
            .clone_from(&binding.original_argument_syntax);
        expected
            .original_source_operand_attachment
            .clone_from(&binding.original_source_operand_attachment);
        if words.as_ref() != tokens.words() || binding != &expected {
            return None;
        }
        let input = binding.original_head_name_input(tokens)?;
        let crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(key) = &input
        else {
            return None;
        };
        let native = crate::registry_invocation::original_native_compiler_words(
            key.source_image(),
            tokens.words(),
            offset,
            key.lexer_config(),
        )?;
        let captured = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
            &native,
            input.policy().string_protocol(),
        )
        .ok()?;
        if native
            .iter()
            .enumerate()
            .any(|(index, word)| word.group().expand || captured.literal(index).is_none())
        {
            return None;
        }
        let rows =
            original_declaration_layouts(template.declaration_layout_observations.as_deref()?)?;
        let mut agreed = None;
        for row in rows {
            let state = &row.snapshot.state;
            if row.words.as_ref() != tokens.words()
                || row.config != key.lexer_config()
                || row.namespace != template.lookup_namespace_key
                || state.has_opaque_domain()
                || state.source_step_observed()
                || state.source_execution_observed(None)
                || !state.command_observers.is_quiet()
                || state.unclosed_command_world
                || !input.is_current(&state.source_variables)
            {
                return None;
            }
            let reference = state.original_reference_for_input(
                &input,
                &row.namespace,
                template.dispatch_site.clone(),
            )?;
            if !reference.is_direct_definition()
                || agreed
                    .as_ref()
                    .is_some_and(|previous| previous != &reference)
            {
                return None;
            }
            agreed = Some(reference);
        }
        agreed
    }

    pub(crate) fn registry_lookups(
        &self,
        tokens: &CommandTokens,
        input: &ResolvedAnalysisInput,
        recorded: Option<&SourceInvocationBinding>,
    ) -> Option<Vec<super::super::OriginalCompilationLookupAdvice>> {
        if input.has_logical_source_name_context() || input.has_hosted_source_name_context() {
            return None;
        }
        let template = self.current_template(tokens)?;
        let binding = tokens.source_binding.as_ref()?;
        let site = template.dispatch_site.as_ref()?;
        let mut expected = template.clone();
        expected
            .original_argument_syntax
            .clone_from(&binding.original_argument_syntax);
        expected
            .original_source_operand_attachment
            .clone_from(&binding.original_source_operand_attachment);
        let ordinary = if binding == &expected {
            None
        } else {
            (recorded == Some(binding)).then_some(())?;
            binding.original_invocation_metadata_for_input(
                tokens,
                input,
                input.borrowed_context_registry().commands(),
            )?;
            Some(binding.original_command_reference(tokens)?)
        };
        self.static_words(tokens, input)?;
        let rows =
            original_declaration_layouts(template.declaration_layout_observations.as_deref()?)?;
        let mut selected = Vec::new();
        for row in rows {
            let state = &row.snapshot.state;
            if row.config != input.lexer_config()
                || row.words.as_ref() != tokens.words()
                || row.namespace != template.lookup_namespace_key
                || state.baseline.source_point_entry.is_none()
                || state.baseline.metadata_context.source_analysis_input() != Some(input)
                || state.baseline.unknown_entry
                || state.baseline.logical_source_input.is_some()
                || state.baseline.vendor_source_input.is_some()
                || state.baseline.compiled_variable_provider.is_some()
                || state.baseline.native_compilation
                    != tcl_registry::native_compilation::NativeCompilationContext::default()
                || state.baseline.native_entry.is_some()
                || state.baseline.hosted_execution_context.is_some()
                || state.has_opaque_domain()
                || state.unclosed_command_world
                || state.source_step_observed()
                || state.source_execution_observed(None)
                || !state.command_observers.is_quiet()
                || !state.namespaces.contains(&row.namespace)
            {
                return None;
            }
            let advice = super::super::original_site_operand_layout_advice(
                site,
                tokens,
                &row.snapshot,
                &row.namespace,
                row.config,
            )?;
            if advice.unknown
                || advice.may_be_absent
                || advice.targets.is_empty()
                || advice.targets.iter().any(|target| {
                    !Self::registry_target_agrees(state, target, ordinary.as_ref(), site)
                })
            {
                return None;
            }
            selected.push(advice);
        }
        (!selected.is_empty()).then_some(selected)
    }

    fn registry_target_agrees(
        state: &super::super::ModuleCommandBindings,
        target: &super::super::SourceCommandTarget,
        ordinary: Option<&super::super::SourceCommandReference>,
        site: &super::super::CommandAllocationSite,
    ) -> bool {
        target.kind == super::super::BindingKind::Builtin && target.registry_backed
            && target.prepended.is_empty() && target.implementation_generation == 0
            && target.runtime_implementation_generation.is_none()
            && state.retained_target_is_current(target)
            && !state.source_execution_observed(target.identity.as_ref())
            && !state.runtime_execution_observed(target.identity.as_ref())
            && ordinary.is_none_or(|reference| {
                reference.matches_original_invocation_site(site)
                    && matches!(reference.binding(), super::super::SourceCommandReferenceBinding::Direct { kind: super::super::BindingKind::Builtin, identity }
                        if target.identity.as_ref() == Some(identity))
            })
    }

    fn current_template(&self, tokens: &CommandTokens) -> Option<&SourceInvocationBinding> {
        if tokens.synthetic.is_some() || !tokens.original_words_match_argv_text() {
            return None;
        }
        let offset = tokens.words().first()?.source().span.start();
        let (words, template) = self.commands.get(&offset)?;
        let binding = tokens.source_binding.as_ref()?;
        (words.as_ref() == tokens.words()
            && binding
                .original_analysis_owner
                .as_ref()
                .is_some_and(|owner| Arc::ptr_eq(owner, &self.owner))
            && binding.invocation_site() == template.invocation_site())
        .then_some(template)
    }

    fn static_words(
        &self,
        tokens: &CommandTokens,
        input: &ResolvedAnalysisInput,
    ) -> Option<Vec<tcl_lexer::NativeWord>> {
        let template = self.current_template(tokens)?;
        let native = crate::registry_invocation::original_native_compiler_words(
            self.owner.source.source_image(),
            tokens.words(),
            template.invocation_site()?.offset,
            input.lexer_config(),
        )?;
        let policy = tokens
            .source_binding
            .as_ref()?
            .original_head_name_input(tokens)?
            .policy();
        let captured = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
            &native,
            policy.string_protocol(),
        )
        .ok()?;
        if native.iter().enumerate().any(|(index, word)| {
            word.group().expand
                || captured.literal(index).is_none()
                || word.executable_parts().all_parts().any(|part| {
                    matches!(
                        part.part,
                        tcl_lexer::ExecutablePart::Command { .. }
                            | tcl_lexer::ExecutablePart::Expression { .. }
                            | tcl_lexer::ExecutablePart::ParseError(_)
                    )
                })
        }) {
            return None;
        }
        Some(native)
    }

    pub(crate) fn stamp(&self, tokens: &mut CommandTokens) -> Option<()> {
        if tokens.synthetic.is_some() || !tokens.original_words_match_argv_text() {
            return None;
        }
        let offset = tokens.words().first()?.source().span.start();
        let (words, template) = self.commands.get(&offset)?;
        let binding = tokens.source_binding.as_ref()?;
        if words.as_ref() != tokens.words()
            || binding
                .original_analysis_owner
                .as_ref()
                .is_none_or(|owner| !Arc::ptr_eq(owner, &self.owner))
        {
            return None;
        }
        // Only the exact missing-point issuer state can receive new geometry.
        // Every other retained facet keeps the existing binding terminal.
        let minimal = SourceInvocationBinding {
            original_analysis_owner: binding.original_analysis_owner.clone(),
            original_argument_syntax: binding.original_argument_syntax.clone(),
            original_source_operand_attachment: binding.original_source_operand_attachment.clone(),
            ..SourceInvocationBinding::unknown()
        };
        if binding != &minimal {
            return Some(());
        }
        let binding = tokens.source_binding.as_mut()?;
        binding.dispatch_site.clone_from(&template.dispatch_site);
        binding
            .declaration_layout_observations
            .clone_from(&template.declaration_layout_observations);
        binding
            .declaration_flow_inventory
            .clone_from(&template.declaration_flow_inventory);
        binding
            .lookup_namespace
            .clone_from(&template.lookup_namespace);
        binding
            .lookup_namespace_key
            .clone_from(&template.lookup_namespace_key);
        binding
            .variable_context
            .clone_from(&template.variable_context);
        binding.variable_frame.clone_from(&template.variable_frame);
        Some(())
    }
}
