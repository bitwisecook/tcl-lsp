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

//! Tests for the LSP semantic-tokens provider.
//! Verifies `semantic_tokens::full` classifies source spans (function/keyword/
//! variable/number/string/comment) correctly.
//!
//! C-Tcl proof: the classification mirrors Tcl's parse — `puts`/`set`/`proc`/
//! `if` are real commands (`info commands`), `$y` is a variable, `42` is a
//! number, and `#…` in command position is a comment.

use tcl_lsp_core::semantic_tokens::{full, legend_token_types};
use tcl_registry::registry_for_dialect;

#[derive(Debug)]
struct Tok {
    line: u32,
    character: u32,
    length: u32,
    ttype: String,
}

/// Decode the LSP delta-encoded token stream into absolute (line, char,
/// length, type-name) tuples.
fn decode(source: &str, dialect: &str) -> Vec<Tok> {
    let registry = registry_for_dialect(dialect);
    let st = full(source, dialect, registry);
    let legend = legend_token_types();
    let mut out = Vec::new();
    let (mut line, mut character) = (0u32, 0u32);
    for chunk in st.data.chunks(5) {
        let (dl, dc, length, ty) = (chunk[0], chunk[1], chunk[2], chunk[3]);
        if dl > 0 {
            line += dl;
            character = dc;
        } else {
            character += dc;
        }
        out.push(Tok {
            line,
            character,
            length,
            ttype: legend[ty as usize].to_string(),
        });
    }
    out
}

fn types(source: &str, dialect: &str) -> Vec<String> {
    decode(source, dialect)
        .into_iter()
        .map(|t| t.ttype)
        .collect()
}

/// The source substring covered by a decoded token, resolved by
/// (line, character, length).  Used to assert *which* word carries a kind.
fn tok_text(source: &str, t: &Tok) -> String {
    let line = source.split('\n').nth(t.line as usize).unwrap_or("");
    line.chars()
        .skip(t.character as usize)
        .take(t.length as usize)
        .collect()
}

/// The token kind covering the first occurrence of `word` in `source`
/// (matched by column), or `None` if no token starts there.
fn kind_of_word(source: &str, dialect: &str, word: &str) -> Option<String> {
    let (needle_line, needle_col) = {
        let mut found = None;
        for (li, line) in source.split('\n').enumerate() {
            if let Some(byte_col) = line.find(word) {
                let char_col = line[..byte_col].chars().count();
                found = Some((u32::try_from(li).unwrap(), u32::try_from(char_col).unwrap()));
                break;
            }
        }
        found?
    };
    decode(source, dialect)
        .into_iter()
        .find(|t| t.line == needle_line && t.character == needle_col)
        .map(|t| t.ttype)
}

#[test]
fn simple_command_and_word() {
    let t = decode("puts hello", "tcl8.6");
    assert_eq!(t.len(), 2);
    // `puts` is a builtin → function, 4 chars long.
    assert_eq!(t[0].ttype, "function");
    assert_eq!(t[0].length, 4);
    // A bare-word argument is classified as a string span.
    assert_eq!(t[1].ttype, "string");
}

#[test]
fn variable_and_command() {
    let ty = types("set x $y", "tcl8.6");
    assert!(ty.contains(&"function".to_string()), "set is a function");
    assert!(ty.contains(&"variable".to_string()), "$y is a variable");
}

#[test]
fn number_literal() {
    let toks = decode("set x 42", "tcl8.6");
    let n: Vec<&Tok> = toks.iter().filter(|t| t.ttype == "number").collect();
    assert_eq!(n.len(), 1);
    assert_eq!(n[0].length, 2);
}

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
    // the next physical line, and that continuation is a comment too (#759).
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
    // command separator — `puts hi ;# tail` (issue #759 review).
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

#[test]
fn proc_is_a_keyword() {
    let t = decode("proc foo {x} {}", "tcl8.6");
    assert_eq!(t[0].ttype, "keyword");
}

#[test]
fn if_elseif_else_are_keywords() {
    let source = "if 1 {\n puts a\n} elseif 2 {\n puts b\n} else {\n puts c\n}";
    let lines: Vec<&str> = source.split('\n').collect();
    let kw: std::collections::HashSet<String> = decode(source, "tcl8.6")
        .into_iter()
        .filter(|t| t.ttype == "keyword")
        .map(|t| {
            let l = lines[t.line as usize];
            l.chars()
                .skip(t.character as usize)
                .take(t.length as usize)
                .collect::<String>()
        })
        .collect();
    for w in ["if", "elseif", "else"] {
        assert!(kw.contains(w), "{w} must be a keyword; got {kw:?}");
    }
}

// TclOO definition-body highlighting (issue #747).
//
// C-Tcl proof: `oo::configurable`, `oo::abstract`, and `oo::singleton` are
// real Tcl 9.0 metaclasses (`info commands oo::*`) that manufacture classes
// with the same `create name ?body?` shape as `oo::class`.  Inside such a
// definition body the words `superclass` / `self` / `property` / `method`
// are TclOO definition sub-keywords, and a `method`/`constructor` body holds
// ordinary Tcl code — so `set x 1` inside it is a real command call.

#[test]
fn oo_configurable_body_keywords_are_highlighted() {
    // Regression for #747: `superclass` / `self` / `property` / `method`
    // inside an `oo::configurable` body must highlight as keywords, exactly
    // as they already do inside `oo::class`.
    let src = concat!(
        "oo::configurable create Widget {\n",
        "    superclass Base\n",
        "    self mixin M\n",
        "    property color\n",
        "    method paint {} { set x 1 }\n",
        "}\n",
    );
    for kw in ["superclass", "self", "property", "method"] {
        assert_eq!(
            kind_of_word(src, "tcl9.0", kw).as_deref(),
            Some("keyword"),
            "`{kw}` inside oo::configurable body must be a keyword",
        );
    }
}

#[test]
fn oo_configurable_method_body_is_recursed() {
    // The method body `{ set x 1 }` must be tokenised (not emitted as one
    // opaque string): `set` → function, `1` → number.
    let src = concat!(
        "oo::configurable create Widget {\n",
        "    method paint {} { set x 1 }\n",
        "}\n",
    );
    let toks = decode(src, "tcl9.0");
    assert!(
        toks.iter()
            .any(|t| t.ttype == "function" && tok_text(src, t) == "set"),
        "method body should tokenise `set` as a function: {toks:?}",
    );
    assert!(
        toks.iter().any(|t| t.ttype == "number"),
        "method body should tokenise the `1` literal as a number: {toks:?}",
    );
}

#[test]
fn oo_class_method_body_is_recursed() {
    // Regression for #747 (comment: "even with oo::class it stops at the
    // method body"): the `method`/`constructor` body inside an oo::class
    // block must be recursed, not left as one opaque string.
    let src = concat!(
        "oo::class create C {\n",
        "    constructor {} { set y 2 }\n",
        "    method m {} { puts hello }\n",
        "}\n",
    );
    let toks = decode(src, "tcl9.0");
    // `puts` (a builtin) inside the method body → function.
    assert!(
        toks.iter()
            .any(|t| t.ttype == "function" && tok_text(src, t) == "puts"),
        "method body should tokenise `puts`: {toks:?}",
    );
    // `set` inside the constructor body → function; `2` → number.
    assert!(
        toks.iter()
            .any(|t| t.ttype == "function" && tok_text(src, t) == "set"),
        "constructor body should tokenise `set`: {toks:?}",
    );
    assert!(
        toks.iter().any(|t| t.ttype == "number"),
        "constructor body should tokenise the `2` literal: {toks:?}",
    );
}

#[test]
fn oo_abstract_and_singleton_bodies_are_recursed() {
    for metaclass in ["oo::abstract", "oo::singleton"] {
        let src = format!("{metaclass} create C {{\n    method m {{}} {{ set z 3 }}\n}}\n");
        assert_eq!(
            kind_of_word(&src, "tcl9.0", "method").as_deref(),
            Some("keyword"),
            "`method` inside {metaclass} body must be a keyword",
        );
        let toks = decode(&src, "tcl9.0");
        assert!(
            toks.iter()
                .any(|t| t.ttype == "function" && tok_text(&src, t) == "set"),
            "{metaclass}: method body should tokenise `set`: {toks:?}",
        );
    }
}

#[test]
fn oo_property_accessor_bodies_are_recursed() {
    // `property NAME -get { BODY } -set { BODY }` accessor bodies inside a
    // configurable class must be recursed too.
    let src = concat!(
        "oo::configurable create Widget {\n",
        "    property color -get { return red } -set { set color $value }\n",
        "}\n",
    );
    let toks = decode(src, "tcl9.0");
    // `return` is itself a language keyword; its presence inside the `-get`
    // body proves the accessor body was recursed.
    assert!(
        toks.iter()
            .any(|t| t.ttype == "keyword" && tok_text(src, t) == "return"),
        "`-get` accessor body should tokenise `return`: {toks:?}",
    );
    assert!(
        toks.iter()
            .any(|t| t.ttype == "function" && tok_text(src, t) == "set"),
        "`-set` accessor body should tokenise `set`: {toks:?}",
    );
    // The `$value` variable substitution inside the `-set` body is recursed.
    assert!(
        toks.iter().any(|t| t.ttype == "variable"),
        "`-set` accessor body should tokenise `$value`: {toks:?}",
    );
}

#[test]
fn top_level_method_is_not_an_oo_body() {
    // Context guard: a top-level user proc named `method` (outside any OO
    // definition body) must NOT be treated as an OO method definition.  A
    // bare `method a b {c}` call at top level is a plain command — its last
    // braced word must stay an opaque string, not be recursed as a body.
    let src = "method a b { this is not code }\n";
    let toks = decode(src, "tcl8.6");
    // No command inside the braced word should be tokenised as a function;
    // `this`/`is`/`not`/`code` must never surface as separate command heads.
    assert!(
        !toks.iter().any(|t| t.ttype == "function"),
        "top-level `method` braced arg must not be recursed as a body: {toks:?}",
    );
}

#[test]
fn oo_define_body_form_recurses_method_bodies() {
    // The `oo::define Cls { … }` script form is an outer OO definition body
    // too — method bodies inside it must recurse.
    let src = concat!("oo::define Cls {\n", "    method m {} { puts hi }\n", "}\n",);
    assert_eq!(
        kind_of_word(src, "tcl9.0", "method").as_deref(),
        Some("keyword"),
    );
    let toks = decode(src, "tcl9.0");
    assert!(
        toks.iter()
            .any(|t| t.ttype == "function" && tok_text(src, t) == "puts"),
        "oo::define body method should recurse: {toks:?}",
    );
}

#[test]
fn oo_define_member_form_body_is_not_oo_context() {
    // Issue #747 review (Codex P2): the member form
    // `oo::define C method m {} { … }` carries an ordinary method body, not
    // a definition script.  A nested `method a b {not code}` inside it must
    // NOT be treated as an OO member definition — its `{not code}` stays an
    // opaque string (no command inside is tokenised as a function).
    let src = "oo::define C method m {} { method a b {not code} }\n";
    let toks = decode(src, "tcl9.0");
    assert!(
        !toks.iter().any(|t| t.ttype == "function"),
        "nested `method a b {{not code}}` inside a member-form method body \
         must not be recursed as an OO body: {toks:?}",
    );
    // The real method body itself is still recursed (its own commands are
    // walked as ordinary code) — a `set` there would tokenise.
    let src2 = "oo::define C method m {} { set x 1 }\n";
    let toks2 = decode(src2, "tcl9.0");
    assert!(
        toks2
            .iter()
            .any(|t| t.ttype == "function" && tok_text(src2, t) == "set"),
        "member-form method body should still tokenise its own code: {toks2:?}",
    );
}

#[test]
fn multiline_braced_string_literal_is_highlighted_per_line() {
    // Issue #757: a braced string literal spanning multiple lines lost its
    // highlighting entirely (the enclosing `string` token was dropped because
    // it crossed a newline).  It must now emit one `string` token per covered
    // line, matching the quoted-string case.
    let src = "set x {some long\nstring that spans\nmultiple lines}\n";
    let toks = decode(src, "tcl9.0");
    for line in 0..=2 {
        assert!(
            toks.iter().any(|t| t.line == line && t.ttype == "string"),
            "line {line} of the braced literal must carry a string token: {toks:?}",
        );
    }
    // A quoted literal spanning the same lines highlights identically — the
    // two forms now behave the same (the crux of the report).
    let quoted = "set x \"some long\nstring that spans\nmultiple lines\"\n";
    let qtoks = decode(quoted, "tcl9.0");
    let strings_on = |toks: &[Tok]| -> Vec<(u32, u32, u32)> {
        toks.iter()
            .filter(|t| t.ttype == "string")
            .map(|t| (t.line, t.character, t.length))
            .collect()
    };
    assert_eq!(
        strings_on(&toks),
        strings_on(&qtoks),
        "braced and quoted multi-line string literals must highlight identically",
    );
}

#[test]
fn multiline_literal_tokens_never_span_a_newline() {
    // The per-line split keeps the LSP invariant that no single emitted token
    // crosses a line boundary — every string entry stays within one line's
    // bounds (character + length must not exceed that line's length).
    let src = "set x {alpha\nbeta gamma\ndelta}\n";
    let toks = decode(src, "tcl9.0");
    for t in &toks {
        let line_chars = src
            .split('\n')
            .nth(t.line as usize)
            .unwrap_or("")
            .chars()
            .count();
        assert!(
            (t.character + t.length) as usize <= line_chars,
            "token {t:?} overruns line {} (len {line_chars})",
            t.line,
        );
    }
}

#[test]
fn hash_inside_multiline_literal_is_string_not_overlapping_comment() {
    // Issue #757 review (Codex P1): a physical line inside a multi-line literal
    // whose first non-whitespace char is `#` is literal text, not a comment.
    // The per-line string entry must be the only token on that line — the
    // comment scanner must not also emit an overlapping `comment` token (which
    // an LSP client would reject).
    for src in [
        "set x {a\n# not a comment\nb}\n",
        "set x \"a\n# not a comment\nb\"\n",
        "set x {a\n   # indented\nb}\n",
    ] {
        let toks = decode(src, "tcl9.0");
        // The `#` line (line 1) is a string, never a comment.
        let line1: Vec<&Tok> = toks.iter().filter(|t| t.line == 1).collect();
        assert!(
            line1.iter().any(|t| t.ttype == "string"),
            "line 1 must carry a string token for {src:?}: {toks:?}",
        );
        assert!(
            !line1.iter().any(|t| t.ttype == "comment"),
            "the `#` inside the literal must not become a comment for {src:?}: {toks:?}",
        );
        // No two tokens overlap anywhere in the stream.
        for (i, a) in toks.iter().enumerate() {
            for b in &toks[i + 1..] {
                if a.line == b.line {
                    let (lo, hi) = if a.character <= b.character {
                        (a, b)
                    } else {
                        (b, a)
                    };
                    assert!(
                        lo.character + lo.length <= hi.character,
                        "overlapping tokens {a:?} and {b:?} for {src:?}",
                    );
                }
            }
        }
    }
    // A genuine full-line comment is still emitted (not covered by any literal).
    let real = decode("# a real comment\nset x 1\n", "tcl9.0");
    assert!(
        real.iter().any(|t| t.line == 0 && t.ttype == "comment"),
        "a real top-level comment must still be a comment token: {real:?}",
    );
    // A comment inside a recursed proc body is still a comment.
    let body = decode("proc f {} {\n# real comment\nset x 1\n}\n", "tcl9.0");
    assert!(
        body.iter().any(|t| t.line == 1 && t.ttype == "comment"),
        "a real comment inside a body must still be a comment token: {body:?}",
    );
}
