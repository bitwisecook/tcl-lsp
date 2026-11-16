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

//! Sub-tokens inside regular expressions, `regsub` specs, `format` and `clock` specifications and backslash escapes.

use super::*;

const RE_TYPES: &[&str] = &[
    "regexp",
    "regexpAnchor",
    "regexpCharClass",
    "regexpQuantifier",
    "regexpGroup",
    "regexpEscape",
    "regexpBackref",
    "regexpAlternation",
];

pub(super) fn is_re_type(t: &str) -> bool {
    RE_TYPES.contains(&t)
}

#[test]
fn test_regexp_pattern_braced() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "regexp {^[a-z]+$} $str\n");
    let tokens = typed(&mut lsp, &lg, &uri);
    assert_eq!(tokens[0].ttype, "function");
    assert!(tokens.iter().any(|t| is_re_type(&t.ttype)));
    assert!(tokens.iter().any(|t| t.ttype == "variable"));
}

#[test]
fn test_regexp_pattern_bare() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "regexp foo $str\n");
    let tokens = typed(&mut lsp, &lg, &uri);
    assert_eq!(tokens.iter().filter(|t| t.ttype == "regexp").count(), 1);
}

#[test]
fn test_regexp_with_option_terminator() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "regexp -nocase -- {^test} $str\n");
    let tokens = typed(&mut lsp, &lg, &uri);
    assert!(tokens.iter().any(|t| is_re_type(&t.ttype)));
}

#[test]
fn test_regsub_pattern() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "regsub {\\d+} $str replacement result\n");
    let tokens = typed(&mut lsp, &lg, &uri);
    assert_eq!(tokens[0].ttype, "function");
    let cc: Vec<&TypedToken> = tokens
        .iter()
        .filter(|t| t.ttype == "regexpCharClass")
        .collect();
    assert!(cc.iter().any(|t| t.length == 2));
    assert!(tokens.iter().any(|t| t.ttype == "regexpQuantifier"));
}

#[test]
fn test_switch_regexp_braced_case_list() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(
        &mut lsp,
        "switch -regexp $x { {^a} {puts a} {^b} {puts b} }\n",
    );
    let tokens = typed(&mut lsp, &lg, &uri);
    assert!(tokens.iter().filter(|t| is_re_type(&t.ttype)).count() >= 2);
}

#[test]
fn test_switch_glob_no_regexp_tokens() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "switch -glob $x {a*} {puts a} {b*} {puts b}\n");
    let tokens = typed(&mut lsp, &lg, &uri);
    assert_eq!(tokens.iter().filter(|t| is_re_type(&t.ttype)).count(), 0);
}

#[test]
fn test_regsub_backref_highlighted() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "regsub {(\\w+)} $str {\\1 text} result\n");
    let nums: Vec<TypedToken> = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .filter(|t| t.ttype == "number")
        .collect();
    assert!(nums.iter().any(|t| t.length == 2));
}

#[test]
fn test_regsub_multiple_backrefs() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "regsub {(a)(b)} $str {\\2\\1} result\n");
    let nums: Vec<TypedToken> = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .filter(|t| t.ttype == "number" && t.length == 2)
        .collect();
    assert_eq!(nums.len(), 2);
}

#[test]
fn test_regex_source_variable_highlights_def_site_literal() {
    // End-to-end proof: a variable holding a regex literal that flows into a
    // `regexp` pattern makes the ORIGINATING `set` literal read as a regex.
    // This exercises the full salsa pipeline (semantic_tokens query demanding
    // the CompilationUnit), not just the token pass.
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    // The subject is bound: reading an unset `$s` raises before `regexp` runs.
    let uri = open_doc(
        &mut lsp,
        "set my_re \".*abc\"\nset s subject\nregexp $my_re $s\n",
    );
    let toks = typed(&mut lsp, &lg, &uri);
    // The `*` in the def-site literal (line 0) is a regex quantifier.
    assert!(
        toks.iter()
            .any(|t| t.line == 0 && t.ttype == "regexpQuantifier"),
        "expected a regexpQuantifier on the `set` literal; got {toks:?}"
    );
    // Sanity: a version with no such flow keeps the literal a plain string.
    let uri2 = open_doc(&mut lsp, "set my_re \".*abc\"\nputs $my_re\n");
    let toks2 = typed(&mut lsp, &lg, &uri2);
    assert!(
        !toks2
            .iter()
            .any(|t| t.line == 0 && t.ttype == "regexpQuantifier"),
        "a literal not used as a pattern must stay a string; got {toks2:?}"
    );
}

#[test]
fn test_backslash_n_highlighted() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "set x hello\\nworld\n");
    let esc: Vec<TypedToken> = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .filter(|t| t.ttype == "escape")
        .collect();
    assert_eq!(esc.len(), 1);
    assert_eq!(esc[0].length, 2);
}

#[test]
fn test_format_specifier_highlighted() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "format \"%s %d\" \"hello\" 123\n");
    let tokens = typed(&mut lsp, &lg, &uri);
    assert_eq!(
        tokens.iter().filter(|t| t.ttype == "formatPercent").count(),
        2
    );
    assert_eq!(tokens.iter().filter(|t| t.ttype == "formatSpec").count(), 2);
}

#[test]
fn test_format_flags_width_precision() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "format \"%-10.5f\" 3.14159\n");
    let tokens = typed(&mut lsp, &lg, &uri);
    assert_eq!(
        tokens.iter().filter(|t| t.ttype == "formatPercent").count(),
        1
    );
    assert_eq!(tokens.iter().filter(|t| t.ttype == "formatFlag").count(), 2);
    assert_eq!(tokens.iter().filter(|t| t.ttype == "formatSpec").count(), 1);
    assert_eq!(
        tokens.iter().filter(|t| t.ttype == "formatWidth").count(),
        2
    );
}

#[test]
fn test_clock_format_specifiers() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "clock format $t -format \"%Y-%m-%d\"\n");
    let tokens = typed(&mut lsp, &lg, &uri);
    assert_eq!(
        tokens.iter().filter(|t| t.ttype == "clockPercent").count(),
        3
    );
    assert_eq!(tokens.iter().filter(|t| t.ttype == "clockSpec").count(), 3);
}

#[test]
fn test_clock_format_locale_modifier() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "clock format $t -format \"%EY\"\n");
    let tokens = typed(&mut lsp, &lg, &uri);
    assert_eq!(
        tokens.iter().filter(|t| t.ttype == "clockPercent").count(),
        1
    );
    assert_eq!(
        tokens.iter().filter(|t| t.ttype == "clockModifier").count(),
        1
    );
    assert_eq!(tokens.iter().filter(|t| t.ttype == "clockSpec").count(), 1);
}

#[test]
fn test_clock_without_format_option() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "clock format $t -gmt true\n");
    let tokens = typed(&mut lsp, &lg, &uri);
    assert_eq!(
        tokens.iter().filter(|t| t.ttype == "clockPercent").count(),
        0
    );
}

/// Tcl's backslash escapes are not uniformly two characters: `\xhh` takes up to
/// two hex digits, `\uhhhh` four, `\Uhhhhhhhh` eight, and `\ooo` up to three
/// octal digits.  Treating every escape as `\` + one char split `\x41` into an
/// `escape` of `\x` and a `string` of `41`.
#[test]
fn test_backslash_escape_widths() {
    let mut lsp = Lsp::tcl();
    let leg = legend(&lsp);
    let src = "set x \"a\\x41b\\u00e9c\\U0001F600d\\101e\\ng\"\n";
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &leg, &uri);
    for esc in ["\\x41", "\\u00e9", "\\U0001F600", "\\101", "\\n"] {
        assert!(
            tokens
                .iter()
                .any(|t| covered(src, t) == esc && t.ttype == "escape"),
            "`{esc}` must be one escape token: {tokens:?}",
        );
    }
    // The digits belong to the escape, so no stray string token starts with one.
    for stray in ["41", "00e9", "0001F600", "01"] {
        assert!(
            !tokens
                .iter()
                .any(|t| covered(src, t) == stray && t.ttype == "string"),
            "`{stray}` must be part of its escape, not a string: {tokens:?}",
        );
    }
}
