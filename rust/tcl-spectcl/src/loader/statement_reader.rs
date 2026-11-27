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

//! The statement reader: a pack source segmented into statements and words, with the byte-order mark and the `speclib` version span.

use tcl_lexer::{LexerConfig, TokenType};
use tcl_source::spectcl::pack_source_lexer_config;
use tcl_source::spectcl::pack_source_statements;

// The statement reader — CST in, words out

/// One word of a statement, with the text the CST resolved for it.
///
/// For a braced word `text` is **every byte between the braces**, verbatim:
/// the loader never reflows, dedents, or joins. That is what lets a ported
/// `description` compare byte for byte against the `&'static str` it came
/// from.
#[derive(Debug, Clone)]
pub(crate) struct Word {
    pub(crate) text: String,
    /// Whether the word was written `{…}` — the one thing that distinguishes
    /// an inline descriptor block from a descriptor named by a bare word.
    pub(crate) braced: bool,
    pub(crate) line: u32,
    pub(crate) local_source: Option<tcl_source::spectcl::LocalPackSourceWord>,
    pub(crate) original_source: Option<tcl_source::spectcl::OriginalPackSourceWord>,
}

/// One `word word…` declaration.
#[derive(Debug, Clone)]
pub(crate) struct Stmt {
    pub(crate) words: Vec<Word>,
    pub(crate) line: u32,
}

impl Stmt {
    pub(crate) fn word_text(&self, i: usize) -> &str {
        self.words.get(i).map_or("", |w| w.text.as_str())
    }

    pub(super) fn arg(&self, i: usize) -> Option<&Word> {
        self.words.get(i)
    }

    /// The words after the statement's own name word.
    pub(super) fn tail(&self) -> &[Word] {
        self.words.get(1..).unwrap_or(&[])
    }
}

/// What a U+FEFF at byte 0 of the buffer being segmented *is*.
///
/// A `.tclspec` is a file, and a byte-order mark at the head of a file is a
/// prologue rather than the first character of the first command word — which
/// is exactly the distinction `tcl_dialect::LexerGrammar::script_skips_leading_bom`
/// draws for Tcl 9's `source`. Nested blocks are *not* files:
/// a U+FEFF at the start of a `hover` summary is a character the author typed,
/// so only [`pack_statements`] and [`speclib_version_span`] may ask to skip it.
///
/// It is a parameter rather than a constant inside [`segment`] because
/// [`block`] re-enters the same function for every braced word, and flipping
/// the flag there would silently edit pack *content*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileBom {
    /// This buffer is a whole pack file: a leading mark is a prologue.
    Skip,
    /// This buffer is a nested block: a leading mark is data.
    Content,
}

impl FileBom {
    /// A discriminator for the memo key. Two segmentations of the same bytes
    /// under different dispositions are different answers, so they cannot
    /// share a cache slot.
    pub(crate) fn key(self) -> u8 {
        match self {
            FileBom::Skip => 1,
            FileBom::Content => 0,
        }
    }
}

/// Segment `source` into statements, numbering lines from `base_line`.
///
/// Comments, `;` separators, and line continuations are handled by the lexer,
/// so the loader inherits exactly the Tcl an author already knows — including
/// the trap that `#` only starts a comment where a command word would start.
///
/// Segmentation is a **pure function of `(source, base_line, bom)`**, which is
/// what lets [`crate::cache`] memoise it: every level of a pack — the file, the
/// `speclib` body, each `command` body, each descriptor block — reaches the
/// CST through this one door, so memoising here captures the whole tree
/// without the loader's readers knowing a cache exists. The memo is a no-op
/// unless a caller installed one.
///
/// `bom` joins the memo key as well as the signature, because it changes the
/// answer rather than merely the question. That is a correctness measure, not
/// a fix for a reachable bug — see [`crate::cache`]'s `Memo` for why the two
/// dispositions cannot currently meet in one memo.
pub(crate) fn statements(source: &str, base_line: u32, bom: FileBom) -> Vec<Stmt> {
    if let Some(mut hit) = crate::cache::memo_get(source, base_line, bom) {
        let config = source_config(bom);
        let count = hit.len();
        let owned = !hit.is_empty()
            && hit.iter().enumerate().all(|(row, stmt)| {
                !stmt.words.is_empty()
                    && stmt.words.iter().enumerate().all(|(ordinal, word)| {
                        word.original_source.is_none()
                            && word.local_source.as_ref().is_some_and(|original| {
                                original.matches_fragment(source, config, count, row, ordinal)
                                    && original.captured_text() == word.text
                                    && original.is_braced() == word.braced
                                    && original
                                        .local_line()
                                        .and_then(|line| base_line.checked_add(line))
                                        == Some(word.line)
                            })
                    })
            });
        if !owned {
            // Disk rows retain values only. Reborrow their entire actual CST
            // vector under this exact source/config; never trust stored spans.
            let parsed = segment(source, base_line, bom);
            let agrees = hit.len() == parsed.len()
                && hit.iter().zip(&parsed).all(|(a, b)| {
                    a.line == b.line
                        && a.words.len() == b.words.len()
                        && a.words.iter().zip(&b.words).all(|(a, b)| {
                            a.text == b.text && a.braced == b.braced && a.line == b.line
                        })
                });
            if !agrees {
                // The input digest names the request, not the serialized row
                // payload. Return and retain the actual current source parse.
                crate::cache::memo_put(source, base_line, bom, &parsed);
                return parsed;
            }
            for (row, stmt) in hit.iter_mut().enumerate() {
                for (ordinal, word) in stmt.words.iter_mut().enumerate() {
                    word.original_source = None;
                    word.local_source = agrees
                        .then(|| parsed[row].words[ordinal].local_source.clone())
                        .flatten();
                }
            }
            crate::cache::memo_retain_original_rows(source, base_line, bom, &hit);
        }
        return hit;
    }
    let parsed = segment(source, base_line, bom);
    crate::cache::memo_put(source, base_line, bom, &parsed);
    parsed
}

pub(super) fn source_config(bom: FileBom) -> LexerConfig {
    pack_source_lexer_config(match bom {
        FileBom::Skip => tcl_lexer::SourceSegmentationPurpose::WholeInput,
        FileBom::Content => tcl_lexer::SourceSegmentationPurpose::NestedScript,
    })
}

/// The uncached segmentation — [`statements`] minus the memo.
pub(super) fn segment(source: &str, base_line: u32, bom: FileBom) -> Vec<Stmt> {
    let parsed = tcl_source::spectcl::original_pack_source_statements(source, source_config(bom));
    let lines = tcl_lexer::LineIndex::new(source);
    let line_of = |offset: usize| -> u32 {
        base_line.saturating_add(lines.line_at(u32::try_from(offset).unwrap_or(u32::MAX)))
    };

    parsed
        .into_iter()
        .map(|original| {
            let segment = original.row();
            let words: Vec<Word> = segment
                .texts
                .iter()
                .zip(segment.argv.iter())
                .enumerate()
                .map(|(ordinal, (text, token))| Word {
                    text: text.clone(),
                    braced: token.kind == TokenType::Str && !token.in_quote,
                    line: line_of(token.span.start() as usize),
                    local_source: original.local_word(ordinal),
                    original_source: None,
                })
                .collect();
            let line = words.first().map_or(base_line, |w| w.line);
            Stmt { words, line }
        })
        .collect()
}

/// The statements inside a braced block word.
///
/// [`FileBom::Content`], always: a block is not a file, so a U+FEFF at the head
/// of a `hover` summary is a character the author typed and must survive into
/// the string the registry gets.
pub(crate) fn block(word: &Word) -> Vec<Stmt> {
    body_statements(&word.text, word.line, word.original_source.as_ref())
}

/// Segment a whole pack source — the loader's entry into [`statements`], and
/// the one [`crate::cache`] keys its on-disk entry from.
///
/// The one place [`FileBom::Skip`] is right: this is the file entry point, so a
/// leading byte-order mark is a prologue. A `.tclspec` saved by an editor that
/// writes "UTF-8 with BOM" otherwise loses its entire `speclib` declaration —
/// the first word decodes as `\u{feff}speclib`, which matches nothing.
pub(crate) fn pack_statements(source: &str) -> Vec<Stmt> {
    let mut rows = statements(source, 1, FileBom::Skip);
    for row in &mut rows {
        for word in &mut row.words {
            word.original_source = word
                .local_source
                .as_ref()
                .map(tcl_source::spectcl::LocalPackSourceWord::in_document);
        }
    }
    rows
}

pub(super) fn body_statements(
    source: &str,
    base_line: u32,
    parent: Option<&tcl_source::spectcl::OriginalPackSourceWord>,
) -> Vec<Stmt> {
    let mut rows = statements(source, base_line, FileBom::Content);
    for row in &mut rows {
        for word in &mut row.words {
            word.original_source =
                parent.and_then(|parent| word.local_source.as_ref()?.in_body(parent));
        }
    }
    rows
}

/// Locate the top-level `speclib` statement's version word: the byte range
/// of the word's *content* (inside any braces/quotes, so a rewrite keeps
/// the author's delimiters) and its decoded text.
///
/// This is the hook `tcl spec upgrade` rewrites through. It must read words
/// exactly the way the loader does — same lexer, same segmentation — so
/// `speclib demo {1.0} { … }` and `speclib demo "1.0" { … }` decode to
/// `1.0` here just as they do at load.
///
/// Same [`FileBom::Skip`] as [`pack_statements`], and for the same reason: this
/// reads a whole file. If it disagreed with the loader about whether a leading
/// mark is a prologue, `tcl spec upgrade` would compute its byte range against
/// a different tokenisation than the one the loader used, and rewrite the
/// wrong span.
#[must_use]
pub fn speclib_version_span(source: &str) -> Option<(std::ops::Range<usize>, String)> {
    for segment in pack_source_statements(
        source,
        pack_source_lexer_config(tcl_lexer::SourceSegmentationPurpose::WholeInput),
    ) {
        if segment.texts.first().map(String::as_str) != Some("speclib") {
            continue;
        }
        let text = segment.texts.get(2)?.clone();
        let token = segment.argv.get(2)?;
        // Word tokens exclude the closing delimiter from `span.end` and
        // carry the opening one via `content_offset`, so
        // `start + content_offset .. end` is exactly the content range for
        // wrapped and bare words alike.
        let start = token.span.start() as usize + usize::from(token.content_offset);
        return Some((start..token.span.end() as usize, text));
    }
    None
}
