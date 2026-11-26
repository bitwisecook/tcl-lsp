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

//! A declared procedure body: its standalone and Logical recipes and what its preview answers.

use super::super::declaration_layout::{
    DeclarationLayoutObservation, original_declaration_layouts,
};
use super::super::{CommandAllocationSite, ExecutedScriptSource, SourceCommandBindings};
use super::procedure_header::{declaration_tokens, procedure_declaration_header};
use super::{DeclaredProcedureBody, OriginalLogicalProcedureStaticFacet, ProcedureOperands};
use crate::registry_invocation::{
    EffectiveInvocationWord, RegistryInvocationResolution, effective_invocation_word,
    effective_words_for_target, resolve_registry_words_in_realm,
};
use std::sync::Arc;

impl DeclaredProcedureBody {
    /// Same genuine Logical declaration behind a conditional catalogue read.
    /// These are source/frame coordinates only, without a reached lookup,
    /// allocated implementation, entered frame or Normal continuation.
    pub(in super::super) fn matches_logical_read_scope(
        &self,
        read: &super::super::source_read_scope::SourceReadScope,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        let advice = read.advice();
        let offset = advice.site().offset;
        let required = |value: bool, stage: &'static str| {
            if !value {
                super::super::source_read_scope::trace_rbs_refusal(offset, stage);
            }
            value
        };
        let Some(input) = advice.logical_source_input() else {
            return required(false, "declared.logical-input");
        };
        let Some(scope) = advice.logical_source_scope() else {
            return required(false, "declared.logical-scope");
        };
        if !input.has_logical_source_name_context() {
            return required(false, "declared.input-purpose");
        }
        if input.lexer_config() != config {
            return required(false, "declared.input-config");
        }
        if !scope.matches_input(input) {
            return required(false, "declared.scope-input");
        }
        if scope.frame_recipe() != super::super::OriginalDeclaredSourceBodyFrame::Procedure {
            return required(false, "declared.scope-frame");
        }
        let body = scope.scope_body();
        let words = body.source_words();
        let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(
            header,
        ) = words.source()
        else {
            return required(false, "declared.header-source");
        };
        let super::super::ExecutedScriptMapping::Contiguous { base } = self.source.mapping else {
            return required(false, "declared.body-mapping");
        };
        let Some(end) = u32::try_from(self.source.text.len())
            .ok()
            .and_then(|length| base.checked_add(length))
        else {
            return required(false, "declared.body-end");
        };
        let Some(namespace) =
            super::super::SourceNamespaceKey::authored_path(scope.namespace_recipe())
        else {
            return required(false, "declared.namespace");
        };
        let Some(shape) = words
            .with_source_schema(input.borrowed_context_registry(), |schema| {
                schema.authored_source_procedure_arguments()
            })
            .flatten()
        else {
            return required(false, "declared.header-shape");
        };
        required(header.site() == &self.declaration, "declared.header-site")
            && required(
                header.logical_source_input() == Some(input),
                "declared.header-input",
            )
            && required(
                self.source.origin == self.declaration.source,
                "declared.source-origin",
            )
            && required(
                body.content_span() == tcl_lexer::Span::new(base, end),
                "declared.body-span",
            )
            && required(
                body.matches_source(self.source.origin.source_image(), config),
                "declared.body-source",
            )
            && required(namespace == self.namespace, "declared.body-namespace")
            && required(
                self.logical_name.as_ref().map(String::as_bytes)
                    == scope
                        .scope_name_input()
                        .map(super::super::OriginalLogicalSourceNameInput::bytes),
                "declared.name",
            )
            && required(
                self.logical_parameters_word
                    .as_ref()
                    .is_some_and(|parameters| {
                        required(
                            words.original_argument_word(shape.parameters) == Some(parameters),
                            "declared.parameters-word",
                        )
                    }),
                "declared.parameters",
            )
    }

    pub(in super::super) fn static_source_uncertainty(
        &self,
        layouts: &super::super::declaration_layout::DeclarationLayouts,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<std::collections::BTreeSet<String>> {
        let mut agreed = None;
        for row in original_declaration_layouts(layouts.get(&self.declaration)?)? {
            if row.config != input.lexer_config() {
                return None;
            }
            let tokens = declaration_tokens(&self.declaration, row)?;
            let advice = super::super::original_site_operand_layout_advice(
                &self.declaration,
                &tokens,
                &row.snapshot,
                &row.namespace,
                row.config,
            )?;
            let facet = logical_procedure_static_facet(
                &self.declaration,
                &tokens,
                row,
                registry,
                input,
                &advice,
                self,
            )?;
            if agreed.as_ref().is_some_and(|previous| previous != &facet) {
                return None;
            }
            agreed = Some(facet);
        }
        Some(
            agreed?
                .statics
                .into_iter()
                .flat_map(|(_, variables)| variables.into_iter().map(|variable| variable.name))
                .collect(),
        )
    }

    pub(in super::super) fn matches_original_procedure_source(
        &self,
        procedure: &crate::ir::Procedure,
        image: &tcl_lexer::SourceImage,
    ) -> bool {
        procedure.declaration_site.as_ref() == Some(&self.declaration)
            && self.declaration.source.source_image() == image
            && self.logical_parameters_text.as_deref() == Some(procedure.params_raw.as_str())
            && self
                .parameters
                .iter()
                .map(|formal| formal.name.as_str())
                .eq(procedure.params.iter().map(String::as_str))
            && self.source.origin.source_image() == image
            && self.source.base() == procedure.body_offset
            && self.source.text.try_text().ok() == procedure.body_source.as_deref()
            && procedure.body.namespace_context.as_deref() == Some(&self.namespace)
    }

    pub(in super::super) fn declaration_site(&self) -> &CommandAllocationSite {
        &self.declaration
    }
    pub(in super::super) fn command_label(&self) -> Option<String> {
        super::super::ModuleCommandBindings::command_key_label(&self.slot)
    }
    /// Closed formal-name geometry in the retained Logical declaration. This
    /// does not select Native formal storage, an entered frame or alias cells.
    pub(in super::super) fn logical_alias_local_name_is_byte_unique(
        &self,
        policy: tcl_syntax::naming::NamePolicyProtocol,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        use tcl_syntax::naming::{NativeNameContext, NativeVariableRootGeometry};
        if policy.authority() != tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
            || self.original_parameters.is_some()
            || self.logical_name.is_none()
        {
            return false;
        }
        let Some(text) = self.logical_parameters_text.as_deref() else {
            return false;
        };
        if super::super::logical_definition::static_formals(text, config.list_parse).as_ref()
            != Some(&self.parameters)
        {
            return false;
        }
        let protocol = policy.recipe();
        self.parameters.iter().all(|formal| {
            let input = protocol.combined_variable_input(formal.name.as_bytes());
            input.element().is_none()
                && matches!(
                    protocol.variable_root_geometry(NativeNameContext::root(), input.root().selected()),
                    NativeVariableRootGeometry::Local(name) if name.as_bytes() == formal.name.as_bytes()
                )
        })
    }
}

impl SourceCommandBindings {
    /// Unanimous conditional procedure body at its genuine declaration.
    /// The lexical recipe is independent of installation and entered calls.
    pub(in super::super) fn original_declared_procedure_at(
        &self,
        site: &CommandAllocationSite,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<DeclaredProcedureBody> {
        let observations = original_declaration_layouts(self.declaration_layouts.get(site)?)?;
        let mut agreed = None;
        for observation in observations {
            let tokens = declaration_tokens(site, observation)?;
            let body = declared_body(site, &tokens, observation, registry)?;
            if agreed.as_ref().is_some_and(|previous| previous != &body) {
                return None;
            }
            agreed = Some(body);
        }
        agreed
    }

    /// Select the current Logical procedure's original authored body. This
    /// retains source geometry only; it supplies no Native entry or completion.
    pub(in super::super) fn logical_called_procedure_body(
        &self,
        target: &super::super::SourceCommandTarget,
        state: &super::super::ModuleCommandBindings,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<DeclaredProcedureBody> {
        let input = state.logical_source_name_advice_input()?;
        if !input.has_logical_source_name_context()
            || state.baseline.metadata_context.source_analysis_input() != Some(input)
            || self.lexer_config != Some(input.lexer_config())
            || state.baseline.native_entry.is_some()
            || state.baseline.unknown_entry
            || state.baseline.execution_name_policy.is_some()
            || state.baseline.hosted_execution_context.is_some()
            || state.has_opaque_domain()
            || !state.command_observers.is_quiet()
            || state.baseline.dialect
                != Some(super::super::source_analysis_entry::source_input_dialect(
                    input,
                ))
            || state.baseline.registry_snapshot.as_ref()
                != Some(&registry.snapshot().semantic_key())
            || input
                .context_registry()
                .commands()
                .snapshot()
                .semantic_key()
                != registry.snapshot().semantic_key()
            || target.registry_backed
            || target.kind != super::super::BindingKind::Proc
            || !target.prepended.is_empty()
            || target.runtime_implementation_generation.is_some()
            || !state.retained_target_is_current(target)
        {
            return None;
        }
        let allocation = target.implementation_allocation.as_ref()?;
        let identity = target.identity.as_ref()?;
        let site = &allocation.site;
        if allocation.incarnation != super::super::AllocationIncarnation::First
            || !Arc::ptr_eq(&site.source, state.current_source_origin.as_ref()?)
            || site.offset != target.implementation_generation
            || allocation.command != target.command
            || identity.runtime.is_some()
            || identity.origin != target.command
            || identity.declaration != site.offset
            || identity.allocation.as_ref() != Some(allocation)
        {
            return None;
        }
        let body = self.original_declared_procedure_at(site, registry)?;
        if body.logical_name.is_none()
            || body.original_parameters.is_some()
            || &body.declaration != site
            || body.slot.authored_spelling()? != target.command
            || body.namespace != allocation.namespace
            || !Arc::ptr_eq(&body.source.origin, &site.source)
            || !matches!(
                body.source.mapping,
                super::super::ExecutedScriptMapping::Contiguous { .. }
            )
            || !state.namespaces.contains(&body.namespace)
        {
            return None;
        }
        Some(body)
    }
}

pub(super) fn declared_body(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    observation: &DeclarationLayoutObservation,
    registry: &tcl_registry::CommandRegistry,
) -> Option<DeclaredProcedureBody> {
    let advice = super::super::original_site_operand_layout_advice(
        site,
        tokens,
        &observation.snapshot,
        &observation.namespace,
        observation.config,
    );
    #[cfg(debug_assertions)]
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_DECLARATION").is_some() {
        eprintln!(
            "ORIGINAL_DECLARATION offset={} stage=body-advice available={}",
            site.offset,
            advice.is_some()
        );
    }
    let advice = advice?;
    if let Some(input) = observation
        .snapshot
        .state
        .logical_source_name_advice_input()
    {
        return logical_procedure_body_recipe(site, tokens, observation, registry, input, &advice);
    }
    if advice.is_authored_standalone_source() {
        return standalone_procedure_body_recipe(site, tokens, observation, registry, &advice);
    }
    let dialect = advice.dialect();
    let mut agreed = None;
    for target in advice.targets() {
        let effective = effective_words_for_target(tokens, target)?;
        let values: Vec<_> = effective
            .words
            .iter()
            .map(|word| {
                effective_invocation_word(word, dialect.lexer_grammar.escapes, dialect.word_values)
            })
            .collect();
        let words: Vec<_> = values
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect();
        let RegistryInvocationResolution::Resolved(facts) =
            resolve_registry_words_in_realm(registry, None, &words, Some(dialect), advice.realm())
                .ok()?
        else {
            return None;
        };
        let recipe = procedure_body_recipe(
            site,
            tokens,
            observation,
            &facts,
            target,
            &ProcedureOperands {
                effective: &effective,
                values: &values,
                dialect,
            },
        );
        #[cfg(debug_assertions)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_DECLARATION").is_some() {
            eprintln!(
                "ORIGINAL_DECLARATION offset={} stage=procedure-recipe available={} arity={:?} roles={}",
                site.offset,
                recipe.is_some(),
                facts.arity_accepts_frozen_arguments(),
                facts.arg_roles_complete
            );
        }
        let recipe = recipe?;
        if agreed.as_ref().is_some_and(|old| old != &recipe) {
            return None;
        }
        agreed = Some(recipe);
    }
    agreed
}

/// Conditional Standalone layouts use original source slots and strict formals,
/// independently of Native operand naming or procedure installation.
fn standalone_procedure_body_recipe(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    observation: &DeclarationLayoutObservation,
    registry: &tcl_registry::CommandRegistry,
    advice: &super::super::OriginalCompilationLookupAdvice,
) -> Option<DeclaredProcedureBody> {
    if advice.has_unclosed_source_coverage()
        || !advice
            .original_declaration_metadata_context(registry, tokens)
            .is_ok_and(|metadata| {
                metadata.is_none_or(
                    crate::registry_invocation::InvocationMetadataContext::is_standalone,
                )
            })
        || observation.config.list_parse != tcl_dialect::ListParse::Strict
        || !matches!(&observation.namespace, super::super::SourceNamespaceKey::Authored(current) if current == "::")
        || !observation.entry.owns_source(&site.source, site.offset)
        || !observation
            .entry
            .owns_original_context(&observation.snapshot.state.source_variables)
        || observation.snapshot.state.variable_frame != *observation.entry.frame()
        || observation.snapshot.state.current_source_origin.as_ref() != Some(&site.source)
    {
        return None;
    }
    let [target] = advice.targets() else {
        return None;
    };
    if !target.registry_backed
        || target.kind != super::super::BindingKind::Builtin
        || !target.prepended.is_empty()
    {
        return None;
    }
    let (original, values) = standalone_procedure_words(site, tokens, observation.config)?;
    let arguments = values
        .get(1..)?
        .iter()
        .map(|value| tcl_registry::InvocationWord::Literal(value))
        .collect::<Vec<_>>();
    let resolution =
        tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
            registry,
            None,
            tcl_registry::InvocationWords::structured(
                tcl_registry::InvocationWord::Literal(&target.command),
                &arguments,
            )
            .with_dialect(advice.dialect()),
            advice.realm(),
        );
    let shape = resolution
        .resolved()?
        .authored_source_procedure_arguments()?;
    let name = values.get(shape.name.checked_add(1)?)?;
    let parameters_text = values.get(shape.parameters.checked_add(1)?)?;
    let parameters = super::super::logical_definition::static_formals(
        parameters_text,
        observation.config.list_parse,
    )?;
    let coordinate = tcl_syntax::naming::authored_source_command_slot(
        &tcl_core_types::ByteNamespacePath::root(),
        name.as_bytes(),
    )?;
    let namespace = super::super::SourceNamespaceKey::authored_path(&coordinate.namespace)?;
    observation
        .snapshot
        .state
        .namespaces
        .contains(&namespace)
        .then_some(())?;
    let key = super::super::SourceCommandKey::authored(
        tcl_syntax::naming::authored_source_command_spelling(&coordinate)?,
    );
    let written = shape.body.checked_add(1)?;
    let source = ExecutedScriptSource::from_word(
        site.clone(),
        shape.body,
        tokens.words().get(written)?,
        values.get(written)?,
        observation.config,
    );
    if source.origin != site.source {
        return None;
    }
    Some(DeclaredProcedureBody {
        declaration: site.clone(),
        candidate: target.clone(),
        slot: key,
        frame: declared_procedure_frame(site, &namespace),
        source,
        namespace,
        parameters,
        logical_name: Some(name.clone()),
        logical_parameters_text: Some(parameters_text.clone()),
        logical_parameters_word: Some(original.get(shape.parameters.checked_add(1)?)?.clone()),
        original_parameters: None,
        authored_original_body_word: Some(original.get(written)?.clone()),
    })
}

fn logical_procedure_body_recipe(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    observation: &DeclarationLayoutObservation,
    registry: &tcl_registry::CommandRegistry,
    input: &crate::analyser::ResolvedAnalysisInput,
    advice: &super::super::OriginalCompilationLookupAdvice,
) -> Option<DeclaredProcedureBody> {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    if !advice.closed_logical_source_lookup()
        || input.lexer_config() != observation.config
        || input
            .context_registry()
            .commands()
            .snapshot()
            .semantic_key()
            != registry.snapshot().semantic_key()
        || !observation.entry.owns_source(&site.source, site.offset)
        || !observation
            .entry
            .owns_original_context(&observation.snapshot.state.source_variables)
        || observation.snapshot.state.variable_frame != *observation.entry.frame()
        || observation.snapshot.state.current_source_origin.as_ref() != Some(&site.source)
    {
        return None;
    }
    let original =
        capture_logical_procedure_words(site, tokens, observation, registry, input, advice)?;
    let dialect = advice.dialect();
    let actual_context = input.context_registry();
    let mut agreed = None;
    for target in advice.targets() {
        let effective = effective_words_for_target(tokens, target)?;
        let values =
            logical_procedure_values(target, &effective, &original, input, site, observation)?;
        let words = values
            .iter()
            .map(|value| tcl_registry::InvocationWord::Literal(value))
            .collect::<Vec<_>>();
        let (&head, arguments) = words.split_first()?;
        let resolution =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                registry,
                Some(actual_context.context()),
                tcl_registry::InvocationWords::structured(head, arguments).with_dialect(dialect),
                advice.realm(),
            );
        let shape = resolution
            .resolved()?
            .authored_source_procedure_arguments()?;
        let name = words.get(shape.name + 1)?.literal()?;
        let key = super::super::logical_definition::source_procedure_key(
            &observation.snapshot.state,
            &observation.namespace,
            name,
        )?;
        let parameters = super::super::logical_definition::static_formals(
            words.get(shape.parameters + 1)?.literal()?,
            observation.config.list_parse,
        )?;
        let crate::registry_invocation::InvocationWordOrigin::Written(written) =
            effective.origins.get(shape.body + 1)?
        else {
            return None;
        };
        let source = ExecutedScriptSource::from_word(
            site.clone(),
            written.checked_sub(1)?,
            tokens.words().get(*written)?,
            words.get(shape.body + 1)?.literal()?,
            observation.config,
        );
        let namespace = key.holder().into_owned();
        let body = DeclaredProcedureBody {
            declaration: site.clone(),
            candidate: target.clone(),
            slot: key,
            frame: declared_procedure_frame(site, &namespace),
            source,
            namespace,
            parameters,
            logical_name: Some(name.to_owned()),
            logical_parameters_text: Some(words.get(shape.parameters + 1)?.literal()?.to_owned()),
            logical_parameters_word: match effective.origins.get(shape.parameters + 1)? {
                crate::registry_invocation::InvocationWordOrigin::Written(written) => {
                    crate::registry_invocation::original_native_compiler_words(
                        site.source.source_image(),
                        tokens.words(),
                        site.offset,
                        observation.config,
                    )
                    .and_then(|original| original.get(*written).cloned())
                }
                _ => None,
            },
            original_parameters: None,
            authored_original_body_word: None,
        };
        if agreed.as_ref().is_some_and(|previous| previous != &body) {
            return None;
        }
        agreed = Some(body);
    }
    agreed
}

fn logical_procedure_static_facet(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    observation: &DeclarationLayoutObservation,
    registry: &tcl_registry::CommandRegistry,
    input: &crate::analyser::ResolvedAnalysisInput,
    advice: &super::super::OriginalCompilationLookupAdvice,
    body: &DeclaredProcedureBody,
) -> Option<OriginalLogicalProcedureStaticFacet> {
    (observation
        .snapshot
        .state
        .logical_source_name_advice_input()?
        == input)
        .then_some(())?;
    let original_body =
        logical_procedure_body_recipe(site, tokens, observation, registry, input, advice)?;
    (original_body == *body).then_some(())?;
    let original =
        capture_logical_procedure_words(site, tokens, observation, registry, input, advice)?;
    let actual_context = input.context_registry();
    let mut agreed = None;
    for target in advice.targets() {
        let effective = effective_words_for_target(tokens, target)?;
        let values =
            logical_procedure_values(target, &effective, &original, input, site, observation)?;
        let words = values
            .iter()
            .map(|value| tcl_registry::InvocationWord::Literal(value))
            .collect::<Vec<_>>();
        let (&head, arguments) = words.split_first()?;
        let resolution =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                registry,
                Some(actual_context.context()),
                tcl_registry::InvocationWords::structured(head, arguments)
                    .with_dialect(advice.dialect()),
                advice.realm(),
            );
        let statics = match resolution
            .resolved()?
            .authored_source_procedure_static_list()?
        {
            tcl_registry::resolved_invocation::AuthoredSourceProcedureStaticList::Absent => None,
            tcl_registry::resolved_invocation::AuthoredSourceProcedureStaticList::Argument(at) => {
                let ordinal = at.checked_add(1)?;
                let crate::registry_invocation::InvocationWordOrigin::Written(written) =
                    effective.origins.get(ordinal)?
                else {
                    return None;
                };
                let word = original.get(*written)?.word();
                if word.image() != site.source.source_image() || word.config() != observation.config
                {
                    return None;
                }
                let variables = tcl_registry::native_procedure::parse_static_variables(
                    words.get(ordinal)?.literal()?,
                )
                .ok()?;
                Some((at, variables))
            }
        };
        let facet = OriginalLogicalProcedureStaticFacet {
            input: input.clone(),
            source: Arc::clone(&site.source),
            frame: observation.snapshot.state.variable_frame.clone(),
            namespace: observation.namespace.clone(),
            variables: Arc::clone(&observation.snapshot.state.source_variables),
            config: observation.config,
            original: original.iter().map(|value| value.word().clone()).collect(),
            effective,
            values,
            statics,
        };
        if agreed.as_ref().is_some_and(|previous| previous != &facet) {
            return None;
        }
        agreed = Some(facet);
    }
    agreed
}

fn procedure_body_recipe(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    observation: &DeclarationLayoutObservation,
    facts: &tcl_registry::InvocationFacts,
    candidate: &super::super::SourceCommandTarget,
    operands: &ProcedureOperands<'_>,
) -> Option<DeclaredProcedureBody> {
    let header =
        procedure_declaration_header(site, tokens, observation, facts, candidate, operands)?;
    let source = ExecutedScriptSource::from_word(
        site.clone(),
        header.written.checked_sub(1)?,
        tokens.words().get(header.written)?,
        &header.value,
        observation.config,
    );
    if source.origin != site.source {
        return None;
    }
    Some(DeclaredProcedureBody {
        declaration: header.declaration,
        candidate: header.candidate,
        slot: header.slot,
        source,
        namespace: header.namespace,
        parameters: header.parameters,
        logical_name: None,
        logical_parameters_text: None,
        logical_parameters_word: None,
        original_parameters: header.original_parameters,
        frame: header.frame,
        authored_original_body_word: None,
    })
}

fn standalone_procedure_words(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    config: tcl_lexer::LexerConfig,
) -> Option<(Vec<tcl_lexer::NativeWord>, Vec<String>)> {
    let original = crate::registry_invocation::original_native_compiler_words(
        site.source.source_image(),
        tokens.words(),
        site.offset,
        config,
    )?;
    if original.iter().any(|word| word.group().expand) {
        return None;
    }
    let values = original
        .iter()
        .map(|word| {
            let value = tcl_syntax::word_rules::original_static_word_unicode_value(word)?;
            Some(std::str::from_utf8(&value).ok()?.to_owned())
        })
        .collect::<Option<Vec<_>>>()?;
    Some((original, values))
}

fn logical_procedure_values(
    target: &super::super::SourceCommandTarget,
    effective: &crate::registry_invocation::EffectiveCommandWords,
    original: &[super::super::logical_word_values::LogicalOriginalWordValue],
    input: &crate::analyser::ResolvedAnalysisInput,
    site: &CommandAllocationSite,
    observation: &DeclarationLayoutObservation,
) -> Option<Vec<String>> {
    let prefix = if target.prepended.is_empty() {
        Vec::new()
    } else {
        target
            .logical_original_prefix
            .as_ref()?
            .matches_state(&observation.snapshot.state)
            .then_some(())?;
        target
            .original_logical_prepended_name_inputs(
                input,
                site.source.source_image(),
                observation.config,
            )?
            .iter()
            .map(|value| std::str::from_utf8(value.bytes()).ok().map(str::to_owned))
            .collect::<Option<Vec<_>>>()?
    };
    let values = effective
        .origins
        .iter()
        .enumerate()
        .map(|(ordinal, origin)| {
            if ordinal == 0 {
                return Some(target.command.clone());
            }
            match origin {
                crate::registry_invocation::InvocationWordOrigin::Written(written) => {
                    Some(original.get(*written)?.value().to_owned())
                }
                crate::registry_invocation::InvocationWordOrigin::BindingPrefix(captured) => {
                    prefix.get(*captured).cloned()
                }
                _ => None,
            }
        })
        .collect::<Option<Vec<_>>>()?;
    Some(values)
}

fn capture_logical_procedure_words(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    observation: &DeclarationLayoutObservation,
    registry: &tcl_registry::CommandRegistry,
    input: &crate::analyser::ResolvedAnalysisInput,
    advice: &super::super::OriginalCompilationLookupAdvice,
) -> Option<Vec<super::super::logical_word_values::LogicalOriginalWordValue>> {
    let state = &observation.snapshot.state;
    let mut options = super::super::SourceAnalysisOptions::for_logical_source(input)?;
    options.invocation_realm = advice.realm();
    let mut context = super::super::root_source_execution_context(
        &state.variable_frame,
        &state.source_variables.namespace,
        &observation.namespace,
        observation.config,
        registry,
        options,
        tcl_lexer::SourceSegmentationPurpose::NestedScript,
    );
    context.invocation_offset = site.offset;
    let original = super::super::logical_word_values::LogicalOriginalWordValue::capture_invocation(
        tokens.words(),
        site.offset,
        state,
        context,
    )?;
    Some(original)
}

pub(in super::super) fn declared_procedure_frame(
    site: &CommandAllocationSite,
    namespace: &super::super::SourceNamespaceKey,
) -> crate::var_resolve::VariableExecutionFrame {
    crate::var_resolve::VariableExecutionFrame::Procedure {
        namespace: namespace.display().unwrap_or_default(),
        identity: super::super::source_activation_name(
            Some(&site.source),
            "declared-procedure-layout",
            site.offset,
        ),
    }
    .with_namespace_identity(namespace.clone())
}
