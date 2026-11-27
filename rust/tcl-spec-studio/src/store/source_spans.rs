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

//! The document's statements as byte ranges: the pack body, command and subcommand statements, the source index and the splice helpers.

use crate::render_spectcl;
use std::collections::{BTreeMap, BTreeSet};
use tcl_lexer::{SourceMap, SourceSegmentationPurpose};
use tcl_source::cst::build::build_document;
use tcl_source::cst::segment::segments_from_document;
use tcl_spectcl::export::Registration;

// The CST side: finding a statement's bytes

/// The byte range of the `speclib` body's **contents** — everything between
/// its braces.
///
/// `None` when the document has no `speclib` statement with a braced body,
/// which is the "not a pack yet" case every caller has to handle anyway.
#[must_use]
pub fn pack_body(source: &str) -> Option<std::ops::Range<usize>> {
    for statement in segments(source, SourceSegmentationPurpose::WholeInput) {
        if statement.words.first().map(String::as_str) != Some("speclib") {
            continue;
        }
        // `speclib <name> <version> { … }` — the body is the fourth word.
        let word = statement.spans.get(3)?;
        return Some(word.inner.clone());
    }
    None
}

/// The byte range of the `command NAME …` statement inside the pack body.
///
/// The whole statement, from its first word to its closing brace — which is
/// what a splice replaces.
#[must_use]
pub fn command_span(source: &str, name: &str) -> Option<std::ops::Range<usize>> {
    find_command(source, pack_body(source), name).map(|(statement, _)| statement)
}

/// The `command NAME …` statement inside `region`, as `(whole statement, body
/// contents)`, both in `source`'s own byte offsets.
///
/// `region` is the pack body for a whole document, or the whole string for a
/// bare pack body such as a freshly rendered block — the two callers of this,
/// and the reason it takes the region rather than deriving it.
pub(super) fn find_command(
    source: &str,
    region: Option<std::ops::Range<usize>>,
    name: &str,
) -> Option<(std::ops::Range<usize>, std::ops::Range<usize>)> {
    let region = region?;
    let base = region.start;
    for statement in segments(&source[region], SourceSegmentationPurpose::NestedScript) {
        if statement.words.first().map(String::as_str) != Some("command") {
            continue;
        }
        if statement.words.get(1).map(String::as_str) != Some(name) {
            continue;
        }
        // `command NAME { … }`, or `command NAME -override { … }`: the body is
        // the last word, and it has to actually be a braced one.
        let word = statement.spans.last()?;
        if statement.words.len() < 3
            || source.as_bytes().get(base + word.outer.start) != Some(&b'{')
        {
            return None;
        }
        return Some((
            base + statement.span.start..base + statement.span.end,
            base + word.inner.start..base + word.inner.end,
        ));
    }
    None
}

/// The `subcommand NAME { … }` statement inside `region` — `command`'s body,
/// in the source it was found in — as `(whole statement, body contents)`,
/// mirroring [`find_command`] one scope down.
pub(super) fn find_subcommand(
    source: &str,
    region: std::ops::Range<usize>,
    name: &str,
) -> Option<(std::ops::Range<usize>, std::ops::Range<usize>)> {
    let base = region.start;
    for statement in segments(
        &source[region.clone()],
        SourceSegmentationPurpose::NestedScript,
    ) {
        if statement.words.first().map(String::as_str) != Some("subcommand") {
            continue;
        }
        if statement.words.get(1).map(String::as_str) != Some(name) {
            continue;
        }
        let word = statement.spans.last()?;
        if statement.words.len() < 3
            || source.as_bytes().get(base + word.outer.start) != Some(&b'{')
        {
            return None;
        }
        return Some((
            base + statement.span.start..base + statement.span.end,
            base + word.inner.start..base + word.inner.end,
        ));
    }
    None
}

/// [`PackStore::carry_forward`]'s reclaim, at one scope: which of
/// `old_statements`' first words vanished, or degraded to a `-native`
/// placeholder, between them and `block[block_region]` — and, when any did,
/// that text with the placeholders replaced by the author's own bytes for
/// those words, read out of `old_source` (the same slice `old_statements`'
/// spans are relative to). `None` when nothing needs reclaiming.
pub(super) fn reclaim(
    old_source: &str,
    old_statements: &[Segmented],
    block: &str,
    block_region: std::ops::Range<usize>,
) -> Option<String> {
    let written = segments(
        &block[block_region.clone()],
        SourceSegmentationPurpose::NestedScript,
    );
    let mut reclaimed: BTreeSet<&str> = BTreeSet::new();
    for statement in old_statements {
        let Some(word) = statement.words.first() else {
            continue;
        };
        let rendered = written.iter().find(|s| s.words.first() == Some(word));
        let lost = match rendered {
            None => true,
            Some(rendered) => is_native_placeholder(rendered) && !is_native_placeholder(statement),
        };
        if lost {
            reclaimed.insert(word.as_str());
        }
    }
    if reclaimed.is_empty() {
        return None;
    }

    // Drop the placeholders, back to front so the earlier spans stay valid.
    let mut body_text = block[block_region].to_owned();
    for statement in written.iter().rev() {
        if statement
            .words
            .first()
            .is_some_and(|word| reclaimed.contains(word.as_str()))
        {
            body_text.replace_range(trimmed_span(&body_text, statement.span.clone()), "");
        }
    }
    // Then append the author's own, verbatim.
    for statement in old_statements {
        if statement
            .words
            .first()
            .is_some_and(|word| reclaimed.contains(word.as_str()))
        {
            body_text.push_str("\n    ");
            body_text.push_str(old_source[statement.span.clone()].trim_end());
            body_text.push('\n');
        }
    }
    Some(body_text)
}

/// The literal command declaration ranges and top-level statements derived
/// from one exact store source.
///
/// Building this takes two CST passes: one over the document and one over its
/// `speclib` body.  Keeping the result on [`PackStore`] makes command lookup
/// constant-time and, critically, prevents [`PackStore::programmed`] from
/// repeating both passes once for every command in a pack.
pub(super) struct SourceIndex {
    pub(super) pack_body: Option<std::ops::Range<usize>>,
    pub(super) commands: BTreeMap<String, CommandLocation>,
    pub(super) document_statements: Vec<Segmented>,
}

pub(super) struct CommandLocation {
    pub(super) statement: std::ops::Range<usize>,
    pub(super) body: std::ops::Range<usize>,
}

impl SourceIndex {
    pub(super) fn new(source: &str) -> Self {
        let mut top_level = segments(source, SourceSegmentationPurpose::WholeInput);
        let pack_body = top_level
            .iter()
            .find(|statement| statement.words.first().map(String::as_str) == Some("speclib"))
            .and_then(|statement| statement.spans.get(3))
            .map(|word| word.inner.clone());

        let mut commands = BTreeMap::new();
        let mut body_statements = pack_body.clone().map_or_default(|body| {
            let base = body.start;
            segments(&source[body], SourceSegmentationPurpose::NestedScript)
                .into_iter()
                .map(|mut statement| {
                    if statement.words.first().map(String::as_str) == Some("command")
                        && statement.words.len() >= 3
                        && let (Some(name), Some(word)) =
                            (statement.words.get(1), statement.spans.last())
                        && source.as_bytes().get(base + word.outer.start) == Some(&b'{')
                    {
                        commands
                            .entry(name.clone())
                            .or_insert_with(|| CommandLocation {
                                statement: base + statement.span.start..base + statement.span.end,
                                body: base + word.inner.start..base + word.inner.end,
                            });
                    }
                    statement.inside_pack = true;
                    statement
                })
                .collect::<Vec<_>>()
        });

        let mut seen_pack = false;
        let mut document_statements = Vec::with_capacity(top_level.len() + body_statements.len());
        for mut statement in top_level.drain(..) {
            if !seen_pack && statement.words.first().map(String::as_str) == Some("speclib") {
                seen_pack = true;
                document_statements.append(&mut body_statements);
            } else {
                statement.inside_pack = false;
                document_statements.push(statement);
            }
        }

        Self {
            pack_body,
            commands,
            document_statements,
        }
    }

    pub(super) fn command(&self, name: &str) -> Option<&CommandLocation> {
        self.commands.get(name)
    }
}

/// One statement, with the byte ranges a splice needs.
pub(super) struct Segmented {
    pub(super) words: Vec<String>,
    pub(super) span: std::ops::Range<usize>,
    pub(super) spans: Vec<WordSpan>,
    /// Whether the statement is one the `speclib` block holds, rather than
    /// one the file states outside it. Set by
    /// [`PackStore::document_statements`]; `false` for a bare body read on
    /// its own, which is the shape every other caller reads.
    pub(super) inside_pack: bool,
}

/// Whether `statement` states, word for word, what `registration` recorded.
///
/// The [`PackStore::straight_line`] pairing reads the head and the first
/// argument, which is enough for a declaration inside the block: the loader
/// records that block's statements, so a computed one shows up as a count
/// that does not match. Outside the block there is no such backstop —
/// `default dialects [list tcl9.0]` registers `default dialects tcl9.0` and
/// pairs on both words it compares — so the rest of the words have to be
/// read. A word that was substituted differs from the value it evaluated to,
/// and the document is a program after all.
pub(super) fn stated_verbatim(statement: &Segmented, registration: &Registration) -> bool {
    statement
        .words
        .iter()
        .enumerate()
        .all(|(index, word)| word == registration.arg(index))
        && registration.arg(statement.words.len()).is_empty()
}

/// A word's byte ranges: `outer` includes any `{ }` / `" "` delimiters,
/// `inner` is the contents the loader reads as the word's text.
pub(super) struct WordSpan {
    outer: std::ops::Range<usize>,
    pub(super) inner: std::ops::Range<usize>,
}

/// Segment `source` into statements, the same way the loader does — same
/// lexer, same segmenter, so what this sees and what the loader reads are the
/// same statements.
pub(super) fn segments(source: &str, purpose: SourceSegmentationPurpose) -> Vec<Segmented> {
    let source_map = SourceMap::new(source);
    // dialect-drift-ok: `.tclspec` pack DSL text, not a document's Tcl — this
    // must read exactly what the SpecTcl loader reads, under its own grammar.
    let (document, _warnings) = build_document(
        source,
        tcl_spectcl::loader::pack_source_lexer_config(purpose),
    );
    segments_from_document(document, &source_map)
        .into_iter()
        .map(|segment| {
            let spans = segment
                .argv
                .iter()
                .map(|token| {
                    let start = token.span.start() as usize;
                    let end = token.span.end() as usize;
                    let lead = usize::from(token.content_offset);
                    let inner = if lead > 0 && end > start + lead {
                        start + lead..end - 1
                    } else {
                        start..end
                    };
                    WordSpan {
                        outer: start..end,
                        inner,
                    }
                })
                .collect();
            Segmented {
                words: segment.texts.clone(),
                span: segment.span.start() as usize..segment.span.end() as usize,
                spans,
                inside_pack: false,
            }
        })
        .collect()
}

/// Whether a statement is the renderer's `field -native ID` stand-in rather
/// than a real declaration.
///
/// A genuinely native hook is written that way too, which is why the *pair* of
/// before-and-after matters and not this predicate alone: a body degrading into
/// a placeholder is a loss, a placeholder staying a placeholder is not.
pub(super) fn is_native_placeholder(statement: &Segmented) -> bool {
    statement.words.get(1).map(String::as_str) == Some("-native")
}

/// Widen `span` to swallow the blank line a removed statement leaves behind.
pub(super) fn trimmed_span(source: &str, span: std::ops::Range<usize>) -> std::ops::Range<usize> {
    let mut end = span.end;
    let bytes = source.as_bytes();
    while end < bytes.len() && (bytes[end] == b'\n' || bytes[end] == b' ' || bytes[end] == b'\t') {
        end += 1;
    }
    let mut start = span.start;
    while start > 0 && (bytes[start - 1] == b' ' || bytes[start - 1] == b'\t') {
        start -= 1;
    }
    start..end
}

/// Re-write every `command <name> {` header in `text` whose name is in
/// `flagged` to carry `-override`.
///
/// The renderer has no spelling for `-override` — it renders a *draft*, and
/// the flag lives on the declaration rather than on `CommandSpec` — so this
/// is where a re-render puts it back. Header lines are matched whole and at
/// column 0, which is where the renderer puts them and where a command body's
/// own (four-space indented) content never is.
pub(super) fn with_override_flags(text: &str, flagged: &BTreeSet<&str>) -> String {
    if flagged.is_empty() {
        return text.to_owned();
    }
    let headers: BTreeMap<String, String> = flagged
        .iter()
        .map(|name| {
            let word = render_spectcl::header_word(name);
            (
                format!("command {word} {{"),
                format!("command {word} -override {{"),
            )
        })
        .collect();
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut out = String::with_capacity(text.len() + flagged.len() * 10);
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        match headers.get(line) {
            Some(replacement) if !seen.contains(line) => {
                seen.insert(line);
                out.push_str(replacement);
            }
            _ => out.push_str(line),
        }
    }
    out
}
