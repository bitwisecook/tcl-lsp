// SPDX-License-Identifier: AGPL-3.0-or-later
//! Mandatory OO payload retirement in the actual original physical interpreter.
use super::native_operation_currency::{
    CleanupRestoration, NativeOperationCurrency, NativeOperationScope, SavedExecutionTarget,
};
use super::{CommandSidecarKey, InterpId, InterpState, Vm};
use crate::cmd_oo::OoState;
use crate::command::Command;
use tcl_cmd_core::CmdError;
use tcl_runtime_api::OoId;
use tcl_syntax::value::ValueError;

mod active_frame;
pub(crate) use active_frame::{OriginalOoActivation, OriginalOoCallExit};

pub(crate) fn retain_error(vm: &mut Vm, error: &CmdError) {
    if let Some(first) = error.native_execution_refusal() {
        vm.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(first.clone()));
    } else if let Some(first) = error.native_access_refusal() {
        vm.retain_tcl_host_failure(crate::error::TclHostFailure::ValueAccess(first));
    }
}

/// Physical storage and original currency are independent of a live OO target.
/// Entry refusal never skips retirement of already-owned original payloads.
pub(crate) struct OriginalOoCleanup {
    target: SavedExecutionTarget,
    interpreter: InterpId,
    operation: Option<NativeOperationScope>,
    entry_failure: Option<CmdError>,
    namespace_traversal: Option<tcl_syntax::native_namespace_name::NativeNamespaceCommandTeardown>,
    pub(crate) method_cache_selected: bool,
    pub(crate) property_recipe:
        Option<tcl_registry::native_property_lookup::NativePropertyLookupProtocol>,
}
/// Readonly original cleanup currency on an authentic OO procedure continuation.
/// It creates no entered invocation and owns no frame, Method or Native payload.
#[derive(Clone)]
struct SavedOriginalOoCleanup {
    target: SavedExecutionTarget,
    interpreter: InterpId,
    currency: Option<NativeOperationCurrency>,
    entry_failure: Option<CmdError>,
    namespace_traversal: Option<tcl_syntax::native_namespace_name::NativeNamespaceCommandTeardown>,
    method_cache_selected: bool,
    property_recipe: Option<tcl_registry::native_property_lookup::NativePropertyLookupProtocol>,
}
impl SavedOriginalOoCleanup {
    fn attach(&self, vm: &mut Vm) -> OriginalOoCleanup {
        // Check before selecting the saved physical target. Returning to that
        // target cannot erase a callback's foreign-interpreter first cause.
        if let Some(currency) = &self.currency
            && let Err(first) = currency.check(vm)
        {
            retain_error(vm, &first);
        }
        let (operation, entry_failure) = match &self.currency {
            Some(currency) => {
                match NativeOperationScope::enter_saved_cleanup(vm, currency, self.interpreter) {
                    Ok(operation) => (Some(operation), self.entry_failure.clone()),
                    Err(first) => {
                        let first = CmdError::from_execution_refusal(first);
                        retain_error(vm, &first);
                        (None, Some(first))
                    }
                }
            }
            None => (None, self.entry_failure.clone()),
        };
        OriginalOoCleanup {
            target: self.target.clone(),
            interpreter: self.interpreter,
            operation,
            entry_failure,
            namespace_traversal: self.namespace_traversal,
            method_cache_selected: self.method_cache_selected,
            property_recipe: self.property_recipe,
        }
    }
}
impl OriginalOoCleanup {
    pub(crate) fn enter(vm: &mut Vm) -> Self {
        let namespace_traversal = vm
            .actual_native_invocation_dialect()
            .native_namespace_name_protocol()
            .map(|protocol| protocol.recipe().command_teardown());
        let method_cache_selected = vm
            .actual_native_invocation_dialect()
            .native_tcloo_method_cache_protocol()
            .is_some();
        let property_recipe = vm
            .actual_native_invocation_dialect()
            .native_property_lookup_protocol();
        let (operation, entry_failure) = match NativeOperationScope::enter(vm) {
            Ok(operation) => (Some(operation), None),
            Err(first) => {
                retain_error(vm, &first);
                (None, Some(first))
            }
        };
        let target = SavedExecutionTarget::capture(vm);
        let interpreter = target.interpreter();
        Self {
            target,
            interpreter,
            operation,
            entry_failure,
            namespace_traversal,
            method_cache_selected,
            property_recipe,
        }
    }
    /// Readonly physical cleanup observations; never issue a new execution owner.
    fn observe(vm: &mut Vm) -> Self {
        let namespace_traversal = vm
            .actual_native_invocation_dialect()
            .native_namespace_name_protocol()
            .map(|protocol| protocol.recipe().command_teardown());
        let method_cache_selected = vm
            .actual_native_invocation_dialect()
            .native_tcloo_method_cache_protocol()
            .is_some();
        let property_recipe = vm
            .actual_native_invocation_dialect()
            .native_property_lookup_protocol();
        let (operation, entry_failure) = match NativeOperationScope::current_entered(vm) {
            Ok(operation) => (operation, None),
            Err(first) => {
                let first = CmdError::from_execution_refusal(first);
                retain_error(vm, &first);
                (None, Some(first))
            }
        };
        let target = SavedExecutionTarget::capture(vm);
        let interpreter = target.interpreter();
        Self {
            target,
            interpreter,
            operation,
            entry_failure,
            namespace_traversal,
            method_cache_selected,
            property_recipe,
        }
    }

    fn saved(&self) -> SavedOriginalOoCleanup {
        SavedOriginalOoCleanup {
            target: self.target.clone(),
            interpreter: self.interpreter,
            currency: self
                .operation
                .as_ref()
                .map(|operation| operation.currency().clone()),
            entry_failure: self.entry_failure.clone(),
            namespace_traversal: self.namespace_traversal,
            method_cache_selected: self.method_cache_selected,
            property_recipe: self.property_recipe,
        }
    }
    fn physical(&self, vm: &Vm) -> bool {
        self.target.owns_target(vm)
    }
    fn missing(vm: &mut Vm) -> CmdError {
        let error = CmdError::from(ValueError::CommandProtocolUnavailable(
            "original OO cleanup interpreter",
        ));
        retain_error(vm, &error);
        match vm.execution_refusal.clone() {
            Some(first) => CmdError::from_execution_refusal(first),
            None => error,
        }
    }
    fn state_mut<'a>(&self, vm: &'a mut Vm) -> Result<&'a mut InterpState, CmdError> {
        if !self.physical(vm) {
            return Err(Self::missing(vm));
        }
        if vm.cur == self.interpreter {
            return Ok(&mut vm.state);
        }
        Ok(vm.interps[self.interpreter.0]
            .parked
            .as_deref_mut()
            .expect("checked original state"))
    }
    pub(crate) fn in_original<T>(
        &self,
        vm: &mut Vm,
        work: impl FnOnce(&mut Vm) -> T,
    ) -> Result<T, CmdError> {
        if let Err(first) = self.check(vm) {
            retain_error(vm, &first);
        }
        if !self.physical(vm) {
            return Err(Self::missing(vm));
        }
        let reached = self
            .target
            .retire_in_original(vm, CleanupRestoration::RestoreCaller, |vm| {
                let metadata = work(vm);
                if let Err(first) = self.check(vm) {
                    retain_error(vm, &first);
                }
                metadata
            })
            .map_err(CmdError::from_execution_refusal)?;
        if let Some(first) = reached.first_cause {
            retain_error(vm, &CmdError::from_execution_refusal(first));
        }
        Ok(reached.metadata)
    }
    pub(crate) fn oo_mut<'a>(&self, vm: &'a mut Vm) -> Result<&'a mut OoState, CmdError> {
        Ok(&mut self.state_mut(vm)?.oo)
    }
    fn refuse(&self, vm: &mut Vm, purpose: &'static str) -> CmdError {
        let error: CmdError = ValueError::CommandProtocolUnavailable(purpose).into();
        retain_error(vm, &error);
        self.check_cleanup(vm).err().unwrap_or(error)
    }
    pub(crate) fn finish_completion(
        &self,
        vm: &mut Vm,
        result: tcl_runtime_api::Completion<crate::value::Value>,
    ) -> tcl_runtime_api::Completion<crate::value::Value> {
        if let Err(first) = self.check(vm) {
            retain_error(vm, &first);
            drop(result);
            let first = self.check(vm).err().unwrap_or(first);
            return crate::command::completion_from_cmd_error(vm, first);
        }
        result
    }
    pub(crate) fn check(&self, vm: &mut Vm) -> Result<(), CmdError> {
        if let Some(first) = vm.execution_refusal.clone() {
            return Err(CmdError::from_execution_refusal(first));
        }
        if let Some(first) = &self.entry_failure {
            return Err(first.clone());
        }
        match &self.operation {
            Some(operation) => operation.check(vm),
            None if self.physical(vm) && vm.cur == self.interpreter => Ok(()),
            None => Err(Self::missing(vm)),
        }
    }
    pub(crate) fn check_cleanup(&self, vm: &mut Vm) -> Result<(), CmdError> {
        if let Some(first) = &self.entry_failure {
            return Err(first.clone());
        }
        if !self.physical(vm) {
            return Err(Self::missing(vm));
        }
        if let Some(first) = vm.execution_refusal.clone() {
            return Err(CmdError::from_execution_refusal(first));
        }
        match &self.operation {
            Some(operation) => operation
                .currency()
                .check_saved_target(vm, self.interpreter)
                .map_err(CmdError::from_execution_refusal),
            None => Ok(()),
        }
    }
    /// The already retained object namespace is a mandatory cleanup holder.
    pub(crate) fn retire_namespace(
        &self,
        vm: &mut Vm,
        namespace: tcl_runtime_api::NsId,
    ) -> Result<(), CmdError> {
        if !self.physical(vm) {
            return Err(Self::missing(vm));
        }
        if namespace == super::ROOT_NS {
            let error: CmdError =
                ValueError::CommandProtocolUnavailable("original OO private namespace").into();
            retain_error(vm, &error);
            return Err(error);
        }
        let retire = |vm: &mut Vm| {
            if vm.native_namespace_token_has_owner(namespace) {
                vm.delete_namespace_token_using(namespace, false, self.namespace_traversal);
            }
        };
        self.in_original(vm, retire)?;
        self.check_cleanup(vm)
    }
    /// Delete only the existing original object's command, never a replacement.
    pub(crate) fn retire_command(
        &self,
        vm: &mut Vm,
        object: OoId,
        key: &CommandSidecarKey,
    ) -> Result<(), CmdError> {
        if !self.physical(vm) {
            return Err(Self::missing(vm));
        }
        let retire = |vm: &mut Vm| {
            if matches!(vm.command_at_sidecar_key(key), Some(Command::Object(id)) if id == object) {
                let retired = vm.retire_command_lifecycle_key(key);
                drop(retired);
            }
        };
        self.in_original(vm, retire)?;
        self.check_cleanup(vm)
    }
}

/// Test-only transport through the real existing physical interpreter switch.
#[cfg(test)]
pub(crate) fn select_cleanup_fixture_interpreter(vm: &mut Vm, target: InterpId) {
    vm.switch_to(target);
}

/// Test-only withdrawal through the actual original name world's retire worker.
#[cfg(test)]
pub(crate) fn withdraw_cleanup_fixture_name_world(vm: &mut Vm) {
    vm.name_world.borrow_mut().retire();
}

#[cfg(test)]
mod original_target_tests;
