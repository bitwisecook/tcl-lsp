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
use tcl_compiler::analyser::Analyser;

#[test]
fn accumulated_source_view_keeps_actual_call_but_replacement_retracts_it() {
    let source = "helper";
    let first = Analyser::new().analyse_with_source_namespace(source, "tcl", "::x");
    let second = Analyser::new().analyse_with_source_namespace(source, "tcl", "::y");
    let uri = "file:///calls.tcl";
    let mut index = WorkspaceIndex::from_documents([(uri, &first)]);
    let call = index
        .logical_command_call_at(uri, source, &first, 0)
        .unwrap();
    assert!(call.is_retained_in(&index));
    assert!(
        index
            .logical_command_call_at(uri, "changed", &first, 0)
            .is_none()
    );
    index.add_document(uri, &second);
    assert!(call.is_retained_in(&index));
    index.replace_document(uri, &second);
    assert!(!call.is_retained_in(&index));
    assert!(
        index
            .logical_command_call_at(uri, source, &second, 0)
            .is_some()
    );
}
