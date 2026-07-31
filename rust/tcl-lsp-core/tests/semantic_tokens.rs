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

use tcl_lsp_core::semantic_tokens::{full, legend_token_modifiers, legend_token_types};
use tcl_registry::registry_for_dialect;

#[derive(Debug)]
struct Tok {
    line: u32,
    character: u32,
    length: u32,
    ttype: String,
    mods: u32,
}

/// Decode the LSP delta-encoded token stream into absolute (line, char,
/// length, type-name, modifier-bitmask) tuples.
fn decode(source: &str, dialect: &str) -> Vec<Tok> {
    let registry = registry_for_dialect(dialect);
    let st = full(source, dialect, registry);
    let legend = legend_token_types();
    let mut out = Vec::new();
    let (mut line, mut character) = (0u32, 0u32);
    for chunk in st.data.chunks(5) {
        let (dl, dc, length, ty, mods) = (chunk[0], chunk[1], chunk[2], chunk[3], chunk[4]);
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
            mods,
        });
    }
    out
}

/// `true` when a decoded token's modifier bitmask carries `defaultLibrary`
/// (a registry built-in), resolved against [`legend_token_modifiers`] rather
/// than a hardcoded bit position.
fn has_default_library(t: &Tok) -> bool {
    let bit = legend_token_modifiers()
        .iter()
        .position(|&m| m == "defaultLibrary")
        .expect("legend declares defaultLibrary");
    t.mods & (1 << bit) != 0
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
fn oo_wrapper_block_body_keeps_member_grammar() {
    // TclOO's bare wrapper-block form `private { … }` / `self { … }` is a
    // nested *definition* script: the members inside it (`method`, `variable`)
    // must still be recognised as keywords, and their bodies recursed — not
    // walked as ordinary Tcl (Codex review of #839: the block dropped out of
    // definition grammar, so `method` inside `private { … }` went unhighlighted).
    for wrapper in ["private", "self"] {
        let src = format!(
            "oo::class create C {{\n    {wrapper} {{\n        method m {{}} {{ set y 2 }}\n        variable secret 0\n    }}\n}}\n",
        );
        // The inner member keywords resolve from the grammar the block kept.
        for kw in ["method", "variable"] {
            assert_eq!(
                kind_of_word(&src, "tcl9.0", kw).as_deref(),
                Some("keyword"),
                "`{kw}` inside `{wrapper} {{ … }}` must be a member keyword: {:?}",
                decode(&src, "tcl9.0"),
            );
        }
        // The inner method's body is recursed (its `set` builtin tokenises).
        let toks = decode(&src, "tcl9.0");
        assert!(
            toks.iter()
                .any(|t| t.ttype == "function" && tok_text(&src, t) == "set"),
            "`{wrapper} {{ … }}` inner method body should tokenise `set`: {toks:?}",
        );
        // The declared instance variable resolves as a variable.
        assert_eq!(
            kind_of_word(&src, "tcl9.0", "secret").as_deref(),
            Some("variable"),
            "`variable secret` inside `{wrapper} {{ … }}` should declare a variable",
        );
    }
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
    // definition body) must NOT be treated as an OO member.  A bare
    // `method a b {c}` call at top level is a plain command — so `method`
    // itself is a plain command head (a `function`, NOT a `keyword` — member
    // keywords are recognised context-sensitively from the definer grammar,
    // exactly like snit's `typemethod`), and its last braced word stays an
    // opaque string, not recursed as a body.
    let src = "method a b { this is not code }\n";
    let toks = decode(src, "tcl8.6");
    assert_ne!(
        kind_of_word(src, "tcl8.6", "method").as_deref(),
        Some("keyword"),
        "a top-level `method` must not be a keyword (context-sensitive): {toks:?}",
    );
    // No word inside the braced body should be recursed as a command head;
    // `this`/`is`/`not`/`code` must never surface as separate function tokens.
    for w in ["this", "is", "not", "code"] {
        assert!(
            !toks
                .iter()
                .any(|t| t.ttype == "function" && tok_text(src, t) == w),
            "top-level `method` braced arg must not be recursed as a body ({w}): {toks:?}",
        );
    }
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
    // The innermost `{not code}` must stay an opaque string — `not`/`code`
    // never surface as command heads.  (The inner `method` head is an ordinary
    // command call inside the method body, so it legitimately reads as a
    // `function`, not a keyword — there is no definition-body context here.)
    for w in ["not", "code"] {
        assert!(
            !toks
                .iter()
                .any(|t| t.ttype == "function" && tok_text(src, t) == w),
            "nested `method a b {{not code}}` inside a member-form method body \
             must not be recursed as an OO body ({w}): {toks:?}",
        );
    }
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
fn oo_define_inline_member_keyword_is_grammar_driven() {
    // The inline form `oo::define C method m {} {…}` puts the member keyword at
    // an argument position; it must still highlight as a keyword, sourced from
    // the definer's `definition_body` grammar (not a hardcoded list).  `self`
    // introduces a second inner member keyword.
    let src = "oo::define C method greet {} { return hi }\n";
    assert_eq!(
        kind_of_word(src, "tcl9.0", "method").as_deref(),
        Some("keyword"),
        "inline `oo::define … method` keyword must highlight: {:?}",
        decode(src, "tcl9.0"),
    );
    let src2 = "oo::objdefine obj self method m {} {}\n";
    for kw in ["self", "method"] {
        assert_eq!(
            kind_of_word(src2, "tcl9.0", kw).as_deref(),
            Some("keyword"),
            "inline `oo::objdefine … self method` keyword `{kw}` must highlight: {:?}",
            decode(src2, "tcl9.0"),
        );
    }
}

#[test]
fn tcloo_members_are_context_sensitive_like_snit() {
    // Parity with `snit_member_keyword_only_inside_body`: a TclOO member word
    // used as a bare top-level command (outside any definition body) must NOT
    // be a keyword — both class systems recognise members from the grammar
    // context, so neither over-colours a same-named top-level command.
    for member in ["constructor", "superclass", "classmethod"] {
        let src = format!("{member} whatever args\n");
        assert_ne!(
            kind_of_word(&src, "tcl9.0", member).as_deref(),
            Some("keyword"),
            "top-level `{member}` must not be a keyword outside a body: {:?}",
            decode(&src, "tcl9.0"),
        );
    }
    // …but inside an `oo::class` body every one of them is a keyword.
    let body = "oo::class create C {\n    superclass B\n    constructor {} {}\n    classmethod mk {} {}\n}\n";
    for member in ["superclass", "constructor", "classmethod"] {
        assert_eq!(
            kind_of_word(body, "tcl9.0", member).as_deref(),
            Some("keyword"),
            "`{member}` inside an oo::class body must be a keyword: {:?}",
            decode(body, "tcl9.0"),
        );
    }
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

// ---------------------------------------------------------------------------
// Issue #774 — `variable a b c` inside a TclOO definition body declares every
// name as an instance variable (the namespace-level `variable name ?value?`
// pairs only mark the leading name). All names must highlight as variables.
// ---------------------------------------------------------------------------

#[test]
fn oo_body_variable_highlights_every_name() {
    let src = "oo::class create Foo {\n    variable width height depth\n    method m {} { return 1 }\n}\n";
    // Each of `width`, `height`, `depth` in the class-body `variable`
    // declaration is a variable, not a bareword string.
    for name in ["width", "height", "depth"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", name).as_deref(),
            Some("variable"),
            "`{name}` in an oo::class body `variable` must be a variable: {:?}",
            decode(src, "tcl8.6"),
        );
    }
}

#[test]
fn plain_variable_command_keeps_pair_classification() {
    // Outside an OO definition body (a normal namespace-level `variable`), the
    // leading name is a write target — highlighted as a variable declaration.
    let src = "namespace eval ns {\n    variable myvar 1\n}\n";
    assert_eq!(
        kind_of_word(src, "tcl8.6", "myvar").as_deref(),
        Some("variable"),
        "the leading `variable` name is a variable: {:?}",
        decode(src, "tcl8.6"),
    );
}

// ---------------------------------------------------------------------------
// Issue #776 — a tcltest command imported into the global scope
// (`namespace import tcltest::*`) resolves to its `tcltest::` spec, so bare
// `test`'s options/body are recognised: `-body`/`-result` become options and
// the `-body` script is recursed.
// ---------------------------------------------------------------------------

#[test]
fn imported_tcltest_test_options_are_recognised() {
    let src = "namespace import tcltest::*\n\
               test mytest-1.1 {desc} -body {\n    set x 1\n} -result 1\n";
    // The `-body` / `-result` switches are recognised options (decorator), not
    // opaque strings, and the body's `set` is recursed as a command.
    assert_eq!(
        kind_of_word(src, "tcl8.6", "-body").as_deref(),
        Some("decorator"),
        "imported `test`'s -body must be an option: {:?}",
        decode(src, "tcl8.6"),
    );
    assert_eq!(
        kind_of_word(src, "tcl8.6", "-result").as_deref(),
        Some("decorator"),
        "imported `test`'s -result must be an option: {:?}",
        decode(src, "tcl8.6"),
    );
    assert_eq!(
        kind_of_word(src, "tcl8.6", "set").as_deref(),
        Some("function"),
        "the -body script must be recursed (set is a command): {:?}",
        decode(src, "tcl8.6"),
    );
}

#[test]
fn bare_test_without_import_stays_unresolved() {
    // Without a `namespace import`, a bare `test` is an ordinary (user) command
    // — its `-body` word is not treated as a tcltest option.
    let src = "test mytest-1.1 {desc} -body {\n    set x 1\n} -result 1\n";
    assert_ne!(
        kind_of_word(src, "tcl8.6", "-body").as_deref(),
        Some("decorator"),
        "an unimported bare `test` must not resolve tcltest options: {:?}",
        decode(src, "tcl8.6"),
    );
}

#[test]
fn import_does_not_retroactively_resolve_earlier_command() {
    // Source order: a `test` used *before* the `namespace import` must NOT be
    // retagged as `tcltest::test` (the import isn't in effect yet), so its
    // `-body` stays a plain word — but the `test` *after* the import resolves.
    // Exactly one of the two `-body` words (the post-import one) is a decorator.
    let src = "test early {d} -body {\n    set a 1\n}\n\
               namespace import tcltest::*\n\
               test late {d} -body {\n    set b 2\n}\n";
    let toks = decode(src, "tcl8.6");
    let body_decorators = toks
        .iter()
        .filter(|t| t.ttype == "decorator" && tok_text(src, t) == "-body")
        .count();
    assert_eq!(
        body_decorators, 1,
        "only the post-import `test`'s -body must resolve (not the earlier one): {toks:?}",
    );
}

// ---------------------------------------------------------------------------
// Issue #775 — the argument to `source` is still tokenised: a command
// substitution `[...]` in the file-name argument is highlighted as a command
// sequence (its head + args), not left as one opaque string.  (No behaviour
// change on the rust branch — this locks the already-correct classification.)
// ---------------------------------------------------------------------------

#[test]
fn source_command_substitution_argument_is_tokenised() {
    let src = "source [file join $dir lib.tcl]\n";
    let toks = decode(src, "tcl8.6");
    // `source` head.
    assert!(
        toks.iter()
            .any(|t| t.character == 0 && (t.ttype == "keyword" || t.ttype == "function")),
        "source head must be highlighted: {toks:?}",
    );
    // The `[file join …]` substitution is recursed: `file` is a command and
    // `$dir` a variable — not swallowed into one opaque string.
    assert_eq!(
        kind_of_word(src, "tcl8.6", "file").as_deref(),
        Some("function"),
        "the command substitution in source's argument must be tokenised: {toks:?}",
    );
    assert_eq!(
        kind_of_word(src, "tcl8.6", "$dir").as_deref(),
        Some("variable"),
        "a variable inside source's argument substitution must be a variable: {toks:?}",
    );
}

// ---------------------------------------------------------------------------
// Peer bugs of #774 — other multi-name variable-declaring commands whose
// trailing names the registry's single leading VarWrite role leaves as
// strings.  The analyser already tracks these correctly (via lowering hooks);
// only the highlighting lagged.
// ---------------------------------------------------------------------------

#[test]
fn global_highlights_every_name() {
    // `global a b c` declares every name — not just the first.
    let src = "proc p {} {\n    global alpha beta gamma\n}\n";
    for name in ["alpha", "beta", "gamma"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", name).as_deref(),
            Some("variable"),
            "`{name}` in `global` must be a variable: {:?}",
            decode(src, "tcl8.6"),
        );
    }
}

#[test]
fn namespace_variable_pairs_highlight_names_not_values() {
    // Namespace-level `variable name value name value` — the names (even arg
    // positions) are variables; the interleaved values keep their own kind.
    let src = "namespace eval ns {\n    variable alpha 1 beta 2\n}\n";
    assert_eq!(
        kind_of_word(src, "tcl8.6", "alpha").as_deref(),
        Some("variable"),
        "{:?}",
        decode(src, "tcl8.6"),
    );
    assert_eq!(
        kind_of_word(src, "tcl8.6", "beta").as_deref(),
        Some("variable"),
        "second name must also be a variable: {:?}",
        decode(src, "tcl8.6"),
    );
    // The value `1` stays a number, not a variable.
    assert_eq!(
        kind_of_word(src, "tcl8.6", "1").as_deref(),
        Some("number"),
        "an interleaved value must not become a variable: {:?}",
        decode(src, "tcl8.6"),
    );
}

// ---------------------------------------------------------------------------
// Loop-variable highlighting — `foreach` / `lmap` / `dict for` bind their
// iteration variables, which must read as variable declarations, whether a
// single bareword or the elements of a braced list.
// ---------------------------------------------------------------------------

#[test]
fn foreach_single_loop_var_is_variable() {
    let src = "foreach item $lst { puts $item }\n";
    assert_eq!(
        kind_of_word(src, "tcl8.6", "item").as_deref(),
        Some("variable"),
        "the foreach loop variable must be a variable: {:?}",
        decode(src, "tcl8.6"),
    );
}

#[test]
fn foreach_braced_loop_vars_are_variables() {
    let src = "foreach {key val} $lst { puts $key }\n";
    for name in ["key", "val"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", name).as_deref(),
            Some("variable"),
            "`{name}` in the braced foreach varlist must be a variable: {:?}",
            decode(src, "tcl8.6"),
        );
    }
}

#[test]
fn foreach_multi_pair_loop_vars_are_variables() {
    // `foreach a $l1 b $l2 …` — the variable spec sits at every other word.
    let src = "foreach aa $l1 bb $l2 { puts $aa }\n";
    for name in ["aa", "bb"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", name).as_deref(),
            Some("variable"),
            "`{name}` in a multi-pair foreach must be a variable: {:?}",
            decode(src, "tcl8.6"),
        );
    }
}

#[test]
fn lmap_and_dict_for_loop_vars_are_variables() {
    assert_eq!(
        kind_of_word("lmap elem $xs { expr {$elem} }\n", "tcl8.6", "elem").as_deref(),
        Some("variable"),
        "lmap loop variable must be a variable",
    );
    let df = "dict for {dkey dval} $d { puts $dkey }\n";
    for name in ["dkey", "dval"] {
        assert_eq!(
            kind_of_word(df, "tcl8.6", name).as_deref(),
            Some("variable"),
            "`{name}` in `dict for` must be a variable: {:?}",
            decode(df, "tcl8.6"),
        );
    }
}

#[test]
fn non_loop_command_first_arg_stays_string() {
    // A user command that merely resembles a loop must not have its first
    // bareword argument reclassified as a loop variable.
    let src = "myproc item $lst { puts hi }\n";
    assert_eq!(
        kind_of_word(src, "tcl8.6", "item").as_deref(),
        Some("string"),
        "a non-loop command's bareword arg must stay a string: {:?}",
        decode(src, "tcl8.6"),
    );
}

// ---------------------------------------------------------------------------
// Parameter-list highlighting — proc / method / constructor / apply-lambda
// parameters, and the registry `LoopVarList` role for `dict map` (peer of
// `dict for`).  A *parameter* carries the standard LSP `parameter` type (#898
// §4), so a theme can tell an argument from an ordinary local; a *loop*
// variable stays a plain variable declaration.
// ---------------------------------------------------------------------------

#[test]
fn proc_parameters_are_parameters() {
    let src = "proc greet {name age} { return $name }\n";
    for name in ["name", "age"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", name).as_deref(),
            Some("parameter"),
            "proc parameter `{name}` must be a parameter: {:?}",
            decode(src, "tcl8.6"),
        );
    }
}

#[test]
fn proc_parameter_default_pair_splits_name_and_value() {
    // `{b 5}` — `b` is the parameter name (variable), `5` the default (number).
    let src = "proc p {a {b 5} args} { return $a }\n";
    for name in ["a", "b", "args"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", name).as_deref(),
            Some("parameter"),
            "`{name}` must be a parameter: {:?}",
            decode(src, "tcl8.6"),
        );
    }
    assert_eq!(
        kind_of_word(src, "tcl8.6", "5").as_deref(),
        Some("number"),
        "the default value must stay a number: {:?}",
        decode(src, "tcl8.6"),
    );
}

#[test]
fn apply_lambda_parameters_are_parameters() {
    let src = "apply {{alpha beta} { return $alpha }} 1 2\n";
    for name in ["alpha", "beta"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", name).as_deref(),
            Some("parameter"),
            "apply lambda parameter `{name}` must be a parameter: {:?}",
            decode(src, "tcl8.6"),
        );
    }
}

#[test]
fn tcloo_method_and_constructor_parameters_are_parameters() {
    let src = "oo::class create C {\n    method greet {who} { return $who }\n    constructor {aa bb} { set x $aa }\n}\n";
    for name in ["who", "aa", "bb"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", name).as_deref(),
            Some("parameter"),
            "TclOO parameter `{name}` must be a parameter: {:?}",
            decode(src, "tcl8.6"),
        );
    }
}

#[test]
fn snit_macro_arglist_parameters_are_parameters() {
    // `snit::macro name arglist body` — the arglist word (registry arg-role
    // `ParamList`) holds the macro's formal parameters, so its names read as
    // variables and the body highlights, exactly like a proc.
    let src = "snit::macro mymac {alpha beta} { return $alpha }\n";
    for name in ["alpha", "beta"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", name).as_deref(),
            Some("parameter"),
            "snit::macro parameter `{name}` must be a parameter: {:?}",
            decode(src, "tcl8.6"),
        );
    }
}

#[test]
fn dict_map_loop_vars_are_variables() {
    // Peer of `dict for`: `dict map {k v}` binds its loop variables too.
    let src = "dict map {mk mv} $d { set mk $mv }\n";
    for name in ["mk", "mv"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", name).as_deref(),
            Some("variable"),
            "`{name}` in `dict map` must be a variable: {:?}",
            decode(src, "tcl8.6"),
        );
    }
}

// ---------------------------------------------------------------------------
// By-reference local variable highlighting — `upvar` / `namespace upvar`
// locals and `dict update` var names read as declarations; the "other" names,
// the level word, and dict keys stay strings.
// ---------------------------------------------------------------------------

#[test]
fn upvar_local_names_are_variables() {
    let src = "upvar 1 othervar localvar\n";
    assert_eq!(
        kind_of_word(src, "tcl8.6", "localvar").as_deref(),
        Some("variable"),
        "the upvar local must be a variable: {:?}",
        decode(src, "tcl8.6"),
    );
    assert_eq!(
        kind_of_word(src, "tcl8.6", "othervar").as_deref(),
        Some("string"),
        "the other-frame name must stay a string: {:?}",
        decode(src, "tcl8.6"),
    );
}

#[test]
fn upvar_without_level_and_multi_pair() {
    // No level word: the local is the second word; multi-pair marks each local.
    let src = "upvar aone bone atwo btwo\n";
    for local in ["bone", "btwo"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", local).as_deref(),
            Some("variable"),
            "`{local}` must be an upvar local: {:?}",
            decode(src, "tcl8.6"),
        );
    }
    for other in ["aone", "atwo"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", other).as_deref(),
            Some("string"),
            "`{other}` must stay a string: {:?}",
            decode(src, "tcl8.6"),
        );
    }
}

#[test]
fn namespace_upvar_locals_are_variables() {
    let src = "namespace upvar ::ns othervar localvar\n";
    assert_eq!(
        kind_of_word(src, "tcl8.6", "localvar").as_deref(),
        Some("variable"),
        "{:?}",
        decode(src, "tcl8.6"),
    );
}

#[test]
fn dict_update_var_names_are_variables() {
    let src = "dict update mydict keyone varone keytwo vartwo { set varone 1 }\n";
    for v in ["varone", "vartwo"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", v).as_deref(),
            Some("variable"),
            "`{v}` must be a dict-update variable: {:?}",
            decode(src, "tcl8.6"),
        );
    }
    // The keys are not variables.
    assert_eq!(
        kind_of_word(src, "tcl8.6", "keyone").as_deref(),
        Some("string"),
        "a dict-update key must stay a string: {:?}",
        decode(src, "tcl8.6"),
    );
}

// ---------------------------------------------------------------------------
// snit — the definition-body grammar is registry data (tcl_registry::definer),
// so `snit::type` / `snit::widget` bodies recurse and highlight like TclOO
// without any snit-specific code in the token walk.
// ---------------------------------------------------------------------------

#[test]
fn snit_type_members_highlight() {
    let src = "snit::type Dog {\n\
               \x20   variable barks\n\
               \x20   method bark {volume} { return $barks }\n\
               \x20   typemethod count {} { return 1 }\n\
               \x20   constructor {args} { set barks 0 }\n\
               }\n";
    // Instance variable declaration.
    assert_eq!(
        kind_of_word(src, "tcl8.6", "barks").as_deref(),
        Some("variable"),
        "snit `variable` name must be a variable: {:?}",
        decode(src, "tcl8.6"),
    );
    // Method parameter — the standard LSP `parameter` type (#898 §4).
    assert_eq!(
        kind_of_word(src, "tcl8.6", "volume").as_deref(),
        Some("parameter"),
        "snit method parameter must be a parameter: {:?}",
        decode(src, "tcl8.6"),
    );
    // snit-specific member keyword.
    assert_eq!(
        kind_of_word(src, "tcl8.6", "typemethod").as_deref(),
        Some("keyword"),
        "`typemethod` must be a keyword inside a snit body: {:?}",
        decode(src, "tcl8.6"),
    );
    // The `set` inside a method body is recursed (not one opaque string).
    assert_eq!(
        kind_of_word(src, "tcl8.6", "set").as_deref(),
        Some("function"),
        "a snit method body must be recursed: {:?}",
        decode(src, "tcl8.6"),
    );
}

#[test]
fn snit_widget_declarations_and_handlers() {
    let src = "snit::widget Dial {\n\
               \x20   typevariable count\n\
               \x20   component inner\n\
               \x20   onconfigure -value valuevar { set count $valuevar }\n\
               }\n";
    for name in ["count", "inner", "valuevar"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", name).as_deref(),
            Some("variable"),
            "snit declaration `{name}` must be a variable: {:?}",
            decode(src, "tcl8.6"),
        );
    }
}

#[test]
fn snit_member_keyword_only_inside_body() {
    // A top-level user proc named `typemethod` must NOT be a keyword — the
    // member keywords are context-sensitive to a definition body.
    let src = "typemethod foo bar\n";
    assert_ne!(
        kind_of_word(src, "tcl8.6", "typemethod").as_deref(),
        Some("keyword"),
        "a top-level `typemethod` must not be a keyword: {:?}",
        decode(src, "tcl8.6"),
    );
}

// ---------------------------------------------------------------------------
// [incr Tcl] — `itcl::class` definition bodies. Members (method / proc /
// variable / common / constructor / destructor / inherit) plus the access
// modifiers public / protected / private (prefix wrappers) all resolve from the
// registry's ITCL grammar, like TclOO/snit.
// ---------------------------------------------------------------------------

#[test]
fn itcl_class_members_highlight() {
    let src = "itcl::class Stack {\n\
               \x20   inherit Deque\n\
               \x20   variable contents {}\n\
               \x20   constructor {aa} { set contents $aa }\n\
               \x20   method push {value} { lappend contents $value }\n\
               }\n";
    // The `itcl::class` head is a keyword (like `oo::class`).
    assert_eq!(
        kind_of_word(src, "tcl8.6", "class").as_deref(),
        Some("keyword"),
        "`itcl::class` must be a keyword: {:?}",
        decode(src, "tcl8.6"),
    );
    // Member keywords.
    for kw in ["inherit", "variable", "constructor", "method"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", kw).as_deref(),
            Some("keyword"),
            "`{kw}` inside an itcl::class body must be a keyword: {:?}",
            decode(src, "tcl8.6"),
        );
    }
    // The instance variable declaration is a variable; the method's parameter
    // carries the standard LSP `parameter` type (#898 §4).
    assert_eq!(
        kind_of_word(src, "tcl8.6", "contents").as_deref(),
        Some("variable")
    );
    assert_eq!(
        kind_of_word(src, "tcl8.6", "value").as_deref(),
        Some("parameter")
    );
}

#[test]
fn itcl_access_modifier_wraps_inner_member() {
    // `public method …` / `private variable …`: the modifier AND the inner
    // member keyword are keywords, the inner param/var declarations resolve, and
    // the inner method body recurses.
    let src = "itcl::class C {\n\
               \x20   public method size {ww} { return $ww }\n\
               \x20   private variable secret 0\n\
               }\n";
    for kw in ["public", "method", "private", "variable"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", kw).as_deref(),
            Some("keyword"),
            "`{kw}` must be a keyword in an itcl body: {:?}",
            decode(src, "tcl8.6"),
        );
    }
    // The wrapped method's parameter and the wrapped variable's name both
    // resolve — the parameter as `parameter` (#898 §4), the variable as a
    // variable.
    assert_eq!(
        kind_of_word(src, "tcl8.6", "ww").as_deref(),
        Some("parameter")
    );
    assert_eq!(
        kind_of_word(src, "tcl8.6", "secret").as_deref(),
        Some("variable")
    );
}

#[test]
fn itcl_member_keyword_only_inside_body() {
    // Context guard: a bare top-level `inherit` / `common` is not a keyword.
    for kw in ["inherit", "common"] {
        let src = format!("{kw} whatever\n");
        assert_ne!(
            kind_of_word(&src, "tcl8.6", kw).as_deref(),
            Some("keyword"),
            "top-level `{kw}` must not be a keyword: {:?}",
            decode(&src, "tcl8.6"),
        );
    }
}

#[test]
fn itcl_variable_config_body_recurses() {
    // itcl `variable NAME init {configbody}` — the trailing config body is a
    // recursable script (grammar `Body` role), so `puts` inside it is a command.
    let src = "itcl::class C {\n\
               \x20   variable color red { puts $color }\n\
               }\n";
    assert_eq!(
        kind_of_word(src, "tcl8.6", "puts").as_deref(),
        Some("function"),
        "the variable config body must recurse: {:?}",
        decode(src, "tcl8.6"),
    );
    // The declared name is still a variable.
    assert_eq!(
        kind_of_word(src, "tcl8.6", "color").as_deref(),
        Some("variable")
    );
}

#[test]
fn itcl_body_external_definition_highlights() {
    // `itcl::body Class::method {params} {body}` — its parameter list declares
    // variables and its body recurses, via the registry arg-roles.
    let src = "itcl::body C::render {ww hh} { set area [expr {$ww * $hh}] }\n";
    assert_eq!(
        kind_of_word(src, "tcl8.6", "set").as_deref(),
        Some("function"),
        "itcl::body's body must recurse: {:?}",
        decode(src, "tcl8.6"),
    );
    for p in ["ww", "hh"] {
        assert_eq!(
            kind_of_word(src, "tcl8.6", p).as_deref(),
            Some("parameter"),
            "itcl::body param `{p}` must be a parameter: {:?}",
            decode(src, "tcl8.6"),
        );
    }
}

// ===========================================================================
// Issue #806 — report::defstyle scoped command environment.
//
// Inside a report::defstyle style script the report configuration methods
// (top/data/columns/…) highlight as library functions and their ensemble
// operations (`top set`) as subcommand keywords — all from registry data.
// ===========================================================================

#[test]
fn scoped_report_line_command_is_function() {
    let src = "::report::defstyle st {} {\n    top set foo\n    columns\n}\n";
    assert_eq!(
        kind_of_word(src, "tcl8.6", "top"),
        Some("function".to_string())
    );
    assert_eq!(
        kind_of_word(src, "tcl8.6", "columns"),
        Some("function".to_string())
    );
}

#[test]
fn scoped_report_operation_is_keyword() {
    let src = "::report::defstyle st {} {\n    top set foo\n}\n";
    // The `set` operation word after `top` colours as a subcommand keyword.
    let t = decode(src, "tcl8.6");
    assert!(
        t.iter()
            .any(|x| x.ttype == "keyword" && tok_text(src, x) == "set"),
        "`set` op is a keyword: {t:?}",
    );
}

#[test]
fn scoped_report_command_not_highlighted_outside_body() {
    // `columns` at top level is an unknown word, not a library function.
    let src = "columns\n";
    // No defaultLibrary function classification — it is a plain function token
    // (unknown command) but must not gain the scoped treatment; assert it is
    // not classified as a keyword and that a genuinely scoped-only op word is
    // absent.  The key scoping fact: `topdatasep` is unknown at top level.
    let src2 = "topdatasep enable\n";
    assert_ne!(
        kind_of_word(src2, "tcl8.6", "enable"),
        Some("keyword".to_string()),
        "`enable` is not a scoped op outside a style body"
    );
    let _ = src;
}

// ---------------------------------------------------------------------------
// Issue #862 — "set"/"lassign"/"incr"/"lappend"/"append"/"expr" (and every
// other plain builtin) rendered unstyled for users whose theme had no rule
// for the `support.function.tcl` scope a `semanticTokenScopes` override
// mapped `function.defaultLibrary` to — shadowing VS Code's built-in
// cross-theme default instead of supplementing it (fixed in
// editors/vscode/package.json; see
// `vscode_semantic_token_scopes_do_not_shadow_standard_defaults` in
// src/semantic_tokens.rs for the editor-side guard).  These tests pin the
// *classification* side: every reported command — and the same commands used
// inside a TclOO method body or a tcltest `-body` script — must resolve to
// `function` with the `defaultLibrary` modifier, so the naming
// infrastructure that drives the fix stays correct generally, not just for
// the specific commands the report happened to list.
// ---------------------------------------------------------------------------

/// The reported commands, plus `lset` (the command bitwisecook's own
/// investigation on the issue checked) and `puts` as a non-regressed control.
const ISSUE_862_BUILTINS: &[&str] = &[
    "set", "lassign", "incr", "lappend", "append", "expr", "lset", "puts",
];

#[test]
fn issue_862_reported_builtins_are_function_default_library_at_top_level() {
    let src = "set a 1\n\
               lassign {1 2 3} x y z\n\
               incr a\n\
               lappend mylist 1 2 3\n\
               append s hello\n\
               set r [expr {$a + 1}]\n\
               lset a 0 2\n\
               puts $r\n";
    let toks = decode(src, "tcl9.0");
    for &name in ISSUE_862_BUILTINS {
        let hit = toks
            .iter()
            .find(|t| t.ttype == "function" && tok_text(src, t) == name);
        let t = hit
            .unwrap_or_else(|| panic!("`{name}` did not classify as a function token: {toks:?}"));
        assert!(
            has_default_library(t),
            "`{name}` must carry the defaultLibrary modifier: {t:?} in {toks:?}",
        );
    }
}

#[test]
fn issue_862_reported_builtins_are_function_default_library_in_tcloo_method_body() {
    // Same builtins, now inside a TclOO method body — the `oo_grammar`
    // context must not reclassify ordinary commands as members or otherwise
    // change their kind.
    let src = "oo::class create Counter {\n\
               \x20   method bump {} {\n\
               \x20       set a 1\n\
               \x20       lassign {1 2 3} x y z\n\
               \x20       incr a\n\
               \x20       lappend mylist 1 2 3\n\
               \x20       append s hello\n\
               \x20       set r [expr {$a + 1}]\n\
               \x20       lset a 0 2\n\
               \x20       puts $r\n\
               \x20   }\n\
               }\n";
    let toks = decode(src, "tcl9.0");
    for &name in ISSUE_862_BUILTINS {
        let hit = toks
            .iter()
            .find(|t| t.ttype == "function" && tok_text(src, t) == name);
        let t = hit.unwrap_or_else(|| {
            panic!(
                "`{name}` did not classify as a function token inside a TclOO method body: {toks:?}"
            )
        });
        assert!(
            has_default_library(t),
            "`{name}` must carry the defaultLibrary modifier inside a TclOO method body: {t:?} in {toks:?}",
        );
    }
}

#[test]
fn issue_862_reported_builtins_are_function_default_library_in_tcltest_body() {
    // Same builtins, now inside an imported tcltest `-body` script.
    let src = "namespace import tcltest::*\n\
               test mytest-1.1 {desc} -body {\n\
               \x20   set a 1\n\
               \x20   lassign {1 2 3} x y z\n\
               \x20   incr a\n\
               \x20   lappend mylist 1 2 3\n\
               \x20   append s hello\n\
               \x20   set r [expr {$a + 1}]\n\
               \x20   lset a 0 2\n\
               \x20   puts $r\n\
               } -result 1\n";
    let toks = decode(src, "tcl8.6");
    for &name in ISSUE_862_BUILTINS {
        let hit = toks
            .iter()
            .find(|t| t.ttype == "function" && tok_text(src, t) == name);
        let t = hit.unwrap_or_else(|| {
            panic!("`{name}` did not classify as a function token inside a tcltest -body: {toks:?}")
        });
        assert!(
            has_default_library(t),
            "`{name}` must carry the defaultLibrary modifier inside a tcltest -body: {t:?} in {toks:?}",
        );
    }
    // The imported `test` command itself is also a registry built-in.
    let test_tok = toks
        .iter()
        .find(|t| t.ttype == "function" && tok_text(src, t) == "test")
        .expect("`test` did not classify as a function token");
    assert!(
        has_default_library(test_tok),
        "imported `test` must carry the defaultLibrary modifier: {test_tok:?} in {toks:?}",
    );
}

/// A `ltm rule { … }` body inside a BIG-IP config is iRules code, and must be
/// tokenised as such — including the object-reference overlay a standalone
/// `.irul` gets, so `pool /Common/api_pool` reads as an `object`.
#[test]
fn bigip_conf_embedded_rule_body_is_tokenised_as_irules() {
    let mut registry = tcl_registry::CommandRegistry::build_default();
    registry.load_dialect(tcl_dialect::DialectSet::parse("f5-irules").unwrap());
    let src =
        "ltm rule /Common/r {\n    when HTTP_REQUEST {\n        pool /Common/api_pool\n    }\n}\n";
    let toks = tcl_lsp_core::semantic_tokens::bigip_conf_full(src, &registry);
    let types = tcl_lsp_core::semantic_tokens::legend_token_types();
    let mut seen = Vec::new();
    let (mut line, mut col) = (0u32, 0u32);
    for c in toks.data.chunks(5) {
        line += c[0];
        col = if c[0] == 0 { col + c[1] } else { c[1] };
        let text: String = src
            .lines()
            .nth(line as usize)
            .and_then(|l| l.get(col as usize..(col + c[2]) as usize))
            .unwrap_or_default()
            .to_owned();
        seen.push((text, types[c[3] as usize]));
    }
    assert!(
        seen.contains(&("when".to_owned(), "keyword")),
        "the rule body must be Tcl: {seen:?}"
    );
    assert!(
        seen.contains(&("HTTP_REQUEST".to_owned(), "event")),
        "the rule body must be iRules: {seen:?}"
    );
    assert!(
        seen.contains(&("/Common/api_pool".to_owned(), "object")),
        "an object reference in an embedded rule must read as an object: {seen:?}"
    );
}

// ---------------------------------------------------------------------------
// Issue #1078 — a brace-quoted variable-name word is a variable, not a string
// ---------------------------------------------------------------------------
//
// tclsh 9.0.4 / 8.6.14, byte-identical: `set {$n} v; info exists {$n}` → 1
// while `info exists n` → 0, so `{$n}` names a real variable — and quoting is
// the *only* way that variable can ever be written.  Painting the word as a
// plain `string` hid the declaration and invited the reader to see the `$n`
// inside as a substitution.

#[test]
fn brace_quoted_variable_name_word_paints_as_a_variable() {
    let src = "proc f {} {\n    set {$n} 1\n    set n 2\n    puts [set {$n}]\n}\n";
    let toks = decode(src, "tcl8.6");
    let declaration_bit = legend_token_modifiers()
        .iter()
        .position(|m| *m == "declaration")
        .map(|i| 1u32 << i)
        .expect("legend carries `declaration`");

    // The write target on line 1 — the whole `{$n}` word, tagged exactly like
    // the plain `n` write on line 2.
    let write = toks
        .iter()
        .find(|t| t.line == 1 && t.character == 8)
        .expect("a token at the `{$n}` write word");
    assert_eq!(write.ttype, "variable", "{write:?}");
    assert_eq!(write.length, 4, "the whole `{{$n}}` word: {write:?}");
    assert!(
        write.mods & declaration_bit != 0,
        "a write target is a declaration: {write:?}"
    );
    let plain_write = toks
        .iter()
        .find(|t| t.line == 2 && t.character == 8)
        .expect("a token at the `n` write word");
    assert_eq!(
        (
            plain_write.ttype.as_str(),
            plain_write.mods & declaration_bit != 0
        ),
        (write.ttype.as_str(), true),
        "the braced and plain write targets must be tagged alike"
    );

    // The one-arg read on line 3 — a variable *reference*, not a declaration.
    let read = toks
        .iter()
        .find(|t| t.line == 3 && t.character == 14)
        .expect("a token at the `{$n}` read word");
    assert_eq!(read.ttype, "variable", "{read:?}");
    assert_eq!(read.length, 4, "{read:?}");

    // TN control: a brace-quoted word that is *not* in a variable-name role
    // stays a string — the retag keys on the registry role, not on braces.
    assert_eq!(
        kind_of_word("puts {$n}\n", "tcl8.6", "{$n}").as_deref(),
        Some("string"),
        "`puts`'s argument is data, not a variable name"
    );
}
