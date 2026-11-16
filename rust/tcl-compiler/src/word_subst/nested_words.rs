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

//! The structured words of a `[...]` command substitution and of a word that is exactly one.

use crate::ir::{CommandTokens, Provenance, SourceSite, WordExpr, WordPart};

/// Why the words of a `[…]` command substitution could not be recovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedWordsDecline {
    /// The spelling is not one complete, non-empty `[…]` command.
    Unmodelled,
    /// It segmented, but carried no words.
    NoWords,
}

/// The structured words of a `[…]` command substitution.
///
/// The word snapshot keeps a command substitution as one opaque spelling, so
/// its inner words are recovered by running the canonical segmenter over the
/// recorded lexical extent — the same segmentation the outer command's words
/// came from, not a bespoke parser. Anything other than exactly one complete
/// command declines.
///
/// The one owner for this: the native and WASM lowerings plan a nested
/// invocation from the same words this module lifts for analysis, and a second
/// recovery that split a word differently would let the two tiers disagree
/// about what a substitution runs.
///
/// # Errors
///
/// Returns [`NestedWordsDecline`] when the spelling is not a single complete
/// `[…]` command, or when it carries no words at all.
pub fn nested_command_words(
    spelling: &str,
    source: &SourceSite,
    config: tcl_lexer::LexerConfig,
) -> Result<CommandTokens, NestedWordsDecline> {
    // A pure function of its inputs that many consumers ask of the same word
    // in turn: re-segmenting a long substitution for each made it cost its
    // length per consumer (#2422).
    type Recent = (
        String,
        SourceSite,
        tcl_lexer::LexerConfig,
        Result<CommandTokens, NestedWordsDecline>,
    );
    const RECENT_WORDS: usize = 8;
    thread_local! {
        static RECENT: std::cell::RefCell<std::collections::VecDeque<Recent>> =
            const { std::cell::RefCell::new(std::collections::VecDeque::new()) };
    }
    let hit = RECENT.with_borrow_mut(|recent| {
        let at = recent.iter().position(|(text, site, at_config, _)| {
            *at_config == config && site == source && text == spelling
        })?;
        let entry = recent.remove(at)?;
        let answer = entry.3.clone();
        recent.push_front(entry);
        Some(answer)
    });
    if let Some(answer) = hit {
        return answer;
    }
    let answer = nested_command_words_uncached(spelling, source, config);
    RECENT.with_borrow_mut(|recent| {
        recent.truncate(RECENT_WORDS - 1);
        recent.push_front((spelling.to_owned(), source.clone(), config, answer.clone()));
    });
    answer
}

fn nested_command_words_uncached(
    spelling: &str,
    source: &SourceSite,
    config: tcl_lexer::LexerConfig,
) -> Result<CommandTokens, NestedWordsDecline> {
    // Word spellings arrive exact, but a substitution recovered from argument
    // text can carry the whitespace that separated it; the offset the trim
    // drops is added back so spans stay anchored where the word really sits.
    let leading = u32::try_from(spelling.len() - spelling.trim_start().len()).unwrap_or(0);
    let inner = spelling
        .trim()
        .strip_prefix('[')
        .and_then(|text| text.strip_suffix(']'))
        .ok_or(NestedWordsDecline::Unmodelled)?;
    if inner.trim().is_empty() {
        return Err(NestedWordsDecline::Unmodelled);
    }
    let base = if source.provenance == Provenance::Source {
        source
            .span
            .start()
            .saturating_add(leading)
            .saturating_add(1)
    } else {
        0
    };
    let config = config.nested();
    let segments = crate::segmenter::segment_commands_with_offset_and_config(inner, base, config);
    let [segment] = segments.as_slice() else {
        return Err(NestedWordsDecline::Unmodelled);
    };
    if segment.is_partial {
        return Err(NestedWordsDecline::Unmodelled);
    }
    // The nested script is a sub-lex: its own text, segmented at the document
    // offset it sits at, so the word model reads `inner` and still reports
    // document spans (`SourceMap::with_base` is that sub-lexing contract).
    let sm = tcl_lexer::SourceMap::new(inner).with_base(base, 0, 0);
    let mut tokens = CommandTokens::from_segmented(&sm, config, segment);
    if source.provenance != Provenance::Source {
        for word in &mut tokens.word_exprs {
            retain_nested_provenance(word, &source.provenance);
        }
    }
    if tokens.word_exprs.is_empty() {
        return Err(NestedWordsDecline::NoWords);
    }
    Ok(tokens)
}

fn retain_nested_provenance(word: &mut WordExpr, provenance: &Provenance) {
    let source = match word {
        WordExpr::Template { parts, source, .. } => {
            for part in parts {
                let site = match part {
                    WordPart::Text { source, .. }
                    | WordPart::Variable { source, .. }
                    | WordPart::CommandSubstitution { source, .. }
                    | WordPart::Opaque { source, .. } => source,
                };
                site.provenance.clone_from(provenance);
            }
            source
        }
        WordExpr::Expand { source, word } => {
            retain_nested_provenance(word, provenance);
            source
        }
        WordExpr::Literal { source, .. }
        | WordExpr::BracedLiteral { source, .. }
        | WordExpr::Variable { source, .. }
        | WordExpr::CommandSubstitution { source, .. }
        | WordExpr::Opaque { source, .. } => source,
    };
    source.provenance.clone_from(provenance);
}

/// Recover the structured words for a value word that consists solely of one
/// command substitution.
///
/// A bare `[cmd …]` and a quoted `"[cmd …]"` have different outer syntax but
/// both evaluate the same one command substitution.  Values with literal text,
/// variables, expansion, or a braced word deliberately decline: no caller may
/// infer a command's word form from their flattened value text.
#[must_use]
pub fn whole_word_command_tokens(
    word: &WordExpr,
    config: tcl_lexer::LexerConfig,
) -> Option<CommandTokens> {
    match word {
        WordExpr::CommandSubstitution { spelling, source } => {
            nested_command_words(spelling, source, config).ok()
        }
        WordExpr::Template { parts, .. } => {
            let mut substitution = None;
            for part in parts {
                match part {
                    WordPart::Text { text, .. } if text.is_empty() => {}
                    WordPart::CommandSubstitution { spelling, source }
                        if substitution.is_none() =>
                    {
                        substitution = Some((spelling, source));
                    }
                    _ => return None,
                }
            }
            let (spelling, source) = substitution?;
            nested_command_words(spelling, source, config).ok()
        }
        _ => None,
    }
}
