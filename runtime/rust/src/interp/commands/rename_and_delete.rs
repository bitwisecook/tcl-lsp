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

//! `rename` and command deletion: moving a bound command, deleting it, and keeping only a selected set.

use super::super::{native_operation_currency, Interp};
use super::Command;
use crate::namespace::{CommandBinding, RenameOutcome};
use crate::obj;
use std::rc::Rc;

impl Interp {
    /// `rename old new` (or `rename old ""` to delete), relative to the current
    /// namespace. Drives the one command table: the guards C's
    /// `TclRenameCommand` applies before anything observable happens, then
    /// [`Self::move_bound_command`] or [`Self::delete_bound_command`].
    pub(crate) fn rename_command(&mut self, old: &[u8], new: &[u8]) -> RenameOutcome {
        let _scope = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => {
                self.refuse_native_execution(first);
                return RenameOutcome::NoSuchCommand;
            }
        };
        let original_key = if new.is_empty() {
            None
        } else {
            match self.generated_jim_command_key(
                self.current_ns.get(),
                new,
                tcl_syntax::naming::NativeNamePurpose::RenameDestination,
            ) {
                Ok(key) => key,
                Err(error) => {
                    self.report_cmd_error(error.into());
                    return RenameOutcome::NoSuchCommand;
                }
            }
        };
        self.rename_command_with_original_jim_key(old, new, original_key)
    }

    pub(crate) fn rename_command_with_original_jim_key(
        &mut self,
        old: &[u8],
        new: &[u8],
        original_key: Option<Rc<obj::Owned>>,
    ) -> RenameOutcome {
        let scope = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => {
                self.refuse_native_execution(first);
                return RenameOutcome::NoSuchCommand;
            }
        };
        if self
            .name_policy_protocol()
            .is_some_and(|protocol| protocol.recipe().is_jim084())
        {
            if self
                .namespaces()
                .original_jim_rename_source_of(self.current_ns.get(), old)
                .is_none()
            {
                return RenameOutcome::NoSuchCommand;
            }
            if let Some(key) = original_key.as_ref() {
                let reached = self.native_object_string_bytes(key.as_ptr());
                if scope.currency().ensure_current_or_refuse().is_err() {
                    return RenameOutcome::NoSuchCommand;
                }
                if let Err(error) = reached {
                    self.report_cmd_error(error.into());
                    return RenameOutcome::NoSuchCommand;
                }
            }
        }
        // C's `TclPreventAliasLoop` guards `rename` too, and refuses before
        // anything observable happens — no rename trace fires for a rename that
        // does not take place.
        let probe =
            self.namespaces
                .borrow_mut()
                .begin_alias_rename_probe(self.current_ns.get(), old, new);
        if let Some(probe) = probe {
            let loops = self.alias_chain_loops_original(probe.destination.0, &probe.destination.1);
            let retired = self
                .namespaces
                .borrow_mut()
                .restore_alias_rename_probe(probe);
            drop(retired);
            match loops {
                Ok(true) => return RenameOutcome::AliasLoop,
                Ok(false) => {}
                Err(error) => {
                    // The original command caller checks retained first Host
                    // before interpreting this internal outcome or publishing.
                    self.report_cmd_error(error);
                    return RenameOutcome::AliasLoop;
                }
            }
        }
        // C's `TclRenameCommand` checks the destination's hash table before
        // touching `old`'s (tclBasic.c), so an occupied destination — self-
        // rename onto the same slot included — is refused before anything
        // observable happens, same as the alias-loop guard above. A
        // release-gated TclOO root this build hides reads as free here too
        // (`is_gate_hidden_object_root`), same as every other "is this name
        // taken?" check.
        let occupant_fqn = self
            .namespaces
            .borrow_mut()
            .destination_occupant_fqn(self.current_ns.get(), new);
        if let Some(occupant_fqn) = occupant_fqn {
            if self.is_gate_hidden_object_root(&occupant_fqn) {
                self.retire_gate_hidden_object_root(&occupant_fqn);
            } else {
                return RenameOutcome::TargetExists;
            }
        }
        // A builtin the emulated release does not carry is not there to be
        // renamed or deleted: rebinding it under a name the registry has no
        // spec for would hand it back ungated, defeating the availability
        // mask outright. Checked before any observable effect, like the
        // alias-loop refusal above.
        let bound = self
            .namespaces
            .borrow()
            .resolve(self.current_ns.get(), old)
            .is_some();
        if bound
            && self
                .resolve_dispatchable(self.current_ns.get(), old)
                .is_none()
        {
            return RenameOutcome::NoSuchCommand;
        }
        if new.is_empty() {
            self.delete_bound_command(old)
        } else {
            self.move_bound_command(old, new, original_key)
        }
    }

    /// Publish the new entry before callbacks; both slots retain one generation.
    pub(in super::super) fn move_bound_command(
        &mut self,
        old: &[u8],
        new: &[u8],
        original_key: Option<Rc<obj::Owned>>,
    ) -> RenameOutcome {
        // Both Jim qualification temporaries survive AddHashEntry and the
        // original source-slot retirement; keyDup owns a separate reference.
        let _qualified_destination = original_key.as_ref().map(Rc::clone);
        let Some(old_fqn) = self
            .resolve_cmd_token(old)
            .and_then(|generation| self.namespaces().command_fqn_by_generation(generation))
        else {
            return RenameOutcome::NoSuchCommand;
        };
        let oo_object = match self.namespaces.borrow().resolve(self.current_ns.get(), old) {
            Some(Command::OoObject(id)) => Some(id),
            _ => None,
        };
        let Some(publication) = self
            .namespaces
            .borrow_mut()
            .publish_rename_destination_with_original_key(
                self.current_ns.get(),
                old,
                new,
                original_key,
            )
        else {
            return RenameOutcome::NoSuchCommand;
        };
        if self.retire_pending_native_ensemble_roles().is_err() {
            self.namespaces
                .borrow_mut()
                .retire_rename_source(&publication);
            if let Err(first) = self.retire_pending_native_ensemble_roles() {
                self.refuse_native_execution(first);
            }
            return RenameOutcome::Renamed;
        }
        let new_fqn = publication.destination_fqn.clone();
        // The trace list (and any OO object) follows to the new name, and so
        // does every `namespace import` redirect of the old name — C's imports
        // hold the source's command token, so they survive a source rename
        // (tclsh-pinned; see `Namespaces::retarget_imports`). All of it happens
        // here, before the callbacks, because C had moved its one `Command`
        // before it fired them.
        self.move_cmd_traces(&old_fqn, &new_fqn, publication.generation);
        self.retarget_import_sources(publication.generation, &new_fqn);
        crate::cmd_coro::on_command_renamed(self, publication.generation, &new_fqn);
        if let Some(object) = oo_object {
            self.oo_command_renamed(object, Some(&new_fqn));
        }
        self.fire_cmd_trace_of_token(
            &new_fqn,
            &old_fqn,
            &new_fqn,
            crate::cmd_trace::ops::RENAME,
            Some(publication.generation),
        );
        // C's `Tcl_DeleteHashEntry(oldHPtr)` at the tail of `TclRenameCommand`:
        // a plain table removal, firing no `delete` trace, after which the
        // command stands under its new name alone. Whatever occupies the slot
        // goes — C's captured entry pointer does not care either, and a
        // callback that rebound the vacating name left C dereferencing freed
        // memory, so there is no behaviour there to match.
        self.namespaces
            .borrow_mut()
            .retire_rename_source(&publication);
        if let Err(first) = self.retire_pending_native_ensemble_roles() {
            self.refuse_native_execution(first);
        }
        RenameOutcome::Renamed
    }

    /// The `rename old ""` half, after [`Self::rename_command`]'s guards — C's
    /// `Tcl_DeleteCommandFromToken`, whose `delete` traces fire while the
    /// command is still bound under its name.
    pub(in super::super) fn delete_bound_command(&mut self, old: &[u8]) -> RenameOutcome {
        if self.uses_original_c_command_retirement() {
            let Some(generation) = self.resolve_cmd_token(old) else {
                return RenameOutcome::NoSuchCommand;
            };
            if let Err(error) = self.retire_original_command_generation(generation, true) {
                self.report_cmd_error(error);
            }
            return RenameOutcome::Deleted;
        }

        let (ensemble_token, import_token) =
            match self.namespaces.borrow().resolve(self.current_ns.get(), old) {
                Some(Command::Ensemble(token)) => (Some(token), None),
                Some(Command::Imported { identity, .. }) => (None, Some(identity)),
                _ => (None, None),
            };
        // Command traces fire *before* the table mutation (the command still
        // exists under its name during the callback), with the fully-qualified
        // old name and an empty new one. C deletes the command *token* it
        // captured here, not whatever the name holds when the callback returns
        // — so the binding is captured alongside the name.
        let bound_before = self.namespaces.borrow().resolve(self.current_ns.get(), old);
        let oo_object = match &bound_before {
            Some(Command::OoObject(id)) => Some(*id),
            _ => None,
        };
        // The token whose trace list this deletion frees, captured before the
        // callbacks can bind a replacement at the same name.
        let dying_token = self.resolve_cmd_token(old);
        let old_fqn = self.resolve_cmd_fqn(old);
        if let Some(of) = &old_fqn {
            if !self.traces.borrow().cmd_traces.is_empty() {
                self.fire_cmd_trace_of_token(
                    of,
                    of,
                    b"",
                    crate::cmd_trace::ops::DELETE,
                    dying_token,
                );
            }
        }
        let existed = old_fqn.is_some();
        // Deleting a suspended coroutine's command tears down its worker first.
        if let Some(generation) = dying_token {
            crate::cmd_coro::on_command_deleted(self, generation);
        }
        // An imported command carries a stable identity. Its delete trace may
        // force-reimport or otherwise replace the same binding; delete the
        // captured old identity wherever it moved, never the callback's fresh
        // command at the old name.
        let removed_import_fqn = import_token
            .as_ref()
            .and_then(|identity| self.remove_import_identity(identity));
        // A delete-trace callback that re-creates the command (`proc foo {} …`)
        // has bound a *new* command at the old name. C's captured token is
        // `CMD_DYING` and no longer owns the hash entry, so its deletion leaves
        // the fresh command standing (`Tcl_DeleteCommandFromToken`,
        // tclBasic.c) — `foo` still exists, and calls the new body. Deleting
        // "whatever is at the name now" would remove the callback's work
        // instead. This is the command half of the rule the import branch just
        // above already applies to its own identity.
        let recreated = match (
            &bound_before,
            self.namespaces.borrow().resolve(self.current_ns.get(), old),
        ) {
            (Some(before), Some(now)) => !before.is_same_binding(&now),
            _ => false,
        };
        let raw = if recreated {
            RenameOutcome::Deleted
        } else if import_token.is_some() {
            if removed_import_fqn.is_some() {
                RenameOutcome::Deleted
            } else {
                RenameOutcome::NoSuchCommand
            }
        } else if self
            .namespaces
            .borrow_mut()
            .delete(self.current_ns.get(), old)
        {
            RenameOutcome::Deleted
        } else {
            RenameOutcome::NoSuchCommand
        };
        drop(bound_before);
        if let Err(first) = self.retire_pending_native_ensemble_roles() {
            self.refuse_native_execution(first);
        }
        // A delete-trace callback may itself delete the command (e.g. by
        // deleting the object's namespace). C captured the command token before
        // the callback, so the deletion still succeeds — treat "existed at
        // entry, gone now" as a normal delete (cleanup is idempotent) rather
        // than reporting "command doesn't exist".
        let outcome = if existed && matches!(raw, RenameOutcome::NoSuchCommand) {
            RenameOutcome::Deleted
        } else {
            raw
        };
        // The command is gone; the dying token's traces and OO registry entry
        // go with it. A replacement the delete callback bound at the same name
        // keeps its own traces (C frees only `cmdPtr->tracePtr`).
        if let (Some(of), RenameOutcome::Deleted) = (old_fqn, outcome) {
            self.remove_cmd_traces_of_token(&of, dying_token);
            let tokens: Vec<_> = ensemble_token.into_iter().collect();
            let mut origins: Vec<u64> = dying_token.into_iter().collect();
            if let Some((removed_fqn, removed_generation)) = removed_import_fqn {
                if removed_fqn != of {
                    self.remove_cmd_traces_of_token(&removed_fqn, Some(removed_generation));
                }
                origins.push(removed_generation);
            }
            self.remove_imports_for_deleted_origins(origins, &tokens);
            if let Some(object) = oo_object {
                self.oo_command_renamed(object, None);
            }
        }
        if let Err(first) = self.retire_pending_native_ensemble_roles() {
            self.refuse_native_execution(first);
        }
        outcome
    }

    /// Delete the command bound to `name` (the alias-clear form); returns whether
    /// it existed.
    pub(crate) fn delete_command(&mut self, name: &[u8]) -> bool {
        if self.uses_original_c_command_retirement() {
            let Some(generation) = self.resolve_cmd_token(name) else {
                return false;
            };
            return self.finish_original_command_deletion(generation, true);
        }

        let _scope = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => {
                self.refuse_native_execution(first);
                return false;
            }
        };
        // If `name` is a suspended coroutine, terminate its worker first.
        let source_generation = self.resolve_cmd_token(name);
        if let Some(generation) = source_generation {
            crate::cmd_coro::on_command_deleted(self, generation);
        }
        let ensemble_token = match self
            .namespaces
            .borrow()
            .resolve(self.current_ns.get(), name)
        {
            Some(Command::Ensemble(token)) => Some(token),
            _ => None,
        };
        let deleted = self
            .namespaces
            .borrow_mut()
            .delete(self.current_ns.get(), name);
        let retirement = self.retire_pending_native_ensemble_roles();
        if deleted {
            if let Some(source_generation) = source_generation {
                let tokens: Vec<_> = ensemble_token.into_iter().collect();
                self.remove_imports_for_deleted_origins([source_generation], &tokens);
            }
        }
        let settled = self.retire_pending_native_ensemble_roles();
        if let Err(first) = retirement.and(settled) {
            self.refuse_native_execution(first);
        }
        deleted
    }

    /// Remove one command generation without recovering an address from its report.
    pub(crate) fn delete_command_generation(&mut self, generation: u64) -> bool {
        if self.uses_original_c_command_retirement() {
            return self.finish_original_command_deletion(generation, true);
        }

        let _scope = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => {
                self.refuse_native_execution(first);
                return false;
            }
        };
        if self.check_entered_native_operation().is_err() {
            return false;
        }
        let original = self.raw_command_by_generation(generation);
        crate::cmd_coro::on_command_deleted(self, generation);
        if self.check_entered_native_operation().is_err() {
            return false;
        }
        let slot = self.namespaces().native_command_slot_at_node(generation);
        let removed = if let Some((namespace, simple)) = slot {
            self.namespaces_mut().remove_in(namespace, &simple)
        } else {
            let hidden = self.hidden.borrow().iter().find_map(|(name, binding)| {
                (binding.generation == generation).then(|| name.clone())
            });
            let retired = hidden.and_then(|name| self.hidden.borrow_mut().remove(&name));
            let removed = retired.is_some();
            if !self
                .name_policy_protocol()
                .is_some_and(|protocol| protocol.recipe().is_jim084())
            {
                if let Some(CommandBinding {
                    command: Command::Proc(procedure),
                    ..
                }) = retired.as_ref()
                {
                    drop(procedure.take_for_retirement());
                }
            }
            drop(retired);
            removed
        };
        drop(original);
        let retirement = self.retire_pending_native_ensemble_roles();
        if removed {
            self.namespaces_mut().retire_native_command_node(generation);
            self.remove_imports_for_deleted_origins([generation], &[]);
            if self.retire_pending_native_ensemble_roles().is_err() {
                return false;
            }
        }
        // The original extension delete callback has run outside every table borrow.
        if retirement.is_err() || self.check_entered_native_operation().is_err() {
            return false;
        }
        removed
    }

    /// Delete every command, in every namespace, whose unrooted name (`set`,
    /// `tcl::mathfunc::abs`) `keep` refuses: the whitelist an engine confines
    /// a body to. A command `keep` names keeps its own binding; nothing is
    /// renamed or replaced.
    pub fn retain_commands(
        &mut self,
        keep: &dyn Fn(&str) -> bool,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        self.retain_command_tokens(&|_, report| core::str::from_utf8(report).is_ok_and(keep))
    }

    /// Keep exact live command generations selected by their original table owners.
    /// Reporting bytes are provided for a caller's byte whitelist; they never
    /// supply the generation used for deletion.
    pub(crate) fn retain_command_tokens(
        &mut self,
        keep: &dyn Fn(u64, &[u8]) -> bool,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        if let Some(cause) = self.native_execution_refusal() {
            return Err(cause);
        }
        let refused: Vec<_> = {
            let namespaces = self.namespaces.borrow();
            namespaces
                .native_command_generations()
                .into_iter()
                .filter(|&generation| {
                    namespaces
                        .native_command_slot_at_node(generation)
                        .is_some_and(|(ns, simple)| {
                            let report = namespaces.command_fqn_at(ns, &simple);
                            !keep(
                                generation,
                                tcl_syntax::naming::unroot_rooted_key_bytes(&report)
                                    .unwrap_or(&report),
                            )
                        })
                })
                .collect()
        };
        for generation in refused {
            self.delete_command_generation(generation);
            if let Some(cause) = self.native_execution_refusal() {
                return Err(cause);
            }
        }
        Ok(())
    }
}
