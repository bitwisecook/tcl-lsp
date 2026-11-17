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

use super::concat_script_window;
use tcl_lexer::{Span, Token, TokenType};

/// Tail words of `source` from `from`, tokenised the way the analyser's
/// argv sees them: each `(text, kind)` pair is located in `source` (in
/// order), and a `Str`/quoted token's span covers its delimiters with
/// `content_offset` pointing past the opener.
fn tail(source: &str, from: usize, pairs: &[(&str, TokenType)]) -> (Vec<String>, Vec<Token>) {
    let mut texts = Vec::new();
    let mut toks = Vec::new();
    let mut at = from;
    for (text, kind) in pairs {
        let delim = usize::from(matches!(*kind, TokenType::Str));
        let content = if text.is_empty() {
            // Locate an empty content by its `{}` delimiter pair.
            source[at..].find("{}").expect("fixture") + at + 1
        } else {
            source[at..].find(text).expect("fixture") + at
        };
        let (s, e) = (content - delim, content + text.len() + delim);
        let mut tok = Token::new(
            *kind,
            Span::new(u32::try_from(s).unwrap(), u32::try_from(e).unwrap()),
        );
        tok.content_offset = u8::try_from(delim).unwrap();
        texts.push((*text).to_owned());
        toks.push(tok);
        at = e;
    }
    (texts, toks)
}

/// A braced word's contents and a bare word join into one script, and
/// every content byte keeps its exact source offset — the window blanks
/// the `{`/`}` delimiters and gap to spaces rather than shifting bytes.
///
/// tclsh8.6.14 / tclsh9.0.4: `eval {set l2} hello` ≡ `set l2 hello`.
#[test]
fn braced_and_bare_words_window_in_place() {
    let src = "eval {set l2} hello";
    let (t, k) = tail(
        src,
        5,
        &[("set l2", TokenType::Str), ("hello", TokenType::Esc)],
    );
    let (win, span) = concat_script_window(&t, &k, src).expect("static");
    assert_eq!(win, " set l2  hello");
    assert_eq!((span.start(), span.end()), (5, 19));
    // The window is positionally exact: `hello` sits at its source offset.
    let off = usize::try_from(span.start()).unwrap();
    assert_eq!(off + win.find("hello").unwrap(), src.find("hello").unwrap());
}

/// A braced word whose contents carry `$`/`[` is still *statically known
/// script text* — the braces blocked the outer substitution, and the
/// eval-family command itself resolves it when the script runs.
///
/// tclsh8.6.14 / tclsh9.0.4: `set value 5; eval {set l2} {$value};
/// puts $l2` → `5`.
#[test]
fn a_braced_word_with_substitution_text_is_static() {
    let src = "eval {set l2} {$value}";
    let (t, k) = tail(
        src,
        5,
        &[("set l2", TokenType::Str), ("$value", TokenType::Str)],
    );
    let (win, _) = concat_script_window(&t, &k, src).expect("braced text is static");
    assert_eq!(win, " set l2   $value ");
}

/// Substitution runs before concatenation, so a dynamic word (or a bare
/// word carrying `$`/`[`/escape/quote bytes) makes the real script
/// unknowable — the helper declines rather than guessing.
#[test]
fn a_dynamic_or_decoded_word_anywhere_declines() {
    let src = "eval $cmd arg";
    let (t, k) = tail(src, 5, &[("$cmd", TokenType::Var), ("arg", TokenType::Esc)]);
    assert_eq!(concat_script_window(&t, &k, src), None);
    let src = "eval set $name";
    let (t, k) = tail(
        src,
        5,
        &[("set", TokenType::Esc), ("$name", TokenType::Var)],
    );
    assert_eq!(concat_script_window(&t, &k, src), None);
    let src = "eval puts [f]";
    let (t, k) = tail(src, 5, &[("puts", TokenType::Esc), ("[f]", TokenType::Cmd)]);
    assert_eq!(concat_script_window(&t, &k, src), None);
    // A backslash in a bare word decodes before the join — the decoded
    // text matches no source bytes, so the window declines.
    let src = "eval set\\ x 5";
    let (t, k) = tail(
        src,
        5,
        &[("set\\ x", TokenType::Esc), ("5", TokenType::Esc)],
    );
    assert_eq!(concat_script_window(&t, &k, src), None);
}

/// An empty braced word contributes nothing — its delimiters blank to
/// whitespace and the remaining word keeps its exact offset. This is the
/// shape where a naive join anchors the tail one byte early.
///
/// tclsh8.6.14 / tclsh9.0.4: `eval {} foo` runs `foo`.
#[test]
fn an_empty_word_blanks_to_whitespace_and_offsets_hold() {
    let src = "eval {} foo";
    let (t, k) = tail(src, 5, &[("", TokenType::Str), ("foo", TokenType::Esc)]);
    let (win, span) = concat_script_window(&t, &k, src).expect("static");
    assert_eq!(win, "   foo");
    let off = usize::try_from(span.start()).unwrap();
    assert_eq!(off + win.find("foo").unwrap(), src.find("foo").unwrap());
}

/// A window with no content at all declines — there is no script to walk.
#[test]
fn an_all_whitespace_window_declines() {
    let src = "eval {} {  }";
    let (t, k) = tail(src, 5, &[("", TokenType::Str), ("  ", TokenType::Str)]);
    assert_eq!(concat_script_window(&t, &k, src), None);
}

/// A word whose text does not sit verbatim at its token's content offset
/// cannot be mapped to source bytes, so the window declines.
#[test]
fn a_text_that_diverges_from_the_source_declines() {
    let src = "eval {set l2} hello";
    let (mut t, k) = tail(
        src,
        5,
        &[("set l2", TokenType::Str), ("hello", TokenType::Esc)],
    );
    t[1] = "other".to_owned();
    assert_eq!(concat_script_window(&t, &k, src), None);
}

/// The trim removes a word's surrounding newlines, so a braced body that
/// ends in one still joins its trailing words into the same command.
///
/// tclsh 8.6.18 / 9.0.4: `namespace eval ::n {\n proc p {} {}\n} extra`
/// → `wrong # args: should be "proc name args body"`.
#[test]
fn trimmed_newlines_do_not_separate_the_joined_command() {
    let src = "eval {\n proc p {} {}\n} extra";
    let (t, k) = tail(
        src,
        5,
        &[
            ("\n proc p {} {}\n", TokenType::Str),
            ("extra", TokenType::Esc),
        ],
    );
    let (win, _) = concat_script_window(&t, &k, src).expect("static");
    assert!(!win.contains('\n'), "{win:?}");
    assert_eq!(
        win.split_whitespace().collect::<Vec<_>>(),
        ["proc", "p", "{}", "{}", "extra"]
    );
}

/// A newline inside a word survives the trim and still separates commands.
///
/// tclsh 8.6.18 / 9.0.4: `eval {set a 1\nset b 2} {}` sets both.
#[test]
fn an_inner_newline_still_separates_commands() {
    let src = "eval {set a 1\nset b 2} {}";
    let (t, k) = tail(
        src,
        5,
        &[("set a 1\nset b 2", TokenType::Str), ("", TokenType::Str)],
    );
    let (win, _) = concat_script_window(&t, &k, src).expect("static");
    assert_eq!(win.matches('\n').count(), 1, "{win:?}");
}

/// Trimming may not expose a final backslash: one whitespace byte stays.
///
/// tclsh 8.6.18 / 9.0.4: `eval {set a b\\ } c` → `wrong # args` (the
/// escaped space keeps `b ` one word and `c` a third argument).
#[test]
fn a_trailing_escaped_space_is_kept() {
    let src = "eval {set a b\\ } c";
    let (t, k) = tail(
        src,
        5,
        &[("set a b\\ ", TokenType::Str), ("c", TokenType::Esc)],
    );
    let (win, _) = concat_script_window(&t, &k, src).expect("static");
    assert!(win.contains("b\\ "), "{win:?}");
}
