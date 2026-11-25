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

//! W146 for a write to a constant: a name `const` created in a scope refuses every later write there.

use crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation;
use crate::analyser::state::Analyser;
use crate::analyser::types::{Diagnostic, Severity};
use tcl_core_types::DiagCode;
use tcl_registry::arg_role::ArgRole;

impl Analyser {
    /// Record the variables a constant declaration creates, and report a later
    /// write to one in the same scope. The declaration comes from the call's
    /// declared semantics, so the analyser names no command.
    pub(in crate::analyser) fn emit_w146_constant_write(
        &mut self,
        original: Option<&OriginalDiagnosticInvocation>,
        scope_path: &[usize],
    ) {
        let Some(original) = original else { return };
        if !original.matches_analysis(&self.result, &self.source) {
            return;
        }
        let Some(Some((declares, roles))) = original.with_schema(|call| {
            let declares = call.semantics.value.declares_constant();
            call.arg_roles().map(|roles| (declares, roles))
        }) else {
            return;
        };
        for (index, _) in roles.iter().filter(|(_, role)| *role == ArgRole::VarWrite) {
            let Some(name) = original.literal(*index) else {
                continue;
            };
            let key = (scope_path.to_vec(), name.to_owned());
            if declares {
                self.constants.insert(key);
            } else if self.constants.contains(&key)
                && let Some(word) = original.word(*index)
            {
                self.result.diagnostics.push(Diagnostic::new(
                    DiagCode::W146,
                    word.span(),
                    format!("'{name}' is a constant; a later write in this scope raises an error."),
                    Severity::Warning,
                ));
            }
        }
    }
}
