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

//! Command and execution traces: registration, running a callback, firing on rename and delete, and moving trace and active-call sidecars with a relocated command.

use super::super::commands::CommandSidecarKey;
use super::super::{
    CmdTraceEntry, CommandSidecarHandle, Vm, command_cleanup, err, native_name_consumers, ok,
};
use super::is_step_capable;
use crate::value::Value;
use std::cell::RefCell;
use std::rc::Rc;
use tcl_runtime_api::Completion;
use tcl_runtime_api::guard::GuardDomain;

impl Vm {
    /// Register a `trace add command|execution` callback on `name` — which,
    /// unlike a variable trace, must resolve to an existing command
    /// (tclsh-pinned: `unknown command "missing"`).
    #[cfg(test)]
    pub(crate) fn add_cmd_trace(
        &mut self,
        execution: bool,
        name: &str,
        ops: Vec<String>,
        callback: String,
    ) -> Completion<Value> {
        self.add_cmd_trace_bytes(execution, name.as_bytes(), ops, callback.into_bytes())
    }

    fn command_trace_key(&mut self, name: &[u8]) -> Result<String, Completion<Value>> {
        match self.resolve_command_bytes_checked(self.current_ns_id(), name, true) {
            Ok(Some(key)) => Ok(key),
            Ok(None) => Err(err([
                b"unknown command \"".as_slice(),
                tcl_core_types::c_string_extent(name),
                b"\"",
            ]
            .concat())),
            Err(error) => Err(self.refuse_host_command(format!(
                "command trace name lookup is unavailable: {error:?}"
            ))),
        }
    }

    pub(crate) fn add_cmd_trace_bytes(
        &mut self,
        execution: bool,
        name: &[u8],
        ops: Vec<String>,
        callback: Vec<u8>,
    ) -> Completion<Value> {
        if self
            .actual_native_invocation_dialect()
            .native_variable_trace_protocol()
            .is_none()
        {
            return self.refuse_host_command("command trace protocol is unavailable".to_owned());
        }
        let key = match self.command_trace_key(name) {
            Ok(key) => key,
            Err(completion) => return completion,
        };
        let is_step = is_step_capable(&ops);
        // The trace belongs to the token standing at this key now, not to the
        // name (C hangs it off `cmdPtr->tracePtr`) — and inside a rename's
        // callbacks the vacating name reaches the destination's list.
        let key = self.renamed_command_key(CommandSidecarKey::visible(key));
        let token = self
            .command_token_identity(&key)
            .map(|identity| identity.generation);
        let first_execution_trace = execution && !self.command_has_execution_trace(&key);
        let hook = self.native_hook_at_sidecar(&key);
        let table = if execution {
            &mut self.exec_traces
        } else {
            &mut self.cmd_traces
        };
        table
            .entry(key)
            .or_default()
            .push(Rc::new(CmdTraceEntry::new(ops, callback, token)));
        self.invalidate_guard_domain(GuardDomain::CommandTrace);
        // Registration changes runtime observer guards. Native global inline
        // compilation changes only when a traced invocation installs its step
        // scope; the token's own first execution trace has a separate policy.
        if execution && (is_step || first_execution_trace) {
            self.bump_trace_deopt_epoch();
        }
        if first_execution_trace {
            self.note_native_compiler_cache_mutation(
                tcl_registry::native_procedure::NativeCompilerCacheMutation::ExecutionTrace {
                    hook,
                },
            );
        }
        ok(Value::empty())
    }

    /// Remove one command/execution trace matching `ops` + `callback`.
    pub(crate) fn remove_cmd_trace_bytes(
        &mut self,
        execution: bool,
        name: &[u8],
        ops: &[String],
        callback: &[u8],
    ) -> Completion<Value> {
        let Some(protocol) = self
            .actual_native_invocation_dialect()
            .native_variable_trace_protocol()
        else {
            return self.refuse_host_command("command trace protocol is unavailable".to_owned());
        };
        let key = match self.command_trace_key(name) {
            Ok(key) => key,
            Err(completion) => return completion,
        };
        let is_step = is_step_capable(ops);
        let key = self.renamed_command_key(CommandSidecarKey::visible(key));
        let step_compilation_was_disabled = self.step_trace_active();
        let hook = self.native_hook_at_sidecar(&key);
        let table = if execution {
            &mut self.exec_traces
        } else {
            &mut self.cmd_traces
        };
        let mut removed = false;
        if let Some(list) = table.get_mut(&key) {
            // Newest-first first match, as `remove_var_trace` explains.
            if let Some(index) = list.iter().rposition(|t| {
                t.ops == ops && protocol.command_prefix_matches(&t.callback, callback)
            }) {
                // Mark before dropping the handle: a firing walk holds its own
                // `Rc` and consults the flag, not the table.
                list[index].untraced.set(true);
                list.remove(index);
                removed = true;
            }
            if list.is_empty() {
                table.remove(&key);
            }
        }
        if removed {
            self.invalidate_guard_domain(GuardDomain::CommandTrace);
        }
        let last_execution_trace_removed =
            execution && removed && !self.command_has_execution_trace(&key);
        // Observer guard invalidation is separate from actual active step
        // scope removal and this token's compiler-hook invalidation.
        if execution && (is_step || last_execution_trace_removed) {
            self.bump_trace_deopt_epoch();
        }
        self.publish_native_inline_trace_transition(step_compilation_was_disabled);
        if last_execution_trace_removed {
            self.note_native_compiler_cache_mutation(
                tcl_registry::native_procedure::NativeCompilerCacheMutation::ExecutionTrace {
                    hook,
                },
            );
        }
        ok(Value::empty())
    }

    /// The `{ops callback}` pairs registered on command `name` (newest first),
    /// for `trace info command|execution`.
    pub(crate) fn cmd_trace_entries_bytes(
        &mut self,
        execution: bool,
        name: &[u8],
    ) -> Completion<Value> {
        // C resolves the name with `TCL_LEAVE_ERR_MSG` before reporting
        // (`TraceCommandObjCmd` `tclTrace.c` 9.0.4:661, `TraceExecutionObjCmd`
        // :454), so introspecting a command that does not exist errors with the
        // name as written — unlike `trace info variable`, which answers empty.
        let Some(protocol) = self
            .actual_native_invocation_dialect()
            .native_variable_trace_protocol()
        else {
            return self.refuse_host_command("command trace protocol is unavailable".to_owned());
        };
        let key = match self.command_trace_key(name) {
            Ok(key) => key,
            Err(completion) => return completion,
        };
        let table = if execution {
            &self.exec_traces
        } else {
            &self.cmd_traces
        };
        let Some(list) = table.get(&self.renamed_command_key(CommandSidecarKey::visible(key)))
        else {
            return ok(Value::empty());
        };
        ok(Value::list(
            list.iter()
                .rev()
                .map(|t| {
                    Value::list(vec![
                        Value::list(t.ops.iter().map(|o| Value::string(o.clone())).collect()),
                        Value::from_native_string_bytes(
                            protocol.command_prefix_report(&t.callback),
                        ),
                    ])
                })
                .collect(),
        ))
    }

    /// Run one **execution** trace callback (`enter`/`leave`/`enterstep`/
    /// `leavestep`) with `args` appended (list-quoted), in the current frame —
    /// C evaluates trace callbacks in the context where the traced operation
    /// occurred.  The entry is disabled while its own callback runs (C's
    /// re-entrancy rule), a nested fire returning ok.
    ///
    /// This is the one callback kind C wraps in `INTERP_TRACE_IN_PROGRESS`
    /// (`TraceExecutionProc`, tclTrace.c 9.0.4:1765), so the callback's own
    /// commands are not step-observed.
    pub(crate) fn run_cmd_trace_callback(
        &mut self,
        entry: &CmdTraceEntry,
        args: &[Value],
    ) -> Completion<Value> {
        self.run_trace_callback(entry, args, true)
    }

    /// Run one **command** (`rename`/`delete`) trace callback.  C's
    /// `CallCommandTraces` sets no interpreter flag, so — unlike an execution
    /// callback — a command this one dispatches is traced like any other: its
    /// `enter`/`leave` traces fire, and an enclosing step scope steps it.
    /// Re-entering *this* command's own rename/delete traces is suppressed per
    /// command by [`Self::firing_cmd_traces`] instead.
    pub(crate) fn run_command_trace_callback(
        &mut self,
        entry: &CmdTraceEntry,
        args: &[Value],
    ) -> Completion<Value> {
        self.run_trace_callback(entry, args, false)
    }

    /// The shared body of the two above; `interp_trace_in_progress` says
    /// whether this callback kind raises C's interpreter-wide gate.
    fn run_trace_callback(
        &mut self,
        entry: &CmdTraceEntry,
        args: &[Value],
        interp_trace_in_progress: bool,
    ) -> Completion<Value> {
        let original_names = match native_name_consumers::OriginalNameConsumerScope::enter(self) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        let original_cleanup = command_cleanup::OriginalCommandCleanup::enter(self);
        let reached = (|| {
            if let Some(refused) = self.refused_completion() {
                return refused;
            }
            if entry.firing.get() {
                return ok(Value::empty());
            }
            entry.firing.set(true);
            let mut script = entry.callback.clone();
            for argument in args {
                let bytes = match original_names.read(self, argument) {
                    Ok(bytes) => bytes,
                    Err(error) => {
                        entry.firing.set(false);
                        return crate::command::completion_from_cmd_error(self, error);
                    }
                };
                script.push(b' ');
                tcl_syntax::list::append_list_element(&mut script, &bytes, false);
            }
            // Saved/restored, not merely set — a callback can itself be dispatched
            // from inside another callback's evaluation only if re-entrant nesting
            // is legitimate, but the common case is one level; save-restore keeps
            // either correct, and keeps a command callback nested inside an
            // execution one from clearing the gate that one raised.
            let saved = self.trace_in_progress.get();
            self.trace_in_progress
                .set(saved || interp_trace_in_progress);
            let Some(protocol) = self
                .actual_native_invocation_dialect()
                .native_variable_trace_protocol()
            else {
                if let Ok(state) = original_cleanup.state_mut(self) {
                    state.trace_in_progress.set(saved);
                }
                entry.firing.set(false);
                return self.refuse_host_command(
                    "command trace callback protocol is unavailable".to_owned(),
                );
            };
            let source = Value::from_native_string_bytes(
                protocol.command_callback_source(interp_trace_in_progress, &script),
            );
            let res = self.eval_value_at_level(self.current_level(), &source);
            if let Ok(state) = original_cleanup.state_mut(self) {
                state.trace_in_progress.set(saved);
            }
            entry.firing.set(false);
            res
        })();
        if let Err(error) = original_names.finish(self) {
            return crate::command::completion_from_cmd_error(self, error);
        }
        reached
    }

    /// Fire the `rename`/`delete` command traces held at `key` as
    /// `callback oldName newName op` — names fully qualified (tclsh-pinned:
    /// `::victim ::victim2 rename`; a delete passes `{}` for the new name).
    /// `old_display` is passed rather than derived from `key`: a rename fires
    /// from the destination's list (see [`Self::on_command_renamed_traces`])
    /// but still names the command it is leaving.  Callback errors are ignored
    /// (C: "Any errors in these traces are ignored").
    fn fire_command_traces_for(
        &mut self,
        key: &CommandSidecarKey,
        old_display: &[u8],
        new_display: &[u8],
        op: &str,
    ) {
        if self.cmd_traces.is_empty() || self.firing_cmd_traces.contains(key) {
            return;
        }
        let Some(entries) = self.cmd_traces.get(key).cloned() else {
            return;
        };
        let original = command_cleanup::OriginalCommandCleanup::enter(self);
        if original.check(self).is_err() {
            return;
        }
        let depth = self.firing_cmd_traces.len();
        self.firing_cmd_traces.push(key.clone());
        let mut saved = None;
        let mut state_selected = false;
        for entry in entries.into_iter().rev() {
            if original.check(self).is_err() {
                break;
            }
            if !entry.ops.iter().any(|o| o == op) || entry.untraced() {
                continue;
            }
            if !state_selected {
                match self.save_native_command_trace_result() {
                    Ok(state) => saved = state,
                    Err(_) => break,
                }
                state_selected = true;
            }
            let _ = self.run_command_trace_callback(
                &entry,
                &[
                    Value::from_string_bytes(old_display),
                    Value::from_string_bytes(new_display),
                    Value::string(op),
                ],
            );
            let _ = original.check(self);
        }
        if let Some(saved) = saved {
            let _ = self.restore_native_interp_trace_result(saved);
        }
        if let Ok(state) = original.state_mut(self) {
            // The actual caller-owned slot, including its legitimate rename,
            // is removed from its original stack; a child stack stays intact.
            if state.firing_cmd_traces.len() > depth {
                state.firing_cmd_traces.remove(depth);
            }
        }
        let _ = original.check(self);
    }

    /// A command at `key` is about to be deleted or overwritten: fire its
    /// `delete` traces (tclsh-pinned: redefining a traced proc fires them
    /// too) and drop every trace registered on it.
    pub(crate) fn on_command_removed(&mut self, key: &str) {
        self.on_command_removed_for(&CommandSidecarKey::visible(key));
    }

    pub(in super::super) fn on_command_removed_for(&mut self, key: &CommandSidecarKey) {
        let original = command_cleanup::OriginalCommandCleanup::enter(self);
        let selected = original.in_original(self, |vm| {
            let hook = vm.native_hook_at_sidecar(key);
            vm.detach_active_sidecars(key);
            let dying = vm
                .command_token_identity(key)
                .map(|identity| identity.generation);
            let namespace = match key {
                CommandSidecarKey::Visible(key) => vm.command_slot(key).map(|slot| slot.namespace),
                CommandSidecarKey::Hidden(_) => None,
            };
            let invalidate_namespace = vm
                .actual_native_invocation_dialect()
                .native_command_name_protocol()
                .is_some_and(tcl_registry::native_command_literal::NativeCommandNameProtocol::deletion_invalidates_namespace_after_trace);
            (
                hook,
                dying,
                namespace,
                invalidate_namespace,
                vm.rooted_command_sidecar_display_bytes(key),
            )
        });
        let Ok((native_hook, dying, namespace, invalidate_namespace, display)) = selected else {
            return;
        };
        if original.check(self).is_ok() {
            let _ = original.in_original(self, |vm| {
                vm.fire_command_traces_for(key, &display, b"", "delete");
            });
        }
        let _ = original.in_original(self, |vm| {
            if let Some(dying) = dying {
                vm.drop_relocated_command_sidecars(key, dying);
            } else {
                let mut removed_trace = vm.drop_retired_cmd_traces(key, None);
                let step_compilation_was_disabled = vm.step_trace_active();
                if let Some(removed) = vm.exec_traces.remove(key) {
                    removed_trace = true;
                    for entry in &removed {
                        entry.untraced.set(true);
                    }
                    if removed.iter().any(|entry| is_step_capable(&entry.ops)) {
                        vm.bump_trace_deopt_epoch();
                    }
                    drop(removed);
                }
                vm.publish_native_inline_trace_transition(step_compilation_was_disabled);
                if removed_trace {
                    vm.invalidate_guard_domain(GuardDomain::CommandTrace);
                }
            }
            // The release predicate belongs to the entered owner; the
            // command's namespace follows an actual same-generation move.
            // A replacement at the old spelling supplies neither observation.
            let reached_namespace = dying
                .and_then(|generation| vm.command_token_at_generation(generation))
                .and_then(|token| match token.key {
                    CommandSidecarKey::Visible(key) => {
                        vm.command_slot(&key).map(|slot| slot.namespace)
                    }
                    CommandSidecarKey::Hidden(_) => None,
                })
                .or(namespace);
            if invalidate_namespace && let Some(namespace) = reached_namespace {
                vm.note_native_namespace_command_lookup(namespace);
            }
            if let Some(dying) = dying {
                // The actual hook was selected before the trace. Recursive
                // unlink does not replace that old compiler mutation owner.
                if vm
                    .name_world
                    .borrow_mut()
                    .command_compiler_deletion_published
                    .insert(dying)
                {
                    vm.note_native_compiler_cache_mutation(
                        tcl_registry::native_procedure::NativeCompilerCacheMutation::CommandToken {
                            hook: native_hook,
                        },
                    );
                }
            }
        });
        let _ = original.check(self);
    }

    /// Drop the command traces the dying token carried — C's "free the whole
    /// `cmdPtr->tracePtr` list". A trace the delete callback added to the dying
    /// command goes with it; one it registered on a replacement it bound at the
    /// same key has a later generation and survives. An unidentifiable dying
    /// token takes the whole list.
    fn drop_retired_cmd_traces(&mut self, key: &CommandSidecarKey, dying: Option<u64>) -> bool {
        let Some(entries) = self.cmd_traces.get_mut(key) else {
            return false;
        };
        let before = entries.len();
        entries.retain(|entry| match (entry.token.get(), dying) {
            (Some(token), Some(dying)) => token > dying,
            _ => false,
        });
        let removed = entries.len() != before;
        if entries.is_empty() {
            self.cmd_traces.remove(key);
        }
        removed
    }

    /// Drop trace and active-call sidecars for a token whose delete trace has
    /// already fired at an earlier location. A callback can rename, hide, or
    /// expose that same generation; firing again at the destination would
    /// duplicate Tcl's one `CMD_DYING` callback pass.
    pub(in super::super) fn drop_relocated_command_sidecars(
        &mut self,
        key: &CommandSidecarKey,
        generation: u64,
    ) {
        self.detach_active_sidecars(key);
        let mut removed = self.drop_retired_cmd_traces(key, Some(generation));
        let step_compilation_was_disabled = self.step_trace_active();
        let mut dropped_step_trace = false;
        if let Some(entries) = self.exec_traces.get_mut(key) {
            let before = entries.len();
            dropped_step_trace = entries.iter().any(|entry| {
                entry.token.get().is_none_or(|token| token <= generation)
                    && is_step_capable(&entry.ops)
            });
            entries.retain(|entry| {
                let keep = entry.token.get().is_some_and(|token| token > generation);
                if !keep {
                    entry.untraced.set(true);
                }
                keep
            });
            removed |= entries.len() != before;
            if entries.is_empty() {
                self.exec_traces.remove(key);
            }
        }
        if dropped_step_trace {
            self.bump_trace_deopt_epoch();
        }
        self.publish_native_inline_trace_transition(step_compilation_was_disabled);
        if removed {
            self.invalidate_guard_domain(GuardDomain::CommandTrace);
        }
    }

    /// A command moved `old_key` → `new_key` (`rename`): move every trace with
    /// it (tclsh-pinned: an execution trace keeps firing under the new name),
    /// then fire its `rename` traces with both fully-qualified names.
    ///
    /// The move comes *first* because C's traces hang off the shared `Command`
    /// rather than off either hash entry, so a callback reaches one list under
    /// either name: `trace info command <old>` and `… <new>` answer the same
    /// thing, and a `trace remove` through either name edits it.  Publishing
    /// the move and aliasing the source key onto the destination for the
    /// callbacks' duration reproduces that; `cmd_rename` keeps the source
    /// command registered across this call so the old name still resolves.
    pub(crate) fn on_command_renamed_traces(&mut self, old_key: &str, new_key: &str) {
        let old_key = CommandSidecarKey::visible(old_key);
        let new_key = CommandSidecarKey::visible(new_key);
        let old_display = self.rooted_command_sidecar_display_bytes(&old_key);
        let new_display = self.rooted_command_sidecar_display_bytes(&new_key);
        self.relocate_active_sidecars(&old_key, &new_key);
        self.relocate_rename_state(&old_key, &new_key);
        let mut moved_trace = false;
        if !self.cmd_traces.is_empty()
            && let Some(entries) = self.cmd_traces.remove(&old_key)
        {
            self.cmd_traces.insert(new_key.clone(), entries);
            moved_trace = true;
        }
        if !self.exec_traces.is_empty()
            && let Some(entries) = self.exec_traces.remove(&old_key)
        {
            self.exec_traces.insert(new_key.clone(), entries);
            moved_trace = true;
        }
        if moved_trace {
            self.invalidate_guard_domain(GuardDomain::CommandTrace);
        }
        self.rename_windows.push((old_key, new_key.clone()));
        self.fire_command_traces_for(&new_key, &old_display, &new_display, "rename");
        self.rename_windows.pop();
    }

    /// The key a command's state lives under now, following any open rename
    /// window (see [`Self::rename_windows`]). Identity outside one.
    pub(crate) fn renamed_command_key(&self, key: CommandSidecarKey) -> CommandSidecarKey {
        match self.rename_windows.iter().find(|(from, _)| *from == key) {
            Some((_, to)) => to.clone(),
            None => key,
        }
    }

    /// A rename moved `old_key` → `new_key`: point every open window and every
    /// in-flight firing record that named the old key at the new one. C needs
    /// no equivalent — both its hash entries reference one `Command` object, so
    /// a nested rename cannot strand them — but the VM's key-addressed state
    /// would otherwise be left on a key the move just vacated: an enclosing
    /// window would stop answering through the source name, and the
    /// `firing_cmd_traces` record standing in for `CMD_TRACE_ACTIVE` would stop
    /// suppressing the command's own remaining callbacks.
    pub(in super::super) fn relocate_rename_state(
        &mut self,
        old_key: &CommandSidecarKey,
        new_key: &CommandSidecarKey,
    ) {
        for (_, destination) in &mut self.rename_windows {
            if destination == old_key {
                destination.clone_from(new_key);
            }
        }
        for firing in &mut self.firing_cmd_traces {
            if firing == old_key {
                firing.clone_from(new_key);
            }
        }
    }

    /// Whether this exact live command token has an execution trace.
    ///
    /// Command-specific traces are runtime state: source can install or remove
    /// one after a module was compiled. Central binding validation and typed
    /// command-head metadata consult this table, matching C Tcl's
    /// `CMD_HAS_EXEC_TRACES` decision without leaking trace-table knowledge
    /// into the compiler.
    pub(crate) fn command_has_execution_trace(&self, key: &CommandSidecarKey) -> bool {
        self.exec_traces
            .get(key)
            .is_some_and(|entries| !entries.is_empty())
    }

    /// Move trace registrations with a command through the hidden table.  A
    /// hide/expose is not a Tcl rename, so no callback is fired.
    pub(in super::super) fn move_command_traces(
        &mut self,
        old_key: &CommandSidecarKey,
        new_key: CommandSidecarKey,
    ) {
        self.relocate_active_sidecars(old_key, &new_key);
        self.relocate_rename_state(old_key, &new_key);
        // The list travels with the token, so re-stamp it to whatever token
        // now stands at the destination; a later deletion there still tells
        // this list from a replacement's.
        let token = self
            .command_token_identity(&new_key)
            .map(|identity| identity.generation);
        let mut moved = false;
        if let Some(entries) = self.cmd_traces.remove(old_key) {
            for entry in &entries {
                entry.token.set(token);
            }
            self.cmd_traces.insert(new_key.clone(), entries);
            moved = true;
        }
        if let Some(entries) = self.exec_traces.remove(old_key) {
            for entry in &entries {
                entry.token.set(token);
            }
            self.exec_traces.insert(new_key, entries);
            moved = true;
        }
        if moved {
            self.invalidate_guard_domain(GuardDomain::CommandTrace);
        }
    }

    /// Create an active identity that follows this command through every
    /// visible/hidden/renamed sidecar relocation.
    pub(crate) fn active_sidecar(&mut self, key: CommandSidecarKey) -> CommandSidecarHandle {
        self.active_sidecar_handles
            .retain(|handle| handle.strong_count() != 0);
        let cell = Rc::new(RefCell::new(Some(key)));
        self.active_sidecar_handles.push(Rc::downgrade(&cell));
        CommandSidecarHandle(cell)
    }

    fn relocate_active_sidecars(
        &mut self,
        old_key: &CommandSidecarKey,
        new_key: &CommandSidecarKey,
    ) {
        self.active_sidecar_handles.retain(|weak| {
            let Some(handle) = weak.upgrade() else {
                return false;
            };
            if handle.borrow().as_ref() == Some(old_key) {
                *handle.borrow_mut() = Some(new_key.clone());
            }
            true
        });
    }

    /// Disconnect all active users of a deleted command binding.  Retaining
    /// the weak cells is harmless; they are pruned when the in-flight calls
    /// complete, while their `None` identity prevents later relocation.
    pub(crate) fn detach_active_sidecars(&mut self, key: &CommandSidecarKey) {
        self.active_sidecar_handles.retain(|weak| {
            let Some(handle) = weak.upgrade() else {
                return false;
            };
            if handle.borrow().as_ref() == Some(key) {
                *handle.borrow_mut() = None;
            }
            true
        });
    }
}
