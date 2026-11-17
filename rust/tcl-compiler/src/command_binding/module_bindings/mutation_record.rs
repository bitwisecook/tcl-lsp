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

//! Rebindings recorded as opaque, dynamic or unnameable, and the summaries projected from them.

use super::super::mutation_summary::{
    PrecisionWrites, RebindingSubjects, builtin_shadowed_by_qualified_definition,
};
use super::super::namespace_context::SourceNamespaceKey;
use super::super::{
    BindingKind, ModuleCommandBindings, ModuleCommandMutations, ProcBindingTrustProjection,
    SourceCommandKey, default_binding,
};
use super::binding_walk::collect_binding_states;
use super::{BindingWalkContext, RetainedBindingRoots};
use crate::ir::Script;
use crate::naming::normalise_qualified_name as nqn;
use std::sync::Arc;
use tcl_registry::CommandRegistry;

impl ModuleCommandBindings {
    /// Widen command resolution without claiming an unknown command-table
    /// mutation. Namespace lookup changes are relevant to source-safe
    /// resolution but do not by themselves invalidate optimiser trust.
    pub(in super::super) fn mark_opaque_resolution(&mut self) {
        Arc::make_mut(&mut self.original_command_world).withdraw();
        self.withdraw_ensemble_dispatches();
        self.opaque_domain = true;
        self.mark_receiver_frame_opacity();
    }

    fn mark_receiver_frame_opacity(&mut self) {
        if matches!(
            self.variable_frame,
            crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
        ) {
            self.receiver_frame_opacity = true;
        }
    }

    pub(in super::super) fn mark_opaque_binding_mutation(&mut self) {
        Arc::make_mut(&mut self.command_observers)
            .fresh_procedures
            .clear();
        Arc::make_mut(&mut self.original_command_world).withdraw();
        self.withdraw_ensemble_dispatches();
        self.ordinary_literal_pool = None;
        Arc::make_mut(&mut self.object_instances).invalidate_dispatch(true);
        self.opaque_domain = true;
        self.opaque_binding_mutation = true;
        self.mark_receiver_frame_opacity();
        Arc::make_mut(&mut self.namespaces).retain(SourceNamespaceKey::is_root);
        Arc::make_mut(&mut self.source_variables).widen();
    }

    /// Record the narrow event that invalidates an otherwise-retained
    /// procedure's declared binding at arbitrary call sites.
    pub(super) fn mark_dynamic_proc_binding(&mut self) {
        self.mark_opaque_binding_mutation();
        self.dynamic_proc_binding = true;
    }

    /// Record that a binding *transition* moved something this lattice cannot
    /// name, so no name in the module can be claimed as untouched (#2168).
    pub(super) fn mark_unnameable_rebinding_subject(&mut self) {
        self.mark_dynamic_proc_binding();
        self.unnameable_rebinding_subject = true;
    }

    pub(in super::super) fn record_proc_rebound_candidates(
        &mut self,
        name: &str,
        namespace: &crate::ir_helpers::ExecutionNamespace,
    ) {
        let Some(context) = namespace.for_head_context(name) else {
            self.mark_dynamic_proc_binding();
            return;
        };
        if let SourceNamespaceKey::Authored(namespace) = context.as_ref() {
            self.proc_rebound_names.extend(
                tcl_syntax::naming::bareword_resolution_candidates(namespace, name)
                    .into_iter()
                    .map(|candidate| nqn(&candidate)),
            );
            return;
        }
        let Ok(candidates) = self.source_keys_checked(name, context.as_ref()) else {
            self.mark_dynamic_proc_binding();
            return;
        };
        let Some(labels) = candidates
            .iter()
            .map(crate::command_binding::ModuleCommandBindings::command_key_label)
            .collect::<Option<Vec<_>>>()
        else {
            self.mark_dynamic_proc_binding();
            return;
        };
        // Labels only invalidate procedure trust. The original typed slots
        // remain the command-table lookup and mutation identities.
        self.proc_rebound_names.extend(labels);
    }

    /// Current changed cells and retained mutation history exclude the
    /// independently captured initial command table. A restored slot remains
    /// in the history even when its current implementation matches the entry.
    pub(in super::super) fn changed_command_spellings(&self) -> impl Iterator<Item = String> + '_ {
        self.rebound_names
            .iter()
            .chain(self.bindings.iter().filter_map(|(key, observed)| {
                let original = self
                    .original_entry_bindings
                    .get(key)
                    .cloned()
                    .unwrap_or_else(|| {
                        Self::unmodified_bindings(key, self.baseline.semantics.binding_names())
                    });
                (*observed != original).then_some(key)
            }))
            .filter_map(|key| self.callable_spelling_for_key(key))
            .chain(self.namespace_resolution.rebound_names.iter().cloned())
    }

    pub(in super::super) fn callable_spelling_for_key(
        &self,
        key: &SourceCommandKey,
    ) -> Option<String> {
        match key {
            SourceCommandKey::Authored(text) => Some(text.clone()),
            SourceCommandKey::Slot { namespace, simple } => {
                let policy = match self.baseline.native_entry.as_ref() {
                    Some(entry) => entry.command_name_policy(),
                    None => self.baseline.execution_name_policy?.native_recipe(),
                };
                let protocol = policy?.recipe();
                let slot = tcl_core_types::NativeByteCommandSlot::new(
                    namespace.source_artifact_path()?,
                    simple.clone(),
                );
                tcl_syntax::naming::native_command_source_spelling(protocol, &slot)
            }
        }
    }

    /// Whether any command spelling may have been affected by an unbounded
    /// command-table mutation.
    #[must_use]
    pub(crate) const fn has_opaque_domain(&self) -> bool {
        self.opaque_domain
    }

    /// The commands the analysed document declares as plain calls, which
    /// this state's command table binds beside the registry's names, with the
    /// frame effect each declaration states ([`Module::declared_frame_effects`]).
    #[must_use]
    pub(crate) fn declared_frame_effects(&self) -> Arc<crate::ir::DeclaredFrameEffects> {
        Arc::clone(&self.baseline.declared)
    }

    /// Whether this state holds a command table to resolve against: the
    /// registry's baseline of the analysed module. A state built for no module
    /// (`Default`) names no command, so it can say nothing about one.
    #[must_use]
    pub(crate) fn holds_a_command_table(&self) -> bool {
        !self.baseline.semantics.binding_names().is_empty()
    }

    /// Whether a `rename`, an `interp alias` or a command delete moved a name
    /// this lattice cannot name, so that no spelling can be claimed to denote
    /// what it did before ([`Self::unnameable_rebinding_subject`]).
    #[must_use]
    pub(crate) const fn has_unnameable_rebinding_subject(&self) -> bool {
        self.unnameable_rebinding_subject
    }

    /// Project this already-computed binding summary into the optimiser's
    /// flow-insensitive command-binding trust view.
    ///
    /// This is the allocation boundary for consumers that already retain a
    /// [`ModuleCommandBindings`]: they must not repeat the module walk via
    /// [`scan_module_command_mutations`]. The published state includes every
    /// transient executable binding, so a rename followed by a restore still
    /// distrusts the affected builtin. Registry-declared namespace effects are
    /// recorded by the same resolved invocation walk, including alias prefixes
    /// and recovered executable roots.
    #[must_use]
    pub(crate) fn mutation_projection(&self, registry: &CommandRegistry) -> ModuleCommandMutations {
        let mut names = std::collections::HashSet::new();
        for (key, observed) in self.bindings.iter() {
            let Some(name) = self.callable_spelling_for_key(key) else {
                continue;
            };
            // A builtin's slot starts as the release's original table says,
            // which the walk read when it resolved the builtin.
            let original = self
                .original_initial_bindings_for_key(key)
                .unwrap_or_else(|| {
                    Self::unmodified_bindings(key, self.baseline.semantics.binding_names())
                });
            if *observed == original {
                continue;
            }
            if default_binding(&name, registry).kind == BindingKind::Builtin {
                names.insert(name.clone());
            } else if let Some(shadowed) = builtin_shadowed_by_qualified_definition(&name, registry)
            {
                // The same tail projection the source scan applies, so a
                // qualified shadow installed through a recovered binding is
                // distrusted too. An alias carrying the prepended name —
                // `interp alias {} make {} proc ::n::expr` then
                // `make {s} {return "SHADOW:$s"}` — defines `::n::expr`
                // without the source scan ever seeing that spelling, and
                // O110 would still rewrite an unqualified `expr` inside
                // `::n` (#2159).
                names.insert(shadowed);
            }
        }
        ModuleCommandMutations {
            names,
            rebound: self
                .rebound_names
                .iter()
                // An authored slot is spelt for `names` (a document's own
                // `proc expr`), but it is also every fresh `proc`, which this
                // set excludes.
                .filter(|key| match key {
                    SourceCommandKey::Authored(_) => true,
                    SourceCommandKey::Slot { namespace, .. } => {
                        self.baseline.native_entry.is_some()
                            && namespace.exact_native_path().is_some()
                    }
                })
                .filter_map(|key| self.callable_spelling_for_key(key))
                // The rename, alias and delete subjects, whichever slot
                // their key took.
                .chain(self.proc_rebound_names.iter().cloned())
                .chain(self.namespace_resolution.rebound_names.iter().cloned())
                .collect(),
            dynamic: self.opaque_binding_mutation
                || self.namespace_resolution.dynamic
                || self.has_redefined_procedures,
            // The narrowest of the three binding-opacity flags, deliberately.
            // `opaque_binding_mutation` also rises for an opaque substitution,
            // a non-literal head, an unresolvable head namespace and the walk
            // depth cap; `dynamic_proc_binding` adds a runtime-selected
            // executable root (every `oo` method body) and a widened binding
            // domain. None of those move a binding, and gating folds on them
            // is the cost #2164 measured and refused. Only a transition that
            // moved a subject this lattice could not name may withdraw the
            // claim that an untouched name still denotes its builtin.
            rebinding_subjects: RebindingSubjects::from_unnameable(
                self.unnameable_rebinding_subject,
            ),
            // This projection is about names the closed lattice observed, not
            // about which frame observed them; the frame fact is the scan's.
            runtime_selected_frames: false,
            resolution_changed: self.namespace_resolution.changed,
            opaque_namespaces: self
                .namespace_resolution
                .opaque_namespaces
                .iter()
                .cloned()
                .collect(),
            // A variable fact, which the source scan owns.
            precision_written: PrecisionWrites::Unwritten,
        }
    }

    /// Project the prepared lattice into the narrower trust fact used when a
    /// call site wants to seed a retained procedure's parameters: only an
    /// explicit rebinding or a dynamic command-binding transition
    /// disqualifies the declared procedure identity; source/lookup/body
    /// opacity alone does not.
    #[must_use]
    pub(crate) fn proc_binding_trust_projection(&self) -> ProcBindingTrustProjection {
        ProcBindingTrustProjection {
            rebound: self.proc_rebound_names.iter().cloned().collect(),
            dynamic: self.dynamic_proc_binding || self.has_redefined_procedures,
        }
    }

    /// Whether executing `script` itself introduces an unbounded command
    /// binding effect, excluding opacity inherited from unrelated module
    /// roots. Effect-summary consumers use this delta query instead of
    /// projecting one root's uncertainty onto every procedure.
    #[must_use]
    pub(crate) fn script_has_opaque_binding_effect(
        &self,
        script: &Script,
        registry: &CommandRegistry,
        namespace: &str,
    ) -> bool {
        let mut baseline = self.clone();
        baseline.opaque_domain = false;
        baseline.opaque_binding_mutation = false;
        baseline.dynamic_proc_binding = false;
        baseline.unnameable_rebinding_subject = false;
        baseline.bindings.clone_from(&self.root_boundary_bindings);
        let mut retained_roots = RetainedBindingRoots::default();
        let execution_namespace = crate::ir::ExecutionNamespace::exact(namespace);
        let outcome = {
            let mut context = BindingWalkContext {
                registry,
                retained_roots: &mut retained_roots,
                timeline: None,
                source_order_mode: false,
            };
            collect_binding_states(script, &mut context, &baseline, &execution_namespace)
        };
        outcome.post.opaque_binding_mutation || outcome.observed.opaque_binding_mutation
    }

    #[cfg(test)]
    pub(in super::super) fn rebound_names(&self) -> impl Iterator<Item = &str> {
        self.rebound_names
            .iter()
            .filter_map(SourceCommandKey::authored_spelling)
    }
}
