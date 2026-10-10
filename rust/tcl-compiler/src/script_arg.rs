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

//! Structural source assistance for scripts built with a list-prefix command.
//!
//! A single command substitution such as `[list upvar 1 a b]` can retain its
//! source word layout for compatibility consumers. The registry selects the
//! builder's `BUILDS_COMMAND_PREFIX` trait; a complete literal bare head and
//! one inner command are required. Dynamic heads and multiple commands decline.
//!
//! This projection supplies source spans and syntax. It does not establish an
//! entered script, an effective Native argument vector, command selection or
//! a frame. Original callback and script-body consumers use their independently
//! selected typed issuers for those questions. The enclosing command's selected
//! argument roles determine whether a value occupies a script position.
//!
//! Builder arguments undergo substitution in the building frame. Their source
//! spans describe the expressions that produce list elements; they do not turn
//! unknown values into literals or move variable reads into a later frame.
//!
//! [`Traits::BUILDS_COMMAND_PREFIX`]: tcl_registry::Traits::BUILDS_COMMAND_PREFIX

use tcl_lexer::{Span, Token, TokenType};
use tcl_registry::CommandRegistry;

use crate::segmenter::SegmentedCommand;

/// Project the source layout of a single literal-head list-built script.
///
/// `tok` and `text` retain the complete command-substitution word. The returned
/// segment drops the builder head and preserves the original producer spans.
/// This compatibility projection supplies neither Native dispatch nor entered
/// script execution; callers retain the enclosing selected argument role.
#[must_use]
pub fn list_quoted_script_command(
    registry: &CommandRegistry,
    tok: Token,
    text: &str,
) -> Option<SegmentedCommand> {
    if tok.kind != TokenType::Cmd {
        return None;
    }
    let seg =
        crate::signature_scan::command_prefix::list_quoted_command_segment(registry, tok, text)?;
    drop_leading_word(&seg)
}

/// Project only the written target geometry from the sealed original builder
/// and its selected source target. Captured alias arguments cannot borrow
/// written spans. This supplies no execution, value object or entered frame.
pub(crate) fn original_list_built_script_command(
    words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
    producer: &SegmentedCommand,
) -> Option<SegmentedCommand> {
    use crate::registry_invocation::InvocationWordOrigin;
    use crate::registry_invocation::source_structure::OriginalRegistrySource;
    let OriginalRegistrySource::ProducedPrefix(prefix) = words.source() else {
        return None;
    };
    let originals = prefix.producer().original_words();
    let head = originals.first()?;
    let image = head.image();
    let config = head.config();
    let tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(image),
        config,
        producer,
    );
    let captured = crate::registry_invocation::original_native_compiler_words(
        image,
        tokens.words(),
        producer.argv.first()?.span.start(),
        config,
    )?;
    if captured != originals
        || words.origins()
            != (1..originals.len())
                .map(InvocationWordOrigin::Written)
                .collect::<Vec<_>>()
    {
        return None;
    }
    drop_leading_word(producer)
}

/// [`list_quoted_script_command`] for a `list HEAD word …` call that is
/// **already segmented** — the shape the semantic-token walker meets, since
/// it recurses into every `[…]` and segments its content before classifying
/// the words.
///
/// The caller owns the "is this position actually a script?" question:
/// `list` only ever returns a value, so `set data [list upvar 1 a b]` builds
/// no command at all.  Answering that needs the *enclosing* argument's role,
/// which this function cannot see.
#[must_use]
pub fn list_build_effective_command(
    registry: &CommandRegistry,
    seg: &SegmentedCommand,
) -> Option<SegmentedCommand> {
    crate::signature_scan::command_prefix::list_build_is_literal(registry, seg)
        .then(|| drop_leading_word(seg))
        .flatten()
}

/// Drop the builder head while retaining the remaining source geometry.
fn drop_leading_word(seg: &SegmentedCommand) -> Option<SegmentedCommand> {
    let head = *seg.argv.get(1)?;
    let start = head.span.start();
    Some(SegmentedCommand {
        span: Span::new(start, seg.span.end().max(start)),
        argv: seg.argv.get(1..)?.to_vec(),
        texts: seg.texts.get(1..)?.to_vec(),
        word_fragments: seg.word_fragments.get(1..)?.to_vec(),
        single_token_word: seg.single_token_word.get(1..)?.to_vec(),
        // Everything from the new head onwards; the dropped `list` word's own
        // tokens must not stay, or a `$`-token walk would credit them twice.
        all_tokens: seg
            .all_tokens
            .iter()
            .filter(|t| t.span.start() >= start)
            .copied()
            .collect(),
        is_partial: seg.is_partial,
        partial_delimiter: seg.partial_delimiter,
        // `{*}` markers are per-word and the leading word is gone; a list
        // build never carries one anyway (it would make the word count
        // unknowable, and `list_quoted_command_segment` requires a literal
        // head).
        expand_word: None,
        preceding_comment: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segmenter::segment_commands;

    fn reg() -> &'static CommandRegistry {
        tcl_registry::model::ingress::static_context_for("tcl8.6").commands()
    }

    /// Segment `src` as one command and resolve its argument `idx` (1-based
    /// over the arguments) as a list-quoted script.
    fn resolve(src: &str, idx: usize) -> Option<SegmentedCommand> {
        let cmds = segment_commands(src);
        let seg = cmds.first()?;
        list_quoted_script_command(reg(), *seg.argv.get(idx)?, seg.texts.get(idx)?)
    }

    #[test]
    fn tp_resolves_a_literal_list_build_to_its_effective_command() {
        let cmd = resolve("uplevel #0 [list upvar #0 ::tk::Priv ::tk::Priv]", 2)
            .expect("a literal `[list upvar …]` resolves");
        assert_eq!(
            cmd.texts,
            vec!["upvar", "#0", "::tk::Priv", "::tk::Priv"],
            "the `list` word is dropped and the rest keep their order"
        );
        // Spans still point at the user's own text.
        let src = "uplevel #0 [list upvar #0 ::tk::Priv ::tk::Priv]";
        assert_eq!(
            &src[cmd.argv[0].span.start() as usize..cmd.argv[0].span.end() as usize],
            "upvar"
        );
    }

    #[test]
    fn tn_a_dynamic_head_stays_unresolved() {
        assert!(
            resolve("uplevel #0 [list $cb ::tk::Priv]", 2).is_none(),
            "`[list $cb …]` names no statically known command"
        );
        assert!(
            resolve("uplevel #0 [$build ::tk::Priv]", 2).is_none(),
            "a computed substitution is not a list build"
        );
    }

    #[test]
    fn tn_a_braced_body_is_not_this_shape() {
        assert!(
            resolve("uplevel #0 {upvar #0 a b}", 2).is_none(),
            "a literal braced body is already walked as a script; this \
             predicate answers only for the built shape"
        );
    }

    #[test]
    fn tn_a_multi_command_substitution_stays_unresolved() {
        assert!(
            resolve("uplevel #0 [list a; list b]", 2).is_none(),
            "two commands in one substitution: the value is the *last* one's \
             result, so the shape is not a single list build"
        );
    }

    #[test]
    fn tn_a_non_list_builder_stays_unresolved() {
        assert!(
            resolve("uplevel #0 [concat upvar #0 a b]", 2).is_none(),
            "`concat` does not carry BUILDS_COMMAND_PREFIX — its result is \
             not a well-formed one-command list"
        );
    }
    #[test]
    fn original_list_builder_projection_preserves_selected_producers_and_refuses_shadows() {
        // naming.source.original-produced-command-prefix
        // docs/design/analysis/name-resolution-proofs/original-produced-command-prefix.md
        for (source, expected) in [
            ("uplevel #0 [list upvar #0 original local]", true),
            (
                "rename list build; uplevel #0 [build upvar #0 original local]",
                true,
            ),
            (
                "interp alias {} build {} list; uplevel #0 [build upvar #0 original local]",
                true,
            ),
            (
                "proc list args {}; uplevel #0 [list upvar #0 original local]",
                false,
            ),
            (
                "rename list {}; uplevel #0 [list upvar #0 original local]",
                false,
            ),
        ] {
            let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
            let config = analysis.resolved_input.as_ref().unwrap().lexer_config();
            let parent =
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
                    .pop()
                    .unwrap();
            let body = parent.argv[2];
            let sm = tcl_lexer::SourceMap::new(source);
            let child = crate::parsing::syntax::descend::descend_token(&sm, body, config);
            let commands = crate::parsing::syntax::segment::segments_from_tree(child.tree(), &sm);
            let producer = &commands[0];
            let words =
                crate::registry_invocation::source_structure::source_produced_command_prefix_words(
                    source, &analysis, producer,
                );
            let projected = words
                .as_ref()
                .and_then(|words| original_list_built_script_command(words, producer));
            assert_eq!(projected.is_some(), expected, "{source}");
            if let Some(projected) = projected {
                assert_eq!(projected.texts, ["upvar", "#0", "original", "local"]);
                assert_eq!(projected.argv, producer.argv[1..]);
                let input = analysis.resolved_input.as_ref().unwrap();
                let realm = analysis.retained_command_realm().unwrap();
                assert_eq!(words, crate::registry_invocation::source_structure::source_produced_command_prefix_words_in(source, input, realm, producer));
                assert!(crate::registry_invocation::source_structure::source_produced_command_prefix_words_in(&format!("{source} "), input, realm, producer).is_none());
            }
        }
    }
}
