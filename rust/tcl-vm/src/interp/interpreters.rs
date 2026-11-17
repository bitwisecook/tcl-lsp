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

//! The interpreter tree: the arena and switching the current interpreter, child interpreters and their command form, aliases, hidden commands and safe interpreters.

use super::commands::CommandSidecarKey;
#[cfg(test)]
use super::err;
use super::{InterpId, InterpSlot, InterpState, Vm, native_operation_currency};
use crate::command::Command;
#[cfg(test)]
use crate::value::Value;
#[cfg(test)]
use tcl_runtime_api::Completion;

mod aliases;
mod child_command;
mod children;
mod hidden_commands;

#[cfg(test)]
pub(crate) use hidden_commands::CommandVisibilityError;
#[cfg(test)]
pub(super) use hidden_commands::native_visibility_completion;

impl InterpSlot {
    pub(super) fn is_root(&self) -> bool {
        self.parent.is_none()
    }
}

impl Vm {
    /// The currently-executing interpreter's id.
    pub(crate) fn cur_interp(&self) -> InterpId {
        self.cur
    }

    /// Read another interpreter's state (or the current one's, uniformly).
    /// `None` for a dead id.
    pub(crate) fn st_of(&self, id: InterpId) -> Option<&InterpState> {
        if id == self.cur {
            Some(&self.state)
        } else {
            self.interps.get(id.0)?.parked.as_deref()
        }
    }

    #[cfg(test)]
    pub(crate) fn current_native_operation_is_entered_for_test(&self) -> bool {
        self.state.native_operation_context.has_entered_operation()
    }

    /// Compare the actual arena identity with this activation's interpreter.
    pub(crate) fn interp_is_current(&self, id: InterpId) -> bool {
        id == self.cur
    }

    /// Whether `id` is still addressable (created and not deleted).
    pub(crate) fn interp_alive(&self, id: InterpId) -> bool {
        self.interps
            .get(id.0)
            .is_some_and(|s| !s.dying && (id == self.cur || s.parked.is_some()))
    }

    /// Run `f` with `id` as the current interpreter — the cross-interp calling
    /// convention (C's `Tcl_EvalObjv(targetInterp, …)` on the shared C stack):
    /// the current state parks in its arena slot, the target state becomes
    /// current, and the previous interpreter is restored on the way out.  The
    /// nested evaluation is a plain native re-entry, so re-entering an
    /// interpreter that is already executing deeper on the stack is legal —
    /// its persistent state lives in the arena throughout.  Deferred teardown:
    /// if the target was deleted while (re-)entered, the last exit drops its
    /// state (C's `Tcl_Preserve`/`Tcl_Release`; tclsh-pinned).
    pub(crate) fn in_interp<R>(&mut self, id: InterpId, f: impl FnOnce(&mut Vm) -> R) -> R {
        self.in_interp_for_cleanup(
            id,
            native_operation_currency::CleanupRestoration::RestoreCaller,
            f,
        )
    }

    /// The same physical calling convention with a mandatory-cleanup policy.
    /// Direct-current command/variable retirement adds no preservation hold.
    pub(super) fn in_interp_for_cleanup<R>(
        &mut self,
        id: InterpId,
        policy: native_operation_currency::CleanupRestoration,
        f: impl FnOnce(&mut Vm) -> R,
    ) -> R {
        self.in_interp_using(id, policy, f, Vm::finalize_slot_for_cleanup)
    }

    pub(super) fn in_interp_using<R>(
        &mut self,
        id: InterpId,
        policy: native_operation_currency::CleanupRestoration,
        f: impl FnOnce(&mut Vm) -> R,
        finalize: impl FnOnce(&mut Vm, InterpId, native_operation_currency::CleanupRestoration),
    ) -> R {
        use native_operation_currency::CleanupRestoration;
        let prev = self.cur;
        if policy == CleanupRestoration::PreserveCallbackSelection && prev == id {
            return f(self);
        }
        self.interps[id.0].active += 1;
        self.switch_to(id);
        let r = f(self);
        let selected = self.cleanup_return_target(id, prev, policy);
        self.switch_to(selected);
        let slot = &mut self.interps[id.0];
        slot.active -= 1;
        if slot.dying && slot.active == 0 {
            finalize(self, id, policy);
        }
        r
    }

    /// C's `TclInterpReady`: an interpreter deleted while it runs evaluates no
    /// further command. `None` while the current interpreter is live, and for
    /// an engine whose protocol does not say what it reports.
    pub(crate) fn deleted_interpreter_refusal(
        &self,
    ) -> Option<tcl_runtime_api::Completion<crate::value::Value>> {
        if !self.interps.get(self.cur.0).is_some_and(|slot| slot.dying) {
            return None;
        }
        let (message, code) = self
            .native_invocation_dialect()
            .native_eval_object_protocol()?
            .deleted_interpreter_error()?;
        Some(crate::command::err_with_code(message, code))
    }

    /// Whether `id` is torn down when the evaluation running in it unwinds: it
    /// was deleted, and that evaluation is its last in-flight one.
    pub(super) fn interp_retires_on_return(&self, id: InterpId) -> bool {
        self.interps
            .get(id.0)
            .is_some_and(|slot| slot.dying && slot.active == 1)
    }

    fn cleanup_return_target(
        &self,
        original: InterpId,
        caller: InterpId,
        policy: native_operation_currency::CleanupRestoration,
    ) -> InterpId {
        if policy == native_operation_currency::CleanupRestoration::PreserveCallbackSelection
            && self.cur != original
        {
            // Observe the actual current state at this stage, never a saved
            // pre-finalization child or a newly issued execution receipt.
            self.cur
        } else {
            caller
        }
    }

    /// Make `id` current by swapping its parked state with the current one.
    pub(super) fn switch_to(&mut self, id: InterpId) {
        if id == self.cur {
            return;
        }
        let incoming = self.interps[id.0]
            .parked
            .take()
            .expect("switch target's state is parked in its slot");
        self.native_operation_context.select(false);
        incoming.native_operation_context.select(true);
        let outgoing = std::mem::replace(&mut self.state, incoming);
        self.interps[self.cur.0].parked = Some(outgoing);
        self.cur = id;
    }

    /// Mint a fresh arena slot for a new interpreter owned by `parent`.
    pub(super) fn new_interp_slot(
        &mut self,
        state: Box<InterpState>,
        parent: InterpId,
    ) -> InterpId {
        state.native_operation_context.select(false);
        let id = InterpId(self.interps.len());
        self.interps.push(InterpSlot {
            parked: Some(state),
            active: 0,
            dying: false,
            command_dying: false,
            parent: Some(parent),
        });
        id
    }

    /// Delete interpreter `id`: its subtree dies first (C deletes children
    /// with their parent), cross-interp aliases TARGETING it are swept out of
    /// their source interps (C's target table — tclsh-pinned: the alias
    /// command disappears from `info commands`), and the state itself is
    /// dropped now — or, when the interp is still executing (a re-entered
    /// child, a target mid-callback), when its last in-flight evaluation
    /// unwinds (tclsh-pinned: the in-flight eval completes normally).
    pub(super) fn retire_interp(&mut self, id: InterpId) {
        if self.interps.get(id.0).is_none_or(|s| s.dying) {
            return;
        }
        if let Some(state) = self.st_of(id) {
            state
                .native_compiler_pass_owner
                .retire_ordinary_entry_observation();
        }
        // An interpreter still executing keeps its world until finalization,
        // as C defers DeleteInterpProc to the last Tcl_Release.
        if self.interps[id.0].active == 0
            && let Some(state) = self.st_of(id)
            && state.actual_native_string_protocol()
                != Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            let retired = {
                let mut world = state.name_world.borrow_mut();
                world.retire();
                std::mem::take(&mut world.jim_command_original_keys)
            };
            drop(retired);
        }
        let child_ids: Vec<InterpId> = self
            .st_of(id)
            .map_or_default(|st| st.children.values().copied().collect());
        for c in child_ids {
            self.retire_interp(c);
        }
        let targeting: Vec<(InterpId, CommandSidecarKey)> = self
            .alias_backrefs
            .iter()
            .filter(|b| b.target == id && b.source != id)
            .map(|b| (b.source, b.key.clone()))
            .collect();
        for (src, key) in targeting {
            self.in_interp(src, |vm| {
                if matches!(vm.command_at_sidecar_key(&key), Some(Command::CrossAlias { target, .. }) if target == id)
                {
                    vm.retire_command_lifecycle_key(&key);
                }
            });
        }
        // Hidden aliases are not dispatchable, so a stale/missing backref must
        // not keep one alive after its target dies.  Sweep the hidden tables as
        // a defensive counterpart to C's target-side alias list.
        let hidden_targeting: Vec<(InterpId, String)> = self
            .interps
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| slot.parked.as_ref().map(|st| (InterpId(index), st)))
            .flat_map(|(source, st)| {
                st.hidden.commands
                    .iter()
                    .filter(move |(_, command)| {
                        matches!(command, Command::CrossAlias { target, .. } if *target == id)
                    })
                    .map(move |(key, _)| (source, key.clone()))
            })
            .collect();
        for (source, key) in hidden_targeting {
            self.in_interp(source, |vm| {
                let sidecar = CommandSidecarKey::hidden(&key);
                vm.retire_command_lifecycle_key(&sidecar);
            });
        }
        self.alias_backrefs.retain(|b| b.target != id);
        let slot = &mut self.interps[id.0];
        slot.dying = true;
        if slot.active == 0 {
            self.finalize_slot(id);
        }
    }

    /// Drop a dying interpreter's state (frames, commands, channels,
    /// coroutines) once nothing is executing in it any more.
    fn finalize_slot(&mut self, id: InterpId) {
        self.finalize_slot_for_cleanup(
            id,
            native_operation_currency::CleanupRestoration::RestoreCaller,
        );
    }

    fn finalize_slot_for_cleanup(
        &mut self,
        id: InterpId,
        policy: native_operation_currency::CleanupRestoration,
    ) {
        self.finalize_slot_using(id, policy, Vm::retire_jim_interpreter);
    }

    pub(super) fn finalize_slot_using(
        &mut self,
        id: InterpId,
        policy: native_operation_currency::CleanupRestoration,
        retire: impl FnOnce(&mut Vm),
    ) {
        let previous = self.cur;
        self.switch_to(id);
        retire(self);
        let selected = self.cleanup_return_target(id, previous, policy);
        // Finalizer callbacks may have selected a child. Retire the actual
        // original world before following the selected return convention.
        self.switch_to(id);
        let retired = {
            let mut world = self.name_world.borrow_mut();
            world.retire();
            std::mem::take(&mut world.jim_command_original_keys)
        };
        drop(retired);
        self.switch_to(selected);
        let slot = &mut self.interps[id.0];
        debug_assert!(slot.dying && slot.active == 0);
        slot.parked = None;
        self.alias_backrefs.retain(|b| b.source != id);
    }

    /// Resolve an `interp` path from the current interpreter: `""`/`{}` names
    /// the current interp, and each list element steps into the named child
    /// (C's multi-level paths — `interp eval {a b} …` addresses grandchild
    /// `b`).  A plain name is the common single-element case.
    #[cfg(test)]
    pub(crate) fn resolve_interp_path(&self, path: &str) -> Result<InterpId, Completion<Value>> {
        if path.is_empty() {
            return Ok(self.cur);
        }
        let needs_split = path
            .bytes()
            .any(|b| b.is_ascii_whitespace() || matches!(b, b'{' | b'}' | b'"' | b'\\'));
        let elems: Vec<String> = if needs_split {
            match tcl_syntax::list::split_list(path) {
                Ok(e) => e.into_iter().map(std::borrow::Cow::into_owned).collect(),
                Err(_) => {
                    return Err(err(format!("could not find interpreter \"{path}\"")));
                }
            }
        } else {
            vec![path.to_string()]
        };
        let mut cur = self.cur;
        for name in &elems {
            let next = self
                .st_of(cur)
                .and_then(|st| st.children.get(name.as_bytes()).copied());
            match next {
                Some(c) if self.interp_alive(c) => cur = c,
                _ => return Err(err(format!("could not find interpreter \"{path}\""))),
            }
        }
        Ok(cur)
    }
}
