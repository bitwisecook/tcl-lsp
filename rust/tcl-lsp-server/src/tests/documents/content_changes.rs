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

//! Incremental content changes and the bare-CR check.

use super::*;

#[test]
fn apply_content_change_full_replacement() {
    // A `None` range replaces the whole document.
    assert_eq!(apply_content_change("old text", None, "new"), "new");
}

#[test]
fn apply_content_change_ranged_edits() {
    // Replace "hi" with "bye" on line 1.
    let text = "set x 1\nputs hi\n";
    let r = Range {
        start: pos(1, 5),
        end: pos(1, 7),
    };
    assert_eq!(
        apply_content_change(text, Some(r), "bye"),
        "set x 1\nputs bye\n"
    );

    // Pure insertion (empty range) at the start of line 1.
    let r = Range {
        start: pos(1, 0),
        end: pos(1, 0),
    };
    assert_eq!(
        apply_content_change(text, Some(r), "# "),
        "set x 1\n# puts hi\n",
    );

    // Multi-line deletion: drop from mid-line-0 through mid-line-1.
    let r = Range {
        start: pos(0, 4),
        end: pos(1, 5),
    };
    assert_eq!(apply_content_change(text, Some(r), ""), "set hi\n");
}

#[test]
fn apply_content_change_sequence_matches_full_text() {
    // Applying a sequence of ranged edits (as INCREMENTAL sync
    // delivers them) reproduces the intended final document.
    let mut text = "abc\ndef\n".to_string();
    // Edit 1: insert "X" at (0,1) -> "aXbc".
    text = apply_content_change(
        &text,
        Some(Range {
            start: pos(0, 1),
            end: pos(0, 1),
        }),
        "X",
    );
    assert_eq!(text, "aXbc\ndef\n");
    // Edit 2: replace "def" with "ghij".
    text = apply_content_change(
        &text,
        Some(Range {
            start: pos(1, 0),
            end: pos(1, 3),
        }),
        "ghij",
    );
    assert_eq!(text, "aXbc\nghij\n");
}

/// Live path: a persisted `LineIndex` driven through
/// `apply_content_change_indexed` over a sequence of ranged + full-replace
/// edits must stay byte-identical to a rebuild from the final text — the
/// invariant `DocumentState` relies on for its persisted index.
#[test]
fn apply_content_change_indexed_keeps_line_index_consistent() {
    let line_starts = |idx: &tcl_lexer::LineIndex| {
        (0..idx.line_count())
            .map(|l| idx.line_start(u32::try_from(l).unwrap_or(u32::MAX)))
            .collect::<Vec<_>>()
    };
    let mut text = "abc\ndef\nghi\n".to_string();
    let mut index = tcl_lexer::LineIndex::new_lsp(&text);
    let mut has_bare_cr = false;
    let edits = [
        (
            Some(Range {
                start: pos(0, 1),
                end: pos(0, 1),
            }),
            "X\nY",
        ), // insert with newline
        (
            Some(Range {
                start: pos(2, 0),
                end: pos(3, 2),
            }),
            "",
        ), // multi-line deletion
        (None, "p\nq\nr\ns"), // full replacement
        (
            Some(Range {
                start: pos(1, 1),
                end: pos(2, 0),
            }),
            "Z",
        ), // collapse a line
    ];
    // None of these edits ever puts a `\r` in the buffer, so every one of
    // them stays on the incremental `LineIndex::apply_edit` path — the
    // same `\r`-free alphabet `apply_edit_matches_rebuild_under_fuzz`
    // (tcl-lexer's own fuzz-equivalence gate) proves patched-vs-rebuilt
    // for. This test is the `did_change` integration of that guarantee.
    for (range, new_text) in edits {
        text = apply_content_change_indexed(&text, range, new_text, &mut index, &mut has_bare_cr);
        assert!(!has_bare_cr, "no edit here ever introduces a bare CR");
        assert_eq!(
            line_starts(&index),
            line_starts(&tcl_lexer::LineIndex::new_lsp(&text)),
            "persisted index diverged from rebuild after edit {new_text:?} -> {text:?}"
        );
    }
}

/// an incremental edit on an old-Mac (bare-`\r`) buffer must
/// resolve against the LSP EOL model so the splice lands at the right byte
/// and the shadow buffer stays correct. A bare-`\r` document is exactly the
/// case [`apply_content_change_indexed`] cannot patch incrementally (the
/// `\n`-only `LineIndex::apply_edit` has no way to place the extra `\r`
/// break), so it must fall back to a full rebuild here — asserted via
/// `has_bare_cr` staying `true`, not just via the result being correct
/// (both paths would produce the same index; only the fallback is sound).
#[test]
fn apply_content_change_indexed_handles_bare_cr_document() {
    // Client models "a\rb\rc" as 3 lines (0="a", 1="b", 2="c").
    let mut text = "a\rb\rc".to_string();
    let mut index = tcl_lexer::LineIndex::new_lsp(&text);
    let mut has_bare_cr = tcl_lexer::LineIndex::contains_bare_cr(&text);
    assert!(
        has_bare_cr,
        "the fixture must actually be a bare-CR document"
    );
    // Replace line 1 ("b") with "BB": range (1,0)..(1,1).
    text = apply_content_change_indexed(
        &text,
        Some(Range {
            start: pos(1, 0),
            end: pos(1, 1),
        }),
        "BB",
        &mut index,
        &mut has_bare_cr,
    );
    assert_eq!(text, "a\rBB\rc", "edit spliced at the wrong offset");
    assert!(has_bare_cr, "the document is still bare-CR after the edit");
    // The persisted index matches a fresh LSP rebuild.
    let rebuilt = tcl_lexer::LineIndex::new_lsp(&text);
    assert_eq!(index.line_count(), rebuilt.line_count());
    for l in 0..u32::try_from(index.line_count()).unwrap() {
        assert_eq!(index.line_start(l), rebuilt.line_start(l), "line {l}");
    }
}

/// An edit on a document already free of a bare CR takes the incremental
/// `LineIndex::apply_edit` path from then on, only falling back for the
/// *specific* edit that would introduce one — [`edit_introduces_bare_cr`]
/// is the decision function that draws that line.
#[test]
fn edit_introduces_bare_cr_detects_boundary_and_internal_lone_cr() {
    // A `\r` inside `new_text` with no following `\n` is bare.
    assert!(edit_introduces_bare_cr("abc", 1, 1, "\rX"));
    // `\r\n` inside `new_text` is a valid pair, not bare.
    assert!(!edit_introduces_bare_cr("abc", 1, 1, "\r\nX"));
    // A trailing `\r` in `new_text` that lands right before a pre-existing
    // `\n` (at `end`) is not bare — the pair straddles the edit boundary.
    assert!(!edit_introduces_bare_cr("a\nb", 1, 1, "\r"));
    // The same trailing `\r`, but nothing follows it, is bare.
    assert!(edit_introduces_bare_cr("ab", 1, 1, "\r"));
    // A `\r` already sitting just before `start`, still paired with the
    // `\n` that follows the edit, is not bare.
    assert!(!edit_introduces_bare_cr("a\r\nb", 2, 2, ""));
    // The same `\r`, once the edit removes the `\n` it was paired with,
    // becomes bare.
    assert!(edit_introduces_bare_cr("a\r\nb", 2, 3, ""));
    // No `\r` anywhere near the edit.
    assert!(!edit_introduces_bare_cr("abcdef", 2, 3, "XY"));
}
