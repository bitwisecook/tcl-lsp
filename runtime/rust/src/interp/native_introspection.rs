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

//! Native frame introspection uses authentic caller argv without string reconstruction.

use super::Interp;
use crate::obj::TclObj;
use tcl_cmd_core::native_info_level::NativeInfoLevelObjects;
use tcl_syntax::{
    scalar_getter::{NativeScalarGetterKind, NativeScalarGetterValue},
    value::ValueError,
};

impl NativeInfoLevelObjects for Interp {
    fn native_level_integer(
        &mut self,
        original: &*mut TclObj,
        kind: NativeScalarGetterKind,
    ) -> Result<i64, ValueError> {
        let dialect = self.native_invocation_dialect();
        if dialect.native_error_log_protocol().is_none() {
            return Err(ValueError::CommandProtocolUnavailable(
                "native info level issuer",
            ));
        }
        match crate::typed_value::native_scalar_getter(*original, dialect, kind)? {
            NativeScalarGetterValue::Wide(integer) => Ok(integer),
            _ => Err(ValueError::ScalarNumericInputUnavailable),
        }
    }
    fn native_level_arguments(&self, level: usize) -> Result<Option<*mut TclObj>, ValueError> {
        if level == 0 || level > self.frames.borrow().current_level() {
            return Ok(None);
        }
        let arguments = self
            .frames
            .borrow()
            .original_error_stack_argv_at(level)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native original frame argv",
            ))?;
        Ok(Some(self.new_list_object(&arguments)))
    }
}
impl Interp {
    pub(crate) fn native_info_level(
        &mut self,
        original: Option<&*mut TclObj>,
    ) -> Result<*mut TclObj, tcl_cmd_core::CmdError> {
        let dialect = self.native_invocation_dialect();
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
