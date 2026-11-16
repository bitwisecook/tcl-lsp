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

//! `interp hide`, `expose`, `invokehidden` and `hidden`: moving a command between the command table and the hidden table.

use super::super::{native_operation_currency, Code, Interp};
use super::{Command, CommandVisibilityOp, CommandVisibilityOutcome};
use crate::namespace::GLOBAL;
use crate::obj::TclObj;
use std::rc::Rc;

impl Interp {
    /// `interp hide name`: move command `name` out of the command table into
    /// the hidden table under `hidden_name`. `Missing` when `name` does not
    /// resolve, `NonGlobal` when it resolves outside the global namespace,
    /// `Collision` when `hidden_name` is already hidden, `Moved` on success —
    /// never a bare success flag, because the caller has to word four
    /// different diagnostics, and C's order between them is observable.
    pub(crate) fn hide_command(
        &mut self,
        name: &[u8],
        hidden_name: &[u8],
    ) -> CommandVisibilityOutcome {
        let _scope = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => {
                self.refuse_native_execution(first);
                return CommandVisibilityOutcome::Missing;
            }
        };
        // Gated: a builtin the emulated release does not carry is not there to
        // be hidden, so `interp hide` cannot park it in the hidden table where
        // `interp invokehidden` would reach it past the surface check.
        let resolved = self.resolve_dispatchable(GLOBAL, name);
        match resolved {
            Some(_) => {
                // C's order, and it is observable: `Tcl_HideCommand` resolves
                // the source before it rejects a non-global one, and rejects a
                // non-global one before it refuses an occupied token
                // (tclBasic.c:2325, :2339, :2365). `interp hide kid nosuch
                // taken` is therefore `unknown command "nosuch"`, not the
                // collision.
                //
                // The global test goes through the shared namespace owner, so a
                // run of colons collapses the way `TclGetNamespaceForQualName`
                // collapses it: `::::foo` names the global `foo` (tclsh-pinned).
                if !tcl_cmd_core::namespace::qualifiers(name).is_empty() {
                    return CommandVisibilityOutcome::NonGlobal;
                }
                if self.hidden.borrow().contains_key(hidden_name) {
                    return CommandVisibilityOutcome::Collision;
                }
                self.invalidate_interpreter_policy();
                let old_fqn = self.namespaces.borrow().resolve_fqn(GLOBAL, name);
                let Some(binding) = self.namespaces.borrow_mut().take(GLOBAL, name) else {
                    return CommandVisibilityOutcome::Missing;
                };
                if self.retire_pending_native_ensemble_roles().is_err() {
                    return CommandVisibilityOutcome::Missing;
                }
                let hidden_fqn = tcl_syntax::naming::root_unrooted_key_bytes(hidden_name);
                if let Some(old_fqn) = &old_fqn {
                    self.move_cmd_traces(old_fqn, &hidden_fqn, binding.generation);
                    self.retarget_import_sources(binding.generation, &hidden_fqn);
                }
                if let Command::Ensemble(token) = &binding.command {
                    token.rename(hidden_fqn);
                }
                self.namespaces
                    .borrow_mut()
                    .note_native_command_hidden(binding.generation);
                self.hidden
                    .borrow_mut()
                    .insert(hidden_name.to_vec(), binding);
                CommandVisibilityOutcome::Moved
            }
            None => CommandVisibilityOutcome::Missing,
        }
    }

    /// `interp expose name`: move a hidden command back into the command table.
    pub(crate) fn expose_command(
        &mut self,
        hidden_name: &[u8],
        name: &[u8],
    ) -> CommandVisibilityOutcome {
        let scope = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => {
                self.refuse_native_execution(first);
                return CommandVisibilityOutcome::Missing;
            }
        };
        // C's order: `Tcl_ExposeCommand` looks the token up (tclBasic.c:2486)
        // before it examines the destination (:2525), so `interp expose kid
        // nosuchtok taken` is `unknown hidden command "nosuchtok"`, not the
        // collision.
        if !self.hidden.borrow().contains_key(hidden_name) {
            return CommandVisibilityOutcome::Missing;
        }
        if self.namespaces.borrow().resolve_fqn(GLOBAL, name).is_some() {
            return CommandVisibilityOutcome::Collision;
        }
        let original_key = match self.generated_jim_command_key(
            GLOBAL,
            name,
            tcl_syntax::naming::NativeNamePurpose::CommandCApiPublication,
        ) {
            Ok(key) => key,
            Err(error) => {
                self.report_cmd_error(error.into());
                return CommandVisibilityOutcome::Missing;
            }
        };
        // Invalidate before removing from the hidden table. A missing entry may
        // over-invalidate, which is preferable to a re-entrant visibility gap.
        self.invalidate_interpreter_policy();
        let binding = self.hidden.borrow_mut().remove(hidden_name);
        match binding {
            Some(binding) => {
                let old_hidden_fqn = tcl_syntax::naming::root_unrooted_key_bytes(hidden_name);
                let generation = binding.generation;
                let ensemble = match &binding.command {
                    Command::Ensemble(token) => Some(Rc::clone(token)),
                    _ => None,
                };
                let restored = {
                    let mut namespaces = self.namespaces.borrow_mut();
                    let slot = namespaces.exposed_command_publication_at(name);
                    namespaces.restore_binding_at_checked(slot, binding, original_key)
                };
                if let Err(unpublished) = restored {
                    self.refuse_host_command("exposed command has no actual publication slot");
                    if let Err(first) =
                        unpublished.retire_checked(&|| scope.currency().ensure_current_or_refuse())
                    {
                        self.refuse_native_execution(first);
                    }
                }
                let retirement = self.retire_pending_native_ensemble_roles();
                if let Err(first) = retirement {
                    self.refuse_native_execution(first);
                }
                if scope.currency().ensure_current_or_refuse().is_err() {
                    drop(ensemble);
                    return CommandVisibilityOutcome::Missing;
                }
                let new_fqn = self
                    .namespaces
                    .borrow()
                    .command_fqn_by_generation(generation);
                let Some(new_fqn) = new_fqn else {
                    self.refuse_host_command("exposed original command generation was retired");
                    drop(ensemble);
                    return CommandVisibilityOutcome::Missing;
                };
                self.move_cmd_traces(&old_hidden_fqn, &new_fqn, generation);
                self.retarget_import_sources(generation, &new_fqn);
                if let Some(token) = ensemble {
                    token.rename(new_fqn);
                }
                CommandVisibilityOutcome::Moved
            }
            None => CommandVisibilityOutcome::Missing,
        }
    }

    /// Convert the typed result of a hidden-table move into Tcl's public
    /// diagnostic. Both `interp hide/expose` and the child command shorthand
    /// use this seam so missing-source and occupied-destination cases cannot
    /// drift in message or structured error code.
    pub(crate) fn finish_command_visibility(
        &mut self,
        op: CommandVisibilityOp,
        source: &[u8],
        destination: &[u8],
        outcome: CommandVisibilityOutcome,
    ) -> Code {
        let (failure, original) = match (op, outcome) {
            (_, CommandVisibilityOutcome::Moved) => {
                self.set_result_bytes(b"");
                return Code::Ok;
            }
            (CommandVisibilityOp::Hide, CommandVisibilityOutcome::Missing) => (
                tcl_syntax::naming::NativeCommandVisibilityFailure::MissingCommand,
                source,
            ),
            (CommandVisibilityOp::Expose, CommandVisibilityOutcome::Missing) => (
                tcl_syntax::naming::NativeCommandVisibilityFailure::MissingHiddenToken,
                source,
            ),
            (CommandVisibilityOp::Hide, CommandVisibilityOutcome::NonGlobal) => (
                tcl_syntax::naming::NativeCommandVisibilityFailure::HideNonGlobal,
                source,
            ),
            (CommandVisibilityOp::Expose, CommandVisibilityOutcome::NonGlobal) => (
                tcl_syntax::naming::NativeCommandVisibilityFailure::HiddenSourceNonGlobal,
                source,
            ),
            (CommandVisibilityOp::Hide, CommandVisibilityOutcome::Collision) => (
                tcl_syntax::naming::NativeCommandVisibilityFailure::AlreadyHidden,
                destination,
            ),
            (CommandVisibilityOp::Expose, CommandVisibilityOutcome::Collision) => (
                tcl_syntax::naming::NativeCommandVisibilityFailure::CommandExists,
                destination,
            ),
        };
        self.report_command_visibility_failure(failure, original)
    }

    pub(crate) fn report_command_visibility_failure(
        &mut self,
        failure: tcl_syntax::naming::NativeCommandVisibilityFailure,
        original: &[u8],
    ) -> Code {
        let Some(policy) = self.name_policy_protocol() else {
            return self.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "command visibility diagnostic",
                )
                .into(),
            );
        };
        let error = match policy
            .recipe()
            .command_visibility_diagnostic(failure, original)
        {
            Ok(error) => error,
            Err(_) => {
                return self.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "command visibility diagnostic",
                    )
                    .into(),
                );
            }
        };
        match error.error_code {
            Some(code) => self.error_with_code(&error.message, &code),
            None => self.error(&error.message),
        }
    }

    /// `interp invokehidden name ?arg ...?` — invoke a hidden command.
    pub(crate) fn invoke_hidden(&mut self, name: &[u8], argv: &[*mut TclObj]) -> Code {
        let binding = self.hidden.borrow().get(name).cloned();
        match binding {
            Some(binding) => {
                let fqn = tcl_syntax::naming::root_unrooted_key_bytes(name);
                self.dispatch_prebound(argv, Some((fqn, binding)))
            }
            None => self.report_command_visibility_failure(
                tcl_syntax::naming::NativeCommandVisibilityFailure::InvokeHiddenMissing,
                name,
            ),
        }
    }

    /// Sorted names of the hidden commands (`interp hidden`).
    pub(crate) fn hidden_names(&self) -> Vec<Vec<u8>> {
        self.hidden.borrow().keys().cloned().collect()
    }
}
