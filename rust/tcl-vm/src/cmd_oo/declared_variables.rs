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

//! `variable` declarations of a class or object and the storage names of private variables.

use super::definition::active_target;
use super::{DeclaredVariable, err_code, native_context, original_name_operands, variable_mapping};
use crate::interp::{Vm, ok};
use crate::value::Value;
use tcl_core_types::NameBytes;
use tcl_runtime_api::{Code, Completion, OoId};

pub(super) fn declared_variable_error(
    error: tcl_syntax::naming::NativeOoVariableError,
) -> Completion<Value> {
    let code = Value::list(
        error
            .error_code
            .into_iter()
            .map(Value::from_native_string_bytes)
            .collect(),
    );
    Completion::new(
        Code::Error,
        Value::from_native_string_bytes(error.message),
        Value::list(vec![Value::string("-errorcode"), code]),
    )
}

/// Read original variable operands and select their native slot operation.
fn read_declared_variable_selection(
    vm: &mut Vm,
    args: &[Value],
    private: bool,
    selected_mapping: bool,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    original_names: &original_name_operands::OriginalOoNameScope,
) -> Result<
    (
        Vec<DeclaredVariable>,
        tcl_syntax::naming::NativeOoVariableSlotOperation,
    ),
    Completion<Value>,
> {
    use tcl_syntax::naming::{NativeOoVariableSlotOperation, native_oo_variable_slot};
    let mut incoming = Vec::new();
    for original in args {
        let bytes = match original_names.read(vm, original) {
            Ok(bytes) => bytes,
            Err(error) => return Err(crate::command::completion_from_cmd_error(vm, error)),
        };
        incoming.push(DeclaredVariable {
            original: if selected_mapping {
                original.native_lifetime_lease().into_value()
            } else {
                original.clone()
            },
            name: NameBytes::from(bytes.as_ref()),
            full_name: None,
        });
    }
    let selection = match native_oo_variable_slot(
        policy.recipe(),
        incoming.first().map(|entry| entry.name.as_bytes()),
    ) {
        Ok(Ok(selection)) => selection,
        Ok(Err(error)) => return Err(declared_variable_error(error)),
        Err(error) => return Err(vm.refuse_host_command(format!("{error:?}"))),
    };
    if selection.consumes_first {
        incoming.remove(0);
    }
    if selection.operation == NativeOoVariableSlotOperation::Clear && !incoming.is_empty() {
        let words = if private {
            vec![
                Value::string("private"),
                Value::string("variable"),
                args[0].clone(),
            ]
        } else {
            vec![Value::string("variable"), args[0].clone()]
        };
        let usage = vm.native_argument_usage_header(&words)?;
        return Err(crate::command::native_wrong_args_bytes(vm, &usage));
    }
    Ok((incoming, selection.operation))
}

fn updated_declared_variables(
    selected_mapping: bool,
    existing: Vec<DeclaredVariable>,
    incoming: Vec<DeclaredVariable>,
    operation: tcl_syntax::naming::NativeOoVariableSlotOperation,
) -> Vec<DeclaredVariable> {
    if selected_mapping {
        tcl_syntax::naming::apply_native_oo_slot_records(existing, incoming, operation, |entry| {
            entry.name.as_bytes()
        })
    } else {
        tcl_syntax::naming::apply_native_oo_variable_slot(existing, incoming, operation, |entry| {
            entry.name.as_bytes()
        })
    }
}

/// Write one `variable` declaration into the target's public or private slot.
pub(super) fn declare_variable(vm: &mut Vm, args: &[Value], private: bool) -> Completion<Value> {
    let target = active_target(vm);
    original_name_operands::run(vm, target, |vm, original_names| {
        use tcl_syntax::naming::{NativeOoVariableSlotOperation, validate_native_oo_variable};
        let Some((is_class, target)) = active_target(vm) else {
            return err_code(
                "this command may only be called from within the context of an ::oo::define or ::oo::objdefine command",
                &["TCL", "OO", "MONKEY_BUSINESS"],
            );
        };
        let Some(policy) = vm.name_policy_protocol() else {
            return vm.refuse_host_command(
                "TclOO variable declaration protocol is unavailable".to_owned(),
            );
        };
        let selected_mapping = variable_mapping::selected(policy);
        let (incoming, operation) = match read_declared_variable_selection(
            vm,
            args,
            private,
            selected_mapping,
            policy,
            original_names,
        ) {
            Ok(selected) => selected,
            Err(completion) => return completion,
        };
        if incoming.is_empty()
            && !matches!(
                operation,
                NativeOoVariableSlotOperation::Set | NativeOoVariableSlotOperation::Clear
            )
        {
            return ok(Value::empty());
        }
        let existing = if selected_mapping
            && matches!(
                operation,
                NativeOoVariableSlotOperation::Set | NativeOoVariableSlotOperation::Clear
            ) {
            Some(Vec::new())
        } else if selected_mapping {
            variable_mapping::snapshot(vm, is_class, target, private)
        } else {
            declared_variable_snapshot(vm, is_class, target, private)
        };
        let Some(mut existing) = existing else {
            return ok(Value::empty());
        };
        for entry in &mut existing {
            let bytes = match original_names.read(vm, &entry.original) {
                Ok(bytes) => bytes,
                Err(error) => return crate::command::completion_from_cmd_error(vm, error),
            };
            entry.name = NameBytes::from(bytes.as_ref());
        }
        let updated = updated_declared_variables(selected_mapping, existing, incoming, operation);
        for entry in &updated {
            match validate_native_oo_variable(policy.recipe(), entry.name.as_bytes()) {
                Ok(None) => {}
                Ok(Some(error)) => return declared_variable_error(error),
                Err(error) => return vm.refuse_host_command(format!("{error:?}")),
            }
        }
        if selected_mapping {
            if let Err(error) = variable_mapping::install(
                vm,
                original_names,
                is_class,
                target,
                private,
                policy,
                &updated,
            ) {
                return crate::command::completion_from_cmd_error(vm, error);
            }
            drop(updated);
            if let Err(error) = original_names.check(vm) {
                return crate::command::completion_from_cmd_error(vm, error);
            }
        } else {
            publish_declared_variables(vm, is_class, target, private, updated);
        }
        if let Err(error) = original_names.check_target(vm) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        ok(Value::empty())
    })
}

fn publish_declared_variables(
    vm: &mut Vm,
    is_class: bool,
    target: OoId,
    private: bool,
    updated: Vec<DeclaredVariable>,
) {
    let slot = if is_class {
        vm.oo.classes.get_mut(&target).map(|class| {
            if private {
                &mut class.private_variables
            } else {
                &mut class.variables
            }
        })
    } else {
        vm.oo.objects.get_mut(&target).map(|object| {
            if private {
                &mut object.private_variables
            } else {
                &mut object.variables
            }
        })
    };
    if let Some(slot) = slot {
        *slot = updated;
    }
}

fn declared_variable_snapshot(
    vm: &Vm,
    is_class: bool,
    target: OoId,
    private: bool,
) -> Option<Vec<DeclaredVariable>> {
    if is_class {
        vm.oo.classes.get(&target).map(|class| {
            if private {
                &class.private_variables
            } else {
                &class.variables
            }
        })
    } else {
        vm.oo.objects.get(&target).map(|object| {
            if private {
                &object.private_variables
            } else {
                &object.variables
            }
        })
    }
    .cloned()
}

/// The storage name a TIP 500 private variable occupies in the object's
/// namespace: C's `PRIVATE_VARIABLE_PATTERN`, `"<creation id> : <name>"`,
/// where the id is the *declaring provider's*.
///
/// That is what keeps a `Base` and a `Derived` each declaring `X` from sharing
/// one slot — measured on tclsh 9.0.4 and 9.1b0, an object of such a `Derived`
/// holds both `20 : X` and `22 : X`.
pub(super) fn private_storage_name_bytes(creation_id: u64, name: &[u8]) -> NameBytes {
    let mut storage = format!("{creation_id} : ").into_bytes();
    storage.extend_from_slice(name);
    NameBytes::from(storage)
}

/// The independently current declaring provider's private storage correspondence.
pub(super) fn frame_private_storage_name(
    vm: &Vm,
    name: &[u8],
    purpose: tcl_syntax::naming::NativeOoPrivateVariablePurpose,
) -> Option<NameBytes> {
    let protocol = vm.name_policy_protocol()?.recipe();
    let frame = native_context::current_method(vm)?;
    let step = frame.chain.get(frame.index)?;
    let declared = if step.is_object {
        &vm.oo.objects.get(&step.provider)?.private_variables
    } else {
        &vm.oo.classes.get(&step.provider)?.private_variables
    };
    let declaration = declared.iter().find(|declaration| {
        tcl_syntax::naming::native_oo_private_variable_matches(
            protocol,
            purpose,
            declaration.name.as_bytes(),
            name,
        ) == Ok(true)
    })?;
    let creation = vm.oo.objects.get(&step.provider)?.creation_id;
    Some(declaration.private_storage(creation))
}
