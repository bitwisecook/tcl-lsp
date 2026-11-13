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

//! The original Jim Import helper's alias-chain walk stays finite.

use super::{Completion, Value, Vm};

impl Vm {
    /// Record one `exists -alias` answer reached from the original Import
    /// helper's own activation. jimsh never leaves a walk that revisits an
    /// alias, so a repeated name refuses instead of supplying a guest result.
    /// A false answer ends the walk for the current candidate.
    pub(crate) fn observe_original_jim_import_alias(
        &mut self,
        name: &[u8],
        exists: bool,
    ) -> Result<(), Completion<Value>> {
        let Some(frame) = self.frames.last_mut().filter(|frame| {
            frame
                .native_procedure_execution
                .as_ref()
                .is_some_and(|procedure| {
                    procedure
                        .declaration()
                        .requires_original_jim_import_traversal()
                })
        }) else {
            return Ok(());
        };
        if !exists {
            frame.jim_import_aliases.clear();
            return Ok(());
        }
        if frame.jim_import_aliases.insert(name.to_vec()) {
            return Ok(());
        }
        frame.jim_import_aliases.clear();
        Err(crate::command::completion_from_cmd_error(
            self,
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim namespace import cyclic alias chain",
            )
            .into(),
        ))
    }
}

#[cfg(test)]
mod tests;
