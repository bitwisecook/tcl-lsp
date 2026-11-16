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

#[test]
fn literal_content_uses_shared_codec_without_opening_parent_or_substitutions() {
    let value = "$literal [command] {unterminated \"quote\"";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        let config = tcl_lexer::LexerConfig::for_dialect(dialect);
        let fragment =
            super::literal_quoted_source_fragment_in_braced_script(value, config).unwrap();
        assert!(fragment.contains("\\$"));
        assert!(fragment.contains("\\["));
        assert!(fragment.contains("\\{"));
        assert_eq!(super::decode_in(&fragment, config.escapes).as_ref(), value);
        let word = format!("\"{fragment}\"");
        assert!(crate::word_rules::lexical_unicode_source_word_matches(
            &format!("{{{word}}}"),
            &word,
            config
        ));
    }
}
