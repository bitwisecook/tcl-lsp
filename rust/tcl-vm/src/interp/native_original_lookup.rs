// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original lookup observations remain independent of String materialisation.

use super::{
    CommandSidecarKey, CommandSlot, NsId, Rc, Vm, native_name_queries::QueryNamespace,
    native_name_world::NativeNameWorld, native_numeric_ops::OriginalNumericOps,
    native_operation_currency::NativeOperationScope,
};
use crate::{Value, value::NativeObjectLifetimeLease};
use std::cell::RefCell;
use tcl_cmd_core::CmdError;

pub(super) struct OriginalLookup {
    operation: NativeOperationScope,
    world: Rc<RefCell<NativeNameWorld>>,
    namespace: QueryNamespace,
    namespace_id: NsId,
    jim_namespace: Option<NativeObjectLifetimeLease>,
    original: Option<NativeObjectLifetimeLease>,
}

pub(super) struct OriginalLookupCommand {
    namespace: QueryNamespace,
    slot: CommandSlot,
    generation: u64,
    epoch: u64,
}

impl OriginalLookup {
    pub(super) fn enter(vm: &mut Vm, namespace: NsId, original: &Value) -> Result<Self, CmdError> {
        let operation = NativeOperationScope::enter(vm)?;
        let namespace_id = namespace;
        let namespace = QueryNamespace::capture(vm, namespace)?;
        let jim_namespace = if vm.uses_native_jim_lookup() {
            Some(vm.with_jim_namespace_at(namespace_id, |value| Ok(value.native_lifetime_lease()))?)
        } else {
            None
        };
        let entered = Self {
            operation,
            world: Rc::clone(&vm.name_world),
            namespace,
            namespace_id,
            jim_namespace,
            original: Some(original.native_lifetime_lease()),
        };
        entered.check(vm)?;
        Ok(entered)
    }

    pub(super) fn check(&self, vm: &mut Vm) -> Result<(), CmdError> {
        self.operation.check(vm)?;
        let same = Rc::ptr_eq(&self.world, &vm.name_world)
            && self.world.borrow().is_live()
            && self.namespace.current(vm);
        self.require(vm, same, "original lookup namespace and world")?;
        if let Some(namespace) = &self.jim_namespace {
            OriginalNumericOps::in_original_scope(vm, &self.operation)?
                .current_for_value(namespace.value())?;
            let same = vm.with_jim_namespace_at(self.namespace_id, |current| {
                Ok(current.is_same_object(namespace.value()))
            })?;
            self.require(vm, same, "original Jim lookup namespace header")?;
        }
        if let Some(original) = &self.original {
            OriginalNumericOps::in_original_scope(vm, &self.operation)?
                .current_for_value(original.value())?;
        }
        Ok(())
    }

    pub(super) fn require(
        &self,
        vm: &mut Vm,
        current: bool,
        purpose: &'static str,
    ) -> Result<(), CmdError> {
        self.operation.check(vm)?;
        if !current {
            return Err(vm.retain_name_query_refusal(purpose));
        }
        Ok(())
    }

    pub(super) fn read(&self, vm: &mut Vm) -> Result<Rc<[u8]>, CmdError> {
        self.check(vm)?;
        let reached = vm.native_name_operand_bytes_checked(
            self.original
                .as_ref()
                .expect("entered original operand")
                .value(),
        );
        self.check(vm)?;
        reached
    }

    pub(super) fn jim_namespace_original(&self) -> Option<&Value> {
        self.jim_namespace.as_ref().map(|holder| holder.value())
    }

    pub(super) fn read_jim_namespace(&self, vm: &mut Vm) -> Result<Rc<[u8]>, CmdError> {
        self.check(vm)?;
        let holder = self
            .jim_namespace
            .as_ref()
            .ok_or_else(|| vm.retain_name_query_refusal("original Jim namespace query holder"))?;
        let reached = vm.native_name_operand_bytes_checked(holder.value());
        self.check(vm)?;
        reached
    }

    pub(super) fn command(
        &self,
        vm: &mut Vm,
        key: &str,
    ) -> Result<OriginalLookupCommand, CmdError> {
        self.check(vm)?;
        let selected = vm.command_slot(key).zip(vm.visible_command_generation(key));
        self.require(vm, selected.is_some(), "original lookup selected command")?;
        let (slot, generation) = selected.expect("checked selected command");
        let epoch = vm
            .name_world
            .borrow()
            .command_name_epochs
            .get(&generation)
            .copied()
            .unwrap_or(0);
        Ok(OriginalLookupCommand {
            namespace: QueryNamespace::capture(vm, slot.namespace)?,
            slot,
            generation,
            epoch,
        })
    }

    pub(super) fn finish<T>(
        mut self,
        vm: &mut Vm,
        reached: Result<T, CmdError>,
    ) -> Result<T, CmdError> {
        if let Err(error) = &reached
            && let Some(cause) = error.native_execution_refusal()
        {
            vm.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(cause.clone()));
        }
        let before = self.check(vm);
        drop(self.original.take());
        drop(self.jim_namespace.take());
        self.check(vm)?;
        before?;
        reached
    }
}

impl OriginalLookupCommand {
    pub(super) fn generation(&self) -> u64 {
        self.generation
    }

    pub(super) fn check(&self, vm: &mut Vm, lookup: &OriginalLookup) -> Result<(), CmdError> {
        lookup.check(vm)?;
        let same = self.namespace.current(vm)
            && vm
                .command_storage_key_at_slot(&self.slot)
                .is_some_and(|key| {
                    vm.visible_command_generation(&key) == Some(self.generation)
                && vm.command_token_at_generation(self.generation).is_some_and(|token| {
                    matches!(token.key, CommandSidecarKey::Visible(ref actual) if actual == &key)
                })
                })
            && vm
                .name_world
                .borrow()
                .command_name_epochs
                .get(&self.generation)
                .copied()
                .unwrap_or(0)
                == self.epoch;
        lookup.require(vm, same, "original lookup selected command binding")
    }
}

#[cfg(test)]
mod tests;
