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

//! Original native Switch primitives and paired entered-procedure controls.
use crate::{Value, Vm};
use tcl_cmd_core::{CmdError, switch::NativeCompiledSwitchObjects};
use tcl_dialect::TclVersion;
use tcl_syntax::value::ValueError;
impl NativeCompiledSwitchObjects for Vm {
    fn compiled_binary_bytes(
        &mut self,
        value: &Value,
        version: TclVersion,
    ) -> Result<std::rc::Rc<[u8]>, CmdError> {
        let protocol = self
            .native_invocation_dialect()
            .native_string_protocol()
            .filter(|protocol| {
                protocol.tcl_version() == Some(version) && version < TclVersion::V9_0
            })
            .ok_or(ValueError::CommandProtocolUnavailable(
                "compiled C8 binary conversion",
            ))?;
        value.check_native_header()?;
        value
            .as_native_byte_array(
                tcl_native_model::native_binary_value::NativeBinaryByteConversion::Narrow(
                    version.string_character_model(),
                ),
                protocol,
            )
            .map_err(Into::into)
    }
}
