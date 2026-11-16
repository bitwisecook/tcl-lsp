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

//! Command heads: computed and variable heads, operators, keywords, the default-library modifier and qualified names.

use super::*;

#[test]
fn computed_command_head_does_not_overlap() {
    // `chartV$node SetOptions …` — a command head containing a `$node`
    // substitution is a *computed* (non-static) command name, so it is not
    // painted as a single command token.  Its fragments tokenise
    // individually (`$node` as a variable) and must not overlap each other
    // (LSP clients reject overlapping semantic tokens).
    let toks = decode_full("chartV$node SetOptions -x {}\n", tcl(), &reg());
    for w in toks.windows(2) {
        let (l0, c0, len0, ..) = w[0];
        let (l1, c1, ..) = w[1];
        if l0 == l1 {
            assert!(
                c1 >= c0 + len0,
                "overlap: token at col {c1} starts before prev end {}; toks={toks:?}",
                c0 + len0
            );
        }
    }
    // The `$node` fragment inside the head reads as a variable — the head
    // is not swallowed into one command-head token (which would hide the
    // substitution and mislabel a dynamic command as a resolved one).
    assert!(
        toks.iter()
            .any(|&(l, c, _, k, _)| l == 0 && c == 6 && k == TokenKind::Variable as u32),
        "expected a `$node` variable fragment at col 6; got {toks:?}"
    );
    // No token is a `Function` command head spanning the computed word.
    assert!(
        !toks
            .iter()
            .any(|&(l, c, _, k, _)| l == 0 && c == 0 && k == TokenKind::Function as u32),
        "computed head must not emit a function command token; got {toks:?}"
    );
}

#[test]
fn command_substitution_head_recurses_not_command_token() {
    // `[dict get $Pins $pin] configure -node $node` — the head
    // is a `[…]` command substitution, a runtime-computed command name, not
    // a resolvable command.  It must recurse into its inner script (`dict`
    // as a builtin, `get` as its subcommand, `$Pins` / `$pin` as variables)
    // rather than paint the whole `[…]` word as one function command token.
    let src = "[dict get $Pins $pin] configure -node $node\n";
    let toks = decode_full(src, tcl(), &reg());
    // `dict` inside the substitution head is a builtin function token.
    assert!(
        toks.iter().any(|&(l, c, _, k, m)| l == 0
            && c == 1
            && k == TokenKind::Function as u32
            && m == MOD_DEFAULT_LIBRARY),
        "expected `dict` builtin token at col 1; got {toks:?}"
    );
    // `$Pins` inside the head is a variable, not swallowed by a command token.
    assert!(
        toks.iter()
            .any(|&(l, c, _, k, _)| l == 0 && c == 10 && k == TokenKind::Variable as u32),
        "expected `$Pins` variable at col 10; got {toks:?}"
    );
    // Nothing paints the computed head as a function command token at col 0.
    assert!(
        !toks
            .iter()
            .any(|&(l, c, _, k, _)| l == 0 && c == 0 && k == TokenKind::Function as u32),
        "the `[…]` head must not be a function command token; got {toks:?}"
    );
}

#[test]
fn variable_command_head_is_a_variable() {
    // `$obj configure -node $node` — an object-handle dispatch whose class
    // is unknown.  The `$obj` head is a variable substitution, not a
    // command name, so it reads as a variable rather than a function token
    // (the `$chart` object-dispatch shape).
    let src = "$obj configure -node $node\n";
    let toks = decode_full(src, tcl(), &reg());
    assert!(
        toks.iter()
            .any(|&(l, c, _, k, _)| l == 0 && c == 0 && k == TokenKind::Variable as u32),
        "expected `$obj` head to read as a variable; got {toks:?}"
    );
    assert!(
        !toks
            .iter()
            .any(|&(l, c, _, k, _)| l == 0 && c == 0 && k == TokenKind::Function as u32),
        "the `$obj` head must not be a function command token; got {toks:?}"
    );
}

#[test]
fn operator_command_head_classified_as_operator() {
    // `+ 3 4` — the operator head is `operator`, not `function`.
    let ks = kinds("+ 3 4\n", tcl(), &reg());
    assert_eq!(ks.first(), Some(&(TokenKind::Operator as u32)), "{ks:?}");
}

/// A hand-typed symbol list would miss every word-form comparison operator
/// — `eq 1 1` (a bare `::tcl::mathop::eq` invocation via `namespace
/// import`) classifying as `function` rather than `operator`. Also covers
/// the TIP 461 `lt`/`le`/`gt`/`ge` mathop commands (9.0+).
#[test]
fn word_form_operator_command_heads_classified_as_operator() {
    for op in ["eq", "ne", "in", "ni", "lt", "le", "gt", "ge"] {
        let src = format!("{op} 1 1\n");
        let ks = kinds(
            &src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &reg(),
        );
        assert_eq!(
            ks.first(),
            Some(&(TokenKind::Operator as u32)),
            "{op}: {ks:?}"
        );
    }
}

#[test]
fn builtin_command_head_gets_default_library_modifier() {
    // `puts` is a registry built-in classified as `function`, so its
    // head token carries the `defaultLibrary` modifier (bit 3 = 8).
    let s = full("puts hi\n", tcl(), &reg());
    assert_eq!(s.data[3], TokenKind::Function as u32, "{:?}", s.data);
    assert_eq!(s.data[4], MOD_DEFAULT_LIBRARY, "{:?}", s.data);
}

#[test]
fn user_proc_head_has_no_default_library_modifier() {
    // A user-defined command isn't in the registry → `function`
    // with no modifier.
    let s = full("my_custom_cmd 1 2\n", tcl(), &reg());
    assert_eq!(s.data[3], TokenKind::Function as u32, "{:?}", s.data);
    assert_eq!(s.data[4], 0, "{:?}", s.data);
}

#[test]
fn keyword_head_has_no_default_library_modifier() {
    // `if` is a language keyword, not a `function` — no defaultLibrary.
    let s = full("if {1} { puts hi }\n", tcl(), &reg());
    assert_eq!(s.data[3], TokenKind::Keyword as u32, "{:?}", s.data);
    assert_eq!(s.data[4], 0, "{:?}", s.data);
}

#[test]
fn keywords_classified_as_keyword() {
    let s = full("if {1} { puts hi }\n", tcl(), &reg());
    // First token's type index should be 0 (Keyword) for `if`.
    // The encoded data: [deltaLine, deltaCol, length, type, modifiers].
    assert_eq!(s.data[3], TokenKind::Keyword as u32, "{:?}", s.data);
}

fn keyword_words(src: &str, registry: &CommandRegistry) -> std::collections::HashSet<String> {
    decode_words(src, registry)
        .into_iter()
        .filter(|(_, _, _, kind, _)| *kind == TokenKind::Keyword as u32)
        .map(|(_, _, _, _, word)| word)
        .collect()
}

#[test]
fn if_else_elseif_are_keywords() {
    // else/elseif structural keywords highlight like `if`.
    let src = "if 1 {\n puts a\n} elseif 2 {\n puts b\n} else {\n puts c\n}";
    let kw = keyword_words(src, &reg());
    for expected in ["if", "elseif", "else"] {
        assert!(kw.contains(expected), "missing {expected:?} in {kw:?}");
    }
}

#[test]
fn try_on_finally_are_keywords() {
    // try's on/trap/finally structural keywords highlight as keywords.
    let src = "try {\n set x 1\n} on error {e} {\n puts $e\n} finally {\n puts d\n}";
    let kw = keyword_words(src, &reg());
    for expected in ["try", "on", "finally"] {
        assert!(kw.contains(expected), "missing {expected:?} in {kw:?}");
    }
}

#[test]
fn original_keyword_contents_preserve_literal_closing_bytes() {
    let config = tcl_lexer::LexerConfig::for_profile(Some(tcl()));
    for (source, expected) in [
        ("{else{}}", "else{}"),
        ("\"else}\"", "else}"),
        ("{else}", "else"),
    ] {
        let plan = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::document(source),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        let word = &plan.commands[0].words[0];
        let token = word.tokens()[0];
        assert_eq!(
            keyword_operand_content(source, token, Some(word), config)
                .unwrap()
                .1,
            expected
        );
        assert!(keyword_operand_content("foreign", token, Some(word), config).is_none());
    }
}

#[test]
fn quoted_structural_keyword_offsets_past_quote() {
    // A quoted `"else"` keyword marks `else`, not `"els`.
    let src = "if 0 {} \"else\" {puts ok}";
    let kw = decode_words(src, &reg())
        .into_iter()
        .find(|(_, col, _, kind, _)| *kind == TokenKind::Keyword as u32 && *col >= 8)
        .expect("a keyword token past the first word");
    assert_eq!(kw.4, "else", "{kw:?}");
}

#[test]
fn classify_command_head_picks_namespace_for_qualified() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let rows = decode_words("::string equal A a\nunknown arg", &reg());
    assert!(
        rows.iter()
            .any(|(_, _, _, kind, text)| *kind == TokenKind::Namespace as u32 && text == "::")
    );
    assert!(
        rows.iter()
            .any(|(_, _, _, kind, text)| *kind == TokenKind::Function as u32 && text == "unknown")
    );
}

#[test]
fn classify_command_head_follows_the_effective_identity() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let source =
        "interp alias {} each {} foreach\neach v {} {}\nproc foreach args {}\nforeach v {} {}";
    let rows = decode_words(source, &reg());
    assert!(
        rows.iter()
            .any(|(_, _, _, kind, text)| *kind == TokenKind::Keyword as u32 && text == "each")
    );
    assert!(!rows.iter().any(|(line, _, _, kind, text)| *line == 3
        && *kind == TokenKind::Keyword as u32
        && text == "foreach"));
}
