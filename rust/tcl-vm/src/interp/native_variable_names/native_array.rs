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

//! Quiet Array opcodes consume actual original name caches and retain their receiver.

use super::{Completion, NativeVariableNameLookupPurpose, Value, Vm};

impl Vm {
    pub(crate) fn array_exists_original_opcode(
        &mut self,
        original: &Value,
    ) -> Result<bool, Completion<Value>> {
        let (_, name, element, resolved) = match self
            .prepare_native_original_variable(original, NativeVariableNameLookupPurpose::Array)
        {
            Ok(value) => value,
            Err(error) if self.refused_completion().is_some() => return Err(error),
            Err(_) => return Ok(false),
        };
        let cell = self.trace_cell_from_resolved(&resolved);
        let check = |vm: &mut Self| {
            vm.fire_var_traces_from_cell_bytes(
                &name,
                "array",
                element.as_deref(),
                None,
                cell.clone(),
            )?;
            Ok(resolved.elem.is_none()
                && resolved
                    .id
                    .and_then(|id| vm.var_arena.get(id))
                    .is_some_and(|cell| matches!(cell.state(), super::super::Local::Array(_))))
        };
        match cell.as_ref() {
            Some(cell) => self.with_variable_operation(cell, check),
            None => check(self),
        }
    }
    pub(crate) fn array_make_original_opcode(
        &mut self,
        original: &Value,
    ) -> Result<(), Completion<Value>> {
        let (_, name, _, resolved) = self.prepare_native_original_variable(
            original,
            NativeVariableNameLookupPurpose::ArrayMake,
        )?;
        if let Some(element) = resolved.elem.as_ref() {
            let exists = resolved.id.and_then(|id| self.var_arena.get(id)).is_some_and(|cell| matches!(cell.state(), super::super::Local::Array(elements) if elements.contains_key(element.as_bytes())));
            let original_bytes = self
                .native_name_operand_bytes(original)
                .map_err(|error| self.refuse_host_command(error.to_string()))?;
            if !exists {
                return Err(self.variable_access_error_input(
                    "set",
                    tcl_syntax::naming::NativeVariableInputForm::Combined(&original_bytes),
                    "no such element in array",
                ));
            }
            return Err(self.array_make_opcode_error(&original_bytes));
        }
        self.ensure_array_original_selected_opcode(&name, &resolved)
    }
    pub(in crate::interp) fn ensure_array_original_selected_opcode(
        &mut self,
        name: &[u8],
        resolved: &super::super::ResolvedVar,
    ) -> Result<(), Completion<Value>> {
        if resolved
            .id
            .and_then(|id| self.var_arena.get(id))
            .is_some_and(|cell| matches!(cell.state(), super::super::Local::Scalar(_)))
        {
            return Err(self.array_make_opcode_error(name));
        }
        self.ensure_array_selected(name, resolved)
    }

    fn array_make_opcode_error(&mut self, name: &[u8]) -> Completion<Value> {
        let protocol = self
            .native_c_variable_name_protocol()
            .expect("actual ARRAY_MAKE issuer");
        let mut message = b"can't array set \"".to_vec();
        message.extend_from_slice(name);
        message.extend_from_slice(b"\": variable isn't array");
        crate::command::completion_from_cmd_error(
            self,
            tcl_cmd_core::CmdError::from_byte_details(tcl_cmd_core::CmdErrorDetails {
                message,
                string_result: protocol.diagnostic_string_protocol(),
                error_code: tcl_cmd_core::CmdErrorCodeUpdate::Set(b"TCL WRITE ARRAY".to_vec()),
                error_info: None,
                error_line: None,
                primitive_getter: None,
            }),
        )
    }
}
