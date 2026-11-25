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
use crate::loader::block_descriptors::definition_body_block;

#[test]
fn actual_authored_sets_select_custom_hints_and_preserve_exact_snittype_recipe() {
    let mut log = Log::default();
    let rows = statements(
        "bare_word_construction -hint-values {spawn café} -hint-prefixes {obj-}",
        1,
        FileBom::Content,
    );
    let grammar = definition_body_block(&rows, &mut log);
    let hint = grammar.bare_word_construction_source_hint.unwrap();
    assert!(hint.recognises("spawn"));
    assert!(hint.recognises("café"));
    assert!(hint.recognises("obj-child"));
    assert!(!hint.recognises("%AUTO%"));
    assert!(!hint.recognises(".widget"));
    assert!(grammar.bare_word_construction_hint.is_none());
    let rows = statements(
        "bare_word_construction -hint-values {%AUTO%} -hint-prefixes {.}",
        1,
        FileBom::Content,
    );
    let grammar = definition_body_block(&rows, &mut log);
    let callback = grammar.bare_word_construction_hint.unwrap();
    assert!(callback("%AUTO%"));
    assert!(callback(".widget"));
    assert!(!callback("spawn"));
    assert_eq!(log.notices, [] as [crate::loader::notices::Notice; 0]);
}

#[test]
fn bare_flags_and_unavailable_option_layouts_do_not_invent_snittype_hints() {
    for source in [
        "bare_word_construction",
        "bare_word_construction -hint-values",
        "bare_word_construction -foreign {x}",
        "bare_word_construction -hint-values {x} -hint-values {y}",
        "bare_word_construction -hint-prefixes {{}}",
    ] {
        let mut log = Log::default();
        let grammar = definition_body_block(&statements(source, 1, FileBom::Content), &mut log);
        assert!(grammar.bare_word_construction);
        assert!(
            grammar.bare_word_construction_source_hint.is_none(),
            "{source}"
        );
        assert!(grammar.bare_word_construction_hint.is_none(), "{source}");
        if source != "bare_word_construction" {
            assert!(!log.notices.is_empty(), "{source}");
        }
    }
}
