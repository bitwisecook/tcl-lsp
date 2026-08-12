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

//! Semantic tokens, end-to-end against the packaged server. A fresh document is
//! opened per case and `textDocument/semanticTokens/full` is requested once,
//! then the delta-encoded `data` is decoded and the legend (advertised in
//! `initialize`) maps type/modifier indices back to names — so a legend or
//! encoder drift in the shipped artifact is caught here.

use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};

use serde_json::Value;

/// A decoded token with its resolved type-name.
#[derive(Debug, Clone)]
struct TypedToken {
    line: i64,
    char: i64,
    length: i64,
    ttype: String,
    modifiers: i64,
}

/// The `tokenTypes` legend advertised in `initialize`.
fn legend(lsp: &Lsp) -> Vec<String> {
    lsp.initialize_result()["capabilities"]["semanticTokensProvider"]["legend"]["tokenTypes"]
        .as_array()
        .expect("tokenTypes legend")
        .iter()
        .map(|v| v.as_str().unwrap_or("").to_owned())
        .collect()
}

/// The `tokenModifiers` legend advertised in `initialize`.
fn modifiers(lsp: &Lsp) -> Vec<String> {
    lsp.initialize_result()["capabilities"]["semanticTokensProvider"]["legend"]["tokenModifiers"]
        .as_array()
        .expect("tokenModifiers legend")
        .iter()
        .map(|v| v.as_str().unwrap_or("").to_owned())
        .collect()
}

/// Decode `uri`'s semantic tokens, mapping each type index to its legend name.
fn typed(lsp: &mut Lsp, legend: &[String], uri: &str) -> Vec<TypedToken> {
    // Converged, not first-response: several tests here assert on the
    // *enriched* tier (regex-source retagging, user-class method resolution),
    // which the server may legitimately deliver a refresh late when the 40 ms
    // fast-path budget loses. Reading only the first response makes those
    // assertions a bet on how much CPU the machine had — see
    // `Lsp::semantic_tokens_settled` (issue #1082).
    let raw = lsp.semantic_tokens_settled(uri);
    decode_semantic_tokens(&raw)
        .into_iter()
        .map(|t| TypedToken {
            line: t.line,
            char: t.char,
            length: t.length,
            ttype: legend[usize::try_from(t.ttype).unwrap()].clone(),
            modifiers: t.modifiers,
        })
        .collect()
}

/// Open `source` in a fresh URI, block until ready, return the URI.
fn open_doc(lsp: &mut Lsp, source: &str) -> String {
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, source);
    uri
}

/// The source text a decoded token covers (single-line tokens only).
fn covered<'a>(source: &'a str, tok: &TypedToken) -> &'a str {
    let line = source
        .split('\n')
        .nth(usize::try_from(tok.line).unwrap())
        .unwrap_or("");
    let start = usize::try_from(tok.char).unwrap();
    let end = usize::try_from(tok.char + tok.length).unwrap();
    // Slicing by UTF-16-ish char offset; the covered() cases here are all ASCII.
    line.get(start..end).unwrap_or("")
}

/// The bit mask for a modifier name.
fn modifier_bit(mods: &[String], name: &str) -> i64 {
    let idx = mods
        .iter()
        .position(|m| m == name)
        .expect("modifier in legend");
    1i64 << idx
}

// -- TestTokenInvariants -------------------------------------------------

/// Representative + adversarial documents the encoder must keep coherent.
fn invariant_corpus() -> Vec<(&'static str, &'static str)> {
    let mut v = vec![
        ("empty", ""),
        ("simple", "puts hello\n"),
        ("vars_and_numbers", "set x 42\nset y $x\nputs $y\n"),
        (
            "proc_with_body",
            "proc greet {name} {\n    puts \"Hello $name\"\n}\ngreet World\n",
        ),
        (
            "nested_blocks",
            "proc p {} {\n  if {1} {\n    foreach x {a b c} {\n      puts $x\n    }\n  }\n}\n",
        ),
        ("string_interp", "set s \"a $b [llength $c] d\"\n"),
        (
            "comments",
            "# leading comment\nputs hi ;# trailing comment\n",
        ),
        // Issue #759: a `#` comment ending in an unescaped backslash continues
        // onto the next physical line; the continuation must stay a comment and
        // its per-line tokens must satisfy the bounds/overlap invariants.
        (
            "comment_continuation",
            "# a comment \\\nstill a comment\nset x 1\n",
        ),
        (
            "comment_continuation_crlf",
            "# a comment \\\r\nstill a comment\r\nset x 1\r\n",
        ),
        ("crlf", "proc p {} {\r\n    set x 1\r\n}\r\n"),
        (
            "multibyte_string",
            "set greeting \"héllo wörld café\"\nputs $greeting\n",
        ),
        // A backslash immediately before a non-ASCII char used to slice a
        // fixed 2 bytes at the escape, splitting the multibyte char and
        // panicking the whole semanticTokens/full request. The escape now
        // spans the backslash plus the full UTF-8 char; drive it through the
        // server so the bounds/overlap invariants (and non-panic) are checked.
        ("escape_before_multibyte", "puts \"\\é\"\nset after 1\n"),
        (
            "escape_before_multibyte_run",
            "puts \"x\\é\\你\\€y\"\nset after 1\n",
        ),
        ("emoji_string", "set e \"😀 tcl 🚀 rocks 🐫\"\nputs $e\n"),
        ("emoji_then_code", "puts \"🚀\"\nset after 1\n"),
        (
            "unicode_after_var",
            "set x 1\nputs \"$x — résumé — 日本語\"\n",
        ),
        ("unterminated_bracket", "set x [foo bar\nputs hi\n"),
        ("unterminated_brace", "proc p {} {\n  set y [foo\n}\nset\n"),
        ("deep_nesting", "set x [a [b [c [d [e\nputs tail\n"),
        ("regexp_subtokens", "regexp {(\\d+)-(\\w+)} $s -> a b\n"),
        (
            "switch_braced",
            "switch $x {\n  {a b} { puts one }\n  default { puts def }\n}\n",
        ),
        // Issue #757 review (Codex P1): a `#`-leading physical line inside a
        // multi-line literal must not produce a `comment` token overlapping the
        // per-line `string` token.  Exercises the non-overlap invariant.
        (
            "hash_in_multiline_literal",
            "set x {a\n# not a comment\nb}\nset y \"a\n  # also literal\nb\"\n",
        ),
    ];
    v.sort_by(|a, b| a.0.cmp(b.0));
    v
}

/// Universal semantic-token invariants over the representative/adversarial corpus.
#[test]
fn test_tokens_satisfy_invariants() {
    let mut lsp = Lsp::tcl();
    let legend_names = legend(&lsp);
    let mod_names = modifiers(&lsp);
    let types_ref: Vec<&str> = legend_names.iter().map(String::as_str).collect();
    let mods_ref: Vec<&str> = mod_names.iter().map(String::as_str).collect();
    for (name, source) in invariant_corpus() {
        let uri = open_doc(&mut lsp, source);
        let raw = lsp.semantic_tokens(&uri);
        let violations = semantic_token_violations(&raw, source, &types_ref, &mods_ref);
        assert!(
            violations.is_empty(),
            "[{name}] semantic-token invariant violations:\n{}",
            violations.join("\n")
        );
    }
}

#[test]
fn test_tokens_strictly_non_overlapping_dense_line() {
    // A single dense line with many adjacent tokens is the worst case for the
    // overlap/ordering invariant.
    let mut lsp = Lsp::tcl();
    let legend_names = legend(&lsp);
    let mod_names = modifiers(&lsp);
    let types_ref: Vec<&str> = legend_names.iter().map(String::as_str).collect();
    let mods_ref: Vec<&str> = mod_names.iter().map(String::as_str).collect();
    let source = "set a 1;set b 2;puts \"$a [expr {$a+$b}] $b\";# tail\n";
    let uri = open_doc(&mut lsp, source);
    let raw = lsp.semantic_tokens(&uri);
    assert!(semantic_token_violations(&raw, source, &types_ref, &mods_ref).is_empty());
}

#[test]
fn test_escape_before_multibyte_char_does_not_panic() {
    // Regression: `\<non-ASCII>` (`\é`, `\你`, `\€`) inside a string used to
    // slice a fixed 2 bytes at the backslash, landing inside the multibyte
    // char and panicking the whole `textDocument/semanticTokens/full`
    // request. Drive the real request through the server and require it to
    // succeed AND emit an `escape` sub-token for the `\X` run.
    let mut lsp = Lsp::tcl();
    let legend_names = legend(&lsp);
    for source in [
        "puts \"\\é\"\n",
        "puts \"a\\你b\"\n",
        "puts \"\\€\"\n",
        "puts \"x\\é\\你\"\n",
    ] {
        let uri = open_doc(&mut lsp, source);
        let toks = typed(&mut lsp, &legend_names, &uri);
        assert!(
            toks.iter().any(|t| t.ttype == "escape"),
            "expected an `escape` token for {source:?}, got {toks:?}",
        );
    }
}

// -- TestCoreTokens ------------------------------------------------------

#[test]
fn test_simple_puts() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "puts hello\n");
    let tokens = typed(&mut lsp, &lg, &uri);
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].ttype, "function");
    assert_eq!(tokens[0].length, 4);
    assert_eq!(tokens[1].ttype, "string");
}

#[test]
fn range_request_honours_semantic_tokens_toggle() {
    // Disabling `semanticTokens` must silence the viewport (`range`) request
    // too, not just `full` / `full/delta` (issue 174).
    let mut lsp = Lsp::with_config(serde_json::json!({
        "features": { "linkedEditingRange": true, "semanticTokens": false }
    }));
    let uri = open_doc(&mut lsp, "set x 1\nputs $x\n");
    let full = lsp.request(
        "textDocument/semanticTokens/full",
        serde_json::json!({ "textDocument": { "uri": uri.clone() } }),
    );
    assert!(full.is_null(), "full must be disabled: {full}");
    let ranged = lsp.request(
        "textDocument/semanticTokens/range",
        serde_json::json!({
            "textDocument": { "uri": uri },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end":   { "line": 1, "character": 8 }
            }
        }),
    );
    assert!(
        ranged.is_null(),
        "range must be disabled by the same toggle: {ranged}"
    );
}

#[test]
fn test_variable() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "set x $y\n");
    let types: Vec<String> = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .map(|t| t.ttype)
        .collect();
    assert!(types.iter().any(|t| t == "function"));
    assert!(types.iter().any(|t| t == "variable"));
}

#[test]
fn test_number() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "set x 42\n");
    let nums: Vec<TypedToken> = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .filter(|t| t.ttype == "number")
        .collect();
    assert_eq!(nums.len(), 1);
    assert_eq!(nums[0].length, 2);
}

#[test]
fn test_comment() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "# hello world\n");
    assert_eq!(typed(&mut lsp, &lg, &uri)[0].ttype, "comment");
}

#[test]
fn test_comment_continuation() {
    // Issue #759, end-to-end through the packaged server: a `#` comment whose
    // line ends in an unescaped backslash continues onto the next line, and an
    // even (escaped) backslash run does not.
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);

    let uri = open_doc(&mut lsp, "# a comment \\\nstill a comment\nset x 1\n");
    let toks = typed(&mut lsp, &lg, &uri);
    let comment_lines: std::collections::BTreeSet<i64> = toks
        .iter()
        .filter(|t| t.ttype == "comment")
        .map(|t| t.line)
        .collect();
    assert!(
        comment_lines.contains(&0) && comment_lines.contains(&1),
        "{comment_lines:?}"
    );
    assert!(!comment_lines.contains(&2), "{comment_lines:?}");
    assert!(
        toks.iter().any(|t| t.line == 2 && t.ttype == "function"),
        "line 2 is code: {toks:?}"
    );

    // Escaped backslash pair — no continuation.
    let uri = open_doc(&mut lsp, "# escaped end \\\\\nset z 3\n");
    let toks = typed(&mut lsp, &lg, &uri);
    let comment_lines: std::collections::BTreeSet<i64> = toks
        .iter()
        .filter(|t| t.ttype == "comment")
        .map(|t| t.line)
        .collect();
    assert!(comment_lines.contains(&0), "{comment_lines:?}");
    assert!(
        !comment_lines.contains(&1),
        "escaped backslash must not continue: {comment_lines:?}"
    );

    // A `;#` tail comment (command position after a separator) is a comment and
    // continues across a trailing backslash.
    let uri = open_doc(&mut lsp, "puts hi ;# tail \\\nmore\nset y 2\n");
    let toks = typed(&mut lsp, &lg, &uri);
    let comment_lines: std::collections::BTreeSet<i64> = toks
        .iter()
        .filter(|t| t.ttype == "comment")
        .map(|t| t.line)
        .collect();
    assert!(
        comment_lines.contains(&0) && comment_lines.contains(&1),
        "expected `;#` tail comment + continuation: {comment_lines:?}"
    );
    assert!(!comment_lines.contains(&2), "{comment_lines:?}");
}

#[test]
fn test_proc_as_keyword() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "proc foo {x} {}\n");
    assert_eq!(typed(&mut lsp, &lg, &uri)[0].ttype, "keyword");
}

#[test]
fn test_user_command_as_function() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "mycommand arg1\n");
    assert_eq!(typed(&mut lsp, &lg, &uri)[0].ttype, "function");
}

#[test]
fn test_operator() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "+ 3 4\n");
    assert_eq!(typed(&mut lsp, &lg, &uri)[0].ttype, "operator");
}

#[test]
fn test_multiline_positions() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "set x 1\nset y 2\n");
    let sets: Vec<TypedToken> = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .filter(|t| t.ttype == "function" && t.length == 3)
        .collect();
    assert_eq!(sets.len(), 2);
    assert_eq!(sets[0].line, 0);
    assert_eq!(sets[1].line, 1);
}

#[test]
fn test_data_is_multiple_of_5() {
    let mut lsp = Lsp::tcl();
    let uri = open_doc(&mut lsp, "set x [+ 1 2]\nputs $x\n");
    let data = lsp.semantic_tokens(&uri)["data"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert_eq!(data.len() % 5, 0);
}

#[test]
fn test_empty_source() {
    let mut lsp = Lsp::tcl();
    let uri = open_doc(&mut lsp, "");
    assert_eq!(lsp.semantic_tokens(&uri)["data"], Value::Array(vec![]));
}

#[test]
fn test_string_in_quotes() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "puts \"hello\"\n");
    let types: Vec<String> = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .map(|t| t.ttype)
        .collect();
    assert!(types.iter().any(|t| t == "function"));
    assert!(types.iter().any(|t| t == "string"));
}

#[test]
fn test_braced_string() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "puts {hello world}\n");
    let types: Vec<String> = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .map(|t| t.ttype)
        .collect();
    assert!(types.iter().any(|t| t == "string"));
}

#[test]
fn test_multiline_braced_string_highlighted_per_line() {
    // Issue #757: a braced string literal spanning multiple lines used to lose
    // its highlighting (the enclosing multi-line `string` token was dropped).
    // End-to-end it must now carry a `string` token on every covered line,
    // exactly like the quoted form.
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let braced = open_doc(
        &mut lsp,
        "set x {some long\nstring that spans\nmultiple lines}\n",
    );
    let btoks = typed(&mut lsp, &lg, &braced);
    for line in 0..=2 {
        assert!(
            btoks.iter().any(|t| t.line == line && t.ttype == "string"),
            "braced literal line {line} must carry a string token: {btoks:?}",
        );
    }
    // The quoted counterpart highlights the same span the same way.
    let quoted = open_doc(
        &mut lsp,
        "set x \"some long\nstring that spans\nmultiple lines\"\n",
    );
    let qtoks = typed(&mut lsp, &lg, &quoted);
    let strings = |toks: &[TypedToken]| -> Vec<(i64, i64, i64)> {
        toks.iter()
            .filter(|t| t.ttype == "string")
            .map(|t| (t.line, t.char, t.length))
            .collect()
    };
    assert_eq!(
        strings(&btoks),
        strings(&qtoks),
        "braced and quoted multi-line string literals must highlight identically",
    );
    // No emitted token may span a newline (LSP encoding invariant).
    for t in &btoks {
        let line = "set x {some long\nstring that spans\nmultiple lines}\n"
            .split('\n')
            .nth(usize::try_from(t.line).unwrap())
            .unwrap_or("");
        assert!(
            usize::try_from(t.char + t.length).unwrap() <= line.chars().count(),
            "token {t:?} overruns its line",
        );
    }
}

#[test]
fn test_if_elseif_else_body_recursion() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(
        &mut lsp,
        "if {$x} { set a 1 } elseif {$y} { set b 2 } else { set c 3 }\n",
    );
    let sets: Vec<TypedToken> = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .filter(|t| t.ttype == "function" && t.length == 3)
        .collect();
    assert_eq!(sets.len(), 3);
}

#[test]
fn test_if_expression_tokenised() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "if {$x > 0} { puts ok }\n");
    let types: std::collections::BTreeSet<String> = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .map(|t| t.ttype)
        .collect();
    for want in ["variable", "operator", "number"] {
        assert!(types.contains(want), "missing {want:?} in {types:?}");
    }
}

#[test]
fn test_command_subst_inside_expression() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "set n [expr {[llength $xs] + 1}]\n");
    let tokens = typed(&mut lsp, &lg, &uri);
    assert!(
        tokens
            .iter()
            .any(|t| t.ttype == "function" && t.length == 7)
    );
    assert!(tokens.iter().any(|t| t.ttype == "variable"));
}

// -- TestTcllibBodyRecursion ---------------------------------------------
//
// tcllib commands that carry a script body (`control::do`,
// `struct::list foreachperm`) or an expression (`control::do`'s test,
// `control::assert`) must recurse into that argument rather than emit it
// as one opaque string — same treatment as core `while`/`if` (issue #760).

#[test]
fn test_control_do_body_recursion() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    // The `body` script and the `while` test expression both recurse:
    // `set` inside the body and `$x` inside the expression are tokenised.
    let src = "package require control\ncontrol::do {\n set x 1\n} while {$x < 10}\n";
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &lg, &uri);
    // `set` recursed from the braced body.
    let has_set = tokens
        .iter()
        .any(|t| t.ttype == "function" && t.length == 3);
    assert!(has_set, "control::do body not recursed: {tokens:?}");
    // `$x` recursed from the `while` expression.
    let has_var = tokens.iter().any(|t| t.ttype == "variable");
    assert!(has_var, "control::do test expr not recursed: {tokens:?}");
}

#[test]
fn test_control_do_while_is_keyword() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let source = "package require control\ncontrol::do {\n incr n\n} while {$n < 3}\n";
    let uri = open_doc(&mut lsp, source);
    let words = keyword_words(&mut lsp, &lg, &uri, source);
    assert!(words.contains("while"), "expected `while`: {words:?}");
}

#[test]
fn test_control_assert_expression_tokenised() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let src = "package require control\ncontrol::assert {$x == 10}\n";
    let uri = open_doc(&mut lsp, src);
    let types: std::collections::BTreeSet<String> = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .map(|t| t.ttype)
        .collect();
    for want in ["variable", "operator"] {
        assert!(types.contains(want), "missing {want:?} in {types:?}");
    }
}

#[test]
fn test_struct_list_foreachperm_body_recursion() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    // `struct::list foreachperm var sequence body` — the trailing body
    // script recurses, so `puts` inside it is a function token.
    let src = "package require struct::list\nstruct::list foreachperm p {a b c} {\n puts $p\n}\n";
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &lg, &uri);
    let has_puts = tokens
        .iter()
        .any(|t| t.ttype == "function" && t.length == 4);
    assert!(has_puts, "foreachperm body not recursed: {tokens:?}");
    let has_var = tokens.iter().any(|t| t.ttype == "variable");
    assert!(has_var, "foreachperm var not recursed: {tokens:?}");
}

#[test]
fn test_uplevel_body_recursion() {
    // Issue #837: the braced body of `uplevel ?level? {…}` is a script and
    // must recurse end-to-end through the packaged server — `foreach` inside
    // it is a keyword and `puts` a function, not one opaque string.
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    for src in [
        "uplevel 1 {foreach x $l { puts $x }}\n",
        "uplevel {foreach x $l { puts $x }}\n",
        "uplevel #0 {foreach x $l { puts $x }}\n",
    ] {
        let uri = open_doc(&mut lsp, src);
        let tokens = typed(&mut lsp, &lg, &uri);
        let has_foreach = tokens
            .iter()
            .any(|t| t.ttype == "keyword" && covered(src, t) == "foreach");
        assert!(
            has_foreach,
            "uplevel body not recursed for {src:?}: {tokens:?}"
        );
        let has_puts = tokens
            .iter()
            .any(|t| t.ttype == "function" && covered(src, t) == "puts");
        assert!(
            has_puts,
            "uplevel body `puts` not recursed for {src:?}: {tokens:?}"
        );
    }
}

#[test]
fn test_uplevel_issue_837_repro_recursion() {
    // The exact reproducer from issue #837 — the `namespace children` /
    // `namespace forget` body inside `uplevel 1 {…}` highlights end-to-end.
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let src = "proc forgetXyce {} {\n    uplevel 1 {foreach nameSpc [namespace children ::SpiceGenTcl::Xyce] {\n        namespace forget ${nameSpc}::*\n    }}\n}\n";
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &lg, &uri);
    let has_foreach = tokens
        .iter()
        .any(|t| t.ttype == "keyword" && covered(src, t) == "foreach");
    assert!(
        has_foreach,
        "issue #837 body `foreach` not recursed: {tokens:?}"
    );
    // `namespace` appears twice inside the body; at least one must highlight.
    let has_namespace = tokens
        .iter()
        .any(|t| covered(src, t) == "namespace" && (t.ttype == "keyword" || t.ttype == "function"));
    assert!(
        has_namespace,
        "issue #837 body `namespace` not recursed: {tokens:?}"
    );
}

#[test]
fn test_bind_script_body_recursion() {
    // `bind tag sequence script` — the trailing event-handler script recurses
    // rather than being emitted as one opaque string (issue #785), so `set`
    // inside it is a function token and `$w` a variable.
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let src = "bind $w <KeyPress> {\n if {$w eq \"Menu\"} {\n  set x 1\n }\n}\n";
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &lg, &uri);
    // `set` recursed from the braced event-handler body.
    let has_set = tokens
        .iter()
        .any(|t| covered(src, t) == "set" && t.ttype == "function");
    assert!(has_set, "bind script body not recursed: {tokens:?}");
    // `$w` recursed from inside the body.
    let has_var = tokens.iter().any(|t| t.ttype == "variable");
    assert!(has_var, "bind script body vars not recursed: {tokens:?}");
    // `if` inside the body is a keyword.
    let words = keyword_words(&mut lsp, &lg, &uri, src);
    assert!(
        words.contains("if"),
        "expected `if` keyword in bind body: {words:?}"
    );
}

#[test]
fn test_bind_query_forms_do_not_recurse() {
    // `bind tag` and `bind tag sequence` are query forms with no script — a
    // braced word in a two-argument call is opaque, not a recursed body.
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let src = "bind $w {puts hi}\n";
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &lg, &uri);
    let puts_recursed = tokens
        .iter()
        .any(|t| covered(src, t) == "puts" && t.ttype == "function");
    assert!(
        !puts_recursed,
        "bind query form must not recurse a body: {tokens:?}"
    );
}

// -- TestStructuralKeywords ----------------------------------------------

/// The set of source words rendered as keyword tokens.
fn keyword_words(
    lsp: &mut Lsp,
    lg: &[String],
    uri: &str,
    source: &str,
) -> std::collections::BTreeSet<String> {
    typed(lsp, lg, uri)
        .into_iter()
        .filter(|t| t.ttype == "keyword")
        .map(|t| covered(source, &t).to_owned())
        .collect()
}

#[test]
fn test_if_else_elseif_are_keywords() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let source = "if 1 {\n puts a\n} elseif 2 {\n puts b\n} else {\n puts c\n}\n";
    let uri = open_doc(&mut lsp, source);
    let words = keyword_words(&mut lsp, &lg, &uri, source);
    for want in ["if", "elseif", "else"] {
        assert!(words.contains(want), "{words:?}");
    }
}

#[test]
fn test_try_on_finally_are_keywords() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let source = "try {\n set x 1\n} on error {e} {\n puts $e\n} finally {\n puts d\n}\n";
    let uri = open_doc(&mut lsp, source);
    let words = keyword_words(&mut lsp, &lg, &uri, source);
    for want in ["try", "on", "finally"] {
        assert!(words.contains(want), "{words:?}");
    }
}

#[test]
fn test_builtin_name_as_bareword_arg_is_string() {
    // `proc` here is a plain dict value, not the command-definition keyword — it
    // must stay a string (the bareword-builtin glitch #637).
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let source = "dict set frame proc \"asasdas asd\"\n";
    let uri = open_doc(&mut lsp, source);
    let tokens = typed(&mut lsp, &lg, &uri);
    let proc_tok = tokens
        .iter()
        .find(|t| covered(source, t) == "proc")
        .expect("proc token");
    assert_eq!(proc_tok.ttype, "string", "{proc_tok:?}");
}

#[test]
fn test_quoted_structural_keyword_offsets_past_quote() {
    // A quoted `"else"` is an ESC-token word whose start sits on the opening
    // quote; PR #643 emits the keyword from the content base so the range covers
    // `else`, not `"els`.
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let source = "if 0 {} \"else\" {puts ok}\n";
    let uri = open_doc(&mut lsp, source);
    let tokens = typed(&mut lsp, &lg, &uri);
    let kw = tokens
        .iter()
        .find(|t| t.ttype == "keyword" && t.char >= 8)
        .expect("keyword past char 8");
    assert_eq!(
        covered(source, kw),
        "else",
        "{:?} {:?}",
        kw,
        covered(source, kw)
    );
}

// -- TestRegexTokens -----------------------------------------------------

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

fn is_re_type(t: &str) -> bool {
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

/// Regression for #758: the braced case-list form of a plain (non-`-regexp`)
/// `switch` must recurse each body as a script so the commands inside are
/// highlighted, rather than treating the whole `{ pat body … }` list as one
/// opaque body.  Before the fix the bodies received no tokens at all.
#[test]
fn test_switch_plain_braced_case_list_recurses_bodies() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let source = "switch -exact -- $var {\n    \"a\" {\n        set x 1\n        puts hi\n    }\n    default {\n        return 2\n    }\n}\n";
    let uri = open_doc(&mut lsp, source);
    let tokens = typed(&mut lsp, &lg, &uri);
    // Body commands are recursed and highlighted as functions.
    let fns: Vec<&str> = tokens
        .iter()
        .filter(|t| t.ttype == "function")
        .map(|t| covered(source, t))
        .collect();
    assert!(fns.contains(&"set"), "set not highlighted: {fns:?}");
    assert!(fns.contains(&"puts"), "puts not highlighted: {fns:?}");
    // `return` is a control-flow keyword inside the second body.
    assert!(
        tokens
            .iter()
            .any(|t| t.ttype == "keyword" && covered(source, t) == "return"),
        "expected `return` in the default body to be a keyword",
    );
    // Plain (non-regexp) mode emits no regex sub-tokens.
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

// -- TestEventDecoratorNamespace -----------------------------------------

#[test]
fn test_when_event_name_highlighted_as_event() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "when HTTP_REQUEST { puts hello }\n");
    let events: Vec<TypedToken> = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .filter(|t| t.ttype == "event")
        .collect();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].length,
        i64::try_from("HTTP_REQUEST".len()).unwrap()
    );
}

#[test]
fn test_when_lowercase_arg_not_event() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "when some_thing { puts ok }\n");
    let events: Vec<TypedToken> = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .filter(|t| t.ttype == "event")
        .collect();
    assert!(events.is_empty());
}

#[test]
fn test_regexp_option_highlighted_as_decorator() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "regexp -nocase {pat} $str\n");
    let decs: Vec<TypedToken> = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .filter(|t| t.ttype == "decorator")
        .collect();
    assert!(
        decs.iter()
            .any(|t| t.length == i64::try_from("-nocase".len()).unwrap())
    );
}

#[test]
fn test_regex_source_variable_highlights_def_site_literal() {
    // End-to-end proof: a variable holding a regex literal that flows into a
    // `regexp` pattern makes the ORIGINATING `set` literal read as a regex.
    // This exercises the full salsa pipeline (semantic_tokens query demanding
    // the CompilationUnit), not just the token pass.
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "set my_re \".*abc\"\nregexp $my_re $s\n");
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

/// Issue #967 end-to-end: `return -code error "bad"` must reach the client
/// with `-code` as a `decorator` and `error` as an `enumMember`, not as
/// plain strings.  The `tcl-lsp-core` unit test pins the classifier; this
/// pins the whole server pipeline (registry `OptionSpec` → token pass →
/// legend-encoded `textDocument/semanticTokens/full` response), which is what
/// the editor actually sees.
#[test]
fn test_return_code_option_decorator_issue_967() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "return -code error \"bad\"\n");
    let toks = typed(&mut lsp, &lg, &uri);
    assert!(
        toks.iter()
            .any(|t| t.ttype == "decorator" && t.length == i64::try_from("-code".len()).unwrap()),
        "expected `-code` to arrive as a decorator; got {toks:?}"
    );
    assert!(
        toks.iter()
            .any(|t| t.ttype == "enumMember" && t.length == i64::try_from("error".len()).unwrap()),
        "expected `error` to arrive as an enumMember; got {toks:?}"
    );

    // TN: the same word passed to a command that declares no options at all
    // stays a plain argument.
    let uri2 = open_doc(&mut lsp, "concat -code error\n");
    let toks2 = typed(&mut lsp, &lg, &uri2);
    assert!(
        !toks2.iter().any(|t| t.ttype == "decorator"),
        "`-code` is not a declared option of `concat`; got {toks2:?}"
    );
}

#[test]
fn test_non_option_dash_word_not_decorator() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "puts -foo\n");
    let decs: Vec<TypedToken> = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .filter(|t| t.ttype == "decorator")
        .collect();
    assert!(decs.is_empty());
}

#[test]
fn test_oo_class_split() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "oo::class create Dog {}\n");
    let tokens = typed(&mut lsp, &lg, &uri);
    assert!(
        tokens
            .iter()
            .any(|t| t.ttype == "namespace" && t.length == i64::try_from("oo::".len()).unwrap())
    );
    assert!(
        tokens
            .iter()
            .any(|t| t.ttype == "keyword" && t.length == i64::try_from("class".len()).unwrap())
    );
}

#[test]
fn test_global_qualified_command() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "::set x 1\n");
    let ns: Vec<TypedToken> = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .filter(|t| t.ttype == "namespace")
        .collect();
    assert_eq!(ns.len(), 1);
    assert_eq!(ns[0].length, i64::try_from("::".len()).unwrap());
}

// -- TestModifiers -------------------------------------------------------

#[test]
fn test_builtin_command_has_default_library() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let mods = modifiers(&lsp);
    let uri = open_doc(&mut lsp, "puts hello\n");
    let cmd = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .find(|t| t.ttype == "function")
        .expect("function token");
    assert!(cmd.modifiers & modifier_bit(&mods, "defaultLibrary") != 0);
}

#[test]
fn test_user_command_no_default_library() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let mods = modifiers(&lsp);
    let uri = open_doc(&mut lsp, "mycommand arg\n");
    let cmd = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .find(|t| t.ttype == "function")
        .expect("function token");
    assert_eq!(cmd.modifiers & modifier_bit(&mods, "defaultLibrary"), 0);
}

#[test]
fn test_subcommand_has_default_library() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let mods = modifiers(&lsp);
    let uri = open_doc(&mut lsp, "string length foo\n");
    let sub: Vec<TypedToken> = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .filter(|t| t.ttype == "keyword" && t.char == 7)
        .collect();
    assert_eq!(sub.len(), 1);
    assert!(sub[0].modifiers & modifier_bit(&mods, "defaultLibrary") != 0);
}

#[test]
fn test_expr_function_has_default_library() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let mods = modifiers(&lsp);
    let uri = open_doc(&mut lsp, "expr {abs(-1)}\n");
    let fns: Vec<TypedToken> = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .filter(|t| t.ttype == "function" && t.length == 3)
        .collect();
    assert_eq!(fns.len(), 1);
    assert!(fns[0].modifiers & modifier_bit(&mods, "defaultLibrary") != 0);
}

#[test]
fn test_proc_definition_no_default_library() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let mods = modifiers(&lsp);
    let uri = open_doc(&mut lsp, "proc greet {name} {}\n");
    let defn = modifier_bit(&mods, "definition");
    let fn_tok = typed(&mut lsp, &lg, &uri)
        .into_iter()
        .find(|t| t.ttype == "function" && (t.modifiers & defn != 0))
        .expect("definition function token");
    assert_eq!(fn_tok.modifiers & modifier_bit(&mods, "defaultLibrary"), 0);
}

// -- TestStringContentTokens ---------------------------------------------

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

// -- georgtree issues #774 / #775 / #776 (end-to-end) --------------------

#[test]
fn test_tcloo_body_variable_declares_every_name() {
    // Issue #774: `variable a b c` in a TclOO class body declares every name as
    // an instance variable, not just the first.
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let src = "oo::class create Foo {\n    variable width height depth\n}\n";
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &lg, &uri);
    for name in ["width", "height", "depth"] {
        let t = tokens
            .iter()
            .find(|t| covered(src, t) == name)
            .unwrap_or_else(|| panic!("no token covers {name:?}: {tokens:?}"));
        assert_eq!(
            t.ttype, "variable",
            "`{name}` must be a variable: {tokens:?}"
        );
    }
}

#[test]
fn test_imported_tcltest_test_structure_recognised() {
    // Issue #776: after `namespace import tcltest::*`, a bare `test` resolves to
    // the tcltest spec — its `-body`/`-result` are options and the body script
    // is recursed.
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let src = "namespace import tcltest::*\n\
               test t-1 {desc} -body {\n    set x 1\n} -result 1\n";
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &lg, &uri);
    let kind_of = |word: &str| {
        tokens
            .iter()
            .find(|t| covered(src, t) == word)
            .map(|t| t.ttype.clone())
    };
    assert_eq!(
        kind_of("-body").as_deref(),
        Some("decorator"),
        "imported test's -body must be an option: {tokens:?}",
    );
    assert_eq!(
        kind_of("-result").as_deref(),
        Some("decorator"),
        "imported test's -result must be an option: {tokens:?}",
    );
    assert_eq!(
        kind_of("set").as_deref(),
        Some("function"),
        "the -body script must be recursed: {tokens:?}",
    );
}

#[test]
fn test_source_command_substitution_argument_is_tokenised() {
    // Issue #775: a command substitution in `source`'s argument is highlighted
    // as a command sequence (locks the rust-branch behaviour).
    //
    // The whole sequence, in order, not spot checks on two words: the point of
    // the issue is that the argument reads as *code*, and that claim is only
    // as strong as the weakest word in it.  `join` in particular has to be the
    // subcommand keyword rather than the `join` list command that shares its
    // spelling — a lookup that goes through `file`'s ensemble, so getting it
    // right is evidence the substitution was resolved and not merely lexed.
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let mods = modifiers(&lsp);
    let src = "source [file join $currentDir testUtilities.tcl]\n";
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &lg, &uri);
    let actual: Vec<(&str, &str)> = tokens
        .iter()
        .map(|t| (covered(src, t), t.ttype.as_str()))
        .collect();
    assert_eq!(
        actual,
        vec![
            ("source", "keyword"),
            ("file", "function"),
            ("join", "keyword"),
            ("$currentDir", "variable"),
            ("testUtilities.tcl", "string"),
        ],
        "the [file join …] substitution must tokenise as a command sequence",
    );
    // `file` and `join` resolve to the built-in ensemble, not to user code —
    // the modifier an editor colours differently from a local proc call.
    let library = modifier_bit(&mods, "defaultLibrary");
    for word in ["file", "join"] {
        let tok = tokens
            .iter()
            .find(|t| covered(src, t) == word)
            .unwrap_or_else(|| panic!("no token covers {word:?}"));
        assert!(
            tok.modifiers & library != 0,
            "`{word}` must carry defaultLibrary: {tok:?}",
        );
    }
}

#[test]
fn test_document_link_never_spans_more_than_one_semantic_token() {
    // Issue #775, the second time.  Correct semantic tokens (the test above)
    // are not enough to make the argument *look* highlighted: an editor paints
    // a document-link range in one flat link colour plus an underline, so a
    // link spanning `file join $currentDir esd_pulse_circuit.tcl` erases every
    // token boundary inside it and the whole substitution reads as one word —
    // exactly what the reporter's screenshots show, both times.
    //
    // The token assertions above passed throughout that, which is why they did
    // not hold the fix.  This is the property that actually failed: a link may
    // cover at most one token, because covering two means hiding the boundary
    // between them.  Stated over links in general, so it also holds for
    // `package require` and for whatever gains a link next.
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let src = "package require tcltest\n\
               set currentDir [file dirname [info script]]\n\
               source [file join $currentDir esd_pulse_circuit.tcl]\n\
               source helper.tcl\n";
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &lg, &uri);
    let links = lsp.document_links(&uri);
    let links = links.as_array().expect("documentLink array");
    assert!(
        !links.is_empty(),
        "expected links to assert about: {links:?}"
    );
    for link in links {
        let range = &link["range"];
        let (line, start, end) = (
            range["start"]["line"].as_i64().expect("start line"),
            range["start"]["character"].as_i64().expect("start char"),
            range["end"]["character"].as_i64().expect("end char"),
        );
        assert_eq!(
            line,
            range["end"]["line"].as_i64().expect("end line"),
            "a source/package link is single-line: {link}",
        );
        let overlapping: Vec<&str> = tokens
            .iter()
            .filter(|t| t.line == line && t.char < end && t.char + t.length > start)
            .map(|t| covered(src, t))
            .collect();
        assert!(
            overlapping.len() <= 1,
            "link over {:?} covers {} semantic tokens ({overlapping:?}) — an editor \
             paints it as one flat run, hiding every boundary inside it: {link}",
            src.split('\n')
                .nth(usize::try_from(line).unwrap())
                .unwrap_or("")
                .get(usize::try_from(start).unwrap()..usize::try_from(end).unwrap())
                .unwrap_or(""),
            overlapping.len(),
        );
    }
}

#[test]
fn test_global_declares_every_name() {
    // Peer of #774: `global a b c` declares every name as a variable.
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let src = "proc p {} {\n    global alpha beta gamma\n}\n";
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &lg, &uri);
    for name in ["alpha", "beta", "gamma"] {
        let t = tokens
            .iter()
            .find(|t| covered(src, t) == name)
            .unwrap_or_else(|| panic!("no token covers {name:?}: {tokens:?}"));
        assert_eq!(
            t.ttype, "variable",
            "`{name}` in `global` must be a variable"
        );
    }
}

#[test]
fn test_foreach_loop_variables_are_variables() {
    // `foreach`/`dict for` bind their iteration variables — a single bareword
    // and each element of a braced list read as variable declarations.
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let src = "foreach item $lst { puts $item }\nforeach {key val} $pairs { puts $key }\n";
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &lg, &uri);
    let is_var = |word: &str| {
        tokens
            .iter()
            .any(|t| covered(src, t) == word && t.ttype == "variable")
    };
    for name in ["item", "key", "val"] {
        assert!(
            is_var(name),
            "loop variable `{name}` must be a variable: {tokens:?}"
        );
    }
}

#[test]
fn test_proc_and_lambda_parameters_are_parameters() {
    // Procedure / apply-lambda parameter names carry the standard LSP
    // `parameter` type (#898 §4) — distinguishable from an ordinary local — while
    // `dict map`'s loop variables stay plain variable declarations.
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let src = "proc greet {name age} { return $name }\n\
               apply {{alpha beta} { return $alpha }} 1 2\n\
               dict map {mk mv} $d { set mk $mv }\n";
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &lg, &uri);
    let has = |word: &str, ttype: &str| {
        tokens
            .iter()
            .any(|t| covered(src, t) == word && t.ttype == ttype)
    };
    for name in ["name", "age", "alpha", "beta"] {
        assert!(
            has(name, "parameter"),
            "`{name}` must be a parameter declaration: {tokens:?}"
        );
    }
    for name in ["mk", "mv"] {
        assert!(
            has(name, "variable"),
            "loop var `{name}` must stay a variable declaration: {tokens:?}"
        );
    }
}

#[test]
fn test_upvar_and_dict_update_locals_are_variables() {
    // `upvar` / `namespace upvar` locals and `dict update` var names read as
    // variable declarations; the other-frame names and dict keys stay strings.
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let src = "upvar 1 othervar localvar\n\
               dict update mydict thekey thevar { set thevar 1 }\n";
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &lg, &uri);
    let kind_of = |word: &str| {
        tokens
            .iter()
            .find(|t| covered(src, t) == word)
            .map(|t| t.ttype.clone())
    };
    assert_eq!(
        kind_of("localvar").as_deref(),
        Some("variable"),
        "{tokens:?}"
    );
    assert_eq!(kind_of("thevar").as_deref(), Some("variable"), "{tokens:?}");
    assert_eq!(kind_of("othervar").as_deref(), Some("string"), "{tokens:?}");
    assert_eq!(kind_of("thekey").as_deref(), Some("string"), "{tokens:?}");
}

#[test]
fn test_snit_type_body_members_highlight() {
    // snit definition bodies are registry data (definition_body grammar), so
    // their members recurse + highlight with no snit-specific walker code.
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let src = "snit::type Dog {\n\
               \x20   variable barks\n\
               \x20   method bark {volume} { return $barks }\n\
               \x20   typemethod count {} { return 1 }\n\
               }\n";
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &lg, &uri);
    // `barks` is a declared variable; `volume` is a method *parameter* and now
    // carries the standard LSP `parameter` type (#898 §4).
    assert!(
        tokens
            .iter()
            .any(|t| covered(src, t) == "barks" && t.ttype == "variable"),
        "snit `barks` must be a variable: {tokens:?}",
    );
    assert!(
        tokens
            .iter()
            .any(|t| covered(src, t) == "volume" && t.ttype == "parameter"),
        "snit `volume` must be a parameter: {tokens:?}",
    );
    // The type name and the member names are `class` / `method`, not strings
    // (#898 §2).
    assert!(
        tokens
            .iter()
            .any(|t| covered(src, t) == "Dog" && t.ttype == "class"),
        "snit `Dog` must be a class: {tokens:?}",
    );
    for m in ["bark", "count"] {
        assert!(
            tokens
                .iter()
                .any(|t| covered(src, t) == m && t.ttype == "method"),
            "snit `{m}` must be a method: {tokens:?}",
        );
    }
    // `typemethod` is a member keyword; the method body's `return` is recursed.
    assert!(
        tokens
            .iter()
            .any(|t| covered(src, t) == "typemethod" && t.ttype == "keyword"),
        "`typemethod` must be a keyword: {tokens:?}",
    );
    assert!(
        tokens
            .iter()
            .any(|t| covered(src, t) == "return" && t.ttype == "keyword"),
        "a snit method body must be recursed: {tokens:?}",
    );
}

#[test]
fn test_itcl_class_body_members_highlight() {
    // [incr Tcl] class bodies are registry data (definition_body grammar) too,
    // including the public/protected/private access-modifier wrappers.
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let src = "itcl::class Dog {\n\
               \x20   inherit Animal\n\
               \x20   variable barks\n\
               \x20   public method bark {volume} { set barks $volume }\n\
               \x20   private variable secret 0\n\
               }\n";
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &lg, &uri);
    // Declared variables (incl. the access-modifier-wrapped ones) are variables.
    for name in ["barks", "secret"] {
        assert!(
            tokens
                .iter()
                .any(|t| covered(src, t) == name && t.ttype == "variable"),
            "itcl `{name}` must be a variable: {tokens:?}",
        );
    }
    // A method's *parameter* carries the standard LSP `parameter` type, so a
    // theme can tell an argument from an ordinary local (#898 §4).
    assert!(
        tokens
            .iter()
            .any(|t| covered(src, t) == "volume" && t.ttype == "parameter"),
        "itcl `volume` must be a parameter: {tokens:?}",
    );
    // The class name is a `class`, not a bare string (#898 §2).
    assert!(
        tokens
            .iter()
            .any(|t| covered(src, t) == "Dog" && t.ttype == "class"),
        "itcl `Dog` must be a class: {tokens:?}",
    );
    // The method's declared name is a `method`, not a bare string (#898 §2).
    assert!(
        tokens
            .iter()
            .any(|t| covered(src, t) == "bark" && t.ttype == "method"),
        "itcl `bark` must be a method: {tokens:?}",
    );
    // The access modifier and the inner wrapped keyword are both keywords.
    for kw in ["inherit", "public", "method", "private"] {
        assert!(
            tokens
                .iter()
                .any(|t| covered(src, t) == kw && t.ttype == "keyword"),
            "itcl `{kw}` must be a keyword: {tokens:?}",
        );
    }
    // The wrapped method body is recursed (`set` is a command there).
    assert!(
        tokens
            .iter()
            .any(|t| covered(src, t) == "set" && t.ttype == "function"),
        "an itcl method body must be recursed: {tokens:?}",
    );
}

/// APL (iApp presentation) is not Tcl: it must be tokenised by its own lexer,
/// not run through the Tcl pipeline.
///
/// Regression for the 2.1.6 state, where a `.apl` document fell through to the
/// Tcl tokenizer, which read each braced block as one literal word and emitted
/// whole *lines* as `string` tokens — mis-colouring the file rather than merely
/// under-colouring it.
#[test]
fn test_apl_presentation_uses_the_apl_token_set() {
    let mut lsp = Lsp::tcl();
    let leg = legend(&lsp);
    let src = concat!(
        "#include \"f5.apl_common\"\n",
        "\n",
        "# a comment\n",
        "define choice yesno_choice {\n",
        "    \"Yes\" => \"yes\"\n",
        "}\n",
        "\n",
        "section basic {\n",
        "    string addr required validator \"IpAddress\"\n",
        "    message \"Welcome to the template.\"\n",
        "}\n",
    );
    let uri = unique_uri("apl");
    // Not `open_ready_lang`: an APL document is not indexed into workspace
    // state, so its `workspace_state.update` log never arrives. Await the
    // diagnostics publish instead — `semanticTokens/full` is ordered behind the
    // `didOpen` that produced it.
    lsp.open_document_lang(&uri, src, "tcl-apl", 1);
    lsp.await_diagnostics_version(&uri, Some(1), std::time::Duration::from_secs(30));
    let tokens = typed(&mut lsp, &leg, &uri);

    let has = |text: &str, ttype: &str| {
        tokens
            .iter()
            .any(|t| covered(src, t) == text && t.ttype == ttype)
    };

    assert!(has("#include", "aplDirective"), "{tokens:?}");
    assert!(has("define", "aplDefine"), "{tokens:?}");
    assert!(has("choice", "aplFieldType"), "{tokens:?}");
    assert!(has("yesno_choice", "aplDefineName"), "{tokens:?}");
    assert!(has("section", "aplSection"), "{tokens:?}");
    assert!(has("basic", "aplSectionName"), "{tokens:?}");
    assert!(has("string", "aplFieldType"), "{tokens:?}");
    assert!(has("addr", "aplFieldName"), "{tokens:?}");
    assert!(has("required", "aplAttribute"), "{tokens:?}");
    assert!(has("validator", "aplAttribute"), "{tokens:?}");
    assert!(has("=>", "operator"), "{tokens:?}");
    assert!(has("# a comment", "comment"), "{tokens:?}");

    // The validator name is split out of its quotes rather than overlaid on
    // the string.
    assert!(has("IpAddress", "aplValidator"), "{tokens:?}");

    // A quoted value after a field-type keyword is a string, never the field's
    // name: a name group that matched non-whitespace greedily would swallow the
    // opening quote and emit `"Welcome` as a field name overlapping the string.
    assert!(
        !tokens
            .iter()
            .any(|t| t.ttype == "aplFieldName" && covered(src, t).starts_with('"')),
        "a quoted value must not be typed as a field name: {tokens:?}",
    );

    // No whole-line `string` dumps: the Tcl fallback emitted one per line.
    assert!(
        !tokens
            .iter()
            .any(|t| t.ttype == "string" && covered(src, t).trim_start().starts_with("string ")),
        "APL must not be tokenised as Tcl literal words: {tokens:?}",
    );

    // The LSP spec forbids overlapping tokens. 1.11.4 emitted 110 overlapping
    // pairs for `samples/apl/example.apl` by overlaying each specific type on a
    // generic one; that must never come back.
    let mut by_line: std::collections::BTreeMap<i64, Vec<&TypedToken>> =
        std::collections::BTreeMap::new();
    for t in &tokens {
        by_line.entry(t.line).or_default().push(t);
    }
    for (line, mut ts) in by_line {
        ts.sort_by_key(|t| (t.char, t.length));
        for w in ts.windows(2) {
            assert!(
                w[0].char + w[0].length <= w[1].char,
                "overlapping APL tokens on line {line}: {:?} then {:?}",
                w[0],
                w[1],
            );
        }
    }
}

/// BIG-IP config (`bigip.conf`, `.scf`) is not Tcl: it must be tokenised by its
/// own config lexer, not run through the Tcl pipeline.
///
/// Regression for the 2.1.6 state, where a `bigip.conf` fell through to the Tcl
/// tokenizer, which read each stanza body as one literal braced word — 272 of
/// the 302 tokens it produced for `samples/bigip/bigip.conf` were whole config
/// *lines* emitted as `string`.
#[test]
fn test_bigip_conf_uses_the_bigip_token_set() {
    let mut lsp = Lsp::tcl();
    let leg = legend(&lsp);
    let src = concat!(
        "# a config comment\n",
        "auth partition Common {\n",
        "    description \"Default partition\"\n",
        "}\n",
        "auth user opsUser {\n",
        "    shell tmsh\n",
        "}\n",
        "ltm node /Common/web1 {\n",
        "    address 10.0.1.10\n",
        "}\n",
        "ltm pool /Common/web_pool {\n",
        "    members {\n",
        "        /Common/web1:80 {\n",
        "        }\n",
        "    }\n",
        "    monitor /Common/http\n",
        "}\n",
    );
    let uri = unique_uri("conf");
    lsp.open_document_lang(&uri, src, "tcl-bigip", 1);
    lsp.await_diagnostics_version(&uri, Some(1), std::time::Duration::from_secs(30));
    let tokens = typed(&mut lsp, &leg, &uri);

    let has = |text: &str, ttype: &str| {
        tokens
            .iter()
            .any(|t| covered(src, t) == text && t.ttype == ttype)
    };

    assert!(has("# a config comment", "comment"), "{tokens:?}");
    assert!(has("ltm", "keyword"), "{tokens:?}");
    assert!(has("pool", "keyword"), "{tokens:?}");
    // A `/Common/x` path splits into its partition and its typed object name.
    assert!(has("Common", "partition"), "{tokens:?}");
    assert!(has("web_pool", "pool"), "{tokens:?}");
    assert!(has("web1", "object"), "{tokens:?}");
    assert!(has("http", "monitor"), "{tokens:?}");
    assert!(has("10.0.1.10", "ipAddress"), "{tokens:?}");
    assert!(has("80", "port"), "{tokens:?}");
    assert!(has("opsUser", "username"), "{tokens:?}");
    assert!(has("address", "property"), "{tokens:?}");

    // `auth partition Common` names a partition directly, not via a path.
    assert!(has("Common", "partition"), "{tokens:?}");

    // No whole-line `string` dumps — the Tcl fallback emitted one per line.
    assert!(
        !tokens
            .iter()
            .any(|t| t.ttype == "string" && covered(src, t).trim_start().starts_with("members")),
        "BIG-IP config must not be tokenised as Tcl literal words: {tokens:?}",
    );

    // The LSP spec forbids overlapping tokens. 1.11.4 produced 65 overlapping
    // pairs for `bigip_base.conf` by overlaying each specific type on a generic
    // one; that must never come back.
    let mut by_line: std::collections::BTreeMap<i64, Vec<&TypedToken>> =
        std::collections::BTreeMap::new();
    for t in &tokens {
        by_line.entry(t.line).or_default().push(t);
    }
    for (line, mut ts) in by_line {
        ts.sort_by_key(|t| (t.char, t.length));
        for w in ts.windows(2) {
            assert!(
                w[0].char + w[0].length <= w[1].char,
                "overlapping BIG-IP tokens on line {line}: {:?} then {:?}",
                w[0],
                w[1],
            );
        }
    }
}

/// Issue #898 — the semantic-token correctness audit against 1.11.4.
///
/// One test per section, so a regression names the section it broke.
#[test]
fn test_issue_898_semantic_token_corrections() {
    let mut lsp = Lsp::tcl();
    let leg = legend(&lsp);
    let src = concat!(
        // §1 — a literal word's closing delimiter must be covered.
        "set greeting \"hello world\"\n",
        "set braced {a b}\n",
        "set bound 3\n",
        "puts ${bound}\n",
        "set esc \"with \\\"q\\\" in\"\n",
        // §3 — an array ref with a *substituted* index is still a variable.
        "set env($greeting) 1\n",
        // §4 / §2 — parameters, method + class names.
        "proc greet {name} { return $name }\n",
        "oo::class create Shape {\n",
        "    method area {} { return 0 }\n",
        "}\n",
        // §5 — a regex group's closer is a group, not a quantifier.
        "regexp {^(foo)+$} $s\n",
        // §6 — expr parens and ternary are operators.
        "set r [expr {($a + $b) * ($c > 1 ? 1 : 2)}]\n",
        // §7 — an empty body emits no stray `}` token.
        "proc empty {args} {}\n",
        // §8 — a quoted string containing `::` is a string, not a namespace.
        "append cmd \"::scan x\"\n",
    );
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &leg, &uri);
    let has = |text: &str, ttype: &str| {
        tokens
            .iter()
            .any(|t| covered(src, t) == text && t.ttype == ttype)
    };

    // §1: both delimiters covered, for quotes, braces and `${…}` alike.
    assert!(has("\"hello world\"", "string"), "§1 quotes: {tokens:?}");
    assert!(has("{a b}", "string"), "§1 braces: {tokens:?}");
    assert!(has("${bound}", "variable"), "§1 braced var: {tokens:?}");
    // An escaped string keeps its delimiters too — that path drops *both*.
    assert!(has("\"", "string"), "§1 escaped-string quotes: {tokens:?}");
    assert!(has("\\\"", "escape"), "§1 escape: {tokens:?}");

    // §3: the literal fragments of `env($greeting)` are the variable, not strings.
    assert!(has("env(", "variable"), "§3 array name: {tokens:?}");
    assert!(has(")", "variable"), "§3 array close: {tokens:?}");

    // §4 / §2.
    assert!(has("name", "parameter"), "§4 parameter: {tokens:?}");
    assert!(has("Shape", "class"), "§2 class name: {tokens:?}");
    assert!(has("area", "method"), "§2 method name: {tokens:?}");

    // §5: `)` is a group delimiter, never a quantifier.
    assert!(has(")", "regexpGroup"), "§5 regex close: {tokens:?}");
    assert!(
        !tokens
            .iter()
            .any(|t| covered(src, t) == ")" && t.ttype == "regexpQuantifier"),
        "§5: a regex `)` must not be a quantifier: {tokens:?}",
    );

    // §6: grouping and ternary punctuation are operators.
    for op in ["(", ")", "?", ":"] {
        assert!(
            has(op, "operator"),
            "§6 `{op}` must be an operator: {tokens:?}"
        );
    }

    // §7: the empty body `{}` must not emit a `}` as a command head.
    assert!(
        !tokens
            .iter()
            .any(|t| covered(src, t) == "}" && t.ttype == "function"),
        "§7: an empty body must not emit `}}` as a function: {tokens:?}",
    );

    // §8: a `::`-bearing *quoted* word is a string literal, not a namespace.
    assert!(
        !tokens
            .iter()
            .any(|t| t.ttype == "namespace" && covered(src, t).starts_with('"')),
        "§8: a quoted string must not be typed as a namespace: {tokens:?}",
    );

    // Nothing above may overlap — the LSP spec forbids it.
    let mut by_line: std::collections::BTreeMap<i64, Vec<&TypedToken>> =
        std::collections::BTreeMap::new();
    for t in &tokens {
        by_line.entry(t.line).or_default().push(t);
    }
    for (line, mut ts) in by_line {
        ts.sort_by_key(|t| (t.char, t.length));
        for w in ts.windows(2) {
            assert!(
                w[0].char + w[0].length <= w[1].char,
                "overlapping tokens on line {line}: {:?} then {:?}",
                w[0],
                w[1],
            );
        }
    }
}

/// Issue #898 §11 — `defaultLibrary` on a built-in reached through a namespace
/// prefix, or through a `namespace import`ed bare alias.
#[test]
fn test_issue_898_default_library_modifier() {
    let mut lsp = Lsp::tcl();
    let leg = legend(&lsp);
    let src = "namespace import msgcat::*\nmc \"hi\"\nputs [tcl::mathop::+ 1 2]\n";
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &leg, &uri);
    let dl = modifier_bit(&modifiers(&lsp), "defaultLibrary");

    // An imported bare alias resolves to its qualified spec, so it is a built-in.
    assert!(
        tokens
            .iter()
            .any(|t| covered(src, t) == "mc" && t.ttype == "function" && t.modifiers & dl != 0),
        "an imported `msgcat::mc` alias must carry defaultLibrary: {tokens:?}",
    );
    // The namespace prefix of a built-in is part of that built-in's name.
    assert!(
        tokens.iter().any(|t| covered(src, t) == "tcl::mathop::"
            && t.ttype == "namespace"
            && t.modifiers & dl != 0),
        "a built-in's namespace prefix must carry defaultLibrary: {tokens:?}",
    );
}

/// `TclOO`, end to end: declarations, call sites, references, the one-liner
/// definer form, and `property`.
///
/// A method is the same entity at its declaration and at its call site, and a
/// `superclass` / `mixin` argument names a class rather than being a free
/// string — the registry's definition-body grammar carries both facts, so no
/// command name appears in the walker.
#[test]
fn test_tcloo_members_references_and_call_sites() {
    let mut lsp = Lsp::tcl();
    let leg = legend(&lsp);
    let src = concat!(
        "oo::class create Base {\n",
        "    superclass Shape\n",
        "    mixin Loggable\n",
        "    variable count\n",
        "    constructor {n} { set count $n }\n",
        "    method bump {by} { my Recalc }\n",
        "    method Recalc {} { return $count }\n",
        "    forward len llength\n",
        "    export bump\n",
        "    unexport Recalc\n",
        "}\n",
        "oo::define Base method inline {z} { return $z }\n",
        "set o [Base new 1]\n",
        "$o bump 2\n",
    );
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &leg, &uri);
    let has = |text: &str, ttype: &str| {
        tokens
            .iter()
            .any(|t| covered(src, t) == text && t.ttype == ttype)
    };

    // Declarations.
    assert!(has("Base", "class"), "class name: {tokens:?}");
    assert!(has("bump", "method"), "method decl: {tokens:?}");
    assert!(has("by", "parameter"), "method param: {tokens:?}");
    assert!(has("n", "parameter"), "constructor param: {tokens:?}");
    assert!(has("count", "variable"), "declared variable: {tokens:?}");

    // References: a `superclass` / `mixin` argument names a *class*; an
    // `export` / `unexport` argument names a *method*.
    assert!(has("Shape", "class"), "superclass ref: {tokens:?}");
    assert!(has("Loggable", "class"), "mixin ref: {tokens:?}");
    assert!(has("Recalc", "method"), "unexport ref: {tokens:?}");
    // `forward NAME cmd` declares NAME as a method.
    assert!(has("len", "method"), "forward name: {tokens:?}");

    // The one-liner definer form gets the same treatment as the body form —
    // name, parameter and body — not just its keyword.
    assert!(has("inline", "method"), "one-liner method name: {tokens:?}");
    assert!(has("z", "parameter"), "one-liner method param: {tokens:?}");

    // Call sites are the same entity as the declaration.
    assert!(
        tokens
            .iter()
            .any(|t| covered(src, t) == "bump" && t.ttype == "method"),
        "`$o bump` call site must be a method: {tokens:?}",
    );
    assert!(
        !tokens
            .iter()
            .any(|t| covered(src, t) == "Recalc" && t.ttype == "function"),
        "`my Recalc` must be a method, not a function: {tokens:?}",
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

/// Expect's `expect { ?-flags? pat body … }` is the same clause-list construct
/// as `switch`, and is described by the same registry data
/// (`CommandSpec::case_list`).
///
/// Regression for the walker's old `if seg.texts[0] != "switch"` hardcode: with
/// it, Expect's clause bodies were never recursed and an entire `expect {…}`
/// block — the central construct of every Expect script — rendered as flat
/// per-line `string` tokens.
#[test]
fn test_expect_clause_list_patterns_and_bodies() {
    let mut lsp = Lsp::tcl();
    let leg = legend(&lsp);
    let src = concat!(
        "spawn ssh host\n",
        "expect {\n",
        "    \"password:\" { send \"pw\\r\" }\n",
        "    -re {ye+s} { send \"yes\\r\" }\n",
        "    timeout { puts slow }\n",
        "    eof { puts done }\n",
        "}\n",
        "set timeout 30\n",
    );
    let uri = unique_uri("exp");
    lsp.open_document_lang(&uri, src, "tcl-expect", 1);
    lsp.await_diagnostics_version(&uri, Some(1), std::time::Duration::from_secs(30));
    let tokens = typed(&mut lsp, &leg, &uri);
    let has = |text: &str, ttype: &str| {
        tokens
            .iter()
            .any(|t| covered(src, t) == text && t.ttype == ttype)
    };

    // Clause bodies are recursed as scripts, not left opaque.
    assert!(
        has("send", "function"),
        "clause body must recurse: {tokens:?}"
    );
    assert!(
        has("puts", "function"),
        "clause body must recurse: {tokens:?}"
    );
    // `timeout` / `eof` match no text — they are keywords.
    assert!(has("timeout", "keyword"), "{tokens:?}");
    assert!(has("eof", "keyword"), "{tokens:?}");
    // A `-re` clause flag is a decorator and puts its pattern in regex mode.
    assert!(has("-re", "decorator"), "{tokens:?}");
    assert!(
        tokens
            .iter()
            .any(|t| covered(src, t) == "+" && t.ttype == "regexpQuantifier"),
        "an `-re` pattern must sub-tokenise as a regex: {tokens:?}",
    );
    // No whole-line `string` dumps.
    assert!(
        !tokens
            .iter()
            .any(|t| t.ttype == "string" && covered(src, t).trim_start().starts_with("timeout {")),
        "an expect block must not be tokenised as one literal: {tokens:?}",
    );
    // Context still matters: `timeout` as a *variable* name is a variable.
    assert!(
        tokens
            .iter()
            .any(|t| covered(src, t) == "timeout" && t.ttype == "variable" && t.line == 7),
        "`set timeout 30` must bind a variable, not a keyword: {tokens:?}",
    );
}

/// A braced word that happens to spell an Expect clause flag is a literal
/// pattern.  The registry-owned clause grammar must still recurse its paired
/// body, without painting that pattern as an option decorator.
#[test]
fn test_expect_clause_list_braced_flag_pattern_is_literal() {
    let mut lsp = Lsp::tcl();
    let leg = legend(&lsp);
    let src = "expect { {-re} { set matched 1 } }\n";
    let uri = unique_uri("exp");
    lsp.open_document_lang(&uri, src, "tcl-expect", 1);
    lsp.await_diagnostics_version(&uri, Some(1), std::time::Duration::from_secs(30));
    let tokens = typed(&mut lsp, &leg, &uri);

    assert!(
        tokens
            .iter()
            .any(|t| covered(src, t) == "set" && t.ttype == "function"),
        "the literal pattern's body must recurse: {tokens:?}",
    );
    assert!(
        !tokens
            .iter()
            .any(|t| covered(src, t) == "-re" && t.ttype == "decorator"),
        "a braced flag-shaped pattern must not become a decorator: {tokens:?}",
    );
}

/// An incomplete clause has no script body to recurse.  Rejecting the typed
/// case-list shape avoids fabricating code tokens from its pattern text.
#[test]
fn test_expect_incomplete_clause_does_not_fabricate_a_body() {
    let mut lsp = Lsp::tcl();
    let leg = legend(&lsp);
    let src = "expect { -re {set ghost 1} }\n";
    let uri = unique_uri("exp");
    lsp.open_document_lang(&uri, src, "tcl-expect", 1);
    lsp.await_diagnostics_version(&uri, Some(1), std::time::Duration::from_secs(30));
    let tokens = typed(&mut lsp, &leg, &uri);

    assert!(
        !tokens
            .iter()
            .any(|t| covered(src, t) == "set" && t.ttype == "function"),
        "an incomplete clause's pattern must not be treated as a body: {tokens:?}",
    );
}

/// Issue #1138 idx 100: Tk's `library/tk.tcl:289` builds its `upvar` with
/// `list` — `uplevel #0 [list upvar #0 ::tk::Priv.$disp ::tk::Priv]` — and
/// declares the same cell as a plain `variable ::tk::Priv` on the next line.
/// A 3-way comparison on tclsh 9.0.4 / 8.6.16 (direct `upvar`, braced
/// `uplevel {upvar …}`, `uplevel [list upvar …]`) shows the three are
/// functionally identical, yet only the `[list …]` form — the one Tk uses —
/// was painted as a namespace word instead of a variable declaration.
#[test]
fn test_list_built_upvar_declares_its_local_like_the_literal_spelling() {
    let mut lsp = Lsp::tcl();
    let leg = legend(&lsp);
    let mods = modifiers(&lsp);
    let src = concat!(
        "proc f {disp} {\n",
        "    uplevel #0 [list upvar #0 ::tk::Priv.$disp ::tk::Priv]\n",
        "    variable ::tk::Priv\n",
        "}\n",
    );
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &leg, &uri);
    let decl_bit = modifier_bit(&mods, "declaration");
    let at = |line: i64| {
        tokens
            .iter()
            .find(|t| t.line == line && covered(src, t) == "::tk::Priv")
            .unwrap_or_else(|| panic!("no `::tk::Priv` token on line {line}: {tokens:?}"))
    };
    let built = at(1);
    let literal = at(2);
    assert_eq!(
        (built.ttype.as_str(), built.modifiers & decl_bit != 0),
        ("variable", true),
        "the built `upvar`'s local is a variable declaration: {built:?}"
    );
    assert_eq!(
        (built.ttype.as_str(), built.modifiers),
        (literal.ttype.as_str(), literal.modifiers),
        "both spellings of the same declaration must agree: {tokens:?}"
    );

    // FP guard: `list` invokes nothing, so the same words in a *value* slot
    // declare nothing.  tclsh: `set x [list upvar 1 a b]; info exists b` -> 0.
    let value_src = "proc g {} {\n    set x [list upvar 1 a b]\n}\n";
    let value_uri = open_doc(&mut lsp, value_src);
    let value_tokens = typed(&mut lsp, &leg, &value_uri);
    assert!(
        !value_tokens.iter().any(|t| covered(value_src, t) == "b"
            && t.ttype == "variable"
            && t.modifiers & decl_bit != 0),
        "a value slot must not declare anything: {value_tokens:?}"
    );
}

/// Issue #1185: a command head's grammar follows its **effective command
/// identity**, not its spelling — end-to-end through the packaged server.
///
/// tclsh-proof, byte-identical on 9.0.4 and 8.6.16: `interp alias {} myformat
/// {} format; myformat %08x 42` → `0000002a`; `rename upvar myupvar` leaves
/// `myupvar 1 outer local` a working `upvar`; `proc scan {args} {return USER};
/// scan %d 7` → `USER`.
#[test]
fn test_command_identity_drives_argument_grammar() {
    let mut lsp = Lsp::tcl();
    let leg = legend(&lsp);
    let mods = modifiers(&lsp);
    let decl_bit = modifier_bit(&mods, "declaration");
    let lib_bit = modifier_bit(&mods, "defaultLibrary");

    let src = concat!(
        "puts [::format {%08x} 42]\n",          // 0 — qualified spelling
        "interp alias {} myformat {} format\n", // 1
        "puts [myformat {%08x} 42]\n",          // 2 — aliased
        "rename upvar myupvar\n",               // 3
        "proc reader {} {\n",                   // 4
        "    myupvar 1 outer local\n",          // 5 — renamed
        "}\n",                                  // 6
        "proc scan {args} { return USER }\n",   // 7
        "puts [scan {%d} 7]\n",                 // 8 — shadowed
    );
    let uri = open_doc(&mut lsp, src);
    let tokens = typed(&mut lsp, &leg, &uri);
    let types_on = |line: i64| -> Vec<&str> {
        tokens
            .iter()
            .filter(|t| t.line == line)
            .map(|t| t.ttype.as_str())
            .collect()
    };

    // TP — the qualified spelling and the alias both surface printf
    // conversion sub-tokens, exactly as a bare `format` call does.
    for line in [0, 2] {
        let types = types_on(line);
        assert!(
            types.contains(&"formatSpec") && types.contains(&"formatPercent"),
            "line {line} must carry format conversion tokens: {types:?}"
        );
    }

    // TP — the renamed `upvar` keeps its keyword classification and still
    // declares its local variable.
    let head = tokens
        .iter()
        .find(|t| t.line == 5 && covered(src, t) == "myupvar")
        .unwrap_or_else(|| panic!("no `myupvar` head token: {tokens:?}"));
    assert_eq!(
        head.ttype, "keyword",
        "a renamed keyword stays a keyword: {head:?}"
    );
    let local = tokens
        .iter()
        .find(|t| t.line == 5 && covered(src, t) == "local")
        .unwrap_or_else(|| panic!("no `local` token: {tokens:?}"));
    assert_eq!(
        (local.ttype.as_str(), local.modifiers & decl_bit != 0),
        ("variable", true),
        "the renamed upvar must still declare its local: {local:?}"
    );

    // FP — a top-level `proc scan` takes the built-in's name over: no scan
    // grammar, no `defaultLibrary` modifier, and the `%d` word stays a string.
    let types = types_on(8);
    assert!(
        !types.contains(&"formatSpec") && !types.contains(&"formatPercent"),
        "a shadowed `scan` must get no conversion tokens: {types:?}"
    );
    let shadowed = tokens
        .iter()
        .find(|t| t.line == 8 && covered(src, t) == "scan")
        .unwrap_or_else(|| panic!("no shadowed `scan` head token: {tokens:?}"));
    assert_eq!(
        shadowed.modifiers & lib_bit,
        0,
        "a shadowed built-in must lose `defaultLibrary`: {shadowed:?}"
    );
    assert!(
        types.contains(&"string"),
        "the shadowed call's `{{%d}}` stays a string: {types:?}"
    );

    // TN — an unprovable binding states nothing, so the name keeps its own
    // (unknown-command) identity and its argument stays a plain string.
    let dyn_src = "interp alias {} myfmt {} $target\nputs [myfmt {%08x} 42]\n";
    let dyn_uri = open_doc(&mut lsp, dyn_src);
    let dyn_tokens = typed(&mut lsp, &leg, &dyn_uri);
    assert!(
        !dyn_tokens
            .iter()
            .any(|t| t.line == 1 && t.ttype == "formatSpec"),
        "a dynamic alias target must not confer format grammar: {dyn_tokens:?}"
    );
}
