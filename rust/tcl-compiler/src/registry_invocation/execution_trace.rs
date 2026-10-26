// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Execution trace hazards from the same selected invocation and original target.

use super::{InvocationMetadataContext, resolved_handler_invocation_with_metadata_context};
use tcl_registry::{
    CommandRegistry, StateTransition, TraceTarget, TraceTransition, TransitionSubject,
};

/// Read the purpose issued by the original binding, after its complete source
/// metadata join. Raw input eligibility never downgrades a Native point.
fn original_trace_source_purpose(
    registry: &CommandRegistry,
    metadata: InvocationMetadataContext<'_>,
    tokens: &crate::ir::CommandTokens,
) -> Option<crate::command_binding::OriginalInvocationSourcePurpose> {
    tokens
        .source_binding
        .as_ref()?
        .original_invocation_source_purpose(tokens, metadata, registry)
}

/// Possible trace keys, never a target binding or successful registration.
/// Native inputs require the genuine frozen non-head operand and retained
/// lookup geometry. Missing or noninjective geometry leaves the hazard dynamic.
/// Explicit Logical/standalone source advice has its separate written grammar.
pub(crate) fn original_execution_trace_keys(
    registry: &CommandRegistry,
    metadata: InvocationMetadataContext<'_>,
    tokens: &crate::ir::CommandTokens,
    subject: &TransitionSubject,
) -> Option<Vec<String>> {
    let purpose = original_trace_source_purpose(registry, metadata, tokens)?;
    if purpose == crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation {
        return None;
    }
    let selected =
        resolved_handler_invocation_with_metadata_context(registry, Some(metadata), tokens)?;
    let joined = selected.facts.state_transitions.declared()?.facts().iter().any(|fact| {
        matches!(&fact.transition,
            StateTransition::Trace(TraceTransition::Add { target: TraceTarget::Execution(original), .. }
                | TraceTransition::Remove { target: TraceTarget::Execution(original), .. })
            if original == subject)
    });
    if !joined {
        return None;
    }
    if matches!(
        purpose,
        crate::command_binding::OriginalInvocationSourcePurpose::Logical
            | crate::command_binding::OriginalInvocationSourcePurpose::Standalone
    ) {
        let written = tcl_syntax::naming::canonical_written_command(subject.literal()?);
        let key = tcl_syntax::naming::unroot_rooted_key(&written).unwrap_or(&written);
        return (!key.is_empty()).then(|| vec![key.to_owned()]);
    }
    let lookup = tokens
        .source_binding
        .as_ref()?
        .original_transition_command_lookup(tokens, subject.argument_index()?)?;
    let mut keys = Vec::new();
    for candidates in lookup.candidates() {
        if candidates.is_empty() {
            return None;
        }
        for slot in candidates {
            let written =
                tcl_syntax::naming::native_command_source_spelling(lookup.policy().recipe(), slot)?;
            let key = tcl_syntax::naming::unroot_rooted_key(&written)?;
            if key.is_empty() {
                return None;
            }
            if !keys.iter().any(|old| old == key) {
                keys.push(key.to_owned());
            }
        }
    }
    (!keys.is_empty()).then_some(keys)
}

/// Implicit operations have no actual invocation receiver. Native spelling
/// inventories, including an empty inventory, cannot supply token trace absence.
/// Explicit Logical/standalone advice retains its authored spelling model.
/// Authored simulation source advice cannot establish token trace absence.
pub(crate) fn trace_set_requires_original_owner(
    registry: &CommandRegistry,
    module: &crate::ir::Module,
    metadata: Option<InvocationMetadataContext<'_>>,
) -> bool {
    match metadata
        .and_then(|metadata| module.original_invocation_source_purpose(registry, metadata))
    {
        Some(
            crate::command_binding::OriginalInvocationSourcePurpose::Logical
            | crate::command_binding::OriginalInvocationSourcePurpose::Standalone,
        ) => false,
        Some(
            crate::command_binding::OriginalInvocationSourcePurpose::Execution
            | crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation,
        )
        | None => true,
    }
}

/// Trace hazard for the same actual selected invocation. Native absence comes
/// from its retained token/entered/source observer owner, never the module's
/// spelling inventory. The initial Native entry trace bit is checked even when
/// that inventory is empty. Descriptor labels remain independent pure data.
pub(crate) fn original_invocation_has_execution_trace(
    registry: &CommandRegistry,
    metadata: InvocationMetadataContext<'_>,
    tokens: &crate::ir::CommandTokens,
    descriptor: tcl_registry::CanonicalCommandDescriptorKey<'_>,
    traced: &std::collections::BTreeSet<String>,
) -> bool {
    let Some(purpose) = original_trace_source_purpose(registry, metadata, tokens) else {
        return true;
    };
    if purpose == crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation {
        return true;
    }
    let Some(selected) =
        resolved_handler_invocation_with_metadata_context(registry, Some(metadata), tokens)
    else {
        return true;
    };
    if selected.facts.descriptor_key() != descriptor {
        return true;
    }
    let Some(binding) = tokens.source_binding.as_ref() else {
        return true;
    };
    if purpose == crate::command_binding::OriginalInvocationSourcePurpose::Execution {
        let Some(target) = binding
            .proved_execution_target()
            .filter(|target| target.registry_backed)
        else {
            return true;
        };
        // The resolver may also use a handler envelope. That envelope cannot
        // stand in for the original actual execution target checked here.
        if binding.proved_handler_target() != Some(target) {
            return true;
        }
        return !binding.original_unobserved_native_dispatch_for_tokens(tokens, target);
    }
    if traced.is_empty() {
        return false;
    }
    let Some(reference) = binding.evaluated_command_reference() else {
        return true;
    };
    let crate::command_binding::SourceCommandReferenceBinding::Direct {
        kind: crate::command_binding::BindingKind::Builtin,
        identity,
    } = reference.binding()
    else {
        return true;
    };
    let Some(slot) = reference.slot() else {
        return true;
    };
    if slot != identity.origin {
        return true;
    }
    let Some(key) = tcl_syntax::naming::unroot_rooted_key(slot) else {
        return true;
    };
    traced.contains(key) || traced.contains(slot) || traced.contains(descriptor.label())
}

/// The direct procedure consumer retains its actual original call receiver.
/// A future fresh source allocation requires its separate sealed absence proof;
/// constructed keys and empty spelling inventories never provide that proof.
pub(crate) fn original_procedure_has_execution_trace(
    registry: &CommandRegistry,
    metadata: Option<InvocationMetadataContext<'_>>,
    tokens: Option<&crate::ir::CommandTokens>,
    callee: &str,
    traced: &std::collections::BTreeSet<String>,
) -> bool {
    let Some(metadata) = metadata.filter(|metadata| metadata.matches_registry(registry)) else {
        return true;
    };
    let Some(tokens) = tokens else {
        return true;
    };
    let Some(purpose) = original_trace_source_purpose(registry, metadata, tokens) else {
        return true;
    };
    if purpose == crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation {
        return true;
    }
    if matches!(
        purpose,
        crate::command_binding::OriginalInvocationSourcePurpose::Logical
            | crate::command_binding::OriginalInvocationSourcePurpose::Standalone
    ) {
        return traced.contains(callee)
            || traced.contains(tcl_syntax::naming::unroot_rooted_key(callee).unwrap_or(callee));
    }
    let Some(binding) = tokens.source_binding.as_ref() else {
        return true;
    };
    let Some(target) = binding.direct_procedure_target(callee) else {
        return true;
    };
    target.command != callee
        || !(binding.original_unobserved_native_dispatch_for_tokens(tokens, target)
            || binding.original_fresh_procedure_unobserved_for_tokens(tokens, target))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;

    fn native_unit(
        source: &str,
        context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
    ) -> crate::environment_ingress::RetainedNativeUnit {
        native_unit_with_setup(source, context, |_| {})
    }

    fn native_unit_with_setup(
        source: &str,
        context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
        setup: impl FnOnce(&mut tcl_vm::Vm),
    ) -> crate::environment_ingress::RetainedNativeUnit {
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let (owner, captured) =
            crate::environment_ingress::captured_native_entry_with_owner_using(profile, setup);
        native_unit_from_entry(source, context, owner, captured)
    }

    fn native_unit_from_entry(
        source: &str,
        context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
        owner: tcl_vm::Vm,
        captured: tcl_runtime_api::NativeCompilationEntry,
    ) -> crate::environment_ingress::RetainedNativeUnit {
        native_profile_unit_from_entry(
            source,
            context,
            owner,
            captured,
            tcl_dialect::DialectProfile::find("tcl9.0").unwrap(),
        )
    }

    fn native_profile_unit_from_entry(
        source: &str,
        context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
        owner: tcl_vm::Vm,
        captured: tcl_runtime_api::NativeCompilationEntry,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> crate::environment_ingress::RetainedNativeUnit {
        let entry = crate::command_binding::SourceAnalysisEntry {
            native_entry: Some(std::sync::Arc::new(captured)),
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..Default::default()
        };
        let unit = CompilationUnit::build_with_context_registry(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config: tcl_lexer::LexerConfig::for_dialect(profile.name),
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            Some(&entry),
            std::sync::Arc::clone(&context),
        );
        let retained = crate::environment_ingress::RetainedNativeUnit::new(unit, owner);
        drop(entry);
        drop(context);
        retained
    }

    fn tokens_at(unit: &CompilationUnit, offset: usize) -> &crate::ir::CommandTokens {
        let offset = u32::try_from(offset).unwrap();
        let function = unit.function_unit_at(offset);
        function
            .cfg
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .filter_map(|statement| {
                crate::ir::CommandBindingSites::unanimous_statement_source_tokens(
                    &function.cfg.command_binding_sites,
                    statement,
                )
            })
            .find(|tokens| {
                tokens
                    .argv
                    .first()
                    .is_some_and(|span| span.start() == offset)
            })
            .expect("actual complete source vector at its original head")
    }

    fn execution_subject(
        registry: &CommandRegistry,
        metadata: InvocationMetadataContext<'_>,
        tokens: &crate::ir::CommandTokens,
    ) -> TransitionSubject {
        let selected =
            resolved_handler_invocation_with_metadata_context(registry, Some(metadata), tokens)
                .expect("actual selected trace handler");
        selected
            .facts
            .state_transitions
            .declared()
            .unwrap()
            .facts()
            .iter()
            .find_map(|fact| match &fact.transition {
                StateTransition::Trace(TraceTransition::Add {
                    target: TraceTarget::Execution(subject),
                    ..
                }) => Some(subject.clone()),
                _ => None,
            })
            .expect("actual located execution subject")
    }

    #[test]
    fn fresh_native_source_procedure_has_separate_trace_absence() {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let registry = context.commands();
        let source = "proc p {} {return READY}; p";
        let unit = native_unit(source, std::sync::Arc::clone(&context));
        let metadata = InvocationMetadataContext::for_module(registry, &unit.ir_module).unwrap();
        let tokens = tokens_at(&unit, source.rfind('p').unwrap());
        let binding = tokens.source_binding.as_ref().unwrap();
        let target = binding.direct_procedure_target("::p").unwrap();
        assert!(target.identity.as_ref().unwrap().runtime.is_none());
        assert!(!binding.original_unobserved_native_dispatch_for_tokens(tokens, target));
        assert!(binding.original_fresh_procedure_unobserved_for_tokens(tokens, target));
        assert!(!original_procedure_has_execution_trace(
            registry,
            Some(metadata),
            Some(tokens),
            "::p",
            &std::collections::BTreeSet::new()
        ));

        let mut changed = tokens.clone();
        changed.argv_texts[0] = "OTHER".to_owned();
        assert!(!binding.original_fresh_procedure_unobserved_for_tokens(&changed, target));
        let mut generation = target.clone();
        generation.implementation_generation = generation.implementation_generation.wrapping_add(1);
        assert!(!binding.original_fresh_procedure_unobserved_for_tokens(tokens, &generation));
        let mut wrong_frame = tokens.clone();
        wrong_frame.source_binding.as_mut().unwrap().variable_frame =
            crate::var_resolve::VariableExecutionFrame::Unknown;
        assert!(
            !wrong_frame
                .source_binding
                .as_ref()
                .unwrap()
                .original_fresh_procedure_unobserved_for_tokens(&wrong_frame, target)
        );
        let other = native_unit(source, std::sync::Arc::clone(&context));
        assert!(!binding.original_fresh_procedure_unobserved_for_tokens(
            tokens_at(&other, source.rfind('p').unwrap()),
            target
        ));
        let foreign = tcl_registry::model::ingress::static_context_for("tcl8.4");
        assert!(original_procedure_has_execution_trace(
            registry,
            Some(foreign.into()),
            Some(tokens),
            "::p",
            &std::collections::BTreeSet::new()
        ));

        let retained = (*unit).clone();
        drop(unit);
        let tokens = tokens_at(&retained, source.rfind('p').unwrap());
        let metadata =
            InvocationMetadataContext::for_module(registry, &retained.ir_module).unwrap();
        assert!(original_procedure_has_execution_trace(
            registry,
            Some(metadata),
            Some(tokens),
            "::p",
            &std::collections::BTreeSet::new()
        ));
    }

    #[test]
    fn fresh_native_source_procedure_refuses_observers_and_intervening_mutations() {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let registry = context.commands();
        for source in [
            "proc p {} {return READY}; trace add execution p enter list; p",
            "proc p {} {return READY}; trace add execution p leave list; p",
            "trace add execution proc enter list; proc p {} {return READY}; p",
            "trace add execution proc leave list; proc p {} {return READY}; p",
            "proc p {} {return READY}; rename p q; rename q p; p",
            "proc p {} {return READY}; interp alias {} a {} p; p",
            "proc p {} {return FIRST}; proc p {} {return READY}; p",
            "set body {return READY}; proc p {} $body; p",
            "proc p {} {return READY}; unknownOperation; p",
        ] {
            let unit = native_unit(source, std::sync::Arc::clone(&context));
            let tokens = tokens_at(&unit, source.rfind('p').unwrap());
            let metadata =
                InvocationMetadataContext::for_module(registry, &unit.ir_module).unwrap();
            assert!(
                original_procedure_has_execution_trace(
                    registry,
                    Some(metadata),
                    Some(tokens),
                    "::p",
                    &std::collections::BTreeSet::new()
                ),
                "{source}"
            );
        }
    }

    #[test]
    fn fresh_native_source_procedure_refuses_existing_native_destination() {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let source = "proc p {} {return READY}; p";
        let unit = native_unit_with_setup(source, std::sync::Arc::clone(&context), |owner| {
            invoke_setup(owner, "proc", &["p", "", "return EXISTING"]);
        });
        let tokens = tokens_at(&unit, source.rfind('p').unwrap());
        let binding = tokens.source_binding.as_ref().unwrap();
        let target = binding.direct_procedure_target("::p").unwrap();
        assert!(!binding.original_fresh_procedure_unobserved_for_tokens(tokens, target));
    }

    #[test]
    fn fresh_native_source_procedure_requires_unobserved_native_factory() {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let registry = context.commands();
        let source = "proc p {} {return READY}; p";
        for operation in ["enter", "leave", "enterstep", "leavestep"] {
            let unit = native_unit_with_setup(source, std::sync::Arc::clone(&context), |owner| {
                invoke_setup(
                    owner,
                    "trace",
                    &["add", "execution", "proc", operation, "list"],
                );
            });
            let tokens = tokens_at(&unit, source.rfind('p').unwrap());
            let metadata =
                InvocationMetadataContext::for_module(registry, &unit.ir_module).unwrap();
            assert!(
                original_procedure_has_execution_trace(
                    registry,
                    Some(metadata),
                    Some(tokens),
                    "::p",
                    &std::collections::BTreeSet::new()
                ),
                "{operation}"
            );
        }
    }

    #[test]
    fn fresh_native_source_procedure_requires_same_live_entry_capture() {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let registry = context.commands();
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let source = "proc p {} {return READY}; p";
        for defect in 0..4 {
            let (owner, mut entry) =
                crate::environment_ingress::captured_native_entry_with_owner(profile);
            match defect {
                0 => entry.epoch = entry.epoch.wrapping_add(1),
                1 => entry.empty_literal_world = None,
                2 => entry.invocation_policy = None,
                _ => entry.closed = false,
            }
            let unit =
                native_unit_from_entry(source, std::sync::Arc::clone(&context), owner, entry);
            let tokens = tokens_at(&unit, source.rfind('p').unwrap());
            let metadata =
                InvocationMetadataContext::for_module(registry, &unit.ir_module).unwrap();
            assert!(
                original_procedure_has_execution_trace(
                    registry,
                    Some(metadata),
                    Some(tokens),
                    "::p",
                    &std::collections::BTreeSet::new()
                ),
                "{defect}"
            );
        }
    }

    #[test]
    fn original_trace_subject_requires_its_actual_non_head_operand_and_context() {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let registry = context.commands();
        for target in ["target", "::::target"] {
            let source = format!("proc target args {{}}; trace add execution {target} enter list");
            let unit = native_unit(&source, std::sync::Arc::clone(&context));
            let metadata =
                InvocationMetadataContext::for_module(registry, &unit.ir_module).unwrap();
            let tokens = tokens_at(&unit, source.find("trace add").unwrap());
            let subject = execution_subject(registry, metadata, tokens);
            assert_eq!(subject.argument_index(), Some(2));
            assert_eq!(
                original_execution_trace_keys(registry, metadata, tokens, &subject),
                Some(vec!["target".to_owned()])
            );
            // Equal authored text supplies neither the selected ordinal nor input.
            assert!(
                original_execution_trace_keys(
                    registry,
                    metadata,
                    tokens,
                    &TransitionSubject::Literal(target.to_owned())
                )
                .is_none()
            );
            let wrong = TransitionSubject::LocatedLiteral {
                value: target.to_owned(),
                argument_index: 0,
            };
            assert!(original_execution_trace_keys(registry, metadata, tokens, &wrong).is_none());
            let mut missing = tokens.clone();
            missing.source_binding = None;
            assert!(
                original_execution_trace_keys(registry, metadata, &missing, &subject).is_none()
            );
            let foreign = tcl_registry::model::ingress::static_context_for("tcl8.4");
            assert!(
                original_execution_trace_keys(registry, foreign.into(), tokens, &subject).is_none()
            );
        }
    }

    #[test]
    fn original_opaque_trace_target_keeps_the_hazard_dynamic() {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let registry = context.commands();
        let source = "proc :target args {}; trace add execution :target enter list";
        let unit = native_unit(source, std::sync::Arc::clone(&context));
        let metadata = InvocationMetadataContext::for_module(registry, &unit.ir_module).unwrap();
        let tokens = tokens_at(&unit, source.find("trace add").unwrap());
        let subject = execution_subject(registry, metadata, tokens);
        let lookup = tokens
            .source_binding
            .as_ref()
            .unwrap()
            .original_transition_command_lookup(tokens, subject.argument_index().unwrap())
            .unwrap();
        assert!(
            lookup
                .candidates()
                .iter()
                .flatten()
                .any(|slot| slot.simple.as_bytes() == b":target")
        );
        assert!(original_execution_trace_keys(registry, metadata, tokens, &subject).is_none());
        assert!(unit.ir_module.has_dynamic_trace);
    }

    fn invoke_setup(vm: &mut tcl_vm::Vm, command: &str, words: &[&str]) {
        let values = words
            .iter()
            .map(|word| tcl_vm::Value::new_native_string_bytes(word.as_bytes()))
            .collect::<Vec<_>>();
        assert_eq!(
            vm.try_invoke_command(command, &values).unwrap().code,
            tcl_runtime_api::Code::Ok
        );
    }

    fn selected_trace_hazard(
        registry: &CommandRegistry,
        unit: &CompilationUnit,
        offset: usize,
        descriptor: tcl_registry::CanonicalCommandDescriptorKey<'_>,
    ) -> bool {
        let metadata = InvocationMetadataContext::for_module(registry, &unit.ir_module).unwrap();
        original_invocation_has_execution_trace(
            registry,
            metadata,
            tokens_at(unit, offset),
            descriptor,
            &unit.ir_module.traced_commands,
        )
    }

    #[test]
    fn original_native_token_observers_follow_add_remove_and_rename() {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let registry = context.commands();
        let plain = native_unit("llength {a b}", std::sync::Arc::clone(&context));
        let metadata = InvocationMetadataContext::for_module(registry, &plain.ir_module).unwrap();
        let selected = resolved_handler_invocation_with_metadata_context(
            registry,
            Some(metadata),
            tokens_at(&plain, 0),
        )
        .unwrap();
        for (source, command, observed) in [
            (
                "proc cb args {}; trace add execution string enter cb; llength {a b}",
                "llength {a b}",
                false,
            ),
            (
                "proc cb args {}; trace add execution llength enter cb; llength {a b}",
                "llength {a b}",
                true,
            ),
            (
                "proc cb args {}; trace add execution llength enter cb; trace remove execution llength enter cb; llength {a b}",
                "llength {a b}",
                false,
            ),
            (
                "proc cb args {}; trace add execution llength enter cb; rename llength kept; kept {a b}",
                "kept {a b}",
                true,
            ),
            (
                "proc cb args {}; trace add execution llength enter cb; rename llength kept; trace remove execution kept enter cb; kept {a b}",
                "kept {a b}",
                false,
            ),
        ] {
            let unit = native_unit(source, std::sync::Arc::clone(&context));
            let offset = source.rfind(command).unwrap();
            let binding = tokens_at(&unit, offset).source_binding.as_ref().unwrap();
            assert_eq!(!binding.unobserved_native_dispatch(), observed, "{source}");
            assert_eq!(
                selected_trace_hazard(registry, &unit, offset, selected.facts.descriptor_key()),
                observed,
                "{source}"
            );
            if !observed {
                let local =
                    InvocationMetadataContext::for_module(registry, &unit.ir_module).unwrap();
                assert!(
                    crate::gvn::is_pure_tokens_with_metadata_context(
                        registry,
                        Some(local),
                        tokens_at(&unit, offset),
                        &unit.ir_module.traced_commands,
                        false
                    ),
                    "{source}"
                );
            }
        }
    }

    #[test]
    fn captured_native_trace_flag_is_reached_before_empty_module_inventory() {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let registry = context.commands();
        let plain = native_unit("llength {a b}", std::sync::Arc::clone(&context));
        let metadata = InvocationMetadataContext::for_module(registry, &plain.ir_module).unwrap();
        let selected = resolved_handler_invocation_with_metadata_context(
            registry,
            Some(metadata),
            tokens_at(&plain, 0),
        )
        .unwrap();
        for (stage, name, observed) in [
            (0, "llength", false),
            (1, "llength", true),
            (2, "llength", false),
            (3, "kept", true),
        ] {
            let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
            let (owner, captured) =
                crate::environment_ingress::captured_native_entry_with_owner_using(profile, |vm| {
                    let original = vm.registered_command_token_bytes(b"llength").unwrap();
                    if stage != 0 {
                        invoke_setup(
                            vm,
                            "trace",
                            &["add", "execution", "llength", "enter", "list"],
                        );
                    }
                    if stage == 2 {
                        invoke_setup(
                            vm,
                            "trace",
                            &["remove", "execution", "llength", "enter", "list"],
                        );
                    }
                    if stage == 3 {
                        invoke_setup(vm, "rename", &["llength", "kept"]);
                        assert_eq!(
                            vm.registered_command_token_bytes(b"kept")
                                .unwrap()
                                .generation(),
                            original.generation()
                        );
                    }
                });
            let row = captured
                .commands
                .iter()
                .find(|row| {
                    row.slot.namespace.is_root() && row.slot.simple.as_bytes() == name.as_bytes()
                })
                .unwrap();
            assert_eq!(row.has_execution_trace, observed);
            let original_token = row.token;
            let original_interpreter = captured.interpreter;
            let source = format!("{name} {{a b}}");
            let unit =
                native_unit_from_entry(&source, std::sync::Arc::clone(&context), owner, captured);
            assert!(unit.ir_module.traced_commands.is_empty());
            let tokens = tokens_at(&unit, 0);
            if let Some(target) = tokens.source_binding.as_ref().unwrap().proved_target() {
                let runtime = target.identity.as_ref().unwrap().runtime.unwrap();
                assert_eq!(runtime.token, original_token);
                assert_eq!(runtime.interpreter, original_interpreter);
            }
            assert_eq!(
                selected_trace_hazard(registry, &unit, 0, selected.facts.descriptor_key()),
                observed
            );
            let local = InvocationMetadataContext::for_module(registry, &unit.ir_module).unwrap();
            assert_eq!(
                crate::gvn::is_pure_tokens_with_metadata_context(
                    registry,
                    Some(local),
                    tokens,
                    &unit.ir_module.traced_commands,
                    false
                ),
                !observed
            );
        }
    }

    #[test]
    fn original_native_trace_owner_survives_alias_and_root_fallback_without_label_substitution() {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let registry = context.commands();
        let plain = native_unit("llength {a b}", std::sync::Arc::clone(&context));
        let metadata = InvocationMetadataContext::for_module(registry, &plain.ir_module).unwrap();
        let selected = resolved_handler_invocation_with_metadata_context(
            registry,
            Some(metadata),
            tokens_at(&plain, 0),
        )
        .unwrap();
        for (source, command, observed) in [
            (
                "proc cb args {}; trace add execution string enter cb; interp alias {} shortcut {} llength; shortcut {a b}",
                "shortcut {a b}",
                false,
            ),
            (
                "proc cb args {}; trace add execution llength enter cb; interp alias {} shortcut {} llength; shortcut {a b}",
                "shortcut {a b}",
                true,
            ),
            (
                "proc cb args {}; trace add execution string enter cb; namespace eval n {llength {a b}}",
                "llength {a b}",
                false,
            ),
            (
                "proc cb args {}; trace add execution llength enter cb; namespace eval n {llength {a b}}",
                "llength {a b}",
                true,
            ),
        ] {
            let unit = native_unit(source, std::sync::Arc::clone(&context));
            assert_eq!(
                selected_trace_hazard(
                    registry,
                    &unit,
                    source.rfind(command).unwrap(),
                    selected.facts.descriptor_key()
                ),
                observed,
                "{source}"
            );
        }
    }

    #[test]
    fn original_native_trace_absence_requires_full_vector_target_and_actual_receiver() {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let registry = context.commands();
        let unit = native_unit("llength {a b}", std::sync::Arc::clone(&context));
        let metadata = InvocationMetadataContext::for_module(registry, &unit.ir_module).unwrap();
        let tokens = tokens_at(&unit, 0);
        let selected =
            resolved_handler_invocation_with_metadata_context(registry, Some(metadata), tokens)
                .unwrap();
        let empty = std::collections::BTreeSet::new();
        assert!(!original_invocation_has_execution_trace(
            registry,
            metadata,
            tokens,
            selected.facts.descriptor_key(),
            &empty
        ));
        let binding = tokens.source_binding.as_ref().unwrap();
        let target = binding.proved_execution_target().unwrap();
        assert!(binding.original_unobserved_native_dispatch_for_tokens(tokens, target));
        let mut other_generation = target.clone();
        other_generation.implementation_generation =
            other_generation.implementation_generation.wrapping_add(1);
        assert!(!binding.original_unobserved_native_dispatch_for_tokens(tokens, &other_generation));
        let mut wrong_frame = tokens.clone();
        wrong_frame.source_binding.as_mut().unwrap().variable_frame =
            crate::var_resolve::VariableExecutionFrame::Unknown;
        assert!(
            !wrong_frame
                .source_binding
                .as_ref()
                .unwrap()
                .original_unobserved_native_dispatch_for_tokens(&wrong_frame, target)
        );
        let mut missing = tokens.clone();
        missing.source_binding = None;
        assert!(original_invocation_has_execution_trace(
            registry,
            metadata,
            &missing,
            selected.facts.descriptor_key(),
            &empty
        ));
        let mut changed = tokens.clone();
        changed.argv_texts[1] = "OTHER".to_owned();
        assert!(original_invocation_has_execution_trace(
            registry,
            metadata,
            &changed,
            selected.facts.descriptor_key(),
            &empty
        ));
        let foreign = tcl_registry::model::ingress::static_context_for("tcl8.4");
        assert!(original_invocation_has_execution_trace(
            registry,
            foreign.into(),
            tokens,
            selected.facts.descriptor_key(),
            &empty
        ));
        let standalone = tcl_registry::model::semantic::SemanticContext::for_profile(
            registry.profile().unwrap(),
        );
        assert!(original_invocation_has_execution_trace(
            registry,
            standalone.into(),
            tokens,
            selected.facts.descriptor_key(),
            &empty
        ));
        assert!(original_procedure_has_execution_trace(
            registry,
            Some(standalone.into()),
            Some(tokens),
            "::p",
            &empty
        ));
        // A descriptor-only implicit operation has no actual original receiver.
        assert!(trace_set_requires_original_owner(
            registry,
            &unit.ir_module,
            Some(metadata)
        ));
    }
    fn assert_same_jim_input_keeps_independent_driver_purposes(
        registry: &tcl_registry::CommandRegistry,
        metadata: InvocationMetadataContext<'_>,
        tokens: &crate::ir::CommandTokens,
        profile: &'static tcl_dialect::DialectProfile,
        context: &std::sync::Arc<tcl_registry::model::ContextRegistry>,
    ) {
        use crate::command_binding::OriginalInvocationSourcePurpose;
        // The same raw Jim assistance input can independently select a genuine
        // source-only driver. Its own entry and vector retain that purpose; it
        // cannot change the purpose of the supplied Native binding above.
        let input = metadata.source_analysis_input().unwrap();
        let entry = crate::command_binding::SourceAnalysisEntry::for_logical_source(input).unwrap();
        let logical = CompilationUnit::build_with_context_registry(
            "llength {a b}",
            crate::compilation_unit::UnitBuildOptions {
                registry,
                defer_top_level: false,
                config: input.lexer_config(),
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            Some(&entry),
            std::sync::Arc::clone(context),
        );
        let logical_metadata =
            InvocationMetadataContext::for_module(registry, &logical.ir_module).unwrap();
        let logical_tokens = tokens_at(&logical, 0);
        assert!(logical.ir_module.source_entry.native_entry.is_none());
        assert_eq!(
            original_trace_source_purpose(registry, logical_metadata, logical_tokens),
            Some(OriginalInvocationSourcePurpose::Logical)
        );
        assert_eq!(
            logical
                .ir_module
                .original_invocation_source_purpose(registry, logical_metadata),
            Some(OriginalInvocationSourcePurpose::Logical)
        );
        assert!(!trace_set_requires_original_owner(
            registry,
            &logical.ir_module,
            Some(logical_metadata)
        ));
        assert_eq!(
            original_trace_source_purpose(registry, logical_metadata, tokens),
            Some(OriginalInvocationSourcePurpose::Execution)
        );
    }

    #[test]
    fn genuine_jim_entry_cannot_borrow_raw_logical_metadata_for_trace_absence() {
        use crate::command_binding::OriginalInvocationSourcePurpose;
        let profile = tcl_dialect::DialectProfile::find("jimtcl").unwrap();
        let context =
            tcl_registry::model::ingress::resolve_environment("jimtcl").default_context_registry();
        let registry = context.commands();
        let (owner, captured) =
            crate::environment_ingress::captured_native_entry_with_owner(profile);
        assert!(captured.command_name_policy().is_none());
        assert!(!captured.commands.is_empty());
        let unit = native_profile_unit_from_entry(
            "llength {a b}",
            std::sync::Arc::clone(&context),
            owner,
            captured,
            profile,
        );
        let metadata = InvocationMetadataContext::for_module(registry, &unit.ir_module).unwrap();
        assert!(metadata.permits_logical_source_names());
        assert!(unit.ir_module.source_entry.native_entry.is_some());
        let tokens = tokens_at(&unit, 0);
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(binding.logical_source_name_advice_input().is_none());
        assert!(
            binding
                .original_invocation_metadata_for_input(
                    tokens,
                    metadata.source_analysis_input().unwrap(),
                    registry,
                )
                .is_some()
        );
        assert_eq!(
            original_trace_source_purpose(registry, metadata, tokens),
            Some(OriginalInvocationSourcePurpose::Execution)
        );
        assert_eq!(
            unit.ir_module
                .original_invocation_source_purpose(registry, metadata),
            Some(OriginalInvocationSourcePurpose::Execution)
        );
        let empty = std::collections::BTreeSet::new();
        assert!(trace_set_requires_original_owner(
            registry,
            &unit.ir_module,
            Some(metadata)
        ));
        assert!(!crate::gvn::is_pure_tokens_with_metadata_context(
            registry,
            Some(metadata),
            tokens,
            &empty,
            false,
        ));
        assert!(original_procedure_has_execution_trace(
            registry,
            Some(metadata),
            Some(tokens),
            "::p",
            &empty,
        ));
        // These rows come from the real captured Jim entry. No supported Jim
        // command-trace registration or trace-bit mutation is invented here.
        let subject = TransitionSubject::LocatedLiteral {
            value: "llength".to_owned(),
            argument_index: 0,
        };
        assert!(original_execution_trace_keys(registry, metadata, tokens, &subject).is_none());
        let foreign_context =
            tcl_registry::model::ingress::resolve_environment("tcl").default_context_registry();
        let foreign_profile = tcl_dialect::DialectProfile::find("tcl").unwrap();
        let foreign_input = crate::analyser::ResolvedAnalysisInput::new(
            foreign_profile,
            foreign_profile,
            std::sync::Arc::clone(&foreign_context),
            tcl_lexer::LexerConfig::for_dialect("tcl"),
        );
        let foreign = InvocationMetadataContext::for_analysis_input(
            foreign_context.commands(),
            &foreign_input,
        )
        .unwrap();
        assert!(foreign.permits_logical_source_names());
        assert!(original_trace_source_purpose(registry, foreign, tokens).is_none());
        assert!(original_procedure_has_execution_trace(
            registry,
            Some(foreign),
            Some(tokens),
            "::p",
            &empty,
        ));
        assert!(original_execution_trace_keys(registry, foreign, tokens, &subject).is_none());

        assert_same_jim_input_keeps_independent_driver_purposes(
            registry, metadata, tokens, profile, &context,
        );
    }

    #[test]
    fn authored_simulation_source_advice_keeps_trace_absence_unavailable() {
        use crate::command_binding::OriginalInvocationSourcePurpose;
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.4").default_context_registry();
        let registry = context.commands();
        let profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
        let config = tcl_lexer::LexerConfig::for_dialect("tcl8.4");
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            config,
        );
        let policy = tcl_syntax::naming::ExecutionNamePolicy::NativeRecipe(
            tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_4),
        );
        let entry = crate::command_binding::SourceAnalysisEntry {
            metadata_context:
                crate::registry_invocation::OwnedInvocationMetadataContext::for_source_input(Some(
                    &input,
                )),
            execution_name_policy: Some(policy),
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            ..Default::default()
        };
        let unit = CompilationUnit::build_with_context_registry(
            "llength {a b}",
            crate::compilation_unit::UnitBuildOptions {
                registry,
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            Some(&entry),
            std::sync::Arc::clone(&context),
        );
        let metadata = InvocationMetadataContext::for_module(registry, &unit.ir_module).unwrap();
        assert_eq!(
            unit.ir_module.source_entry.execution_name_policy,
            Some(policy)
        );
        assert!(unit.ir_module.source_entry.native_entry.is_none());
        assert!(unit.ir_module.source_entry.logical_source_input.is_none());
        assert_eq!(
            unit.ir_module
                .original_invocation_source_purpose(registry, metadata),
            Some(OriginalInvocationSourcePurpose::AuthoredSimulation),
        );
        let tokens = tokens_at(&unit, 0);
        let selected =
            resolved_handler_invocation_with_metadata_context(registry, Some(metadata), tokens)
                .expect("actual selected software-model command facts");
        let empty = std::collections::BTreeSet::new();
        assert!(trace_set_requires_original_owner(
            registry,
            &unit.ir_module,
            Some(metadata)
        ));
        assert!(original_invocation_has_execution_trace(
            registry,
            metadata,
            tokens,
            selected.facts.descriptor_key(),
            &empty,
        ));
        assert!(original_procedure_has_execution_trace(
            registry,
            Some(metadata),
            Some(tokens),
            "::p",
            &empty,
        ));
        let subject = TransitionSubject::LocatedLiteral {
            value: "llength".to_owned(),
            argument_index: 0,
        };
        assert!(original_execution_trace_keys(registry, metadata, tokens, &subject).is_none());
        let mut missing = unit.ir_module.clone();
        missing.source_entry.execution_name_policy = None;
        assert_eq!(
            missing.original_invocation_source_purpose(registry, metadata),
            None
        );
        missing.source_entry.unknown_entry = true;
        missing.source_entry.execution_name_policy = Some(policy);
        assert_eq!(
            missing.original_invocation_source_purpose(registry, metadata),
            None
        );
        missing.source_entry.unknown_entry = false;
        missing.source_entry.metadata_context =
            crate::registry_invocation::OwnedInvocationMetadataContext::for_source_input(None);
        assert_eq!(
            missing.original_invocation_source_purpose(registry, metadata),
            None
        );
    }
}
