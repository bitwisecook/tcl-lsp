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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Mandatory command retirement in the actual original interpreter table.
use super::native_operation_currency::{
    CleanupRestoration, NativeOperationScope, SavedExecutionTarget,
};
use super::{
    CommandSidecarKey, CommandTokenIdentity, ImportRetirementNode, InterpId, InterpState,
    NativeNameWorld, Vm,
};
use crate::command::Command;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use tcl_cmd_core::{
    CmdError,
    native_command_retirement::{
        NativeCommandRetirementOrder, NativeCommandRetirementStage, retire_original_command,
    },
};
use tcl_syntax::value::ValueError;

/// Allocation-only physical observation and an independently genuine entered
/// operation. It adds no command/Proc/Namespace or Native reference role.
pub(crate) struct OriginalCommandCleanup {
    target: SavedExecutionTarget,
    interpreter: InterpId,
    world: Weak<RefCell<NativeNameWorld>>,
    operation: Result<NativeOperationScope, CmdError>,
    c_order: Option<NativeCommandRetirementOrder>,
    c_names: Option<tcl_registry::native_command_literal::NativeCommandNameProtocol>,
}
impl OriginalCommandCleanup {
    pub(crate) fn enter(vm: &mut Vm) -> Self {
        let world = Rc::downgrade(&vm.name_world);
        let c_order = vm
            .actual_native_invocation_dialect()
            .native_command_creation_protocol()
            .and_then(tcl_registry::native_command_creation::NativeCommandCreationProtocol::c_deletion_order);
        let c_names = vm
            .actual_native_invocation_dialect()
            .native_command_name_protocol();
        let operation = NativeOperationScope::enter(vm);
        if let Err(first) = &operation {
            super::variable_cleanup::retain(vm, first);
        }
        let target = SavedExecutionTarget::capture(vm);
        let interpreter = target.interpreter();
        Self {
            target,
            interpreter,
            world,
            operation,
            c_order,
            c_names,
        }
    }
    fn unavailable(vm: &mut Vm) -> CmdError {
        let error: CmdError =
            ValueError::CommandProtocolUnavailable("original command retirement table").into();
        super::variable_cleanup::retain(vm, &error);
        vm.execution_refusal
            .clone()
            .map(CmdError::from_execution_refusal)
            .unwrap_or(error)
    }
    fn physical(&self, vm: &Vm) -> bool {
        self.target.owns_target(vm)
            && vm.st_of(self.interpreter).is_some_and(|state| {
                self.world
                    .upgrade()
                    .is_some_and(|world| Rc::ptr_eq(&world, &state.name_world))
            })
    }
    pub(super) fn state_mut<'a>(&self, vm: &'a mut Vm) -> Result<&'a mut InterpState, CmdError> {
        if !self.physical(vm) {
            return Err(Self::unavailable(vm));
        }
        if vm.cur == self.interpreter {
            Ok(&mut vm.state)
        } else {
            Ok(vm.interps[self.interpreter.0]
                .parked
                .as_deref_mut()
                .expect("checked original command state"))
        }
    }
    pub(crate) fn check(&self, vm: &mut Vm) -> Result<(), CmdError> {
        let reached = match &self.operation {
            Ok(operation) => operation.check(vm),
            Err(first) => Err(first.clone()),
        };
        if let Err(first) = &reached {
            super::variable_cleanup::retain(vm, first);
        }
        reached?;
        if !self.physical(vm)
            || !self
                .world
                .upgrade()
                .is_some_and(|world| world.borrow().is_live())
        {
            return Err(Self::unavailable(vm));
        }
        Ok(())
    }
    /// Re-enter only the same physical table for already-owned cleanup. Every
    /// callback-bearing step returns here before the next table operation.
    pub(crate) fn in_original<T>(
        &self,
        vm: &mut Vm,
        work: impl FnOnce(&mut Vm) -> T,
    ) -> Result<T, CmdError> {
        let _ = self.check(vm);
        if !self.physical(vm) {
            return Err(Self::unavailable(vm));
        }
        let reached = self
            .target
            .retire_in_original(vm, CleanupRestoration::PreserveCallbackSelection, |vm| {
                let metadata = work(vm);
                if let Err(first) = self.check(vm) {
                    super::variable_cleanup::retain(vm, &first);
                }
                metadata
            })
            .map_err(CmdError::from_execution_refusal)?;
        if let Some(first) = reached.first_cause {
            super::variable_cleanup::retain(vm, &CmdError::from_execution_refusal(first));
        }
        Ok(reached.metadata)
    }
    fn token(&self, vm: &mut Vm, generation: u64) -> Option<CommandTokenIdentity> {
        self.in_original(vm, |vm| vm.command_token_at_generation(generation))
            .ok()
            .flatten()
    }
    fn begin_c(&self, vm: &mut Vm, generation: u64) -> bool {
        if self.c_order.is_none() {
            return true;
        }
        self.in_original(vm, |vm| {
            let first = {
                let mut world = vm.name_world.borrow_mut();
                world.command_deletion_active.insert(generation)
            };
            if self
                .c_names
                .is_some_and(|protocol| protocol.deletion_epoch_before_trace(!first))
            {
                vm.note_native_command_name_node_changed(generation);
            }
            first
        })
        .unwrap_or(false)
    }
    pub(super) fn unlink_recursive_c_delete(
        &self,
        vm: &mut Vm,
        source: &CommandTokenIdentity,
    ) -> bool {
        let deleting = self.c_order.is_some()
            && self
                .in_original(vm, |vm| {
                    vm.name_world
                        .borrow()
                        .command_deletion_active
                        .contains(&source.generation)
                })
                .unwrap_or(false);
        if deleting {
            let _ = self.begin_c(vm, source.generation);
            self.unlink(vm, source, true);
        }
        deleting
    }
    fn end_c(&self, generation: u64) {
        // Withdraw only this pure guard in the captured allocation, even when
        // operation currency was withdrawn. No current table is selected here.
        if self.c_order.is_some()
            && let Some(world) = self.world.upgrade()
        {
            let mut world = world.borrow_mut();
            world.command_deletion_active.remove(&generation);
            world
                .command_compiler_deletion_published
                .remove(&generation);
        }
    }
    fn unlink(&self, vm: &mut Vm, source: &CommandTokenIdentity, recursive: bool) {
        let Some(live) = self.token(vm, source.generation) else {
            // Modern C's recursive CMD_DYING branch advances cmdEpoch even
            // after a prior recursive request removed hPtr. The retained
            // original generation still owns this mandatory stage.
            if recursive
                && self
                    .c_names
                    .is_some_and(|protocol| protocol.deletion_epoch_after_unlink(true))
            {
                let _ = self.in_original(vm, |vm| {
                    vm.note_native_command_name_node_changed(source.generation);
                });
            }
            return;
        };
        let _ = self.in_original(vm, |vm| {
            let removed = vm.take_command_sidecar_for_retirement(&live.key, self.c_order.is_some());
            if self
                .c_names
                .is_some_and(|protocol| protocol.deletion_epoch_after_unlink(recursive))
            {
                vm.note_native_command_name_node_changed(live.generation);
            }
            drop(removed);
        });
        let _ = self.in_original(vm, |vm| vm.drop_alias_backref_key(&live.key));
    }
    fn imports(&self, vm: &mut Vm, source: &CommandTokenIdentity) {
        let source = self
            .token(vm, source.generation)
            .unwrap_or_else(|| source.clone());
        let nodes = self
            .in_original(vm, |vm| vm.direct_import_retirement_nodes(&source))
            .unwrap_or_default();
        for node in nodes {
            self.import(vm, &node);
        }
    }
    fn import(&self, vm: &mut Vm, node: &ImportRetirementNode) {
        let expected = self
            .in_original(vm, |vm| {
                if let CommandSidecarKey::Visible(name) = &node.token.key {
                    vm.materialise_retained_binding(name);
                }
                vm.command_token_identity(&node.token.key).as_ref() == Some(&node.token)
                    && vm.import_binding_at_sidecar_key(&node.token.key)
                        == Some(node.binding.clone())
            })
            .unwrap_or(false);
        if !expected {
            return;
        }
        if !self.begin_c(vm, node.token.generation) {
            self.unlink(vm, &node.token, true);
            return;
        }
        let _ = self.in_original(vm, |vm| vm.on_command_removed_for(&node.token.key));
        // Nested deletion may unlink the dying import while the outer owner
        // still retires its original downstream references.
        let selected = self
            .in_original(vm, |vm| {
                vm.import_retirement_node_by_generation(node.binding.generation)
            })
            .ok()
            .flatten()
            .unwrap_or_else(|| node.clone());
        self.imports(vm, &selected.token);
        if let Some(live) = self
            .in_original(vm, |vm| {
                vm.import_retirement_node_by_generation(node.binding.generation)
            })
            .ok()
            .flatten()
        {
            let still_same_import = self
                .in_original(vm, |vm| {
                    vm.command_token_identity(&live.token.key).as_ref() == Some(&live.token)
                        && vm.import_binding_at_sidecar_key(&live.token.key)
                            == Some(live.binding.clone())
                })
                .unwrap_or(false);
            if still_same_import {
                let _ = self.in_original(vm, |vm| {
                    vm.drop_relocated_command_sidecars(&live.token.key, live.token.generation);
                });
                self.unlink(vm, &live.token, false);
            }
        }
        self.end_c(node.token.generation);
        let _ = self.check(vm);
    }
    fn implementation(
        &self,
        vm: &mut Vm,
        source: &CommandTokenIdentity,
        command: &Command,
        replacing: bool,
    ) {
        if replacing || self.c_order.is_none() {
            let _ = self.in_original(vm, |vm| {
                if vm
                    .name_world
                    .borrow_mut()
                    .command_name_retiring
                    .insert(source.generation)
                {
                    vm.note_native_command_name_node_changed(source.generation);
                    if let CommandSidecarKey::Visible(key) = &source.key
                        && let Some(slot) = vm.command_slot(key)
                    {
                        vm.note_native_namespace_command_lookup(slot.namespace);
                    }
                }
            });
        }
        if let Command::Proc(procedure) = command {
            let _ = self.in_original(vm, |_| procedure.retire());
        }
        if matches!(command, Command::Alias { .. } | Command::CrossAlias { .. }) {
            let _ = self.in_original(vm, |vm| {
                vm.alias_registrations
                    .forget_deleted(|token| token.generation == source.generation);
            });
        }
        if let Command::Native(native) = command {
            let _ = self.in_original(vm, |vm| {
                let completion = native.retire(vm);
                if completion.code != crate::Code::Ok && vm.execution_refusal.is_none() {
                    let _ = vm.refuse_host_command(
                        "native command retirement cannot return a guest completion".into(),
                    );
                }
                drop(completion);
            });
        }
        // A deleteProc can move the same token. Select only its actual original
        // generation after that callback; never adopt a reborn spelling.
        let key = self
            .token(vm, source.generation)
            .map_or_else(|| source.key.clone(), |live| live.key);
        let _ = self.in_original(vm, |vm| {
            crate::cmd_coro::on_command_deleted_original(vm, &key, source.generation, self);
        });
        if let Command::Object(object) = command {
            let _ = self.in_original(vm, |vm| {
                if replacing {
                    crate::cmd_oo::oo_command_replaced(vm, *object);
                } else {
                    crate::cmd_oo::oo_command_deleted(vm, *object);
                }
            });
        }
        if let Command::Ensemble(token) = command {
            let _ = self.in_original(vm, |vm| {
                vm.untrack_namespace_ensemble(token);
                token.mark_deleted();
            });
        }
        if let Command::ChildInterp(child) = command {
            let _ = self.in_original(vm, |vm| {
                vm.children.retain(|_, candidate| candidate != child);
            });
            let _ = self.in_original(vm, |vm| vm.retire_interp(*child));
        }
    }
    fn real(&self, vm: &mut Vm, source: &CommandTokenIdentity, command: &Command, trace: bool) {
        if !self.begin_c(vm, source.generation) {
            // C's recursive CMD_IS_DELETED branch removes only hPtr. The outer
            // owner still calls the original deleteProc and retires imports.
            self.unlink(vm, source, true);
            return;
        }
        if trace {
            let _ = self.in_original(vm, |vm| vm.on_command_removed_for(&source.key));
        }
        let selected = self
            .token(vm, source.generation)
            .unwrap_or_else(|| source.clone());
        // Neutral/Jim retirement releases the implementation before imports.
        // C import authority requires its independently selected recipe.
        let order = self
            .c_order
            .unwrap_or(NativeCommandRetirementOrder::ImplementationBeforeImports);
        let reached = retire_original_command(order, |stage| {
            match stage {
                NativeCommandRetirementStage::Implementation => {
                    self.implementation(vm, &selected, command, false);
                }
                NativeCommandRetirementStage::Imports => self.imports(vm, &selected),
            }
            self.check(vm)
        });
        if let Err(first) = &reached {
            super::variable_cleanup::retain(vm, first);
        }
        if let Some(live) = self.token(vm, source.generation) {
            let _ = self.in_original(vm, |vm| {
                vm.drop_relocated_command_sidecars(&live.key, live.generation);
            });
            self.unlink(vm, &live, false);
        }
        self.end_c(source.generation);
        let _ = self.check(vm);
    }
}
impl Vm {
    pub(super) fn retire_registered_real_command(
        &mut self,
        source: &CommandTokenIdentity,
        command: &Command,
    ) {
        let original = OriginalCommandCleanup::enter(self);
        original.real(self, source, command, true);
    }
    pub(super) fn retire_real_command_after_trace(
        &mut self,
        source: &CommandTokenIdentity,
        command: &Command,
    ) {
        let original = OriginalCommandCleanup::enter(self);
        original.real(self, source, command, false);
    }
    pub(super) fn retire_replaced_real_command(
        &mut self,
        source: &CommandTokenIdentity,
        command: &Command,
    ) {
        let original = OriginalCommandCleanup::enter(self);
        original.implementation(self, source, command, true);
        let _ = original.check(self);
    }
    pub(super) fn retire_import_tree_snapshot(&mut self, node: &ImportRetirementNode) {
        let original = OriginalCommandCleanup::enter(self);
        original.import(self, node);
    }
    pub(super) fn retire_downstream_imports(&mut self, source: &CommandTokenIdentity) {
        let original = OriginalCommandCleanup::enter(self);
        original.imports(self, source);
        let _ = original.check(self);
    }
}

#[cfg(test)]
mod tests;
