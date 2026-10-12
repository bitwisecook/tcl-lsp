// SPDX-License-Identifier: AGPL-3.0-or-later
//! Retained package execution; grammar and table algorithms keep their existing owners.

use crate::interp::{
    PackageTableReceipt, Vm, native_numeric_ops::OriginalNumericOps,
    native_operation_currency::NativeOperationScope,
};
use crate::value::{NativeObjectLifetimeLease, Value};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use tcl_cmd_core::CmdError;
use tcl_core_types::NameBytes;
use tcl_runtime_api::Completion;

pub(super) struct OriginalPackage<'a> {
    vm: &'a mut Vm,
    operation: NativeOperationScope,
    table: PackageTableReceipt,
    operands: Vec<NativeObjectLifetimeLease>,
}

impl OriginalPackage<'_> {
    pub(super) fn run(
        vm: &mut Vm,
        authored: bool,
        operands: &[Value],
        callback: impl FnOnce(&mut OriginalPackage<'_>) -> Completion<Value>,
    ) -> Completion<Value> {
        let operation = match NativeOperationScope::enter(vm) {
            Ok(operation) => operation,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        let reached = vm.with_package_table_checked(authored, &operation, |vm, table| {
            let mut original = OriginalPackage {
                vm,
                operation: operation.clone(),
                table: table.clone(),
                operands: operands.iter().map(Value::native_lifetime_lease).collect(),
            };
            let completion = match original.check() {
                Ok(()) => callback(&mut original),
                Err(error) => crate::command::completion_from_cmd_error(original.vm, error),
            };
            let checked = original.check_value(&completion.result)
                .and_then(|()| original.check_value(&completion.options));
            let held = std::mem::take(&mut original.operands);
            drop(held);
            let checked = checked.and_then(|()| original.check());
            match checked {
                Ok(()) => completion,
                Err(error) => {
                    drop(completion);
                    crate::command::completion_from_cmd_error(original.vm, error)
                }
            }
        });
        match reached {
            Ok(completion) => completion,
            Err(error) => crate::command::completion_from_cmd_error(vm, error),
        }
    }

    pub(super) fn check(&mut self) -> Result<(), CmdError> {
        self.table.check(self.vm, &self.operation)
    }

    pub(super) fn check_value(&mut self, value: &Value) -> Result<(), CmdError> {
        self.check()?;
        OriginalNumericOps::in_original_scope(self.vm, &self.operation)?.current_for_value(value)?;
        self.check()
    }

    pub(super) fn string(&mut self, value: &Value) -> Result<Rc<[u8]>, CmdError> {
        self.check()?;
        let held = value.native_lifetime_lease();
        let reached = (|| {
            let ops = OriginalNumericOps::in_original_scope(self.vm, &self.operation)?;
            let protocol = ops.dialect().native_string_protocol().ok_or(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable("package String protocol"),
            )?;
            let reached = ops.string(held.value(), protocol);
            if let Err(error) = &reached {
                retain_at_owner(&ops, error);
            }
            ops.current_for_value(held.value())?;
            reached
        })();
        drop(held);
        self.check()?;
        reached
    }

    pub(super) fn list(&mut self, value: &Value) -> Result<crate::NativeListItems, CmdError> {
        self.check()?;
        let ops = OriginalNumericOps::in_original_scope(self.vm, &self.operation)?;
        let protocol = ops.dialect().native_string_protocol().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("package List protocol"),
        )?;
        let reached = ops.list(value, protocol);
        if let Err(error) = &reached {
            retain_at_owner(&ops, error);
        }
        ops.current_for_value(value)?;
        drop(ops);
        self.check()?;
        reached
    }

    pub(super) fn retain_failure(&mut self, error: &CmdError) {
        if let Some(first) = error.native_execution_refusal() {
            self.vm.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(first.clone()));
        } else if let Some(first) = error.native_access_refusal() {
            self.vm.retain_tcl_host_failure(crate::error::TclHostFailure::ValueAccess(first));
        }
    }

    pub(super) fn checked<T>(
        &mut self,
        callback: impl FnOnce(&mut Vm) -> Result<T, CmdError>,
    ) -> Result<T, CmdError> {
        self.check()?;
        let reached = callback(self.vm);
        if let Err(error) = &reached { self.retain_failure(error); }
        self.check()?;
        reached
    }

    pub(super) fn checked_tcl<T>(
        &mut self,
        callback: impl FnOnce(&mut Vm) -> Result<T, crate::TclError>,
    ) -> Result<T, crate::TclError> {
        self.check().map_err(original_tcl_failure)?;
        let reached = callback(self.vm);
        if let Err(crate::TclError::Host(first)) = &reached {
            self.vm.retain_tcl_host_failure(first.clone());
        }
        self.check().map_err(original_tcl_failure)?;
        reached
    }

    pub(super) fn cleanup<R>(&mut self, callback: impl FnOnce(&mut Vm) -> R) -> Result<R, CmdError> {
        self.table.cleanup(self.vm, &self.operation, callback)
    }

    pub(super) fn end_loading(&mut self, name: &NameBytes, version: &NameBytes) -> Result<(), CmdError> {
        self.table.end_loading(self.vm, &self.operation, name, version)
    }

    pub(super) fn eval_script(&mut self, script: &Value) -> Completion<Value> {
        if let Err(error) = self.check_value(script) {
            return crate::command::completion_from_cmd_error(self.vm, error);
        }
        // The existing global-frame worker completes its saved-target restoration.
        self.vm.eval_value_at_level_checked(0, script)
    }

    pub(super) fn settle_callback(&mut self, completion: Completion<Value>) -> Completion<Value> {
        match self.check() {
            Ok(()) => completion,
            Err(error) => {
                drop(completion);
                crate::command::completion_from_cmd_error(self.vm, error)
            }
        }
    }
}

impl Deref for OriginalPackage<'_> {
    type Target = Vm;
    fn deref(&self) -> &Vm { self.vm }
}
impl DerefMut for OriginalPackage<'_> {
    fn deref_mut(&mut self) -> &mut Vm { self.vm }
}

fn retain_at_owner(ops: &OriginalNumericOps<'_>, error: &CmdError) {
    ops.retain_failure(error);
    if let Some(first) = error.native_access_refusal() {
        ops.retain_failure(&CmdError::from_execution_refusal(first.into()));
    }
}

fn original_tcl_failure(error: CmdError) -> crate::TclError {
    crate::TclError::from_execution_failure(
        error.native_execution_refusal().expect("original package currency refusal").clone(),
    )
}

#[cfg(test)]
mod tests;
