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
use tcl_registry::model::ingress::static_context_for;

mod comments;
mod declarative_dialects;
mod definition_bodies;
mod variable_names;

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
    let registry = static_context_for(dialect).commands();
    let st = full(
        source,
        tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
        registry,
    );
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

#[test]
fn multiline_braced_string_literal_is_highlighted_per_line() {
    // A braced string literal spanning multiple lines must emit one `string`
    // token per covered line, matching the quoted-string case; a single token
    // crossing a newline is dropped, losing the highlighting entirely.
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
    // A physical line inside a multi-line literal
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

// A tcltest command imported into the global scope
// (`namespace import tcltest::*`) resolves to its `tcltest::` spec, so bare
// `test`'s options/body are recognised: `-body`/`-result` become options and
// the `-body` script is recursed.

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

// The argument to `source` is still tokenised: a command
// substitution `[...]` in the file-name argument is highlighted as a command
// sequence (its head + args), not left as one opaque string.

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

// A `semanticTokenScopes` override mapping `function.defaultLibrary` to the
// `support.function.tcl` scope shadows VS Code's built-in cross-theme default
// instead of supplementing it, so plain builtins ("set"/"lassign"/"incr"/
// "lappend"/"append"/"expr" and the rest) render unstyled under a theme with
// no rule for that scope.  The editor-side guard is
// `vscode_semantic_token_scopes_do_not_shadow_standard_defaults` in
// src/semantic_tokens.rs; these tests pin the *classification* side: every
// builtin — and the same commands used inside a TclOO method body or a
// tcltest `-body` script — must resolve to `function` with the
// `defaultLibrary` modifier.

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
    registry.load_surface(tcl_dialect::model::SurfaceLayer::Core(
        tcl_dialect::model::Family::F5Irules,
        "",
    ));
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

/// The embedded rule's Tcl region is the body the shared rule parser reads,
/// ended by the iRules brace grammar: an escaped `\}` inside the iRule does
/// not truncate it, so the command after it is still tokenised as iRules.
#[test]
fn bigip_conf_embedded_rule_body_ends_where_the_rule_parser_ends_it() {
    let mut registry = tcl_registry::CommandRegistry::build_default();
    registry.load_surface(tcl_dialect::model::SurfaceLayer::Core(
        tcl_dialect::model::Family::F5Irules,
        "",
    ));
    let src = "ltm rule /Common/r {\n    when HTTP_REQUEST {\n        set x \\}\n        pool /Common/api_pool\n    }\n}\n";
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
        seen.push((line, text, types[c[3] as usize]));
    }
    assert!(
        seen.contains(&(3, "/Common/api_pool".to_owned(), "object")),
        "the command after the escaped brace is still iRules: {seen:?}"
    );
    assert!(
        !seen
            .iter()
            .any(|(line, _, ty)| *line == 3 && *ty == "property"),
        "no config token inside the rule body: {seen:?}"
    );
}

/// The first command of a BOM'd Tcl 9 file must be tokenised as
/// if the mark were not there: it is the script prologue `source` strips, not
/// part of the command name.
///
/// tclsh-proof: tclsh8.6.14 rejects a BOM'd first command
/// (`invalid command name "<BOM>proc"`), which is why the skip is Tcl 9 only.
#[test]
fn tcl9_leading_bom_is_not_part_of_the_first_command_token() {
    let src = "\u{FEFF}proc greet {} { return 1 }\n";
    let toks = decode(src, "tcl9.0");
    let head = toks
        .iter()
        .find(|t| t.line == 0 && t.ttype == "keyword")
        .unwrap_or_else(|| panic!("no keyword token on line 0: {toks:?}"));
    assert_eq!(
        (head.character, head.length),
        (1, u32::try_from("proc".len()).expect("fits")),
        "the head token starts after the mark and spans `proc` alone: {toks:?}",
    );
    // The whole stream matches the mark-less file, shifted by the mark's one
    // UTF-16 unit — nothing else about the tokenisation changes.
    let plain = decode(&src["\u{FEFF}".len()..], "tcl9.0");
    let shifted: Vec<(u32, u32, u32, &str, u32)> = plain
        .iter()
        .map(|t| {
            (
                t.line,
                if t.line == 0 {
                    t.character + 1
                } else {
                    t.character
                },
                t.length,
                t.ttype.as_str(),
                t.mods,
            )
        })
        .collect();
    let actual: Vec<(u32, u32, u32, &str, u32)> = toks
        .iter()
        .map(|t| (t.line, t.character, t.length, t.ttype.as_str(), t.mods))
        .collect();
    assert_eq!(actual, shifted, "BOM'd tokens = mark-less tokens, shifted");
}

/// TN control — Tcl 8.x's `source` does **not** strip the mark, so the first
/// command really is named `<BOM>proc` there and must not be highlighted as
/// the `proc` keyword.
#[test]
fn tcl86_leading_bom_stays_part_of_the_first_command_token() {
    let src = "\u{FEFF}proc greet {} { return 1 }\n";
    let toks = decode(src, "tcl8.6");
    assert!(
        !toks
            .iter()
            .any(|t| t.line == 0 && t.character == 1 && t.ttype == "keyword"),
        "8.x keeps the mark in the command name: {toks:?}",
    );
}
