// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original consumer context, independent of selected command and table roles.

use super::{
    Vm,
    native_name_queries::QueryNamespace,
    native_name_world::NativeNameWorld,
    native_operation_currency::{NativeOperationCurrency, NativeOperationScope},
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
    state: OriginalNameConsumerState,
    operation: NativeOperationScope,
}

struct OriginalNameConsumerState {
    jim_namespace: Option<NativeObjectLifetimeLease>,
    frame: Option<(Rc<ActivationIdentity>, usize)>,
    namespace: NsId,
    namespace_owner: QueryNamespace,
    selected_namespace: Option<QueryNamespace>,
    world: Rc<RefCell<NativeNameWorld>>,
}

/// Saved context is descriptive until its same original currency is resumed.
pub(crate) struct ParkedOriginalNameConsumerScope {
    state: OriginalNameConsumerState,
    currency: NativeOperationCurrency,
}

impl OriginalNameConsumerScope {
    pub(crate) fn enter(vm: &mut Vm) -> Result<Self, CmdError> {
        Self::enter_for_namespace(vm, None)
    }

    pub(crate) fn enter_for_namespace(
        vm: &mut Vm,
        selected: Option<NsId>,
    ) -> Result<Self, CmdError> {
        let operation = NativeOperationScope::enter(vm)?;
        let frame = vm
            .frames
            .len()
            .checked_sub(1)
            .map(|level| (vm.retained_frame_identity(false), level));
        let namespace = vm.current_ns_id();
        let namespace_owner = QueryNamespace::capture(vm, namespace)?;
        let selected_namespace = selected
            .map(|id| QueryNamespace::capture(vm, id))
            .transpose()?;
        let world = Rc::clone(&vm.name_world);
        let jim_namespace = if vm.uses_native_jim_lookup() && frame.is_some() {
            Some(vm.with_jim_current_namespace(|original| Ok(original.native_lifetime_lease()))?)
        } else {
            None
        };
        let scope = Self {
            state: OriginalNameConsumerState {
                jim_namespace,
                frame,
                namespace,
                namespace_owner,
                selected_namespace,
                world,
            },
            operation,
        };
        scope.check(vm)?;
        Ok(scope)
    }

    pub(crate) fn check(&self, vm: &mut Vm) -> Result<(), CmdError> {
        self.operation.check(vm)?;
        let world_current = Rc::ptr_eq(&self.state.world, &vm.name_world)
            && self.state.world.borrow().is_live()
            && self.state.namespace_owner.current(vm)
            && self
                .state
                .selected_namespace
                .as_ref()
                .is_none_or(|selected| selected.current(vm));
        self.require(
            vm,
            world_current,
            "original Name consumer namespace and world",
        )?;
        let frame_current = self.state.frame.as_ref().is_none_or(|(identity, level)| {
            vm.retained_frame_level(identity) == Some(*level)
                && vm.frames.len().checked_sub(1) == Some(*level)
        });
        self.require(vm, frame_current, "original Name consumer frame activation")?;
        let namespace_current = vm.current_ns_id() == self.state.namespace
            && vm.native_namespace_token_has_owner(self.state.namespace);
        self.require(
            vm,
            namespace_current,
            "original Name consumer namespace incarnation",
        )?;
        if let Some(original) = &self.state.jim_namespace {
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

    pub(crate) fn capture_binding(
        &self,
        vm: &mut Vm,
        binding: &super::VarBinding,
    ) -> Result<OriginalNameBinding, CmdError> {
        self.check(vm)?;
        let raw = vm.raw_variable_binding(binding);
        let cell = raw.and_then(|id| vm.var_arena.weak_jim_cell(id));
        self.require(vm, cell.is_some(), "original Name selected binding cell")?;
        Ok(OriginalNameBinding {
            binding: binding.clone(),
            raw: raw.expect("checked original binding"),
            cell: cell.expect("checked original cell"),
        })
    }

    pub(crate) fn capture_cell(
        &self,
        vm: &mut Vm,
        id: super::VarId,
    ) -> Result<crate::vars::WeakJimVariableCell, CmdError> {
        self.check(vm)?;
        let cell = vm.var_arena.weak_jim_cell(id);
        self.require(vm, cell.is_some(), "original Name selected cell birth")?;
        Ok(cell.expect("checked original cell"))
    }

    pub(crate) fn check_cell(
        &self,
        vm: &mut Vm,
        id: super::VarId,
        cell: &crate::vars::WeakJimVariableCell,
    ) -> Result<(), CmdError> {
        self.check(vm)?;
        let current = vm.var_arena.validate_jim_cell(cell) == Some(id);
        self.require(vm, current, "original Name selected cell birth")
    }

    pub(crate) fn capture_frame(
        &self,
        vm: &mut Vm,
        level: usize,
    ) -> Result<OriginalNameFrame, CmdError> {
        self.check(vm)?;
        Ok(OriginalNameFrame {
            level,
            identity: vm
                .frames
                .get(level)
                .map(|frame| Rc::clone(&frame.activation)),
        })
    }

    pub(crate) fn operation_scope(&self) -> &NativeOperationScope {
        &self.operation
    }

    pub(super) fn currency(&self) -> &super::native_operation_currency::NativeOperationCurrency {
        self.operation.currency()
    }

    pub(crate) fn park(self) -> ParkedOriginalNameConsumerScope {
        let Self { state, operation } = self;
        let currency = operation.currency().clone();
        drop(operation);
        ParkedOriginalNameConsumerScope { state, currency }
    }

    pub(crate) fn current_for_value(&self, vm: &mut Vm, original: &Value) -> Result<(), CmdError> {
        self.check(vm)?;
        super::native_numeric_ops::OriginalNumericOps::in_original_scope(vm, &self.operation)?
            .current_for_value(original)
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
        drop(self.state.jim_namespace.take());
        self.check(vm)?;
        before
    }
}

impl ParkedOriginalNameConsumerScope {
    pub(crate) fn currency(&self) -> &NativeOperationCurrency {
        &self.currency
    }

    pub(crate) fn resume(self, vm: &mut Vm) -> Result<OriginalNameConsumerScope, CmdError> {
        let Self { state, currency } = self;
        let operation = match NativeOperationScope::resume_saved(vm, &currency) {
            Ok(operation) => operation,
            Err(first) => {
                if let Ok(cleanup) = NativeOperationScope::cleanup_saved(vm, &currency) {
                    drop(state);
                    let _ = cleanup.check(vm);
                } else {
                    drop(state);
                }
                return Err(CmdError::from_execution_refusal(first));
            }
        };
        let original = OriginalNameConsumerScope { state, operation };
        if let Err(first) = original.check(vm) {
            let _ = original.finish(vm);
            return Err(first);
        }
        Ok(original)
    }
}

/// A weak observation of the actual selected slot, never a variable role grant.
pub(crate) struct OriginalNameBinding {
    binding: super::VarBinding,
    raw: super::VarId,
    cell: crate::vars::WeakJimVariableCell,
}
impl OriginalNameBinding {
    pub(crate) fn check(
        &self,
        vm: &mut Vm,
        scope: &OriginalNameConsumerScope,
    ) -> Result<(), CmdError> {
        scope.check_cell(vm, self.raw, &self.cell)?;
        let current = vm.raw_variable_binding(&self.binding) == Some(self.raw);
        scope.require(vm, current, "original Name selected binding slot")
    }
}

/// The already selected target frame; a missing frame stays missing.
pub(crate) struct OriginalNameFrame {
    level: usize,
    identity: Option<Rc<ActivationIdentity>>,
}
impl OriginalNameFrame {
    pub(crate) fn check(
        &self,
        vm: &mut Vm,
        scope: &OriginalNameConsumerScope,
    ) -> Result<(), CmdError> {
        scope.check(vm)?;
        let current = match (&self.identity, vm.frames.get(self.level)) {
            (Some(saved), Some(actual)) => Rc::ptr_eq(saved, &actual.activation),
            (None, None) => true,
            _ => false,
        };
        scope.require(vm, current, "original Name selected target activation")
    }
}

#[cfg(test)]
mod tests;
