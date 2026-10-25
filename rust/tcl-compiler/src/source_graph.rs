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
    if analysis.original_source_region.is_some()
        || analysis
            .retained_command_realm()
            .is_some_and(|realm| realm.source_bindings_ref().has_conditional_source_region())
    {
        return None;
    }
    let image = analysis.retained_command_realm()?.original_source_image()?;
    if image.channel() != tcl_lexer::SourceChannel::Document || image.bytes() != source.as_bytes() {
        return None;
    }
    let image = image.clone();
    let config = analysis.body_lexer_config?;
    analysis
        .matches_original_source_image(&image, config)
        .then_some((image, config))
}

/// Conditional new-source publication proposal under the actual whole document
/// and its source-transition owner. This supplies source collision advice only;
/// diagnostic applicability, review and editing remain independent obligations.
#[must_use]
pub fn original_logical_publication_proposal(
    source: &str,
    analysis: &AnalysisResult,
    template: &str,
    call: &str,
) -> Option<crate::command_binding::OriginalSourcePublicationProposal> {
    let (image, config) = current_analysis(source, analysis)?;
    let input = analysis.resolved_input.as_ref()?;
    if !input.has_logical_source_name_context() || input.lexer_config() != config {
        return None;
    }
    let proposal = analysis
        .retained_command_realm()?
        .source_bindings_ref()
        .original_logical_publication_proposal(input, template, call)?;
    proposal.matches_source(&image, input).then_some(proposal)
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

/// Select an original declaration body using its current source card or
/// independently matching implementation allocation, executed source and namespace.
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
        .chain(module.body_lowerings())
        .filter_map(|(id, procedure)| {
            (procedure.declaration_site.as_ref() == Some(declaration.declaration_site()))
                .then_some(())?;
            if let Some(card) = module.original_native_declaration_body_units.get(id) {
                let input = analysis.resolved_input.as_ref()?;
                if module.source_metadata_input.as_ref() != Some(input)
                    || !module.procedure_ids.owns(*id)
                    || !card.matches_procedure(module, procedure)
                    || card.original_name_input().original_word_key()
                        != Some(declaration.name_input())
                    || declaration.metadata().params.len() != card.parameters().len()
                    || !declaration
                        .metadata()
                        .params
                        .iter()
                        .zip(card.parameters())
                        .all(|(stored, original)| {
                            stored.name == original.name
                                && stored.has_default == original.default.is_some()
                                && stored.default_value.as_deref() == original.default.as_deref()
                        })
                {
                    return None;
                }
                return Some((*id, procedure));
            }
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
                .matches_original_procedure_summary(id, procedure)
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

/// Original Registry source schema for one authentic complete command operand.
/// The current Module, full input and selected source occurrence remain owners;
/// lexical brackets alone supply no descriptor, Native dispatch or completion.
#[must_use]
pub fn registry_words_for_original_substitution(
    source: &str,
    analysis: &AnalysisResult,
    module: &Module,
    operand: &tcl_lexer::NativeWord,
    registry: &tcl_registry::CommandRegistry,
) -> Option<crate::registry_invocation::source_structure::OriginalRegistryWords> {
    let (image, config) = current_analysis(source, analysis)?;
    if module.source != image
        || module.lexer_config != config
        || operand.image() != &image
        || operand.config() != config
        || module.source_metadata_input.as_ref() != analysis.resolved_input.as_ref()
        || !module
            .registry_snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.semantic_key() == registry.snapshot().semantic_key())
        || !module
            .retained_source_bindings
            .as_ref()?
            .matches_module(module, registry)
    {
        return None;
    }
    let original =
        crate::registry_invocation::source_structure::original_single_source_command_substitution(
            operand,
        )?;
    let head = original.command().words.first()?;
    let selected = crate::registry_invocation::source_structure::source_registry_words_at(
        source,
        analysis,
        head.span().start(),
    )?;
    let input = analysis.resolved_input.as_ref()?;
    (selected.head_source()?.word()? == head
        && selected.matches_source(&image, config)
        && selected.matches_registry(registry)
        && selected.context() == Some(input.context_registry().context()))
    .then_some(selected)
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

/// The same authentic event retained by a current module-issued function body.
/// Source correspondence supplies no event activation or worker authority.
#[must_use]
pub fn event_body_for_function<'a>(
    module: &'a Module,
    id: &ProcedureId,
    function: &crate::compilation_unit::FunctionUnit,
    registry: &tcl_registry::CommandRegistry,
) -> Option<&'a SourceIrulesEventBody> {
    (function.body_id == Some(*id)).then_some(())?;
    let procedure = module.procedures.get(id)?;
    function.invocation_metadata_context_for_module(registry, module)?;
    let original = module.irules_event_bodies.get(id)?;
    let retained = function.irules_event_body.as_ref()?;
    std::sync::Arc::ptr_eq(original, retained).then_some(())?;
    event_body_for_procedure(module, id, procedure, registry)
}

/// Actual hosted declaration source body under this Module's issued ID.
/// A reporting label cannot supply a procedure publication or entered frame.
#[must_use]
pub fn hosted_procedure_body_for_procedure<'a>(
    module: &'a Module,
    id: &ProcedureId,
    procedure: &Procedure,
    registry: &tcl_registry::CommandRegistry,
) -> Option<&'a crate::command_binding::OriginalHostedProcedureBody> {
    module.procedure_ids.owns(*id).then_some(())?;
    let body = module.original_hosted_declaration_body_units.get(id)?;
    let current = module.body_units.get(id)?;
    body.matches_procedure(module, current, registry)
        .then_some(())?;
    body.matches_procedure(module, procedure, registry)
        .then_some(body.as_ref())
}

/// Select a hosted declaration's current Module-issued body through its retained
/// original words and body card. No publication or entered frame is established.
#[must_use]
pub fn hosted_procedure_body_for_declaration<'a>(
    source: &str,
    analysis: &AnalysisResult,
    module: &'a Module,
    declaration: &crate::signature_scan::vendor_name::VendorSourceDeclarationMetadata<ProcDef>,
) -> Option<(ProcedureId, &'a Procedure)> {
    let (image, config) = current_analysis(source, analysis)?;
    let input = analysis.resolved_input.as_ref()?;
    let registry = analysis.resolved_registry()?;
    if module.source != image
        || module.lexer_config != config
        || module.source_metadata_input.as_ref() != Some(input)
        || !analysis
            .original_vendor_procedure_declarations()
            .any(|row| row == declaration)
        || !declaration.name_input().matches_source(&image, config)
    {
        return None;
    }
    let mut bodies = module.body_units.iter().filter_map(|(id, procedure)| {
        let body = hosted_procedure_body_for_procedure(module, id, procedure, registry)?;
        body.matches_declaration(declaration)
            .then_some((*id, procedure))
    });
    let body = bodies.next()?;
    bodies.next().is_none().then_some(body)
}

/// The genuine hosted source declaration retained by its own current function.
/// This source-only correspondence issues neither event nor Native authority.
#[must_use]
pub fn hosted_procedure_body_for_function<'a>(
    module: &'a Module,
    id: &ProcedureId,
    function: &crate::compilation_unit::FunctionUnit,
    registry: &tcl_registry::CommandRegistry,
) -> Option<&'a crate::command_binding::OriginalHostedProcedureBody> {
    (function.body_id == Some(*id)).then_some(())?;
    let procedure = module.body_units.get(id)?;
    function.invocation_metadata_context_for_module(registry, module)?;
    let original = module.original_hosted_declaration_body_units.get(id)?;
    let retained = function.original_hosted_procedure_body.as_ref()?;
    std::sync::Arc::ptr_eq(original, retained).then_some(())?;
    hosted_procedure_body_for_procedure(module, id, procedure, registry)
}

#[cfg(test)]
mod event_function_owner_tests {
    use super::*;
    use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};

    fn build_unit(
        source: &str,
        context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
    ) -> CompilationUnit {
        let result = build_unit_borrowed(source, &context);
        drop(context);
        result
    }

    fn build_unit_borrowed(
        source: &str,
        context: &std::sync::Arc<tcl_registry::model::ContextRegistry>,
    ) -> CompilationUnit {
        let profile = tcl_dialect::DialectProfile::irules();
        let config = LexerConfig::for_file_grammar(profile.grammar);
        let options = UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        };
        CompilationUnit::build_with_context_registry(
            source,
            options,
            None,
            std::sync::Arc::clone(context),
        )
    }

    #[test]
    fn genuine_rule_loader_events_retain_only_their_current_original_operands() {
        let context = tcl_registry::model::ingress::resolve_environment("f5-irules")
            .default_context_registry();
        let source = "when HTTP_REQUEST {}; when CLIENT_ACCEPTED {}";
        let unit = build_unit(source, context.clone());
        let module = &unit.ir_module;
        assert_eq!(module.irules_event_bodies.len(), 2);
        assert!(module.source_entry.native_entry.is_none());
        let foreign =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let stale = tcl_lexer::SourceImage::from_bytes(
            b"changed source".as_slice(),
            module.source.channel(),
        );
        let changed_config = tcl_lexer::LexerConfig {
            strict_quoting: !module.lexer_config.strict_quoting,
            ..module.lexer_config
        };
        for (id, procedure) in &module.procedures {
            let event = event_body_for_procedure(module, id, procedure, context.commands())
                .expect("genuine retained declaration body");
            let word = event
                .event_word(&module.source, module.lexer_config, context.commands())
                .expect("original event operand is source geometry");
            assert_eq!(
                module.source.bytes().get(word.span().as_range()),
                Some(event.event().as_bytes())
            );
            assert!(
                event
                    .event_word(&stale, module.lexer_config, context.commands())
                    .is_none()
            );
            assert!(
                event
                    .event_word(&module.source, changed_config, context.commands())
                    .is_none()
            );
            assert!(
                event
                    .event_word(&module.source, module.lexer_config, foreign.commands())
                    .is_none()
            );
        }
    }

    #[test]
    fn shadowed_event_command_cannot_borrow_the_registry_event_operand() {
        let context = tcl_registry::model::ingress::resolve_environment("f5-irules")
            .default_context_registry();
        let unit = build_unit("proc when args {}; when HTTP_REQUEST {}", context);
        assert!(unit.ir_module.irules_event_bodies.is_empty());
    }

    fn ordinary_reporting_name_diagnostics(body: &str) -> Vec<crate::analyser::types::Diagnostic> {
        let context = tcl_registry::model::ingress::resolve_environment("f5-irules")
            .default_context_registry();
        let source = format!("proc ordinary {{}} {{{body}}}\nwhen HTTP_REQUEST {{puts request}}\n");
        let mut unit = build_unit(&source, context.clone());
        let id = *unit
            .ir_module
            .original_hosted_declaration_body_units
            .keys()
            .next()
            .unwrap();
        let original = unit.body_units.get(&id).unwrap();
        assert!(
            hosted_procedure_body_for_function(&unit.ir_module, &id, original, context.commands())
                .is_some()
        );
        unit.body_units.get_mut(&id).unwrap().name = "::when::RULE_INIT".into();
        unit.ir_module
            .body_units
            .get_mut(&id)
            .unwrap()
            .qualified_name = "::when::RULE_INIT".into();
        let mut analyser = crate::analyser::Analyser::new()
            .with_resolved_input(unit.ir_module.source_metadata_input.clone().unwrap());
        analyser.analyse_and_retain_result_for_test(&source, "f5-irules");
        analyser.result.diagnostics.clear();
        analyser.emit_cfg_ssa_diagnostics_with_cu(&unit, context.commands());
        analyser.result.diagnostics
    }

    #[test]
    fn ordinary_rule_init_reporting_name_cannot_suppress_worker_state_warning() {
        let diagnostics = ordinary_reporting_name_diagnostics("set ::static::counter 1");
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::Irule4001)
        );
    }

    #[test]
    fn ordinary_rule_init_reporting_name_cannot_authorise_unqualified_global_fix() {
        let all = ordinary_reporting_name_diagnostics("global counter; set counter 1");
        let diagnostics = all
            .iter()
            .filter(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::Irule6001)
            .collect::<Vec<_>>();
        assert!(!diagnostics.is_empty());
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.fixes.is_empty())
        );
    }

    #[test]
    fn event_function_owner_uses_actual_body_identity_and_ignores_report_names() {
        let context = tcl_registry::model::ingress::resolve_environment("f5-irules")
            .default_context_registry();
        let source =
            "proc ordinary {} {set static::fake 1}\nwhen HTTP_REQUEST {set static::counter 1}\n";
        let unit = build_unit(source, std::sync::Arc::clone(&context));
        let (id, original) = unit
            .procedures
            .iter()
            .find(|(id, _)| unit.ir_module.irules_event_bodies.contains_key(id))
            .unwrap();
        let event =
            event_body_for_function(&unit.ir_module, id, original, context.commands()).unwrap();
        assert_eq!(event.event(), "HTTP_REQUEST");
        let mut forged = original.clone();
        forged.name = "::when::RULE_INIT".into();
        assert_eq!(
            event_body_for_function(&unit.ir_module, id, &forged, context.commands())
                .unwrap()
                .event(),
            "HTTP_REQUEST"
        );
        let (ordinary_id, ordinary) = unit
            .body_units
            .iter()
            .find(|(id, _)| {
                unit.ir_module
                    .original_hosted_declaration_body_units
                    .contains_key(id)
            })
            .unwrap();
        assert!(
            hosted_procedure_body_for_function(
                &unit.ir_module,
                ordinary_id,
                ordinary,
                context.commands()
            )
            .is_some()
        );
        assert!(
            event_body_for_function(&unit.ir_module, ordinary_id, ordinary, context.commands())
                .is_none()
        );
        assert!(
            event_body_for_function(&unit.ir_module, ordinary_id, original, context.commands())
                .is_none()
        );
        let mut ordinary = ordinary.clone();
        ordinary.name = "::when::RULE_INIT".into();
        assert!(
            hosted_procedure_body_for_function(
                &unit.ir_module,
                ordinary_id,
                &ordinary,
                context.commands()
            )
            .is_some()
        );
        assert!(
            event_body_for_function(&unit.ir_module, ordinary_id, &ordinary, context.commands())
                .is_none()
        );
        let mut wrong_entry = unit.ir_module.clone();
        wrong_entry.source_entry.invocation_realm =
            tcl_dialect::model::InvocationRealm::InterpreterRuntime;
        assert!(
            hosted_procedure_body_for_function(
                &wrong_entry,
                ordinary_id,
                &ordinary,
                context.commands()
            )
            .is_none()
        );
        let saved_procedure = unit.ir_module.body_units.get(ordinary_id).unwrap();
        let mut removed_body = unit.ir_module.clone();
        removed_body.body_units.remove(ordinary_id);
        assert!(
            hosted_procedure_body_for_procedure(
                &removed_body,
                ordinary_id,
                saved_procedure,
                context.commands()
            )
            .is_none()
        );
        let mut wrong_body = unit.ir_module.clone();
        wrong_body
            .body_units
            .get_mut(ordinary_id)
            .unwrap()
            .body
            .statements
            .clear();
        assert!(
            hosted_procedure_body_for_function(
                &wrong_body,
                ordinary_id,
                &ordinary,
                context.commands()
            )
            .is_none()
        );
        assert!(
            hosted_procedure_body_for_procedure(
                &wrong_body,
                ordinary_id,
                saved_procedure,
                context.commands()
            )
            .is_none()
        );
        let mut wrong_kind = unit.ir_module.clone();
        wrong_kind
            .body_units
            .get_mut(ordinary_id)
            .unwrap()
            .body_kind = crate::ir::ProcedureBodyKind::Named;
        assert!(
            hosted_procedure_body_for_function(
                &wrong_kind,
                ordinary_id,
                &ordinary,
                context.commands()
            )
            .is_none()
        );
        assert!(
            hosted_procedure_body_for_function(&unit.ir_module, id, &ordinary, context.commands())
                .is_none()
        );
        assert!(
            hosted_procedure_body_for_function(
                &unit.ir_module,
                ordinary_id,
                &ordinary,
                tcl_registry::default_registry()
            )
            .is_none()
        );
        ordinary.original_hosted_procedure_body = ordinary
            .original_hosted_procedure_body
            .map(|card| std::sync::Arc::new(card.as_ref().clone()));
        assert!(
            hosted_procedure_body_for_function(
                &unit.ir_module,
                ordinary_id,
                &ordinary,
                context.commands()
            )
            .is_none()
        );
        forged.irules_event_body = forged
            .irules_event_body
            .map(|event| std::sync::Arc::new(event.as_ref().clone()));
        assert!(
            event_body_for_function(&unit.ir_module, id, &forged, context.commands()).is_none()
        );
        let mut changed = unit.ir_module.clone();
        changed.source = SourceImage::document(&format!("{source} "));
        assert!(event_body_for_function(&changed, id, original, context.commands()).is_none());
        let foreign = build_unit(source, context.clone());
        assert!(
            event_body_for_function(&foreign.ir_module, id, original, context.commands()).is_none()
        );
        assert!(
            event_body_for_function(
                &unit.ir_module,
                id,
                original,
                tcl_registry::default_registry()
            )
            .is_none()
        );
    }
}
