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
use std::fmt::Write as _;

use crate::line_index::LineIndex;
use crate::tokens::{ByteCol, SourcePosition};

struct Lexed<'src> {
    source_map: SourceMap<'src>,
    tokens: Vec<Token>,
}

impl<'src> Lexed<'src> {
    fn run(source: &'src str) -> Self {
        let lexer = Lexer::new(source);
        let source_map = lexer.source_map().clone();
        let tokens = lexer.tokenise_all().expect("L3 lexer accepts fixture");
        Self { source_map, tokens }
    }

    fn kinds(&self) -> Vec<TokenType> {
        self.tokens.iter().map(|t| t.kind).collect()
    }

    fn texts(&self) -> Vec<&'src str> {
        self.tokens
            .iter()
            .map(|t| self.source_map.text(t.span))
            .collect()
    }

    fn positions(&self, idx: usize) -> (SourcePosition, SourcePosition) {
        self.source_map.range_positions(self.tokens[idx].span)
    }
}

#[test]
fn empty_source_produces_no_tokens() {
    let lexed = Lexed::run("");
    assert_eq!(lexed.tokens, [] as [Token; 0]);
}

#[test]
fn single_word_emits_esc_and_trailing_eol() {
    let lexed = Lexed::run("foo");
    assert_eq!(lexed.tokens.len(), 2);
    assert_eq!(lexed.tokens[0].kind, TokenType::Esc);
    assert_eq!(lexed.source_map.text(lexed.tokens[0].span), "foo");
    assert_eq!(lexed.tokens[1].kind, TokenType::Eol);
    assert_eq!(lexed.source_map.text(lexed.tokens[1].span), "");
}

#[test]
fn two_words_separated_by_space() {
    let lexed = Lexed::run("foo bar");
    assert_eq!(
        lexed.kinds(),
        vec![
            TokenType::Esc,
            TokenType::Sep,
            TokenType::Esc,
            TokenType::Eol,
        ]
    );
    assert_eq!(lexed.texts(), vec!["foo", " ", "bar", ""]);
}

#[test]
fn multiple_spaces_collapse_into_one_sep_token() {
    let lexed = Lexed::run("foo   bar");
    assert_eq!(lexed.texts(), vec!["foo", "   ", "bar", ""]);
}

#[test]
fn tab_separator() {
    let lexed = Lexed::run("foo\tbar");
    assert_eq!(lexed.texts(), vec!["foo", "\t", "bar", ""]);
}

#[test]
fn cr_is_separator_not_eol() {
    // Under `eval` (a native value) `\r` is a separator char, not an
    // end-of-line char. tclsh 8.4-9.1 `source` reads a Document's lone
    // `\r` as LF, so there it ends the command.
    let source = "foo\rbar";
    let kinds = |lexer: Lexer<'_>| {
        lexer
            .tokenise_all()
            .unwrap()
            .into_iter()
            .map(|token| token.kind)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        kinds(Lexer::with_bytes(source.as_bytes(), LexerConfig::default())),
        vec![
            TokenType::Esc,
            TokenType::Sep,
            TokenType::Esc,
            TokenType::Eol,
        ]
    );
    assert_eq!(
        kinds(Lexer::new(source)),
        vec![
            TokenType::Esc,
            TokenType::Eol,
            TokenType::Esc,
            TokenType::Eol,
        ]
    );
}

#[test]
fn lf_is_eol() {
    let lexed = Lexed::run("foo\nbar");
    assert_eq!(
        lexed.kinds(),
        vec![
            TokenType::Esc,
            TokenType::Eol,
            TokenType::Esc,
            TokenType::Eol,
        ]
    );
}

#[test]
fn semicolon_is_eol() {
    let lexed = Lexed::run("foo;bar");
    assert_eq!(
        lexed.kinds(),
        vec![
            TokenType::Esc,
            TokenType::Eol,
            TokenType::Esc,
            TokenType::Eol,
        ]
    );
}

#[test]
fn mixed_eol_and_whitespace_becomes_single_eol_token() {
    let lexed = Lexed::run("foo\n \t;\nbar");
    assert_eq!(
        lexed.kinds(),
        vec![
            TokenType::Esc,
            TokenType::Eol,
            TokenType::Esc,
            TokenType::Eol,
        ]
    );
    assert_eq!(lexed.source_map.text(lexed.tokens[1].span), "\n \t;\n");
}

#[test]
fn leading_whitespace_before_word() {
    let lexed = Lexed::run("  foo");
    assert_eq!(
        lexed.kinds(),
        vec![TokenType::Sep, TokenType::Esc, TokenType::Eol]
    );
}

#[test]
fn trailing_whitespace_still_emits_ghost_eol() {
    let lexed = Lexed::run("foo  ");
    assert_eq!(
        lexed.kinds(),
        vec![TokenType::Esc, TokenType::Sep, TokenType::Eol]
    );
    assert_eq!(lexed.source_map.text(lexed.tokens[2].span), "");
}

#[test]
fn trailing_newline_does_not_add_second_eol() {
    let lexed = Lexed::run("foo\n");
    assert_eq!(lexed.kinds(), vec![TokenType::Esc, TokenType::Eol]);
    assert_eq!(lexed.source_map.text(lexed.tokens[1].span), "\n");
}

#[test]
fn brace_word_budget_uses_lexical_context_and_escape_boundaries() {
    // Implementation contract: naming.editor.original-source-formatting-budget
    // docs/design/analysis/name-resolution-proofs/editor-original-source-formatting-budget.md
    let one = tcl_core_types::RecursionLimit(1);
    assert_eq!(
        Lexer::new("set value {a {b}}").braced_word_nesting_within(one),
        Ok(false)
    );
    assert_eq!(
        Lexer::new(r"set value {\{ \{ \} \}}").braced_word_nesting_within(one),
        Ok(true)
    );
    let zero = tcl_core_types::RecursionLimit(0);
    for source in [
        "# {{{{\nset value plain",
        "set value \"{{{{\"",
        "set value data{{{{",
    ] {
        assert_eq!(
            Lexer::new(source).braced_word_nesting_within(zero),
            Ok(true),
            "{source}"
        );
    }
    assert_eq!(
        Lexer::new("{a {b}}")
            .as_quoted_body()
            .braced_word_nesting_within(zero),
        Ok(true)
    );
}

#[test]
fn brace_word_budget_retains_full_config_and_strict_errors() {
    // Implementation contract: naming.editor.original-source-formatting-budget
    // docs/design/analysis/name-resolution-proofs/editor-original-source-formatting-budget.md
    let zero = tcl_core_types::RecursionLimit(0);
    let core =
        LexerConfig::for_file_grammar(tcl_dialect::DialectProfile::find("tcl8.6").unwrap().grammar);
    let jim = LexerConfig::for_file_grammar(tcl_dialect::grammar_of_dialect_name(Some("jim")));
    let source = "set value\u{000b}{{nested}}";
    assert_eq!(
        Lexer::with_config(source, core).braced_word_nesting_within(zero),
        Ok(false)
    );
    assert_eq!(
        Lexer::with_config(source, jim).braced_word_nesting_within(zero),
        Ok(true)
    );
    let malformed = "set value {ok}tail";
    assert_eq!(
        Lexer::with_config(malformed, core)
            .braced_word_nesting_within(tcl_core_types::RecursionLimit(1)),
        Ok(true)
    );
    let strict = LexerConfig {
        strict_quoting: true,
        ..core
    };
    assert!(
        Lexer::with_config(malformed, strict)
            .braced_word_nesting_within(tcl_core_types::RecursionLimit(1))
            .is_err()
    );
}

#[test]
fn comment_at_command_start() {
    let lexed = Lexed::run("# hello world");
    assert_eq!(lexed.tokens[0].kind, TokenType::Comment);
    assert_eq!(lexed.source_map.text(lexed.tokens[0].span), "# hello world");
}

#[test]
fn comment_terminated_by_newline() {
    let lexed = Lexed::run("# hello\nfoo");
    assert_eq!(lexed.tokens[0].kind, TokenType::Comment);
    assert_eq!(lexed.source_map.text(lexed.tokens[0].span), "# hello");
    assert_eq!(lexed.tokens[1].kind, TokenType::Eol);
    assert_eq!(lexed.source_map.text(lexed.tokens[1].span), "\n");
    assert_eq!(lexed.tokens[2].kind, TokenType::Esc);
    assert_eq!(lexed.source_map.text(lexed.tokens[2].span), "foo");
}

#[test]
fn comment_after_whitespace_at_command_start() {
    // Leading whitespace preserves `at_command_start`, so `#` is
    // still a comment.
    let lexed = Lexed::run("   # comment");
    assert_eq!(lexed.tokens[0].kind, TokenType::Sep);
    assert_eq!(lexed.tokens[1].kind, TokenType::Comment);
    assert_eq!(lexed.source_map.text(lexed.tokens[1].span), "# comment");
}

#[test]
fn hash_not_at_command_start_is_part_of_word() {
    let lexed = Lexed::run("foo #bar");
    assert_eq!(
        lexed.kinds(),
        vec![
            TokenType::Esc,
            TokenType::Sep,
            TokenType::Esc,
            TokenType::Eol,
        ]
    );
    assert_eq!(lexed.source_map.text(lexed.tokens[2].span), "#bar");
}

#[test]
fn two_commands_separated_by_eol_both_allow_comments() {
    let lexed = Lexed::run("foo\n# comment");
    assert!(lexed.tokens.iter().any(|t| t.kind == TokenType::Comment));
}

#[test]
fn position_tracking_simple_word() {
    let lexed = Lexed::run("foo");
    let (start, end) = lexed.positions(0);
    assert_eq!(start, SourcePosition::new(0, ByteCol::new(0), 0));
    assert_eq!(end, SourcePosition::new(0, ByteCol::new(2), 2));
}

#[test]
fn position_tracking_across_newline() {
    let lexed = Lexed::run("ab\ncd");
    // ESC "ab" at (0,0)-(0,1)
    let (start, end) = lexed.positions(0);
    assert_eq!(lexed.source_map.text(lexed.tokens[0].span), "ab");
    assert_eq!(start, SourcePosition::new(0, ByteCol::new(0), 0));
    assert_eq!(end, SourcePosition::new(0, ByteCol::new(1), 1));
    // EOL "\n" at (0,2)-(0,2)
    let (start, end) = lexed.positions(1);
    assert_eq!(lexed.tokens[1].kind, TokenType::Eol);
    assert_eq!(lexed.source_map.text(lexed.tokens[1].span), "\n");
    assert_eq!(start, SourcePosition::new(0, ByteCol::new(2), 2));
    assert_eq!(end, SourcePosition::new(0, ByteCol::new(2), 2));
    // ESC "cd" at (1,0)-(1,1)
    let (start, end) = lexed.positions(2);
    assert_eq!(lexed.source_map.text(lexed.tokens[2].span), "cd");
    assert_eq!(start, SourcePosition::new(1, ByteCol::new(0), 3));
    assert_eq!(end, SourcePosition::new(1, ByteCol::new(1), 4));
}

#[test]
fn spans_are_accurate() {
    let lexed = Lexed::run("foo bar");
    assert_eq!(lexed.tokens[0].span, Span::new(0, 3)); // "foo"
    assert_eq!(lexed.tokens[1].span, Span::new(3, 4)); // " "
    assert_eq!(lexed.tokens[2].span, Span::new(4, 7)); // "bar"
    assert!(lexed.tokens[3].span.is_empty()); // ghost EOL
}

#[test]
fn dollar_is_no_longer_an_unsupported_character() {
    // `$` is an ordinary lexable character: the lexer accepts `$bar`
    // as a VAR token, not an error.
    let tokens = Lexer::new("foo $bar").tokenise_all().unwrap();
    let kinds: Vec<TokenType> = tokens.iter().map(|t| t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenType::Esc,
            TokenType::Sep,
            TokenType::Var,
            TokenType::Eol,
        ]
    );
}

#[test]
fn brace_is_no_longer_an_unsupported_character() {
    // `{` and `}` are ordinary lexable characters: `foo {bar}` lexes
    // as ESC + SEP + STR + EOL.
    let tokens = Lexer::new("foo {bar}").tokenise_all().unwrap();
    let kinds: Vec<TokenType> = tokens.iter().map(|t| t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenType::Esc,
            TokenType::Sep,
            TokenType::Str,
            TokenType::Eol,
        ]
    );
}

#[test]
fn bracket_is_no_longer_an_unsupported_character() {
    // `[` is an ordinary lexable character: `[cmd]` lexes as a CMD
    // token, not an error.
    let tokens = Lexer::new("[cmd]").tokenise_all().unwrap();
    let kinds: Vec<TokenType> = tokens.iter().map(|t| t.kind).collect();
    assert_eq!(kinds, vec![TokenType::Cmd, TokenType::Eol]);
}

#[test]
fn backslash_in_bare_word_is_literal_pair() {
    // `foo\nbar` — `\n` (backslash + 'n') is a literal pair
    // inside a bare word, matching C Tcl's tokenisation.
    let lexed = Lexed::run(r"foo\nbar");
    assert_eq!(lexed.texts(), vec![r"foo\nbar", ""]);
}

#[test]
fn backslash_in_comment_continues_line() {
    // `# comment\<newline>continues` — the backslash-newline
    // continues the comment to the next line.
    let lexed = Lexed::run("# hello\\\nworld");
    assert_eq!(lexed.kinds(), vec![TokenType::Comment, TokenType::Eol]);
    assert_eq!(
        lexed.source_map.token_text(lexed.tokens[0]),
        "# hello\\\nworld"
    );
}

#[test]
fn backslash_in_bare_word_does_not_error() {
    // `\` is an ordinary lexable character inside a bare word.
    let lexed = Lexed::run(r"foo\nbar");
    assert_eq!(lexed.kinds(), vec![TokenType::Esc, TokenType::Eol]);
}

#[test]
fn shared_source_map_constructor() {
    let source = "foo\nbar";
    let map = SourceMap::new(source);
    let via_shared = Lexer::with_source_map(map, LexerConfig::default())
        .tokenise_all()
        .unwrap();
    let via_new = Lexer::new(source).tokenise_all().unwrap();
    assert_eq!(via_shared, via_new);
}

#[test]
fn shared_line_index_via_source_map() {
    let source = "alpha beta";
    let idx = LineIndex::new(source);
    let map = SourceMap::with_line_index(source, idx);
    let tokens = Lexer::with_source_map(map, LexerConfig::default())
        .tokenise_all()
        .unwrap();
    assert_ne!(tokens, [] as [Token; 0]);
}

// Variable substitution

fn var_token_text(source: &str) -> (Vec<(TokenType, String)>, SourceMap<'_>) {
    let lexer = Lexer::new(source);
    let map = lexer.source_map().clone();
    let tokens = lexer.tokenise_all().expect("L4 lexer accepts fixture");
    // `token_text` strips the leading `$` / `${` for VAR tokens
    // so the assertions check just the variable name.
    let rows = tokens
        .iter()
        .map(|t| (t.kind, map.token_text(*t).to_owned()))
        .collect();
    (rows, map)
}

#[test]
fn var_simple_identifier() {
    let (rows, _) = var_token_text("$foo");
    assert_eq!(
        rows,
        vec![
            (TokenType::Var, "foo".into()),
            (TokenType::Eol, String::new()),
        ]
    );
}

#[test]
fn var_with_underscore() {
    let (rows, _) = var_token_text("$_private");
    assert_eq!(rows[0], (TokenType::Var, "_private".into()));
}

#[test]
fn var_alphanumeric_accepts_digits_anywhere() {
    assert_eq!(
        var_token_text("$foo1").0[0],
        (TokenType::Var, "foo1".into())
    );
    // Digits are allowed at the start of variable names —
    // Tcl uses `$1`, `$2` etc. for regexp backrefs.
    assert_eq!(var_token_text("$1").0[0], (TokenType::Var, "1".into()));
}

#[test]
fn var_uppercase() {
    assert_eq!(var_token_text("$FOO").0[0], (TokenType::Var, "FOO".into()));
}

#[test]
fn var_namespace_separator() {
    let (rows, _) = var_token_text("$ns::var");
    assert_eq!(rows[0], (TokenType::Var, "ns::var".into()));
}

#[test]
fn var_multi_level_namespace() {
    let (rows, _) = var_token_text("$a::b::c");
    assert_eq!(rows[0], (TokenType::Var, "a::b::c".into()));
}

#[test]
fn var_leading_namespace() {
    // `$::global` — starts with `::` (double colon).
    let (rows, _) = var_token_text("$::global");
    assert_eq!(rows[0], (TokenType::Var, "::global".into()));
}

#[test]
fn var_consumes_entire_colon_run() {
    // `$a:::b` — C Tcl's `Tcl_ParseVarName` consumes the whole colon run
    // once a `::` starts it, so the variable is `a:::b`, not `a::` + `:b`.
    let (rows, _) = var_token_text("$a:::b");
    assert_eq!(rows[0], (TokenType::Var, "a:::b".into()));
    // Four colons likewise stay in the name.
    assert_eq!(
        var_token_text("$a::::b").0[0],
        (TokenType::Var, "a::::b".into())
    );
}

#[test]
fn var_single_colon_terminates_name() {
    // A single `:` is not part of the identifier; it ends the
    // VAR token and the rest becomes an ESC token.
    let (rows, _) = var_token_text("$foo:bar");
    assert_eq!(rows[0], (TokenType::Var, "foo".into()));
    assert_eq!(rows[1], (TokenType::Esc, ":bar".into()));
}

#[test]
fn var_braced_form() {
    let (rows, _) = var_token_text("${name}");
    // The braces are stripped — the token text is the body only.
    assert_eq!(rows[0], (TokenType::Var, "name".into()));
}

#[test]
fn var_braced_empty_body() {
    let (rows, _) = var_token_text("${}");
    assert_eq!(rows[0], (TokenType::Var, String::new()));
}

#[test]
fn var_braced_allows_arbitrary_characters() {
    // Inside `${…}` all characters except `}` are legal, including
    // spaces, `$`, `[`, etc.
    let (rows, _) = var_token_text("${weird name with spaces}");
    assert_eq!(rows[0], (TokenType::Var, "weird name with spaces".into()));
}

#[test]
fn var_braced_unterminated_tokenises_best_effort() {
    // Missing `}` tokenises the remaining input as the variable
    // name; the non-fatal warning is not yet emitted.
    let (rows, _) = var_token_text("${unterminated");
    assert_eq!(rows[0], (TokenType::Var, "unterminated".into()));
}

// `${name}` brace-name parsing under the DEFAULT config follows C Tcl
// 9.0.3's `Tcl_ParseVarName` (the project's reference standard — see
// `docs/design/rust/engineering-guide.md` principle #0): inner `{…}` nests with brace
// counting and `\X` is consumed as a literal pair, so the closer is
// the first `}` at brace-depth zero. The expectations below were
// confirmed against `tclsh9.0` (9.0.3) via the variable-name a failed
// read reports — e.g. `${a\}b}` reads var `a\}b`, `${a{b}c}` reads var
// `a{b}c`. Tcl 8.4/8.5/8.6 instead stop at the *first* `}` (their
// `Tcl_ParseVarName` is `while (numBytes && (*src != '}'))`, no brace
// counting, no backslash) — [`BracedVarStyle::FirstClose`], which every
// 8.x-runtime dialect profile selects via `LexerConfig::for_dialect`
// (dialect-profile-model.md).

#[test]
fn var_braced_escaped_close_brace_is_part_of_name() {
    // `${a\}b}` — the `\}` is a literal backslash + brace inside
    // the name, so the name is `a\}b` and the *real* closer is the
    // final `}`. Verified against tclsh 9.0.3.
    let (rows, _) = var_token_text(r"${a\}b}");
    assert_eq!(rows[0], (TokenType::Var, r"a\}b".into()));
    assert_eq!(rows.len(), 2); // VAR + synthetic EOL, no trailing ESC
}

#[test]
fn var_braced_nested_braces_balance() {
    // `${a{b}c}` — the inner `{…}` is balanced and consumed as part
    // of the name; the name is `a{b}c`. Verified against tclsh 9.0.3.
    let (rows, _) = var_token_text("${a{b}c}");
    assert_eq!(rows[0], (TokenType::Var, "a{b}c".into()));
    assert_eq!(rows.len(), 2);
}

#[test]
fn var_braced_deeply_nested_braces() {
    let (rows, _) = var_token_text("${a{b{c}d}e}");
    assert_eq!(rows[0], (TokenType::Var, "a{b{c}d}e".into()));
    assert_eq!(rows.len(), 2);
}

#[test]
fn var_braced_name_ending_in_brace() {
    // `${a{b}}` — the inner `{b}` balances, so the name is `a{b}` and it
    // *ends* with the inner `}`; the outer `}` is the closer.
    // `token_text` must keep that trailing `}`; stripping it
    // unconditionally would yield `a{b`. Verified against tclsh
    // 9.0.3 (`${a{b}}` reads var `a{b}`).
    let (rows, _) = var_token_text("${a{b}}");
    assert_eq!(rows[0], (TokenType::Var, "a{b}".into()));
    assert_eq!(rows.len(), 2); // VAR + synthetic EOL, no stray `}` word
}

#[test]
fn var_braced_newline_in_name() {
    // A bare newline inside `${…}` is ordinary name content (it is not
    // a delimiter and not a backslash). Name = `a\nb`.
    let (rows, _) = var_token_text("${a\nb}");
    assert_eq!(rows[0], (TokenType::Var, "a\nb".into()));
}

#[test]
fn var_braced_trailing_backslash_at_eof() {
    // A `\` with nothing after it is consumed best-effort without
    // panicking; the (unterminated) name is the lone backslash.
    let (rows, _) = var_token_text("${a\\");
    assert_eq!(rows[0], (TokenType::Var, "a\\".into()));
}

#[test]
fn var_array_index() {
    let (rows, _) = var_token_text("$arr(idx)");
    // Span covers the whole `arr(idx)` — including the parens —
    // but not the leading `$`.
    assert_eq!(rows[0], (TokenType::Var, "arr(idx)".into()));
}

#[test]
fn var_array_index_nested_parens() {
    // C Tcl does NOT nest parens in an array index — it
    // terminates at the first `)`. `$arr(one(two)three)` is the variable
    // `arr(one(two)` followed by the literal text `three)`.
    let (rows, _) = var_token_text("$arr(one(two)three)");
    assert_eq!(rows[0].0, TokenType::Var);
    assert_eq!(rows[0].1, "arr(one(two)");
    assert!(
        rows.iter().any(|(_, t)| t.contains("three")),
        "trailing `three)` must be a separate token: {rows:?}",
    );
}

#[test]
fn var_array_index_first_paren_terminates() {
    // `$a((b)` ends the variable at the first `)`; the paren-counting bug
    // never reached depth 0 and swallowed the rest of the source.
    let (rows, _) = var_token_text("$a((b) rest");
    assert_eq!(rows[0].0, TokenType::Var);
    assert_eq!(rows[0].1, "a((b)");
}

#[test]
fn var_array_index_escaped_paren_stays() {
    // `$a(x\)y)` — the escaped `)` is part of the index; the index ends at
    // the final, unescaped `)`.
    let (rows, _) = var_token_text("$a(x\\)y)");
    assert_eq!(rows[0].0, TokenType::Var);
    assert_eq!(rows[0].1, "a(x\\)y)");
}

#[test]
fn var_array_index_command_sub_parens_do_not_terminate() {
    // A `)` inside a `[…]` command substitution is not the terminator.
    let (rows, _) = var_token_text("$a([foo(x)])");
    assert_eq!(rows[0].0, TokenType::Var);
    assert_eq!(rows[0].1, "a([foo(x)])");
}

#[test]
fn var_array_index_command_sub_bracketed_brace_does_not_close_early() {
    // A `]` inside a braced word within the `[…]` substitution must not
    // close the substitution early (skip_command_in_index tracks brace
    // depth). `$a([puts {]}])` — the command is `puts {]}`, so the index is
    // the whole `[puts {]}]` and the VAR token is `a([puts {]}])`.
    let (rows, _) = var_token_text("$a([puts {]}])");
    assert_eq!(rows[0].0, TokenType::Var);
    assert_eq!(rows[0].1, "a([puts {]}])");
    // Same for a `]` inside a double-quoted word.
    let (rows, _) = var_token_text("$a([puts \"]\"])");
    assert_eq!(rows[0].0, TokenType::Var);
    assert_eq!(rows[0].1, "a([puts \"]\"])");
}

#[test]
fn var_array_index_with_inner_braced_var() {
    // `${key}` inside the index scans to the matching `}` as a
    // unit — the `(` / `)` inside such a braced name would not
    // count against the array-index depth, so a
    // variable-named-with-parens doesn't fool the index
    // scanner.
    let (rows, _) = var_token_text("$arr(${key})");
    assert_eq!(rows[0], (TokenType::Var, "arr(${key})".into()));
}

#[test]
fn var_array_index_unterminated_tokenises_best_effort() {
    let (rows, _) = var_token_text("$arr(idx");
    assert_eq!(rows[0], (TokenType::Var, "arr(idx".into()));
}

/// Tcl 9 rejects a literal brace in an array-index source,
/// while the 8.x grammar retains it as ordinary index text. The recovery
/// token is intentionally identical on both paths; only the modern path
/// carries the fatal compiler-facing warning.
#[test]
fn array_index_literal_brace_follows_the_release_rule() {
    let source = "$arr({key})";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let (tokens, warnings) =
            Lexer::with_source_map(SourceMap::new(source), LexerConfig::for_dialect(dialect))
                .tokenise_all_with_warnings()
                .expect("lexes recoverably");
        let variable = tokens
            .iter()
            .find(|token| token.kind == TokenType::Var)
            .expect("variable token");
        assert_eq!(SourceMap::new(source).token_text(*variable), "arr({key})");
        assert_eq!(
            warnings
                .iter()
                .any(|warning| warning.message == crate::INVALID_CHARACTER_IN_ARRAY_INDEX),
            dialect.starts_with("tcl9"),
            "{dialect}: {warnings:?}"
        );
    }
}

#[test]
fn array_index_substitution_and_escape_shield_tcl9_source_bytes() {
    for source in ["$arr(\\{key\\})", "$arr(${key})", "$arr([format \\{])"] {
        let (_tokens, warnings) =
            Lexer::with_source_map(SourceMap::new(source), LexerConfig::for_dialect("tcl9.0"))
                .tokenise_all_with_warnings()
                .expect("lexes");
        assert!(
            warnings
                .iter()
                .all(|warning| warning.message != crate::INVALID_CHARACTER_IN_ARRAY_INDEX),
            "{source}: {warnings:?}"
        );
    }
}

#[test]
fn bare_dollar_is_an_str_token() {
    // A bare `$` is emitted as an STR token whose text is the
    // `$` character — not a VAR.
    let (rows, _) = var_token_text("$");
    assert_eq!(rows[0], (TokenType::Str, "$".into()));
}

#[test]
fn bare_dollar_followed_by_space() {
    let (rows, _) = var_token_text("$ foo");
    assert_eq!(rows[0], (TokenType::Str, "$".into()));
    assert_eq!(rows[1], (TokenType::Sep, " ".into()));
    assert_eq!(rows[2], (TokenType::Esc, "foo".into()));
}

#[test]
fn bare_dollar_followed_by_lf() {
    let (rows, _) = var_token_text("$\n");
    assert_eq!(rows[0], (TokenType::Str, "$".into()));
    assert_eq!(rows[1], (TokenType::Eol, "\n".into()));
}

#[test]
fn var_followed_by_word() {
    // `$foo bar` — VAR then SEP then ESC.
    let (rows, _) = var_token_text("$foo bar");
    assert_eq!(rows[0], (TokenType::Var, "foo".into()));
    assert_eq!(rows[1], (TokenType::Sep, " ".into()));
    assert_eq!(rows[2], (TokenType::Esc, "bar".into()));
}

#[test]
fn multiple_vars() {
    let (rows, _) = var_token_text("$a $b $c");
    assert_eq!(rows[0], (TokenType::Var, "a".into()));
    assert_eq!(rows[2], (TokenType::Var, "b".into()));
    assert_eq!(rows[4], (TokenType::Var, "c".into()));
}

#[test]
fn var_resets_at_command_start() {
    // After a VAR token, `#` is no longer a comment opener.
    let (rows, _) = var_token_text("$foo #bar");
    assert_eq!(rows[0], (TokenType::Var, "foo".into()));
    assert_eq!(rows[1], (TokenType::Sep, " ".into()));
    // `#bar` should be an ESC, not a COMMENT.
    assert_eq!(rows[2], (TokenType::Esc, "#bar".into()));
}

#[test]
fn esc_stops_at_dollar() {
    // `foo$bar` — ESC "foo", VAR "bar". The `$` terminates the
    // bare word rather than being consumed as a literal.
    let (rows, _) = var_token_text("foo$bar");
    assert_eq!(rows[0], (TokenType::Esc, "foo".into()));
    assert_eq!(rows[1], (TokenType::Var, "bar".into()));
}

#[test]
fn var_span_positions() {
    // `$foo bar` — the VAR span covers the whole `$foo` (offset
    // 0..4): the span starts at the `$` and ends at the last char
    // of the name. `token_text` is how you get just the "foo"
    // part.
    let lexer = Lexer::new("$foo bar");
    let map = lexer.source_map().clone();
    let tokens = lexer.tokenise_all().unwrap();
    let var = tokens.iter().find(|t| t.kind == TokenType::Var).unwrap();
    assert_eq!(var.span.start(), 0);
    assert_eq!(var.span.end(), 4);
    let (start, end) = map.range_positions(var.span);
    assert_eq!(start, SourcePosition::new(0, ByteCol::new(0), 0));
    assert_eq!(end, SourcePosition::new(0, ByteCol::new(3), 3));
    assert_eq!(map.token_text(*var), "foo");
}

#[test]
fn braced_var_span_covers_delimiter_and_name() {
    // `${name}` — span is [0, 6), covering "${name" but NOT the
    // closing `}`. The lexer consumes the `}` so the next
    // dispatch starts at offset 7. `token_text` strips the `${`
    // wrapper so the visible text is "name".
    let lexer = Lexer::new("${name}");
    let map = lexer.source_map().clone();
    let tokens = lexer.tokenise_all().unwrap();
    let var = tokens.iter().find(|t| t.kind == TokenType::Var).unwrap();
    assert_eq!(var.span.start(), 0);
    assert_eq!(var.span.end(), 6);
    assert_eq!(map.text(var.span), "${name");
    assert_eq!(map.token_text(*var), "name");
}

#[test]
fn into_source_map_round_trip() {
    let source = "a\nb\nc";
    let lexer = Lexer::new(source);
    let map = lexer.into_source_map();
    assert_eq!(map.line_index().line_count(), 3);
}

// Command substitution

fn cmd_token_rows(source: &str) -> (Vec<(TokenType, String)>, SourceMap<'_>) {
    let lexer = Lexer::new(source);
    let map = lexer.source_map().clone();
    let tokens = lexer.tokenise_all().expect("L5 lexer accepts fixture");
    let rows = tokens
        .iter()
        .map(|t| (t.kind, map.token_text(*t).to_owned()))
        .collect();
    (rows, map)
}

#[test]
fn cmd_simple_body() {
    let (rows, _) = cmd_token_rows("[+ 1 2]");
    assert_eq!(
        rows,
        vec![
            (TokenType::Cmd, "+ 1 2".into()),
            (TokenType::Eol, String::new()),
        ]
    );
}

#[test]
fn cmd_empty_body() {
    // Degenerate `[]` — exercises the empty-body end-position clamp.
    let (rows, _) = cmd_token_rows("[]");
    assert_eq!(rows[0], (TokenType::Cmd, String::new()));
}

#[test]
fn cmd_nested_brackets() {
    let (rows, _) = cmd_token_rows("[+ 1 [+ 2 3]]");
    assert_eq!(rows[0], (TokenType::Cmd, "+ 1 [+ 2 3]".into()));
}

#[test]
fn cmd_braced_var_name_handles_escapes_and_nesting() {
    // `[set ${a\}] x}]` — the `${…}` braced variable name is `a\}] x`
    // (the `\}` is a literal pair, the `]` is inside the name). The
    // command-sub scan must sub-scan `${…}` with the same brace-nesting +
    // backslash-pair rules as `parse_var`, so the inner `]` does not close
    // the command early. One CMD token spans the whole thing.
    let (rows, _) = cmd_token_rows(r"[set ${a\}] x}]");
    assert_eq!(rows[0], (TokenType::Cmd, r"set ${a\}] x}".into()));
    assert_eq!(rows[1], (TokenType::Eol, String::new()));
    // Brace nesting inside `${…}` is also honoured.
    let (rows2, _) = cmd_token_rows(r"[set ${a{b}c} 1]");
    assert_eq!(rows2[0], (TokenType::Cmd, r"set ${a{b}c} 1".into()));
}

#[test]
fn cmd_deeply_nested_brackets() {
    let (rows, _) = cmd_token_rows("[a [b [c [d]]]]");
    assert_eq!(rows[0], (TokenType::Cmd, "a [b [c [d]]]".into()));
}

#[test]
fn cmd_followed_by_word() {
    let (rows, _) = cmd_token_rows("[cmd] tail");
    assert_eq!(rows[0], (TokenType::Cmd, "cmd".into()));
    assert_eq!(rows[1], (TokenType::Sep, " ".into()));
    assert_eq!(rows[2], (TokenType::Esc, "tail".into()));
}

#[test]
fn word_then_cmd() {
    // `foo[cmd]` — ESC then CMD. The `[` terminates the bare
    // word.
    let (rows, _) = cmd_token_rows("foo[cmd]");
    assert_eq!(rows[0], (TokenType::Esc, "foo".into()));
    assert_eq!(rows[1], (TokenType::Cmd, "cmd".into()));
}

#[test]
fn cmd_then_word() {
    let (rows, _) = cmd_token_rows("[cmd]tail");
    assert_eq!(rows[0], (TokenType::Cmd, "cmd".into()));
    assert_eq!(rows[1], (TokenType::Esc, "tail".into()));
}

#[test]
fn cmd_with_quoted_substring() {
    // `"..."` inside a command body is NOT a quoted string at
    // the top level — parse_command just tracks it for its
    // bracket-nesting state so that `]` inside the quotes does
    // not close the command.
    let (rows, _) = cmd_token_rows(r#"[puts "hello world"]"#);
    assert_eq!(rows[0], (TokenType::Cmd, r#"puts "hello world""#.into()));
}

#[test]
fn cmd_with_bracket_inside_quotes_does_not_close() {
    let (rows, _) = cmd_token_rows(r#"[puts "a]b"]"#);
    assert_eq!(rows[0], (TokenType::Cmd, r#"puts "a]b""#.into()));
}

#[test]
fn cmd_with_braced_substring() {
    // `{…}` inside a command body adjusts `blevel` so `]`
    // inside the braces does not close the command.
    let (rows, _) = cmd_token_rows("[list {a b c}]");
    assert_eq!(rows[0], (TokenType::Cmd, "list {a b c}".into()));
}

#[test]
fn cmd_with_bracket_inside_braces_does_not_close() {
    let (rows, _) = cmd_token_rows("[list {a ] b}]");
    assert_eq!(rows[0], (TokenType::Cmd, "list {a ] b}".into()));
}

#[test]
fn cmd_with_nested_braces() {
    let (rows, _) = cmd_token_rows("[list {a {nested} b}]");
    assert_eq!(rows[0], (TokenType::Cmd, "list {a {nested} b}".into()));
}

#[test]
fn cmd_with_backslash_escape() {
    // `\]` inside the body is inert — it doesn't close the
    // command. The backslash consumes two bytes as a pair.
    let (rows, _) = cmd_token_rows(r"[a \] b]");
    assert_eq!(rows[0], (TokenType::Cmd, r"a \] b".into()));
}

#[test]
fn cmd_with_backslash_quote() {
    // `\"` inside the body is inert — the quote state does not
    // toggle because the `"` is escaped.
    let (rows, _) = cmd_token_rows(r#"[a \" b]"#);
    assert_eq!(rows[0], (TokenType::Cmd, r#"a \" b"#.into()));
}

#[test]
fn cmd_with_dollar_braced_var_inside() {
    // `${...}` inside the command body is sub-scanned so a `}`
    // inside the braced name does not throw off the counter.
    let (rows, _) = cmd_token_rows("[set ${odd}name value]");
    assert_eq!(rows[0], (TokenType::Cmd, "set ${odd}name value".into()));
}

#[test]
fn cmd_with_plain_dollar_var_inside() {
    let (rows, _) = cmd_token_rows("[expr $a + $b]");
    assert_eq!(rows[0], (TokenType::Cmd, "expr $a + $b".into()));
}

#[test]
fn cmd_multiline_body() {
    let (rows, _) = cmd_token_rows("[a\nb\nc]");
    assert_eq!(rows[0], (TokenType::Cmd, "a\nb\nc".into()));
}

#[test]
fn cmd_comment_hides_closing_bracket() {
    // The `]` after `#` is in a command-position comment.  The command
    // token must therefore extend through the following command and end
    // at the final `]`.
    let (rows, _) = cmd_token_rows("[\n# ] hidden\nset y \"b\"\n]");
    assert_eq!(
        rows[0],
        (TokenType::Cmd, "\n# ] hidden\nset y \"b\"\n".into())
    );
}

#[test]
fn cmd_comment_after_separator_hides_closing_bracket() {
    let (rows, _) = cmd_token_rows("[set x 1; # ] hidden\nset y 2]");
    assert_eq!(
        rows[0],
        (TokenType::Cmd, "set x 1; # ] hidden\nset y 2".into())
    );
}

#[test]
fn cmd_hash_mid_command_is_not_a_comment() {
    // A hash after the first word is ordinary word content.  This keeps
    // the command-position guard from becoming an over-broad `#` rule.
    let (rows, _) = cmd_token_rows("[set #not-a-comment]");
    assert_eq!(rows[0], (TokenType::Cmd, "set #not-a-comment".into()));
}

#[test]
fn cmd_hash_in_quote_or_brace_is_not_a_comment() {
    let (rows, _) = cmd_token_rows("[\"# ]\"]");
    assert_eq!(rows[0], (TokenType::Cmd, "\"# ]\"".into()));
    let (rows, _) = cmd_token_rows("[{# ]}]");
    assert_eq!(rows[0], (TokenType::Cmd, "{# ]}".into()));
}

#[test]
fn cmd_comment_continuation_hides_closing_bracket() {
    let (rows, _) = cmd_token_rows("[\n# hidden \\\n] still comment\nset y 2]");
    assert_eq!(
        rows[0],
        (
            TokenType::Cmd,
            "\n# hidden \\\n] still comment\nset y 2".into()
        )
    );
}

#[test]
fn cmd_comment_crlf_continuation_hides_closing_bracket() {
    let (rows, _) = cmd_token_rows("[\r\n# hidden \\\r\n] visible]");
    assert_eq!(
        rows[0],
        (TokenType::Cmd, "\r\n# hidden \\\r\n] visible]".into())
    );
}

#[test]
fn cmd_unterminated_tokenises_best_effort() {
    let (rows, _) = cmd_token_rows("[unterminated");
    assert_eq!(rows[0], (TokenType::Cmd, "unterminated".into()));
}

#[test]
fn cmd_span_positions() {
    // `[cmd] rest` — the CMD span covers the whole `[cmd]` range
    // (offset 0..4: start at `[`, end at the last char of the
    // body, `token_text` strips the leading `[`).
    let lexer = Lexer::new("[cmd] rest");
    let map = lexer.source_map().clone();
    let tokens = lexer.tokenise_all().unwrap();
    let cmd = tokens.iter().find(|t| t.kind == TokenType::Cmd).unwrap();
    assert_eq!(cmd.span.start(), 0);
    assert_eq!(cmd.span.end(), 4);
    let (start, end) = map.range_positions(cmd.span);
    assert_eq!(start, SourcePosition::new(0, ByteCol::new(0), 0));
    assert_eq!(end, SourcePosition::new(0, ByteCol::new(3), 3));
    assert_eq!(map.token_text(*cmd), "cmd");
}

#[test]
fn standalone_closing_bracket_is_part_of_word() {
    // `foo]bar` — `]` is not a deferred character, so it's
    // included in the bare word.
    let (rows, _) = cmd_token_rows("foo]bar");
    assert_eq!(rows[0], (TokenType::Esc, "foo]bar".into()));
}

#[test]
fn cmd_resets_at_command_start() {
    let (rows, _) = cmd_token_rows("[cmd] #not-a-comment");
    assert_eq!(rows[0], (TokenType::Cmd, "cmd".into()));
    assert_eq!(rows[1], (TokenType::Sep, " ".into()));
    assert_eq!(rows[2], (TokenType::Esc, "#not-a-comment".into()));
}

#[test]
fn cmd_after_eol_allows_comment_before() {
    let (rows, _) = cmd_token_rows("# c\n[cmd]");
    assert!(rows.iter().any(|(k, _)| *k == TokenType::Comment));
    assert!(rows.iter().any(|(k, _)| *k == TokenType::Cmd));
}

// Braced strings

fn str_token_rows(source: &str) -> (Vec<(TokenType, String)>, SourceMap<'_>) {
    let lexer = Lexer::new(source);
    let map = lexer.source_map().clone();
    let tokens = lexer.tokenise_all().expect("L6 lexer accepts fixture");
    let rows = tokens
        .iter()
        .map(|t| (t.kind, map.token_text(*t).to_owned()))
        .collect();
    (rows, map)
}

#[test]
fn braced_simple_body() {
    let (rows, _) = str_token_rows("{hello world}");
    assert_eq!(
        rows,
        vec![
            (TokenType::Str, "hello world".into()),
            (TokenType::Eol, String::new()),
        ]
    );
}

#[test]
fn braced_empty_body() {
    let (rows, _) = str_token_rows("{}");
    assert_eq!(rows[0], (TokenType::Str, String::new()));
}

#[test]
fn braced_nested_once() {
    let (rows, _) = str_token_rows("{a {b c} d}");
    assert_eq!(rows[0], (TokenType::Str, "a {b c} d".into()));
}

#[test]
fn braced_deeply_nested() {
    let (rows, _) = str_token_rows("{a {b {c {d}}}}");
    assert_eq!(rows[0], (TokenType::Str, "a {b {c {d}}}".into()));
}

#[test]
fn braced_after_word() {
    // `proc foo {body}` — SEP after "foo" makes `{body}` a
    // newword, so the `{` starts a braced string.
    let (rows, _) = str_token_rows("proc foo {body}");
    assert_eq!(rows[0], (TokenType::Esc, "proc".into()));
    assert_eq!(rows[2], (TokenType::Esc, "foo".into()));
    assert_eq!(rows[4], (TokenType::Str, "body".into()));
}

#[test]
fn braced_midword_is_regular_character() {
    // `foo{bar}` — the `{` is NOT at a word boundary (the
    // previous token was ESC, not SEP/EOL/STR/EXPAND), so it's
    // a regular character in the bare word.
    let (rows, _) = str_token_rows("foo{bar}");
    assert_eq!(rows[0], (TokenType::Esc, "foo{bar}".into()));
}

#[test]
fn close_brace_midword_is_regular_character() {
    let (rows, _) = str_token_rows("foo}bar");
    assert_eq!(rows[0], (TokenType::Esc, "foo}bar".into()));
}

#[test]
fn braced_multiline_body() {
    let (rows, _) = str_token_rows("{line1\nline2}");
    assert_eq!(rows[0], (TokenType::Str, "line1\nline2".into()));
}

#[test]
fn braced_with_dollar_is_literal() {
    // `{foo $bar}` — `$` inside a braced string is a literal
    // character, not a variable substitution. This happens
    // naturally because the brace scanner only looks for
    // balanced `{` / `}` and backslashes.
    let (rows, _) = str_token_rows("{foo $bar}");
    assert_eq!(rows[0], (TokenType::Str, "foo $bar".into()));
}

#[test]
fn braced_with_brackets_is_literal() {
    let (rows, _) = str_token_rows("{foo [bar]}");
    assert_eq!(rows[0], (TokenType::Str, "foo [bar]".into()));
}

#[test]
fn braced_with_backslash_is_literal_pair() {
    // `{foo\nbar}` — the backslash and `n` are preserved in
    // the token text because braces are verbatim in Tcl. The
    // scanner skips them as a pair so `\}` does not count as
    // a close brace.
    let (rows, _) = str_token_rows(r"{foo\nbar}");
    assert_eq!(rows[0], (TokenType::Str, r"foo\nbar".into()));
}

#[test]
fn braced_with_backslash_close_brace_is_inert() {
    let (rows, _) = str_token_rows(r"{a\}b}");
    assert_eq!(rows[0], (TokenType::Str, r"a\}b".into()));
}

#[test]
fn braced_with_backslash_open_brace_is_inert() {
    let (rows, _) = str_token_rows(r"{a\{b}");
    assert_eq!(rows[0], (TokenType::Str, r"a\{b".into()));
}

#[test]
fn braced_followed_by_word() {
    let (rows, _) = str_token_rows("{foo} bar");
    assert_eq!(rows[0], (TokenType::Str, "foo".into()));
    assert_eq!(rows[1], (TokenType::Sep, " ".into()));
    assert_eq!(rows[2], (TokenType::Esc, "bar".into()));
}

#[test]
fn braced_then_braced() {
    // After a STR token, `newword` is True again (STR is in
    // the `newword` set alongside SEP/EOL/EXPAND), so a second
    // `{` starts another braced string.
    let (rows, _) = str_token_rows("{a}{b}");
    assert_eq!(rows[0], (TokenType::Str, "a".into()));
    assert_eq!(rows[1], (TokenType::Str, "b".into()));
}

#[test]
fn braced_unterminated_tokenises_best_effort() {
    let (rows, _) = str_token_rows("{unterminated");
    assert_eq!(rows[0], (TokenType::Str, "unterminated".into()));
}

#[test]
fn braced_at_command_start() {
    // A `{` at the start of a line (initial at_command_start=
    // true, initial last_kind=Eol → newword=true) is a STR.
    let (rows, _) = str_token_rows("{hello}");
    assert_eq!(rows[0], (TokenType::Str, "hello".into()));
}

#[test]
fn braced_inside_command_substitution() {
    // `[list {a b}]` — the braces inside a CMD body are handled
    // by `parse_command`'s own `blevel` tracking, not by
    // `parse_brace`. The STR stays one CMD token.
    let (rows, _) = str_token_rows("[list {a b}]");
    assert_eq!(rows[0], (TokenType::Cmd, "list {a b}".into()));
}

#[test]
fn braced_span_positions() {
    // `{hello}` — span covers `{hello` (0..6), token_text
    // strips `{`, end position is at the 'o' not the `}`.
    let lexer = Lexer::new("{hello}");
    let map = lexer.source_map().clone();
    let tokens = lexer.tokenise_all().unwrap();
    let brace = tokens.iter().find(|t| t.kind == TokenType::Str).unwrap();
    assert_eq!(brace.span.start(), 0);
    assert_eq!(brace.span.end(), 6);
    let (start, end) = map.range_positions(brace.span);
    assert_eq!(start, SourcePosition::new(0, ByteCol::new(0), 0));
    assert_eq!(end, SourcePosition::new(0, ByteCol::new(5), 5));
    assert_eq!(map.token_text(*brace), "hello");
}

#[test]
fn braced_resets_at_command_start() {
    // After a STR, `at_command_start` is False so `#` is not
    // a comment.
    let (rows, _) = str_token_rows("{body} #not-a-comment");
    assert_eq!(rows[0], (TokenType::Str, "body".into()));
    assert_eq!(rows[1], (TokenType::Sep, " ".into()));
    assert_eq!(rows[2], (TokenType::Esc, "#not-a-comment".into()));
}

#[test]
fn braced_preserves_newword_for_next_token() {
    // `{a}{b}{c}` — every STR sets `last_kind = Str`, which is
    // in the `newword` set, so subsequent `{`s start new
    // braced strings.
    let (rows, _) = str_token_rows("{a}{b}{c}");
    assert_eq!(rows[0], (TokenType::Str, "a".into()));
    assert_eq!(rows[1], (TokenType::Str, "b".into()));
    assert_eq!(rows[2], (TokenType::Str, "c".into()));
}

// Quoted strings

fn quoted_rows(source: &str) -> (Vec<(TokenType, String, bool)>, SourceMap<'_>) {
    let lexer = Lexer::new(source);
    let map = lexer.source_map().clone();
    let tokens = lexer.tokenise_all().expect("L7 lexer accepts fixture");
    let rows = tokens
        .iter()
        .map(|t| (t.kind, map.token_text(*t).to_owned(), t.in_quote))
        .collect();
    (rows, map)
}

#[test]
fn quoted_simple() {
    let (rows, _) = quoted_rows(r#""hello""#);
    assert_eq!(
        rows,
        vec![
            (TokenType::Esc, "hello".into(), false),
            (TokenType::Eol, String::new(), false),
        ]
    );
}

#[test]
fn quoted_with_space() {
    let (rows, _) = quoted_rows(r#""hello world""#);
    assert_eq!(rows[0], (TokenType::Esc, "hello world".into(), false));
}

#[test]
fn quoted_empty() {
    // `""` — empty body. The span covers both `"`s (end clamp)
    // and `token_text` returns `""`.
    let (rows, _) = quoted_rows(r#""""#);
    assert_eq!(rows[0], (TokenType::Esc, String::new(), false));
}

#[test]
fn quoted_contains_braces_literally() {
    // Inside a quoted string, `{` and `}` are regular content,
    // NOT braced-string delimiters.
    let (rows, _) = quoted_rows(r#""hello {world}""#);
    assert_eq!(rows[0], (TokenType::Esc, "hello {world}".into(), false));
}

#[test]
fn quoted_contains_separators_literally() {
    let (rows, _) = quoted_rows(r#""tab  spaces""#);
    assert_eq!(rows[0], (TokenType::Esc, "tab  spaces".into(), false));
}

#[test]
fn quoted_with_hash_is_literal() {
    let (rows, _) = quoted_rows("\"# not a comment\"");
    assert_eq!(rows[0], (TokenType::Esc, "# not a comment".into(), false));
}

#[test]
fn quoted_with_var_interpolation() {
    let (rows, _) = quoted_rows(r#""hello $foo world""#);
    assert_eq!(rows[0], (TokenType::Esc, "hello ".into(), true));
    assert_eq!(rows[1], (TokenType::Var, "foo".into(), true));
    assert_eq!(rows[2], (TokenType::Esc, " world".into(), false));
}

#[test]
fn quoted_with_cmd_interpolation() {
    let (rows, _) = quoted_rows(r#""a [cmd] b""#);
    assert_eq!(rows[0], (TokenType::Esc, "a ".into(), true));
    assert_eq!(rows[1], (TokenType::Cmd, "cmd".into(), true));
    assert_eq!(rows[2], (TokenType::Esc, " b".into(), false));
}

#[test]
fn quoted_with_var_and_cmd() {
    let (rows, _) = quoted_rows(r#""a $b [c] d""#);
    assert_eq!(rows[0], (TokenType::Esc, "a ".into(), true));
    assert_eq!(rows[1], (TokenType::Var, "b".into(), true));
    assert_eq!(rows[2], (TokenType::Esc, " ".into(), true));
    assert_eq!(rows[3], (TokenType::Cmd, "c".into(), true));
    assert_eq!(rows[4], (TokenType::Esc, " d".into(), false));
}

#[test]
fn quoted_opening_empty_with_var() {
    // `"$foo"` — empty first ESC, then VAR, then empty
    // closing ESC.
    let (rows, _) = quoted_rows(r#""$foo""#);
    assert_eq!(rows[0], (TokenType::Esc, String::new(), true));
    assert_eq!(rows[1], (TokenType::Var, "foo".into(), true));
    assert_eq!(rows[2], (TokenType::Esc, String::new(), false));
}

#[test]
fn quoted_opening_empty_with_cmd() {
    let (rows, _) = quoted_rows(r#""[cmd]""#);
    assert_eq!(rows[0], (TokenType::Esc, String::new(), true));
    assert_eq!(rows[1], (TokenType::Cmd, "cmd".into(), true));
    assert_eq!(rows[2], (TokenType::Esc, String::new(), false));
}

#[test]
fn quoted_mid_word_is_regular_character() {
    // `foo"bar"` — the `"` is NOT at a word boundary, so it's
    // a regular character in the bare word.
    let (rows, _) = quoted_rows(r#"foo"bar""#);
    assert_eq!(rows[0], (TokenType::Esc, r#"foo"bar""#.into(), false));
}

#[test]
fn quoted_after_esc_then_space_is_word_start() {
    let (rows, _) = quoted_rows(r#"foo "bar""#);
    assert_eq!(rows[0], (TokenType::Esc, "foo".into(), false));
    assert_eq!(rows[1], (TokenType::Sep, " ".into(), false));
    assert_eq!(rows[2], (TokenType::Esc, "bar".into(), false));
}

#[test]
fn quoted_then_mid_word_quote() {
    // `"ab""cd"` — first `"ab"` is a quoted string, then `"cd"`
    // is NOT (the preceding token was an ESC, so `newword =
    // false`). The second run becomes a bare ESC containing
    // literal quote characters.
    let (rows, _) = quoted_rows(r#""ab""cd""#);
    assert_eq!(rows[0], (TokenType::Esc, "ab".into(), false));
    assert_eq!(rows[1], (TokenType::Esc, r#""cd""#.into(), false));
}

#[test]
fn quoted_unterminated_tokenises_best_effort() {
    // `"abc` — no closing quote. The lexer scans "abc" and
    // leaves `in_quote = true`. The trailing ghost EOL
    // inherits `in_quote = false` because it uses the default
    // `Token::new` which sets `in_quote = false`.
    let (rows, _) = quoted_rows(r#""abc"#);
    assert_eq!(rows[0], (TokenType::Esc, "abc".into(), true));
}

#[test]
fn quoted_multiline_body() {
    let (rows, _) = quoted_rows("\"line1\nline2\"");
    assert_eq!(rows[0], (TokenType::Esc, "line1\nline2".into(), false));
}

#[test]
fn quoted_span_positions() {
    // `"hello"` — span covers `"hello`, end position is at the
    // last content char (the 'o'), not at the closing `"`.
    let lexer = Lexer::new(r#""hello""#);
    let map = lexer.source_map().clone();
    let tokens = lexer.tokenise_all().unwrap();
    let esc = tokens.iter().find(|t| t.kind == TokenType::Esc).unwrap();
    assert_eq!(esc.span.start(), 0);
    assert_eq!(esc.span.end(), 6);
    let (start, end) = map.range_positions(esc.span);
    assert_eq!(start, SourcePosition::new(0, ByteCol::new(0), 0));
    assert_eq!(end, SourcePosition::new(0, ByteCol::new(5), 5));
    assert_eq!(map.token_text(*esc), "hello");
}

#[test]
fn quoted_inside_cmd_is_managed_by_parse_command() {
    // `[puts "hello"]` — the `"…"` inside a command body is
    // handled by `parse_command`'s own `in_quotes` tracking,
    // not by the outer `in_quote` state. The result is a
    // single CMD token.
    let (rows, _) = quoted_rows(r#"[puts "hello"]"#);
    assert_eq!(rows[0], (TokenType::Cmd, r#"puts "hello""#.into(), false));
}

#[test]
fn quoted_resets_at_command_start() {
    let (rows, _) = quoted_rows(r#""body" #not-a-comment"#);
    assert_eq!(rows[0], (TokenType::Esc, "body".into(), false));
    assert_eq!(rows[1], (TokenType::Sep, " ".into(), false));
    assert_eq!(rows[2], (TokenType::Esc, "#not-a-comment".into(), false));
}

#[test]
fn quoted_in_quote_propagates_to_sub_tokens() {
    // Verify explicitly that sub-tokens inside a quoted run
    // carry `in_quote = true`.
    let (rows, _) = quoted_rows(r#""$a [b] $c""#);
    assert_eq!(rows[0], (TokenType::Esc, String::new(), true));
    assert_eq!(rows[1], (TokenType::Var, "a".into(), true));
    assert_eq!(rows[2], (TokenType::Esc, " ".into(), true));
    assert_eq!(rows[3], (TokenType::Cmd, "b".into(), true));
    assert_eq!(rows[4], (TokenType::Esc, " ".into(), true));
    assert_eq!(rows[5], (TokenType::Var, "c".into(), true));
    assert_eq!(rows[6], (TokenType::Esc, String::new(), false));
}

// `{*}` expansion prefix + dialect flags

fn expand_rows(source: &str) -> Vec<(TokenType, String)> {
    let lexer = Lexer::new(source);
    let map = lexer.source_map().clone();
    lexer
        .tokenise_all()
        .expect("L8 lexer accepts fixture")
        .iter()
        .map(|t| (t.kind, map.token_text(*t).to_owned()))
        .collect()
}

fn expand_rows_no_expand(source: &str) -> Vec<(TokenType, String)> {
    let config = LexerConfig {
        expand_syntax: false,
        ..LexerConfig::default()
    };
    let map = SourceMap::new(source);
    let lexer = Lexer::with_source_map(map.clone(), config);
    lexer
        .tokenise_all()
        .expect("L8 lexer accepts fixture")
        .iter()
        .map(|t| (t.kind, map.token_text(*t).to_owned()))
        .collect()
}

#[test]
fn expand_prefix_before_bare_word() {
    let rows = expand_rows("{*}list");
    assert_eq!(rows[0], (TokenType::Expand, String::new()));
    assert_eq!(rows[1], (TokenType::Esc, "list".into()));
}

#[test]
fn expand_prefix_before_var() {
    let rows = expand_rows("{*}$var");
    assert_eq!(rows[0], (TokenType::Expand, String::new()));
    assert_eq!(rows[1], (TokenType::Var, "var".into()));
}

#[test]
fn expand_prefix_before_cmd() {
    let rows = expand_rows("{*}[cmd]");
    assert_eq!(rows[0], (TokenType::Expand, String::new()));
    assert_eq!(rows[1], (TokenType::Cmd, "cmd".into()));
}

#[test]
fn expand_prefix_before_braced() {
    let rows = expand_rows("{*}{a b}");
    assert_eq!(rows[0], (TokenType::Expand, String::new()));
    assert_eq!(rows[1], (TokenType::Str, "a b".into()));
}

#[test]
fn expand_prefix_mid_command() {
    let rows = expand_rows("cmd {*}$args");
    assert_eq!(rows[0], (TokenType::Esc, "cmd".into()));
    assert_eq!(rows[2], (TokenType::Expand, String::new()));
    assert_eq!(rows[3], (TokenType::Var, "args".into()));
}

#[test]
fn expand_followed_by_separator_is_braced_string() {
    // `{*} list` — the space after `}` means it's NOT an
    // expansion prefix; it's a braced string `{*}` = STR("*").
    let rows = expand_rows("{*} list");
    assert_eq!(rows[0], (TokenType::Str, "*".into()));
}

#[test]
fn expand_at_eol_is_braced_string() {
    // `{*}` alone (at EOF) — nothing follows, so it's STR("*").
    let rows = expand_rows("{*}");
    assert_eq!(rows[0], (TokenType::Str, "*".into()));
}

#[test]
fn expand_syntax_disabled_always_parses_as_brace() {
    // With `expand_syntax = false`, `{*}list` is STR("*")
    // followed by ESC("list").
    let rows = expand_rows_no_expand("{*}list");
    assert_eq!(rows[0], (TokenType::Str, "*".into()));
    assert_eq!(rows[1], (TokenType::Esc, "list".into()));
}

#[test]
fn expand_newword_is_true_after_expand() {
    // After EXPAND, `newword` should be true (EXPAND is in the
    // `is_newword` set), so a `{` starts a braced string.
    let rows = expand_rows("{*}{a b}");
    assert_eq!(rows[0], (TokenType::Expand, String::new()));
    assert_eq!(rows[1], (TokenType::Str, "a b".into()));
}

#[test]
fn expand_span_is_empty() {
    // The EXPAND token's span covers just the `{` (one byte) —
    // a zero-width marker.
    let lexer = Lexer::new("{*}list");
    let map = lexer.source_map().clone();
    let tokens = lexer.tokenise_all().unwrap();
    let expand = tokens.iter().find(|t| t.kind == TokenType::Expand).unwrap();
    assert_eq!(expand.span.start(), 0);
    assert_eq!(expand.span.end(), 0);
    let (start, end) = map.range_positions(expand.span);
    assert_eq!(start, SourcePosition::new(0, ByteCol::new(0), 0));
    assert_eq!(end, SourcePosition::new(0, ByteCol::new(0), 0));
}

#[test]
fn for_dialect_tcl84_disables_expand_syntax() {
    let cfg = LexerConfig::for_dialect("tcl8.4");
    assert!(!cfg.expand_syntax);
    assert!(!cfg.irules_brace_separator);
}

#[test]
fn for_dialect_tcl86_keeps_defaults() {
    let cfg = LexerConfig::for_dialect("tcl8.6");
    assert!(cfg.expand_syntax);
    assert!(!cfg.irules_brace_separator);
}

#[test]
fn for_dialect_irules_enables_brace_separator() {
    let cfg = LexerConfig::for_dialect("f5-irules");
    assert!(!cfg.expand_syntax);
    assert!(cfg.irules_brace_separator);
}

#[test]
fn for_dialect_unknown_falls_back_to_defaults() {
    let cfg = LexerConfig::for_dialect("not-a-real-dialect");
    let default = LexerConfig::default();
    assert_eq!(cfg.expand_syntax, default.expand_syntax);
    assert_eq!(cfg.irules_brace_separator, default.irules_brace_separator);
    assert_eq!(cfg.braced_var, default.braced_var);
}

#[test]
fn for_dialect_braced_var_follows_the_embedded_runtime() {
    // 8.x runtimes — plain, F5 (tmsh included), EDA, and Expect (an
    // embedded Tcl 8.6) — use the first-close `${…}` rule. Expect and
    // f5-tmsh must resolve through the dialect profile: a lookup keyed
    // only on a fixed string set would miss them and fall through to the
    // modern nesting rule.
    for d in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "f5-irules",
        "f5-iapps",
        "f5-tmsh",
        "expect",
        "xilinx-eda-tcl",
        "synopsys-eda-tcl",
    ] {
        assert_eq!(
            LexerConfig::for_dialect(d).braced_var,
            BracedVarStyle::FirstClose,
            "{d}"
        );
    }
    // 9.x runtimes nest — bpf embeds Tcl 9.0.
    for d in ["tcl9.0", "tcl9.1", "bpf"] {
        assert_eq!(
            LexerConfig::for_dialect(d).braced_var,
            BracedVarStyle::Tcl9Nesting,
            "{d}"
        );
    }
}

#[test]
fn expect_braced_var_lexes_to_first_close() {
    // Behavioural proof of the flip, not just config plumbing:
    // `${a{b}c}` under expect names the variable `a{b` (the 8.x
    // first-close rule, 8.6.14 tclParse.c:1466) — the trailing `c}` is
    // ordinary word text.
    let src = "set x ${a{b}c}\n";
    let cfg = LexerConfig::for_dialect("expect");
    let map = SourceMap::new(src);
    let tokens = Lexer::with_source_map(map, cfg)
        .tokenise_all()
        .expect("lexes");
    let sm = SourceMap::new(src);
    let var_texts: Vec<&str> = tokens
        .iter()
        .filter(|t| t.kind == TokenType::Var)
        .map(|t| sm.token_text(*t))
        .collect();
    assert_eq!(var_texts, ["a{b"], "expect uses the first-close rule");
}

// ghost-token recovery

#[test]
fn ghost_bracket_terminates_unclosed_command() {
    // `set x [foo` — inject a ghost `]` at offset 10 (one past
    // "foo"). The lexer should lex a terminated Cmd token spanning
    // `[foo` and emit no "missing close-bracket" warning.
    let src = "set x [foo";
    let mut ghosts = std::collections::BTreeMap::new();
    ghosts.insert(10u32, b']');
    let lexer = Lexer::new(src).with_ghosts(ghosts);
    let (tokens, warnings) = lexer.tokenise_all_with_warnings().expect("lex ok");
    let cmd = tokens
        .iter()
        .find(|t| t.kind == TokenType::Cmd)
        .expect("a Cmd token");
    // The ghost `]` is zero-width, so the Cmd span ends at the ghost
    // offset (no extra byte consumed) and no real char is skipped.
    assert_eq!(cmd.span.start(), 6);
    assert_eq!(cmd.span.end(), 10);
    assert!(
        warnings
            .iter()
            .all(|w| w.message != "missing close-bracket"),
        "ghost should suppress the unterminated warning: {warnings:?}",
    );
}

#[test]
fn ghost_bracket_at_eof_with_deep_nesting_stays_in_bounds() {
    // A ghost `]` consumed while nesting is ≥ 2 must be
    // zero-width — it must not advance `pos` past the real byte (here past
    // EOF to `len + 1`), which produced a token span one past the buffer and
    // panicked `SourceMap::text`.
    let src = "[a[b"; // outer `[` (level 1) then inner `[b` (level 2)
    let len = u32::try_from(src.len()).unwrap();
    let mut ghosts = std::collections::BTreeMap::new();
    ghosts.insert(len, b']'); // a single ghost at EOF — closes one level
    let lexer = Lexer::new(src).with_ghosts(ghosts);
    let (tokens, _warnings) = lexer.tokenise_all_with_warnings().expect("lex ok");
    for t in &tokens {
        assert!(
            t.span.end() <= len,
            "token span end {} exceeds source length {len}: {t:?}",
            t.span.end()
        );
    }
}

#[test]
fn ghost_bracket_mid_stream_with_nesting_does_not_skip_real_byte() {
    // A ghost `]` at an interior offset with level ≥ 2 must not swallow the
    // real byte sitting at that offset.
    let src = "[a[bc"; // `[`0 `a`1 `[`2 `b`3 `c`4
    let mut ghosts = std::collections::BTreeMap::new();
    ghosts.insert(3u32, b']'); // ghost at `b`, level is 2 here
    let lexer = Lexer::new(src).with_ghosts(ghosts);
    let (tokens, _warnings) = lexer.tokenise_all_with_warnings().expect("lex ok");
    // The `b` byte must still fall within the emitted Cmd token (not skipped
    // out of the span).
    let cmd = tokens
        .iter()
        .find(|t| t.kind == TokenType::Cmd)
        .expect("a Cmd token");
    assert!(cmd.span.end() <= u32::try_from(src.len()).unwrap());
}

#[test]
fn ghost_bracket_splits_swallowed_following_command() {
    // `[foo bar\nputs done` with a ghost `]` after "bar" re-lexes as
    // a terminated `[foo bar]` followed by a clean `puts done`.
    let src = "[foo bar\nputs done";
    let mut ghosts = std::collections::BTreeMap::new();
    ghosts.insert(8u32, b']'); // one past "bar", at the '\n'
    let lexer = Lexer::new(src).with_ghosts(ghosts);
    let (tokens, _) = lexer.tokenise_all_with_warnings().expect("lex ok");
    // The first Cmd token covers `[foo bar` (zero-width close at 8).
    let cmd = tokens.iter().find(|t| t.kind == TokenType::Cmd).unwrap();
    assert_eq!((cmd.span.start(), cmd.span.end()), (0, 8));
    // `puts` and `done` are lexed as ordinary words after the split,
    // with their original absolute offsets intact.
    let words: Vec<&str> = tokens
        .iter()
        .filter(|t| t.kind == TokenType::Esc)
        .map(|t| &src[t.span.start() as usize..t.span.end() as usize])
        .collect();
    assert!(words.contains(&"puts"), "{words:?}");
    assert!(words.contains(&"done"), "{words:?}");
}

#[test]
fn no_ghosts_is_identical_to_plain_lexing() {
    let src = "set x [foo]\nputs $x\n";
    let plain = Lexer::new(src).tokenise_all().unwrap();
    let with_empty = Lexer::new(src)
        .with_ghosts(std::collections::BTreeMap::new())
        .tokenise_all()
        .unwrap();
    assert_eq!(plain, with_empty);
}

/// `scan_array_index_body` and
/// `skip_var_in_index` are mutually recursive on nested `$a($b($c(…)))`
/// array-index references, so a depth cap is required: unguarded input
/// overflows the native stack (SIGABRT) around depth 20,000-25,000 on a
/// 2 MiB thread (`cargo test`'s per-test default). 5000 is comfortably
/// past both that crash range and `MAX_ARRAY_INDEX_DEPTH` (64); the
/// assertion is that lexing returns at all, not what it returns.
#[test]
fn deeply_nested_array_index_survives_lexing() {
    const DEPTH: usize = 5000;
    let mut src = String::from("set x $a0");
    for i in 0..DEPTH {
        src.push('(');
        let _ = write!(src, "a{}", i + 1);
    }
    src.push('1');
    for _ in 0..DEPTH {
        src.push(')');
    }
    src.push('\n');
    let _ = Lexer::new(&src).tokenise_all_with_warnings();
}

/// A moderately nested array index (well under `MAX_ARRAY_INDEX_DEPTH`)
/// still lexes as a single `Var` token — the safety net must not fire
/// on realistic nesting depths.
#[test]
fn moderately_nested_array_index_still_lexes_as_one_var() {
    let src = "set x $a($b($c(1)))\n";
    let lexed = Lexed::run(src);
    let var = lexed
        .tokens
        .iter()
        .zip(lexed.texts())
        .find(|(t, _)| t.kind == TokenType::Var)
        .expect("expected a Var token");
    assert_eq!(var.1, "$a($b($c(1)))");
}

#[test]
fn as_quoted_body_treats_embedded_braces_as_literal_content() {
    // `{$a}` is ordinary literal-then-substitution content, not a fresh
    // brace-quoted word: no `Str` token at all, and `$a` surfaces as its
    // own `Var` token.
    let src = "prefix {$a} suffix";
    let lexer = Lexer::new(src).as_quoted_body();
    let sm = lexer.source_map().clone();
    let tokens = lexer
        .tokenise_all()
        .expect("quoted-body lexing is best-effort, never fails");
    assert!(
        !tokens.iter().any(|t| t.kind == TokenType::Str),
        "no token should be brace-quoted in quoted-body mode: {tokens:?}"
    );
    let var = tokens
        .iter()
        .find(|t| t.kind == TokenType::Var)
        .expect("$a must surface as its own Var token");
    assert_eq!(sm.token_text(*var), "a");
}

#[test]
fn default_lexing_of_the_same_text_brace_quotes_it_instead() {
    // Contrast case: without `as_quoted_body`, the identical text is
    // lexed as fresh top-level command words, so `{$a}` is one
    // non-substituting `Str` token and `$a` never becomes a `Var` token
    // at all — this is the exact mis-tokenisation idx 125 fixed by
    // giving callers `as_quoted_body` to opt out of.
    let src = "prefix {$a} suffix";
    let tokens = Lexer::new(src).tokenise_all().expect("lexes fine");
    assert!(
        tokens.iter().any(|t| t.kind == TokenType::Str),
        "the default top-level lexer should brace-quote {{$a}}: {tokens:?}"
    );
    assert!(
        !tokens.iter().any(|t| t.kind == TokenType::Var),
        "so $a must NOT surface as a Var token: {tokens:?}"
    );
}

#[test]
fn as_quoted_body_still_dispatches_dollar_and_bracket_normally() {
    let src = "$x [foo $y]";
    let lexer = Lexer::new(src).as_quoted_body();
    let sm = lexer.source_map().clone();
    let tokens = lexer.tokenise_all().expect("lexes fine");
    let var_texts: Vec<&str> = tokens
        .iter()
        .filter(|t| t.kind == TokenType::Var)
        .map(|t| sm.token_text(*t))
        .collect();
    assert_eq!(var_texts, vec!["x"], "only the top-level $x is a Var here");
    let cmd = tokens
        .iter()
        .find(|t| t.kind == TokenType::Cmd)
        .expect("[foo $y] must still be recognised as a Cmd token");
    assert_eq!(sm.token_text(*cmd), "foo $y");
}

#[test]
fn as_quoted_body_treats_the_missing_close_quote_as_a_soft_warning() {
    // There is no real closing `"` to find (the caller already stripped
    // it, if there ever was one) — this must be a best-effort, non-fatal
    // warning under the default (non-`strict_quoting`) config, not a
    // `LexError`, since every value-body scan otherwise relies on
    // `tokenise_all()` succeeding.
    let lexer = Lexer::new("plain bareword value").as_quoted_body();
    let result = lexer.tokenise_all();
    assert!(
        result.is_ok(),
        "a missing close-quote must not be a hard error by default: {result:?}"
    );
}

#[test]
fn smoke_lex_canonical_snippet() {
    // Word, braced word, var substitution, command substitution, and a
    // comment, in one pass — the cheapest "the lexer still starts up and
    // produces sane output" check.
    let lexed = Lexed::run("set x {a b}\nputs $x ;# comment\nset y [llength $x]\n");
    let kinds = lexed.kinds();
    assert!(kinds.contains(&TokenType::Esc), "{kinds:?}");
    assert!(kinds.contains(&TokenType::Str), "{kinds:?}");
    assert!(kinds.contains(&TokenType::Var), "{kinds:?}");
    assert!(kinds.contains(&TokenType::Cmd), "{kinds:?}");
    assert!(kinds.contains(&TokenType::Comment), "{kinds:?}");
}
