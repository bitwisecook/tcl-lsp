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
fn original_selected_commands_keep_the_parent_prologue_and_complete_vector_separate() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Original Document geometry and conditional source schema, not Native entry.
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    for (source, bom, offset, head) in [
        (
            "\u{feff}\u{feff}puts value",
            tcl_lexer::LeadingBom::Skip,
            3,
            "\u{feff}puts",
        ),
        (
            "\u{feff}puts value",
            tcl_lexer::LeadingBom::Content,
            0,
            "\u{feff}puts",
        ),
        ("\u{feff}puts value", tcl_lexer::LeadingBom::Skip, 3, "puts"),
        (
            "rename puts \u{feff}puts; \u{feff}puts value",
            tcl_lexer::LeadingBom::Skip,
            21,
            "\u{feff}puts",
        ),
    ] {
        let config = tcl_lexer::LexerConfig {
            leading_bom: bom,
            strict_quoting: true,
            braced_var: tcl_lexer::BracedVarStyle::FirstClose,
            ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
        };
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry(),
            config,
        );
        let analysis = crate::analyser::Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, profile.name);
        let realm = analysis.retained_command_realm().unwrap();
        let parent =
            tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
        let actual = parent.last().unwrap();
        assert_eq!(actual.argv[0].span.start(), offset, "{source:?}");
        let image = tcl_lexer::SourceImage::document(source);
        let tokens = realm
            .retained_original_tokens(&image, config, offset, &actual.argv)
            .unwrap();
        let (segment, native) = original_segment_for_tokens(source, &analysis, &tokens).unwrap();
        assert_eq!(segment.texts[0], head, "{source:?}");
        assert_eq!(segment.argv, actual.argv);
        assert_eq!(tokens.argv_texts[0], head);
        assert!(
            native
                .iter()
                .all(|word| word.image().bytes() == source.as_bytes() && word.config() == config)
        );
        assert_eq!(
            tokens
                .source_binding
                .as_ref()
                .and_then(|binding| binding.original_lexer_config_for_tokens(&tokens)),
            Some(config)
        );
        assert_eq!(analysis.resolved_input.as_ref(), Some(&input));
        if !head.starts_with('\u{feff}') || source.starts_with("rename ") {
            let selected = original_registry_words_for_tokens(source, &analysis, &tokens)
                .expect("actual original conditional handler keeps its full vector");
            assert_eq!(selected.head_source().unwrap().word(), native.first());
            assert!(selected.matches_source(&image, config));
        } else {
            assert!(original_registry_words_for_tokens(source, &analysis, &tokens).is_none());
        }
        assert!(original_segment_for_tokens(&format!("{source} "), &analysis, &tokens).is_none());
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(original_registry_words_for_tokens(source, &missing, &tokens).is_none());
        let mut changed = analysis.clone();
        changed.body_lexer_config = Some(tcl_lexer::LexerConfig {
            strict_quoting: !config.strict_quoting,
            ..config
        });
        assert!(original_segment_for_tokens(source, &changed, &tokens).is_none());
    }
}
