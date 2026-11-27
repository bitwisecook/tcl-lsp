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

//! The native command node behind each binding: compiler attachments, cache epochs, creation entries, unlinking and node-addressed lookups.

use super::{
    CommandBinding, Namespaces, NativeCommandCreation, NativeCompilerRecipe, NsId,
    RetiredCommandSlot, GLOBAL,
};
use crate::interp::{BuiltinFn, Command};
use tcl_syntax::naming::written_command_parts;

impl Namespaces {
    pub(crate) fn native_command_generations(&self) -> Vec<u64> {
        self.native_command_nodes.keys().copied().collect()
    }

    /// Actual compiler attachment on a retained raw token, independent of
    /// origin redirects and current handler semantics.
    pub(crate) fn native_compiler_hook(
        &self,
        generation: u64,
    ) -> Option<tcl_runtime_api::native_compilation::NativeCompilerHookPresence> {
        self.native_command_nodes
            .get(&generation)
            .map(|node| node.compiler_hook)
    }

    /// Install an actual compiler attachment. C's procedure NoOp installation
    /// does not itself advance the interpreter compiler epoch. Existing imports
    /// retain the attachment they copied when their own token was created.
    #[cfg(test)]
    pub(crate) fn set_native_compiler_hook(
        &mut self,
        generation: u64,
        hook: tcl_runtime_api::native_compilation::NativeCompilerHookPresence,
    ) {
        if let Some(node) = self.native_command_nodes.get_mut(&generation) {
            node.compiler_hook = hook;
            node.compiler_handler = None;
            node.compiler_recipe = match hook {
                tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Absent => {
                    NativeCompilerRecipe::Absent
                }
                _ => NativeCompilerRecipe::Unknown,
            };
        }
    }

    pub(crate) fn native_compiler_recipe(&self, generation: u64) -> Option<NativeCompilerRecipe> {
        self.native_command_nodes
            .get(&generation)
            .map(|node| node.compiler_recipe.clone())
    }

    pub(crate) fn set_native_compiler_recipe(
        &mut self,
        generation: u64,
        recipe: NativeCompilerRecipe,
    ) {
        use tcl_runtime_api::native_compilation::NativeCompilerHookPresence as Hook;
        if let Some(node) = self.native_command_nodes.get_mut(&generation) {
            node.compiler_hook = match recipe {
                NativeCompilerRecipe::Absent => Hook::Absent,
                NativeCompilerRecipe::Unknown => Hook::Unknown,
                NativeCompilerRecipe::Registered { .. } | NativeCompilerRecipe::ProcedureNoOp => {
                    Hook::Present
                }
            };
            node.compiler_recipe = recipe;
            node.compiler_handler = None;
        }
    }

    pub(crate) fn native_compiler_handler(&self, generation: u64) -> Option<BuiltinFn> {
        self.native_command_nodes.get(&generation)?.compiler_handler
    }

    pub(crate) fn set_native_compiler_handler(
        &mut self,
        generation: u64,
        handler: Option<BuiltinFn>,
    ) {
        if let Some(node) = self.native_command_nodes.get_mut(&generation) {
            node.compiler_handler = handler;
        }
    }

    /// Apply the shared native invalidation policy at its actual mutation door.
    /// Namespace resolver epochs and the interpreter compiler epoch are separate.
    pub(crate) fn note_native_compiler_mutation(
        &mut self,
        namespace: Option<NsId>,
        mutation: tcl_registry::native_procedure::NativeCompilerCacheMutation,
    ) {
        let Some(version) = self.native_command_version else {
            return;
        };
        let invalidated = tcl_registry::native_procedure::native_compiler_cache_invalidated(
            tcl_native_model::InvocationDialect::for_version(version),
            mutation,
        );
        let epoch = match namespace {
            Some(namespace) => &mut self.arena[namespace].native_resolver_epoch,
            None => &mut self.native_compiler_epoch,
        };
        match invalidated {
            Some(false) => {}
            Some(true) => {
                *epoch = epoch.map(|epoch| {
                    epoch
                        .checked_add(1)
                        .expect("native compiler epoch exhausted")
                });
            }
            None => *epoch = None,
        }
    }

    pub(crate) fn native_compiler_cache_epochs(&self, namespace: NsId) -> Option<(u64, u64)> {
        self.native_command_version?;
        if !self.namespace_is_live(namespace)
            && self.activations.get(namespace).copied().unwrap_or(0) == 0
            && !self.dying.contains(&namespace)
            && !self.deferred.contains(&namespace)
        {
            return None;
        }
        Some((
            self.native_compiler_epoch?,
            self.arena.get(namespace)?.native_resolver_epoch?,
        ))
    }

    /// Select independently authenticated native command-cache semantics.
    pub(crate) fn set_native_command_version(&mut self, version: Option<tcl_dialect::TclVersion>) {
        if self.native_command_version != version {
            for node in &mut self.arena {
                node.command_reference_epoch = node
                    .command_reference_epoch
                    .checked_add(1)
                    .expect("native command reference epoch exhausted");
            }
            for node in self.native_command_nodes.values_mut() {
                node.epoch = node
                    .epoch
                    .checked_add(1)
                    .expect("native command epoch exhausted");
            }
            self.native_command_version = version;
        }
    }

    pub(super) fn increment_command_reference(&mut self, ns: NsId) {
        self.arena[ns].command_reference_epoch = self.arena[ns]
            .command_reference_epoch
            .checked_add(1)
            .expect("native command reference epoch exhausted");
    }

    /// TclInvalidateNsPath: each actual reverse-path entry invalidates its creator.
    pub(super) fn invalidate_native_path_dependents(&mut self, target: NsId) {
        if self
            .native_command_version
            .is_none_or(|version| version == tcl_dialect::TclVersion::V8_4)
        {
            return;
        }
        for ns in 0..self.arena.len() {
            let count = self.arena[ns]
                .path
                .iter()
                .filter(|entry| **entry == target)
                .count();
            for _ in 0..count {
                self.increment_command_reference(ns);
            }
        }
    }

    /// TclInvalidateNsCmdLookup is distinct from reverse-path invalidation.
    pub(crate) fn ensemble_export_epoch(&self, namespace: NsId) -> u64 {
        self.arena[namespace].ensemble_export_epoch
    }

    pub(crate) fn advance_ensemble_export_epoch(&mut self, namespace: NsId) {
        self.arena[namespace].ensemble_export_epoch = self.arena[namespace]
            .ensemble_export_epoch
            .checked_add(1)
            .expect("native ensemble export epoch exhausted");
    }

    pub(crate) fn invalidate_native_command_lookup(&mut self, ns: NsId) {
        if !self.arena[ns].exports.is_empty() {
            self.note_guard_command_lookup_policy_mutation();
            self.advance_ensemble_export_epoch(ns);
        }

        if self
            .native_command_version
            .is_some_and(|version| version != tcl_dialect::TclVersion::V8_4)
            && !self.arena[ns].path.is_empty()
        {
            self.increment_command_reference(ns);
        }
    }

    /// TclResetShadowedCmdRefs walks physical parents and global child trails.
    pub(super) fn reset_native_shadowed_references(&mut self, namespace: NsId, simple: &[u8]) {
        if self.native_command_version.is_none() {
            return;
        }
        let mut trail: Vec<NsId> = Vec::new();
        let mut current = Some(namespace);
        while let Some(ns) = current.filter(|ns| *ns != GLOBAL) {
            let mut shadow = Some(GLOBAL);
            for &child in trail.iter().rev() {
                shadow = shadow.and_then(|parent| {
                    self.arena[parent]
                        .children
                        .get(&self.arena[child].name)
                        .copied()
                });
            }
            if let Some(global) =
                shadow.filter(|global| self.arena[*global].commands.contains_key(simple))
            {
                let generation = self.arena[global]
                    .commands
                    .generation(simple)
                    .expect("shadowed command retains its token");
                let hook = self
                    .native_compiler_hook(generation)
                    .expect("shadowed command retains its native node");
                self.note_native_compiler_mutation(Some(ns),
                    tcl_registry::native_procedure::NativeCompilerCacheMutation::NamespaceCommandShadow { hook });
                self.increment_command_reference(ns);
                self.invalidate_native_path_dependents(ns);
            }
            trail.push(ns);
            current = self.arena[ns].parent;
        }
    }

    /// Capture occupancy and C8.5's early ObjCommand path invalidation.
    pub(crate) fn native_command_creation_entry(
        &mut self,
        ns: NsId,
        simple: &[u8],
    ) -> NativeCommandCreation {
        self.native_command_creation_entry_with_jim_key(ns, simple, None)
    }

    pub(crate) fn native_command_creation_entry_with_jim_key(
        &mut self,
        ns: NsId,
        simple: &[u8],
        incoming: Option<tcl_syntax::naming::NativeJimCommandTableKey>,
    ) -> NativeCommandCreation {
        self.native_command_creation_entry_with_original_key(ns, simple, incoming, None)
    }

    /// The actual constructor supplies its key object separately from reporting
    /// units. Byte-only table utilities do not manufacture an original header.
    pub(crate) fn native_command_creation_entry_with_original_key(
        &mut self,
        ns: NsId,
        simple: &[u8],
        incoming: Option<tcl_syntax::naming::NativeJimCommandTableKey>,
        original_jim_key: Option<std::rc::Rc<crate::obj::Owned>>,
    ) -> NativeCommandCreation {
        if self.native_command_version == Some(tcl_dialect::TclVersion::V8_5) {
            self.invalidate_native_path_dependents(ns);
        }
        let occupied = self.arena[ns].commands.binding(simple);
        let jim_table_key = self
            .variable_name_protocol
            .filter(|protocol| protocol.is_jim084())
            .and_then(|protocol| {
                let incoming = incoming.or_else(|| {
                    tcl_syntax::naming::NativeJimCommandTableKey::from_comparison_key(
                        protocol, simple,
                    )
                })?;
                assert_eq!(
                    incoming.comparison_bytes(),
                    simple,
                    "actual Jim table comparison slot"
                );
                Some(incoming.retain_for_replacement(
                    occupied.and_then(|binding| binding.jim_table_key.as_ref()),
                ))
            });
        NativeCommandCreation {
            namespace: ns,
            original_tail: simple.to_vec(),
            occupied: occupied.is_some(),
            occupied_jim_key: self.arena[ns]
                .commands
                .entries
                .get(simple)
                .and_then(|slot| slot.original_jim_key.as_ref())
                .map(std::rc::Rc::downgrade),
            jim_table_key,
            original_jim_key,
        }
    }

    /// A value destructor may replace the value under the same original key.
    /// It may not donate a deleted/recreated key slot to this old operation.
    pub(crate) fn original_jim_creation_slot_is_current(
        &self,
        entry: &NativeCommandCreation,
    ) -> bool {
        if !self
            .variable_name_protocol
            .is_some_and(|protocol| protocol.is_jim084())
        {
            return true;
        }
        let current = self
            .arena
            .get(entry.namespace)
            .and_then(|namespace| {
                namespace
                    .commands
                    .entries
                    .get(entry.original_tail.as_slice())
            })
            .and_then(|slot| slot.original_jim_key.as_ref());
        match (&entry.occupied_jim_key, current) {
            (None, None) => true,
            (Some(original), Some(current)) => {
                std::rc::Weak::ptr_eq(original, &std::rc::Rc::downgrade(current))
                    && crate::obj::allocation_is_live(current.as_ptr())
            }
            _ => false,
        }
    }

    pub(super) fn note_native_command_created(
        &mut self,
        entry: NativeCommandCreation,
        simple: &[u8],
    ) {
        let ns = entry.namespace;
        if !entry.occupied {
            self.invalidate_native_command_lookup(ns);
            if self.native_command_version != Some(tcl_dialect::TclVersion::V8_5) {
                self.invalidate_native_path_dependents(ns);
            }
        }
        self.reset_native_shadowed_references(ns, simple);
    }

    pub(super) fn increment_native_command_epoch(&mut self, generation: u64) {
        if let Some(hook) = self.native_compiler_hook(generation) {
            self.note_native_compiler_mutation(
                None,
                tcl_registry::native_procedure::NativeCompilerCacheMutation::CommandToken { hook },
            );
        }
        if let Some(node) = self.native_command_nodes.get_mut(&generation) {
            node.epoch = node
                .epoch
                .checked_add(1)
                .expect("native command epoch exhausted");
        }
    }

    pub(crate) fn original_command_deletion_started(&self, generation: u64) -> bool {
        self.native_command_nodes
            .get(&generation)
            .is_some_and(|node| node.deletion_started)
    }

    pub(crate) fn begin_original_command_deletion(&mut self, generation: u64) {
        if let Some(node) = self.native_command_nodes.get_mut(&generation) {
            node.deletion_started = true;
        }
    }

    /// Selected deletion's command epoch is independent of compiler mutation.
    pub(crate) fn advance_original_deletion_epoch(&mut self, generation: u64) {
        if let Some(node) = self.native_command_nodes.get_mut(&generation) {
            node.epoch = node
                .epoch
                .checked_add(1)
                .expect("native command epoch exhausted");
        }
    }

    /// The original post-trace compiler and namespace stages. No reported name
    /// is resolved, and the ordinary rename/hide workers retain their own effects.
    pub(crate) fn original_deletion_after_trace(
        &mut self,
        generation: u64,
        invalidate_namespace: bool,
    ) {
        if invalidate_namespace {
            if let Some(namespace) = self
                .native_command_nodes
                .get(&generation)
                .map(|node| node.namespace)
            {
                self.invalidate_native_command_lookup(namespace);
            }
        }
        if let Some(hook) = self.native_compiler_hook(generation) {
            self.note_native_compiler_mutation(
                None,
                tcl_registry::native_procedure::NativeCompilerCacheMutation::CommandToken { hook },
            );
        }
    }

    /// Raw unlink for this one already captured deletion. Implementation roles
    /// are retired by the outside-borrow owner; no generic epoch is replayed.
    pub(crate) fn unlink_original_command_generation(
        &mut self,
        generation: u64,
    ) -> Option<CommandBinding> {
        let (namespace, simple) = self.native_command_slot_at_node(generation)?;
        self.invalidate_single_original_unlink(generation);
        let binding = self.arena[namespace].commands.remove_binding(&simple)?;
        if matches!(&binding.command, Command::Imported { .. }) {
            self.note_guard_command_lookup_policy_mutation();
        }
        self.native_command_nodes.get_mut(&generation)?.placement = None;
        Some(binding)
    }

    pub(super) fn unlink_native_binding(
        &mut self,
        ns: NsId,
        simple: &[u8],
    ) -> Option<CommandBinding> {
        if self.arena[ns].commands.contains_key(simple) {
            self.invalidate_original_publications();
        }
        let binding = self.arena[ns].commands.remove_binding(simple)?;
        self.increment_native_command_epoch(binding.generation);
        self.native_command_nodes
            .get_mut(&binding.generation)
            .expect("bound command retains its native command node")
            .placement = None;
        self.invalidate_native_command_lookup(ns);
        self.note_jim_command_retirement(binding.generation);
        Some(binding)
    }

    pub(super) fn remove_native_binding(
        &mut self,
        ns: NsId,
        simple: &[u8],
    ) -> Option<CommandBinding> {
        let binding = self.unlink_native_binding(ns, simple)?;
        if self.variable_name_protocol != Some(tcl_syntax::naming::NativeNameProtocol::Jim084) {
            let procedure = match &binding.command {
                Command::Proc(procedure) => procedure.take_for_retirement(),
                _ => None,
            };
            let view = binding.clone();
            self.arena[ns]
                .commands
                .retired
                .borrow_mut()
                .push(RetiredCommandSlot {
                    procedure,
                    original_jim_key: None,
                    binding: Some(binding),
                });
            Some(view)
        } else {
            // remove_binding already extracted Jim's real key and value owners.
            Some(binding)
        }
    }

    pub(super) fn take_native_binding_for_replacement(
        &mut self,
        ns: NsId,
        simple: &[u8],
    ) -> Option<CommandBinding> {
        if !self
            .variable_name_protocol
            .is_some_and(|protocol| protocol.is_jim084())
        {
            return self.remove_native_binding(ns, simple);
        }
        if self.arena[ns].commands.contains_key(simple) {
            self.invalidate_original_publications();
        }
        let binding = self.arena[ns].commands.take_slot(simple)?;
        self.increment_native_command_epoch(binding.generation);
        self.native_command_nodes
            .get_mut(&binding.generation)
            .expect("bound command retains its native command node")
            .placement = None;
        self.note_jim_command_retirement(binding.generation);
        Some(binding)
    }

    /// Value replacement retains Jim's genuine occupied key. C still removes
    /// the original entry using its independent replacement recipe.
    pub(crate) fn remove_in_for_replacement(&mut self, ns: NsId, simple: &[u8]) -> bool {
        self.take_native_binding_for_replacement(ns, simple)
            .is_some_and(|binding| {
                Self::mark_command_deleted(&binding.command, &self.retired_ensemble_roles);
                // Actual callers retain the displaced command through this stage.
                true
            })
    }

    /// Hide retains the command node, but invalidates its cached references.
    pub(crate) fn note_native_command_hidden(&mut self, generation: u64) {
        self.invalidate_original_publications();
        self.increment_native_command_epoch(generation);
        if let Some(node) = self.native_command_nodes.get_mut(&generation) {
            let placement = node.placement.take();
            if let Some((ns, _)) = placement {
                self.invalidate_native_command_lookup(ns);
            }
        }
    }

    /// Retire a hidden node without deriving its identity from the hidden name.
    pub(crate) fn retire_native_command_node(&mut self, generation: u64) {
        self.invalidate_original_publications();
        self.increment_native_command_epoch(generation);
        if let Some(node) = self.native_command_nodes.get_mut(&generation) {
            node.placement = None;
        }
    }

    /// Native Jim epoch; independent of compiler guards and C command epochs.
    pub(crate) const fn jim_procedure_epoch(&self) -> u64 {
        self.jim_procedure_epoch
    }

    pub(super) fn advance_jim_procedure_epoch(&mut self) {
        self.jim_procedure_epoch = self
            .jim_procedure_epoch
            .checked_add(1)
            .expect("Jim procedure epoch exhausted");
        self.jim_retired_commands = 0;
        self.jim_retired_tokens.clear();
    }

    pub(super) fn note_jim_command_retirement(&mut self, token: u64) {
        if self.variable_name_protocol != Some(tcl_syntax::naming::NativeNameProtocol::Jim084) {
            return;
        }
        self.note_retained_jim_command_retirement(token);
    }

    pub(super) fn note_retained_jim_command_retirement(&mut self, token: u64) {
        if self.jim_active_commands.contains_key(&token)
            || self
                .jim_previous_commands
                .values()
                .any(|node| node.generation == token)
            || !self.jim_retired_tokens.insert(token)
        {
            return;
        }
        self.jim_previous_locations.remove(&token);
        self.release_jim_previous(token);
        self.jim_retired_commands += 1;
        if self.jim_retired_commands >= 1000 {
            self.advance_jim_procedure_epoch();
        }
    }

    pub(crate) fn enter_jim_command(&mut self, token: u64) {
        *self.jim_active_commands.entry(token).or_default() += 1;
    }

    pub(crate) fn leave_jim_command(&mut self, token: u64) {
        let count = self
            .jim_active_commands
            .get_mut(&token)
            .expect("active Jim command lease");
        *count -= 1;
        if *count == 0 {
            self.jim_active_commands.remove(&token);
            if self.native_command_at_node(token).is_none() {
                self.note_retained_jim_command_retirement(token);
            }
        }
    }

    /// Current independently mutable command-reference context.
    pub(crate) fn native_command_reference(
        &self,
        ns: NsId,
    ) -> tcl_runtime_api::native_command_name::NativeCommandNameReference {
        tcl_runtime_api::native_command_name::NativeCommandNameReference {
            namespace_token: ns as u64,
            command_reference_epoch: self.arena[ns].command_reference_epoch,
        }
    }

    /// Observe an original node without retaining its callable implementation.
    pub(crate) fn native_command_target(
        &self,
        generation: u64,
    ) -> Option<tcl_runtime_api::native_command_name::NativeCommandNameTarget> {
        let node = self.native_command_nodes.get(&generation)?;
        let (ns, simple) = node.placement.as_ref()?;
        (self.arena[*ns].commands.generation(simple) == Some(generation)).then_some(
            tcl_runtime_api::native_command_name::NativeCommandNameTarget {
                token: generation,
                implementation_generation: generation,
                command_epoch: node.epoch,
                namespace_token: *ns as u64,
                namespace_dying: self.dead.contains(ns) || self.dying.contains(ns),
            },
        )
    }

    /// Actual current command holder/tail for one still-live command generation.
    pub(crate) fn native_command_slot_at_node(&self, generation: u64) -> Option<(NsId, Vec<u8>)> {
        let node = self.native_command_nodes.get(&generation)?;
        let (namespace, simple) = node.placement.as_ref()?;
        (self.arena[*namespace].commands.generation(simple) == Some(generation))
            .then(|| (*namespace, simple.clone()))
    }

    /// Borrow a live native command's actual simple table key without display parsing.
    pub(crate) fn native_command_simple_at_node(&self, generation: u64) -> Option<&[u8]> {
        let node = self.native_command_nodes.get(&generation)?;
        let (namespace, simple) = node.placement.as_ref()?;
        (self.arena[*namespace].commands.generation(simple) == Some(generation))
            .then_some(simple.as_slice())
    }

    /// Check the original published slot without retaining its callable payload.
    pub(crate) fn original_publication_is_current(
        &self,
        namespace: NsId,
        name: &[u8],
        generation: u64,
    ) -> bool {
        self.native_command_nodes
            .get(&generation)
            .is_some_and(|node| {
                node.placement.as_ref().is_some_and(|(holder, key)| {
                    *holder == namespace
                        && key.as_slice() == name
                        && self.arena[*holder].commands.generation(key) == Some(generation)
                })
            })
    }

    /// Join a genuine prepared Proc observation to the same original table node.
    pub(crate) fn original_procedure_publication_is_current(
        &self,
        namespace: NsId,
        name: &[u8],
        generation: u64,
        original: &crate::interp::OriginalProcedureBinding,
    ) -> bool {
        self.original_publication_is_current(namespace, name, generation)
            && self.arena[namespace].commands.get(name).is_some_and(|command| {
                matches!(command, Command::Proc(procedure) if procedure.observes_original_binding(original))
            })
    }

    /// Borrow Jim's independently retained active worker location.
    pub(super) fn retained_jim_command_at_node(
        &self,
        generation: u64,
    ) -> Option<(&Command, NsId, &[u8])> {
        let (namespace, simple) = self.jim_previous_locations.get(&generation)?;
        let binding = self
            .jim_previous_commands
            .values()
            .find(|binding| binding.generation == generation)?;
        Some((&binding.command, *namespace, simple))
    }

    /// Project the actual current node placement without imposing callable ownership.
    pub(super) fn native_command_at_node_using<T>(
        &self,
        generation: u64,
        project: impl Fn(&Command, NsId, &[u8]) -> T,
    ) -> Option<T> {
        let node = self.native_command_nodes.get(&generation)?;
        let (ns, simple) = node.placement.as_ref()?;
        (self.arena[*ns].commands.generation(simple) == Some(generation)).then(|| {
            project(
                self.arena[*ns]
                    .commands
                    .get(simple)
                    .expect("matching generation has a command"),
                *ns,
                simple,
            )
        })
    }

    /// Clone a live worker only after its actual node placement is checked.
    pub(crate) fn native_command_at_node(&self, generation: u64) -> Option<(Command, Vec<u8>)> {
        if let Some((command, namespace, simple)) = self.retained_jim_command_at_node(generation) {
            return Some((command.clone(), self.command_fqn(namespace, simple)));
        }
        self.native_command_at_node_using(generation, |command, namespace, simple| {
            (command.clone(), self.command_fqn(namespace, simple))
        })
    }

    /// Capture the actual selected registration and release-specific reference context.
    pub(crate) fn native_command_name_cache(
        &self,
        interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
        current: NsId,
        original: &[u8],
        protocol: tcl_native_model::native_command_literal::NativeCommandNameProtocol,
    ) -> Option<tcl_runtime_api::native_command_name::NativeCommandNameCache> {
        let (ns, simple) = self.home_of(current, original)?;
        let generation = self.arena[ns].commands.generation(&simple)?;
        let node = self.native_command_nodes.get(&generation)?;
        Some(
            tcl_runtime_api::native_command_name::NativeCommandNameCache {
                interpreter,
                version: protocol.version(),
                slot: tcl_core_types::NativeByteCommandSlot {
                    namespace: self.arena[ns].constructed_path.clone(),
                    simple: tcl_core_types::NameBytes::from(simple),
                },
                namespace_token: ns as u64,
                token: generation,
                implementation_generation: generation,
                command_epoch: node.epoch,
                reference: protocol.lookup_reference(
                    written_command_parts(original).absolute,
                    self.native_command_reference(current),
                    self.native_command_reference(GLOBAL),
                ),
            },
        )
    }

    /// Prime an already selected node without reparsing its reported full name.
    pub(crate) fn native_command_name_cache_from_binding(
        &self,
        interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
        binding: &tcl_runtime_api::native_compilation::NativeCompilationBinding,
        protocol: tcl_native_model::native_command_literal::NativeCommandNameProtocol,
    ) -> Option<tcl_runtime_api::native_command_name::NativeCommandNameCache> {
        let node = self.native_command_nodes.get(&binding.token)?;
        let (namespace, simple) = node.placement.as_ref()?;
        if *namespace as u64 != binding.namespace_token
            || self.arena[*namespace].commands.generation(simple) != Some(binding.token)
            || binding.implementation_generation != binding.token
            || self.arena[*namespace].constructed_path != binding.slot.namespace
            || simple.as_slice() != binding.slot.simple.as_bytes()
        {
            return None;
        }
        Some(
            tcl_runtime_api::native_command_name::NativeCommandNameCache {
                interpreter,
                version: protocol.version(),
                slot: binding.slot.clone(),
                namespace_token: binding.namespace_token,
                token: binding.token,
                implementation_generation: binding.implementation_generation,
                command_epoch: node.epoch,
                reference: None,
            },
        )
    }
}
