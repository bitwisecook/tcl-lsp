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

//! Jim helper fallback performs ordinary lookup of the original selector's head.

use super::{Completion, Value, Vm};
use crate::interp::native_name_consumers::OriginalNameConsumerScope;
use tcl_cmd_core::CmdError;
use tcl_syntax::value::ValueError;

pub(super) fn dispatch(
    vm: &mut Vm,
    selector: &Value,
    arguments: &[Value],
    scope: &OriginalNameConsumerScope,
) -> Completion<Value> {
    let reached = (|| -> Result<_, CmdError> {
        scope.current_for_value(vm, selector)?;
        let context = vm.native_jim_object_context()?;
        scope.current_for_value(vm, selector)?;
        let original = scope.read(vm, selector)?;
        let head = vm
            .actual_native_invocation_dialect()
            .native_namespace_scripted_helper(&original)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "Jim original namespace helper head",
            ))?;
        let head = Value::new_native_string_bytes(head);
        head.bind_native_jim_context(&context)?;
        scope.current_for_value(vm, selector)?;
        let completion = vm.invoke_host_original_object_vector(&head, arguments);
        scope.current_for_value(vm, selector)?;
        Ok(completion)
    })();
    match reached {
        Ok(completion) => completion,
        Err(error) => crate::command::completion_from_cmd_error(vm, error),
    }
}
