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

//! Token / source-range helpers for the optimiser.
//!
//! The full-command-range helper is a thin passthrough: the IR
//! statement `span` (produced by `lowering::structured`) already
//! covers the whole command, including trailing whitespace and
//! closing braces, so there is no need to re-run the lexer.
//!
//! Consumers: [`super::super::structure_elimination`] (body
//! extraction when replacing a dead if/while/for/switch with its
//! surviving branch).

use tcl_lexer::Span;

/// Extract a statically known script operand from its complete written-word
/// range under the grammar retained by lowering. Unknown substitution,
/// expansion, malformed words and invalid ranges decline the rewrite.
///
/// The shared word owner performs delimiter removal and escape processing.
/// Keep its evaluated bytes verbatim: trimming or re-indenting script text
/// changes nested braced data. A final newline prevents a body comment from
/// consuming the command following the removed wrapper.
#[must_use]
pub fn extract_body_text(
    source: &str,
    body_span: Span,
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    let raw = source.get(body_span.as_range())?;
    let commands = crate::segmenter::segment_commands_with_offset_and_config(raw, 0, config);
    let [command] = commands.as_slice() else {
        return None;
    };
    if command.is_partial || command.argv.len() != 1 {
        return None;
    }
    let tokens =
        crate::ir::CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(raw), config, command);
    let [word] = tokens.words() else { return None };
    let crate::registry_invocation::EffectiveInvocationWord::Literal(mut text) =
        crate::registry_invocation::effective_invocation_word(
            word,
            config.escapes,
            tcl_syntax::word_rules::WordValueRules::from_config(&config),
        )
    else {
        return None;
    };
    // An empty selected body returns the empty Tcl value; removing it can
    // expose the preceding command's result. The caller needs an explicit
    // result-use proof before this case can be rewritten.
    let body_commands = crate::segmenter::segment_commands_with_offset_and_config(&text, 0, config);
    if body_commands.is_empty() || body_commands.iter().any(|command| command.is_partial) {
        return None;
    }
    let body_map = tcl_lexer::SourceMap::new(&text);
    if body_commands.iter().any(|command| {
        crate::ir::CommandTokens::from_segmented(&body_map, config, command)
            .words()
            .iter()
            .any(|word| matches!(word, crate::ir::WordExpr::Opaque { .. }))
    }) {
        return None;
    }
    if !text.ends_with('\n') {
        text.push('\n');
    }
    Some(text)
}

/// Column (byte offset into its line) of `offset` within
/// `source`.
#[must_use]
pub fn column_of_offset(source: &str, offset: usize) -> usize {
    let clamped = offset.min(source.len());
    let prefix = &source[..clamped];
    match prefix.rfind('\n') {
        Some(nl) => clamped - (nl + 1),
        None => clamped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_simple_cases() {
        assert_eq!(column_of_offset("abcdef", 3), 3);
        assert_eq!(column_of_offset("abc\ndef", 4), 0);
        assert_eq!(column_of_offset("abc\ndef", 6), 2);
        assert_eq!(column_of_offset("ab\ncd\nef", 7), 1);
        // Offset past end clamps.
        assert_eq!(column_of_offset("abc", 100), 3);
    }

    fn extract(word: &str, config: tcl_lexer::LexerConfig) -> Option<String> {
        extract_body_text(
            word,
            Span::new(0, u32::try_from(word.len()).unwrap()),
            config,
        )
    }

    #[test]
    fn nested_braces_and_data_indentation_are_preserved() {
        let body = "{\n    set x {first\n        second}\n    puts $x\n}";
        assert_eq!(
            extract(body, tcl_lexer::LexerConfig::default()).unwrap(),
            "\n    set x {first\n        second}\n    puts $x\n"
        );
    }

    #[test]
    fn quoted_script_uses_evaluated_word_value() {
        assert_eq!(
            extract(r#""puts\x20hi""#, tcl_lexer::LexerConfig::default()),
            Some("puts hi\n".into())
        );
    }

    #[test]
    fn continuations_use_the_actual_dialect_grammar() {
        let word = "{puts\\\n  hi}";
        let tcl = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
        let jim = tcl_lexer::LexerConfig::for_dialect("jim");
        assert_eq!(extract(word, tcl), Some("puts hi\n".into()));
        assert_eq!(extract(word, jim), Some("puts\\\n  hi\n".into()));
    }

    #[test]
    fn final_comment_cannot_consume_following_outer_command() {
        assert_eq!(
            extract("{puts hi; # final}", tcl_lexer::LexerConfig::default()),
            Some("puts hi; # final\n".into())
        );
    }

    #[test]
    fn unknown_invalid_and_empty_words_decline() {
        let config = tcl_lexer::LexerConfig::default();
        for word in [
            "$body",
            "[getBody]",
            "\"puts $x\"",
            "{broken",
            "{}",
            "{  }",
            "{# comment only}",
            "{;\n;}",
            r#""set x \{""#,
            "two words",
        ] {
            assert_eq!(extract(word, config), None, "{word}");
        }
        assert_eq!(extract_body_text("é", Span::new(1, 2), config), None);
        assert_eq!(extract_body_text("hi", Span::new(5, 10), config), None);
    }
}
