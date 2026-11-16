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
fn actual_expansion_child_registry_hover_does_not_require_a_static_parent_word() {
    // Exact existing Registry expansion-child positive; its genuine child
    // owner is independent of the expanded containing word's label.
    let source = "string {*}{length value}";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    analysis.all_procs.clear();
    analysis.all_classes.clear();
    analysis.command_invocations.clear();
    let cursor = u32::try_from(source.find("length").unwrap() + 1).unwrap();
    assert!(!analysis.allows_lexical_declaration_advice());
    assert!(
        crate::source_structure::original_complete_cursor_word(source, &analysis, cursor)
            .and_then(|word| word.static_unicode_value())
            .is_none()
    );
    let original = original_registry::hover(source, &analysis, cursor)
        .expect("actual original expansion-child Registry owner");
    assert!(original.value.contains("`string length`"));
    let position = tcl_lexer::LineIndex::new_lsp(source).position_at_utf16(cursor, source);
    let selected = hover(
        source,
        position.line,
        position.character.get(),
        &analysis,
        None,
    )
    .expect("independent Registry branch remains reachable");
    assert_eq!(selected.value, original.value);
}
