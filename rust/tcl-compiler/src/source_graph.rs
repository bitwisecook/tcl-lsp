// SPDX-License-Identifier: AGPL-3.0-or-later
//! Readonly source graph joins shared by editor graphs and structural diagrams.
//! These joins retain original source declarations and allocations. They supply
//! no editable range, runtime dispatch, entered frame or completion receipt.

use crate::analyser::{AnalysisResult, ProcDef};
use crate::ir::{Module, Procedure, ProcedureId, SourceIrulesEventBody};
use crate::signature_scan::original_name::SourceDeclarationMetadata;
use tcl_lexer::{LexerConfig, SourceImage};

/// Complete current analysis image and lexer configuration, without execution.
#[must_use]
pub fn current_analysis(
    source: &str,
    analysis: &AnalysisResult,
) -> Option<(SourceImage, LexerConfig)> {
    let image = SourceImage::document(source);
    let config = analysis.body_lexer_config?;
    analysis
        .matches_original_source_image(&image, config)
        .then_some((image, config))
}

/// Match current consumer and declaration sources against the retained original
/// invocation and actual command reference. Independent URI ownership remains
/// the caller's responsibility; links follow only already retained allocations.
#[must_use]
pub fn invocation_targets_declaration_in<T>(
    source: &str,
    analysis: &AnalysisResult,
    declaration_source: &str,
    declaration_analysis: &AnalysisResult,
    invocation: &crate::signature_scan::types::SignatureCommandInvocation,
    declaration: &SourceDeclarationMetadata<T>,
    follow_links: bool,
) -> bool {
    let Some((image, _)) = current_analysis(source, analysis) else {
        return false;
    };
    let Some((declaring_image, config)) =
        current_analysis(declaration_source, declaration_analysis)
    else {
        return false;
    };
    if declaration.name_input().source_image() != &declaring_image
        || declaration.name_input().lexer_config() != config
    {
        return false;
    }
    let (Some(input), Some(lookup)) =
        (&invocation.original_name_input, &invocation.original_lookup)
    else {
        return false;
    };
    if lookup.name_input() != input
        || lookup.site().source.source_image() != &image
        || input.policy() != declaration.name().policy()
    {
        return false;
    }
    let Some(reference) = &invocation.resolved_command_reference else {
        return false;
    };
    if !reference.matches_original_invocation_site(lookup.site()) {
        return false;
    }
    reference_matches_declaration(reference, declaration, follow_links)
}

/// Compare a retained original reference with one declaration allocation.
/// Direct editable references additionally require its publication slot/policy.
/// This comparison issues no editing or native dispatch authority.
#[must_use]
pub fn reference_matches_declaration<T>(
    reference: &crate::command_binding::SourceCommandReference,
    declaration: &SourceDeclarationMetadata<T>,
    follow_links: bool,
) -> bool {
    if !follow_links
        && (!reference.is_direct_definition()
            || reference.original_slot() != Some(declaration.name().slot())
            || reference.original_name_policy() != Some(declaration.name().policy()))
    {
        return false;
    }
    let selected = if follow_links {
        reference
            .linked_definition()
            .or_else(|| reference.definition())
    } else {
        reference.definition()
    };
    selected
        .is_some_and(|definition| &definition.allocation().site == declaration.declaration_site())
}

/// Select one compiled body only when its original implementation allocation,
/// executed source and namespace independently match this current declaration.
/// The returned body ID joins authenticated units without a display-name lookup.
#[must_use]
pub fn procedure_body_for_declaration<'a>(
    source: &str,
    analysis: &AnalysisResult,
    module: &'a Module,
    declaration: &SourceDeclarationMetadata<ProcDef>,
) -> Option<(ProcedureId, &'a Procedure)> {
    let (image, config) = current_analysis(source, analysis)?;
    if module.source != image
        || module.lexer_config != config
        || !analysis
            .original_procedure_declarations()
            .any(|row| row == declaration)
        || declaration.name_input().source_image() != &image
        || declaration.name_input().lexer_config() != config
        || module.registry_snapshot.as_ref()?.semantic_key()
            != analysis.resolved_registry()?.snapshot().semantic_key()
    {
        return None;
    }
    let mut bodies = module
        .procedures
        .iter()
        .chain(module.body_units.iter())
        .filter_map(|(id, procedure)| {
            (procedure.declaration_site.as_ref() == Some(declaration.declaration_site()))
                .then_some(())?;
            let executed = procedure.body.executed_source.as_deref()?;
            let namespace = procedure.body.namespace_context.as_deref()?;
            if matches_original_logical_declaration(analysis, module, declaration, *id, procedure) {
                return Some((*id, procedure));
            }
            let owner = module.procedure_implementation_bodies.iter().find(|body| {
                &body.allocation.site == declaration.declaration_site()
                    && body.source == *executed
                    && &body.namespace_key == namespace
            })?;
            if let Some(allocation) = module
                .original_declaration_body_units
                .get(id)
                .or_else(|| module.installed_procedure_body_units.get(id))
                && allocation != &owner.allocation
            {
                return None;
            }
            Some((*id, procedure))
        });
    let first = bodies.next()?;
    module.procedure_ids.owns(first.0).then_some(())?;
    bodies.next().is_none().then_some(first)
}

fn matches_original_logical_declaration(
    analysis: &AnalysisResult,
    module: &Module,
    declaration: &SourceDeclarationMetadata<ProcDef>,
    id: ProcedureId,
    procedure: &Procedure,
) -> bool {
    let Some(input) = analysis.resolved_input.as_ref() else {
        return false;
    };
    if !input.has_logical_source_name_context()
        || module.source_metadata_input.as_ref() != Some(input)
        || procedure.body_kind != crate::ir::ProcedureBodyKind::Named
        || !module.procedure_ids.owns(id)
        || !module
            .procedures
            .get(&id)
            .is_some_and(|current| std::ptr::eq(current, procedure))
    {
        return false;
    }
    let Some(realm) = analysis.retained_command_realm() else {
        return false;
    };
    let binding = realm.invocation_at_source("", declaration.declaration_site().offset);
    let Some((_, tokens)) = binding.original_recorded_command() else {
        return false;
    };
    if tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.invocation_site())
        != Some(declaration.declaration_site())
    {
        return false;
    }
    let Some(registry) = analysis.resolved_registry() else {
        return false;
    };
    let Some(recipe) = realm
        .source_bindings_ref()
        .original_logical_procedure_declaration(&tokens, registry, input)
    else {
        return false;
    };
    procedure.body.executed_source.as_deref() == Some(recipe.source())
        && procedure.body.namespace_context.as_deref() == Some(recipe.namespace())
        && procedure.body_offset == recipe.source().base()
        && procedure.body_source.as_deref() == recipe.source().try_text().ok()
        && Some(procedure.params_raw.as_str()) == recipe.parameters_text()
        && procedure.params.iter().map(String::as_str).eq(recipe
            .parameters()
            .iter()
            .map(|formal| formal.name.as_str()))
        && declaration.metadata().params.len() == recipe.parameters().len()
        && declaration
            .metadata()
            .params
            .iter()
            .zip(recipe.parameters())
            .all(|(stored, original)| {
                stored.name == original.name
                    && stored.has_default == original.default.is_some()
                    && stored.default_value.as_deref() == original.default.as_deref()
            })
}

/// Select a unanimous source call body from the actual Module's original words.
/// The issued ID supplies readonly reachability, without Native dispatch,
/// argument evaluation, frame, completion or erasure permission.
#[must_use]
pub fn procedure_body_for_original_call<'a>(
    source: &str,
    analysis: &AnalysisResult,
    module: &'a Module,
    tokens: &crate::ir::CommandTokens,
    registry: &tcl_registry::CommandRegistry,
) -> Option<(ProcedureId, &'a Procedure)> {
    module_matches_original_call(source, analysis, module, tokens, registry).then_some(())?;
    let binding = tokens.source_binding.as_ref()?;
    let metadata = binding.original_invocation_metadata_for_module(tokens, module, registry)?;
    let ids = match binding.original_invocation_source_purpose(tokens, metadata, registry)? {
        crate::command_binding::OriginalInvocationSourcePurpose::Logical => {
            crate::registry_invocation::original_logical_procedure_calls_for_module(
                tokens, module, registry,
            )?
            .into_iter()
            .map(|call| call.id())
            .collect::<Vec<_>>()
        }
        crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation => {
            crate::registry_invocation::original_authored_procedure_calls_for_module(
                tokens, module, registry,
            )?
            .into_iter()
            .map(|call| call.id())
            .collect::<Vec<_>>()
        }
        crate::command_binding::OriginalInvocationSourcePurpose::Execution
        | crate::command_binding::OriginalInvocationSourcePurpose::Standalone => {
            let reference = binding.original_command_reference(tokens)?;
            let definition = reference
                .linked_definition()
                .or_else(|| reference.definition())?;
            (definition.kind() == crate::command_binding::SourceCommandDefinitionKind::Procedure)
                .then_some(())?;
            let (id, procedure) =
                module.source_procedure_for_allocation(definition.allocation())?;
            module
                .retained_source_bindings
                .as_ref()?
                .matches_original_procedure(procedure)
                .then_some(())?;
            vec![id]
        }
    };
    let id = *ids.first()?;
    ids.iter().all(|candidate| *candidate == id).then_some(())?;
    module.procedure_ids.owns(id).then_some(())?;
    Some((id, module.procedures.get(&id)?))
}

/// Read the shared original Registry source schema for this actual Module call.
/// Printed command names and independently reconstructed namespaces cannot
/// select a schema or supply Native invocation or completion permission.
#[must_use]
pub fn registry_words_for_original_call(
    source: &str,
    analysis: &AnalysisResult,
    module: &Module,
    tokens: &crate::ir::CommandTokens,
    registry: &tcl_registry::CommandRegistry,
) -> Option<crate::registry_invocation::source_structure::OriginalRegistryWords> {
    module_matches_original_call(source, analysis, module, tokens, registry).then_some(())?;
    crate::registry_invocation::source_structure::original_registry_words_for_tokens(
        source, analysis, tokens,
    )
}

fn module_matches_original_call(
    source: &str,
    analysis: &AnalysisResult,
    module: &Module,
    tokens: &crate::ir::CommandTokens,
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    let Some((image, config)) = current_analysis(source, analysis) else {
        return false;
    };
    module.source == image
        && module.lexer_config == config
        && module
            .source_metadata_input
            .as_ref()
            .is_some_and(|input| analysis.resolved_input.as_ref() == Some(input))
        && module
            .registry_snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.semantic_key() == registry.snapshot().semantic_key())
        && tokens.synthetic.is_none()
        && tokens.source_binding.as_ref().is_some_and(|binding| {
            binding
                .original_invocation_metadata_for_module(tokens, module, registry)
                .is_some()
        })
}

/// The actual Registry-selected event declaration belonging to this procedure.
/// A reporting label cannot create an event or a worker/TMM execution grant.
#[must_use]
pub fn event_body_for_procedure<'a>(
    module: &'a Module,
    id: &ProcedureId,
    procedure: &Procedure,
    registry: &tcl_registry::CommandRegistry,
) -> Option<&'a SourceIrulesEventBody> {
    module.procedure_ids.owns(*id).then_some(())?;
    let event = module.irules_event_bodies.get(id)?;
    event
        .owns_procedure(procedure, &module.source, module.lexer_config, registry)
        .then_some(event.as_ref())
}
