// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original consumer context, independent of selected command and table roles.

use super::{
    Vm, native_name_queries::QueryNamespace, native_name_world::NativeNameWorld,
    native_operation_currency::NativeOperationScope,
};
use crate::{
    Value, error::TclHostFailure, frame::ActivationIdentity, value::NativeObjectLifetimeLease,
};
use std::cell::RefCell;
use std::rc::Rc;
use tcl_cmd_core::CmdError;
use tcl_core_types::NsId;
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::raw_string::NativeValueAccessRefusal;

/// Keep the entered currency and the existing caller context through cleanup.
/// A Jim header lease protects allocation only; it issues no table identity.
pub(crate) struct OriginalNameConsumerScope {
    jim_namespace: Option<NativeObjectLifetimeLease>,
    frame: Option<(Rc<ActivationIdentity>, usize)>,
    namespace: NsId,
    namespace_owner: QueryNamespace,
    world: Rc<RefCell<NativeNameWorld>>,
    operation: NativeOperationScope,
}

impl OriginalNameConsumerScope {
    pub(crate) fn enter(vm: &mut Vm) -> Result<Self, CmdError> {
        let operation = NativeOperationScope::enter(vm)?;
        let frame = vm
            .frames
            .len()
            .checked_sub(1)
            .map(|level| (vm.retained_frame_identity(false), level));
        let namespace = vm.current_ns_id();
        let namespace_owner = QueryNamespace::capture(vm, namespace)?;
        let world = Rc::clone(&vm.name_world);
        let jim_namespace = if vm.uses_native_jim_lookup() && frame.is_some() {
            Some(vm.with_jim_current_namespace(|original| Ok(original.native_lifetime_lease()))?)
        } else {
            None
        };
        let scope = Self {
            jim_namespace,
            frame,
            namespace,
            namespace_owner,
            world,
            operation,
        };
        scope.check(vm)?;
        Ok(scope)
    }

    pub(crate) fn check(&self, vm: &mut Vm) -> Result<(), CmdError> {
        self.operation.check(vm)?;
        let world_current = Rc::ptr_eq(&self.world, &vm.name_world)
            && self.world.borrow().is_live()
            && self.namespace_owner.current(vm);
        self.require(
            vm,
            world_current,
            "original Name consumer namespace and world",
        )?;
        let frame_current = self.frame.as_ref().is_none_or(|(identity, level)| {
            vm.retained_frame_level(identity) == Some(*level)
                && vm.frames.len().checked_sub(1) == Some(*level)
        });
        self.require(vm, frame_current, "original Name consumer frame activation")?;
        let namespace_current = vm.current_ns_id() == self.namespace
            && vm.native_namespace_token_has_owner(self.namespace);
        self.require(
            vm,
            namespace_current,
            "original Name consumer namespace incarnation",
        )?;
        if let Some(original) = &self.jim_namespace {
            let same = vm.with_jim_current_namespace(|current| {
                Ok(current.is_same_object(original.value()))
            })?;
            self.require(vm, same, "original Jim consumer namespace header")?;
        }
        Ok(())
    }

    pub(crate) fn require(
        &self,
        vm: &mut Vm,
        current: bool,
        purpose: &'static str,
    ) -> Result<(), CmdError> {
        self.operation.check(vm)?;
        if current {
            return Ok(());
        }
        let cause = NativeExecutionError::ValueAccessRefusal(
            NativeValueAccessRefusal::CommandProtocolUnavailable(purpose),
        );
        vm.retain_tcl_host_failure(TclHostFailure::Execution(cause));
        self.operation.check(vm)
    }

    pub(super) fn currency(&self) -> &super::native_operation_currency::NativeOperationCurrency {
        self.operation.currency()
    }

    pub(crate) fn read(&self, vm: &mut Vm, original: &Value) -> Result<Rc<[u8]>, CmdError> {
        self.check(vm)?;
        let reached = vm.native_name_operand_bytes_checked(original);
        self.check(vm)?;
        reached
    }

    /// Retire the allocation lease while its original operation is entered.
    pub(crate) fn finish(mut self, vm: &mut Vm) -> Result<(), CmdError> {
        let before = self.check(vm);
        drop(self.jim_namespace.take());
        self.check(vm)?;
        before
    }
}

#[cfg(test)]
mod tests;
