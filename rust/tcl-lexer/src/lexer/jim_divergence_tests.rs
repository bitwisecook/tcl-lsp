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

use super::*;
use crate::tokens::TokenType;
use tcl_dialect::model::{Family, Release, grammar};

fn cfg(family: Family, release: Release) -> LexerConfig {
    LexerConfig::from_grammar(grammar(family, release))
}

fn jim() -> LexerConfig {
    cfg(Family::Jim, Release::JIM_0_84)
}

fn tcl() -> LexerConfig {
    cfg(Family::Tcl, Release::TCL_8_6)
}

fn lex(source: &str, config: LexerConfig) -> (Vec<(TokenType, String)>, Vec<String>) {
    let mut lexer = Lexer::with_config(source, config);
    let mut out = Vec::new();
    while let Some(Ok(t)) = lexer.next() {
        out.push((t.kind, source[t.span.as_range()].to_string()));
    }
    let warns = lexer.warnings().iter().map(|w| w.message.clone()).collect();
    (out, warns)
}

fn kinds(source: &str, config: LexerConfig) -> Vec<(TokenType, String)> {
    lex(source, config).0
}

/// `$(…)` is an expression substitution in Jim and a variable named
/// `(…)` in every build of the Tcl core.
#[test]
fn expr_sugar_is_jim_only() {
    let src = "set a $($b * 2)";
    for release in Family::Jim.releases() {
        assert!(
            kinds(src, cfg(Family::Jim, *release))
                .contains(&(TokenType::ExprSugar, "$($b * 2".into())),
            "{release}"
        );
    }
    for release in Family::Tcl.releases() {
        let toks = kinds(src, cfg(Family::Tcl, *release));
        assert!(
            toks.contains(&(TokenType::Var, "$($b * 2)".into())),
            "{release} reads $( as a variable name"
        );
        assert!(
            !toks.iter().any(|(k, _)| *k == TokenType::ExprSugar),
            "{release} never emits an expression-sugar token"
        );
    }
}

/// The sugar body nests and honours a backslash, like Jim's scanner.
#[test]
fn expr_sugar_body_nests() {
    assert!(
        kinds("puts $(($a+1)*2)", jim()).contains(&(TokenType::ExprSugar, "$(($a+1)*2".into()))
    );
    assert!(kinds(r"puts $(a\)b)", jim()).contains(&(TokenType::ExprSugar, r"$(a\)b".into())));
}

/// `$name(index)` counts nested parens in Jim; C Tcl stops at the first
/// `)` at token level.
#[test]
fn index_parens_nest_only_in_jim() {
    assert!(kinds("puts $d(a(b))", jim()).contains(&(TokenType::Var, "$d(a(b))".into())));
    assert!(kinds("puts $d(a(b))", tcl()).contains(&(TokenType::Var, "$d(a(b)".into())));
}

/// Jim's recovery for an unbalanced index: end just after the last `)`
/// rather than run to end of source.
#[test]
fn unbalanced_index_backs_up_to_the_last_close_paren() {
    let toks = kinds("puts $a((b) tail", jim());
    assert!(
        toks.contains(&(TokenType::Var, "$a((b)".into())),
        "{toks:?}"
    );
    assert!(toks.contains(&(TokenType::Esc, "tail".into())), "{toks:?}");
}

/// A bare `$name` may hold any byte >= 0x80 in Jim.
#[test]
fn high_bytes_continue_a_name_only_in_jim() {
    assert!(kinds("puts $café", jim()).contains(&(TokenType::Var, "$café".into())));
    assert!(kinds("puts $café", tcl()).contains(&(TokenType::Var, "$caf".into())));
}

/// `\v` separates words in C Tcl and is an ordinary word character in
/// Jim: `eval "f a\vb"` passes 2 arguments under tclsh 8.6 and 1 under
/// every modelled Jim release.
#[test]
fn vertical_tab_is_a_separator_only_in_tcl() {
    assert!(
        kinds("set x a\u{0B}b", jim()).contains(&(TokenType::Esc, "a\u{0B}b".into())),
        "jim keeps the vertical tab inside the word"
    );
    assert!(
        kinds("set x a\u{0B}b", tcl()).contains(&(TokenType::Sep, "\u{0B}".into())),
        "tcl splits on the vertical tab"
    );
    // The three shared separators behave identically either way.
    for ws in [" ", "\t", "\u{0C}"] {
        for config in [jim(), tcl()] {
            assert!(
                kinds(&format!("set x a{ws}b"), config)
                    .iter()
                    .any(|(k, _)| *k == TokenType::Sep),
                "{ws:?}"
            );
        }
    }
    // A Document's lone CR does not. C Tcl's `source` reads it as LF
    // (`-translation auto`), so it ends the command. Jim's `source` reads
    // the bytes unchanged, so it stays a word separator.
    assert!(
        kinds("set x a\rb", jim()).contains(&(TokenType::Sep, "\r".into())),
        "jim keeps the lone CR as a separator"
    );
    assert!(
        kinds("set x a\rb", tcl()).contains(&(TokenType::Eol, "\r".into())),
        "tcl ends the command at the lone CR"
    );
}

/// `puts "abc"def` is one word in Jim and a parse error in C Tcl. The
/// token stream is the same either way — only the diagnostic differs.
#[test]
fn extra_characters_after_close_quote_is_tcl_only() {
    let (jim_toks, jim_warns) = lex("puts \"abc\"def", jim());
    let (tcl_toks, tcl_warns) = lex("puts \"abc\"def", tcl());
    assert_eq!(jim_toks, tcl_toks, "the word boundaries agree");
    assert!(jim_warns.is_empty(), "jim has no such check: {jim_warns:?}");
    assert_eq!(tcl_warns, vec!["extra characters after close-quote"]);
}

/// The brace twin of the rule above is *not* on this axis: Jim rejects
/// `{abc}def` exactly as C Tcl does, so that diagnostic is unconditional.
#[test]
fn extra_characters_after_close_brace_still_fires_for_jim() {
    for config in [jim(), tcl()] {
        assert_eq!(
            lex("puts {abc}def", config).1,
            vec!["extra characters after close-brace"]
        );
    }
}
