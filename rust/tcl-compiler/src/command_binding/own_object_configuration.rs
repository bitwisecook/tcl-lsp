// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Represented stock own-object method registration at its actual allocation.

use super::{
    Arc, CommandAllocationSite, ModuleCommandBindings, SourceCommandBindings,
    SourceExecutionContext, SourceNativeInvocation, SourceObjectInstanceProof, SourceOutcomes,
};
use tcl_registry::{
    InvocationFacts, ObjectDispatchLayer, ObjectDispatchTransition, StateTransition,
};

/// One complete selected registration operation. This retains original
/// producers and implementation dependencies; it grants no physical object,
/// compiler hook, generated namespace or method-body execution capability.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct OriginalOwnObjectMethodConfiguration {
    receiver: Arc<SourceObjectInstanceProof>,
    site: CommandAllocationSite,
    words: Vec<crate::ir::WordExpr>,
    config: tcl_lexer::LexerConfig,
    target_input: Option<crate::signature_scan::scope::SignatureSourceNameInput>,
    handler: super::SourceCommandTarget,
    body: super::ExecutedScriptSource,
    transfers: Vec<OriginalOwnObjectMethodTransfer>,
}

/// Complete handler transfer at one authentic original worker invocation.
/// Native syntax, selected worker and stored source body are independent facets.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct OriginalOwnObjectMethodTransfer {
    site: CommandAllocationSite,
    words: Vec<crate::ir::WordExpr>,
    worker: super::SourceCommandTarget,
    recipe: tcl_registry::definer::NativeDeferredMethodSetter,
    entry: super::SourceReceiverMethodEntry,
}

impl OriginalOwnObjectMethodConfiguration {
    pub(super) fn receiver(&self) -> &SourceObjectInstanceProof {
        &self.receiver
    }
}

impl SourceCommandBindings {
    pub(super) fn walk_closed_own_object_method_configuration(
        &mut self,
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceOutcomes> {
        if state.has_opaque_domain()
            || state.source_step_observed()
            || state.source_execution_observed(None)
            || !native.target.registry_backed
            || !native.target.prepended.is_empty()
            || native.target.implementation_generation != 0
            || native.invocation.arguments().exact_argv_len() != Some(2)
            || !facts.arg_roles.contains(&(1, tcl_registry::ArgRole::Body))
        {
            return None;
        }
        let target_index = own_object_configuration_target(facts)?;
        let target_input = native
            .original_variable_operands
            .input(target_index, &state.source_variables)
            .cloned();
        let receiver = context
            .written_objects
            .and_then(|values| values.get(target_index + 1))
            .and_then(Option::as_ref)
            .cloned()
            .or_else(|| {
                state
                    .original_named_instance_for_operand(
                        target_input.as_ref()?,
                        &context.namespace_identity(),
                    )
                    .map(|proof| Arc::new(proof.clone()))
            })?;
        if !state.receiver_allocation_is_current(&receiver) {
            return None;
        }
        let class = state
            .class_definitions
            .get(receiver.class_target().identity.as_ref()?)?;
        let dialect = state.baseline.dialect?;
        let grammar = context.registry.native_default_construction_grammar(
            &class.factory,
            dialect,
            context.realm,
        )?;
        let selected = context
            .registry
            .get_for_surface(
                &native.target.command,
                dialect
                    .authoring_query()
                    .map(|query| query.with_realm(context.realm)),
            )?
            .definition_body?;
        if selected.family != grammar.family
            || !state.default_construction_dependencies_hold(grammar)
        {
            return None;
        }
        let grammar = selected;
        let parameters = dialect.parameter_grammar()?;
        let policy = state.baseline.execution_name_policy?.native_recipe()?;
        let body = super::retained_script_operand(
            1,
            native.script_operands(),
            state,
            native.segment.span.start(),
            context.config,
        )?;
        let transfers = own_object_method_transfers(
            grammar,
            &body,
            &receiver,
            dialect,
            super::deferred_method::MethodParameterSource { parameters, policy },
            state,
            &context,
        )?;
        let origin = state.current_source_origin.as_ref()?;
        let configuration = OriginalOwnObjectMethodConfiguration {
            receiver: Arc::clone(&receiver),
            site: CommandAllocationSite {
                source: Arc::clone(origin),
                offset: native.segment.span.start(),
            },
            words: native.words.to_vec(),
            config: context.config,
            target_input,
            handler: native.target.clone(),
            body: body.clone(),
            transfers: transfers.clone(),
        };
        let mut outcomes =
            install_own_object_method_transfers(&configuration, transfers, receiver, state)?;
        self.register_original_method_bodies(grammar, &body, Some(dialect), parameters, context);
        outcomes.retain_closed_native_handler_completion(native, facts, context);
        outcomes.publish(state);
        Some(outcomes)
    }
}

fn own_object_configuration_target(facts: &InvocationFacts) -> Option<usize> {
    let transitions = facts.state_transitions.declared()?;
    let mut targets = transitions
        .facts()
        .iter()
        .filter_map(|fact| match &fact.transition {
            StateTransition::ObjectDispatch(ObjectDispatchTransition::Configure {
                target,
                layer: ObjectDispatchLayer::Object,
            }) => Some(target),
            _ => None,
        });
    let target = targets.next()?;
    if targets.next().is_some() {
        return None;
    }
    if transitions
        .facts()
        .iter()
        .any(|fact| match &fact.transition {
            StateTransition::ObjectDispatch(ObjectDispatchTransition::Configure {
                target: candidate,
                layer: ObjectDispatchLayer::Object,
            }) => candidate != target,
            StateTransition::Widen(widening) => {
                widening.subject != *target
                    || widening.domains.iter().any(|domain| {
                        *domain != tcl_registry::StateTransitionDomain::ObjectDispatch
                    })
            }
            _ => true,
        })
    {
        return None;
    }
    target.argument_index()
}

fn own_object_method_transfers(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    body: &super::ExecutedScriptSource,
    receiver: &SourceObjectInstanceProof,
    dialect: tcl_registry::InvocationDialect,
    parameters: super::deferred_method::MethodParameterSource,
    state: &ModuleCommandBindings,
    context: &SourceExecutionContext<'_>,
) -> Option<Vec<OriginalOwnObjectMethodTransfer>> {
    let context = *context;
    let map = tcl_lexer::SourceMap::from_image(&body.text).with_base(body.base(), 0, 0);
    let commands = crate::segmenter::segment_commands_image_with_offset_and_config(
        &body.text,
        body.base(),
        context.config,
    )?;
    let mut transfers = Vec::new();
    for command in &commands {
        if command.is_partial {
            return None;
        }
        let words = crate::ir::CommandTokens::from_segmented(&map, context.config, command);
        let values = super::source_effective_words(words.words(), Some(dialect), None);
        let member = grammar.member(values.first()?.as_registry_word().literal()?)?;
        let query = dialect
            .authoring_query()
            .map(|query| query.with_realm(context.realm));
        let recipe = grammar.native_deferred_method_setter(
            member,
            tcl_registry::definer::DefinitionReceiver::Class,
            query,
        )?;
        let arguments = values
            .get(1..)?
            .iter()
            .map(crate::registry_invocation::EffectiveInvocationWord::literal_bytes)
            .collect::<Option<Vec<_>>>()?;
        if arguments.len() != recipe.argument_count()
            || recipe.recipe() != parameters.policy.recipe()
            || recipe
                .recipe()
                .oo_method_input(arguments[recipe.name_argument()])
                .is_err()
            || !super::formal_topology::native_formal_parameter_bytes_valid(
                arguments[recipe.parameters_argument()],
                recipe.recipe(),
            )
        {
            return None;
        }
        // A body is stored as a value; its future source execution is not
        // part of the setter. Its genuine original producer is retained.
        arguments.get(recipe.body_argument())?;
        let worker = super::deferred_method::original_definition_member_target(
            grammar,
            member,
            tcl_registry::definer::DefinitionReceiver::Class,
            state,
            context,
        )?;
        let entry = super::deferred_method::retained_method_entry(
            grammar,
            body,
            &map,
            command,
            Some(dialect),
            parameters,
            context,
        )?
        .with_own_object_provider(receiver);
        transfers.push(OriginalOwnObjectMethodTransfer {
            site: entry.declaration().clone(),
            words: words.words().to_vec(),
            worker,
            recipe,
            entry,
        });
    }
    Some(transfers)
}

fn install_own_object_method_transfers(
    configuration: &OriginalOwnObjectMethodConfiguration,
    transfers: Vec<OriginalOwnObjectMethodTransfer>,
    receiver: Arc<SourceObjectInstanceProof>,
    state: &mut ModuleCommandBindings,
) -> Option<SourceOutcomes> {
    // Compose actual selected intrinsic setter transfers. Every step has
    // an existing object target, a live unchanged worker and a complete
    // native-valid argument vector; no method body or callback executes.
    let mut outcomes = SourceOutcomes::normal(state);
    outcomes.retain_complete_normal_evaluation();
    let mut receiver = receiver;
    if transfers.is_empty() {
        Arc::make_mut(&mut state.object_instances).install_own_method_configuration(
            configuration.clone(),
            super::SourceReceiverMethodEntries::new(),
        )?;
        outcomes = SourceOutcomes::normal(state);
        outcomes.retain_complete_normal_evaluation();
    }
    for transfer in transfers {
        let mut continuing = outcomes.normal.take()?;
        if !continuing.retained_target_is_current(&transfer.worker)
            || !continuing.receiver_allocation_is_current(&receiver)
        {
            return None;
        }
        let mut step = configuration.clone();
        step.receiver = Arc::clone(&receiver);
        step.transfers = vec![transfer.clone()];
        let mut entries = super::SourceReceiverMethodEntries::new();
        entries.insert(
            (
                transfer.entry.receiver(),
                tcl_core_types::NameBytes::from(transfer.entry.original_name_input().bytes()),
            ),
            transfer.entry,
        );
        receiver = Arc::make_mut(&mut continuing.object_instances)
            .install_own_method_configuration(step, entries)?;
        let mut completed = SourceOutcomes::normal(&continuing);
        completed.retain_complete_normal_evaluation();
        outcomes.join(&completed);
    }
    Some(outcomes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyse(source: &str, dialect: &str) -> SourceCommandBindings {
        let profile = tcl_dialect::DialectProfile::find(dialect).unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
    }

    #[test]
    fn original_own_object_entries_keep_allocation_and_counted_names_separate() {
        // Implementation contract: naming.tcloo.original-own-object-method-transfer
        // docs/design/analysis/name-resolution-proofs/tcloo-original-own-object-method-transfer.md
        let source = r"oo::class create C {method shared {} {}}; C create d; C create e; oo::objdefine d {method shared {} {}; method p\uD800 {} {}; method p\uD801 {} {}}; d shared; d p\uD800; d p\uD801; e shared";
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let bindings = analyse(source, dialect);
            let mut declarations = Vec::new();
            for (selector, expected) in [
                ("shared", b"shared".as_slice()),
                (r"p\uD800", b"p\xed\xa0\x80".as_slice()),
                (r"p\uD801", b"p\xed\xa0\x81".as_slice()),
            ] {
                let offset =
                    u32::try_from(source.rfind(&format!("d {selector}")).unwrap()).unwrap();
                let binding = bindings.invocation_at_source("d", offset);
                let proof = binding.named_object_instance_at_dispatch().expect(dialect);
                let (_, entry, _) = binding.named_object_receiver_method_entry().expect(dialect);
                assert_eq!(
                    entry.declaring_object(),
                    Some(proof.allocation()),
                    "{dialect}: {selector}"
                );
                assert!(entry.declaring_class().is_none());
                assert_eq!(entry.original_name_input().bytes(), expected);
                assert_eq!(
                    binding
                        .own_object_method_entries_at_dispatch()
                        .unwrap()
                        .1
                        .len(),
                    3
                );
                declarations.push(entry.declaration().clone());
            }
            assert_ne!(declarations[1], declarations[2]);
            let offset = u32::try_from(source.rfind("e shared").unwrap()).unwrap();
            let binding = bindings.invocation_at_source("e", offset);
            let (_, entry, _) = binding.named_object_receiver_method_entry().expect(dialect);
            assert!(entry.declaring_object().is_none());
            assert!(entry.declaring_class().is_some());
        }
    }

    #[test]
    fn original_own_object_readonly_target_retains_no_generated_name() {
        // Implementation contract: naming.tcloo.original-own-object-method-transfer
        // docs/design/analysis/name-resolution-proofs/tcloo-original-own-object-method-transfer.md
        let source = r"oo::class create C {}; set object [C new]; oo::objdefine $object {method own\uD800 {} {}}; $object own\uD800";
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let bindings = analyse(source, dialect);
            let offset = u32::try_from(source.rfind("$object own").unwrap()).unwrap();
            let binding = bindings.invocation_at_source("", offset);
            let (proof, entries) = binding
                .own_object_method_entries_at_dispatch()
                .expect(dialect);
            let entry = entries.values().next().unwrap();
            assert_eq!(entry.declaring_object(), Some(proof.allocation()));
            assert_eq!(entry.original_name_input().bytes(), b"own\xed\xa0\x80");
            let state = &binding.lookup_state.as_ref().unwrap().state;
            let table = state
                .object_instances
                .own_methods
                .iter()
                .find(|table| &table.allocation == proof.allocation())
                .unwrap();
            assert!(table.configurations.first().unwrap().target_input.is_none());
        }
    }

    #[test]
    fn original_own_object_unknown_or_changed_workers_withdraw_entries() {
        // Implementation contract: naming.tcloo.original-own-object-method-transfer
        // docs/design/analysis/name-resolution-proofs/tcloo-original-own-object-method-transfer.md
        for configuration in [
            "unrepresentedOperation",
            "class Other",
            "mixin Other",
            "filter guard",
            "method own {a b c d} {unrepresentedBody}; unrepresentedOperation",
        ] {
            let source = format!(
                "oo::class create C {{}}; C create d; oo::objdefine d {{{configuration}}}; d own"
            );
            let bindings = analyse(&source, "tcl8.6");
            let binding = bindings
                .invocation_at_source("d", u32::try_from(source.rfind("d own").unwrap()).unwrap());
            assert!(
                binding.named_object_receiver_method_entry().is_none(),
                "{configuration}"
            );
            assert!(
                binding.own_object_method_entries_at_dispatch().is_none(),
                "{configuration}"
            );
        }
        let source = "oo::class create C {}; C create d; rename ::oo::objdefine::method ::oldWorker; proc ::oo::objdefine::method args {}; oo::objdefine d {method own {} {}}; d own";
        let bindings = analyse(source, "tcl8.6");
        let binding = bindings
            .invocation_at_source("d", u32::try_from(source.rfind("d own").unwrap()).unwrap());
        assert!(binding.named_object_receiver_method_entry().is_none());
    }

    #[test]
    fn original_own_object_body_does_not_borrow_class_instance_variables() {
        // Implementation contract: naming.tcloo.original-own-object-method-transfer
        // docs/design/analysis/name-resolution-proofs/tcloo-original-own-object-method-transfer.md
        let source = "oo::class create C {variable x; method base {} {}}; C create d; oo::objdefine d {method own {} {::info exists x}}; d own";
        let bindings = analyse(source, "tcl8.6");
        let binding = bindings.invocation_at_source(
            "::info",
            u32::try_from(source.find("::info exists x").unwrap()).unwrap(),
        );
        assert!(!binding.variable_context.instance_vars.contains("x"));
        assert_eq!(
            binding.variable_context.activation_contents_world,
            Some(crate::var_resolve::ContentsWorld::Unknown)
        );
    }

    #[test]
    fn original_own_object_replacement_retires_prior_captured_prefix() {
        // Implementation contract: naming.tcloo.original-own-object-method-transfer
        // docs/design/analysis/name-resolution-proofs/tcloo-original-own-object-method-transfer.md
        for (configuration, expected) in [("", 1), ("oo::objdefine d {method shared {} {}};", 0)] {
            let source = format!(
                "oo::class create C {{method shared {{}} {{}}}}; C create d; set callback [list d shared]; {configuration} after 0 $callback"
            );
            let bindings = analyse(&source, "tcl8.6");
            let binding = bindings.invocation_at_source(
                "after",
                u32::try_from(source.rfind("after 0").unwrap()).unwrap(),
            );
            assert_eq!(
                binding.captured_method_prefix_arguments().count(),
                expected,
                "{configuration}"
            );
        }
    }
}
