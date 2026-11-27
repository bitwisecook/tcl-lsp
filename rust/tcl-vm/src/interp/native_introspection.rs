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

//! Original frame introspection retains real argv and selected numeric getter caches.

use super::{Value, Vm};
use tcl_cmd_core::native_info_level::NativeInfoLevelObjects;
use tcl_syntax::{
    scalar_getter::{NativeScalarGetterKind, NativeScalarGetterValue},
    value::ValueError,
};

impl NativeInfoLevelObjects for Vm {
    fn native_level_integer(
        &mut self,
        original: &Value,
        kind: NativeScalarGetterKind,
    ) -> Result<i64, ValueError> {
        let dialect = self.actual_native_invocation_dialect();
        if dialect.native_error_log_protocol().is_none() {
            return Err(ValueError::CommandProtocolUnavailable(
                "native info level issuer",
            ));
        }
        match super::native_numeric_ops::OriginalNumericOps::entered_value(self)?
            .getter_value(original, kind)?
        {
            NativeScalarGetterValue::Wide(integer) => Ok(integer),
            _ => Err(ValueError::ScalarNumericInputUnavailable),
        }
    }
    fn native_level_arguments(&self, level: usize) -> Result<Option<Value>, ValueError> {
        let Some(arguments) = self.frame_argv(level) else {
            return Ok(None);
        };
        if arguments.is_empty() {
            return Err(ValueError::CommandProtocolUnavailable(
                "native original frame argv",
            ));
        }
        Ok(Some(Value::list(arguments)))
    }
}

impl Vm {
    pub(crate) fn native_info_level(
        &mut self,
        original: Option<&Value>,
    ) -> Result<Value, tcl_cmd_core::CmdError> {
        let dialect = self.actual_native_invocation_dialect();
        let version = dialect
            .tcl_version
            .filter(|version| {
                *version >= tcl_dialect::TclVersion::V8_6
                    && dialect.native_error_log_protocol().is_some()
            })
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native info level opcode",
            ))?;
        tcl_cmd_core::native_info_level::native_level(
            self,
            original,
            tcl_registry::native_introspection_compilation::native_info_level_getter(version),
            tcl_syntax::native_string::NativeStringProtocol::C(version),
        )
    }
}
