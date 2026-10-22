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
            _ => return None,
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
pub(super) struct NormalSourceOutcomeQuery {
    pending: Cell<usize>,
    sealed: Cell<bool>,
    refused: Cell<bool>,
    graph: RefCell<Option<OriginalSourcePointAdviceGraph>>,
}

impl AdviceInvocationContext<'_> {
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
    ) -> Option<Arc<OriginalSourcePointAdviceGraph>> {
        let query = NormalSourceOutcomeQuery {
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
