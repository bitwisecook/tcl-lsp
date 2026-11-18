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

//! Source-order views of the state: a command's registry barrier and the statement timeline.

use super::super::namespace_context::SourceCommandTable;
use super::super::{ModuleCommandBindings, NamespaceResolutionProjection};
use super::binding_walk::collect_binding_states;
use super::invocation_transitions::apply_resolved_may_transitions;
use super::{BindingWalkContext, RetainedBindingRoots};
use crate::ir::Script;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tcl_registry::CommandRegistry;

impl ModuleCommandBindings {
    /// Closed root-boundary effects do not include pre-definition missing
    /// candidates from historical observations. A completed retained call can
    /// publish those effects without inventing arbitrary binding mutation.
    fn source_order_call_boundary(&self) -> Arc<Self> {
        let mut effects = self.clone();
        effects.bindings.clone_from(&self.root_boundary_bindings);
        effects.source_order_user_call_effects = None;
        Arc::new(effects)
    }

    /// Apply the closed module effects after a retained user call.
    pub(crate) fn mark_source_order_user_procedure_call(&mut self) {
        if let Some(effects) = self.source_order_user_call_effects.clone() {
            self.join(&effects);
        } else {
            self.mark_opaque_binding_mutation();
        }
    }

    /// Advance this source-order state past one recovered command, in Tcl's
    /// evaluation order: the registry binding transitions the command makes
    /// (`rename`, `interp alias`, an opaque readable-eval body), and the closed
    /// module effects of a retained user procedure it calls. A computed head,
    /// or one whose namespace is not known, may change any binding. A command
    /// expression control may skip (`conditional`) joins the state it leaves
    /// with the one before it.
    pub(crate) fn advance_source_order_for_command(
        &mut self,
        words: &[crate::ir_helpers::CommandWord],
        conditional: bool,
        registry: &CommandRegistry,
        namespace: &crate::ir_helpers::ExecutionNamespace,
    ) {
        let owner = self.baseline.metadata_context.clone();
        self.source_order_registry_barrier_for_command_with_metadata_context(
            words,
            conditional,
            registry,
            namespace,
            tcl_registry::Traits::empty(),
            owner
                .metadata_context(registry)
                .and_then(crate::registry_invocation::InvocationMetadataSelection::context),
        );
    }

    /// Advance conditional source effects using the independently retained
    /// availability. Missing or foreign metadata widens instead of borrowing
    /// catalogue effects; this supplies no physical command execution.
    pub(crate) fn source_order_registry_barrier_for_command_with_metadata_context(
        &mut self,
        words: &[crate::ir_helpers::CommandWord],
        conditional: bool,
        registry: &CommandRegistry,
        namespace: &crate::ir_helpers::ExecutionNamespace,
        barrier_traits: tcl_registry::Traits,
        metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    ) -> bool {
        if self.source_order_user_call_effects.is_none() {
            self.source_order_user_call_effects = Some(self.source_order_call_boundary());
        }
        let Some(head) = words
            .first()
            .and_then(crate::ir_helpers::CommandWord::literal)
        else {
            self.mark_opaque_binding_mutation();
            return true;
        };
        let Some(command_namespace) = namespace.for_head_context(head) else {
            self.mark_opaque_binding_mutation();
            return true;
        };
        let skipped = conditional.then(|| self.clone());
        let source_may_be_unknown = self.target_may_be_unknown(head, command_namespace.as_ref());
        let reaches_user_procedure = self
            .targets(head, command_namespace.as_ref())
            .iter()
            .any(|target| !target.registry_backed);
        let Some(metadata) = metadata.filter(|metadata| metadata.matches_registry(registry)) else {
            self.mark_opaque_binding_mutation();
            return true;
        };
        let facts = self.resolve_command_words_with_metadata_context(
            words,
            registry,
            Some(metadata),
            command_namespace.as_ref(),
        );
        let barrier = source_may_be_unknown
            || facts
                .iter()
                .any(|facts| facts.traits.intersects(barrier_traits));
        apply_resolved_may_transitions(facts, source_may_be_unknown, true, self, namespace);
        if reaches_user_procedure {
            self.mark_source_order_user_procedure_call();
        }
        if let Some(skipped) = skipped {
            self.join(&skipped);
        }
        barrier
    }
}

impl ModuleCommandBindings {
    /// Replay an independently callable body from completed root boundaries,
    /// excluding historical pre-definition missing candidates.
    pub(crate) fn source_binding_timeline_from_boundary(
        &self,
        script: &Script,
        registry: &CommandRegistry,
        namespace: &crate::ir::ExecutionNamespace,
    ) -> SourceBindingTimeline {
        let effects = self.source_order_call_boundary();
        let mut initial = effects.as_ref().clone();
        initial.source_order_user_call_effects = Some(effects);
        Self::source_binding_timeline_from_initial(script, registry, namespace, &initial)
    }

    /// Replay one root in source order for CFG-local registry projections.
    /// Top level begins with its fresh registry state; independently callable
    /// procedure roots begin from the historical state reachable after module
    /// initialisation and never feed their transient states back into it.
    #[must_use]
    pub(crate) fn source_binding_timeline(
        &self,
        script: &Script,
        registry: &CommandRegistry,
        namespace: &crate::ir::ExecutionNamespace,
        top_level_root: bool,
    ) -> SourceBindingTimeline {
        let mut initial = self.clone();
        if initial.source_order_user_call_effects.is_none() {
            initial.source_order_user_call_effects = Some(self.source_order_call_boundary());
        }
        if top_level_root {
            // The document starts from its entry's command rows: none for an
            // authored module, the interpreter's own for a native entry.
            initial.bindings = Arc::clone(&self.original_entry_bindings);
            if let Some(entry) = &self.original_entry_packages {
                initial.packages = Arc::clone(&entry.packages);
                initial.revoked_loaders = Arc::clone(&entry.revoked_loaders);
                initial.loader_handler_unknown = entry.loader_handler_unknown;
                initial.original_command_world = Arc::new(
                    initial
                        .original_command_world
                        .with_entry_availability(&entry.command_world),
                );
            }
            initial.root_boundary_bindings = Arc::new(SourceCommandTable::default());
            initial.logical_alias_prefixes = Arc::new(BTreeMap::new());
            initial.opaque_domain = false;
            initial.opaque_binding_mutation = false;
            initial.dynamic_proc_binding = false;
            initial.unnameable_rebinding_subject = false;
            initial.namespace_resolution = NamespaceResolutionProjection::default();
            if !initial.rebound_names.is_empty() {
                initial.rebound_names.clear();
            }
            initial.proc_rebound_names.clear();
        }
        Self::source_binding_timeline_from_initial(script, registry, namespace, &initial)
    }

    /// Replay one source root from a caller-provided entry state. Procedure
    /// CFGs use this to start from the states reachable after their own
    /// definition, instead of the module's historical union from before that
    /// definition existed.
    #[must_use]
    pub(crate) fn source_binding_timeline_from_initial(
        script: &Script,
        registry: &CommandRegistry,
        namespace: &crate::ir::ExecutionNamespace,
        initial: &ModuleCommandBindings,
    ) -> SourceBindingTimeline {
        let mut timeline = SourceBindingTimeline::default();
        let mut retained_roots = RetainedBindingRoots::default();
        let mut context = BindingWalkContext {
            registry,
            retained_roots: &mut retained_roots,
            timeline: Some(&mut timeline),
            source_order_mode: true,
        };
        timeline.post = Some(collect_binding_states(script, &mut context, initial, namespace).post);
        timeline
    }
}

/// Source-order command states for one executable script root. The two
/// snapshots distinguish substitutions, which run before the host command,
/// from the direct invocation itself.
#[derive(Debug, Clone, Default)]
pub(crate) struct SourceBindingTimeline {
    states: HashMap<tcl_lexer::Span, StatementBindingStates>,
    /// State after this source root completes. This is not historical
    /// observation: a procedure defined late in the root can begin from it,
    /// while a procedure defined earlier joins it with the suffix it could
    /// have observed after becoming callable.
    pub(in super::super) post: Option<ModuleCommandBindings>,
    /// Cache suffix joins once for all procedure definitions in a source root.
    /// Replaying every suffix separately makes large multi-proc files cubic.
    suffix_states: std::sync::OnceLock<Vec<(u32, ModuleCommandBindings)>>,
}

#[derive(Debug, Clone)]
pub(crate) struct StatementBindingStates {
    before_substitutions: ModuleCommandBindings,
    before_direct_call: Option<ModuleCommandBindings>,
}

impl SourceBindingTimeline {
    /// States a procedure may enter after its defining command at
    /// `definition_span` has run.
    ///
    /// Source snapshots before that definition are deliberately excluded: a
    /// later `proc eval` has replaced the builtin before a later `proc p`
    /// first becomes callable. The source suffix and final post-state retain
    /// temporary and terminal transitions that occur after `p` is available.
    /// The closed root-boundary state adds independently executable roots
    /// without replaying the module's earlier historical observations.
    pub(crate) fn entry_after(
        &self,
        definition_span: tcl_lexer::Span,
        module: &ModuleCommandBindings,
    ) -> Option<ModuleCommandBindings> {
        let post = self.post.as_ref()?;
        let suffixes = self.suffix_states.get_or_init(|| {
            let mut by_start: BTreeMap<u32, Vec<&StatementBindingStates>> = BTreeMap::new();
            for (span, states) in &self.states {
                by_start.entry(span.start()).or_default().push(states);
            }
            let mut suffix = post.clone();
            let mut snapshots = Vec::with_capacity(by_start.len());
            for (start, states) in by_start.into_iter().rev() {
                for states in states {
                    if !suffix.same_state(&states.before_substitutions) {
                        suffix.join(&states.before_substitutions);
                    }
                    if let Some(before_call) = &states.before_direct_call
                        && !suffix.same_state(before_call)
                    {
                        suffix.join(before_call);
                    }
                }
                snapshots.push((start, suffix.clone()));
            }
            snapshots.reverse();
            snapshots
        });
        let next = suffixes.partition_point(|(start, _)| *start <= definition_span.start());
        let mut entry = suffixes.get(next).map_or(post, |(_, state)| state).clone();
        let mut boundary = module.clone();
        boundary.bindings = Arc::clone(&module.root_boundary_bindings);
        if !entry.same_state(&boundary) {
            entry.join(&boundary);
        }
        Some(entry)
    }

    pub(super) fn join_region(&mut self, script: &Script, state: &ModuleCommandBindings) {
        crate::ir::for_each_statement(script, &mut |stmt| {
            self.record_before_substitutions(stmt.span(), state);
            self.record_before_direct_call(stmt.span(), state);
        });
    }

    pub(super) fn record_before_substitutions(
        &mut self,
        span: tcl_lexer::Span,
        state: &ModuleCommandBindings,
    ) {
        self.record(span, state, true);
    }

    pub(super) fn record_before_direct_call(
        &mut self,
        span: tcl_lexer::Span,
        state: &ModuleCommandBindings,
    ) {
        self.record(span, state, false);
    }

    pub(in super::super) fn record(
        &mut self,
        span: tcl_lexer::Span,
        state: &ModuleCommandBindings,
        substitutions: bool,
    ) {
        self.suffix_states.take();
        if let Some(existing) = self.states.get_mut(&span) {
            if substitutions {
                if !existing.before_substitutions.same_state(state) {
                    existing.before_substitutions.join(state);
                }
            } else {
                match &mut existing.before_direct_call {
                    Some(before_direct_call) => {
                        if !before_direct_call.same_state(state) {
                            before_direct_call.join(state);
                        }
                    }
                    None => existing.before_direct_call = Some(state.clone()),
                }
            }
        } else {
            self.states.insert(
                span,
                StatementBindingStates {
                    before_substitutions: state.clone(),
                    before_direct_call: (!substitutions).then(|| state.clone()),
                },
            );
        }
    }

    pub(crate) fn before_substitutions(
        &self,
        span: tcl_lexer::Span,
    ) -> Option<&ModuleCommandBindings> {
        self.states
            .get(&span)
            .map(|states| &states.before_substitutions)
    }

    pub(crate) fn before_direct_call(
        &self,
        span: tcl_lexer::Span,
    ) -> Option<&ModuleCommandBindings> {
        self.states
            .get(&span)
            .and_then(|states| states.before_direct_call.as_ref())
    }
}
