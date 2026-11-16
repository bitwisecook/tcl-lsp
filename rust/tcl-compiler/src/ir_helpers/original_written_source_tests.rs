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
fn original_recovered_written_words_keep_full_source_and_component_extents_separate() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let config = LexerConfig {
        braced_var: tcl_lexer::BracedVarStyle::FirstClose,
        ..LexerConfig::default()
    };
    let source = "list {} {é $x} \"[set é]$x\" prefix-${é} ${a{b} {*}[set é]";
    let words = tokenise_command_words(source, config);
    assert_eq!(words.len(), 1);
    assert_eq!(
        words[0]
            .iter()
            .map(|word| word.raw.as_str())
            .collect::<Vec<_>>(),
        vec![
            "list",
            "{}",
            "{é $x}",
            "\"[set é]$x\"",
            "prefix-${é}",
            "${a{b}",
            "{*}[set é]"
        ]
    );
    assert!(words[0].iter().all(CommandWord::has_source_geometry));
    let map = SourceMap::new(source);
    let command =
        crate::segmenter::segment_commands_with_offset_and_config(source, 0, config.nested())
            .remove(0);
    let original = CommandTokens::from_segmented(&map, config, &command);
    assert_eq!(map.text(original.words()[5].source().span), "${a{b");
    assert!(words[0][5].substituted);
    assert!(words[0][6].expanded);
}

#[test]
fn original_written_word_inventory_refuses_changed_cst_vectors_and_source_geometry() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    let config = LexerConfig::default();
    let source = "prefix; list {é $x}";
    let map = SourceMap::new(source);
    let command =
        crate::segmenter::segment_commands_with_offset_and_config(source, 0, config.nested())
            .remove(1);
    assert_eq!(command_words(&map, config, &command)[1].raw, "{é $x}");
    let mut changed = command.clone();
    changed.word_fragments[1][0].token.span = tcl_lexer::Span::new(0, 1);
    assert!(
        command_words(&map, config, &changed)
            .iter()
            .all(|word| word.literal().is_none())
    );
    changed = command.clone();
    changed.texts[1] = "FOREIGN".to_owned();
    assert!(
        command_words(&map, config, &changed)
            .iter()
            .all(|word| word.literal().is_none())
    );
    let missing = SourceMap::new("x");
    assert!(command_written_spans(&missing, config, &command).is_none());
    assert!(
        command_words(&missing, config, &command)
            .iter()
            .all(|word| word.literal().is_none())
    );
    changed = command.clone();
    changed.all_tokens[0].content_offset = u8::MAX;
    assert!(
        command_words(&map, config, &changed)
            .iter()
            .all(|word| word.literal().is_none())
    );
    assert_eq!(
        command_words(&map, config, &command)[0].literal(),
        Some("list")
    );
}
