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

//! Original scalar getters and genuine native integer result manufacture.

use super::{Value, Vm};
use tcl_registry::native_scalar_compilation::NativeScalarOperation;
use tcl_syntax::value::{ValueError, ValueOps};

impl Vm {
    /// Only the admitted non-strict C ASCII `STR_CLASS` form reaches this worker.
    pub(super) fn execute_native_ascii_predicate(
        &mut self,
        original: &Value,
        version: tcl_dialect::TclVersion,
    ) -> Result<Value, tcl_cmd_core::CmdError> {
        let ops = crate::interp::native_numeric_ops::OriginalNumericOps::entered(self)?;
        let dialect = ops.dialect();
        if version < tcl_dialect::TclVersion::V8_6
            || dialect.tcl_version != Some(version)
            || dialect.native_error_log_protocol().is_none()
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "native compiled ASCII predicate issuer",
            )
            .into());
        }
        let protocol =
            dialect
                .native_string_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "native compiled ASCII Unicode getter",
                ))?;
        let units = original.native_unicode_units_checked(
            protocol,
            &mut || ops.current_for_value(original),
            &mut || ops.string(original, protocol),
        )?;
        ops.current_for_value(original)?;
        let matched = units.iter().all(|unit| *unit <= 0x7f);
        drop(units);
        ops.with_vm_for_value(original, |vm| {
            vm.native_c_execution_boolean(matched, version)
                .map_err(Into::into)
        })
    }

    pub(super) fn execute_native_scalar_length(
        &mut self,
        original: &Value,
        operation: NativeScalarOperation,
        version: tcl_dialect::TclVersion,
    ) -> Result<Value, ValueError> {
        let dialect = self.actual_native_invocation_dialect();
        if dialect.native_error_log_protocol().is_none() || dialect.tcl_version != Some(version) {
            return Err(ValueError::CommandProtocolUnavailable(
                "native scalar length result issuer",
            ));
        }
        let length = match operation {
            NativeScalarOperation::StringLength => self.native_char_len(original)?,
            NativeScalarOperation::ListLength => self.list_len(original)?,
            NativeScalarOperation::StringEqual | NativeScalarOperation::StringIsAscii => {
                return Err(ValueError::CommandProtocolUnavailable(
                    "native length instruction",
                ));
            }
        };
        let length = i64::try_from(length)
            .map_err(|_| ValueError::CommandProtocolUnavailable("native scalar length width"))?;
        let result = Value::int(length);
        result.set_native_unshared_integer(length, version)?;
        Ok(result)
    }
}
