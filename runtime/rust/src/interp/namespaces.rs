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

//! The interpreter's side of the namespace tree: the current namespace, activations, `namespace eval` frames, creating namespaces and tearing them down.

use super::commands::Command;
use super::{native_operation_currency, new_string, CmdFrame, Code, FrameKind, Interp};
use crate::namespace::{Namespaces, NsId, GLOBAL};
use crate::obj;
use crate::obj::TclObj;
use std::rc::Rc;

impl Interp {
    /// The current namespace (the eval context) — for the `namespace` builtin.
    pub(crate) fn current_ns(&self) -> NsId {
        self.current_ns.get()
    }

    /// Set the current namespace without changing the selected variable frame.
    /// The caller owns frame selection and namespace restoration.
    pub(crate) fn set_current_ns(&self, ns: NsId) {
        self.current_ns.set(ns);
    }

    /// Record the activation `Tcl_PushCallFrame` adds to the namespace token a
    /// new call frame runs in. Every frame counts — proc, `apply`, TclOO
    /// method, `namespace eval`/`inscope` — and a namespace deleted while the
    /// count is non-zero keeps its contents until the matching pop.
    pub(crate) fn enter_namespace_activation(&self, ns: NsId) {
        self.namespaces.borrow_mut().activation_enter(ns);
    }

    /// Give back the activation a popped frame held. When it was the last one
    /// holding a namespace whose deletion waited on it, the teardown runs here,
    /// exactly as C's `Tcl_PopCallFrame` calls `Tcl_DeleteNamespace` again —
    /// with the frame already gone and the caller's namespace current.
    pub(crate) fn leave_namespace_activation(&mut self, popped: Option<NsId>) {
        let Some(ns) = popped else { return };
        if self.namespaces.borrow_mut().activation_leave(ns) {
            self.delete_namespace_by_id(ns);
        }
    }

    /// The namespace tree (mutable) — for the `namespace` builtin's mutations
    /// (`export`/`import`/`forget`/`path`).
    pub(crate) fn namespaces_mut(&self) -> std::cell::RefMut<'_, Namespaces> {
        // The actual mutators record lookup-policy revisions; an ordinary
        // command publication must not invalidate unrelated command guards.
        self.namespaces.borrow_mut()
    }

    /// Shared `namespace eval` core: enter `name`, push a namespace var-scope
    /// frame *and* a `CmdFrame` for the body (C's `namespace eval` is its own
    /// `info frame` level — depth and `info level` both advance — with `proc`
    /// cleared and `level` reported relative to the new scope). `loc` is the
    /// body's TIP 280 location when it is a located literal (`type source`),
    /// else `None` (`type eval`). A `list` body is that canonical list, one
    /// command dispatched directly (`TclEvalObjEx`).
    pub(super) fn ns_eval_framed(
        &mut self,
        name: &[u8],
        body: &[u8],
        loc: Option<(Option<Rc<[u8]>>, u32)>,
        add_eval_frame: bool,
        original_body: Option<(
            tcl_registry::native_eval_object::EvalObjectPurpose,
            *mut TclObj,
        )>,
        original_arguments: Option<&[*mut TclObj]>,
    ) -> Code {
        if let Err(first) = self.clear_return_options_checked() {
            return self.refuse_native_execution(first);
        }
        let target = if self
            .name_policy_protocol()
            .is_some_and(|protocol| protocol.recipe().is_jim084())
        {
            let original = crate::obj::Owned::fresh(crate::obj::new_string_bytes(name));
            let namespace = match self.jim_current_namespace_object() {
                Ok(namespace) => namespace,
                Err(error) => return self.report_cmd_error(error.into()),
            };
            let canonical = match self.jim_canonical_namespace_object(&namespace, original.as_ptr())
            {
                Ok(canonical) => canonical,
                Err(error) => return self.report_cmd_error(error.into()),
            };
            let bytes =
                match tcl_syntax::value::ValueOps::native_string_bytes(self, &canonical.as_ptr()) {
                    Ok(bytes) => bytes,
                    Err(error) => return self.report_cmd_error(error.into()),
                };
            self.namespaces_mut().retain_jim_namespace(canonical, bytes)
        } else {
            match self.publish_namespace_address(name) {
                Ok(namespace) => namespace,
                Err(error) => return self.report_cmd_error(error),
            }
        };
        self.ns_eval_in_token(
            target,
            body,
            loc,
            add_eval_frame,
            original_body,
            original_arguments,
        )
    }

    pub(crate) fn ns_eval_in_token(
        &mut self,
        target: NsId,
        body: &[u8],
        loc: Option<(Option<Rc<[u8]>>, u32)>,
        add_eval_frame: bool,
        original_body: Option<(
            tcl_registry::native_eval_object::EvalObjectPurpose,
            *mut TclObj,
        )>,
        original_arguments: Option<&[*mut TclObj]>,
    ) -> Code {
        let saved = self.current_ns.get();
        self.current_ns.set(target);
        // A namespace frame: a new scope whose unqualified vars resolve to the
        // namespace (so `set`/`variable`/`upvar 0` inside `namespace eval` —
        // including when nested in a proc — target the namespace, not the
        // enclosing proc's locals).
        self.enter_namespace_activation(target);
        self.frames.borrow_mut().push_namespace(target);
        if let Some(arguments) = original_arguments {
            let installed = self
                .frames
                .borrow_mut()
                .install_original_error_stack_argv(arguments);
            if let Err(error) = installed {
                let popped = self.pop_native_call_frame();
                self.current_ns.set(saved);
                self.leave_namespace_activation(popped);
                return self.report_cmd_error(error.into());
            }
        }
        if let Err(error) = self.retain_native_jim_frame_namespace(target) {
            let popped = self.pop_native_call_frame();
            self.current_ns.set(saved);
            self.leave_namespace_activation(popped);
            return self.report_cmd_error(error.into());
        }
        let jim_body = if self.invocation_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            Some(match original_body {
                Some((_, original)) => obj::Owned::retain(original),
                None => obj::Owned::fresh(new_string(body)),
            })
        } else {
            None
        };
        let jim_body_pointer = jim_body.as_ref().map(obj::Owned::as_ptr);
        if let Some(original) = jim_body {
            self.frames.borrow_mut().retain_jim_frame_body(original);
        }
        let (kind, file, line_base) = match loc {
            Some((file, line)) => (FrameKind::Source, file, line.saturating_sub(1)),
            None => (FrameKind::Eval, None, 0),
        };
        let (ns_level, ns_index) = {
            let f = self.frames.borrow();
            (f.current_level(), f.current_frame_index())
        };
        let frame = CmdFrame {
            kind,
            file,
            proc: None,
            level: ns_level,
            omit_level: false,
            frame_index: ns_index,
            line_base,
            proc_line_base: line_base,
            cmd: Vec::new(),
            original_command: None,
            line: 1,
            oo: None,
            lambda: None,
        };
        let code = match original_body {
            Some((purpose, original)) => self.eval_original_body_framed(purpose, original, frame),
            None => match jim_body_pointer {
                Some(original) => self.eval_original_body_framed(
                    tcl_registry::native_eval_object::EvalObjectPurpose::NamespaceBody,
                    original,
                    frame,
                ),
                None => self.eval_framed(body, frame),
            },
        };
        if code == Code::Error && add_eval_frame && !self.uses_jim_error_stack() {
            // `(in namespace eval "::ns" script line N)` — the body's own frame.
            let fqn = self.namespaces.borrow().qualified_name(target);
            self.append_namespace_eval_frame(self.namespace_frame_command.get(), &fqn);
        }
        self.clean_current_jim_local_commands();
        let popped = self.pop_native_call_frame();
        self.current_ns.set(saved);
        self.leave_namespace_activation(popped);
        code
    }

    /// Run `f`, whose namespace body's error frame names `command`
    /// (`namespace inscope`) rather than `eval`.
    pub(crate) fn with_namespace_frame_command<T>(
        &mut self,
        command: &'static str,
        f: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let outer = self.namespace_frame_command.replace(command);
        let result = f(self);
        self.namespace_frame_command.set(outer);
        result
    }

    /// Report an existing namespace only after the selected byte-address lookup.
    pub(crate) fn resolve_namespace_name(
        &self,
        name: &[u8],
    ) -> Result<Option<Vec<u8>>, tcl_syntax::value::ValueError> {
        let current = u32::try_from(self.current_ns()).map_err(|_| {
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("namespace context width")
        })?;
        let selected = tcl_runtime_api::Namespaces::find_namespace_bytes_checked(
            self,
            tcl_runtime_api::NsId(current),
            name,
        )?;
        Ok(selected.map(|namespace| self.namespaces().qualified_name(namespace.0 as usize)))
    }

    /// Resolve (creating if needed) a namespace by name, relative to the current
    /// namespace — for `apply`'s optional namespace term.
    pub(crate) fn ensure_namespace(&mut self, name: &[u8]) -> NsId {
        self.invalidate_namespace_structure_guards();
        self.namespaces
            .borrow_mut()
            .ensure_namespace(self.current_ns.get(), name)
    }

    /// Create an OO command's default instance namespace under the exact
    /// namespace token that will receive the command. This differs from public
    /// `namespace eval` creation only during a synchronous delete callback,
    /// where the command and its owned namespace must both join the dying
    /// generation.
    pub(crate) fn ensure_command_owned_namespace(&mut self, name: &[u8]) -> NsId {
        self.namespaces
            .borrow_mut()
            .ensure_command_owned_namespace(self.current_ns.get(), name)
    }

    /// Create an internal test namespace from the global token.
    #[cfg(test)]
    pub(crate) fn ensure_global_namespace(&mut self, name: &[u8]) -> NsId {
        self.invalidate_namespace_structure_guards();
        self.namespaces.borrow_mut().ensure_namespace(GLOBAL, name)
    }

    /// Delete the namespace `ns` (by id), e.g. an OO object's instance namespace
    /// when the object is destroyed.
    pub(crate) fn delete_namespace_by_id(&mut self, ns: NsId) {
        let recipe = self.original_command_deletion_recipe();
        if let Some(recipe) = recipe {
            self.original_command_retirement_recipes
                .borrow_mut()
                .push(recipe);
        }
        self.delete_namespace_by_id_using(ns);
        if recipe.is_some() {
            let _ = self.original_command_retirement_recipes.borrow_mut().pop();
        }
    }

    fn delete_namespace_by_id_using(&mut self, ns: NsId) {
        // The reached namespace cleanup keeps its original receipt through
        // every extracted table/key release, including a refused entry.
        let saved = native_operation_currency::SavedNativeOperationCurrency::capture(self);
        let cleanup = match native_operation_currency::NativeOperationScope::enter_saved_cleanup(
            &saved, self,
        ) {
            Ok(cleanup) => cleanup,
            Err(first) => {
                self.refuse_native_execution(first);
                return;
            }
        };
        self.invalidate_namespace_structure_guards();
        if self.defer_namespace_teardown(ns) {
            let _ = cleanup.check();
            return;
        }
        let teardown_ids = self.namespaces.borrow().descendant_ids(ns);
        // TclOO state follows the exact namespace token. This must happen only
        // once deletion is no longer deferred: an active old token can coexist
        // with fresh objects at the same Tcl-facing name. Descendants receive
        // the same check when recursion reaches their own token.
        self.oo_namespace_deleted(ns);
        let native_error_reset = ns == GLOBAL && !self.native_error_cells.borrow().is_empty();
        if ns == GLOBAL {
            self.native_error_cells.borrow_mut().clear();
        }
        self.delete_namespace_token(ns);
        self.sweep_dying_namespace(ns, &teardown_ids);
        self.namespaces
            .borrow_mut()
            .finish_namespace_teardown(&teardown_ids);
        if cleanup.check().is_ok() && native_error_reset {
            self.install_native_error_variable_traces();
        }
        let _ = cleanup.check();
    }

    /// C's `activationCount > (nsPtr == globalNsPtr)` branch of
    /// `Tcl_DeleteNamespace`: a token a call frame is still running in retires
    /// its owned ensembles and drops its public name now, but keeps its command
    /// table, variables and children for those frames. `Tcl_PopCallFrame` calls
    /// the deletion again once the last of them goes away. Reports whether the
    /// teardown was deferred.
    fn defer_namespace_teardown(&mut self, ns: NsId) -> bool {
        if !self.namespaces.borrow().namespace_is_active(ns) {
            return false;
        }
        self.retire_namespace_owned_ensembles(ns);
        self.retire_namespace_unknown_root(ns);
        let retired = self.namespaces.borrow_mut().defer_namespace(ns);
        drop(retired);
        true
    }

    /// The same token teardown, entered the way C's
    /// `TclDeleteNamespaceChildren` enters it — through `Tcl_DeleteNamespace`,
    /// so a child with its own live frame defers in turn.
    fn delete_namespace_token_checked(&mut self, ns: NsId) {
        if self.defer_namespace_teardown(ns) {
            return;
        }
        self.oo_namespace_deleted(ns);
        self.delete_namespace_token(ns);
    }

    /// Retire the ensemble tokens configured against `ns`, and the imports of
    /// the commands they were bound to. C pops `nsPtr->ensembles` before it
    /// looks at the activation count, so an owned ensemble dies immediately
    /// even when the namespace itself is retained.
    pub(super) fn retire_namespace_owned_ensembles(&mut self, ns: NsId) {
        let ids = std::collections::HashSet::from([ns]);
        let mut deleted_origins = std::collections::HashSet::<u64>::new();
        let mut ensemble_victims = self.namespaces.borrow().ensembles_for(&ids);
        let hidden_tokens: Vec<(Vec<u8>, Rc<crate::ensemble::EnsembleToken>)> = self
            .hidden
            .borrow()
            .iter()
            .filter_map(|(name, binding)| match &binding.command {
                Command::Ensemble(token) if ids.contains(&token.config().ns) => {
                    let fqn = tcl_syntax::naming::root_unrooted_key_bytes(name);
                    Some((fqn, Rc::clone(token)))
                }
                _ => None,
            })
            .collect();
        ensemble_victims.extend(hidden_tokens);
        let mut deleted_tokens = Vec::with_capacity(ensemble_victims.len());
        for (_fqn, token) in ensemble_victims {
            if deleted_tokens.iter().any(|seen| Rc::ptr_eq(seen, &token)) {
                continue;
            }
            if let Some((_, generation)) = self.ensemble_identity_location(&token) {
                deleted_origins.insert(generation);
            }
            if let Some((_, generation)) = self.retire_ensemble_identity(&token) {
                deleted_origins.insert(generation);
            }
            deleted_tokens.push(token);
        }
        self.remove_imports_for_deleted_origins(deleted_origins, &deleted_tokens);
    }

    /// Tear down one exact namespace token in Tcl's recursive order. Its owned
    /// ensembles retire while this token and all children are live; then only
    /// this token becomes dying and loses its ordinary command table. Children
    /// receive the same lifecycle recursively after the parent's callbacks.
    fn delete_namespace_token(&mut self, ns: NsId) {
        if self
            .require_variable_name_protocol()
            .is_ok_and(|protocol| protocol.is_jim084())
        {
            let qualified = self.namespaces.borrow().qualified_name(ns);
            let key = tcl_syntax::naming::jim_global_variable_key_bytes(b"", &qualified);
            let prefix = (!key.is_empty()).then(|| [key.as_slice(), b"::"].concat());
            let names = self.namespaces.borrow().var_names(GLOBAL);
            for name in names.into_iter().filter(|name| {
                prefix
                    .as_ref()
                    .is_none_or(|prefix| name.starts_with(prefix))
            }) {
                let absolute = tcl_syntax::naming::root_unrooted_key_bytes(&name);
                self.var_unset(&absolute);
            }
        }
        self.retire_namespace_owned_ensembles(ns);
        self.retire_namespace_unknown_root(ns);
        // Keep this token's original variable path during its callbacks. Its
        // public namespace identity is dying; the parent edge retires afterwards.
        self.namespaces
            .borrow_mut()
            .begin_namespace_variable_teardown(ns);
        self.retire_namespace_variables(ns);
        self.namespaces.borrow_mut().begin_namespace_teardown(ns);
        let retired = self.namespaces.borrow_mut().clear_namespace_token(ns);
        drop(retired);
        let _ = self.check_entered_native_operation();
        self.tear_down_command_table(ns);
        self.namespaces
            .borrow_mut()
            .finish_native_namespace_command_path(ns);

        // Tcl snapshots and recursively deletes children only after this
        // token's ordinary command callbacks have completed. A callback may
        // already have deleted one; skip any token no longer publicly live.
        let children = self.namespaces.borrow().children_hash_order(ns);
        for child in children {
            if self.namespaces.borrow().namespace_is_live(child) {
                self.delete_namespace_token_checked(child);
            }
        }
        self.namespaces
            .borrow_mut()
            .namespace_name_finish_deletion(ns);
    }

    /// Delete one dying namespace's command table in its selected C traversal:
    /// Tcl 8.4/8.5 select the first entry again after each command; Tcl 8.6/9
    /// retain each complete table frontier until that deletion pass finishes.
    ///
    /// Each token's `delete` traces fire while its entry is still in the table
    /// (`Tcl_DeleteCommandFromToken` calls `CallCommandTraces` before
    /// `Tcl_DeleteHashEntry`), and its imports retire depth-first straight
    /// after — not in a bulk pass over the whole namespace. A callback that
    /// deletes or redefines a snapshotted entry changes its generation; C's
    /// `CMD_DYING` early return then leaves the replacement to the next
    /// snapshot.
    fn tear_down_command_table(&mut self, ns: NsId) {
        if self.uses_original_c_command_retirement() {
            self.retire_original_namespace_command_table(ns);
            return;
        }

        // Native proof: naming.namespace.original-command-holder-routing
        // docs/design/analysis/name-resolution-proofs/namespace-original-command-holder-routing.md
        // The public callback rows and pinned C teardown traversal stay separate.
        let traversal = self
            .name_policy_protocol()
            .and_then(|policy| policy.recipe().namespace_command_teardown());
        loop {
            let mut snapshot = self.namespaces.borrow().command_hash_order(ns);
            if let Some(traversal) = traversal {
                snapshot.truncate(traversal.frontier_len(snapshot.len()));
            }
            if snapshot.is_empty() {
                break;
            }
            let mut retired_any = false;
            for (tail, generation) in snapshot {
                if self.namespaces.borrow().command_generation(ns, &tail) != Some(generation) {
                    continue;
                }
                if self.check_entered_native_operation().is_ok() {
                    self.on_bound_command_replaced(ns, &tail);
                }
                if self.namespaces.borrow().command_generation(ns, &tail) != Some(generation) {
                    // The callback deleted this token, or redefined the name:
                    // its own deletion already unlinked the entry, and any
                    // replacement is a distinct token for the next snapshot.
                    retired_any = true;
                    continue;
                }
                let command = self.namespaces.borrow().command_in(ns, &tail);
                let ensemble_tokens = match &command {
                    Some(Command::Ensemble(token)) => vec![Rc::clone(token)],
                    _ => Vec::new(),
                };
                let oo_owner = command.as_ref().and_then(Command::oo_object);
                self.namespaces.borrow_mut().remove_in(ns, &tail);
                drop(command);
                if let Err(first) = self.retire_pending_native_ensemble_roles() {
                    self.refuse_native_execution(first);
                }
                self.remove_imports_for_deleted_origins([generation], &ensemble_tokens);
                if let Some(owner) = oo_owner {
                    self.oo_command_renamed(owner, None);
                }
                retired_any = true;
            }
            if !retired_any {
                break;
            }
        }
    }

    /// `root`'s subtree with every retained token's subtree stepped over — the
    /// nodes this teardown still owns.
    pub(super) fn retained_teardown_ids(&self, root: NsId) -> Vec<NsId> {
        let namespaces = self.namespaces.borrow();
        namespaces
            .descendant_ids(root)
            .into_iter()
            .filter(|id| !namespaces.under_deferred_token(*id))
            .collect()
    }

    /// Finish commands created re-entrantly while a namespace's original
    /// delete callbacks ran. The detached namespace remains command-addressable
    /// during this sweep, but never reappears in the visible namespace tree.
    fn sweep_dying_namespace(&mut self, root: NsId, retained_ids: &[NsId]) {
        if self.uses_original_c_command_retirement() {
            self.sweep_original_dying_namespace(root, retained_ids);
            return;
        }

        // A child whose own frame was still running deferred its teardown; it
        // and its subtree keep every binding they had until that frame pops.
        let mut ids: Vec<NsId> = {
            let namespaces = self.namespaces.borrow();
            retained_ids
                .iter()
                .copied()
                .filter(|id| !namespaces.under_deferred_token(*id))
                .collect()
        };
        let mut traced = std::collections::HashSet::<(NsId, Vec<u8>)>::new();
        let mut origins = std::collections::HashSet::<u64>::new();
        let mut tokens = Vec::<Rc<crate::ensemble::EnsembleToken>>::new();

        loop {
            for id in self.retained_teardown_ids(root) {
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
            let id_set: std::collections::HashSet<NsId> = ids.iter().copied().collect();

            let mut ensemble_victims = self.namespaces.borrow().ensembles_for(&id_set);
            ensemble_victims.extend(self.hidden.borrow().iter().filter_map(|(name, binding)| {
                let Command::Ensemble(token) = &binding.command else {
                    return None;
                };
                if !id_set.contains(&token.config().ns)
                    || tokens.iter().any(|seen| Rc::ptr_eq(seen, token))
                {
                    return None;
                }
                let fqn = tcl_syntax::naming::root_unrooted_key_bytes(name);
                Some((fqn, Rc::clone(token)))
            }));
            let mut found_new_token = false;
            for (_fqn, token) in ensemble_victims {
                if tokens.iter().any(|seen| Rc::ptr_eq(seen, &token)) {
                    continue;
                }
                found_new_token = true;
                if let Some((_, generation)) = self.ensemble_identity_location(&token) {
                    origins.insert(generation);
                }
                if let Some((_, generation)) = self.retire_ensemble_identity(&token) {
                    origins.insert(generation);
                }
                tokens.push(token);
            }

            let slots = self.namespaces.borrow().command_slots_in_ids(&ids);
            let new_slots: Vec<(NsId, Vec<u8>)> = slots
                .into_iter()
                .filter(|slot| !traced.contains(slot))
                .collect();
            for (id, tail) in &new_slots {
                // Callback-created command traces fire while the command is
                // still addressable through the dying namespace token.
                if self.check_entered_native_operation().is_ok() {
                    self.on_bound_command_replaced(*id, tail);
                }
                traced.insert((*id, tail.clone()));
            }

            for id in self.retained_teardown_ids(root) {
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
            origins.extend(
                self.namespaces
                    .borrow()
                    .command_locations_in_ids(&ids)
                    .into_iter()
                    .map(|(_, generation)| generation),
            );
            self.remove_imports_for_deleted_origins(origins.iter().copied(), &tokens);

            let remaining = self.namespaces.borrow().command_slots_in_ids(&ids);
            if !found_new_token
                && new_slots.is_empty()
                && remaining.iter().all(|slot| traced.contains(slot))
            {
                break;
            }
        }

        // Import delete callbacks may have added a new child under the detached
        // root during the last fixed-point pass.
        for id in self.retained_teardown_ids(root) {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        origins.extend(
            self.namespaces
                .borrow()
                .command_locations_in_ids(&ids)
                .into_iter()
                .map(|(_, generation)| generation),
        );
        self.remove_imports_for_deleted_origins(origins.iter().copied(), &tokens);
        let cleared = self.namespaces.borrow().command_locations_in_ids(&ids);
        if self
            .name_policy_protocol()
            .is_some_and(|policy| policy.recipe().is_jim084())
        {
            // The residual fixed-point sweep may contain callback-created Jim
            // slots. Retire each captured generation before variable/namespace
            // cleanup; an original key destructor runs outside the borrow.
            let slots = {
                let namespaces = self.namespaces.borrow();
                namespaces
                    .command_slots_in_ids(&ids)
                    .into_iter()
                    .filter_map(|(ns, tail)| {
                        Some((ns, tail.clone(), namespaces.command_generation(ns, &tail)?))
                    })
                    .collect::<Vec<_>>()
            };
            for (ns, tail, generation) in slots {
                {
                    let mut namespaces = self.namespaces.borrow_mut();
                    if namespaces.command_generation(ns, &tail) == Some(generation) {
                        namespaces.remove_in(ns, &tail);
                    }
                }
                if let Err(first) = self.retire_pending_native_ensemble_roles() {
                    self.refuse_native_execution(first);
                }
            }
            // A destructor may publish a distinct new slot. It needs the
            // existing fresh teardown frontier and callbacks, never the old
            // captured generation's authority or a blind bulk removal.
            for &ns in &ids {
                self.tear_down_command_table(ns);
            }
        }
        let retired = self.namespaces.borrow_mut().clear_namespace_ids(&ids);
        if let Err(first) = self.retire_pending_native_ensemble_roles() {
            self.refuse_native_execution(first);
        }
        drop(retired);
        for (fqn, generation) in cleared {
            self.remove_cmd_traces_of_token(&fqn, Some(generation));
        }
    }

    /// Resolve a (relative/absolute) namespace name to its id, or `None`.
    pub(crate) fn find_namespace_id(&self, name: &[u8]) -> Option<NsId> {
        self.find_named_namespace(self.current_ns.get(), name)
    }

    /// The fully-qualified name of namespace `ns` (`"::"` for the root). Backs
    /// `Namespaces::name` (`state_traits.rs`), keeping the namespace-table access here.
    pub(crate) fn ns_qualified_name(&self, ns: NsId) -> Vec<u8> {
        self.namespaces.borrow().qualified_name(ns)
    }
}
