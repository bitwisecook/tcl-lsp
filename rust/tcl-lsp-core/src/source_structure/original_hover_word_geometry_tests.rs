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
fn complete_cursor_word_uses_whole_configured_original_groups() {
    for (source, needle, expected) in [
        ("puts {café name}", "café", "café name"),
        ("puts \"café name\"", "café", "café name"),
        ("puts {first\nsecond}", "second", "first\nsecond"),
        ("puts a\\ b", "a", "a b"),
    ] {
        let analysis = Analyser::new().analyse(source, "tcl");
        let offset = u32::try_from(source.find(needle).unwrap()).unwrap();
        let selected = original_complete_cursor_word(source, &analysis, offset)
            .expect("actual complete original word");
        assert_eq!(selected.static_unicode_value().as_deref(), Some(expected));
        assert_eq!(selected.cursor.word_index(), 1);
        assert_eq!(selected.word.image().bytes(), source.as_bytes());
        assert_eq!(selected.word.config(), analysis.body_lexer_config.unwrap());
        assert!(
            original_complete_cursor_word(&source.replace("puts", "xxxx"), &analysis, offset)
                .is_none()
        );
    }
}

#[test]
fn complete_cursor_word_does_not_recover_computed_tails_or_partial_words() {
    for source in [
        "${ns}::setdef",
        "[namespace current]::setdef",
        "puts {unfinished",
        "# setdef",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl");
        let offset = u32::try_from(source.len() - 2).unwrap();
        assert!(
            original_complete_cursor_word(source, &analysis, offset)
                .and_then(|word| word.static_unicode_value())
                .is_none(),
            "{source}"
        );
    }
    let source = "puts {café}";
    let mut analysis = Analyser::new().analyse(source, "tcl");
    let offset = u32::try_from(source.find("café").unwrap()).unwrap();
    let old = analysis.body_lexer_config.unwrap().strict_quoting;
    analysis.body_lexer_config.as_mut().unwrap().strict_quoting = !old;
    assert!(original_complete_cursor_word(source, &analysis, offset).is_none());
}
