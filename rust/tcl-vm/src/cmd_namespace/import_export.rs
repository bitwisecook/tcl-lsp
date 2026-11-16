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

//! `namespace export` and `namespace import` over the namespace's export patterns.

use crate::interp::{Vm, err, ok};
use crate::value::Value;
use tcl_runtime_api::{Code, Completion};

pub(super) fn ns_export(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let original_names =
        match crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(vm) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
    let reached = (|| {
        if rest.is_empty() {
            return ok(Value::list(
                vm.exports_get()
                    .into_iter()
                    .map(|bytes| Value::from_native_string_bytes(bytes.as_bytes()))
                    .collect(),
            ));
        }
        let Some(policy) = vm.name_policy_protocol() else {
            return vm.refuse_host_command("native export protocol is unavailable".into());
        };
        if policy.recipe().is_jim084() {
            return ok(Value::empty());
        }
        let mut values = rest;
        if let Some(first) = values.first() {
            let bytes = match original_names.read(vm, first) {
                Ok(bytes) => bytes,
                Err(error) => return crate::command::completion_from_cmd_error(vm, error),
            };
            let Ok(clear) = policy.recipe().namespace_export_clears(&bytes) else {
                return vm
                    .refuse_host_command("native export control projection is unavailable".into());
            };
            if clear {
                vm.clear_exports();
                values = &values[1..];
            }
        }
        // Each native getter is followed by validation and publication before
        // visiting the next original operand. Earlier clear/exports survive failure.
        for value in values {
            let bytes = match original_names.read(vm, value) {
                Ok(bytes) => bytes,
                Err(error) => return crate::command::completion_from_cmd_error(vm, error),
            };
            let selected = match policy.recipe().namespace_pattern_input(
                &bytes,
                tcl_syntax::naming::NativeNamePurpose::NamespaceExportPattern,
            ) {
                Ok(selected) => tcl_core_types::NameBytes::from(selected.selected()),
                Err(_) => {
                    return vm.refuse_host_command(
                        "native export pattern projection is unavailable".into(),
                    );
                }
            };
            if tcl_syntax::naming::is_qualified(selected.as_bytes()) {
                let mut message = b"invalid export pattern \"".to_vec();
                message.extend_from_slice(selected.as_bytes());
                message.extend_from_slice(b"\": pattern can't specify a namespace");
                return crate::command::completion_from_cmd_error(
                    vm,
                    tcl_cmd_core::CmdError::new_bytes(message),
                );
            }
            vm.add_exports(std::slice::from_ref(&selected));
        }
        ok(Value::empty())
    })();
    if let Err(error) = original_names.finish(vm) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    reached
}

/// `namespace import ?-force? ?pattern ...?` (`NamespaceImportCmd` /
/// `Tcl_Import`, `tclNamesp.c:3668-3732`).
///
/// `-force` is positional in the same way: only `objv[1]` is read as the flag,
/// and every later word is a pattern — including a trailing `-force`, which
/// then fails the "the pattern must name a source namespace" check.
fn import_source_name_patterns(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let operation = match crate::interp::native_operation_currency::NativeOperationScope::enter(vm)
    {
        Ok(operation) => operation,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    for pattern in rest {
        let reached = tcl_syntax::value::ValueOps::native_string_bytes(vm, pattern);
        if let Err(error) = operation.check(vm) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        let original = match reached {
            Ok(bytes) => bytes,
            Err(error) => {
                return crate::command::completion_from_cmd_error(vm, error.into());
            }
        };
        let result = vm.import_source_names(&original, &operation);
        if let Err(error) = operation.check(vm) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        if result.code != Code::Ok {
            return result;
        }
    }
    ok(Value::empty())
}

pub(super) fn ns_import(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let original_names =
        match crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(vm) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
    let reached = (|| {
        if vm.dialect_profile().namespace_import_binding()
            == Some(tcl_dialect::NamespaceImportBinding::SourceName)
        {
            return import_source_name_patterns(vm, rest);
        }
        if rest.is_empty() {
            // Pinned C8.4 predates the later imported-command query form.
            let Some(protocol) = vm
                .actual_native_invocation_dialect()
                .native_namespace_name_protocol()
            else {
                return vm
                    .refuse_host_command("native import query protocol is unavailable".into());
            };
            if !protocol.import_queries_existing_commands() {
                return ok(Value::empty());
            }
            return ok(Value::list(
                vm.imported_command_tails()
                    .into_iter()
                    .map(|bytes| Value::from_native_string_bytes(bytes.as_bytes()))
                    .collect(),
            ));
        }
        let Some(policy) = vm.name_policy_protocol() else {
            return vm.refuse_host_command("native import protocol is unavailable".into());
        };
        let first = match original_names.read(vm, &rest[0]) {
            Ok(bytes) => bytes,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        let Ok(allow_overwrite) = policy.recipe().namespace_import_forces(&first) else {
            return vm.refuse_host_command("native import control is unavailable".into());
        };
        for operand in &rest[usize::from(allow_overwrite)..] {
            // Native repeats the original first getter when it was not -force.
            let pattern = match original_names.read(vm, operand) {
                Ok(bytes) => bytes,
                Err(error) => return crate::command::completion_from_cmd_error(vm, error),
            };
            let Ok(projection) = policy.recipe().namespace_pattern_input(
                &pattern,
                tcl_syntax::naming::NativeNamePurpose::NamespaceImportPattern,
            ) else {
                return vm
                    .refuse_host_command("native import pattern projection is unavailable".into());
            };
            let selected = projection.selected();
            match vm.original_c_import_preload(selected, &original_names) {
                Ok(Some(completion)) => return completion,
                Ok(None) => {}
                Err(error) => return crate::command::completion_from_cmd_error(vm, error),
            }
            let destination = match original_names.current_namespace_token(vm) {
                Ok(destination) => destination,
                Err(error) => return crate::command::completion_from_cmd_error(vm, error),
            };
            let imported =
                match tcl_cmd_core::namespace::import_pattern_checked(vm, destination, selected) {
                    Ok(Ok(imported)) => imported,
                    Ok(Err(problem)) => return err(problem.message()),
                    Err(error) => {
                        return crate::command::completion_from_cmd_error(vm, error.into());
                    }
                };
            if let Err(problem) =
                vm.import_commands_selected(selected, &imported, allow_overwrite, &original_names)
            {
                return problem.completion(vm);
            }
            if let Err(error) = original_names.check(vm) {
                return crate::command::completion_from_cmd_error(vm, error);
            }
        }
        ok(Value::empty())
    })();
    if let Err(error) = original_names.finish(vm) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    reached
}
