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
fn original_argument_slices_keep_bom_values_and_selected_variable_geometry() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Lexical argument values/counts only; no handler or physical argv grant.
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    for bom in [tcl_lexer::LeadingBom::Skip, tcl_lexer::LeadingBom::Content] {
        let config = tcl_lexer::LexerConfig {
            leading_bom: bom,
            strict_quoting: true,
            braced_var: tcl_lexer::BracedVarStyle::FirstClose,
            ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
        };
        let dialect = tcl_registry::InvocationDialect::of_profile(profile);
        for (source, expected) in [
            ("cmd \u{feff}one two", [Some("\u{feff}one"), Some("two")]),
            ("cmd \u{feff}one ${a{b}", [Some("\u{feff}one"), None]),
        ] {
            let parent =
                tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
            assert_eq!(parent.len(), 1);
            let tokens = &parent[0].argv[1..];
            let values = with_source_argument_words(source, tokens, config, dialect, |arguments| {
                (
                    arguments.exact_argv_len(),
                    [
                        arguments.literal_at(0).map(str::to_owned),
                        arguments.literal_at(1).map(str::to_owned),
                    ],
                )
            })
            .unwrap();
            assert_eq!(values.0, Some(2));
            assert_eq!(values.1, expected.map(|value| value.map(str::to_owned)));
            let plan = tcl_lexer::native_script_words_in(
                tcl_lexer::SourceImage::document(source),
                tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
                config,
            )
            .unwrap();
            assert!(
                plan.commands[0]
                    .words
                    .iter()
                    .all(|word| word.config() == config)
            );
        }
    }
}
