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

//! BIG-IP find-references — token-bounded textual search for a TMSH path.
//!
//! Resolve the `/Partition/Name` path-shaped token under the cursor, then
//! return every *identifier-bounded* occurrence of that exact token in the
//! source (definitions and references alike). Bare (non-`/`) tokens are not
//! renameable/searchable BIG-IP paths, so a cursor on one yields nothing.
//!
//! This is a single-document provider: it searches the open document only.
//! Cross-file references are a workspace-scanner concern the LSP does not yet
//! index; the same-file occurrences this returns are the dominant editor case.

use tcl_lexer::{LineIndex, Utf16Col};

use crate::graph::{GraphContext, build_objects_for_source};
use crate::range::Range;

/// Whether `b` is part of a BIG-IP path-shaped token (`[A-Za-z0-9_/.-]`).
/// The same character class the `rename_object` boundary rules use, so the
/// search set lines up with the renamer.
fn is_token_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'/' | b'.' | b'-')
}

/// The byte `[start, end)` span of the path-shaped token covering (or ending
/// at) `cursor_offset`, or `None` when the cursor is not on a token.
fn token_span_at(source: &str, cursor_offset: usize) -> Option<(usize, usize)> {
    let bytes = source.as_bytes();
    let n = bytes.len();
    let c = cursor_offset.min(n);
    // Expand left over token bytes (covers a cursor sitting just past the
    // token's last byte, e.g. on the trailing brace/space — the
    // `match.start() <= character <= match.end()` boundary is inclusive).
    let mut start = c;
    while start > 0 && is_token_byte(bytes[start - 1]) {
        start -= 1;
    }
    // Expand right over token bytes.
    let mut end = c;
    while end < n && is_token_byte(bytes[end]) {
        end += 1;
    }
    (start < end).then_some((start, end))
}

/// Token-bounded occurrences of `needle` in `source`: each match must have a
/// non-token byte (or document edge) on both sides, matching the
/// `(?<![A-Za-z0-9_/.-])needle(?![A-Za-z0-9_/.-])` pattern.
fn bounded_occurrences(source: &str, needle: &str) -> Vec<(usize, usize)> {
    let bytes = source.as_bytes();
    let n = bytes.len();
    let mut out = Vec::new();
    if needle.is_empty() {
        return out;
    }
    let mut from = 0;
    while let Some(rel) = source[from..].find(needle) {
        let start = from + rel;
        let end = start + needle.len();
        let before_ok = start == 0 || !is_token_byte(bytes[start - 1]);
        let after_ok = end == n || !is_token_byte(bytes[end]);
        if before_ok && after_ok {
            out.push((start, end));
        }
        // Non-overlapping scan (`finditer` semantics).
        from = end.max(start + 1);
    }
    out
}

/// Return every token-bounded occurrence of the BIG-IP path token at
/// `(line, character)` (UTF-16) as inclusive [`Range`]s, or an empty vector
/// when the cursor is not on a `/`-prefixed path token.
///
/// When `include_declaration` is `false`, occurrences that fall inside the
/// stanza of an object *named* by the token are dropped — the LSP
/// "references only" convention (the declaration of the object whose name
/// this is should not appear in its own reference list).
#[must_use]
pub fn references_at(
    source: &str,
    line: u32,
    character: u32,
    include_declaration: bool,
) -> Vec<Range> {
    if source.is_empty() {
        return Vec::new();
    }
    let line_index = LineIndex::new(source);
    let cursor = line_index.offset_at_utf16(line, Utf16Col::new(character), source) as usize;
    let Some((ts, te)) = token_span_at(source, cursor) else {
        return Vec::new();
    };
    let token = &source[ts..te];
    // Only TMSH paths (`/Partition/Name`) are searchable BIG-IP objects.
    if !token.starts_with('/') {
        return Vec::new();
    }

    let occurrences = bounded_occurrences(source, token);
    let decl_lines = if include_declaration {
        Vec::new()
    } else {
        declaration_line_spans(source, token)
    };

    occurrences
        .into_iter()
        .map(|(s, e)| Range::from_offsets(source, &line_index, s, e - 1))
        .filter(|r| {
            decl_lines
                .iter()
                .all(|&(ds, de)| !(ds <= r.start.line && r.start.line <= de))
        })
        .collect()
}

/// Inclusive `(start_line, end_line)` spans of every stanza that *declares*
/// an object whose identifier equals `token`. Used to drop the declaration
/// occurrences for "references only" requests.
fn declaration_line_spans(source: &str, token: &str) -> Vec<(u32, u32)> {
    let ctx = GraphContext::new();
    build_objects_for_source("", source, &ctx)
        .into_iter()
        .filter(|n| n.identifier == token)
        .map(|n| (n.range.start.line, n.range.end.line))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 0-based UTF-16 column of the first occurrence of `needle` in the
    /// `line`-th line — a small cursor-placement helper for the tests.
    fn col_of(source: &str, line: u32, needle: &str) -> u32 {
        let line_text = source.lines().nth(line as usize).unwrap();
        u32::try_from(line_text.find(needle).unwrap()).unwrap()
    }

    const CONF: &str = "ltm pool /Common/p {\n    members {\n        /Common/n:80 { }\n    }\n}\nltm virtual /Common/v {\n    pool /Common/p\n}\n";

    #[test]
    fn references_finds_definition_and_use() {
        // Cursor on `/Common/p` in the virtual's `pool /Common/p` line.
        let line = 6;
        let col = col_of(CONF, line, "/Common/p");
        let refs = references_at(CONF, line, col, true);
        // Two occurrences: the pool's stanza header (line 0) and the
        // virtual's `pool /Common/p` reference (line 6).
        let lines: Vec<u32> = refs.iter().map(|r| r.start.line).collect();
        assert!(lines.contains(&0), "pool decl on line 0 missing: {lines:?}");
        assert!(lines.contains(&6), "pool ref on line 6 missing: {lines:?}");
        assert_eq!(refs.len(), 2, "expected exactly 2 occurrences: {lines:?}");
    }

    #[test]
    fn references_excludes_declaration_when_requested() {
        let line = 6;
        let col = col_of(CONF, line, "/Common/p");
        let refs = references_at(CONF, line, col, false);
        let lines: Vec<u32> = refs.iter().map(|r| r.start.line).collect();
        assert!(
            !lines.contains(&0),
            "declaration (line 0) must be dropped: {lines:?}",
        );
        assert!(
            lines.contains(&6),
            "reference (line 6) must remain: {lines:?}"
        );
    }

    #[test]
    fn bare_token_is_not_a_path_reference() {
        // Cursor on `members` (a bare keyword, not a `/`-path) → no refs.
        let line = 1;
        let col = col_of(CONF, line, "members");
        assert!(references_at(CONF, line, col, true).is_empty());
    }

    #[test]
    fn token_is_identifier_bounded() {
        // `/Common/p` must not match inside `/Common/pool2`.
        let src = "ltm pool /Common/p {\n}\nltm pool /Common/pool2 {\n}\nltm virtual /Common/v {\n    pool /Common/p\n}\n";
        let line = 5;
        let col = col_of(src, line, "/Common/p");
        let refs = references_at(src, line, col, true);
        let lines: Vec<u32> = refs.iter().map(|r| r.start.line).collect();
        // Lines 0 (decl) and 5 (ref) only — NOT line 2 (`/Common/pool2`).
        assert!(lines.contains(&0) && lines.contains(&5), "{lines:?}");
        assert!(
            !lines.contains(&2),
            "must not match substring of pool2: {lines:?}"
        );
    }

    #[test]
    fn cursor_off_token_yields_nothing() {
        // Column far past the end of a short line.
        assert!(references_at(CONF, 4, 50, true).is_empty());
    }

    #[test]
    fn empty_source_is_safe() {
        assert!(references_at("", 0, 0, true).is_empty());
    }
}
