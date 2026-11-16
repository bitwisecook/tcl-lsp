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
fn original_refactor_actions_keep_document_registry_and_full_source() {
    // Implementation contract: naming.refactor.original-document-context
    // docs/design/analysis/name-resolution-proofs/refactor-original-document-context.md
    let source = "if {$host eq \"a.com\"} {pool web_pool} elseif {$host eq \"b.com\"} {pool web_pool} elseif {$host eq \"c.com\"} {pool web_pool}";
    let range = LspRange {
        start_line: 0,
        start_character: 0,
        end_line: 0,
        end_character: 0,
    };
    let index = LineIndex::new(source);
    let mut plain = Analyser::new().analyse(source, "tcl8.6");
    assert!(
        !extract_inline_actions(source, range, &plain, &index, None)
            .iter()
            .any(|action| action.data_group_definition.is_some())
    );
    plain.dialect = "f5-irules".to_owned();
    assert!(
        !extract_inline_actions(source, range, &plain, &index, None)
            .iter()
            .any(|action| action.data_group_definition.is_some())
    );
    let mut vendor = Analyser::new().analyse(source, "f5-irules");
    let actions = extract_inline_actions(source, range, &vendor, &index, None);
    assert!(
        actions
            .iter()
            .any(|action| action.data_group_definition.is_some()),
        "{actions:?}"
    );
    vendor.dialect = "tcl8.6".to_owned();
    assert!(
        extract_inline_actions(source, range, &vendor, &index, None)
            .iter()
            .any(|action| action.data_group_definition.is_some())
    );
    let displaced = format!("#{source}");
    assert_eq!(
        extract_inline_actions(
            &displaced,
            range,
            &vendor,
            &LineIndex::new(&displaced),
            None
        ),
        [] as [crate::code_actions::CodeAction; 0]
    );
    vendor.body_lexer_config = None;
    assert_eq!(
        extract_inline_actions(source, range, &vendor, &index, None),
        [] as [crate::code_actions::CodeAction; 0]
    );
}
