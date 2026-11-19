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

//! Component tables for `regsub` substitutions, glob patterns and regular expressions.

use super::*;

fn c_regex_hover(text: &str) -> String {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let components = crate::pattern_components::components(
        text,
        profile,
        Some(tcl_cmd_core::regex::RegexFlags::default()),
    )
    .unwrap();
    regex_hover_text(text, &components)
}

#[test]
fn regex_and_glob_hover_escape_table_pipes() {
    // A `|` inside a code-span table cell must be `\|` or it splits the GFM
    // row (issue 187).
    let rx = c_regex_hover("foo|bar");
    assert!(
        rx.lines().all(|l| !l.starts_with("| `|`")),
        "raw pipe token would break the row: {rx}"
    );
    assert!(
        rx.contains("\\|"),
        "alternation `|` should be escaped: {rx}"
    );
    // Glob alternation `{a,b}` has no pipe, but a bracketed pipe would; a
    // char-class-style pattern with `|` escapes too.
    let rxcc = c_regex_hover("[a|b]");
    assert!(rxcc.contains("\\|"));
    // Every non-separator row keeps a balanced cell count (3 pipes → 2
    // cells) after escaping.
    for line in rx.lines().filter(|l| l.starts_with("| ")) {
        let unescaped = line.replace("\\|", "");
        assert_eq!(
            unescaped.matches('|').count(),
            3,
            "row must have exactly two cells: {line}"
        );
    }
}

// regsub substitution-spec hover

#[test]
fn scan_regsub_backrefs_finds_each_backref() {
    let r = scan_regsub_backrefs(r"é & \1 \& \\1 \0");
    assert_eq!(r, [("&", 0), (r"\1", 1), (r"\0", 0)]);
}

#[test]
fn regsub_hover_renders_backref_table() {
    let text = regsub_hover_text("prefix \\1 suffix");
    assert!(text.contains("**Substitution spec**"), "{text}");
    assert!(text.contains("| `\\1` | Capture group 1 |"), "{text}");
}

#[test]
fn regsub_hover_handles_no_backrefs() {
    let text = regsub_hover_text("plain text");
    assert!(text.contains("No backreferences found"), "{text}");
}

#[test]
fn hover_fires_for_regsub_backref() {
    let src = "regsub foo bar {\\1-baz} out\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    let h = hover(src, 0, 18, &analysis, None).expect("hover");
    assert!(h.value.contains("Substitution spec"), "{}", h.value);
}

// glob pattern hover

#[test]
fn scan_glob_metachars_finds_star_and_question() {
    let m = scan_glob_metachars("*.tcl");
    let toks: Vec<&str> = m.iter().map(|(t, _)| t.as_str()).collect();
    assert!(toks.contains(&"*"), "{m:?}");
}

#[test]
fn scan_glob_metachars_finds_character_class() {
    let m = scan_glob_metachars("[abc]*.tcl");
    let toks: Vec<&str> = m.iter().map(|(t, _)| t.as_str()).collect();
    assert!(toks.contains(&"[abc]"), "{m:?}");
    assert!(toks.contains(&"*"), "{m:?}");
}

#[test]
fn glob_hover_renders_table() {
    let text = glob_hover_text("*.tcl");
    assert!(text.contains("**Glob pattern**"), "{text}");
    assert!(text.contains("| `*` |"), "{text}");
}

#[test]
fn glob_hover_for_literal_string() {
    let text = glob_hover_text("plain");
    assert!(text.contains("Literal string"), "{text}");
}

#[test]
fn hover_fires_for_glob_pattern() {
    // Braced glob pattern — single-line literal detection
    // requires `"..."` or `{...}` delimiters.  Bare globs
    // (`glob *.tcl`) fall through to the proc / word
    // lookup; their support lives in the same multi-line
    // / arg-position machinery that other `*-rich`
    // sub-strips defer.
    let src = "glob {*.tcl}\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    // Cursor inside the braced pattern.
    let h = hover(src, 0, 8, &analysis, None).expect("hover");
    assert!(h.value.contains("Glob pattern"), "{}", h.value);
}

// regex pattern hover

#[test]
fn regex_hover_renders_table() {
    let text = c_regex_hover("^foo$");
    assert!(text.contains("**Regex pattern**"), "{text}");
    assert!(text.contains("| `^` |"), "{text}");
}

#[test]
fn hover_fires_for_regex_pattern() {
    let src = "regexp {^foo.*$} $line\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    // Cursor inside the pattern literal.
    let h = hover(src, 0, 10, &analysis, None).expect("hover");
    assert!(h.value.contains("Regex pattern"), "{}", h.value);
}
