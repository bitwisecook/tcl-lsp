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

//! Binding a fresh command token over an existing binding: the publication stages and the checks that a retained publication is still current.

use super::super::{
    native_command_recreation, native_operation_currency, native_procedure_resources, Interp,
    OriginalCommandPublication,
};
use super::Command;
use crate::namespace::NsId;
use crate::obj;
use std::rc::Rc;

impl Interp {
    /// Install a fresh command token at an exact namespace binding, applying
    /// Tcl's command-replacement lifecycle first. The displaced command's
    /// delete trace runs while it is still visible, and all command/execution
    /// trace sidecars belonging to that old token are discarded before the new
    /// command is bound. Fresh bindings use the same funnel (the lifecycle step
    /// is then a no-op).
    pub(crate) fn bind_command_replacement(&mut self, ns: NsId, tail: &[u8], command: Command) {
        self.bind_command_replacement_with_jim_key(ns, tail, command, None);
    }

    pub(super) fn bind_command_replacement_with_jim_key(
        &mut self,
        ns: NsId,
        tail: &[u8],
        command: Command,
        incoming: Option<tcl_syntax::naming::NativeJimCommandTableKey>,
    ) -> OriginalCommandPublication {
        let scope = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => {
                self.refuse_native_execution(first);
                drop(command);
                return OriginalCommandPublication::Refused;
            }
        };
        let reached = self.generated_jim_command_key(
            ns,
            tail,
            tcl_syntax::naming::NativeNamePurpose::CommandCApiPublication,
        );
        if scope.currency().ensure_current_or_refuse().is_err() {
            return OriginalCommandPublication::Refused;
        }
        let original_key = match reached {
            Ok(key) => key,
            Err(error) => {
                self.report_cmd_error(error.into());
                return OriginalCommandPublication::Refused;
            }
        };
        let reached = self.bind_command_replacement_with_original_jim_key(
            ns,
            tail,
            command,
            incoming,
            original_key,
        );
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            self.refuse_native_execution(first);
            return OriginalCommandPublication::Refused;
        }
        reached
    }

    pub(in super::super) fn bind_command_replacement_with_original_jim_key(
        &mut self,
        ns: NsId,
        tail: &[u8],
        command: Command,
        incoming: Option<tcl_syntax::naming::NativeJimCommandTableKey>,
        original_key: Option<Rc<obj::Owned>>,
    ) -> OriginalCommandPublication {
        let scope = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => {
                self.refuse_native_execution(first);
                drop(command);
                drop(original_key);
                return OriginalCommandPublication::Refused;
            }
        };
        // The original registration temporary is independent of keyDup. An Rc
        // lease adds no native reference and settles after every entered exit.
        let registration_name = original_key.as_ref().map(Rc::clone);
        let reached = self.bind_command_replacement_entered(
            ns,
            tail,
            command,
            incoming,
            original_key,
            &scope,
        );
        drop(registration_name);
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            self.refuse_native_execution(first);
            return OriginalCommandPublication::Refused;
        }
        if let OriginalCommandPublication::Published(generation) = reached {
            if !self.original_command_publication_is_current(ns, tail, generation) {
                return OriginalCommandPublication::Refused;
            }
        }
        reached
    }

    fn bind_command_replacement_entered(
        &mut self,
        ns: NsId,
        tail: &[u8],
        command: Command,
        incoming: Option<tcl_syntax::naming::NativeJimCommandTableKey>,
        original_key: Option<Rc<obj::Owned>>,
        scope: &native_operation_currency::NativeOperationScope,
    ) -> OriginalCommandPublication {
        // RegisterCommand/QualifyName owns this temporary through replacement
        // callbacks and publication, separately from a fresh table-key retain.
        // Retain the actual holder before any trace or delete procedure. Its
        // name and NsId alone cannot admit a recreated namespace incarnation.
        let namespace = self
            .namespaces()
            .namespace_name_token(self.native_command_interpreter, ns);
        let Some(namespace) = namespace else {
            self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original command publication namespace retired",
                ),
            );
            return OriginalCommandPublication::Refused;
        };
        let lifecycle = namespace.lifecycle();
        if !self.original_publication_namespace_is_current(ns, &namespace, lifecycle) {
            return OriginalCommandPublication::Refused;
        }
        let native_entry = self
            .namespaces
            .borrow_mut()
            .native_command_creation_entry_with_original_key(ns, tail, incoming, original_key);
        if self.jim_local_depth.get() != 0
            && self
                .native_invocation_dialect()
                .native_jim_local_protocol()
                .is_some()
        {
            let generation =
                self.namespaces
                    .borrow_mut()
                    .bind_jim_local(native_entry, tail, command);
            if self.retire_pending_native_ensemble_roles().is_err() {
                return OriginalCommandPublication::Refused;
            }
            if scope.currency().ensure_current_or_refuse().is_err()
                || !self.original_publication_namespace_is_current(ns, &namespace, lifecycle)
                || !self.original_command_publication_is_current(ns, tail, generation)
            {
                return OriginalCommandPublication::Refused;
            }
            return OriginalCommandPublication::Published(generation);
        }
        let creation_protocol = self
            .native_invocation_dialect()
            .native_command_creation_protocol();
        let creates_object_command = matches!(&command, Command::ObjCmd(command)
            if command.creation_purpose == Some(tcl_native_model::native_command_creation::NativeObjectCommandCreationPurpose::LegacyPublicObjectCommand));
        let displaced = self.namespaces.borrow().command_in(ns, tail);
        let original_object_retirement =
            native_command_recreation::OriginalIdleObjectRetirement::capture(displaced.as_ref());
        let had_displaced = displaced.is_some();
        let slot_was_empty = if self.uses_original_c_command_retirement() {
            // C creation calls the same actual token deletion even while the
            // old command has a live invocation. The creation entry and its
            // callback-recreated disposal remain independent producer purposes.
            let original_generation = self.namespaces.borrow().command_generation(ns, tail);
            if let Some(original_generation) = original_generation {
                if let Err(error) = self.retire_original_command_for_creation(original_generation) {
                    self.report_cmd_error(error);
                    return OriginalCommandPublication::Refused;
                }
                if scope.currency().ensure_current_or_refuse().is_err()
                    || !self.original_publication_namespace_is_current(ns, &namespace, lifecycle)
                {
                    return OriginalCommandPublication::Refused;
                }
                // The original exact slot is withdrawn. A different generation
                // left by its delete callback has its separate disposal receipt.
                self.namespaces.borrow().command_generation(ns, tail) != Some(original_generation)
            } else {
                true
            }
        } else {
            self.on_bound_command_replaced(ns, tail);
            if scope.currency().ensure_current_or_refuse().is_err()
                || !self.original_publication_namespace_is_current(ns, &namespace, lifecycle)
            {
                return OriginalCommandPublication::Refused;
            }
            if let Some(owner) = displaced.as_ref().and_then(Command::oo_object) {
                // The old object command's delete trace has run while the token was
                // still visible. Unlink that exact binding before TclOO teardown so
                // identity-based cleanup cannot remove the replacement installed
                // below, then run the displaced owner's destructor and cascades.
                let still_displaced =
                    self.namespaces
                        .borrow()
                        .command_in(ns, tail)
                        .is_some_and(|current| {
                            displaced
                                .as_ref()
                                .is_some_and(|old| old.is_same_binding(&current))
                        });
                if still_displaced {
                    self.namespaces
                        .borrow_mut()
                        .remove_in_for_replacement(ns, tail);
                }
                self.oo_command_renamed(owner, None);
                if scope.currency().ensure_current_or_refuse().is_err()
                    || !self.original_publication_namespace_is_current(ns, &namespace, lifecycle)
                {
                    return OriginalCommandPublication::Refused;
                }
            }
            // Keep the actual old handle while unlinking its exact slot. Releasing
            // it outside the table borrow runs an idle extension's deleteProc before
            // publication; a running call's independent Rc still defers retirement.
            let still_displaced =
                self.namespaces
                    .borrow()
                    .command_in(ns, tail)
                    .is_some_and(|current| {
                        displaced
                            .as_ref()
                            .is_some_and(|old| old.is_same_binding(&current))
                    });
            if still_displaced {
                self.namespaces
                    .borrow_mut()
                    .remove_in_for_replacement(ns, tail);
            }
            self.namespaces.borrow().command_in(ns, tail).is_none()
        };
        drop(displaced);
        if self.retire_pending_native_ensemble_roles().is_err() {
            return OriginalCommandPublication::Refused;
        }
        if scope.currency().ensure_current_or_refuse().is_err()
            || !self.original_publication_namespace_is_current(ns, &namespace, lifecycle)
        {
            return OriginalCommandPublication::Refused;
        }
        if !self
            .namespaces()
            .original_jim_creation_slot_is_current(&native_entry)
        {
            self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original Jim command key slot retired",
                ),
            );
            return OriginalCommandPublication::Refused;
        }
        // The original idle public object-command overwrite discards a record
        // created by the old deleteProc. Reuse the exact creation entry, actual
        // slot generation and unique record ownership; a spelling is no proof.
        let recreated_generation = self.namespaces.borrow().command_generation(ns, tail);
        if let (true, Some(recreated_generation)) = (creates_object_command, recreated_generation) {
            let admissible = original_object_retirement
                .as_ref()
                .is_some_and(|original| original.completed_delete(slot_was_empty));
            let selected = creation_protocol.filter(|_| admissible);
            let Some(selected) = selected else {
                self.refuse_native_access(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "original idle object-command recreation purpose unavailable",
                    ),
                );
                return OriginalCommandPublication::Refused;
            };
            let recreated = self
                .namespaces
                .borrow_mut()
                .take_recreated_object_for_creation(&native_entry, tail, recreated_generation);
            let discarded = recreated.is_some_and(|recreated| {
                // Release only the actual intermediate object outside the table
                // borrow. Additional owners keep their ordinary retirement.
                native_command_recreation::discard_idle_recreated_object(
                    selected,
                    recreated.command,
                )
                .is_ok()
            });
            if scope.currency().ensure_current_or_refuse().is_err()
                || !self.original_publication_namespace_is_current(ns, &namespace, lifecycle)
            {
                return OriginalCommandPublication::Refused;
            }
            if !discarded {
                self.refuse_native_access(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "original idle object-command recreation ownership unavailable",
                    ),
                );
                return OriginalCommandPublication::Refused;
            }
        }
        // Other publication purposes keep their ordinary retained-handle
        // retirement; no idle object-command receipt is donated to them.
        let recreated = self.namespaces.borrow().command_in(ns, tail);
        if creation_protocol.is_some() && recreated.is_some() {
            // Native C and Jim overwrite a callback-created token without a
            // second payload delete. Only the existing original idle ObjCmd
            // disposal above carries that purpose; Proc payload retirement is
            // not inferred from its spelling or a surviving metadata handle.
            self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "callback-recreated command payload disposal purpose unavailable",
                ),
            );
            return OriginalCommandPublication::Refused;
        }
        let generation = if self
            .name_policy_protocol()
            .is_some_and(|protocol| protocol.recipe().is_jim084())
        {
            self.namespaces
                .borrow_mut()
                .remove_in_for_replacement(ns, tail);
            drop(recreated);
            if self.retire_pending_native_ensemble_roles().is_err() {
                return OriginalCommandPublication::Refused;
            }
            if scope.currency().ensure_current_or_refuse().is_err()
                || !self.original_publication_namespace_is_current(ns, &namespace, lifecycle)
            {
                return OriginalCommandPublication::Refused;
            }
            if !self
                .namespaces()
                .original_jim_creation_slot_is_current(&native_entry)
            {
                self.refuse_native_access(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "original Jim command key slot retired",
                    ),
                );
                return OriginalCommandPublication::Refused;
            }
            self.namespaces
                .borrow_mut()
                .bind_after_native_creation_entry(native_entry, tail, command)
        } else {
            let generation = self
                .namespaces
                .borrow_mut()
                .bind_after_native_creation_entry(native_entry, tail, command);
            drop(recreated);
            generation
        };
        if scope.currency().ensure_current_or_refuse().is_err()
            || !self.original_publication_namespace_is_current(ns, &namespace, lifecycle)
        {
            return OriginalCommandPublication::Refused;
        }
        if self.retire_pending_native_ensemble_roles().is_err() {
            return OriginalCommandPublication::Refused;
        }
        if scope.currency().ensure_current_or_refuse().is_err()
            || !self.original_publication_namespace_is_current(ns, &namespace, lifecycle)
        {
            return OriginalCommandPublication::Refused;
        }
        if !self.original_command_publication_is_current(ns, tail, generation) {
            return OriginalCommandPublication::Refused;
        }
        if had_displaced {
            let fqn = self.namespaces().command_fqn_at(ns, tail);
            self.reattach_missing_import_sources(&fqn, generation);
        }
        OriginalCommandPublication::Published(generation)
    }

    /// Fixed original slot continuation; a renamed token is not a new receipt.
    pub(in super::super) fn original_command_publication_is_current(
        &mut self,
        namespace: NsId,
        name: &[u8],
        generation: u64,
    ) -> bool {
        if self
            .namespaces()
            .original_publication_is_current(namespace, name, generation)
        {
            return true;
        }
        self.refuse_native_access(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "original command publication binding changed",
            ),
        );
        false
    }

    pub(in super::super) fn check_original_procedure_publication(
        &mut self,
        namespace: NsId,
        name: &[u8],
        generation: u64,
        original: &native_procedure_resources::OriginalProcedureBinding,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        self.check_entered_native_operation()?;
        if self
            .namespaces()
            .original_procedure_publication_is_current(namespace, name, generation, original)
        {
            return Ok(());
        }
        self.refuse_native_access(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "original procedure publication binding changed",
            ),
        );
        Err(self
            .native_execution_refusal()
            .expect("the exact original binding refusal was retained"))
    }

    /// Check the retained holder without resolving its reporting name again.
    /// A caller's original dying OO holder remains distinct from a live holder
    /// that a callback retires. The permanent root retains its existing token.
    pub(crate) fn original_publication_namespace_is_current(
        &mut self,
        ns: NsId,
        namespace: &tcl_runtime_api::native_namespace_name::NativeNamespaceNameToken,
        lifecycle: tcl_syntax::native_namespace_name::NativeNamespaceLifecycle,
    ) -> bool {
        use tcl_syntax::native_namespace_name::NativeNamespaceLifecycle;
        let current = {
            let namespaces = self.namespaces();
            namespaces.owns_namespace_name_token(namespace)
                && namespace.lifecycle() == lifecycle
                && match lifecycle {
                    NativeNamespaceLifecycle::Live => namespaces.namespace_is_live(ns),
                    NativeNamespaceLifecycle::Dying => true,
                    NativeNamespaceLifecycle::Dead => false,
                }
        };
        if !current {
            self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original command publication namespace retired",
                ),
            );
        }
        current
    }

    /// Fire deletion traces for the exact binding the caller already holds.
    /// A fully-qualified name is not a token identity once a retained
    /// namespace and a same-named recreation both hold `::N::q`: resolving the
    /// name again would find the live one and fire its traces instead. An
    /// unbound slot has no token, so — like C's `TclCreateObjCommandInNs`,
    /// which only deletes when `Tcl_FindHashEntry` found an entry — nothing
    /// fires and nothing is dropped.
    pub(crate) fn on_bound_command_replaced(&mut self, ns: NsId, tail: &[u8]) {
        let (fqn, dying) = {
            let namespaces = self.namespaces.borrow();
            (
                namespaces.command_fqn_at(ns, tail),
                namespaces.command_generation(ns, tail),
            )
        };
        if dying.is_none() {
            return;
        }
        self.fire_delete_traces_of_token(&fqn, dying);
        if self.check_entered_native_operation().is_err() {
            return;
        }
        if let Some(generation) = dying {
            crate::cmd_coro::on_command_deleted(self, generation);
            let _ = self.check_entered_native_operation();
        }
    }
}
