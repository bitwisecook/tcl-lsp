// SPDX-License-Identifier: AGPL-3.0-or-later
//! Readonly source graph joins shared by editor graphs and structural diagrams.
//! These joins retain original source declarations and allocations. They supply
//! no editable range, runtime dispatch, entered frame or completion receipt.

use crate::analyser::{AnalysisResult, ProcDef};
use crate::ir::{Module, Procedure, SourceIrulesEventBody};
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
/// Labels index authenticated units and do not select a source declaration.
#[must_use]
pub fn procedure_body_for_declaration<'a>(
    source: &str,
    analysis: &AnalysisResult,
    module: &'a Module,
    declaration: &SourceDeclarationMetadata<ProcDef>,
) -> Option<(&'a str, &'a Procedure)> {
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
        .filter_map(|(label, procedure)| {
            let executed = procedure.body.executed_source.as_deref()?;
            let namespace = procedure.body.namespace_context.as_deref()?;
            let owner = module.procedure_implementation_bodies.iter().find(|body| {
                &body.allocation.site == declaration.declaration_site()
                    && body.source == *executed
                    && &body.namespace_key == namespace
            })?;
            if let Some(allocation) = module
                .original_declaration_body_units
                .get(label)
                .or_else(|| module.installed_procedure_body_units.get(label))
                && allocation != &owner.allocation
            {
                return None;
            }
            Some((label.as_str(), procedure))
        });
    let first = bodies.next()?;
    bodies.next().is_none().then_some(first)
}

/// The actual Registry-selected event declaration belonging to this procedure.
/// A reporting label cannot create an event or a worker/TMM execution grant.
#[must_use]
pub fn event_body_for_procedure<'a>(
    module: &'a Module,
    label: &str,
    procedure: &Procedure,
    registry: &tcl_registry::CommandRegistry,
) -> Option<&'a SourceIrulesEventBody> {
    let event = module.irules_event_bodies.get(label)?;
    event
        .owns_procedure(procedure, &module.source, module.lexer_config, registry)
        .then_some(event.as_ref())
}
