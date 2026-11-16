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

//! Incremental content changes applied to an open buffer.

use super::DocumentPublication;
use super::path_admission::DocumentPathAdmission;
use crate::{Arc, Backend, DocumentState, Range, change_could_affect_dialect_hint};

impl Backend {
    pub(in super::super) fn apply_document_content_changes(
        entry: &mut DocumentState,
        changes: &[tower_lsp_server::ls_types::TextDocumentContentChangeEvent],
        path_admission: &Arc<DocumentPathAdmission>,
        change_version: i32,
    ) -> (Arc<str>, String, String, u64, bool) {
        if entry.path_admission.is_none() {
            entry.path_admission = Some(Arc::clone(path_admission));
        }
        // Build-and-swap, not mutate-in-place: each splice produces a fresh
        // buffer and the shared handle is replaced wholesale at the end, so
        // any snapshot already handed to an in-flight request keeps the
        // exact revision it was taken at.
        let mut text: Arc<str> = Arc::clone(&entry.text);
        // Patch the persisted `LineIndex` alongside each splice instead of
        // rebuilding it per edit / per position lookup.
        // Take it out, patch through the edit sequence, put it back.
        let mut index = std::mem::replace(&mut entry.line_index, tcl_lexer::LineIndex::new(""));
        let mut has_bare_cr = entry.has_bare_cr;
        // Only a generically-opened `tcl` buffer re-resolves the in-source
        // dialect hint below (see the tail block), so only that case is
        // worth watching the edits for a hint-touching change.
        let watch_dialect_hint = entry.language_id == "tcl";
        let mut dialect_hint_may_have_changed = false;
        for change in changes {
            if watch_dialect_hint
                && !dialect_hint_may_have_changed
                && change_could_affect_dialect_hint(&text, change.range, &change.text, &index)
            {
                dialect_hint_may_have_changed = true;
            }
            text = apply_content_change_indexed(
                &text,
                change.range,
                &change.text,
                &mut index,
                &mut has_bare_cr,
            )
            .into();
        }
        // `text` and `line_index` are installed together — the snapshot
        // invariant on `DocumentState`: no reader may ever pair text from
        // this revision with an index from the previous one.
        let index_needs_seed = entry.publication != DocumentPublication::Indexed;
        entry.text = Arc::clone(&text);
        entry.line_index = index;
        entry.has_bare_cr = has_bare_cr;
        // The client supplied text for this revision, not the on-disk
        // bytes that established the old report. Do not carry an encoding
        // verdict across even a no-op-looking edit.
        entry.decode_report = None;
        entry.saved_source_bytes = None;
        if dialect_hint_may_have_changed {
            // Persist this across deferred publication. If a later edit
            // supersedes this handler, that authoritative handler must
            // still resolve the combined buffer's hint.
            entry.mark_dialect_resolution_dirty();
        }
        entry.bump_revision(change_version);
        let dialect = entry.dialect.clone();
        let language_id = entry.language_id.clone();
        let revision = entry.revision;
        (text, dialect, language_id, revision, index_needs_seed)
    }
}

/// Apply one LSP content change to `text` (incremental document sync).
///
/// A `None` range is a full-document replacement; a `Some` range is a
/// ranged edit whose `(line, character)` UTF-16 positions are resolved to
/// byte offsets via [`tcl_lexer::LineIndex::offset_at_utf16`] and spliced.
/// Offsets are clamped and ordered so the splice indices are always valid
/// char boundaries within `text`.
#[cfg(test)]
pub(in super::super) fn apply_content_change(
    text: &str,
    range: Option<Range>,
    new_text: &str,
) -> String {
    // Throwaway index + flag for the splice-behaviour tests; the live path
    // persists both through [`apply_content_change_indexed`].
    let mut index = tcl_lexer::LineIndex::new_lsp(text);
    let mut has_bare_cr = tcl_lexer::LineIndex::contains_bare_cr(text);
    apply_content_change_indexed(text, range, new_text, &mut index, &mut has_bare_cr)
}

/// Whether splicing `new_text` into `text[start..end)` could leave a bare
/// `\r` — a `\r` not immediately followed by `\n` — somewhere the edit
/// touches, given that `text` itself has none (checked by the caller via
/// [`tcl_lexer::LineIndex::contains_bare_cr`] before relying on this).
///
/// A change can only ever *introduce* a bare `\r` at the seam it touches: a
/// `\r` inside `new_text` itself, or a `\r` immediately before `start` that
/// `new_text` no longer pairs with a following `\n`. Nothing elsewhere in
/// `text` is affected, so this is a local, `O(new_text.len())` check — not a
/// document-wide rescan.
pub(in super::super) fn edit_introduces_bare_cr(
    text: &str,
    start: usize,
    end: usize,
    new_text: &str,
) -> bool {
    let bytes = text.as_bytes();
    let new_bytes = new_text.as_bytes();
    let stays_paired = |next: Option<u8>| next == Some(b'\n');
    if start > 0
        && bytes[start - 1] == b'\r'
        && !stays_paired(
            new_bytes
                .first()
                .copied()
                .or_else(|| bytes.get(end).copied()),
        )
    {
        return true;
    }
    new_bytes.iter().enumerate().any(|(i, &b)| {
        b == b'\r'
            && !stays_paired(
                new_bytes
                    .get(i + 1)
                    .copied()
                    .or_else(|| bytes.get(end).copied()),
            )
    })
}

/// [`apply_content_change`] that resolves the edit through a **persisted**
/// [`tcl_lexer::LineIndex`] built with the LSP end-of-line model
/// ([`tcl_lexer::LineIndex::new_lsp`]), keeping it (and `has_bare_cr`) in
/// lock-step with the returned text.
///
/// The index **must** use the LSP EOL model (`\n`, `\r\n`, *and lone `\r`*):
/// the client resolves the incoming `range` against that model, so a `\n`-only
/// index would resolve a bare-`\r` file's edit to the wrong byte offset and
/// corrupt the shadow buffer.
///
/// [`tcl_lexer::LineIndex::apply_edit`] only tracks `\n`-delimited breaks
/// (plus the LF half of a CRLF pair), so it patches an `new_lsp`-built index
/// exactly as a full [`tcl_lexer::LineIndex::new_lsp`] rebuild would **only**
/// while the document has no bare `\r` to disambiguate — the same condition
/// its own fuzz-equivalence test (`apply_edit_matches_rebuild_under_fuzz`,
/// whose edit alphabet is `\n`/`x` only) relies on. `*has_bare_cr` is the
/// document-level fast path (checked once per edit rather than rescanning);
/// [`edit_introduces_bare_cr`] additionally catches an edit that would be the
/// *first* to introduce one, so a document that opened clean stays on the
/// incremental path for exactly as long as it stays clean, and falls back —
/// correctly, not just fast — the moment it doesn't.
pub(in super::super) fn apply_content_change_indexed(
    text: &str,
    range: Option<Range>,
    new_text: &str,
    index: &mut tcl_lexer::LineIndex,
    has_bare_cr: &mut bool,
) -> String {
    let Some(range) = range else {
        *index = tcl_lexer::LineIndex::new_lsp(new_text);
        *has_bare_cr = tcl_lexer::LineIndex::contains_bare_cr(new_text);
        return new_text.to_owned();
    };
    let a = index.offset_at_utf16(
        range.start.line,
        tcl_lexer::Utf16Col::new(range.start.character),
        text,
    ) as usize;
    let b = index.offset_at_utf16(
        range.end.line,
        tcl_lexer::Utf16Col::new(range.end.character),
        text,
    ) as usize;
    let len = text.len();
    let start = a.min(b).min(len);
    let end = a.max(b).min(len);
    let mut out = String::with_capacity(len - (end - start) + new_text.len());
    out.push_str(&text[..start]);
    out.push_str(new_text);
    out.push_str(&text[end..]);
    if *has_bare_cr || edit_introduces_bare_cr(text, start, end, new_text) {
        *index = tcl_lexer::LineIndex::new_lsp(&out);
        *has_bare_cr = tcl_lexer::LineIndex::contains_bare_cr(&out);
    } else {
        index.apply_edit(
            u32::try_from(start).expect("splice start fits u32 (LineIndex's own 4 GiB budget)"),
            u32::try_from(end).expect("splice end fits u32 (LineIndex's own 4 GiB budget)"),
            new_text,
        );
    }
    out
}
