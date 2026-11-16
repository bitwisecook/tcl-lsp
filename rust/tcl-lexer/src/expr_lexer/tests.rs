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

#[test]
fn jim_expression_frontiers_retain_original_constructor_spans() {
    let grammar = tcl_dialect::model::grammar(
        tcl_dialect::model::Family::Jim,
        tcl_dialect::model::Release::JIM_0_84,
    );
    let config = crate::LexerConfig::from_grammar(grammar);
    for (source, first) in [
        (b"0x1p4".as_slice(), b"0x1".as_slice()),
        (b"0x1.8p1", b"0x1.8p1"),
        (b"0x1.", b"0x1."),
        (b"0x.p4", b"0"),
        (b"Infinity", b"Inf"),
        (b"12x", b"12"),
        (b".5", b".5"),
    ] {
        let (tokens, unknown) = tokenise_expr_bytes_checked_with_expression_grammar(
            source,
            &grammar,
            Some(tcl_dialect::TclVersion::V9_0),
            None,
        );
        // The malformed radix suffix still contains its actual invalid dot.
        // Its first numeral has a valid native integer-prefix extent.
        assert_eq!(unknown, source == b"0x.p4", "{source:?}");
        assert_eq!(tokens[0].kind, ExprTokenType::Number, "{source:?}");
        assert_eq!(tokens[0].text, first, "{source:?}");
        let term = expression_term(source, &tokens[0], config).unwrap();
        assert_eq!(term.source, 0..u32::try_from(first.len()).unwrap());
        assert!(term.jim_numeric_kind.is_some());
    }
}

#[test]
fn native_expression_tokens_retain_opaque_leaf_bytes_and_offsets() {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let source = b"\"\xff\0tail\" eq {\xff\0tail} && ${\xff} == [set {\xff}]";
    let (tokens, unknown) = super::tokenise_expr_bytes_checked_with_expression_grammar(
        source,
        &profile.grammar,
        profile.expr_grammar_base,
        profile.f5_core_expr_grammar(),
    );
    assert!(!unknown);
    for token in &tokens {
        assert_eq!(
            token.text.as_slice(),
            &source[token.start as usize..=token.end as usize]
        );
    }
    assert!(
        tokens
            .iter()
            .any(|token| token.kind == super::ExprTokenType::String && token.text.contains(&0xff))
    );
    assert!(
        tokens
            .iter()
            .any(|token| token.kind == super::ExprTokenType::Variable && token.text == b"${\xff}")
    );
    assert!(
        tokens
            .iter()
            .any(|token| token.kind == super::ExprTokenType::Command
                && token.text == b"[set {\xff}]")
    );
}

fn types(source: &str) -> Vec<ExprTokenType> {
    tokenise_expr(source, None)
        .into_iter()
        .filter(|t| t.kind != ExprTokenType::Whitespace)
        .map(|t| t.kind)
        .collect()
}

fn texts(source: &str) -> Vec<String> {
    tokenise_expr(source, None)
        .into_iter()
        .filter(|t| t.kind != ExprTokenType::Whitespace)
        .map(|t| t.text)
        .collect()
}

fn texts_for(source: &str, dialect: &str) -> Vec<String> {
    tokenise_expr(source, Some(dialect))
        .into_iter()
        .filter(|t| t.kind != ExprTokenType::Whitespace)
        .map(|t| t.text)
        .collect()
}

#[test]
fn integer() {
    assert_eq!(texts("42"), vec!["42"]);
}

#[test]
fn trailing_backslash_in_command_does_not_panic() {
    // A `[...` command substitution ending in a bare `\`
    // must not push the cursor past end-of-input (out-of-bounds slice in
    // `tok`). The whole token, backslash included, is returned.
    let toks = tokenise_expr(r"[a \", None);
    assert!(toks.iter().any(|t| t.kind == ExprTokenType::Command));
    // A realistic reachable form: `if {$x && [foo \` body extracted and
    // re-tokenised must not crash.
    let _ = tokenise_expr(r"$x && [foo \", None);
}

#[test]
fn trailing_backslash_in_quoted_does_not_panic() {
    // A `"...` string ending in a bare `\`.
    let toks = tokenise_expr(r#""a\"#, None);
    assert!(toks.iter().any(|t| t.kind == ExprTokenType::String));
}

#[test]
fn command_and_quote_tokens_use_the_shared_tcl_range_grammar() {
    let source = "[set x 1; # ] remains a script comment\nHTTP::host]";
    let (tokens, unknown) = tokenise_expr_checked(source, Some("tcl9.0"));
    assert!(!unknown, "a complete script substitution is executable");
    assert_eq!(
        tokens
            .iter()
            .find(|token| token.kind == ExprTokenType::Command)
            .map(|token| token.text.as_str()),
        Some(source)
    );

    let quoted = r#""a[format "%s" b]c""#;
    let (tokens, unknown) = tokenise_expr_checked(quoted, Some("tcl9.0"));
    assert!(!unknown, "a quote may contain a quoted nested script word");
    assert_eq!(
        tokens
            .iter()
            .find(|token| token.kind == ExprTokenType::String)
            .map(|token| token.text.as_str()),
        Some(quoted)
    );
}

#[test]
fn unterminated_command_and_quote_are_checked_lex_errors() {
    for source in [r"[a ", r#""a\"#] {
        let (_tokens, unknown) = tokenise_expr_checked(source, None);
        assert!(unknown, "unterminated expression token: {source:?}");
    }
}

#[test]
fn backslash_newline_is_whitespace_continuation() {
    // GAP-B7: `1 \<LF>    + 2` — the backslash-newline is a Tcl 9
    // line continuation and must lex as whitespace, not an Unknown
    // token, so the expression stays structured.
    let src = "1 \\\n    + 2";
    assert_eq!(
        types(src),
        vec![
            ExprTokenType::Number,
            ExprTokenType::Operator,
            ExprTokenType::Number
        ],
    );
    let (_toks, has_unknown) = tokenise_expr_checked(src, None);
    assert!(!has_unknown, "backslash-newline should not be unknown");
}

#[test]
fn lone_backslash_without_newline_stays_unknown() {
    // A bare `\` not before a newline is still not valid expr
    // syntax — only the continuation form is whitespace.
    let (_toks, has_unknown) = tokenise_expr_checked("1 \\ 2", None);
    assert!(has_unknown);
}

#[test]
fn float_and_scientific() {
    assert_eq!(texts("3.14"), vec!["3.14"]);
    assert_eq!(texts("1.5e10"), vec!["1.5e10"]);
}

#[test]
fn hex_octal_binary() {
    assert_eq!(texts("0xFF"), vec!["0xFF"]);
    assert_eq!(texts("0o77"), vec!["0o77"]);
    assert_eq!(texts("0b1010"), vec!["0b1010"]);
}

#[test]
fn operators() {
    assert_eq!(texts("1 + 2"), vec!["1", "+", "2"]);
    assert_eq!(texts("$a == $b"), vec!["$a", "==", "$b"]);
}

#[test]
fn word_operators() {
    assert_eq!(texts("1 eq 2"), vec!["1", "eq", "2"]);
}

#[test]
fn word_operator_boundary() {
    assert_eq!(texts("equal"), vec!["equal"]);
}

/// The F5 word-form operators are an `f5-tcl` **trunk** fact — measured
/// valid in tmsh and iApp `expr` too, not iRules-only
/// (`docs/design/f5/bigip-irule-parser-measurements.md` §4a) — so every
/// F5Tcl-cored profile lexes them as one `Operator` token, derived from
/// the family's `ExprGrammar` word table.
#[test]
fn f5_word_operators_are_a_trunk_fact() {
    let f5_words = [
        "and",
        "or",
        "not",
        "contains",
        "starts_with",
        "ends_with",
        "equals",
        "matches",
        "matches_glob",
        "matches_regex",
    ];
    for dialect in ["f5-irules", "f5-tmsh", "f5-iapps"] {
        for op in f5_words {
            let tokens = tokenise_expr(op, Some(dialect));
            assert_eq!(tokens.len(), 1, "{dialect}: {op}");
            assert_eq!(
                tokens[0].kind,
                ExprTokenType::Operator,
                "{dialect}: {op} must lex as an operator"
            );
        }
    }
    // Plain-Tcl parity: outside the F5 tree the same spellings stay
    // barewords (function-shaped lexemes), byte-identical to before.
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        for op in f5_words {
            let tokens = tokenise_expr(op, Some(dialect));
            assert_eq!(tokens.len(), 1, "{dialect}: {op}");
            assert_eq!(
                tokens[0].kind,
                ExprTokenType::Function,
                "{dialect}: {op} must stay a bareword"
            );
        }
    }
}

#[test]
fn variables() {
    assert_eq!(texts("$x"), vec!["$x"]);
    assert_eq!(texts("${name}"), vec!["${name}"]);
    assert_eq!(texts("$arr(idx)"), vec!["$arr(idx)"]);
}

#[test]
fn array_index_first_paren_terminates() {
    // No paren nesting — the index ends at the first `)`.
    assert_eq!(texts("$a((b)").first().map(String::as_str), Some("$a((b)"));
    // An escaped `)` stays in the index; a `[…]`/`${…}` inner `)` too.
    assert_eq!(
        texts("$a(x\\)y)").first().map(String::as_str),
        Some("$a(x\\)y)")
    );
    assert_eq!(
        texts("$a([f(x)])").first().map(String::as_str),
        Some("$a([f(x)])")
    );
    assert_eq!(
        texts("$a(${b(c)})").first().map(String::as_str),
        Some("$a(${b(c)})")
    );
}

#[test]
fn array_index_literal_brace_follows_the_release_rule() {
    let source = "$a({key})";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let (tokens, unknown) = tokenise_expr_checked(source, Some(dialect));
        assert_eq!(
            tokens.first().map(|token| token.text.as_str()),
            Some(source)
        );
        assert_eq!(unknown, dialect.starts_with("tcl9"), "{dialect}");
    }
    for source in [r"$a(\{key\})", "$a(${key})", "$a([format \\{])"] {
        let (_tokens, unknown) = tokenise_expr_checked(source, Some("tcl9.0"));
        assert!(
            !unknown,
            "Tcl 9 accepts substitution/escaped source: {source}"
        );
    }
}

/// Issue #1601 — the `${…}` closer follows the release's
/// `Tcl_ParseVarName` rule, the same one the carrying command's condition
/// already used, not a local first-`}` walk.
///
/// Oracles (`set {a{b}c} 7`, `set {a\}b} 7`, `set {a{b}} 7`):
///
/// | expression | tclsh 9.0.4 | tclsh 8.6.16 |
/// |---|---|---|
/// | `expr {${a{b}c} + 1}` | `8` | `invalid bareword "c"` |
/// | `expr {${a\}b} + 1}`  | `8` | `invalid bareword "b"` |
/// | `expr {${a{b}} + 1}`  | `8` | `invalid character "}"` |
#[test]
fn braced_variable_closer_follows_the_release_rule() {
    // 9.x nests `{…}` and consumes `\X`, so each of these is one Variable
    // spanning the whole reference.
    for source in [r"${a{b}c}", r"${a\}b}", r"${a{b}}"] {
        let t = texts(source);
        assert_eq!(
            t.first().map(String::as_str),
            Some(source),
            "9.x names the whole reference: {source:?} -> {t:?}"
        );
        assert_eq!(t.len(), 1, "no trailing lexemes at 9.x: {t:?}");
    }

    // 8.x ends the name at the first literal `}` — `{` and `\` are name
    // characters — so the remainder lexes as its own (invalid) lexemes.
    let t = texts_for("${a{b}c}", "tcl8.6");
    assert_eq!(t.first().map(String::as_str), Some("${a{b}"), "{t:?}");
    assert!(t.iter().any(|s| s == "c"), "trailing bareword: {t:?}");
    let t = texts_for(r"${a\}b}", "tcl8.6");
    assert_eq!(t.first().map(String::as_str), Some(r"${a\}"), "{t:?}");
    assert!(t.iter().any(|s| s == "b"), "trailing bareword: {t:?}");

    // The nesting rule also *widens* what counts as unterminated: `${a{b}`
    // closes at 8.x but runs off the end at 9.x.
    let (tokens, unknown) = tokenise_expr_checked("${a{b}", Some("tcl8.6"));
    assert!(!unknown, "8.x closes at the first `}}`");
    assert_eq!(tokens.first().map(|t| t.text.as_str()), Some("${a{b}"));
    let (tokens, unknown) = tokenise_expr_checked("${a{b}", Some("tcl9.0"));
    assert!(
        unknown,
        "9.x has no closer left, so the expression is inexecutable"
    );
    assert_eq!(tokens.first().map(|t| t.text.as_str()), Some("${a{b}"));
}

#[test]
fn quoted_and_command_boundaries_retain_the_selected_variable_grammar() {
    for profile in ["tcl8.6", "tcl9.0"] {
        let profile = tcl_dialect::DialectProfile::find(profile).unwrap();
        for style in [
            tcl_dialect::BracedVarStyle::FirstClose,
            tcl_dialect::BracedVarStyle::Tcl9Nesting,
        ] {
            let grammar = tcl_dialect::LexerGrammar {
                braced_var: style,
                ..profile.grammar
            };
            for source in [r#""${a{b}""#, "[list ${a{b}]"] {
                let (tokens, unknown) = tokenise_expr_checked_with_grammar(source, &grammar);
                assert_eq!(unknown, style.nests(), "{source:?}: {style:?}");
                assert_eq!(
                    tokens.first().map(|token| token.text.as_str()),
                    Some(source)
                );
            }
            for source in [r#""${missing""#, "[list ${missing]"] {
                let (_, unknown) = tokenise_expr_checked_with_grammar(source, &grammar);
                assert!(unknown, "{source:?}: {style:?}");
            }
        }
    }
}

#[test]
fn single_colon_ends_variable_name() {
    // A lone `:` is not a name char — the ternary separator
    // in `$y:$z` must not be swallowed into the variable, or the spaceless
    // ternary `$x>0?$y:$z` degrades to Raw.
    let t = texts("$y:$z");
    assert_eq!(t.first().map(String::as_str), Some("$y"), "{t:?}");
    assert!(
        t.iter().any(|s| s == "$z"),
        "second var must survive: {t:?}"
    );
    // `::` namespace separators are still part of the name.
    assert_eq!(texts("$::g").first().map(String::as_str), Some("$::g"));
    assert_eq!(texts("$a::b").first().map(String::as_str), Some("$a::b"));
    // `$a:b` reports the variable as `a`, not `a:b`.
    assert_eq!(texts("$a:b").first().map(String::as_str), Some("$a"));
}

#[test]
fn command_sub() {
    assert_eq!(texts("[cmd]"), vec!["[cmd]"]);
}

#[test]
fn quoted_string() {
    assert_eq!(texts(r#""hello""#), vec![r#""hello""#]);
}

#[test]
fn function_call() {
    let t = types("sin($x)");
    assert_eq!(t[0], ExprTokenType::Function);
}

#[test]
fn boolean_literals() {
    assert_eq!(types("true"), vec![ExprTokenType::Bool]);
    // Tcl booleans are case-insensitive (`Tcl_GetBoolean`).
    for word in ["True", "FALSE", "Yes", "NO", "On", "Off"] {
        assert_eq!(
            types(word),
            vec![ExprTokenType::Bool],
            "word `{word}` should tokenise as BOOL",
        );
    }
}

#[test]
fn ieee_754_special_literals() {
    // Tcl's `TclParseNumber` treats these case-insensitively. The shared
    // scanner also owns the NaN payload and number/bareword junction:
    // tclsh 8.6.17 and 9.0.3 both accept `iNf`, `NaN(1)`, and
    // `Infinityeq 1`, but keep `Infinityx` a bareword/function name.
    for word in [
        "Inf", "inf", "iNf", "Infinity", "infinity", "NaN", "nan", "NaN(1)",
    ] {
        assert_eq!(
            types(word),
            vec![ExprTokenType::Number],
            "word `{word}` should tokenise as NUMBER",
        );
    }
    assert_eq!(
        texts("Infinityeq 1"),
        vec!["Infinity", "eq", "1"],
        "a word operator after a special float starts a new lexeme",
    );
    assert_eq!(types("Infinityx"), vec![ExprTokenType::Function]);
}

#[test]
fn ternary() {
    assert_eq!(
        types("$a ? 1 : 0"),
        vec![
            ExprTokenType::Variable,
            ExprTokenType::TernaryQ,
            ExprTokenType::Number,
            ExprTokenType::TernaryC,
            ExprTokenType::Number,
        ]
    );
}

#[test]
fn braced() {
    let t = tokenise_expr("{1 + 2}", None);
    let nw: Vec<_> = t
        .into_iter()
        .filter(|t| t.kind != ExprTokenType::Whitespace)
        .collect();
    assert_eq!(nw[0].kind, ExprTokenType::String);
    assert_eq!(nw[0].text, "{1 + 2}");
}

#[test]
fn braced_escaped_brace_does_not_close() {
    // `{a\}b} eq $x` — Tcl's brace scan treats `\}` as a literal pair, so
    // the whole `{a\}b}` is one braced String token, leaving a clean
    // `eq $x` behind rather than ending at the escaped `}` and degrading
    // to Raw (issue 165).
    let t = tokenise_expr(r"{a\}b} eq $x", None);
    let nw: Vec<_> = t
        .into_iter()
        .filter(|t| t.kind != ExprTokenType::Whitespace)
        .collect();
    assert_eq!(nw[0].kind, ExprTokenType::String);
    assert_eq!(nw[0].text, r"{a\}b}");
    assert_eq!(nw[1].kind, ExprTokenType::Operator);
    assert_eq!(nw[1].text, "eq");
    assert_eq!(nw[2].kind, ExprTokenType::Variable);
}

#[test]
fn braced_escaped_open_brace_balances() {
    // `\{` likewise does not open a level: `{a\{b}` is one word.
    let t = tokenise_expr(r"{a\{b}", None);
    let nw: Vec<_> = t
        .into_iter()
        .filter(|t| t.kind != ExprTokenType::Whitespace)
        .collect();
    assert_eq!(nw[0].kind, ExprTokenType::String);
    assert_eq!(nw[0].text, r"{a\{b}");
}

#[test]
fn checked_no_unknown() {
    let (_, u) = tokenise_expr_checked("1 + 2", None);
    assert!(!u);
}

#[test]
fn checked_rejects_unclosed_variable_delimiters() {
    for source in ["${name", "$array(key", "$array([command)"] {
        let (_, unknown) = tokenise_expr_checked(source, Some("f5-irules"));
        assert!(
            unknown,
            "unterminated variable syntax stayed executable: {source:?}"
        );
    }
}

#[test]
fn non_ascii_does_not_panic_at_operator_scan() {
    // Regression: the byte-wise scanner steps over a multi-byte UTF-8 char
    // (e.g. `é`) one byte at a time, so a later `multi_op` check ran at a
    // byte index *inside* the char. The old `self.s[self.i..]` string slice
    // panicked on the non-char-boundary; the byte-prefix check is total.
    // Surfaced via the optimiser running the expr lexer on a `foreach` whose
    // list variable had a non-ASCII name (`foreach item $itemés { … }`).
    for src in [
        "é",
        "$itemés",
        "aé eq b",
        "$x + é",
        " café ne thé",
        "\u{e9}\u{e9}\u{e9}",
    ] {
        // The regression guard is that neither entry point panics; both must
        // also agree on the token stream they produce.
        let toks = tokenise_expr(src, None);
        let (checked, _unknown) = tokenise_expr_checked(src, None);
        assert_eq!(
            toks.len(),
            checked.len(),
            "entry points disagree for {src:?}"
        );
    }
    // The non-ASCII char must not swallow a following operator: the scan has
    // to make progress past it and still recognise the `+`.
    let plus = tokenise_expr("$x + é", None);
    assert!(
        plus.iter()
            .any(|t| t.kind == ExprTokenType::Operator && t.text == "+"),
        "operator after non-ASCII not found: {plus:?}",
    );
}

// TIP 582 `#` comments (`tclCompExpr.c:1931-1942`)

/// The comment token's text, for each comment in `source`.
fn comments(source: &str) -> Vec<String> {
    tokenise_expr(source, None)
        .into_iter()
        .filter(|t| t.kind == ExprTokenType::Comment)
        .map(|t| t.text)
        .collect()
}

/// `#` runs to the end of the line or the end of the expression, wherever
/// it appears — C skips a `COMMENT` lexeme exactly like a whitespace run
/// (`tclCompExpr.c:701`), so it may sit anywhere whitespace may.
#[test]
fn a_comment_may_appear_wherever_whitespace_may() {
    // A trailing comment preserves the preceding expression tokens.
    assert_eq!(comments("1 + 2 # note"), vec!["# note"]);
    assert_eq!(
        types("1 + 2 # note"),
        vec![
            ExprTokenType::Number,
            ExprTokenType::Operator,
            ExprTokenType::Number,
            ExprTokenType::Comment,
        ]
    );
    // Mid-expression, and immediately before an operator with no
    // intervening space.
    assert_eq!(comments("1 #c\n+ 2"), vec!["#c"]);
    assert_eq!(comments("1#c\n+2"), vec!["#c"]);
    // A comment swallows the rest of the line, operators included, so
    // expr-62.1's `expr {1 # + 2}` is just `1`.
    assert_eq!(
        types("1 # + 2"),
        vec![ExprTokenType::Number, ExprTokenType::Comment]
    );
    // Inside a function's argument list, on either side of the comma.
    assert_eq!(comments("max(1,# comment\n2)"), vec!["# comment"]);
    assert_eq!(comments("max(1# comment\n,2)"), vec!["# comment"]);
}

/// C's `:750` lookahead: a comment between a bareword and its `(` still
/// leaves a function call ("Actually a function call, but with obscuring
/// comments"). Pins expr-62.10's token shape.
#[test]
fn a_comment_may_obscure_a_function_call_paren() {
    assert_eq!(
        types("max# comment\n(1,2)"),
        vec![
            ExprTokenType::Function,
            ExprTokenType::Comment,
            ExprTokenType::ParenOpen,
            ExprTokenType::Number,
            ExprTokenType::Comma,
            ExprTokenType::Number,
            ExprTokenType::ParenClose,
        ]
    );
}

/// The terminating newline is *not* part of the comment: C returns
/// `size - (byte == '\n')`, leaving the newline to the whitespace scan.
/// This is what keeps expr-62.2's `expr "1 #\n+ 2"` equal to 3 — if the
/// comment ate the newline nothing would change here, but if it ate the
/// rest of the *expression* the `+ 2` would vanish.
#[test]
fn a_comment_stops_before_its_newline() {
    let toks = tokenise_expr("1 #\n+ 2", None);
    let comment = toks
        .iter()
        .find(|t| t.kind == ExprTokenType::Comment)
        .expect("comment token");
    assert_eq!(comment.text, "#");
    assert_eq!((comment.start, comment.end), (2, 2));
    // The newline survives as its own whitespace token, and the `+ 2` is
    // still there.
    assert_eq!(
        types("1 #\n+ 2"),
        vec![
            ExprTokenType::Number,
            ExprTokenType::Comment,
            ExprTokenType::Operator,
            ExprTokenType::Number,
        ]
    );
    assert!(
        tokenise_expr("1 #\n+ 2", None)
            .iter()
            .any(|t| t.kind == ExprTokenType::Whitespace && t.text == "\n"),
        "the newline must be a whitespace token of its own"
    );
    // A `\` before the newline does not extend the comment either: C's scan
    // is byte-wise and stops at the first raw `\n`.
    assert_eq!(comments("1 #c\\\n+ 2"), vec!["#c\\"]);
    assert_eq!(
        types("1 #c\\\n+ 2"),
        vec![
            ExprTokenType::Number,
            ExprTokenType::Comment,
            ExprTokenType::Operator,
            ExprTokenType::Number,
        ]
    );
}

/// `doc/expr.n`'s one exception: `#` begins a comment "at any point in the
/// expression **except within double quotes or braces**". Those operands are
/// scanned as whole tokens before a `#` inside them is ever looked at, so a
/// hash there stays ordinary text.
#[test]
fn a_hash_inside_a_quoted_or_braced_operand_is_not_a_comment() {
    for src in [r#""a#b" eq $x"#, r"{a#b} eq $x"] {
        let toks = tokenise_expr(src, None);
        assert!(
            !toks.iter().any(|t| t.kind == ExprTokenType::Comment),
            "{src:?} must not lex a comment: {toks:?}"
        );
        // The operand keeps its `#`, and the `eq $x` after it still lexes —
        // proof the hash did not swallow the rest of the line.
        assert_eq!(toks[0].kind, ExprTokenType::String);
        assert!(toks[0].text.contains('#'), "{src:?}");
        assert_eq!(
            types(src),
            vec![
                ExprTokenType::String,
                ExprTokenType::Operator,
                ExprTokenType::Variable,
            ],
            "{src:?}"
        );
    }
}

/// A `#` inside a comment is ordinary text — the comment does not nest or
/// re-open, it simply runs to end of line.
#[test]
fn a_comment_may_contain_a_hash() {
    assert_eq!(comments("1 + 2 # a # b"), vec!["# a # b"]);
    assert_eq!(
        comments("1 ## still one comment"),
        vec!["## still one comment"]
    );
}

/// An unterminated comment — one with no closing newline — runs to the end
/// of the input and is not an error. C's scan simply stops at `numBytes`.
#[test]
fn an_unterminated_comment_runs_to_end_of_input() {
    for src in ["1 + 2 # no newline", "1 + 2 #", "#"] {
        let (toks, unknown) = tokenise_expr_checked(src, None);
        assert!(!unknown, "{src:?} must not report an unknown character");
        let comment = toks
            .iter()
            .find(|t| t.kind == ExprTokenType::Comment)
            .unwrap_or_else(|| panic!("no comment token for {src:?}"));
        // The token covers every remaining byte, so the stream still spans
        // the whole source — which is what keeps the diagnoser from
        // mistaking an uncovered offset for an invalid character.
        assert_eq!(
            comment.end as usize + 1,
            src.len(),
            "comment must reach end of input for {src:?}"
        );
    }
}

/// C stops the scan at an embedded NUL *and consumes it*: the
/// `size - (byte == '\n')` adjustment fires only for a newline, so unlike
/// a newline the NUL is inside the comment.
#[test]
fn an_embedded_nul_ends_the_comment_but_is_consumed() {
    let toks = tokenise_expr("1 #a\0+ 2", None);
    let comment = toks
        .iter()
        .find(|t| t.kind == ExprTokenType::Comment)
        .expect("comment token");
    assert_eq!(comment.text, "#a\0");
    // Scanning resumes after the NUL, so the `+ 2` is still lexed.
    assert_eq!(
        types("1 #a\0+ 2"),
        vec![
            ExprTokenType::Number,
            ExprTokenType::Comment,
            ExprTokenType::Operator,
            ExprTokenType::Number,
        ]
    );
}

/// The gate: TIP 582 is 9.0+. Under an 8.x dialect `#` is still an unknown
/// character, which is what C 8.4-8.6 does with it — neither the `COMMENT`
/// lexeme nor `ParseLexeme`'s `case '#':` exists in `tclCompExpr.c` at
/// `core-8-5-19` / `core-8-6-16`, nor in 8.4's `tclParseExpr.c`.
#[test]
fn comments_are_tcl9_only() {
    for dialect in ["tcl9.0", "tcl9.1", "tcl"] {
        let (toks, unknown) = tokenise_expr_checked("1 + 2 # note", Some(dialect));
        assert!(!unknown, "{dialect} should accept a comment");
        assert!(
            toks.iter().any(|t| t.kind == ExprTokenType::Comment),
            "{dialect} should lex a comment token"
        );
    }
    // 8.x runtimes, and iRules (a genuine embedded 8.4).
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "f5-irules"] {
        let (toks, unknown) = tokenise_expr_checked("1 + 2 # note", Some(dialect));
        assert!(unknown, "{dialect} should reject `#` as unknown");
        assert!(
            !toks.iter().any(|t| t.kind == ExprTokenType::Comment),
            "{dialect} must not lex a comment token"
        );
    }
}
