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
fn logical_rename_proposals_share_qualification_and_reject_unsafe_public_inputs() {
    for (prefix, qualified, declaration) in [
        ("", "::replacement", "replacement"),
        ("::A", "::A::replacement", "::A::replacement"),
        (":::", ":::::replacement", ":::::replacement"),
    ] {
        assert_eq!(
            qualified_and_decl_text(prefix, "replacement"),
            (qualified.to_owned(), declaration.to_owned())
        );
    }
    let source = "proc original {} {}; original";
    let analysis = Analyser::new().analyse(source, "tcl");
    let index = crate::workspace_index::WorkspaceIndex::from_documents([("provider", &analysis)]);
    assert!(index.allows_lexical_rename_advice());
    for replacement in ["", "::escape", "nested::escape", "has space", "$computed"] {
        assert_eq!(
            cross_document_symbol_edits("::original", replacement, &index, "caller"),
            [] as [crate::rename::WorkspaceTextEdit; 0]
        );
        assert!(workspace_symbol_rename_edits("::original", replacement, &index).is_none());
    }
    assert!(workspace_symbol_rename_edits("::original", "replacement", &index).is_some());
}
