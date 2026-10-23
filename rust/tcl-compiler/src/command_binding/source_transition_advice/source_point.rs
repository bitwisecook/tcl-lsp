// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! A derived source graph at the same parent point as the entry issuer.

use super::{
    AdviceGraph, AdviceInvocationContext, AdviceNamingPolicy, OriginalSourceCataloguePurpose,
    OriginalSourceTransitionSeed, SourceAdviceQuery,
};
use crate::command_binding::{CommandAllocationSite, SourceNamespaceKey};
use std::cell::{Cell, RefCell};
use std::sync::{Arc, atomic::AtomicUsize};
use tcl_lexer::NativeWord;
use tcl_registry::model::ContextRegistry;

/// Derived source-only lookup seed, never a Native/runtime command table.
#[derive(Clone)]
pub(in crate::command_binding) struct OriginalSourcePointAdviceGraph {
    graph: AdviceGraph,
}
impl std::fmt::Debug for OriginalSourcePointAdviceGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OriginalSourcePointAdviceGraph")
    }
}
impl OriginalSourcePointAdviceGraph {
    pub(in crate::command_binding) fn owns_procedure_declaration(
        &self,
        declaration: &Arc<super::OriginalSourceCommandTransitionAdvice>,
    ) -> bool {
        self.graph.cells.values().any(|cell| {
            matches!(cell, super::AdviceCell::SourceProcedure(body)
                if Arc::ptr_eq(&body.declaration, declaration))
        })
    }

    /// Only a retained conditional frame/selected restoration may call this.
    /// Naming coordinates remain source advice, never an entered Native frame.
    pub(in crate::command_binding) fn in_conditional_namespace(
        &self,
        namespace: &SourceNamespaceKey,
    ) -> Option<Arc<Self>> {
        let path = namespace.source_artifact_path()?;
        if self.graph.native_baseline
            || !self.graph.namespaces.contains(&path)
            || self.graph.unknown_paths.contains(&path)
        {
            return None;
        }
        let mut graph = self.child_graph(&self.graph.policy)?;
        match &graph.policy {
            AdviceNamingPolicy::Native(policy)
                if matches!(
                    policy.recipe(),
                    tcl_syntax::naming::NativeNameProtocol::C(_)
                ) =>
            {
                graph.c_source_namespace = Some(path);
            }
            AdviceNamingPolicy::Logical(_) => graph.logical_namespace = Some(path),
            AdviceNamingPolicy::Native(_) => return None,
        }
        Some(Arc::new(Self { graph }))
    }

    pub(super) fn child_graph(&self, policy: &AdviceNamingPolicy) -> Option<AdviceGraph> {
        match (&self.graph.policy, policy) {
            (AdviceNamingPolicy::Native(a), AdviceNamingPolicy::Native(b)) if a == b => {}
            (AdviceNamingPolicy::Logical(a), AdviceNamingPolicy::Logical(b)) if a == b => {}
            _ => return None,
        }
        let mut graph = self.graph.clone();
        // Parent cells/path/imports retain their original producers. They do
        // not become child declarations or a borrowed containing script body.
        graph.procedure_declarations.clear();
        graph.class_declarations.clear();
        graph.procedure_body = None;
        if let Some(namespace) = graph.procedure_namespace.take() {
            graph.c_source_namespace = Some(namespace);
        }
        graph.procedure_path.clear();
        graph.future_procedure_source = false;
        graph.handles.clear();
        graph.class_handles.clear();
        graph.procedure_steps = Arc::new(AtomicUsize::new(0));
        graph.decoded_phase_steps = Arc::new(AtomicUsize::new(0));
        Some(graph)
    }
}

pub(super) struct SourcePointQuery<'a> {
    site: &'a CommandAllocationSite,
    original: &'a [NativeWord],
    path: usize,
    namespace: &'a SourceNamespaceKey,
    seen: Cell<bool>,
    refused: Cell<bool>,
    graph: RefCell<Option<OriginalSourcePointAdviceGraph>>,
}

impl AdviceInvocationContext<'_> {
    pub(super) fn observe_source_point(
        &self,
        graph: &AdviceGraph,
        original: &[NativeWord],
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) {
        let Some(SourceAdviceQuery::SourcePoint(query)) = self.query else {
            return;
        };
        if self.origin != &query.site.source || original != query.original {
            return;
        }
        if query.seen.replace(true) {
            query.refused.set(true);
            return;
        }
        let actual_namespace = graph
            .c_source_lookup_namespace()
            .or(graph.logical_namespace.as_ref());
        let expected_namespace = query.namespace.source_artifact_path();
        let load_paths = schema
            .state_transitions()
            .facts()
            .iter()
            .filter_map(|fact| match &fact.transition {
                tcl_registry::StateTransition::Package(
                    tcl_registry::model::binding::PackageTransition::SourceLoad { path },
                ) => path.argument_index(),
                _ => None,
            })
            .collect::<Vec<_>>();
        if graph.native_baseline
            || !graph.uncertain_operations.is_empty()
            || !graph.arity_debts.is_empty()
            || actual_namespace != expected_namespace.as_ref()
            || expected_namespace
                .as_ref()
                .is_none_or(|namespace| graph.unknown_paths.contains(namespace))
            || schema.semantics.body_interpreter != tcl_registry::BodyInterpreter::Current
            || load_paths.is_empty()
            || load_paths.iter().any(|path| *path != query.path)
        {
            query.refused.set(true);
            return;
        }
        *query.graph.borrow_mut() = Some(OriginalSourcePointAdviceGraph {
            graph: graph.clone(),
        });
    }
}

impl OriginalSourceTransitionSeed {
    pub(in crate::command_binding) fn source_point_graph(
        &self,
        context: &ContextRegistry,
        site: &CommandAllocationSite,
        original: &[NativeWord],
        path: usize,
        namespace: &SourceNamespaceKey,
    ) -> Option<Arc<OriginalSourcePointAdviceGraph>> {
        if self.origin != site.source || !self.matches_registry(context.commands()) {
            return None;
        }
        let query = SourcePointQuery {
            site,
            original,
            path,
            namespace,
            seen: Cell::new(false),
            refused: Cell::new(false),
            graph: RefCell::new(None),
        };
        self.prepare_for_query(
            context,
            OriginalSourceCataloguePurpose::CompletePrefix,
            Some(SourceAdviceQuery::SourcePoint(&query)),
        )?;
        (!query.refused.get()).then_some(())?;
        query.graph.into_inner().map(Arc::new)
    }
}

/// Separate genuine root-graph result, sealed before future-body surveys.
pub(super) struct NormalSourceOutcomeQuery<'a> {
    bindings: &'a crate::command_binding::SourceCommandBindings,
    pending: Cell<usize>,
    sealed: Cell<bool>,
    refused: Cell<bool>,
    graph: RefCell<Option<OriginalSourcePointAdviceGraph>>,
}

fn normal_source_variable_effects_bounded(facts: &tcl_registry::InvocationFacts) -> bool {
    use tcl_registry::world_effect::{
        CallbackKinds, EffectAccessMode, InterpreterScope, Reentrancy, WorldStateDomain,
    };
    if facts.arity_accepts_frozen_arguments() != Some(true)
        || !facts.arg_roles_complete
        || facts.argument_offset != 0
        || facts.arg_roles.as_slice() != [(0, tcl_registry::ArgRole::VarWrite)]
        || facts.frame_effect.is_some()
        || !matches!(
            facts.effects.callback().kinds,
            CallbackKinds::NONE | CallbackKinds::TRACE
        )
        || facts.effects.callback().reentrancy == Reentrancy::AnyInterpreter
        || !facts.effects.accesses().iter().any(|access| {
            access.domain == WorldStateDomain::VariableStore
                && access.mode == EffectAccessMode::Write
        })
        || facts.effects.accesses().iter().any(|access| {
            access.interpreter != InterpreterScope::Current
                || match access.domain {
                    WorldStateDomain::VariableStore => access.mode != EffectAccessMode::Write,
                    WorldStateDomain::VariableTraces | WorldStateDomain::InterpreterPolicy => {
                        access.mode != EffectAccessMode::Read
                    }
                    _ => true,
                }
        })
    {
        return false;
    }
    true
}

fn normal_source_variable_point_is_closed(
    point: &crate::command_binding::SourceBindingPoint,
    native: &[NativeWord],
    origin: &Arc<crate::command_binding::SourceOriginId>,
    operands: &crate::variable_bindings::OriginalVariableInvocation,
    registry: &tcl_registry::CommandRegistry,
    canonical: &str,
) -> bool {
    let state = &point.state;
    if point.declaration_preview
        || !point.original_arguments_complete_normally
        || point.observer_execution.observed()
        || point
            .original_written_words
            .as_ref()
            .is_none_or(|words| !words.matches_native_words(native))
        || state
            .current_source_origin
            .as_ref()
            .is_none_or(|current| !Arc::ptr_eq(current, origin))
        || state.has_opaque_domain()
        || state.baseline.unknown_entry
        || state.source_step_observed()
        || state.source_variables.dynamic_bindings
        || state.source_variables.dynamic_traces
    {
        return false;
    }
    let selected = crate::command_binding::SourceCommandBindings::query_dispatch_points(
        std::iter::once(point),
    );
    let [target] = selected.targets.as_slice() else {
        return false;
    };
    if selected.unknown
        || selected.may_be_absent
        || !target.registry_backed
        || !target.prepended.is_empty()
        || !tcl_registry::command_descriptor_labels_match(&target.command, canonical)
        || target
            .identity
            .as_ref()
            .is_none_or(|identity| state.source_execution_observed(Some(identity)))
    {
        return false;
    }
    let receiver = operands.access(
        0,
        &state.source_variables,
        registry,
        tcl_registry::TraceOperation::Write,
        false,
    );
    if receiver.dynamic
        || receiver.observed
        || receiver.kind != crate::place::PlaceKind::Scalar
        || receiver.index.is_some()
        || state
            .source_variables
            .original_normal_value_write(&receiver, registry)
            .is_none()
    {
        return false;
    }
    true
}

impl AdviceInvocationContext<'_> {
    /// Refine an otherwise unknown identity transition only for this genuine
    /// normal-outcome query. The existing quiet-write owner closes callbacks;
    /// descriptor callback-retention absence alone never does so.
    pub(super) fn normal_source_variable_effects_closed(
        &self,
        invocation: super::AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> bool {
        let Some(SourceAdviceQuery::NormalSourceOutcome(query)) = self.query else {
            return false;
        };
        let facts = schema.facts();
        if query.sealed.get()
            || !invocation.lineage.is_empty()
            || !normal_source_variable_effects_bounded(&facts)
            || !invocation.native.iter().all(|word| {
                tcl_syntax::word_rules::original_static_word_unicode_value(word).is_some()
            })
        {
            return false;
        }
        let Some(first) = invocation.native.first() else {
            return false;
        };
        let bindings = query.bindings;
        let site = CommandAllocationSite {
            source: Arc::clone(self.origin),
            offset: first.span().start(),
        };
        let Some(operands) = bindings
            .original_variable_invocations
            .get(&site)
            .and_then(Option::as_deref)
        else {
            return false;
        };
        let mut seen = false;
        for point in bindings.dispatch_points_at(site.offset) {
            seen = true;
            if !normal_source_variable_point_is_closed(
                point,
                invocation.native,
                self.origin,
                operands,
                self.context.commands(),
                &facts.canonical_command,
            ) {
                return false;
            }
        }
        seen
    }

    pub(super) fn begin_normal_source_invocation(&self) {
        let Some(SourceAdviceQuery::NormalSourceOutcome(query)) = self.query else {
            return;
        };
        if query.sealed.get() {
            return;
        }
        match query.pending.get().checked_add(1) {
            Some(pending) => query.pending.set(pending),
            None => query.refused.set(true),
        }
    }

    pub(super) fn observe_normal_source_operation(
        &self,
        _graph: &AdviceGraph,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) {
        let Some(SourceAdviceQuery::NormalSourceOutcome(query)) = self.query else {
            return;
        };
        if query.sealed.get() {
            return;
        }
        match query.pending.get().checked_sub(1) {
            Some(pending) => query.pending.set(pending),
            None => query.refused.set(true),
        }
        let facts = schema.facts();
        if schema.authored_source_invocation_completion()
            != tcl_registry::registry::InvocationCompletion::FallsThrough
            || facts.arity_accepts_frozen_arguments() != Some(true)
            || !facts.arg_roles_complete
            || facts.source_callback_retention()
                != tcl_registry::resolved_invocation::SourceCallbackRetention::Absent
        {
            query.refused.set(true);
        }
    }

    pub(super) fn finish_normal_source_graph(&self, graph: &AdviceGraph) {
        let Some(SourceAdviceQuery::NormalSourceOutcome(query)) = self.query else {
            return;
        };
        query.sealed.set(true);
        if query.refused.get()
            || query.pending.get() != 0
            || graph.native_baseline
            || !graph.uncertain_operations.is_empty()
            || !graph.arity_debts.is_empty()
        {
            query.refused.set(true);
            return;
        }
        *query.graph.borrow_mut() = Some(OriginalSourcePointAdviceGraph {
            graph: graph.clone(),
        });
    }
}

impl OriginalSourceTransitionSeed {
    pub(in crate::command_binding) fn normal_source_outcome_graph(
        &self,
        context: &ContextRegistry,
        bindings: &crate::command_binding::SourceCommandBindings,
    ) -> Option<Arc<OriginalSourcePointAdviceGraph>> {
        // Equal-source memo reuse has a distinct analysis attachment even when
        // the original source origin and baseline allocations are shared.
        if self
            .analysis_owner
            .as_ref()
            .zip(bindings.original_analysis_owner())
            .is_none_or(|(expected, actual)| !Arc::ptr_eq(expected, actual))
            || bindings
                .source_origin()
                .is_none_or(|origin| !Arc::ptr_eq(origin, &self.origin))
            || !Arc::ptr_eq(&bindings.final_state.baseline, &self.baseline)
            || !bindings.matches_original_source_image(self.origin.source_image(), self.config)
        {
            return None;
        }
        let query = NormalSourceOutcomeQuery {
            bindings,
            pending: Cell::new(0),
            sealed: Cell::new(false),
            refused: Cell::new(false),
            graph: RefCell::new(None),
        };
        let purpose = if self
            .baseline
            .source_point_entry
            .as_ref()
            .is_some_and(|entry| entry.conditional_region().is_some())
        {
            OriginalSourceCataloguePurpose::ConditionalRegion
        } else {
            OriginalSourceCataloguePurpose::CompleteProgram
        };
        self.prepare_for_query(
            context,
            purpose,
            Some(SourceAdviceQuery::NormalSourceOutcome(&query)),
        )?;
        (!query.refused.get() && query.sealed.get()).then_some(())?;
        query.graph.into_inner().map(Arc::new)
    }
}

#[cfg(test)]
mod quiet_variable_normal_graph_controls {
    use super::*;
    use crate::analyser::Analyser;

    fn has_normal_graph(source: &str, dialect: &str) -> bool {
        let analysis = Analyser::new().analyse(source, dialect);
        let input = analysis.resolved_input.as_ref().unwrap();
        let bindings = analysis
            .retained_command_realm()
            .unwrap()
            .source_bindings_ref();
        bindings
            .original_source_transition_seed()
            .unwrap()
            .normal_source_outcome_graph(input.borrowed_context_registry(), bindings)
            .is_some()
    }

    #[test]
    fn normal_source_outcome_requires_actual_quiet_variable_effects() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            assert!(
                has_normal_graph("set fresh VALUE", dialect),
                "{dialect}: genuine quiet store"
            );
            for source in [
                "proc rewrite args {rename victim {}; proc victim {} {return CHANGED}}; proc victim {} {return OLD}; trace add variable fresh write rewrite; set fresh VALUE",
                "proc set args {return SHADOW}; set fresh VALUE",
                "set name fresh; set $name VALUE",
                "array set fresh {}; set fresh VALUE",
            ] {
                assert!(!has_normal_graph(source, dialect), "{dialect}: {source}");
            }
        }
    }

    #[test]
    fn normal_source_outcome_refuses_foreign_equal_source_and_changed_config() {
        let source = "set fresh VALUE";
        let left = Analyser::new().analyse(source, "tcl8.6");
        let foreign = Analyser::new().analyse(source, "tcl8.6");
        let input = left.resolved_input.as_ref().unwrap();
        let bindings = left.retained_command_realm().unwrap().source_bindings_ref();
        let other = foreign
            .retained_command_realm()
            .unwrap()
            .source_bindings_ref();
        let seed = bindings.original_source_transition_seed().unwrap();
        assert!(
            seed.normal_source_outcome_graph(input.borrowed_context_registry(), bindings)
                .is_some()
        );
        assert!(!Arc::ptr_eq(
            bindings.original_analysis_owner().unwrap(),
            other.original_analysis_owner().unwrap(),
        ));
        assert_eq!(seed, other.original_source_transition_seed().unwrap());
        assert!(
            seed.normal_source_outcome_graph(input.borrowed_context_registry(), other)
                .is_none()
        );
        let mut missing_owner = bindings.clone();
        missing_owner.original_analysis_owner = None;
        assert!(
            seed.normal_source_outcome_graph(input.borrowed_context_registry(), &missing_owner)
                .is_none()
        );
        let mut foreign_owner = bindings.clone();
        foreign_owner.original_analysis_owner = other.original_analysis_owner().cloned();
        assert!(
            seed.normal_source_outcome_graph(input.borrowed_context_registry(), &foreign_owner)
                .is_none()
        );
        let mut missing_seed_owner = seed.clone();
        missing_seed_owner.analysis_owner = None;
        assert!(
            missing_seed_owner
                .normal_source_outcome_graph(input.borrowed_context_registry(), bindings)
                .is_none()
        );
        // Keep every other actual point field unchanged; only replace its
        // captured source owner with another real source generation.
        let other_source = Analyser::new().analyse("set different VALUE", "tcl8.6");
        let other_bindings = other_source
            .retained_command_realm()
            .unwrap()
            .source_bindings_ref();
        let other_origin = other_bindings.source_origin().unwrap();
        assert!(!Arc::ptr_eq(other_origin, &seed.origin));
        let mut point = bindings.dispatch_points_at(0).next().unwrap().clone();
        point.state.current_source_origin = Some(Arc::clone(other_origin));
        let plan = seed
            .origin
            .original_script_words_in(
                tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
                seed.config,
                tcl_lexer::SourceSegmentationPurpose::WholeInput,
            )
            .unwrap();
        let site = CommandAllocationSite {
            source: Arc::clone(&seed.origin),
            offset: 0,
        };
        let operands = bindings
            .original_variable_invocations
            .get(&site)
            .unwrap()
            .as_ref()
            .unwrap();
        assert!(!normal_source_variable_point_is_closed(
            &point,
            &plan.commands[0].words,
            &seed.origin,
            operands,
            input.borrowed_context_registry().commands(),
            "set",
        ));
        let mut changed = seed;
        changed.config.base_line += 1;
        assert!(
            changed
                .normal_source_outcome_graph(input.borrowed_context_registry(), bindings)
                .is_none()
        );
        let second_seed = other.original_source_transition_seed().unwrap();
        let second_input = foreign.resolved_input.as_ref().unwrap();
        assert!(
            second_seed
                .normal_source_outcome_graph(second_input.borrowed_context_registry(), other)
                .is_some()
        );
    }
}

#[cfg(test)]
mod conditional_region_purpose_controls {
    use super::*;
    use crate::analyser::{Analyser, AnalysisResult};
    use crate::command_binding::OriginalSourceConditionalRegion;
    use crate::registry_invocation::source_structure::original_project_source_load_at;

    fn actual_region() -> (
        AnalysisResult,
        AnalysisResult,
        OriginalSourceConditionalRegion,
    ) {
        let source = "source child.tcl; set after DONE";
        let parent = Analyser::new().analyse(source, "tcl8.6");
        let load = original_project_source_load_at(source, &parent, 0).unwrap();
        let point = load.source_point_entry(&parent).unwrap();
        let child = Analyser::new()
            .analyse_with_source_point_entry("set child VALUE", "tcl8.6", &point)
            .unwrap();
        let chain = load
            .conditional_continuation(&parent, &child)
            .unwrap()
            .source_regions(&parent, &child)
            .unwrap();
        assert_eq!(chain.regions().len(), 1);
        (parent, child, chain.regions()[0].clone())
    }

    #[test]
    fn conditional_region_purpose_refuses_a_genuine_whole_child() {
        let (_, child, _) = actual_region();
        assert!(child.original_source_region().is_none());
        let input = child.resolved_input.as_ref().unwrap();
        let context = input.borrowed_context_registry();
        let realm = child.retained_command_realm().unwrap();
        let seed = realm
            .source_bindings_ref()
            .original_source_transition_seed()
            .unwrap();
        assert!(
            seed.prepare_for(context, OriginalSourceCataloguePurpose::CompleteProgram)
                .is_some()
        );
        assert!(
            seed.prepare_for(context, OriginalSourceCataloguePurpose::ConditionalRegion)
                .is_none()
        );
    }

    #[test]
    fn complete_program_purpose_refuses_a_genuine_conditional_region() {
        let (parent, child, region) = actual_region();
        let view = Analyser::new()
            .analyse_source_region(&region, &parent, &child)
            .unwrap();
        assert!(region.matches_region_analysis(&view));
        let input = view.resolved_input.as_ref().unwrap();
        let context = input.borrowed_context_registry();
        let realm = view.retained_command_realm().unwrap();
        let seed = realm
            .source_bindings_ref()
            .original_source_transition_seed()
            .unwrap();
        assert!(
            seed.prepare_for(context, OriginalSourceCataloguePurpose::ConditionalRegion)
                .is_some()
        );
        assert!(
            seed.prepare_for(context, OriginalSourceCataloguePurpose::CompleteProgram)
                .is_none()
        );
    }
}

#[cfg(test)]
mod nested_source_point_controls {
    use crate::analyser::Analyser;
    use crate::registry_invocation::source_structure::original_project_source_load_at;

    #[test]
    fn nested_namespace_point_uses_the_selected_body_graph_without_normal_or_native_grants() {
        let source = "namespace eval ::app {source child.tcl}";
        let parent = Analyser::new().analyse(source, "tcl8.6");
        let offset = u32::try_from(source.find("source child").unwrap()).unwrap();
        let load = original_project_source_load_at(source, &parent, offset).unwrap();
        let point = load.source_point_entry(&parent).unwrap();
        assert!(point.matches_parent_analysis(&parent));
        assert!(!point.matches_parent_analysis(&Analyser::new().analyse(source, "tcl8.6")));
        let child = Analyser::new()
            .analyse_with_source_point_entry("proc child {} {}", "tcl8.6", &point)
            .expect("actual nested namespace source graph");
        assert!(point.matches_child_analysis(&child));
        assert!(child.all_procs.contains_key("::app::child"));
        assert!(!child.all_procs.contains_key("::child"));
        assert!(!point.obligations().is_empty());
        let source = "namespace eval ::app {unknown_writer; source child.tcl}";
        let parent = Analyser::new().analyse(source, "tcl8.6");
        let offset = u32::try_from(source.find("source child").unwrap()).unwrap();
        assert!(
            original_project_source_load_at(source, &parent, offset)
                .unwrap()
                .source_point_entry(&parent)
                .is_none()
        );
    }
}
