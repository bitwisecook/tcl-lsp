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

//! The encoding invariants, and the tokens of plain words: commands, variables, numbers, comments, operators and strings.

use super::*;

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
        // A `#` comment ending in an unescaped backslash continues
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
        // A fixed 2-byte slice at a backslash escape immediately before a
        // non-ASCII char would split the multibyte char and panic the whole
        // semanticTokens/full request. The escape must span the backslash
        // plus the full UTF-8 char; drive it through the server so the
        // bounds/overlap invariants (and non-panic) are checked.
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
        // A `#`-leading physical line inside a
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
    assert_eq!(
        semantic_token_violations(&raw, source, &types_ref, &mods_ref),
        [] as [std::string::String; 0]
    );
}

#[test]
fn test_escape_before_multibyte_char_does_not_panic() {
    // `\<non-ASCII>` (`\é`, `\你`, `\€`) inside a string must not be
    // handled as a fixed 2-byte slice at the backslash: that would land
    // inside the multibyte char and panic the whole
    // `textDocument/semanticTokens/full` request. Drive the real request
    // through the server and require it to succeed AND emit an `escape`
    // sub-token for the `\X` run.
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
    // too, not just `full` / `full/delta`.
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
    // End-to-end through the packaged server: a `#` comment whose
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
    // A braced string literal spanning multiple lines must not lose its
    // highlighting: the enclosing multi-line `string` token must not be
    // dropped. End-to-end it must carry a `string` token on every covered
    // line, exactly like the quoted form.
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
