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

//! Depth coverage for the `tcl-lexer` public surface the existing lexer test
//! files skip: the lexer's configuration / recovery / warning paths,
//! `SourceMap` sub-lexing offsets and `token_text` stripping, the
//! `LineIndex` UTF-16 / offset round-trips and in-place edit, the
//! `ExprToken` lexer's checked / dialect / fallback branches, and the
//! `structural_index` accessors (`events`, `inert_spans`, `balance`).
//!
//! Read alongside `parse_lexer.rs`, `parse_expr.rs`,
//! `edge_structural.rs`, and `bug_repro.rs` — this file deliberately
//! avoids re-testing what they already pin and reaches the remaining
//! branches instead.
//!
//! ## What is C-Tcl-proven vs lexer-internal
//!
//! Where a tokenisation reflects an *observable* Tcl-lexing fact — how
//! Tcl splits words/commands, what a `\x` / `\u` / `\NNN` escape decodes
//! to, that `{` is verbatim, that an escaped space/semicolon does not end
//! a word, whether a string is a complete command — it is confirmed
//! against real `tclsh` (8.6 / 9.0) and cited with a `// tclsh:` note.
//! Token-*stream* shape and *span* geometry (which byte a `Cmd` span ends
//! on, the `EXPAND` zero-width marker, the order of structural-index
//! events) are lexer-internal contracts with no `tclsh` observable, so
//! they are asserted structurally without a citation.
//!
//! Tcl escape dialect (verified against tclsh, not assumed):
//!   `\xNN`  consumes **exactly** up to 2 hex digits — `\x411` is `A1`
//!           (tclsh: `string length "\x411"` -> 2, index 0 -> `A`).
//!   `\uNNNN` consumes 1-4 hex digits — `\u41` is `A`.
//!   `\NNN`  octal, 1-3 digits capped to a byte — `\400` is space + `0`.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use tcl_lexer::{
    BraceIndex, BracketIndex, ByteCol, ExprParenIndex, ExprTokenType, LexError, LexWarning, Lexer,
    LexerConfig, LineIndex, ParenBalance, SourceMap, SourcePosition, Token, TokenType, Utf16Col,
    backslash_subst, command_boundaries, reparse_window, script_is_complete, tokenise_expr,
    tokenise_expr_checked, word_closer_offset, word_end_position,
};

// Shared helpers — mirror the sibling test files' `lex` / `kinds` shape.

/// Non-trivia tokens (drops SEP / EOL / EOF / COMMENT), text-resolved.
fn words(source: &str) -> Vec<(TokenType, String)> {
    let toks = Lexer::new(source).tokenise_all().expect("lexes");
    let sm = SourceMap::new(source);
    toks.into_iter()
        .filter(|t| {
            !matches!(
                t.kind,
                TokenType::Sep | TokenType::Eol | TokenType::Eof | TokenType::Comment
            )
        })
        .map(|t| (t.kind, sm.token_text(t).to_string()))
        .collect()
}

/// Every token kind in stream order (unfiltered).
fn all_kinds(source: &str) -> Vec<TokenType> {
    Lexer::new(source)
        .tokenise_all()
        .expect("lexes")
        .into_iter()
        .map(|t| t.kind)
        .collect()
}

/// Lex with a config and collect tokens + warnings.
fn lex_with(source: &str, config: LexerConfig) -> (Vec<Token>, Vec<LexWarning>) {
    let sm = SourceMap::new(source);
    Lexer::with_source_map(sm, config)
        .tokenise_all_with_warnings()
        .expect("non-strict lex never errors")
}

fn subst(s: &str) -> String {
    backslash_subst(s).into_owned()
}

// LexerConfig presets and the dialect-gated `{*}` expansion.

#[test]
fn dialect_presets_set_expansion_and_separator_flags() {
    let c84 = LexerConfig::for_dialect("tcl8.4");
    assert!(!c84.expand_syntax);
    assert!(!c84.irules_brace_separator);

    let cir = LexerConfig::for_dialect("f5-irules");
    assert!(!cir.expand_syntax);
    assert!(cir.irules_brace_separator);

    // An unknown languageId falls back to Tcl-8.5+ defaults so a typo
    // never silently changes parsing.
    let cunk = LexerConfig::for_dialect("totally-made-up");
    assert!(cunk.expand_syntax);
    assert!(!cunk.irules_brace_separator);

    let def = LexerConfig::default();
    assert!(def.expand_syntax);
    assert!(!def.strict_quoting);
}

#[test]
fn tcl84_treats_brace_star_as_a_literal_braced_word() {
    // tclsh 8.4 has no `{*}` expansion: `list {*}x` is a 2-element list
    // (`{*}` is the literal one-char string `*`, then `x`). In 8.5+ the
    // same source expands. So under the 8.4 preset the lexer must NOT
    // emit an EXPAND token; the `{*}` is a braced STR "*".
    let sm = SourceMap::new("cmd {*}x");
    let toks = Lexer::with_source_map(sm, LexerConfig::for_dialect("tcl8.4"))
        .tokenise_all()
        .unwrap();
    let kinds: Vec<_> = toks.iter().map(|t| t.kind).collect();
    assert!(!kinds.contains(&TokenType::Expand));
    let sm2 = SourceMap::new("cmd {*}x");
    let strs: Vec<&str> = toks
        .iter()
        .filter(|t| t.kind == TokenType::Str)
        .map(|t| sm2.token_text(*t))
        .collect();
    assert_eq!(strs, ["*"]);

    // The default (8.5+) preset DOES expand the same source.
    assert!(all_kinds("cmd {*}x").contains(&TokenType::Expand));
}

#[test]
fn expand_marker_is_zero_width_at_the_brace() {
    // Lexer-internal: the EXPAND token is a zero-width marker anchored at
    // the `{`, so start == end. (No tclsh observable for span geometry.)
    let toks = Lexer::new("cmd {*}$v").tokenise_all().unwrap();
    let expand = toks
        .iter()
        .find(|t| t.kind == TokenType::Expand)
        .expect("an EXPAND token");
    assert!(expand.span.is_empty());
    assert_eq!(expand.span.start(), 4); // the `{` of `{*}`
}

// strict_quoting: the recovery paths that become hard errors.

/// Lex in strict mode and return the error (if any).
fn strict_error(source: &str) -> Option<LexError> {
    let sm = SourceMap::new(source);
    let cfg = LexerConfig {
        strict_quoting: true,
        ..LexerConfig::default()
    };
    Lexer::with_source_map(sm, cfg).tokenise_all().err()
}

#[test]
fn strict_quoting_rejects_unterminated_brace() {
    // tclsh: `info complete "set x \173"` (a bare `{`) -> 0 (incomplete).
    // In strict mode the lexer turns that recoverable state into an error.
    assert_eq!(
        strict_error("set x {abc"),
        Some(LexError::SyntaxError {
            message: "missing close-brace".to_string()
        })
    );
}

#[test]
fn strict_quoting_rejects_unterminated_bracket_and_quote() {
    // Both are "incomplete" to tclsh `info complete`; strict mode errors.
    assert_eq!(
        strict_error("set x [foo bar"),
        Some(LexError::SyntaxError {
            message: "missing close-bracket".to_string()
        })
    );
    assert_eq!(
        strict_error("set x \"hello"),
        Some(LexError::SyntaxError {
            message: "missing \"".to_string()
        })
    );
}

#[test]
fn strict_quoting_rejects_unterminated_braced_var_and_array() {
    assert_eq!(
        strict_error("${name"),
        Some(LexError::SyntaxError {
            message: "missing close-brace for variable name".to_string()
        })
    );
    assert_eq!(
        strict_error("$arr(idx"),
        Some(LexError::SyntaxError {
            message: "missing )".to_string()
        })
    );
}

#[test]
fn strict_quoting_rejects_extra_chars_after_close() {
    // tclsh: `info complete {a}b` style — a `}` or `"` immediately
    // followed by a non-separator is a parse error. Strict mode raises it.
    assert_eq!(
        strict_error("\"ab\"x"),
        Some(LexError::SyntaxError {
            message: "extra characters after close-quote".to_string()
        })
    );
    assert_eq!(
        strict_error("{ab}x"),
        Some(LexError::SyntaxError {
            message: "extra characters after close-brace".to_string()
        })
    );
}

#[test]
fn strict_quoting_accepts_well_formed_input() {
    // A fully-closed script must NOT error in strict mode.
    assert!(strict_error("set x {a b}\nputs \"hi\" ;# c").is_none());
    assert!(strict_error("if {[info exists x]} {puts $x}").is_none());
}

#[test]
fn strict_quoting_allows_separator_and_bracket_after_close_quote() {
    // A close-quote followed by a separator, `]`, or backslash-newline is
    // legal even in strict mode (these are the documented OK followers).
    assert!(strict_error("\"a\" b").is_none());
    assert!(strict_error("[set x \"a\"]").is_none());
    assert!(strict_error("\"a\"\\\nb").is_none());
}

// Non-strict warning collection.

#[test]
fn warnings_report_unterminated_brace_with_offset() {
    // Non-strict mode tokenises best-effort and records the recoverable
    // problem as a warning rather than failing.
    let (toks, warns) = lex_with("set x {abc", LexerConfig::default());
    assert!(!toks.is_empty());
    assert_eq!(warns.len(), 1);
    assert_eq!(warns[0].message, "missing close-brace");
    assert_eq!(warns[0].offset, 10); // at EOF, past `{abc`
}

#[test]
fn warnings_report_each_recovery_message() {
    for (src, want) in [
        ("set x [foo", "missing close-bracket"),
        ("\"open", "missing \""),
        ("${v", "missing close-brace for variable name"),
        ("$a(i", "missing )"),
        ("{ok}junk", "extra characters after close-brace"),
        ("\"ok\"junk", "extra characters after close-quote"),
    ] {
        let (_t, warns) = lex_with(src, LexerConfig::default());
        assert!(
            warns.iter().any(|w| w.message == want),
            "{src:?} should warn {want:?}, got {:?}",
            warns.iter().map(|w| &w.message).collect::<Vec<_>>()
        );
    }
}

#[test]
fn well_formed_input_collects_no_warnings() {
    let (_t, warns) = lex_with(
        "proc p {a b} {return [expr {$a + $b}]}",
        LexerConfig::default(),
    );
    assert!(warns.is_empty(), "unexpected warnings: {warns:?}");
}

#[test]
fn into_warnings_surfaces_recovery_after_manual_drain() {
    // Driving the iterator by hand and then `into_warnings` must still
    // yield the collected diagnostics.
    let mut lexer = Lexer::new("{oops");
    let mut count = 0;
    for r in lexer.by_ref() {
        r.expect("non-strict never errors");
        count += 1;
    }
    assert!(count > 0);
    let warns = lexer.into_warnings();
    assert_eq!(warns.len(), 1);
    assert_eq!(warns[0].message, "missing close-brace");
}

#[test]
fn warnings_accessor_borrows_without_consuming() {
    // `tokenise_all_with_warnings` consumes; `warnings()` borrows. Exercise
    // the borrow path via a partially-driven lexer.
    let mut lexer = Lexer::new("[unterminated");
    for r in lexer.by_ref() {
        r.unwrap();
    }
    assert_eq!(lexer.warnings().len(), 1);
    assert_eq!(lexer.warnings()[0].message, "missing close-bracket");
}

// iRules `}{` brace-word separator.

#[test]
fn irules_brace_separator_splits_adjacent_braced_words() {
    // In the iRules dialect `}{` at a brace boundary injects a zero-width
    // ghost SEP so the segmenter sees two words (`{b}` and `{c}`) rather
    // than an "extra characters after close-brace" error. This is an
    // iRules-only structural concession with no plain-Tcl observable, so
    // it is asserted structurally.
    let sm = SourceMap::new("a {b}{c}");
    let toks = Lexer::with_source_map(sm, LexerConfig::for_dialect("f5-irules"))
        .tokenise_all()
        .unwrap();
    let kinds: Vec<_> = toks.iter().map(|t| t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenType::Esc, // a
            TokenType::Sep, // space
            TokenType::Str, // {b}
            TokenType::Sep, // injected zero-width boundary
            TokenType::Str, // {c}
            TokenType::Eol,
        ]
    );
    // The injected SEP is zero-width (an empty span at the `}{` seam).
    let sep = toks
        .iter()
        .rev()
        .find(|t| t.kind == TokenType::Sep)
        .unwrap();
    assert!(sep.span.is_empty());

    // Under the default (non-iRules) dialect the same `}{` is instead an
    // "extra characters after close-brace" warning, not a split.
    let (_t, warns) = lex_with("a {b}{c}", LexerConfig::default());
    assert!(
        warns
            .iter()
            .any(|w| w.message == "extra characters after close-brace")
    );
}

// Ghost closing-delimiter recovery (`with_ghosts`).

#[test]
fn ghost_close_bracket_recovers_unterminated_command_sub() {
    // An E201-style recovery inserts a zero-width ghost `]` at EOF so the
    // unterminated `[foo` re-lexes as a *terminated* command without
    // shifting any real byte offsets. Lexer-internal recovery machinery —
    // no tclsh observable.
    let src = "set x [foo";
    let mut ghosts = BTreeMap::new();
    ghosts.insert(u32::try_from(src.len()).unwrap(), b']');
    let sm = SourceMap::new(src);
    let toks = Lexer::with_source_map(sm, LexerConfig::default())
        .with_ghosts(ghosts)
        .tokenise_all()
        .unwrap();
    // The `[foo` now closes — there is a CMD token and no warning.
    assert!(toks.iter().any(|t| t.kind == TokenType::Cmd));

    // Same source WITHOUT the ghost warns about the unterminated bracket.
    let (_t, warns) = lex_with(src, LexerConfig::default());
    assert!(warns.iter().any(|w| w.message == "missing close-bracket"));
}

#[test]
fn empty_ghost_map_is_the_normal_path() {
    // `with_ghosts(empty)` must be identical to the default lexing.
    let src = "puts $x";
    let plain = Lexer::new(src).tokenise_all().unwrap();
    let sm = SourceMap::new(src);
    let ghosted = Lexer::with_source_map(sm, LexerConfig::default())
        .with_ghosts(BTreeMap::new())
        .tokenise_all()
        .unwrap();
    assert_eq!(plain, ghosted);
}

// EOF / ghost-EOL behaviour and `source_map` round-trips.

#[test]
fn empty_source_yields_no_tokens() {
    assert!(Lexer::new("").tokenise_all().unwrap().is_empty());
    assert_eq!(all_kinds(""), Vec::<TokenType>::new());
}

#[test]
fn trailing_ghost_eol_is_added_once_for_unterminated_line() {
    // A source not ending in an EOL gets exactly one trailing ghost EOL;
    // a source already ending in `\n` does not get a second one.
    let no_nl = all_kinds("foo");
    assert_eq!(no_nl, vec![TokenType::Esc, TokenType::Eol]);
    let with_nl = all_kinds("foo\n");
    assert_eq!(with_nl, vec![TokenType::Esc, TokenType::Eol]);
    // The ghost EOL has an empty span at EOF.
    let toks = Lexer::new("foo").tokenise_all().unwrap();
    let eol = toks.last().unwrap();
    assert_eq!(eol.kind, TokenType::Eol);
    assert!(eol.span.is_empty());
    assert_eq!(eol.span.start(), 3);
}

#[test]
fn into_source_map_resolves_tokens_after_the_lexer_is_gone() {
    let lexer = Lexer::new("set value 42");
    let toks = lexer.source_map().clone();
    let _ = toks; // borrow path
    let lexer = Lexer::new("set value 42");
    let tokens = {
        let mut l = lexer;
        let mut v = Vec::new();
        for r in l.by_ref() {
            v.push(r.unwrap());
        }
        (v, l.into_source_map())
    };
    let (tokens, sm) = tokens;
    // Resolve the first word through the moved-out SourceMap.
    let first = tokens[0];
    assert_eq!(sm.text(first.span), "set");
}

// SourceMap: token_text stripping per kind, and sub-lexing base offsets.

#[test]
fn token_text_strips_wrapper_delimiters_per_kind() {
    // VAR strips `$` / `${`; CMD strips `[`; STR strips `{`; quoted-opening
    // ESC strips `"`. The *raw* text keeps the delimiters; `token_text`
    // returns the human-readable inner content.
    let cases = [
        ("$abc", TokenType::Var, "abc"),
        ("${a b}", TokenType::Var, "a b"),
        ("[cmd arg]", TokenType::Cmd, "cmd arg"),
        ("{braced}", TokenType::Str, "braced"),
        ("\"quoted\"", TokenType::Esc, "quoted"),
    ];
    for (src, kind, inner) in cases {
        let toks = Lexer::new(src).tokenise_all().unwrap();
        let sm = SourceMap::new(src);
        let tok = toks
            .iter()
            .find(|t| t.kind == kind)
            .unwrap_or_else(|| panic!("{src}: no {kind:?} token"));
        assert_eq!(sm.token_text(*tok), inner, "token_text for {src}");
        // The raw span still includes the opening delimiter.
        assert!(
            sm.text(tok.span).len() >= inner.len(),
            "raw text for {src} keeps delimiters"
        );
    }
}

#[test]
fn token_text_empty_clamp_for_degenerate_wrappers() {
    // The empty `{}` / `[]` / `""` / `${}` forms return "" from
    // token_text even though their span was widened to cover the closer.
    // tclsh: `string length {}` -> 0, `string length ""` -> 0.
    for (src, kind) in [
        ("{}", TokenType::Str),
        ("[]", TokenType::Cmd),
        ("\"\"", TokenType::Esc),
        ("${}", TokenType::Var),
    ] {
        let toks = Lexer::new(src).tokenise_all().unwrap();
        let sm = SourceMap::new(src);
        let tok = toks.iter().find(|t| t.kind == kind).unwrap();
        assert_eq!(sm.token_text(*tok), "", "{src} token_text should be empty");
    }
}

#[test]
fn token_text_keeps_a_real_closer_that_ends_the_content() {
    // A non-degenerate braced var whose name legitimately ends in `}` via
    // nesting (`${a{b}}` names `a{b}`) must NOT be clamped to empty. This is
    // the **Tcl 9** rule (the default config): 9.0.1's `Tcl_ParseVarName`
    // tracks nested braces. The 8.x family instead closes at the FIRST `}`
    // (`a{b` is the name there) — covered by the dialect-gated test below.
    let toks = Lexer::new("${a{b}}").tokenise_all().unwrap();
    let sm = SourceMap::new("${a{b}}");
    let var = toks.iter().find(|t| t.kind == TokenType::Var).unwrap();
    assert_eq!(sm.token_text(*var), "a{b}");
}

#[test]
fn braced_var_delimiting_is_dialect_gated() {
    // Tcl 8.x (tclsh8.6-verified): `${a{b}c}` reads the variable `a{b` —
    // the name ends at the FIRST literal `}`; `c}` is ordinary word text.
    // `\}` does not escape the closer either: `${a\}b}` reads `a\`.
    for (src, want_name) in [("${a{b}c}", "a{b"), (r"${a\}b}", r"a\")] {
        let toks = Lexer::with_source_map(SourceMap::new(src), LexerConfig::for_dialect("tcl8.6"))
            .tokenise_all()
            .unwrap();
        let sm = SourceMap::new(src);
        let var = toks.iter().find(|t| t.kind == TokenType::Var).unwrap();
        assert_eq!(
            sm.token_text(*var),
            want_name,
            "8.x first-close rule for {src}"
        );
    }

    // Tcl 9 (default / tcl9.0): the same sources track nesting and consume
    // `\X` pairs, so the whole braced body is the name.
    for (src, want_name) in [("${a{b}c}", "a{b}c"), (r"${a\}b}", r"a\}b")] {
        let toks = Lexer::with_source_map(SourceMap::new(src), LexerConfig::for_dialect("tcl9.0"))
            .tokenise_all()
            .unwrap();
        let sm = SourceMap::new(src);
        let var = toks.iter().find(|t| t.kind == TokenType::Var).unwrap();
        assert_eq!(sm.token_text(*var), want_name, "9.x nesting rule for {src}");
    }
}

#[test]
fn bare_var_names_are_ascii_only() {
    // `TclIsBareword` is ASCII-only in every Tcl (8.6.14 and 9.0.1,
    // tclsh8.6-verified: `$café` reads variable `caf`, `é` is word text; a
    // `$` before a non-ASCII letter is a literal `$`).
    let toks = Lexer::new("$café").tokenise_all().unwrap();
    let sm = SourceMap::new("$café");
    let var = toks.iter().find(|t| t.kind == TokenType::Var).unwrap();
    assert_eq!(sm.token_text(*var), "caf");

    // `$变量` — no bareword char after `$`: the `$` is a literal string.
    let toks2 = Lexer::new("$变量").tokenise_all().unwrap();
    assert!(
        toks2.iter().all(|t| t.kind != TokenType::Var),
        "`$` before a non-ASCII letter must not start a variable: {toks2:?}"
    );
}

#[test]
fn source_map_base_offsets_shift_resolved_positions() {
    // Sub-lexing a fragment: base offset/line/col are added to resolved
    // positions so a fragment's tokens read relative to the parent doc.
    // base_col applies only to the fragment's first line.
    let sm = SourceMap::new("xyz").with_base(100, 5, 10);
    assert_eq!(
        sm.position_at(0),
        SourcePosition::new(5, ByteCol::new(10), 100)
    );
    assert_eq!(
        sm.position_at(1),
        SourcePosition::new(5, ByteCol::new(11), 101)
    );

    // Across a newline inside the fragment, the column resets and base_col
    // is NOT re-applied (it only offsets line 0 of the fragment).
    let sm2 = SourceMap::new("a\nb").with_base(100, 5, 10);
    assert_eq!(
        sm2.position_at(2),
        SourcePosition::new(6, ByteCol::new(0), 102)
    );
}

#[test]
fn source_map_with_line_index_reuses_the_index() {
    let src = "one\ntwo\nthree";
    let idx = LineIndex::new(src);
    let sm = SourceMap::with_line_index(src, idx);
    assert_eq!(sm.source(), src);
    assert_eq!(sm.line_index().line_count(), 3);
    // A position on line 2 resolves correctly through the shared index.
    assert_eq!(sm.position_at(8).line, 2);
}

// word_closer_offset / word_end_position: a few branches the ranges.rs
// unit tests reach, re-pinned through the public re-export.

#[test]
fn word_closer_offset_lands_on_the_real_delimiter() {
    // Non-empty braced word: closer is one byte past the inclusive end.
    let src = "{abc}";
    let sm = SourceMap::new(src);
    let tok = Lexer::new(src).tokenise_all().unwrap()[0];
    let off = word_closer_offset(&sm, tok).unwrap();
    assert_eq!(src.as_bytes()[off as usize], b'}');

    // A bare word has no closer.
    let bare = "plain";
    let sm = SourceMap::new(bare);
    let tok = Lexer::new(bare).tokenise_all().unwrap()[0];
    assert!(word_closer_offset(&sm, tok).is_none());
}

#[test]
fn word_end_position_advances_over_a_multiline_closer() {
    // A braced body ending in a newline puts the `}` at column 0 of the
    // next line — exercises the closer_position newline branch.
    let src = "{a\n}";
    let sm = SourceMap::new(src);
    let tok = Lexer::new(src).tokenise_all().unwrap()[0];
    assert_eq!(tok.kind, TokenType::Str);
    let pos = word_end_position(&sm, tok);
    assert_eq!(pos, SourcePosition::new(1, ByteCol::new(0), 3));
    assert_eq!(src.as_bytes()[pos.offset as usize], b'}');
}

// backslash_subst: the escape-decode contract, against tclsh ground truth.

#[test]
fn hex_escape_consumes_exactly_two_digits() {
    // tclsh: `string length "\x41"` -> 1 and `"\x41" eq "A"` -> 1.
    assert_eq!(subst(r"\x41"), "A");
    // tclsh: `string length "\x411"` -> 2; index 0 -> `A`. So `\x` stops
    // after two hex digits and the trailing `1` is literal.
    assert_eq!(subst(r"\x411"), "A1");
    // tclsh: first char of `"\x1g"` is U+0001 (scan %c -> 1), then `g`.
    assert_eq!(subst(r"\x1g"), "\u{01}g");
    // tclsh: `"\xg" eq "xg"` -> 1 (no hex digit, the `x` is literal).
    assert_eq!(subst(r"\xg"), "xg");
}

#[test]
fn unicode_and_wide_unicode_escapes_decode() {
    // tclsh: `"\u41" eq "A"` -> 1 (fewer than 4 digits is fine).
    assert_eq!(subst(r"\u41"), "A");
    // tclsh: `"é" eq "é"` -> 1.
    assert_eq!(subst(r"é"), "é");
    // tclsh: `string length "\U0001F600"` -> 1 (one supplementary char).
    assert_eq!(subst(r"\U0001F600"), "\u{1F600}");
}

#[test]
fn octal_escape_caps_at_a_byte() {
    // tclsh: `"\101" eq "A"` -> 1.
    assert_eq!(subst(r"\101"), "A");
    // tclsh: `string length "\400"` -> 2; first char scan %c -> 32 (space),
    // i.e. only `\40` (= space) is the escape, then a literal `0`.
    assert_eq!(subst(r"\400"), " 0");
}

#[test]
fn unknown_escape_passes_the_following_char_through() {
    // tclsh: `"\q" eq "q"` -> 1.
    assert_eq!(subst(r"\q"), "q");
    // Backslash-free input is returned untouched (zero-copy borrow path).
    assert_eq!(subst("no escapes here"), "no escapes here");
    // A multi-byte char after an unknown `\` survives intact.
    assert_eq!(subst("\\é"), "é");
}

// Word / command splitting facts, proven against tclsh.

#[test]
fn escaped_space_is_one_word() {
    // tclsh: `proc cnt args {llength $args}; cnt hello\ world` -> 1.
    // The escaped space keeps the two halves in a single command word.
    let w = words("hello\\ world");
    assert_eq!(w.len(), 1);
    assert_eq!(w[0], (TokenType::Esc, "hello\\ world".to_string()));
}

#[test]
fn escaped_semicolon_does_not_end_the_command() {
    // tclsh: `cnt a\;b` -> 1 (one word; the `;` is escaped, not an EOL).
    // So `a\;b` is a single ESC word, and the only EOL is the trailing
    // ghost — no `;` ever lexes as an end-of-line separator.
    let src = "a\\;b";
    let sm = SourceMap::new(src);
    let toks = Lexer::new(src).tokenise_all().unwrap();
    assert!(
        !toks
            .iter()
            .any(|t| t.kind == TokenType::Eol && sm.text(t.span).contains(';'))
    );
    let w = words(src);
    assert_eq!(w, [(TokenType::Esc, "a\\;b".to_string())]);
}

#[test]
fn braces_suppress_dollar_substitution() {
    // tclsh: `string index {$b} 0` -> `$` — inside braces `$b` is verbatim
    // data, not a variable. So a braced word is one STR whose text keeps
    // the `$`, with no VAR token emitted.
    let w = words("set k {$b}");
    assert!(w.iter().any(|(k, t)| *k == TokenType::Str && t == "$b"));
    assert!(!w.iter().any(|(k, _)| *k == TokenType::Var));
}

#[test]
fn backslash_newline_continuation_decodes_to_a_single_space() {
    // tclsh: a `\<newline>` line continuation collapses to one space, and
    // any run of leading whitespace on the continued line is folded into
    // it — so `"a\<nl>b"` decodes to the string "a b" (and tclsh
    // `llength "a\<nl>b"` -> 2, two elements of that joined string).
    assert_eq!(subst("a\\\nb"), "a b");
    assert_eq!(subst("a\\\n   b"), "a b"); // leading run after NL stripped
    // CRLF and bare-CR continuations collapse the same way.
    assert_eq!(subst("a\\\r\nb"), "a b");
    assert_eq!(subst("a\\\rb"), "a b");
}

// Comment rules (`#` only at command start; `\<nl>` continues a comment).

#[test]
fn hash_is_a_comment_only_at_command_start() {
    // tclsh: `string length #foo` runs fine — `#foo` mid-line is a data
    // word, not a comment. A `#` in command position starts a comment.
    assert!(all_kinds("# a comment").contains(&TokenType::Comment));
    assert!(!all_kinds("puts #notcomment").contains(&TokenType::Comment));
    // A `#` after leading whitespace is still at command start.
    assert!(all_kinds("   # spaced comment").contains(&TokenType::Comment));
}

#[test]
fn backslash_newline_continues_a_comment_swallowing_the_next_line() {
    // tclsh-proven (per parse_lexer.rs): a `\` at end of a comment
    // line continues the comment onto the next physical line, swallowing
    // it. So the `puts` here is part of the comment, and only `set x 1`
    // survives as a command.
    let src = "# TODO fix \\\nputs SHOULD-NOT-RUN\nset x 1";
    let toks = Lexer::new(src).tokenise_all().unwrap();
    let sm = SourceMap::new(src);
    let comment = toks
        .iter()
        .find(|t| t.kind == TokenType::Comment)
        .expect("a comment token");
    let text = sm.text(comment.span);
    assert!(
        text.contains("puts SHOULD-NOT-RUN"),
        "comment swallows the line"
    );
    assert!(words(src).iter().any(|(_, t)| t == "set"));
}

// Quoted-string substitution context.

#[test]
fn quotes_substitute_var_and_command_unlike_braces() {
    // tclsh: inside `"..."` a `$var` and `[cmd]` ARE substituted (unlike
    // braces). The lexer emits VAR / CMD sub-tokens carrying in_quote.
    let toks = Lexer::new("\"v=$x [f]\"").tokenise_all().unwrap();
    let kinds: Vec<_> = toks.iter().map(|t| t.kind).collect();
    assert!(kinds.contains(&TokenType::Var));
    assert!(kinds.contains(&TokenType::Cmd));
    // The VAR sub-token is flagged in_quote.
    let var = toks.iter().find(|t| t.kind == TokenType::Var).unwrap();
    assert!(var.in_quote);
}

// LineIndex: UTF-16 columns, offset round-trips, in-place edit, accessors.

#[test]
fn line_index_utf16_column_counts_code_units_not_bytes() {
    // `é` is U+00E9 — 2 UTF-8 bytes but 1 UTF-16 code unit. In `aébc`,
    // the byte offset of `b` is 3; the byte column there is 3, the UTF-16
    // column is 2. At offset 4 (`c`) byte col is 4, UTF-16 col is 3.
    let src = "aébc";
    let idx = LineIndex::new(src);
    assert_eq!(idx.position_at(3).character.get(), 3);
    assert_eq!(idx.position_at_utf16(3, src).character.get(), 2);
    assert_eq!(idx.position_at_utf16(4, src).character.get(), 3);
}

#[test]
fn line_index_utf16_counts_two_units_for_supplementary_plane() {
    // U+1F600 is 4 UTF-8 bytes and 2 UTF-16 code units (a surrogate pair).
    let src = "a\u{1F600}b";
    let idx = LineIndex::new(src);
    assert_eq!(idx.position_at_utf16(5, src).character.get(), 3); // 1 + 2
}

#[test]
fn line_index_utf16_clamps_offset_past_eof() {
    // An offset past EOF clamps the column to the line's full UTF-16
    // length while preserving the raw offset.
    let src = "abc";
    let idx = LineIndex::new(src);
    let len = u32::try_from(src.len()).unwrap();
    let pos = idx.position_at_utf16(len + 2, src);
    assert_eq!(pos.character.get(), len);
    assert_eq!(pos.offset, len + 2);
}

#[test]
fn line_index_offset_at_utf16_round_trips_and_clamps() {
    let src = "ab\ncde\nf";
    let idx = LineIndex::new(src);
    // (line, char) -> byte offset.
    assert_eq!(idx.offset_at_utf16(0, Utf16Col::new(0), src), 0);
    assert_eq!(idx.offset_at_utf16(1, Utf16Col::new(0), src), 3);
    // Past the line content clamps to the content end (before the `\n`).
    assert_eq!(idx.offset_at_utf16(0, Utf16Col::new(99), src), 2);
    // A line past EOF clamps to the source length.
    assert_eq!(
        idx.offset_at_utf16(9, Utf16Col::new(0), src),
        u32::try_from(src.len()).unwrap()
    );
    // Round-trips with position_at_utf16 at char boundaries.
    for off in [0u32, 1, 3, 5, 6, 7] {
        let p = idx.position_at_utf16(off, src);
        assert_eq!(
            idx.offset_at_utf16(p.line, p.character, src),
            off,
            "off {off}"
        );
    }
}

#[test]
fn line_index_offset_at_utf16_counts_surrogate_pairs() {
    // 'a' + U+1F600 (2 units) + 'b': char 3 lands on 'b' at byte 5.
    let src = "a\u{1F600}b";
    let idx = LineIndex::new(src);
    assert_eq!(idx.offset_at_utf16(0, Utf16Col::new(1), src), 1); // emoji start
    assert_eq!(idx.offset_at_utf16(0, Utf16Col::new(3), src), 5); // 'b'
}

#[test]
fn line_index_apply_edit_matches_a_rebuild() {
    // Patch in place for an insertion, a deletion that merges lines, and a
    // replacement at a boundary — each must equal a fresh rebuild.
    let starts = |idx: &LineIndex| {
        (0..idx.line_count())
            .map(|l| idx.line_start(u32::try_from(l).unwrap()))
            .collect::<Vec<_>>()
    };

    // Insert "X\nY" at offset 3 (just before the original `\n`): the
    // splice is "abc" + "X\nY" + "\ndef" = "abcX\nY\ndef" (two newlines).
    let mut idx = LineIndex::new("abc\ndef");
    idx.apply_edit(3, 3, "X\nY");
    assert_eq!(starts(&idx), starts(&LineIndex::new("abcX\nY\ndef")));

    let mut idx = LineIndex::new("ab\ncd\nef");
    idx.apply_edit(2, 6, ""); // delete "\ncd\n" -> collapse three lines to one
    assert_eq!(idx.line_count(), 1);
    assert_eq!(starts(&idx), starts(&LineIndex::new("abef")));

    let mut idx = LineIndex::new("a\nb\nc");
    idx.apply_edit(2, 2, "Z\n"); // insert at a line boundary
    assert_eq!(starts(&idx), starts(&LineIndex::new("a\nZ\nb\nc")));
}

#[test]
fn line_index_line_at_is_encoding_independent() {
    // `line_at` agrees with both column variants' `.line`, even across a
    // multi-byte char (the line number must not depend on column units).
    let src = "á\nbc\nd";
    let idx = LineIndex::new(src);
    for off in [0u32, 2, 3, 5, 6] {
        assert_eq!(
            idx.line_at(off),
            idx.position_at(off).line,
            "byte off {off}"
        );
        assert_eq!(
            idx.line_at(off),
            idx.position_at_utf16(off, src).line,
            "utf16 off {off}"
        );
    }
}

#[test]
fn line_index_bare_cr_is_not_a_line_break() {
    // In Tcl a lone `\r` is horizontal whitespace, not an EOL, and the
    // index counts only `\n`. So `abc\rdef` is one line.
    let idx = LineIndex::new("abc\rdef");
    assert_eq!(idx.line_count(), 1);
    assert_eq!(
        idx.position_at(4),
        SourcePosition::new(0, ByteCol::new(4), 4)
    );
    // CRLF still breaks (on its LF): `abc\r\ndef` is two lines.
    let idx = LineIndex::new("abc\r\ndef");
    assert_eq!(idx.line_count(), 2);
    assert_eq!(idx.line_start(1), 5);
}

// Expression lexer: checked / dialect / fallback branches.

/// Non-whitespace expr tokens (kind, text).
fn expr_pairs(source: &str) -> Vec<(ExprTokenType, String)> {
    tokenise_expr(source, None)
        .into_iter()
        .filter(|t| t.kind != ExprTokenType::Whitespace)
        .map(|t| (t.kind, t.text))
        .collect()
}

#[test]
fn expr_checked_reports_unknown_characters() {
    // `@` is not valid expr syntax: tokenise_expr skips it, and the
    // checked variant flags that an unknown char was seen.
    let (_t, unknown) = tokenise_expr_checked("1 @ 2", None);
    assert!(unknown);
    // A clean expression reports no unknowns.
    let (_t, unknown) = tokenise_expr_checked("1 + 2", None);
    assert!(!unknown);
    // Backslash-newline is whitespace, not unknown (tclsh 9 continuation).
    let (_t, unknown) = tokenise_expr_checked("1 \\\n+ 2", None);
    assert!(!unknown);
    // A lone backslash (not before a newline) stays unknown.
    let (_t, unknown) = tokenise_expr_checked("1 \\ 2", None);
    assert!(unknown);
}

#[test]
fn expr_f5_irules_dialect_promotes_word_operators() {
    // In the f5-irules dialect, words like `and` / `or` / `contains` are
    // operators; under default Tcl they are plain identifiers (Function).
    let f5 = tokenise_expr("$a and $b", Some("f5-irules"));
    let and = f5.iter().find(|t| t.text == "and").unwrap();
    assert_eq!(and.kind, ExprTokenType::Operator);

    let default = tokenise_expr("$a and $b", None);
    let and = default.iter().find(|t| t.text == "and").unwrap();
    assert_eq!(and.kind, ExprTokenType::Function);

    // A non-iRules word operator is unaffected by the dialect.
    for op in ["or", "not", "contains", "starts_with", "matches_regex"] {
        let src = format!("$a {op} $b");
        let toks = tokenise_expr(&src, Some("f5-irules"));
        let found = toks.iter().find(|t| t.text == op).unwrap();
        assert_eq!(found.kind, ExprTokenType::Operator, "{op}");
    }
}

#[test]
fn expr_unterminated_brace_falls_back_to_a_one_char_string() {
    // tclsh: `info complete "expr {1 + 2"` -> 0 (incomplete). The expr
    // lexer's braced() scanner, finding no matching `}`, rewinds and emits
    // a one-char STRING `{`, then continues tokenising the body.
    let pairs = expr_pairs("{1 + 2");
    assert_eq!(pairs[0], (ExprTokenType::String, "{".to_string()));
    // The remainder still tokenises as numbers and an operator.
    assert!(
        pairs
            .iter()
            .any(|(k, t)| *k == ExprTokenType::Number && t == "1")
    );
    assert!(
        pairs
            .iter()
            .any(|(k, t)| *k == ExprTokenType::Operator && t == "+")
    );

    // A *balanced* brace is one STRING covering the whole `{...}`.
    let balanced = expr_pairs("{1 + 2}");
    assert_eq!(
        balanced,
        vec![(ExprTokenType::String, "{1 + 2}".to_string())]
    );
}

#[test]
fn expr_math_function_set_is_exposed() {
    // The exported function set lets consumers detect shadowed math funcs.
    let funcs = tcl_lexer::expr_math_functions();
    for name in [
        "sin", "cos", "sqrt", "pow", "atan2", "isqrt", "isinf", "isnan",
    ] {
        assert!(funcs.contains(name), "{name} missing from math_functions");
    }
    // It is a non-trivial set.
    assert!(funcs.len() > 25);
}

/// Regression coverage for issue #996: the expr lexer's
/// `Inner::scan_array_index` self-recurses once per nested `$name(…)`
/// array-index reference, with no depth cap before this fix.
/// Empirically, unguarded input overflowed the native stack (SIGABRT)
/// between depth 100,000-200,000 on a 2 MiB thread (`cargo test`'s
/// per-test default) — bypassing the Pratt parser's own `MAX_EXPR_DEPTH`
/// cap entirely, since the whole nested chain is swallowed into one
/// `Variable` token during lexing. 5000 is comfortably past
/// `MAX_EXPR_ARRAY_INDEX_DEPTH` (64); the assertion is that tokenising
/// returns at all, not what it returns.
#[test]
fn deeply_nested_expr_array_index_survives_lexing() {
    const DEPTH: usize = 5000;
    let mut src = String::from("$a0");
    for i in 0..DEPTH {
        src.push('(');
        let _ = write!(src, "a{}", i + 1);
    }
    src.push('1');
    for _ in 0..DEPTH {
        src.push(')');
    }
    let _ = tokenise_expr(&src, None);
}

#[test]
fn expr_unterminated_quote_and_command_do_not_panic() {
    // tclsh treats these as incomplete; the lexer must still produce
    // tokens best-effort without panicking.
    assert!(!tokenise_expr("\"open string", None).is_empty());
    assert!(!tokenise_expr("[open cmd", None).is_empty());
    // A `$arr(` with an unterminated index runs to EOF as one VARIABLE.
    let v = tokenise_expr("$arr(idx", None);
    assert_eq!(v[0].kind, ExprTokenType::Variable);
}

// structural_index: the accessors and balance verdicts the edge tests
// leave unread (`events`, `inert_spans`, `ParenBalance`).

#[test]
fn bracket_index_records_structural_events_and_skips_inert() {
    // `a [b] c` — one structural `[` (+1) at offset 2 and `]` (-1) at 4;
    // no inert spans. The verdict is balanced.
    let idx = BracketIndex::build("a [b] c");
    assert_eq!(idx.events(), &[(2, 1), (4, -1)]);
    assert!(idx.inert_spans().is_empty());
    assert_eq!(idx.unterminated_count(), 0);

    // `{ [ }` — the `[` is inside a brace word, so it is literal: zero
    // structural events and one terminated inert span covering the word.
    let idx = BracketIndex::build("{ [ }");
    assert!(idx.events().is_empty());
    assert_eq!(idx.inert_spans(), &[(0, 5, true)]);
    assert!(idx.is_inert(2)); // the `[` sits inside the inert span
    // `is_inert` is membership `start <= off < end`: offset 0 (the `{`) is
    // in the span, but offset 5 (== end, past the `}`) is not.
    assert!(idx.is_inert(0));
    assert!(!idx.is_inert(5));
}

#[test]
fn bracket_index_escape_pair_and_unterminated_brace_spans() {
    // A `\[` escape pair is a terminated inert span: the `[` is literal.
    let idx = BracketIndex::build(r"\[");
    assert_eq!(idx.inert_spans(), &[(0, 2, true)]);
    assert!(idx.events().is_empty());

    // `[foo {bar` — the `[` is structural (+1 at 0), but the unterminated
    // brace word makes a `(5, EOF, false)` span that swallows the tail.
    let idx = BracketIndex::build("[foo {bar");
    assert_eq!(idx.events(), &[(0, 1)]);
    assert_eq!(idx.inert_spans(), &[(5, 9, false)]);
    assert_eq!(idx.unterminated_count(), 1);
}

#[test]
fn brace_index_records_events_and_unterminated_depth() {
    // `set x {a b}` — structural `{` (+1) at 6 and `}` (-1) at 10.
    let idx = BraceIndex::build("set x {a b}");
    assert_eq!(idx.events(), &[(6, 1), (10, -1)]);
    assert!(idx.inert_spans().is_empty());
    assert_eq!(idx.unterminated_count(), 0);

    // `{a {b` — two opens, both unterminated: depth 2.
    let idx = BraceIndex::build("{a {b");
    assert_eq!(idx.events(), &[(0, 1), (3, 1)]);
    assert_eq!(idx.unterminated_count(), 2);
    // A `}` inserted at EOF still leaves one unterminated brace.
    assert!(!idx.close_brace_balances(5));
}

#[test]
fn expr_paren_balance_classifies_the_three_states() {
    // Balanced grouping parens.
    let idx = ExprParenIndex::build("(1 + 2) * 3");
    assert_eq!(idx.balance(), ParenBalance::Balanced);
    assert_eq!(idx.unmatched_opens(), 0);

    // Open-heavy: a grouping `(` with no closer. The `$a(b)` paren is part
    // of a VARIABLE token, so it is inert and does not count.
    let idx = ExprParenIndex::build("$a(b) + (c");
    assert_eq!(idx.balance(), ParenBalance::OpenHeavy);
    assert_eq!(idx.unmatched_opens(), 1);
    // Inserting one `)` at EOF balances it.
    assert!(idx.close_paren_balances(u32::try_from("$a(b) + (c".len()).unwrap()));

    // Close-heavy: a `)` appears before any opener.
    let idx = ExprParenIndex::build("1 + 2)");
    assert_eq!(idx.balance(), ParenBalance::CloseHeavy);
    assert_eq!(idx.unmatched_opens(), 0);
}

#[test]
fn expr_paren_string_interior_is_opaque() {
    // tclsh: `expr {"a(b"}` is a valid balanced expr — the `(` inside the
    // string literal is not a grouping paren. The index treats the whole
    // string token as inert.
    let idx = ExprParenIndex::build(r#""a(b""#);
    assert_eq!(idx.balance(), ParenBalance::Balanced);
}

// script_is_complete / command_boundaries / reparse_window — proven against
// tclsh `info complete` where applicable.

#[test]
fn script_is_complete_matches_info_complete() {
    // tclsh `info complete` ground truth:
    //   {puts hello}                -> 1 (complete)
    //   "set x \173"  (= `set x {`) -> 0 (incomplete)
    //   "set x \"a"   (= `set x "a`)-> 0 (incomplete)
    //   {set x \\}    trailing `\`  -> 0 (line continuation, incomplete)
    assert!(script_is_complete("puts hello"));
    assert!(script_is_complete("set x 1\nputs $x\n"));
    assert!(!script_is_complete("set x {"));
    assert!(!script_is_complete("set x \"a"));
    assert!(!script_is_complete("puts hello\\\n")); // trailing continuation

    // A `[...]` interior parses recursively as a script, so `[set x {b}{`
    // is complete (the `[` is a terminal extra-chars error -> complete),
    // matching tclsh's documented `info complete` behaviour.
    assert!(script_is_complete("[set x {b}{]"));
}

#[test]
fn command_boundaries_split_only_top_level_separators() {
    // "set x 1\nputs hi" — boundaries at the end of each command (the
    // separator the lexer emits). Nested newlines inside braces / command
    // subs do not split.
    assert_eq!(command_boundaries("set x 1\nputs hi"), vec![8, 15]);
    let src = "if {[a]\n b} {\n c\n}\nset y 1";
    let b = command_boundaries(src);
    assert_eq!(*b.last().unwrap(), u32::try_from(src.len()).unwrap());
    // The `if` body's interior newlines did not create extra boundaries:
    // only two top-level commands.
    assert_eq!(b.len(), 2);
}

/// Regression coverage for issue #996: `BracketIndex`/`BraceIndex`'s
/// `scan_cmd_sub`/`scan_script`/`scan_quoted`, and the
/// `script_is_complete`/`command_boundaries` scanner's
/// `scan_complete`/`scan_complete_quoted`, all recurse once per nested
/// `[...]`, with no depth cap before this fix. Empirically, unguarded
/// input overflowed the native stack (SIGABRT) around depth 10,000-50,000
/// on a 2 MiB thread (`cargo test`'s per-test default). 5000 is
/// comfortably past both that crash range and `MAX_NESTED_BRACKET_DEPTH`
/// (128); the assertion is that each call returns at all, not what it
/// returns.
#[test]
fn deeply_nested_brackets_survive_structural_scanning() {
    const DEPTH: usize = 5000;
    let mut src = String::from("set x ");
    for _ in 0..DEPTH {
        src.push('[');
    }
    src.push_str("a 1");
    for _ in 0..DEPTH {
        src.push(']');
    }
    src.push('\n');

    let _ = script_is_complete(&src);
    let _ = command_boundaries(&src);
    let _ = BracketIndex::build(&src);
    let _ = BraceIndex::build(&src);
}

/// Same regression, but nested inside a quoted string — exercises
/// `scan_quoted`/`scan_complete_quoted`'s own recursive `[` dispatch
/// rather than the top-level script scanner's.
#[test]
fn deeply_nested_brackets_in_a_quoted_string_survive_structural_scanning() {
    const DEPTH: usize = 5000;
    let mut src = String::from("set x \"");
    for _ in 0..DEPTH {
        src.push('[');
    }
    src.push_str("a 1");
    for _ in 0..DEPTH {
        src.push(']');
    }
    src.push_str("\"\n");

    let _ = script_is_complete(&src);
    let _ = command_boundaries(&src);
    let _ = BracketIndex::build(&src);
    let _ = BraceIndex::build(&src);
}

/// A moderately nested bracket chain (well under
/// `MAX_NESTED_BRACKET_DEPTH`) is still classified exactly as before — the
/// safety net must not fire on realistic nesting depths.
#[test]
fn moderately_nested_brackets_still_scan_correctly() {
    let src = "set x [a [b [c 1]]]\n";
    assert!(script_is_complete(src));
    let idx = BracketIndex::build(src);
    assert_eq!(idx.unterminated_count(), 0);
}

#[test]
fn reparse_window_snaps_to_command_extents() {
    let src = "set x 1\nputs hi\nset y 2\n";
    // An edit inside the middle command snaps to that whole command.
    let (s, e) = reparse_window(src, 9, 9);
    assert_eq!(&src[s as usize..e as usize], "puts hi\n");
    // An edit spanning two commands snaps to cover both.
    let (s, e) = reparse_window(src, 6, 13);
    assert_eq!(&src[s as usize..e as usize], "set x 1\nputs hi\n");
    // An edit entirely past EOF clamps to an empty window (nothing in the
    // source actually changed).
    let past = "set x 1\n";
    let (s, e) = reparse_window(past, 20, 30);
    assert_eq!(e, s, "past-EOF edit must produce an empty window");
    assert_eq!(&past[s as usize..e as usize], "");
}
