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

//! Comment tokens and UTF-16 token lengths.

use super::*;

#[test]
fn comment_inside_multiline_string_is_not_a_comment() {
    // A `#`-first line inside a multi-line `"…"` string is string text,
    // not a command comment — it must not emit a Comment token (which
    // would overlap the `$x` variable substitution).
    let src = "append s \"line1\n# not a comment $x\nline3\"\n";
    let toks = decode_full(src, tcl(), &reg());
    assert!(
        !toks
            .iter()
            .any(|(_, _, _, k, _)| *k == TokenKind::Comment as u32),
        "a `#` inside a string must not be a comment; got {toks:?}"
    );
    // A real comment still is one.
    let toks = decode_full("# real\nset x 1\n", tcl(), &reg());
    assert!(
        toks.iter()
            .any(|(_, _, _, k, _)| *k == TokenKind::Comment as u32),
        "expected a real comment token; got {toks:?}"
    );
}

#[test]
fn comments_classified_as_comment() {
    let s = full("# this is a comment\nset x 1\n", tcl(), &reg());
    // The first token should be the comment.
    assert_eq!(s.data[3], TokenKind::Comment as u32, "{:?}", s.data);
}

#[test]
fn semantic_token_lengths_use_utf16_code_units() {
    let data = full("# 😀x\n", tcl(), &reg()).data;
    assert_eq!(
        &data[..5],
        &[0, 0, 5, TokenKind::Comment as u32, 0],
        "comment token length must count the emoji as two UTF-16 code units",
    );
}

#[test]
fn many_comment_lines_do_not_drift_out_of_bounds() {
    // Hand-incrementing a byte cursor to the end of each comment line
    // while the `chars()` iterator advances one char drifts the cursor past
    // the buffer and slices out of bounds (panic) on files with several
    // comment lines.
    use std::fmt::Write as _;
    let mut src = String::new();
    for i in 0..40 {
        let _ = writeln!(src, "# comment line number {i} with some padding text");
    }
    src.push_str("set x 1\n");
    src.push_str("# trailing comment after code, no final newline");
    let st = full(&src, tcl(), &reg()); // must not panic
    let comments = st
        .data
        .chunks(5)
        .filter(|c| c[3] == TokenKind::Comment as u32)
        .count();
    assert_eq!(comments, 41, "expected one token per comment line");
}
