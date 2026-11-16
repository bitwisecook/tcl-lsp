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

use super::*;

#[test]
fn actual_unavailable_cfg_entry_cannot_reconstruct_source_config_on_rebuild() {
    let registry = CommandRegistry::build_default();
    let refused = unavailable_cfg_entry(&registry);
    assert_eq!(refused.source_variables.read_lexer_config(&registry), None);
    let rebuilt = ModuleCommandBindings::initial_entry_from_baseline(
        &registry,
        Arc::clone(&refused.baseline),
    );
    assert_eq!(rebuilt.source_variables.read_lexer_config(&registry), None);
    let child = rebuilt
        .source_variables
        .in_frame(&crate::var_resolve::VariableExecutionFrame::Global);
    assert_eq!(child.read_lexer_config(&registry), None);
}
