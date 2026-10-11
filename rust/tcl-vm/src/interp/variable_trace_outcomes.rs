// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original callback outcomes and physical saved-state settlement.

use super::native_operation_currency::{
    NativeOperationContext, NativeOperationCurrency, NativeOperationScope,
};
use super::{InterpId, InterpState, NativeTraceResultState, Value, Vm};
use std::rc::Rc;
use tcl_runtime_api::{Completion, NativeExecutionError};

pub(super) type VariableTraceOutcome = Result<Result<(), Completion<Value>>, NativeExecutionError>;

pub(super) struct TracePhysicalOwner {
    owner: u64,
    interpreter: InterpId,
    context: Rc<NativeOperationContext>,
    currency: NativeOperationCurrency,
}

impl TracePhysicalOwner {
    pub(super) fn capture(vm: &Vm, currency: NativeOperationCurrency) -> Self {
        Self {
            owner: vm.owner_nonce,
            interpreter: vm.cur,
            context: Rc::clone(&vm.native_operation_context),
            currency,
        }
    }
}

impl Vm {
    fn original_trace_state_mut(
        &mut self,
        owner: &TracePhysicalOwner,
    ) -> Result<&mut InterpState, NativeExecutionError> {
        let current = self.owner_nonce == owner.owner
            && self
                .st_of(owner.interpreter)
                .is_some_and(|state| Rc::ptr_eq(&owner.context, &state.native_operation_context));
        if !current {
            return Err(self.retain_host_command_refusal(
                "original saved trace interpreter is unavailable".into(),
            ));
        }
        if owner.interpreter == self.cur {
            return Ok(&mut self.state);
        }
        match self
            .interps
            .get_mut(owner.interpreter.0)
            .and_then(|slot| slot.parked.as_deref_mut())
        {
            Some(original) => Ok(original),
            None => Err(self
                .execution_refusal
                .get_or_insert(NativeExecutionError::ValueAccessRefusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "original saved trace interpreter",
                    ),
                ))
                .clone()),
        }
    }

    fn variable_trace_guest_or_unwind(
        &mut self,
        fire: impl FnOnce(&mut Self) -> VariableTraceOutcome,
    ) -> Result<(), Completion<Value>> {
        let outcome = fire(self);
        match outcome {
            Ok(guest) => guest,
            Err(first) => {
                Err(self.refuse_tcl_host_failure(crate::error::TclHostFailure::Execution(first)))
            }
        }
    }

    fn variable_trace_command_error(
        &mut self,
        error: tcl_cmd_core::CmdError,
    ) -> Result<Completion<Value>, NativeExecutionError> {
        if let Some(first) = error.native_execution_refusal() {
            return Err(self
                .retain_tcl_host_failure(crate::error::TclHostFailure::Execution(first.clone())));
        }
        if let Some(first) = error.native_access_refusal() {
            return Err(
                self.retain_tcl_host_failure(crate::error::TclHostFailure::ValueAccess(first))
            );
        }
        Ok(crate::command::completion_from_cmd_error(self, error))
    }

    fn retain_variable_trace_value_error(
        &mut self,
        error: tcl_syntax::value::ValueError,
    ) -> NativeExecutionError {
        match error.native_access_refusal() {
            Some(first) => {
                self.retain_tcl_host_failure(crate::error::TclHostFailure::ValueAccess(first))
            }
            None => self.retain_host_command_refusal(error.to_string()),
        }
    }

    fn restore_original_trace_state(
        &mut self,
        state: NativeTraceResultState,
    ) -> Result<(), NativeExecutionError> {
        let NativeTraceResultState {
            result,
            interpreter,
            recipe,
            physical_owner,
        } = state;
        // Reject a foreign physical target before installing any saved field.
        self.original_trace_state_mut(&physical_owner)?;
        let cleanup = NativeOperationScope::enter_saved_cleanup(
            self,
            &physical_owner.currency,
            physical_owner.interpreter,
        )?;
        if recipe == tcl_registry::native_variable_trace::NativeTraceStateRecipe::MoveResult {
            let reuse = self
                .original_trace_state_mut(&physical_owner)?
                .native_c_result
                .borrow()
                .as_ref()
                .filter(|value| !value.native_object_is_shared())
                .map(Value::native_lifetime_lease);
            if let Some(original) = reuse {
                original.value().reset_native_c_result();
                drop(original);
            } else {
                let retired = self
                    .original_trace_state_mut(&physical_owner)?
                    .native_c_result
                    .replace(Some(Value::new_native_string_bytes(b"".as_slice())));
                drop(retired);
            }
            // This is the captured C84 reset recipe, not the callback's current profile.
            let retired = {
                let original = self.original_trace_state_mut(&physical_owner)?;
                original.native_errors.error_info = None;
                original.native_errors.native_error_info_len = 0;
                original.native_errors.native_error_result = None;
                original.native_errors.native_error_legacy_copy = false;
                original.native_errors.c84_global_code_set = false;
                original.native_errors.error_logged = false;
                original.native_errors.primitive_error_code.take()
            };
            drop(retired);
        }
        let (retired_result, retired_error, retired_stack) = {
            let original = self.original_trace_state_mut(&physical_owner)?;
            let (retired_error, retired_stack) = if let Some(saved) = interpreter {
                original.native_errors.native_c_return_state = saved.return_state;
                let retired_error = original.replace_native_error_trace_state(saved.error);
                let retired_stack = saved
                    .stack
                    .map(|stack| std::mem::replace(&mut original.native_errors.error_stack, stack));
                (Some(retired_error), retired_stack)
            } else {
                (None, None)
            };
            let retired_result = original.native_c_result.replace(Some(result));
            (retired_result, retired_error, retired_stack)
        };
        drop(retired_error);
        drop(retired_stack);
        drop(retired_result);
        cleanup
            .currency()
            .check_saved_target(self, physical_owner.interpreter)
    }
}

#[cfg(test)]
#[path = "variable_trace_outcomes_tests.rs"]
mod tests;
