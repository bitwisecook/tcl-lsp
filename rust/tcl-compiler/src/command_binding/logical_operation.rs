// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Quiet original operations on the independently selected Logical source model.

use super::{
    Arc, BindingKind, CommandBindingTransition, MayBinding, ModuleCommandBindings,
    SourceCommandTarget, SourceExecutionContext, SourceNamespaceKey, SourceOutcomes,
    StateTransition, apply_may_binding_transition, apply_namespace_transition,
    source_binding_projection,
};
use tcl_registry::{
    ArgRole, InvocationFacts, SemanticOperationId, StateTransitions,
    hooks::{AnalyserHookId, LoweringHookId},
    world_effect::{CallbackKinds, EffectAccessMode, InterpreterScope, WorldStateDomain},
};

struct LogicalOriginalInvocation {
    words: Vec<tcl_lexer::NativeWord>,
    arguments: Vec<String>,
    target: SourceCommandTarget,
    namespace: SourceNamespaceKey,
}

pub(super) fn transfer(
    words: &[crate::ir::WordExpr],
    target: &SourceCommandTarget,
    offset: u32,
    state: &mut ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<SourceOutcomes> {
    // naming.source.logical-original-operation-transfer
    // docs/design/analysis/name-resolution-proofs/logical-original-operation-transfer.md
    let original = LogicalOriginalInvocation::capture(words, target, offset, state, context)?;
    let input = super::logical_definition::logical_entry(state, context)?.clone();
    let actual = input.context_registry();
    let arguments = original
        .arguments
        .iter()
        .map(|value| tcl_registry::InvocationWord::Literal(value))
        .collect::<Vec<_>>();
    let resolution =
        tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
            actual.commands(),
            Some(actual.context()),
            tcl_registry::InvocationWords::structured(
                tcl_registry::InvocationWord::Literal(&original.target.command),
                &arguments,
            )
            .with_dialect(super::source_analysis_entry::source_input_dialect(&input)),
            context.realm,
        );
    let schema = resolution.resolved()?;
    let facts = schema.facts();
    if facts.arity_accepts_frozen_arguments() != Some(true) || facts.frame_effect.is_some() {
        return None;
    }
    if facts.operation == SemanticOperationId::StructuredLowering(LoweringHookId::Set) {
        scalar_store(&original, &facts, state, offset, context)?;
    } else if matches!(
        facts.analyser_hook,
        Some(AnalyserHookId::Rename | AnalyserHookId::InterpAlias)
    ) {
        command_transfer(
            &original,
            &facts,
            &schema.state_transitions(),
            state,
            offset,
            context,
        )?;
    } else {
        return None;
    }
    // A continuation of this authored model has no Native handler, command
    // publication, compiler, frame, argv or Normal completion certificate.
    Some(SourceOutcomes::normal(state))
}

impl LogicalOriginalInvocation {
    fn capture(
        words: &[crate::ir::WordExpr],
        target: &SourceCommandTarget,
        offset: u32,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<Self> {
        if !target.registry_backed
            || !matches!(target.kind, BindingKind::Builtin | BindingKind::Alias)
        {
            return None;
        }
        let original = super::logical_definition::original_static_invocation_words(
            words, offset, state, context,
        )?;
        let values = original
            .iter()
            .map(super::logical_definition::static_value)
            .collect::<Option<Vec<_>>>()?;
        let namespace = context.namespace_identity();
        let selected = source_binding_projection(state, values.first()?, &namespace);
        if selected.unknown
            || selected.may_be_absent
            || selected.targets.as_slice() != std::slice::from_ref(target)
        {
            return None;
        }
        let mut arguments = target
            .prepended
            .iter()
            .map(super::logical_definition::captured_static_value)
            .collect::<Option<Vec<_>>>()?;
        arguments.extend(values.into_iter().skip(1));
        Some(Self {
            words: original,
            arguments,
            target: target.clone(),
            namespace,
        })
    }

    fn is_current(
        &self,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> bool {
        let Some(origin) = state.current_source_origin.as_ref() else {
            return false;
        };
        self.namespace == context.namespace_identity()
            && self.words.iter().all(|word| {
                word.image() == origin.source_image() && word.config() == context.config
            })
            && self
                .words
                .first()
                .and_then(super::logical_definition::static_value)
                .is_some_and(|head| {
                    let selected = source_binding_projection(state, &head, &self.namespace);
                    !selected.unknown
                        && !selected.may_be_absent
                        && selected.targets.as_slice() == std::slice::from_ref(&self.target)
                })
    }
}

fn bounded_effects(facts: &InvocationFacts, domains: &[WorldStateDomain]) -> bool {
    matches!(
        facts.effects.callback().kinds,
        CallbackKinds::NONE | CallbackKinds::TRACE
    ) && facts.effects.accesses().iter().all(|access| {
        access.interpreter == InterpreterScope::Current
            && access.mode != EffectAccessMode::Clobber
            && (domains.contains(&access.domain)
                || (access.domain == WorldStateDomain::InterpreterPolicy
                    && access.mode == EffectAccessMode::Read))
    })
}

fn scalar_store(
    original: &LogicalOriginalInvocation,
    facts: &InvocationFacts,
    state: &mut ModuleCommandBindings,
    offset: u32,
    context: SourceExecutionContext<'_>,
) -> Option<()> {
    let [name, value] = original.arguments.as_slice() else {
        return None;
    };
    if !original.is_current(state, context)
        || !facts.arg_roles_complete
        || facts.argument_offset != 0
        || facts.arg_roles.as_slice() != [(0, ArgRole::VarWrite)]
        || !bounded_effects(
            facts,
            &[
                WorldStateDomain::VariableStore,
                WorldStateDomain::VariableTraces,
            ],
        )
        || state.source_variables.dynamic_bindings
        || state.source_variables.dynamic_traces
    {
        return None;
    }
    let access = crate::var_resolve::resolve_literal_access(
        name,
        &state.source_variables,
        false,
        context.registry,
        tcl_registry::TraceOperation::Write,
    );
    if access.dynamic
        || access.observed
        || access.kind != crate::place::PlaceKind::Scalar
        || access.index.is_some()
        || !logical_cell_is_local(&access, state, context)
        || state.source_variables.root_contents_kind(&access)
            == Some(crate::var_resolve::RootContentsKind::Array)
        || state.source_variables.store_would_error(&access)
    {
        return None;
    }
    let variables = Arc::make_mut(&mut state.source_variables);
    variables.record_contents_write(&access, offset, false);
    variables.define_literal(name, value, context.registry);
    Some(())
}

/// Check an authored continuation's cell coordinate, independently of the
/// storage-domain facet. This never selects a physical namespace or store.
fn logical_cell_is_local(
    access: &crate::place::Place,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> bool {
    use crate::place::CellOwner;
    use tcl_registry::f5::VariableStorageDomain;
    if super::logical_definition::logical_entry(state, context).is_none() {
        return false;
    }
    let variables = &state.source_variables;
    let namespace = context.namespace_identity();
    let Some(cell) = access.cell.as_ref() else {
        return false;
    };
    if !matches!(namespace, SourceNamespaceKey::Authored(_))
        || variables.namespace_identity.as_ref() != Some(&namespace)
        || variables.interpreter.is_some()
        || variables.execution.is_some()
        || variables.hosted_execution_context.is_some()
        || variables.execution_name_policy.is_some()
        || cell.interpreter.is_some()
        || cell.execution.is_some()
        || cell.generation == crate::place::CellGeneration::Unknown
        || cell.generation
            != variables
                .generations
                .get(&crate::var_resolve::cell_key(access))
                .copied()
                .unwrap_or_default()
    {
        return false;
    }
    match &cell.owner {
        CellOwner::Namespace(_) | CellOwner::NamespaceIdentity(_) => {
            variables.namespace_footprint(access) == Some(namespace)
                && matches!(
                    cell.storage_domain,
                    None | Some(VariableStorageDomain::InterpreterNamespace)
                )
        }
        CellOwner::Activation(activation) => {
            variables.activation.as_ref() == Some(activation) && cell.storage_domain.is_none()
        }
        _ => false,
    }
}

fn command_transfer(
    original: &LogicalOriginalInvocation,
    facts: &InvocationFacts,
    transitions: &StateTransitions,
    state: &mut ModuleCommandBindings,
    offset: u32,
    context: SourceExecutionContext<'_>,
) -> Option<()> {
    if !original.is_current(state, context)
        || !bounded_effects(
            facts,
            &[
                WorldStateDomain::CommandBindings,
                WorldStateDomain::CommandTraces,
                WorldStateDomain::NamespaceLookup,
            ],
        )
        || transitions.facts().is_empty()
    {
        return None;
    }
    let namespace =
        crate::ir_helpers::ExecutionNamespace::SourceContext(original.namespace.clone());
    let mut candidate = state.clone();
    for fact in transitions.facts() {
        // A quiet authored continuation interprets the selected transition
        // conditionally. Native abrupt-edge commit classification remains on
        // the descriptor; it is neither changed nor a Normal certificate.
        match &fact.transition {
            StateTransition::Namespace(
                transition @ tcl_registry::NamespaceTransition::Ensure { .. },
            ) => {
                apply_namespace_transition(&mut candidate, transition, &namespace, offset, None);
            }
            StateTransition::CommandBinding(transition) => {
                admissible_transition(&candidate, transition, &original.namespace)?;
                apply_may_binding_transition(
                    &mut candidate,
                    transition,
                    &namespace,
                    None,
                    offset,
                    None,
                    None,
                );
                if let CommandBindingTransition::Alias { alias, .. } = transition {
                    let selected = source_binding_projection(
                        &candidate,
                        alias.literal()?,
                        &SourceNamespaceKey::authored("::"),
                    );
                    if selected.unknown {
                        return None;
                    }
                }
            }
            _ => return None,
        }
    }
    if candidate.has_opaque_domain() {
        return None;
    }
    *state = candidate;
    Some(())
}

fn admissible_transition(
    state: &ModuleCommandBindings,
    transition: &CommandBindingTransition,
    namespace: &SourceNamespaceKey,
) -> Option<()> {
    match transition {
        CommandBindingTransition::Move { from, to } => {
            known_source(state, from.literal()?, namespace)?;
            let key = state.publication_key_at(
                namespace,
                to.literal()?,
                super::namespace_slots::PublicationPurpose::Rename,
            )?;
            if !state.namespaces.contains(key.holder().as_ref())
                || state.binding_alternatives(&key)
                    != std::collections::BTreeSet::from([MayBinding::Missing])
            {
                return None;
            }
        }
        CommandBindingTransition::Delete { interpreter, name } => {
            let namespace = match interpreter {
                None => namespace.clone(),
                Some(path) if path.literal() == Some("") => SourceNamespaceKey::authored("::"),
                Some(_) => return None,
            };
            known_source(state, name.literal()?, &namespace)?;
        }
        CommandBindingTransition::Alias {
            source_interpreter,
            alias,
            target_interpreter,
            target,
            arguments,
            ..
        } => {
            if source_interpreter.literal() != Some("")
                || target_interpreter.literal() != Some("")
                || target.literal()?.is_empty()
                || arguments.iter().any(|value| value.literal().is_none())
            {
                return None;
            }
            let key = state.publication_key_at(
                &SourceNamespaceKey::authored("::"),
                alias.literal()?,
                super::namespace_slots::PublicationPurpose::Alias,
            )?;
            if key.authored_spelling()?.is_empty()
                || !state.namespaces.contains(key.holder().as_ref())
            {
                return None;
            }
        }
        CommandBindingTransition::Define { .. } | CommandBindingTransition::Unknown { .. } => {
            return None;
        }
    }
    Some(())
}

fn known_source(
    state: &ModuleCommandBindings,
    name: &str,
    namespace: &SourceNamespaceKey,
) -> Option<()> {
    let keys = state.source_keys_checked(name, namespace).ok()?;
    let [key] = keys.as_slice() else {
        return None;
    };
    let alternatives = state.binding_alternatives(key);
    if alternatives.len() != 1 || !matches!(alternatives.first(), Some(MayBinding::Target(_))) {
        return None;
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::{Analyser, ResolvedAnalysisInput};
    use crate::command_binding::{
        SourceAnalysisOptions, SourceCommandSlotPresence, SourceInvocationBinding,
    };

    fn input() -> ResolvedAnalysisInput {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        )
    }

    fn final_binding(source: &str, input: &ResolvedAnalysisInput) -> SourceInvocationBinding {
        let analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, input.analyser_profile().name);
        let command = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            input.lexer_config(),
        )
        .pop()
        .unwrap();
        let binding = analysis
            .retained_command_realm()
            .unwrap()
            .invocation_at_source("", command.span.start());
        assert_eq!(binding.logical_source_name_advice_input(), Some(input));
        assert!(binding.original_recorded_head_name_input().is_none());
        binding
    }

    #[test]
    fn logical_namespace_cell_continuation_keeps_the_independent_storage_facet() {
        // naming.source.logical-original-operation-transfer
        // docs/design/analysis/name-resolution-proofs/logical-original-operation-transfer.md
        let input = input();
        let registry = input.borrowed_context_registry().commands();
        let options = SourceAnalysisOptions::for_logical_source(&input).unwrap();
        let binding = final_binding("set", &input);
        let state = binding.lookup_state.as_ref().unwrap().state.clone();
        let namespace = state.source_variables.namespace_identity.clone().unwrap();
        let frame = state.variable_frame.clone();
        let context = super::super::root_source_execution_context(
            &frame,
            &state.source_variables.namespace,
            &namespace,
            input.lexer_config(),
            registry,
            options,
        );
        let access = crate::var_resolve::resolve_literal_access(
            "suffix",
            &state.source_variables,
            false,
            registry,
            tcl_registry::TraceOperation::Write,
        );
        let storage = Some(tcl_registry::f5::VariableStorageDomain::InterpreterNamespace);
        assert_eq!(access.cell.as_ref().unwrap().storage_domain, storage);
        assert!(logical_cell_is_local(&access, &state, context));
        assert_eq!(access.cell.as_ref().unwrap().storage_domain, storage);
        let mut worker = access.clone();
        worker.cell.as_mut().unwrap().storage_domain =
            Some(tcl_registry::f5::VariableStorageDomain::WorkerNamespace);
        assert!(!logical_cell_is_local(&worker, &state, context));
        let mut foreign = access.clone();
        foreign.cell.as_mut().unwrap().owner = crate::place::CellOwner::Namespace("::other".into());
        assert!(!logical_cell_is_local(&foreign, &state, context));
        // A same-display Native coordinate supplies no authored correspondence.
        let native = SourceNamespaceKey::Native(
            tcl_runtime_api::native_compilation::NativeNamespaceContext {
                interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity {
                    owner: 37,
                    interpreter: 13,
                },
                token: 99,
                path: tcl_core_types::ByteNamespacePath::root(),
            },
        );
        assert_eq!(native.display(), namespace.display());
        let mut native_access = access.clone();
        native_access.cell.as_mut().unwrap().owner =
            crate::place::CellOwner::NamespaceIdentity(Box::new(native));
        assert!(!logical_cell_is_local(&native_access, &state, context));
        let mut recreated = access.clone();
        recreated.cell.as_mut().unwrap().generation = crate::place::CellGeneration::After(17);
        assert!(!logical_cell_is_local(&recreated, &state, context));
        let mut physical = access.clone();
        physical.cell.as_mut().unwrap().interpreter = Some("retained interpreter".into());
        assert!(!logical_cell_is_local(&physical, &state, context));
        let mut missing = state.clone();
        super::super::Arc::make_mut(&mut missing.baseline).logical_source_input = None;
        assert!(!logical_cell_is_local(&access, &missing, context));
        let mut config = input.lexer_config();
        config.strict_quoting = !config.strict_quoting;
        assert!(!logical_cell_is_local(
            &access,
            &state,
            SourceExecutionContext { config, ..context }
        ));
    }

    #[test]
    fn logical_scalar_continuation_keeps_original_values_and_captured_set_operands() {
        // naming.source.logical-original-operation-transfer
        // docs/design/analysis/name-resolution-proofs/logical-original-operation-transfer.md
        let input = input();
        for source in [
            "proc foo_hi {} {}; rename foo_hi {}; set suffix _hi; foo$suffix",
            "proc foo_hi {} {}; rename foo_hi {}; interp alias {} write {} set suffix; write _hi; foo$suffix",
            "proc foo_é {} {}; rename foo_é {}; interp alias {} ::écrire {} set suffix; ::écrire _é; foo$suffix",
        ] {
            let binding = final_binding(source, &input);
            assert_eq!(
                binding.selected_slot_diagnostic_presence(),
                SourceCommandSlotPresence::Absent,
                "{source}"
            );
            assert!(binding.variable_context.execution_name_policy.is_none());
            assert!(binding.variable_context.interpreter.is_none());
            assert!(binding.variable_context.execution.is_none());
        }
        for source in [
            "proc foo_hi {} {}; rename foo_hi {}; proc set args {}; set suffix _hi; foo$suffix",
            "proc foo_hi {} {}; rename foo_hi {}; set suffix $unknown; foo$suffix",
            "proc foo_hi {} {}; rename foo_hi {}; mystery; set suffix _hi; foo$suffix",
        ] {
            assert_eq!(
                final_binding(source, &input).selected_slot_diagnostic_presence(),
                SourceCommandSlotPresence::Unknown,
                "{source}"
            );
        }
    }

    #[test]
    fn logical_original_transfers_preserve_unicode_registry_moves_and_aliases() {
        // naming.source.logical-original-operation-transfer
        // docs/design/analysis/name-resolution-proofs/logical-original-operation-transfer.md
        // These are conditional authored continuations, not Native rename or
        // alias execution, abrupt-edge exclusion or successful allocation.
        let input = input();
        for source in [
            "rename proc ::α; ::α subject {} {}; subject",
            "interp alias {} ::α {} proc; ::α subject {} {}; subject",
            "rename proc ::α; interp alias {} ::β {} ::α; ::β subject {} {}; subject",
        ] {
            let binding = final_binding(source, &input);
            assert_eq!(
                binding.selected_slot_diagnostic_presence(),
                SourceCommandSlotPresence::Present,
                "{source}"
            );
            assert!(binding.variable_context.execution_name_policy.is_none());
        }
    }

    #[test]
    fn logical_original_transfers_preserve_moves_deletes_and_scalar_head_values() {
        // naming.source.logical-original-operation-transfer
        // docs/design/analysis/name-resolution-proofs/logical-original-operation-transfer.md
        let input = input();
        for (source, presence) in [
            (
                "proc original {} {}; rename original moved; moved",
                SourceCommandSlotPresence::Present,
            ),
            (
                "proc original {} {}; rename original moved; original",
                SourceCommandSlotPresence::Absent,
            ),
            (
                "proc original {} {}; rename original {}; original",
                SourceCommandSlotPresence::Absent,
            ),
            (
                "proc original {} {}; rename original ::fresh::nested::moved; ::fresh::nested::moved",
                SourceCommandSlotPresence::Present,
            ),
            (
                "proc foo_hi {} {}; rename foo_hi {}; set suffix _hi; foo$suffix",
                SourceCommandSlotPresence::Absent,
            ),
            (
                "proc foo_hi {} {}; rename foo_hi {}; set suffix _hi; proc foo_hi {} {}; foo$suffix",
                SourceCommandSlotPresence::Present,
            ),
        ] {
            assert_eq!(
                final_binding(source, &input).selected_slot_diagnostic_presence(),
                presence,
                "{source}"
            );
        }
        let source = "proc foo_hi {} {}; rename foo_hi {}; set suffix _hi; foo$suffix";
        let analysis = Analyser::new()
            .with_resolved_input(input)
            .analyse(source, "tcl");
        assert_eq!(
            analysis
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::W123)
                .count(),
            1
        );
    }

    #[test]
    fn logical_original_alias_capture_composes_registry_operations_at_the_call() {
        // naming.source.logical-original-operation-transfer
        // docs/design/analysis/name-resolution-proofs/logical-original-operation-transfer.md
        let input = input();
        for source in [
            "proc victim {} {}; interp alias {} drop {} rename victim; drop {}; victim",
            "proc foo_hi {} {}; rename foo_hi {}; interp alias {} write {} set suffix; write _hi; foo$suffix",
            "proc victim {} {}; interp alias {} inner {} rename; interp alias {} outer {} inner victim; outer {}; victim",
        ] {
            assert_eq!(
                final_binding(source, &input).selected_slot_diagnostic_presence(),
                SourceCommandSlotPresence::Absent,
                "{source}"
            );
        }
        let source = "interp alias {} ensure {} package require; ensure Tcl";
        let binding = final_binding(source, &input);
        assert_eq!(binding.targets.len(), 1);
        assert!(binding.targets[0].registry_backed);
        assert_eq!(
            binding.targets[0].prepended,
            vec![crate::registry_invocation::EffectiveInvocationWord::Literal("require".into())]
        );
        assert!(binding.original_recorded_head_name_input().is_none());
    }

    #[test]
    fn logical_original_operation_refusals_do_not_recover_from_reporting_state() {
        // naming.source.logical-original-operation-transfer
        // docs/design/analysis/name-resolution-proofs/logical-original-operation-transfer.md
        let input = input();
        for source in [
            "proc victim {} {}; mystery; rename victim {}; victim",
            "proc victim {} {}; rename $which {}; victim",
            "proc victim {} {}; proc rename args {}; rename victim {}; victim",
            "proc victim {} {}; interp alias {} drop foreign rename victim; drop {}; victim",
            "proc victim {} {}; interp alias {} drop {} rename victim; proc rename args {}; drop {}; victim",
            "proc victim {} {}; proc occupied {} {}; rename victim occupied; victim",
            "proc foo_hi {} {}; rename foo_hi {}; set suffix(index) _hi; foo$suffix",
            "proc foo_hi {} {}; rename foo_hi {}; set suffix $unknown; foo$suffix",
            "proc victim {} {}; trace add command victim delete observer; rename victim {}; victim",
            "proc foo_hi {} {}; rename foo_hi {}; trace add variable suffix write observer; set suffix _hi; foo$suffix",
            "interp alias {} cycle {} cycle; proc victim {} {}; victim",
        ] {
            assert_ne!(
                final_binding(source, &input).selected_slot_diagnostic_presence(),
                SourceCommandSlotPresence::Absent,
                "{source}"
            );
        }
    }

    fn with_store_fixture(
        test: impl FnOnce(
            &crate::ir::CommandTokens,
            &SourceCommandTarget,
            ModuleCommandBindings,
            SourceExecutionContext<'_>,
        ),
    ) {
        let input = input();
        let actual = input.context_registry();
        let registry = actual.commands();
        let options = SourceAnalysisOptions {
            logical_source_input: Some(&input),
            invocation_dialect: Some(super::super::source_analysis_entry::source_input_dialect(
                &input,
            )),
            ..SourceAnalysisOptions::default()
        };
        let image = tcl_lexer::SourceImage::document("set suffix _hi");
        let mut state = ModuleCommandBindings::initial_with_options(
            registry,
            options,
            Some(input.lexer_config()),
        );
        state.current_source_origin = Some(Arc::new(super::super::SourceOriginId::authored_image(
            image.clone(),
        )));
        let namespace = SourceNamespaceKey::authored("::");
        let frame = crate::var_resolve::VariableExecutionFrame::Global;
        let context = super::super::root_source_execution_context(
            &frame,
            "::",
            &namespace,
            input.lexer_config(),
            registry,
            options,
        );
        let command = crate::segmenter::segment_commands_image_with_offset_and_config(
            &image,
            0,
            context.config,
        )
        .unwrap()
        .remove(0);
        let tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::from_image(&image),
            context.config,
            &command,
        );
        let selected = source_binding_projection(&state, "set", &namespace);
        test(&tokens, &selected.targets[0], state, context);
    }

    #[test]
    fn logical_original_store_transfer_retains_source_purpose_without_native_normal() {
        // naming.source.logical-original-operation-transfer
        // docs/design/analysis/name-resolution-proofs/logical-original-operation-transfer.md
        with_store_fixture(|tokens, target, state, context| {
            let outcomes =
                transfer(tokens.words(), target, 0, &mut state.clone(), context).unwrap();
            assert!(outcomes.normal_completion.is_none());
            assert!(outcomes.native_normal_completion.is_none());
            let normal = outcomes.normal.unwrap();
            assert_eq!(normal.original_command_world, state.original_command_world);
            assert_eq!(
                normal
                    .source_variables
                    .literal_value("suffix", context.registry),
                Some("_hi")
            );
        });
    }

    #[test]
    fn logical_original_store_transfer_refuses_observers_foreign_and_missing_owners() {
        // naming.source.logical-original-operation-transfer
        // docs/design/analysis/name-resolution-proofs/logical-original-operation-transfer.md
        with_store_fixture(|tokens, target, state, context| {
            for dynamic in [false, true] {
                let mut observed = state.clone();
                Arc::make_mut(&mut observed.source_variables).dynamic_traces = dynamic;
                Arc::make_mut(&mut observed.source_variables).dynamic_bindings = !dynamic;
                assert!(transfer(tokens.words(), target, 0, &mut observed, context).is_none());
            }
            let mut foreign = state.clone();
            foreign.current_source_origin =
                Some(Arc::new(super::super::SourceOriginId::authored_image(
                    tcl_lexer::SourceImage::document("set foreign _hi"),
                )));
            assert!(transfer(tokens.words(), target, 0, &mut foreign, context).is_none());
            let mut config = context.config;
            config.strict_quoting = !config.strict_quoting;
            assert!(
                transfer(
                    tokens.words(),
                    target,
                    0,
                    &mut state.clone(),
                    SourceExecutionContext { config, ..context }
                )
                .is_none()
            );
            let mut missing = state.clone();
            Arc::make_mut(&mut missing.baseline).logical_source_input = None;
            assert!(transfer(tokens.words(), target, 0, &mut missing, context).is_none());
            let mut foreign_cell = state.clone();
            let mut selected_cell = crate::var_resolve::resolve_literal_access(
                "suffix",
                &state.source_variables,
                false,
                context.registry,
                tcl_registry::TraceOperation::Write,
            );
            selected_cell.cell.as_mut().unwrap().owner =
                crate::place::CellOwner::Namespace("::other".into());
            Arc::make_mut(&mut foreign_cell.source_variables)
                .alias_bindings
                .insert("suffix", selected_cell);
            assert!(transfer(tokens.words(), target, 0, &mut foreign_cell, context).is_none());
            let mut foreign_frame = state.clone();
            foreign_frame.variable_frame = crate::var_resolve::VariableExecutionFrame::Procedure {
                namespace: "::".into(),
                identity: "other-activation".into(),
            };
            assert!(transfer(tokens.words(), target, 0, &mut foreign_frame, context).is_none());
            for dialect in [
                "tcl8.4",
                "tcl8.5",
                "tcl8.6",
                "tcl9.0",
                "tcl9.1",
                "jim",
                "f5-irules",
            ] {
                let ingress = tcl_registry::model::ingress::resolve_environment(dialect);
                let profile = ingress.analyser_profile();
                let native = ResolvedAnalysisInput::new(
                    profile,
                    profile,
                    ingress.default_context_registry(),
                    tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
                );
                let options = SourceAnalysisOptions {
                    logical_source_input: Some(&native),
                    invocation_dialect: Some(
                        super::super::source_analysis_entry::source_input_dialect(&native),
                    ),
                    ..SourceAnalysisOptions::default()
                };
                let mut outside = ModuleCommandBindings::initial_with_options(
                    context.registry,
                    options,
                    Some(native.lexer_config()),
                );
                outside.current_source_origin = state.current_source_origin.clone();
                assert!(
                    transfer(tokens.words(), target, 0, &mut outside, context).is_none(),
                    "{dialect}"
                );
            }
        });
    }
}
