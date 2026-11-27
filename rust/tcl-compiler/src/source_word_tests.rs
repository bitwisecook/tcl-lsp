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

use tcl_lexer::{LexerConfig, SourceMap, Span};
use tcl_source::words::*;
#[cfg(test)]
mod expression_source_tests {
    use super::*;

    fn original_word(source: &str) -> WordExpr {
        let config = LexerConfig::for_dialect("jim");
        let command =
            tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config)
                .remove(0);
        let fragments = command.word_fragments[1]
            .iter()
            .map(|part| part.token)
            .collect::<Vec<_>>();
        WordExpr::from_word(
            &SourceMap::new(source),
            config,
            &fragments,
            &command.texts[1],
            false,
            None,
        )
    }

    #[test]
    fn jim_bare_and_mixed_sugar_keep_the_original_closing_parenthesis() {
        assert!(
            matches!(original_word("list $(k)"), WordExpr::Opaque { text, .. } if text == "$(k)")
        );
        let WordExpr::Template { parts, .. } = original_word("list pre$(1+(2))") else {
            panic!("mixed original word must remain a template");
        };
        assert!(
            matches!(&parts[1], WordPart::Opaque { text, source } if text == "$(1+(2))" && source.span == Span::new(8, 15))
        );
        assert!(
            matches!(original_word("list \"$(1+2)\""), WordExpr::Template { parts, .. } if matches!(&parts[..], [WordPart::Opaque { text, .. }] if text == "$(1+2)"))
        );
    }

    fn component_or_wrapper_extent(source: &str, word: &WordExpr) -> Span {
        match word {
            WordExpr::BracedLiteral { source: site, .. }
            | WordExpr::CommandSubstitution { source: site, .. } => {
                tcl_lexer::word_span_at(source, site.span)
            }
            _ => word.source().span,
        }
    }

    #[test]
    fn original_word_wrappers_keep_full_delimiters_without_trailing_source() {
        let config = LexerConfig::default();
        for (source, expected) in [
            ("list [change] ; # after", "[change]"),
            ("list [] ; # after", "[]"),
            ("list pre[change] ; # after", "pre[change]"),
            ("list pre[] ; # after", "pre[]"),
            ("list \"pre[change]\" ; # after", "\"pre[change]\""),
            ("list \"\" ; # after", "\"\""),
            ("list {value} ; # after", "{value}"),
            ("list {} ; # after", "{}"),
            ("list {value\\}} ; # after", "{value\\}}"),
            ("lappend x {*}[change] ; # after", "{*}[change]"),
            ("lappend x {*}[] ; # after", "{*}[]"),
            ("lappend x {*}\"value\" ; # after", "{*}\"value\""),
            ("lappend x {*}{value} ; # after", "{*}{value}"),
            ("lappend x {*}{} ; # after", "{*}{}"),
        ] {
            let command =
                tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config)
                    .remove(0);
            let words =
                crate::ir::CommandTokens::from_segmented(&SourceMap::new(source), config, &command);
            let word = words.word_exprs.last().unwrap();
            let extent = component_or_wrapper_extent(source, word);
            assert_eq!(&source[extent.as_range()], expected, "{source}");
            if let WordExpr::Expand { word, .. } = word {
                assert_eq!(
                    &source[component_or_wrapper_extent(source, word).as_range()],
                    expected.strip_prefix("{*}").unwrap(),
                    "{source}",
                );
            }
        }
    }

    #[test]
    fn expanded_word_extent_retains_derived_source_coordinates() {
        let source = "lappend x {*}[change]";
        let base = 71;
        let config = LexerConfig::default();
        let command =
            tcl_source::segment::segment_commands_with_offset_and_config(source, base, config)
                .remove(0);
        let words = crate::ir::CommandTokens::from_segmented(
            &SourceMap::new(source).with_base(base, 0, 0),
            config,
            &command,
        );
        let word = words.word_exprs.last().unwrap();
        assert_eq!(word.source().span, Span::new(base + 10, base + 21));
        assert_eq!(
            &source[(word.source().span.start() - base) as usize
                ..(word.source().span.end() - base) as usize],
            "{*}[change]",
        );
    }

    #[test]
    fn command_substitution_sites_retain_the_exact_whole_component() {
        let config = LexerConfig::default();
        for base in [0, 71] {
            for (source, expected) in [
                ("list [puts $payload] ; # after", "[puts $payload]"),
                ("list [] ; # after", "[]"),
                (
                    "list \"prefix[puts $payload]suffix\" ; # after",
                    "[puts $payload]",
                ),
                (
                    "list pre[list [puts $payload]] ; # after",
                    "[list [puts $payload]]",
                ),
            ] {
                let command = tcl_source::segment::segment_commands_with_offset_and_config(
                    source, base, config,
                )
                .remove(0);
                let words = crate::ir::CommandTokens::from_segmented(
                    &SourceMap::new(source).with_base(base, 0, 0),
                    config,
                    &command,
                );
                let word = words.word_exprs.last().unwrap();
                let components = match word {
                    WordExpr::CommandSubstitution { spelling, source } => vec![(spelling, source)],
                    WordExpr::Template { parts, .. } => parts
                        .iter()
                        .filter_map(|part| {
                            if let WordPart::CommandSubstitution { spelling, source } = part {
                                Some((spelling, source))
                            } else {
                                None
                            }
                        })
                        .collect(),
                    _ => panic!("original command substitution must be retained"),
                };
                assert_eq!(components.len(), 1);
                let (spelling, site) = components[0];
                assert_eq!(spelling, expected);
                assert_eq!(site.provenance, tcl_source::words::Provenance::Source);
                assert_eq!(
                    &source[(site.span.start() - base) as usize..(site.span.end() - base) as usize],
                    expected
                );
            }
        }
    }

    #[test]
    fn typed_written_extents_do_not_borrow_an_enclosing_command_closer() {
        let source = "list [string tolower [puts $payload]] ; # after";
        let config = LexerConfig::default();
        let command =
            tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config)
                .remove(0);
        let words =
            crate::ir::CommandTokens::from_segmented(&SourceMap::new(source), config, &command);
        let outer = words.word_exprs.last().unwrap();
        let (spelling, site) = outer.sole_command_substitution().unwrap();
        let nested = crate::word_subst::nested_command_words(spelling, site, config).unwrap();
        let inner = nested.word_exprs.last().unwrap();
        let extent = inner.original_written_span(source).unwrap();
        assert_eq!(&source[extent.as_range()], "[puts $payload]");
        assert_eq!(source.as_bytes().get(extent.end() as usize), Some(&b']'));
        let replay_site = tcl_names::command::CommandAllocationSite {
            source: std::sync::Arc::new(tcl_names::origin::SourceOriginId::authored_image(
                tcl_lexer::SourceImage::document(source),
            )),
            offset: nested.word_exprs[0].source().span.start(),
        };
        assert_eq!(
            crate::registry_invocation::native_compiler_replay_source(&nested, &replay_site,)
                .as_deref(),
            Some("string tolower [puts $payload]")
        );
        assert!(
            inner
                .original_written_span(source.replace("puts", "Puts"))
                .is_none()
        );
        let mut withdrawn = inner.clone();
        let WordExpr::CommandSubstitution { source: site, .. } = &mut withdrawn else {
            panic!("nested command substitution");
        };
        site.provenance = tcl_source::words::Provenance::Opaque;
        assert!(withdrawn.original_written_span(source).is_none());
    }

    #[test]
    fn typed_written_extents_preserve_variable_and_literal_token_conventions() {
        for source in ["list ${payload}", "list {}", "list {payload}"] {
            let word = original_word(source);
            let before = word.source().span;
            let span = word.original_written_span(source).unwrap();
            assert_eq!(&source[span.as_range()], &source[5..]);
            assert_eq!(word.source().span, before);
        }
        let source = "list \"[puts $payload]\"";
        let word = original_word(source);
        assert_eq!(
            &source[word.original_written_span(source).unwrap().as_range()],
            "\"[puts $payload]\""
        );
        assert_eq!(
            &source[word
                .original_command_substitution_span(source)
                .unwrap()
                .as_range()],
            "[puts $payload]"
        );
    }
}

#[cfg(test)]
mod original_quoted_extent_tests {
    use super::*;

    #[test]
    fn original_quoted_templates_keep_the_whole_word_and_configured_projection() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let config = LexerConfig::for_dialect(dialect);
            for written in [
                r#""prefix ${value} suffix""#,
                r#""${value}""#,
                r#""prefix [list] suffix""#,
                r#""${a{b}c} suffix""#,
                r#""""#,
            ] {
                let source = format!("list {written} ; # after");
                for base in [0, 71] {
                    let command = tcl_source::segment::segment_commands_with_offset_and_config(
                        &source, base, config,
                    )
                    .remove(0);
                    let words = crate::ir::CommandTokens::from_segmented(
                        &SourceMap::new(&source).with_base(base, 0, 0),
                        config,
                        &command,
                    );
                    let word = words.word_exprs.last().unwrap();
                    assert_eq!(
                        word.source().span,
                        Span::new(base + 5, base + 5 + u32::try_from(written.len()).unwrap()),
                        "{dialect}: {source}"
                    );
                    if base == 0 {
                        let original = crate::registry_invocation::original_native_compiler_words(
                            &tcl_lexer::SourceImage::document(&source),
                            words.words(),
                            0,
                            config,
                        )
                        .expect("complete original quoted vector");
                        assert_eq!(original.last().unwrap().bytes(), written.as_bytes());
                    }
                }
            }
        }
    }

    #[test]
    fn expanded_quoted_templates_retain_both_original_wrappers() {
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let config = LexerConfig::for_dialect(dialect);
            let source = r#"list {*}"prefix ${value} suffix" ; # after"#;
            let command =
                tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config)
                    .remove(0);
            let words =
                crate::ir::CommandTokens::from_segmented(&SourceMap::new(source), config, &command);
            let word = words.word_exprs.last().unwrap();
            let WordExpr::Expand { word: inner, .. } = word else {
                panic!("original expansion wrapper");
            };
            assert_eq!(
                &source[word.source().span.as_range()],
                r#"{*}"prefix ${value} suffix""#
            );
            assert_eq!(
                &source[inner.source().span.as_range()],
                r#""prefix ${value} suffix""#
            );
            let original = crate::registry_invocation::original_native_compiler_words(
                &tcl_lexer::SourceImage::document(source),
                words.words(),
                0,
                config,
            )
            .expect("complete original expanded vector");
            let captured = original.last().unwrap();
            assert!(captured.group().expand);
            assert_eq!(captured.span(), word.source().span);
            assert_eq!(
                captured.written_bytes(),
                r#"{*}"prefix ${value} suffix""#.as_bytes()
            );
            assert_eq!(captured.bytes(), r#""prefix ${value} suffix""#.as_bytes());
        }
    }
}
