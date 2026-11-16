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

//! Command lookup and resolution: resolving a name from a namespace to its slot, in any interpreter, with the resolution cache epochs and the command kind.

use super::super::{CommandSlot, InterpState, NamespacePath, RetainedNamespace, Vm};
use super::{CommandSidecarKey, NativeCommandLookupUnavailable};
use crate::command::Command;
use tcl_core_types::NameBytes;
#[cfg(all(test, debug_assertions))]
use tcl_core_types::VarId;
use tcl_runtime_api::guard::GuardDomain;
use tcl_runtime_api::{NsId, ROOT_NS};

impl Vm {
    /// The `info cmdtype` kind of `name` (`native`/`proc`/`alias`), or `None`
    /// when there is no such command.
    pub(crate) fn command_kind(&self, name: &str) -> Option<&'static str> {
        self.command_kind_bytes_checked(name.as_bytes())
            .ok()
            .flatten()
    }

    /// Classify the command selected by its purpose-owned original bytes.
    pub(crate) fn command_kind_bytes_checked(
        &self,
        name: &[u8],
    ) -> Result<Option<&'static str>, NativeCommandLookupUnavailable> {
        Ok(self
            .lookup_command_bytes_checked(self.current_ns_id(), name)?
            .map(|(_, c)| match c {
                Command::Builtin(_) | Command::Native(_) => "native",
                Command::Proc(_) => "proc",
                // Cross-interp aliases are aliases (C's `info cmdtype` says
                // `alias` for every `interp alias` product).
                Command::Alias { .. } | Command::CallerAlias(_) | Command::CrossAlias { .. } => {
                    "alias"
                }
                Command::ChildInterp(_) => "interp",
                Command::Ensemble(_) => "ensemble",
                Command::Object(_) => "object",
            }))
    }

    pub(crate) fn lookup_command_bytes_checked(
        &self,
        context: NsId,
        name: &[u8],
    ) -> Result<Option<(String, Command)>, NativeCommandLookupUnavailable> {
        let Some(key) = self.resolve_command_bytes_checked(context, name, true)? else {
            return Ok(None);
        };
        Ok(self
            .visible_command_at_key(&key)
            .map(|command| (key, command)))
    }

    pub(crate) fn lookup_command(&self, name: &str) -> Option<Command> {
        self.lookup_command_in_namespace(self.current_ns(), name)
            .map(|(_, command)| command)
    }

    /// Resolve and retrieve a command in an explicit constructed namespace.
    /// Resolve a compiled source spelling through its retained native context.
    /// Invalid native ownership declines; presentation cannot select a substitute.
    pub(crate) fn resolve_compiled_binding_name(
        &self,
        legacy_namespace: &str,
        native_context: Option<&tcl_runtime_api::CompiledNamespaceContext>,
        name: &str,
    ) -> Option<String> {
        let Some(context) = native_context else {
            return self.resolve_command_fqn(legacy_namespace, name);
        };
        let token = self.resolve_compiled_namespace_context(context)?;
        self.resolve_command_from_token_bytes(token, name.as_bytes(), true)
    }

    /// Resolve an exact compiled namespace owner without its display spelling.
    pub(crate) fn resolve_compiled_namespace_context(
        &self,
        context: &tcl_runtime_api::CompiledNamespaceContext,
    ) -> Option<NsId> {
        let token = match context {
            tcl_runtime_api::CompiledNamespaceContext::Native(context) => {
                if context.interpreter != self.native_interpreter_identity() {
                    return None;
                }
                let token = NsId(u32::try_from(context.token).ok()?);
                if !self.native_namespace_token_has_owner(token)
                    || self.namespace_path_for_token(token) != context.path
                {
                    return None;
                }
                token
            }
            tcl_runtime_api::CompiledNamespaceContext::ConstructedPath(path) => {
                self.native_namespace_context_for_path(path).ok()?
            }
        };
        if !self.native_namespace_token_has_owner(token) {
            return None;
        }
        Some(token)
    }

    /// Resolve and capture a compiled binding in the same original namespace.
    pub(crate) fn lookup_compiled_command_binding(
        &self,
        binding: &tcl_runtime_api::CommandBindingIdentity,
    ) -> Option<(String, Command)> {
        let key = self.resolve_compiled_binding_name(
            &binding.resolution_namespace,
            binding.namespace_context.as_ref(),
            &binding.name,
        )?;
        let command = self.command_at_resolved_key(&binding.name, &key)?;
        self.builtin_command_visible_for_surface(&key, &command)
            .then_some((key, command))
    }

    /// Resolve and retrieve a command in an explicit constructed namespace.
    /// Returning the winning key alongside the callable keeps entered-command
    /// sidecars attached to the same namespace lookup result.
    pub(crate) fn lookup_command_in_namespace(
        &self,
        namespace: &str,
        name: &str,
    ) -> Option<(String, Command)> {
        let key = self.resolve_command_fqn(namespace, name)?;
        // A relative lookup from a retained namespace activation resolves in
        // that token's own command table; an absolute spelling still reaches
        // a same-named live recreation in the ordinary table.
        let command = self.command_at_resolved_key(name, &key)?;
        if !self.builtin_command_visible_for_surface(&key, &command) {
            return None;
        }
        Some((key, command))
    }

    /// Whether the exact visible command-table owner exists on the active
    /// release surface.  Unlike [`Self::lookup_command`], this never applies
    /// current-namespace or namespace-path resolution.
    pub(in super::super) fn visible_command_exists_exact(&self, key: &str) -> bool {
        self.commands
            .get(key)
            .is_some_and(|command| self.builtin_command_visible_for_surface(key, command))
    }
}

impl InterpState {
    pub(in super::super) fn ns_path(&self, id: NsId) -> NamespacePath {
        self.name_world
            .borrow()
            .ns_arena
            .get(id.0 as usize)
            .cloned()
            .unwrap_or_default()
    }

    /// Copy the exact slot so callers cannot retain a table borrow over callbacks.
    pub(in super::super) fn command_slot(&self, key: &str) -> Option<CommandSlot> {
        self.name_world
            .borrow()
            .command_identity
            .slot_by_key
            .get(key)
            .cloned()
    }

    /// Private storage is selected by the exact native slot, never by a display name.
    pub(in super::super) fn command_storage_key_at_slot(
        &self,
        slot: &CommandSlot,
    ) -> Option<String> {
        self.name_world
            .borrow()
            .command_identity
            .by_slot
            .get(slot)
            .cloned()
    }

    pub(in super::super) fn visible_command_generation(&self, key: &str) -> Option<u64> {
        let root = self.retained_root_for_visible_key(key);
        let world = self.name_world.borrow();
        root.and_then(|root| world.ns_deferral.retained.get(&root))
            .and_then(|record| record.generations.get(key))
            .copied()
            .or_else(|| world.command_identity.generations.get(key).copied())
    }

    /// Generation of a command in the live placement table, excluding retained tables.
    pub(in super::super) fn live_command_generation(&self, key: &str) -> Option<u64> {
        self.name_world
            .borrow()
            .command_identity
            .generations
            .get(key)
            .copied()
    }

    /// Exact reporting bytes for a private command storage key. The original
    /// written argument and constructed slot remain independent of this view.
    pub(crate) fn command_display_key_bytes(&self, key: &str) -> NameBytes {
        self.name_world
            .borrow()
            .command_identity
            .display_by_key
            .get(key)
            .cloned()
            .expect("a registered command has an exact native display name")
    }

    /// Checked Unicode compatibility view of an exact native display key.
    /// Native command consumers use `command_display_key_bytes` instead.
    pub(crate) fn command_display_key(&self, key: &str) -> String {
        self.command_display_key_bytes(key)
            .try_utf8()
            .expect("Unicode command display requires a checked native name")
            .to_owned()
    }

    /// Owned native reporting bytes, selected by the actual visible or hidden placement.
    pub(crate) fn command_sidecar_display_bytes(&self, key: &CommandSidecarKey) -> NameBytes {
        match key {
            CommandSidecarKey::Visible(name) => self.command_display_key_bytes(name),
            CommandSidecarKey::Hidden(name) => self
                .hidden
                .names_by_key
                .get(name)
                .cloned()
                .expect("hidden sidecar retains exact reporting bytes"),
        }
    }

    /// Root exact reporting bytes without interpreting the private placement key.
    pub(crate) fn rooted_command_sidecar_display_bytes(&self, key: &CommandSidecarKey) -> Vec<u8> {
        tcl_syntax::naming::root_unrooted_key_bytes(
            self.command_sidecar_display_bytes(key).as_bytes(),
        )
    }

    /// Invalidate the command-resolution memo (M16.4): every command-table /
    /// `namespace path` / runtime-version mutation routes through here so a
    /// cached `(namespace, name) → key` can never outlive the state it was
    /// computed from.
    pub(crate) fn advance_binding_epoch(&self) {
        let epochs = &self.compilation_epochs;
        epochs
            .binding_epoch
            .set(epochs.binding_epoch.get().wrapping_add(1));
    }

    pub(crate) fn bump_cmd_epoch(&self) {
        self.advance_binding_epoch();
        // Clearing on every mutation makes the resolution memo safe even if its
        // diagnostic epoch saturates. Guard epochs use their own checked,
        // poison-on-exhaustion counters.
        self.compilation_epochs
            .cmd_resolve_cache
            .borrow_mut()
            .1
            .clear();
        self.compilation_epochs
            .cmd_epoch
            .set(self.compilation_epochs.cmd_epoch.get().saturating_add(1));
    }

    /// Inspect actual variable operation ownership without issuing a storage role.
    #[cfg(all(test, debug_assertions))]
    pub(crate) fn has_variable_operation_refs_for_test(&self, id: VarId) -> bool {
        self.var_arena.has_operation_refs(id)
    }

    /// Invalidate the guard domains that depend on the command lookup
    /// environment: namespace paths and interpreter topology.
    ///
    /// A command-table mutation does not come here. What a guard needs of its
    /// command is decided when it is checked, by resolving the guarded name to
    /// a token generation and finding an attestation there
    /// ([`Vm::attested_identities`]), so replacing, deleting, renaming, hiding,
    /// or aliasing one command invalidates that command's guards and no other's.
    pub(crate) fn invalidate_lookup_guards(&self) {
        let mut guards = self.guards.borrow_mut();
        guards.invalidate(GuardDomain::CommandEnvironment);
        guards.invalidate(GuardDomain::Namespace);
        guards.invalidate(GuardDomain::UnknownHandling);
    }

    /// Namespace creation and deletion, exports, imports and unknown
    /// handlers change what a name reaches without touching the command
    /// lookup policy: they move `Namespace` and `UnknownHandling`, as the
    /// runtime's lookup-environment revision does.
    pub(crate) fn invalidate_namespace_structure_guards(&self) {
        let mut guards = self.guards.borrow_mut();
        guards.invalidate(GuardDomain::Namespace);
        guards.invalidate(GuardDomain::UnknownHandling);
    }

    /// Resolve `name` from namespace `cxt` to its command's canonical key
    /// (the `commands` map key — a qualified name without the leading `::`),
    /// via the shared C-Tcl resolution rule
    /// ([`tcl_syntax::naming::resolve_command_with`]): an absolute `::name`
    /// directly, else `cxt`'s candidate, then each of `cxt`'s
    /// `namespace path` entries in order, then global — dispatching the
    /// first that **exists**.  `None` if unresolved.
    pub(crate) fn resolve_command_fqn(&self, cxt: &str, name: &str) -> Option<String> {
        let cxt_id = if cxt == self.current_ns() {
            self.current_ns_id()
        } else {
            let path = NamespacePath::from_segments(tcl_syntax::naming::key_segments(cxt));
            self.name_world
                .borrow()
                .ns_intern
                .get(&path)
                .copied()
                .unwrap_or(ROOT_NS)
        };
        self.resolve_command_from_token(cxt_id, name, true)
    }

    fn resolve_command_from_token(
        &self,
        cxt: NsId,
        name: &str,
        filter_public_surface: bool,
    ) -> Option<String> {
        self.resolve_command_from_token_bytes(cxt, name.as_bytes(), filter_public_surface)
    }

    pub(in super::super) fn resolve_command_from_token_bytes(
        &self,
        cxt: NsId,
        original: &[u8],
        filter_public_surface: bool,
    ) -> Option<String> {
        if self.retained_record_of(cxt).is_some()
            || !self
                .name_world
                .borrow()
                .pending_publications_are_committed()
        {
            return self.resolve_command_bytes_uncached(cxt, original, filter_public_surface);
        }
        let epoch = self.compilation_epochs.cmd_epoch.get();
        let memo_key = (cxt, original.into(), filter_public_surface);
        {
            let cache = self.compilation_epochs.cmd_resolve_cache.borrow();
            if cache.0 == epoch
                && let Some(hit) = cache.1.get(&memo_key)
            {
                return hit.clone();
            }
        }
        let result = self.resolve_command_bytes_uncached(cxt, original, filter_public_surface);
        let mut cache = self.compilation_epochs.cmd_resolve_cache.borrow_mut();
        if cache.0 != epoch {
            cache.0 = epoch;
            cache.1.clear();
        }
        if cache.1.len() >= 4096 {
            cache.1.clear();
        }
        cache.1.insert(memo_key, result.clone());
        result
    }

    /// Resolve a registered command for VM/embedder teardown without applying
    /// the emulated release's public command-surface filter.
    ///
    /// This deliberately bypasses only availability filtering. Namespace,
    /// namespace-path, and global candidate ordering remain the active Tcl
    /// release's resolution rules. Never use this for Tcl-visible dispatch.
    pub(in super::super) fn resolve_command_fqn_raw(
        &self,
        cxt: &str,
        name: &str,
    ) -> Option<String> {
        let cxt_id = if cxt == self.current_ns() {
            self.current_ns_id()
        } else {
            let path = NamespacePath::from_segments(tcl_syntax::naming::key_segments(cxt));
            self.name_world
                .borrow()
                .ns_intern
                .get(&path)
                .copied()
                .unwrap_or(ROOT_NS)
        };
        // Public dispatch and embedder teardown deliberately see different
        // release-filtered surfaces. The public memo is keyed only by context
        // and spelling, so a raw lookup must not reuse or populate it.
        self.resolve_command_fqn_uncached(cxt_id, name, false)
    }

    /// The record retaining `id`, if any — `id` may be the retained token
    /// itself or any namespace below it.
    pub(in super::super) fn retained_record_of(
        &self,
        id: NsId,
    ) -> Option<std::cell::Ref<'_, RetainedNamespace>> {
        let world = self.name_world.borrow();
        let root = *world.ns_deferral.owners.get(&id)?;
        std::cell::Ref::filter_map(world, |world| world.ns_deferral.retained.get(&root)).ok()
    }

    pub(in super::super) fn retained_current_record(
        &self,
    ) -> Option<std::cell::Ref<'_, RetainedNamespace>> {
        self.retained_record_of(
            self.resolution_stacks
                .ns_id_stack
                .last()?
                .resolved_token()?,
        )
    }

    /// Walk exact namespace arena edges below `base`. Namespace-path entries
    /// retain their target token, so resolution must not render and re-look up
    /// that token after its public spelling has been unpublished.
    pub(in super::super) fn namespace_descendant_token(
        &self,
        base: NsId,
        qualifiers: &[NameBytes],
    ) -> Option<NsId> {
        let world = self.name_world.borrow();
        let mut current = base;
        for tail in qualifiers {
            current = world.namespace_child_token(current, tail.as_bytes())?;
        }
        Some(current)
    }

    /// The record owning namespace `name`, when the current frame holds a token
    /// whose retained subtree contains it. Definitions and command-table
    /// bookkeeping for such a name belong to that record: the public spelling
    /// may already have been taken by a recreation.
    pub(in super::super) fn retained_owner_of_namespace(
        &self,
        path: &NamespacePath,
    ) -> Option<NsId> {
        let record = self.retained_current_record()?;
        record.subtree.contains_key(path).then_some(record.root)
    }

    /// Resolve the complete materialised operand under its selected native
    /// purpose, then traverse retained namespace tokens and ordered fallbacks.
    fn resolve_command_fqn_uncached(
        &self,
        cxt: NsId,
        name: &str,
        filter_public_surface: bool,
    ) -> Option<String> {
        self.resolve_command_bytes_uncached(cxt, name.as_bytes(), filter_public_surface)
    }

    /// Retained record selected by an exact visible command slot. The private
    /// key is only an index into [`NativeNameWorld`]; namespace ownership
    /// always comes from its structured slot.
    pub(in super::super) fn retained_root_for_visible_key(&self, key: &str) -> Option<NsId> {
        let namespace = self.command_slot(key)?.namespace;
        self.name_world
            .borrow()
            .ns_deferral
            .owners
            .get(&namespace)
            .copied()
    }

    pub(in super::super) fn visible_command_at_key(&self, key: &str) -> Option<Command> {
        let root = self.retained_root_for_visible_key(key);
        let retained = root.and_then(|root| {
            self.name_world
                .borrow()
                .ns_deferral
                .retained
                .get(&root)
                .and_then(|record| record.commands.get(key))
                .cloned()
        });
        retained.or_else(|| self.commands.get(key).cloned())
    }

    pub(in super::super) fn command_at_exact_slot(
        &self,
        slot: &CommandSlot,
        filter_public_surface: bool,
    ) -> Option<String> {
        self.command_at_exact_slot_checked(slot, filter_public_surface)
            .ok()
            .flatten()
    }

    pub(crate) fn command_at_exact_slot_checked(
        &self,
        slot: &CommandSlot,
        filter_public_surface: bool,
    ) -> Result<Option<String>, NativeCommandLookupUnavailable> {
        let pending = self.name_world.borrow().pending_slot_presence(slot);
        match pending {
            Some(true) => {
                return Err(
                    NativeCommandLookupUnavailable::PendingImplementationUnavailable(slot.clone()),
                );
            }
            Some(false) => return Ok(None),
            None => {}
        }
        let Some(key) = self.command_storage_key_at_slot(slot) else {
            return Ok(None);
        };
        let retained = self
            .retained_record_of(slot.namespace)
            .and_then(|record| record.commands.get(&key).cloned());
        let Some(command) = retained.or_else(|| self.commands.get(&key).cloned()) else {
            return Ok(None);
        };
        Ok(
            (!filter_public_surface || self.builtin_command_visible_for_surface(&key, &command))
                .then_some(key),
        )
    }

    fn resolve_command_bytes_uncached(
        &self,
        cxt: NsId,
        original: &[u8],
        filter_public_surface: bool,
    ) -> Option<String> {
        self.resolve_command_bytes_checked(cxt, original, filter_public_surface)
            .ok()
            .flatten()
    }

    pub(crate) fn resolve_command_bytes_checked(
        &self,
        cxt: NsId,
        original: &[u8],
        filter_public_surface: bool,
    ) -> Result<Option<String>, NativeCommandLookupUnavailable> {
        use tcl_syntax::naming::NativeNameContext;
        if !self.name_world.borrow().is_live() {
            return Err(NativeCommandLookupUnavailable::Retired);
        }
        let protocol = self
            .name_policy_protocol()
            .ok_or(NativeCommandLookupUnavailable::ProtocolUnavailable)?
            .recipe();
        let path = self.ns_path(cxt);
        if protocol.is_jim084() {
            let namespace_object = self
                .name_world
                .borrow()
                .jim_namespace_objects
                .get(&cxt)
                .cloned()
                .ok_or(NativeCommandLookupUnavailable::NamespaceContextUnavailable)?;
            let context = NativeNameContext::with_jim_namespace(&path, namespace_object.as_bytes());
            for simple in protocol
                .jim_command_lookup_keys(context, original)
                .map_err(|_| NativeCommandLookupUnavailable::NamespaceContextUnavailable)?
            {
                let slot = CommandSlot {
                    namespace: ROOT_NS,
                    simple,
                };
                if let Some(key) =
                    self.command_at_exact_slot_checked(&slot, filter_public_surface)?
                {
                    return Ok(Some(key));
                }
            }
            return Ok(None);
        }
        let projected = protocol
            .command_lookup_projection(NativeNameContext::new(&path), original)
            .map_err(|_| NativeCommandLookupUnavailable::NamespaceContextUnavailable)?;
        let simple = projected.slot().simple.clone();
        let qualifiers = projected.qualifiers();
        // Copying path IDs reaches no namespace or callback; the shared iterator
        // selects which retained tokens lookup actually visits.
        let paths = self
            .retained_record_of(cxt)
            .and_then(|record| record.paths.get(&cxt).cloned())
            .or_else(|| self.name_world.borrow().ns_paths.get(&cxt).cloned())
            .unwrap_or_default();
        let bases = projected
            .lookup_bases(cxt, &paths)
            .map_err(|_| NativeCommandLookupUnavailable::NamespaceContextUnavailable)?;
        for base in bases {
            let base = match base {
                tcl_syntax::naming::CommandLookupBase::Context(token) => token,
                tcl_syntax::naming::CommandLookupBase::Root => ROOT_NS,
            };
            let Some(namespace) = self.namespace_descendant_token(base, qualifiers) else {
                continue;
            };
            let slot = CommandSlot {
                namespace,
                simple: simple.clone(),
            };
            if let Some(key) = self.command_at_exact_slot_checked(&slot, filter_public_surface)? {
                return Ok(Some(key));
            }
        }
        Ok(None)
    }
}
