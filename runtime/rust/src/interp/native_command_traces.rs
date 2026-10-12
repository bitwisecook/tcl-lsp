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

//! Counted command and selected execution-trace callback source boundaries.

use super::{Code, Interp};

impl Interp {
    /// Evaluate the assembled native callback under its actual C entry recipe.
    pub(super) fn eval_native_command_trace_script(
        &mut self,
        execution: bool,
        assembled: &[u8],
    ) -> Code {
        let Some(protocol) = self
            .native_invocation_dialect()
            .native_variable_trace_protocol()
        else {
            return self.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "command trace callback source",
                )
                .into(),
            );
        };
        self.eval_str(protocol.command_callback_source(execution, assembled))
    }
}
