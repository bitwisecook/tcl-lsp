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

//! The shim that runs a host command as a VM native command: projected invocation, retirement and the completions a host outcome becomes.

use super::original_recovery::recover_original_completion;
use super::value_conversion::{ActualScalarImport, from_vm_object_value, from_vm_value_using};
use super::{
    HostCommandShim, VmOriginalObject, VmOriginalValue, VmRegistrar, check_projection, limit_kind,
    projection_error, retain_engine_failure, to_vm_code,
};
use std::cell::RefCell;
use std::rc::Rc;
use tcl_engine_api::{EngineError, HostOutcome, Value};
use tcl_vm::{Code, Completion, NativeCommand, Vm};

/// A host outcome as the completion the VM receives. Only a `Return` has a
/// use for options, as in C Tcl, where `catch` of any other code reports its
/// own.
fn host_outcome_completion(
    outcome: HostOutcome,
) -> Completion<tcl_engine_api::OriginalObjectResult> {
    let options = if outcome.code == tcl_engine_api::CompletionCode::Return {
        outcome.options
    } else {
        Value::Empty
    };
    Completion::new(
        to_vm_code(outcome.code),
        tcl_engine_api::OriginalObjectResult::Value(outcome.value),
        tcl_engine_api::OriginalObjectResult::Value(options),
    )
}

fn host_error_completion(vm: &mut Vm, error: EngineError) -> Completion<tcl_vm::Value> {
    match error {
        EngineError::Script { message, code } => {
            let options = code.map_or_else(tcl_vm::Value::empty, |code| {
                tcl_vm::Value::list(vec![
                    tcl_vm::Value::string("-code"),
                    tcl_vm::Value::int(1),
                    tcl_vm::Value::string("-level"),
                    tcl_vm::Value::int(0),
                    tcl_vm::Value::string("-errorcode"),
                    tcl_vm::Value::string(code),
                ])
            });
            Completion::new(Code::Error, tcl_vm::Value::string(message), options)
        }
        EngineError::ScriptBytes {
            message,
            code,
            options,
        } => {
            let options =
                options.map_or_else(|| error_options(code), tcl_vm::Value::from_string_bytes);
            Completion::new(
                Code::Error,
                tcl_vm::Value::from_string_bytes(message),
                options,
            )
        }
        EngineError::ExecutionFailure(first) => {
            vm.refuse_tcl_host_failure(tcl_vm::TclHostFailure::Execution(*first))
        }
        EngineError::ExecutionRefusal(reason) => vm.refuse_host_command(reason),
        // A budget the host command outran is recorded as the VM's own
        // breach, so the invocation reports it even past a `catch`.
        EngineError::BudgetExceeded(kind) => vm.exceed_limit(limit_kind(kind)),
        error => vm.refuse_host_command(error.to_string()),
    }
}

fn retirement_error_completion(vm: &mut Vm, error: EngineError) -> Completion<tcl_vm::Value> {
    match error {
        EngineError::ExecutionFailure(first) => {
            vm.refuse_tcl_host_failure(tcl_vm::TclHostFailure::Execution(*first))
        }
        error => vm.refuse_host_command(error.to_string()),
    }
}

fn projection_unwind(vm: &mut Vm, error: tcl_vm::TclError) -> Completion<tcl_vm::Value> {
    match error.into_completion() {
        Ok(completion) => completion,
        Err(first) => vm.refuse_tcl_host_failure(first),
    }
}

impl HostCommandShim {
    pub(super) fn invoke_projected(
        &self,
        vm: &mut Vm,
        original: &tcl_vm::embed::OriginalOperandProjection,
        scalar: &tcl_vm::NativeScalarIngress,
        source: &[tcl_vm::Value],
    ) -> Result<Completion<tcl_vm::Value>, EngineError> {
        let dialect = vm.native_scalar_carrier_dialect();
        let interpreter = vm.native_object_bridge_identity();
        let view = self.command.argument_view();
        let mut originals: Vec<Rc<dyn tcl_engine_api::OriginalObject>> = Vec::new();
        let mut arguments = Vec::new();
        let reached = (|| {
            if view == tcl_engine_api::HostArgumentView::OriginalObjects {
                for value in source {
                    vm.check_original_projected_value(original, value)
                        .map_err(|error| projection_error(vm, original, error))?;
                    originals.push(Rc::new(VmOriginalObject {
                        value: VmOriginalValue::Header(value.native_lifetime_lease()),
                        dialect,
                        interpreter,
                    }));
                }
            }
            for value in source {
                vm.check_original_projected_value(original, value)
                    .map_err(|error| projection_error(vm, original, error))?;
                let reached = match view {
                    tcl_engine_api::HostArgumentView::MaterializedStrings => {
                        from_vm_value_using(vm, original, value)
                    }
                    tcl_engine_api::HostArgumentView::NativeObjectSnapshots => {
                        from_vm_object_value(value)
                    }
                    tcl_engine_api::HostArgumentView::OriginalObjects => Ok(Value::Empty),
                };
                if let Err(error) = &reached {
                    retain_engine_failure(vm, original, error);
                }
                check_projection(vm, original)?;
                arguments.push(reached?);
            }
            vm.check_original_scalar_ingress(scalar)
                .map_err(EngineError::execution_failure)?;
            let mut registrar = VmRegistrar {
                vm,
                host_commands: Rc::clone(&self.host_commands),
                original,
            };
            let outcome = if view == tcl_engine_api::HostArgumentView::OriginalObjects {
                self.command
                    .invoke_original_completion_with_registrar(&mut registrar, &originals)
            } else {
                self.command
                    .invoke_with_registrar(&mut registrar, &arguments)
                    .map(host_outcome_completion)
            };
            if let Err(error) = &outcome {
                retain_engine_failure(registrar.vm, original, error);
            }
            // Retain the original callback cause before its returned ownership retires.
            let current = check_projection(registrar.vm, original);
            let scalar_current = registrar
                .vm
                .check_original_scalar_ingress(scalar)
                .map_err(EngineError::execution_failure);
            if let Err(first) = current.and(scalar_current) {
                drop(outcome);
                check_projection(registrar.vm, original)?;
                return Err(first);
            }
            match outcome {
                Ok(result) => {
                    let reached = {
                        let actual = ActualScalarImport {
                            vm: RefCell::new(&mut *registrar.vm),
                            original: scalar,
                        };
                        recover_original_completion(&result, dialect, interpreter, &actual)
                    };
                    if let Err(error) = &reached {
                        retain_engine_failure(registrar.vm, original, error);
                    }
                    let current = check_projection(registrar.vm, original);
                    drop(result);
                    check_projection(registrar.vm, original)?;
                    current?;
                    reached
                }
                Err(error) => Ok(host_error_completion(registrar.vm, error)),
            }
        })();
        if let Err(error) = &reached {
            retain_engine_failure(vm, original, error);
        }
        drop(arguments);
        drop(originals);
        check_projection(vm, original)?;
        reached
    }
}

impl NativeCommand for HostCommandShim {
    fn retire(&self, vm: &mut Vm) -> Completion<tcl_vm::Value> {
        let original = match vm.capture_original_operand_projection() {
            Ok(original) => original,
            Err(error) => return projection_unwind(vm, error),
        };
        let reached = {
            let mut registrar = VmRegistrar {
                vm,
                host_commands: Rc::clone(&self.host_commands),
                original: &original,
            };
            self.command.retire_with_registrar(&mut registrar)
        };
        if let Err(error) = &reached {
            // Every retirement failure remains Host, including Script/Budget;
            // latch it before mandatory original-owner settlement.
            drop(retirement_error_completion(vm, error.clone()));
        }
        let current = check_projection(vm, &original);
        let retired = vm.finish_original_operand_projection(original);
        match current {
            Err(first) => retirement_error_completion(vm, first),
            Ok(()) => match retired {
                Err(first) => projection_unwind(vm, first),
                Ok(()) => match reached {
                    Ok(()) => {
                        Completion::new(Code::Ok, tcl_vm::Value::empty(), tcl_vm::Value::empty())
                    }
                    Err(error) => retirement_error_completion(vm, error),
                },
            },
        }
    }

    fn invoke(&self, vm: &mut Vm, arguments: &[tcl_vm::Value]) -> Completion<tcl_vm::Value> {
        let scalar = match vm.capture_original_scalar_ingress() {
            Ok(original) => original,
            Err(first) => {
                return vm.refuse_tcl_host_failure(tcl_vm::TclHostFailure::Execution(first));
            }
        };
        let original = match vm.capture_original_operand_projection() {
            Ok(original) => original,
            Err(error) => {
                let completion = projection_unwind(vm, error);
                let _ = vm.finish_original_scalar_ingress(scalar);
                return completion;
            }
        };
        let reached = self.invoke_projected(vm, &original, &scalar, arguments);
        if let Err(error) = &reached {
            retain_engine_failure(vm, &original, error);
        }
        if let Err(first) = check_projection(vm, &original) {
            drop(reached);
            let _ = check_projection(vm, &original);
            let _ = vm.finish_original_operand_projection(original);
            let _ = vm.finish_original_scalar_ingress(scalar);
            return host_error_completion(vm, first);
        }
        if let Err(first) = vm.finish_original_scalar_ingress(scalar) {
            drop(reached);
            let _ = check_projection(vm, &original);
            let _ = vm.finish_original_operand_projection(original);
            return vm.refuse_tcl_host_failure(tcl_vm::TclHostFailure::Execution(first));
        }
        if let Err(first) = vm.finish_original_operand_projection(original) {
            drop(reached);
            return projection_unwind(vm, first);
        }
        reached.unwrap_or_else(|error| host_error_completion(vm, error))
    }
}

fn error_options(code: Option<Vec<u8>>) -> tcl_vm::Value {
    code.map_or_else(tcl_vm::Value::empty, |code| {
        tcl_vm::Value::list(vec![
            tcl_vm::Value::string("-code"),
            tcl_vm::Value::int(1),
            tcl_vm::Value::string("-level"),
            tcl_vm::Value::int(0),
            tcl_vm::Value::string("-errorcode"),
            tcl_vm::Value::from_string_bytes(code),
        ])
    })
}
