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

//! Whitespace geometry from complete original words under the selected grammar.

use std::collections::BTreeSet;

use tcl_core_types::RecursionLimit;
use tcl_lexer::{ExecutablePart, LexerConfig, NativeScriptWordsPlan, SourceImage, Span};

pub(super) fn words_in(
    image: &SourceImage,
    region: Span,
    config: LexerConfig,
) -> Option<NativeScriptWordsPlan> {
    let plan = tcl_lexer::native_script_words_in(image.clone(), region, config).ok()?;
    plan.fatal_tail.is_none().then_some(plan)
}

/// Clamp trimming to trivia outside every original whole word. A newline in a
/// nested command or quoted word remains protected by its containing word.
pub(super) fn trim_trailing_whitespace(text: &str, config: LexerConfig) -> String {
    // naming.editor.original-source-whitespace-geometry
    // docs/design/analysis/name-resolution-proofs/original-source-whitespace-geometry.md
    let image = SourceImage::document(text);
    let Ok(end) = u32::try_from(text.len()) else {
        return text.to_owned();
    };
    let Some(plan) = words_in(&image, Span::new(0, end), config) else {
        return text.to_owned();
    };
    let spans = plan
        .commands
        .iter()
        .flat_map(|command| command.words.iter().map(tcl_lexer::NativeWord::span))
        .collect::<Vec<_>>();
    let mut offset = 0;
    let mut output = String::with_capacity(text.len());
    for chunk in text.split_inclusive('\n') {
        let ending = if chunk.ends_with("\r\n") {
            "\r\n"
        } else if chunk.ends_with('\n') {
            "\n"
        } else {
            ""
        };
        let line = &chunk[..chunk.len() - ending.len()];
        let line_end = offset + line.len();
        let inside_word = spans
            .iter()
            .any(|span| span.start() as usize <= line_end && line_end < span.end() as usize);
        let keep = if inside_word {
            line.len()
        } else {
            let candidate = offset + line.trim_end().len();
            spans
                .iter()
                .filter(|span| {
                    (span.start() as usize) < line_end && (span.end() as usize) > candidate
                })
                .map(|span| (span.end() as usize).min(line_end))
                .max()
                .unwrap_or(candidate)
                .max(candidate)
                - offset
        };
        output.push_str(&line[..keep]);
        output.push_str(ending);
        offset += chunk.len();
    }
    output
}

/// Gaps between complete words from the shared root and bracket-script owners.
fn word_gaps(text: &str, config: LexerConfig, limit: RecursionLimit) -> Option<Vec<(Span, u32)>> {
    let image = SourceImage::document(text);
    let end = u32::try_from(text.len()).ok()?;
    let mut pending = vec![(Span::new(0, end), 0)];
    let mut visited = BTreeSet::new();
    let mut result = Vec::new();
    while let Some((region, depth)) = pending.pop() {
        if limit.exceeded(depth) {
            return None;
        }
        if !visited.insert((region.start(), region.end())) {
            continue;
        }
        let plan = words_in(&image, region, config)?;
        for command in &plan.commands {
            for pair in command.words.windows(2) {
                result.push((
                    Span::new(pair[0].span().end(), pair[1].span().start()),
                    depth,
                ));
            }
            for word in &command.words {
                for part in word.executable_parts().all_parts() {
                    if let ExecutablePart::Command { body } = part.part {
                        pending.push((body, depth.checked_add(1)?));
                    }
                }
            }
        }
    }
    result.sort_unstable_by_key(|(span, depth)| (span.start(), span.end(), *depth));
    result.dedup_by_key(|(span, _)| (span.start(), span.end()));
    Some(result)
}

/// Spaces between actual complete command words, including separately owned
/// bracket scripts. No literal, array index or quoted fragment is scanned again.
pub(super) fn separator_spaces(
    text: &str,
    config: LexerConfig,
    limit: RecursionLimit,
) -> Option<Vec<(usize, u32)>> {
    // naming.editor.original-source-whitespace-geometry
    // docs/design/analysis/name-resolution-proofs/original-source-whitespace-geometry.md
    let mut result = Vec::new();
    for (gap, depth) in word_gaps(text, config, limit)? {
        for (index, byte) in text.as_bytes().get(gap.as_range())?.iter().enumerate() {
            if *byte == b' ' {
                result.push((gap.start() as usize + index, depth));
            }
        }
    }
    Some(result)
}

/// Collapse only command separator continuations. Nested word data retains its
/// original spelling even when it contains a continuation of its own.
pub(super) fn collapse_script_separator_continuations(
    text: &str,
    config: LexerConfig,
    limit: RecursionLimit,
) -> Option<String> {
    // naming.editor.original-source-whitespace-geometry
    // docs/design/analysis/name-resolution-proofs/original-source-whitespace-geometry.md
    let gaps = word_gaps(text, config, limit)?;
    let mut output = String::with_capacity(text.len());
    let mut previous = 0;
    for (gap, _) in gaps {
        let start = gap.start() as usize;
        let end = gap.end() as usize;
        if start < previous {
            return None;
        }
        output.push_str(text.get(previous..start)?);
        output.push_str(
            &tcl_syntax::backslash::collapse_separator_continuations_str(text.get(start..end)?),
        );
        previous = end;
    }
    output.push_str(text.get(previous..)?);
    Some(output)
}

pub(super) fn selection_keeps_whole_words(plan: &NativeScriptWordsPlan, selection: Span) -> bool {
    plan.commands
        .iter()
        .flat_map(|command| &command.words)
        .all(|word| {
            let span = word.span();
            span.end() <= selection.start()
                || selection.end() <= span.start()
                || (selection.start() <= span.start() && span.end() <= selection.end())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trimming_retains_nested_quoted_data_and_bare_unicode_units() {
        // naming.editor.original-source-whitespace-geometry
        // docs/design/analysis/name-resolution-proofs/original-source-whitespace-geometry.md
        let config = LexerConfig::default();
        let source = "set x \"[set y \"inner   \ntext\"] outer\"   \nset y value\u{a0}   \n";
        assert_eq!(
            trim_trailing_whitespace(source, config),
            "set x \"[set y \"inner   \ntext\"] outer\"\nset y value\u{a0}\n"
        );
        let source = "set x {a   \r\nb}   \r\nset y \\uD800   \r\n";
        assert_eq!(
            trim_trailing_whitespace(source, config),
            "set x {a   \r\nb}\r\nset y \\uD800\r\n"
        );
        let malformed = "set x \"inner   \n";
        assert_eq!(trim_trailing_whitespace(malformed, config), malformed);
    }

    #[test]
    fn separators_belong_to_full_original_words_and_child_scripts() {
        // naming.editor.original-source-whitespace-geometry
        // docs/design/analysis/name-resolution-proofs/original-source-whitespace-geometry.md
        let source = "puts \"a [list {b c} d] e\" $array(a b) tail";
        let separators =
            separator_spaces(source, LexerConfig::default(), RecursionLimit(128)).unwrap();
        assert!(separators.contains(&(4, 0)));
        let nested = source.find("list ").unwrap() + 4;
        assert!(separators.contains(&(nested, 1)));
        for data in ["b c", "a b", "] e"] {
            let offset = source.find(data).unwrap() + data.find(' ').unwrap();
            assert!(
                !separators.iter().any(|&(index, _)| index == offset),
                "{data}"
            );
        }
        let braced_variable = "puts ${name with space} tail";
        let separators =
            separator_spaces(braced_variable, LexerConfig::default(), RecursionLimit(128)).unwrap();
        assert_eq!(separators, vec![(4, 0), (23, 0)]);
        assert!(
            separator_spaces(
                "puts {missing close",
                LexerConfig::default(),
                RecursionLimit(128)
            )
            .is_none()
        );
    }
}
