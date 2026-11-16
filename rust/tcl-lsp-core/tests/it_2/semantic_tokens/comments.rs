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

//! Comment tokens: continuation lines, line endings and comments after a semicolon.

use super::*;

#[test]
fn comment_is_one_token() {
    let t = decode("# hello world", "tcl8.6");
    assert_eq!(t[0].ttype, "comment");
}

#[test]
fn comment_with_namespace_qualifiers_stays_one_comment() {
    // `::` inside a comment must NOT split into namespace tokens.
    let source = "# TCP::collect / TCP::payload / TCP::release";
    let t = decode(source, "f5-irules");
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].ttype, "comment");
    assert_eq!(t[0].length as usize, source.chars().count());
}

#[test]
fn multiline_comment_header_all_comments() {
    let source =
        "# Flow:\n#   1. CLIENT_ACCEPTED -> TCP::collect\n#   2. CLIENT_DATA -> TCP::payload\n";
    let t = decode(source, "f5-irules");
    assert!(
        t.iter().all(|x| x.ttype == "comment"),
        "expected all comment tokens, got {t:?}"
    );
}

/// Set of line numbers carrying a `comment` token, for continuation tests.
fn comment_lines(source: &str, dialect: &str) -> std::collections::BTreeSet<u32> {
    decode(source, dialect)
        .into_iter()
        .filter(|t| t.ttype == "comment")
        .map(|t| t.line)
        .collect()
}

#[test]
fn comment_line_continuation_is_comment() {
    // A `#` comment whose line ends in an unescaped backslash continues onto
    // the next physical line, and that continuation is a comment too.
    let source = "# a comment \\\nstill a comment\nset x 1\n";
    let lines = comment_lines(source, "tcl8.6");
    assert!(lines.contains(&0) && lines.contains(&1), "{lines:?}");
    // Line 2 is real code, not swallowed by the comment.
    assert!(!lines.contains(&2), "{lines:?}");
    let t = decode(source, "tcl8.6");
    assert!(
        t.iter()
            .any(|x| x.line == 2 && x.ttype == "function" && x.length == 3),
        "expected `set` on line 2: {t:?}"
    );
    // The continuation line ("still a comment", 15 chars) is fully a comment.
    assert!(
        t.iter()
            .any(|x| x.line == 1 && x.ttype == "comment" && x.length == 15),
        "{t:?}"
    );
}

#[test]
fn comment_multiline_continuation_all_comments() {
    let source = "# level one \\\nlevel two \\\nlevel three\nset y 2\n";
    let lines = comment_lines(source, "tcl8.6");
    assert!(
        lines.contains(&0) && lines.contains(&1) && lines.contains(&2),
        "{lines:?}"
    );
    assert!(!lines.contains(&3), "{lines:?}");
}

#[test]
fn comment_escaped_backslash_does_not_continue() {
    // `# ... \\` ends in an escaped backslash (even run) — the comment stops and
    // the next line is ordinary code.
    let source = "# escaped end \\\\\nset z 3\n";
    let lines = comment_lines(source, "tcl8.6");
    assert!(lines.contains(&0), "{lines:?}");
    assert!(
        !lines.contains(&1),
        "escaped backslash must not continue: {lines:?}"
    );
    let t = decode(source, "tcl8.6");
    assert!(
        t.iter().any(|x| x.line == 1 && x.ttype == "function"),
        "{t:?}"
    );
}

#[test]
fn comment_odd_backslash_run_continues() {
    // Three backslashes = one escaped pair + one continuation backslash.
    let source = "# three \\\\\\\nyes continued\nset a 4\n";
    let lines = comment_lines(source, "tcl8.6");
    assert!(lines.contains(&0) && lines.contains(&1), "{lines:?}");
    assert!(!lines.contains(&2), "{lines:?}");
}

#[test]
fn comment_continuation_crlf() {
    // CRLF line endings: the `\r` is stripped so the token stays within the
    // line's width, and the continuation is still recognised.
    let source = "# a comment \\\r\nstill a comment\r\nset x 1\r\n";
    let lines = comment_lines(source, "tcl8.6");
    assert!(lines.contains(&0) && lines.contains(&1), "{lines:?}");
    assert!(!lines.contains(&2), "{lines:?}");
    let t = decode(source, "tcl8.6");
    // No comment token may overrun its line (no trailing `\r` in the length).
    assert!(
        t.iter()
            .any(|x| x.line == 0 && x.ttype == "comment" && x.length == 13),
        "line 0 comment length excludes the `\\r`: {t:?}"
    );
    assert!(
        t.iter()
            .any(|x| x.line == 1 && x.ttype == "comment" && x.length == 15),
        "{t:?}"
    );
}

#[test]
fn comment_after_semicolon_is_a_comment() {
    // `#` is a comment at command position, which includes right after a `;`
    // command separator — `puts hi ;# tail`.
    let source = "puts hi ;# tail comment\n";
    let t = decode(source, "tcl8.6");
    assert!(
        t.iter().any(|x| x.line == 0 && x.ttype == "comment"),
        "expected a `;#` tail comment; got {t:?}"
    );
    // The `puts` command is still highlighted (the comment doesn't swallow it).
    assert!(t.iter().any(|x| x.ttype == "function"), "{t:?}");
}

#[test]
fn comment_after_semicolon_continues() {
    // A `;#` tail comment continues across a trailing backslash too.
    let source = "puts hi ;# tail \\\nmore\nset y 2\n";
    let lines = comment_lines(source, "tcl8.6");
    assert!(lines.contains(&0) && lines.contains(&1), "{lines:?}");
    assert!(!lines.contains(&2), "{lines:?}");
}

#[test]
fn semicolon_hash_inside_string_is_not_a_comment() {
    // A `;#` that falls inside a string literal is not a command-position
    // comment — the string's tokens cover it and it must be suppressed.
    let source = "set x \"a;#b\"\nputs $x\n";
    assert!(
        comment_lines(source, "tcl8.6").is_empty(),
        "a `;#` inside a string must not be a comment"
    );
}
