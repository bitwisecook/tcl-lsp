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

//! Ordinary compiled UPVAR selects an actual frame and binds its exact local slot.

use super::*;

impl Interp {
    pub(in crate::interp) fn link_original_compiled_upvar(
        &mut self,
        original: *mut TclObj,
        level: usize,
        slot: usize,
    ) -> Code {
        let purpose = NativeVariableNameLookupPurpose::Link;
        let mut target = match self.prepare_original_c_link_target(original, level) {
            Ok(Some(target)) => target,
            Ok(None) => {
                return self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("original compiled upvar receiver")
                        .into(),
                )
            }
            Err(code) => return code,
        };
        if let Err(error) = self.prepare_upvar_target(&mut target) {
            return self.original_c_lookup_var_error(original, purpose, error);
        }
        self.settle_original_c_local_alias(Some(slot), b"", target)
    }
}
