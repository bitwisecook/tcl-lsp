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
fn original_clause_elements_keep_parent_geometry_and_nested_bom_content() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Original Document/list geometry only; the public String completeness
    // observations supply no list value, header or handler receipt.
    let source = "\u{feff}switch -- $v {\u{feff}$pattern {puts ok} plain {puts tail}}";
    let environment = tcl_dialect::DialectProfile::find("tcl9.1").unwrap();
    for bom in [tcl_lexer::LeadingBom::Skip, tcl_lexer::LeadingBom::Content] {
        let config = LexerConfig {
            leading_bom: bom,
            braced_var: tcl_lexer::BracedVarStyle::FirstClose,
            ..LexerConfig::for_file_grammar(environment.grammar)
        };
        let commands = segment_commands_with_offset_and_config(source, 0, config);
        let [command] = commands.as_slice() else {
            panic!("actual original whole command")
        };
        let body = command.argv[3];
        let elements =
            flatten_clause_list_elements_with_config(source, &command.texts[3], body, config);
        assert_eq!(
            elements
                .iter()
                .map(|(text, _)| text.as_str())
                .collect::<Vec<_>>(),
            ["\u{feff}$pattern", "puts ok", "plain", "puts tail"]
        );
        assert_eq!(elements[0].1.kind, TokenType::Esc);
        assert_eq!(elements[1].1.kind, TokenType::Str);
        let first = elements[0].1.span;
        assert_eq!(source.get(first.as_range()), Some("\u{feff}$pattern"));
        assert_eq!(
            first.start(),
            body.span.start() + u32::from(body.content_offset)
        );
        assert_eq!(elements[1].1.content_offset, 1);
        assert!(
            elements
                .iter()
                .all(|(_, token)| body.span.start() <= token.span.start()
                    && token.span.end() <= body.span.end())
        );
        let elements_with_content = flatten_clause_list_elements_with_config(
            source,
            &command.texts[3],
            body,
            LexerConfig {
                leading_bom: tcl_lexer::LeadingBom::Content,
                ..config
            },
        );
        assert_eq!(elements, elements_with_content);
    }
}
