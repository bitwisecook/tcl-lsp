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

//! Procedure declaration headers: operands, formal names and parameter topology.

use super::super::declaration_layout::{
    DeclarationLayoutObservation, original_declaration_layouts,
};
use super::super::{CommandAllocationSite, SourceCommandBindings};
use super::declared_body::{declared_body, declared_procedure_frame};
use super::{DeclaredProcedureHeader, ProcedureOperands};
use crate::registry_invocation::{
    EffectiveInvocationWord, RegistryInvocationResolution, effective_invocation_word,
    effective_words_for_target,
};

impl SourceCommandBindings {
    /// Conditional Native source formal names only. This selects no namespace
    /// cell, installed procedure, frame, body entry or completion.
    pub(crate) fn conditional_native_procedure_formal_names_at(
        &self,
        site: &CommandAllocationSite,
        expected_name: &crate::signature_scan::scope::SignatureSourceNameInput,
        registry: &tcl_registry::CommandRegistry,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Vec<crate::signature_scan::formal_parameters::OriginalSourceFormalName>> {
        if input.has_logical_source_name_context()
            || input.has_hosted_source_name_context()
            || self.lexer_config != Some(input.lexer_config())
            || self.root_origin.as_ref() != Some(&site.source)
            || input
                .borrowed_context_registry()
                .commands()
                .snapshot()
                .semantic_key()
                != registry.snapshot().semantic_key()
        {
            return None;
        }
        let rows = original_declaration_layouts(self.declaration_layouts.get(site)?)?;
        let mut agreed = None;
        for row in rows {
            if row.config != input.lexer_config()
                || !row.entry.owns_source(&site.source, site.offset)
                || !row
                    .entry
                    .owns_original_context(&row.snapshot.state.source_variables)
                || row.snapshot.state.variable_frame != *row.entry.frame()
                || row.snapshot.state.current_source_origin.as_ref() != Some(&site.source)
                || row.snapshot.state.baseline.registry_snapshot
                    != Some(registry.snapshot().semantic_key())
            {
                return None;
            }
            let tokens = declaration_tokens(site, row)?;
            let advice = super::super::original_site_operand_layout_advice(
                site,
                &tokens,
                &row.snapshot,
                &row.namespace,
                row.config,
            )?;
            if !advice.closed_lookup()
                || advice.dialect()
                    != super::super::source_analysis_entry::source_input_dialect(input)
            {
                return None;
            }
            for target in advice.targets() {
                let topology = conditional_native_formal_for_target(
                    site,
                    &tokens,
                    row,
                    &advice,
                    target,
                    (registry, input),
                    expected_name,
                )?;
                if agreed.as_ref().is_some_and(|old| old != &topology) {
                    return None;
                }
                agreed = Some(topology);
            }
        }
        Some(crate::signature_scan::formal_parameters::SignatureSourceFormalParameters::from_original_topology(&agreed?).name_sources())
    }

    /// Unanimous original procedure `ParamList` syntax at its actual declaration
    /// site. This does not select an installation or an entered body.
    pub(crate) fn original_procedure_formals_at(
        &self,
        site: &CommandAllocationSite,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<crate::signature_scan::formal_parameters::SignatureSourceFormalParameters> {
        let observations = original_declaration_layouts(self.declaration_layouts.get(site)?)?;
        let mut agreed = None;
        for observation in observations {
            let tokens = declaration_tokens(site, observation)?;
            let body = declared_body(site, &tokens, observation, registry)?;
            let topology = body.original_parameters?;
            if agreed.as_ref().is_some_and(|old| old != &topology) {
                return None;
            }
            agreed = Some(topology);
        }
        Some(crate::signature_scan::formal_parameters::SignatureSourceFormalParameters::from_original_topology(&agreed?))
    }

    /// Source-only Logical formals retain the selected declaration recipe and
    /// original word. They never acquire Native topology or an entered frame.
    pub(crate) fn original_logical_formal_name_sources_at(
        &self,
        site: &CommandAllocationSite,
        registry: &tcl_registry::CommandRegistry,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Vec<crate::signature_scan::formal_parameters::OriginalSourceFormalName>> {
        if !input.has_logical_source_name_context()
            || !self.matches_original_source_image(site.source.source_image(), input.lexer_config())
        {
            return None;
        }
        for row in original_declaration_layouts(self.declaration_layouts.get(site)?)? {
            if row.config != input.lexer_config()
                || row.snapshot.state.logical_source_name_advice_input() != Some(input)
            {
                return None;
            }
        }
        let body = self.original_declared_procedure_at(site, registry)?;
        if body.original_parameters.is_some() || body.logical_name.is_none() {
            return None;
        }
        super::super::logical_definition::formal_name_sources(
            body.logical_parameters_word.as_ref(),
            body.logical_parameters_text.as_deref()?,
            &body.parameters,
            input.lexer_config(),
        )
    }

    /// Typed source Logical formal topology from the exact retained declaration.
    /// The returned values keep authored source syntax separate from Native
    /// `ParamList` bytes and never grant an entered frame or installed locals.
    pub(crate) fn original_logical_formal_topology_at(
        &self,
        site: &CommandAllocationSite,
        registry: &tcl_registry::CommandRegistry,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<crate::signature_scan::formal_parameters::LogicalProcedureFormalTopology> {
        if !input.has_logical_source_name_context()
            || !self.matches_original_source_image(site.source.source_image(), input.lexer_config())
        {
            return None;
        }
        for row in original_declaration_layouts(self.declaration_layouts.get(site)?)? {
            if row.config != input.lexer_config()
                || row.snapshot.state.logical_source_name_advice_input() != Some(input)
            {
                return None;
            }
        }
        let body = self.original_declared_procedure_at(site, registry)?;
        if body.original_parameters.is_some() || body.logical_name.is_none() {
            return None;
        }
        let text = body.logical_parameters_text.as_deref()?;
        let names = super::super::logical_definition::formal_name_sources(
            body.logical_parameters_word.as_ref(),
            text,
            &body.parameters,
            input.lexer_config(),
        )?;
        crate::signature_scan::formal_parameters::LogicalProcedureFormalTopology::from_selected_declaration(
            body.logical_parameters_word.as_ref()?,
            &body.parameters,
            names,
            input.lexer_config(),
        )
    }
}

pub(in super::super) fn declaration_tokens(
    site: &CommandAllocationSite,
    observation: &DeclarationLayoutObservation,
) -> Option<crate::ir::CommandTokens> {
    let text = site.source.try_text().ok()?;
    let first = observation.words.first()?.original_written_span(text)?;
    let last = observation.words.last()?.original_written_span(text)?;
    if first.start() != site.offset || first.start() > last.end() {
        return None;
    }
    let image = tcl_lexer::SourceImage::from_bytes(
        text.as_bytes()
            .get(first.start() as usize..last.end() as usize)?,
        site.source.source_image().channel(),
    );
    let segments = tcl_source::segment::segment_commands_image_with_offset_and_config(
        &image,
        first.start(),
        observation.config.nested(),
    )?;
    let [segment] = segments.as_slice() else {
        return None;
    };
    let tokens = super::super::source_command_tokens_boxed(
        &image,
        first.start(),
        observation.config,
        segment,
    );
    (tokens.words() == observation.words.as_ref()).then_some(*tokens)
}

/// A readonly source operand follows its actual effective ordinal. It does
/// not acquire installation, entered-frame or Normal authority here.
pub(super) fn original_declaration_operand_input(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    observation: &DeclarationLayoutObservation,
    candidate: &super::super::SourceCommandTarget,
    effective: &crate::registry_invocation::EffectiveCommandWords,
    index: usize,
) -> Option<crate::signature_scan::scope::SignatureSourceNameInput> {
    use crate::registry_invocation::InvocationWordOrigin;
    use crate::signature_scan::scope::{
        SignatureSourceNameInput, SignatureSourceNameKey, SignatureSourceNameValue,
    };
    let variables = &observation.snapshot.state.source_variables;
    let policy = variables.execution_name_policy?.native_recipe()?;
    let origin = effective.origins.get(index)?;
    if let InvocationWordOrigin::BindingPrefix(prefix) = origin {
        let input = candidate.original_prepended_name_inputs()?.get(*prefix)?;
        return (input.policy() == policy && input.is_current(variables)).then(|| input.clone());
    }
    let native = crate::registry_invocation::original_native_compiler_words(
        site.source.source_image(),
        tokens.words(),
        site.offset,
        observation.config,
    )?;
    let rules = tcl_syntax::word_rules::WordValueRules::from_config(&observation.config);
    match origin {
        InvocationWordOrigin::Written(written) => {
            SignatureSourceNameKey::from_original_native_word(native.get(*written)?, rules, policy)
                .map(SignatureSourceNameInput::OriginalWord)
        }
        InvocationWordOrigin::ExpandedElement { written, element } => {
            let parent = SignatureSourceNameInput::OriginalValue(
                SignatureSourceNameValue::from_original_static_word(
                    native.get(*written)?,
                    rules,
                    policy,
                )?,
            );
            let children = parent.original_list_elements()?;
            let projected = effective.origins.iter().filter_map(|origin| match origin {
                InvocationWordOrigin::ExpandedElement {
                    written: parent,
                    element,
                } if parent == written => Some(*element),
                _ => None,
            });
            projected.eq(0..children.len()).then_some(())?;
            children.get(*element).cloned()
        }
        InvocationWordOrigin::ResolvedHead | InvocationWordOrigin::BindingPrefix(_) => None,
    }
}

pub(super) fn procedure_declaration_header(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    observation: &DeclarationLayoutObservation,
    facts: &tcl_registry::InvocationFacts,
    candidate: &super::super::SourceCommandTarget,
    operands: &ProcedureOperands<'_>,
) -> Option<DeclaredProcedureHeader> {
    use tcl_registry::{CommandBindingDefinitionKind, CommandBindingTransition};
    let ProcedureOperands {
        effective,
        values,
        dialect,
    } = *operands;
    if !facts.arg_roles_complete || facts.arity_accepts_frozen_arguments() != Some(true) {
        return None;
    }
    let name_argument = facts
        .state_transitions
        .declared()?
        .command_bindings()
        .find_map(|transition| {
            if let CommandBindingTransition::Define {
                name,
                kind: CommandBindingDefinitionKind::Procedure,
            } = transition
            {
                name.argument_index()
            } else {
                None
            }
        })?;
    let name = original_declaration_operand_input(
        site,
        tokens,
        observation,
        candidate,
        effective,
        name_argument.checked_add(1)?,
    )?;
    if !observation.entry.owns_source(&site.source, site.offset)
        || !observation
            .entry
            .owns_original_context(&observation.snapshot.state.source_variables)
        || observation.snapshot.state.variable_frame != *observation.entry.frame()
        || observation.snapshot.state.current_source_origin.as_ref() != Some(&site.source)
    {
        return None;
    }
    let (key, source_slot) = observation
        .snapshot
        .state
        .original_conditional_procedure_coordinates(&observation.namespace, &name)?;
    let namespace = key.holder().into_owned();
    let words: Vec<_> = values
        .iter()
        .map(EffectiveInvocationWord::as_registry_word)
        .collect();
    let arguments =
        tcl_registry::InvocationArguments::structured(words.get(1..)?).with_dialect(dialect);
    let original_parameters =
        original_procedure_parameter_topology(site, tokens, observation, facts, effective, dialect);
    let parameters = original_parameters
        .as_ref()
        .map(super::super::formal_topology::OriginalFormalTopology::advisory_parameters)
        .or_else(|| super::super::native_procedure_parameters(facts, arguments))?;
    let body_index = facts.arg_roles.iter().find_map(|(index, role)| {
        (*role == tcl_registry::ArgRole::Body)
            .then_some(facts.argument_offset + usize::from(*index) + 1)
    })?;
    let crate::registry_invocation::InvocationWordOrigin::Written(written) =
        effective.origins.get(body_index)?
    else {
        return None;
    };
    let EffectiveInvocationWord::Literal(value) = values.get(body_index)? else {
        return None;
    };
    written.checked_sub(1)?;
    tokens.words().get(*written)?;
    let frame = declared_procedure_frame(site, &namespace);
    Some(DeclaredProcedureHeader {
        source_slot,
        name_input: name,
        declaration: site.clone(),
        candidate: candidate.clone(),
        slot: key,
        namespace,
        parameters,
        original_parameters,
        frame,
        written: *written,
        value: value.to_owned(),
    })
}

/// Project the actual Registry-selected Native parameter operand for one target.
fn conditional_native_formal_for_target(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    row: &DeclarationLayoutObservation,
    advice: &super::super::OriginalCompilationLookupAdvice,
    target: &super::super::SourceCommandTarget,
    owner: (
        &tcl_registry::CommandRegistry,
        &crate::analyser::ResolvedAnalysisInput,
    ),
    expected_name: &crate::signature_scan::scope::SignatureSourceNameInput,
) -> Option<super::super::formal_topology::OriginalFormalTopology> {
    use tcl_registry::{CommandBindingDefinitionKind, CommandBindingTransition};
    let (registry, input) = owner;
    let metadata = advice
        .original_declaration_metadata_context(registry, tokens)
        .ok()??;
    if metadata.source_analysis_input() != Some(input) {
        return None;
    }
    let effective = effective_words_for_target(tokens, target)?;
    let dialect = advice.dialect();
    let values = effective
        .words
        .iter()
        .map(|word| {
            effective_invocation_word(word, dialect.lexer_grammar.escapes, dialect.word_values)
        })
        .collect::<Vec<_>>();
    let words = values
        .iter()
        .map(EffectiveInvocationWord::as_registry_word)
        .collect::<Vec<_>>();
    let RegistryInvocationResolution::Resolved(facts) =
        crate::registry_invocation::resolve_registry_words_in_realm_with_metadata_context(
            registry,
            Some(metadata),
            &words,
            Some(dialect),
            advice.realm(),
        )
        .ok()?
    else {
        return None;
    };
    if !facts.arg_roles_complete || facts.arity_accepts_frozen_arguments() != Some(true) {
        return None;
    }
    let argument = facts
        .state_transitions
        .declared()?
        .command_bindings()
        .find_map(|transition| match transition {
            CommandBindingTransition::Define {
                name,
                kind: CommandBindingDefinitionKind::Procedure,
            } => name.argument_index(),
            _ => None,
        })?;
    let name = original_declaration_operand_input(
        site,
        tokens,
        row,
        target,
        &effective,
        argument.checked_add(1)?,
    )?;
    if &name != expected_name {
        return None;
    }
    original_procedure_parameter_topology(site, tokens, row, &facts, &effective, dialect)
}

fn original_procedure_parameter_topology(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    observation: &DeclarationLayoutObservation,
    facts: &tcl_registry::InvocationFacts,
    effective: &crate::registry_invocation::EffectiveCommandWords,
    dialect: tcl_native_model::InvocationDialect,
) -> Option<super::super::formal_topology::OriginalFormalTopology> {
    facts
        .arg_roles
        .iter()
        .find_map(|(index, role)| {
            (*role == tcl_registry::ArgRole::ParamList)
                .then_some(facts.argument_offset + usize::from(*index))
        })
        .and_then(|argument| effective.written_argument(argument))
        .and_then(|written| {
            super::super::formal_topology::capture(
                site.source.source_image(),
                tokens.words(),
                site.offset,
                written + 1,
                observation.config,
                dialect,
                observation
                    .snapshot
                    .state
                    .source_variables
                    .execution_name_policy?
                    .native_recipe()?,
            )
        })
}
