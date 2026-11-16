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

//! The script bodies the walk recurses into, clause lists, and the structural keywords of control commands.

use super::*;

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

//
// tcllib commands that carry a script body (`control::do`,
// `struct::list foreachperm`) or an expression (`control::do`'s test,
// `control::assert`) must recurse into that argument rather than emit it
// as one opaque string — same treatment as core `while`/`if`.

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
    // The braced body of `uplevel ?level? {…}` is a script and
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
    // A `namespace children` /
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
    // rather than being emitted as one opaque string, so `set`
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
    // must stay a string (the bareword-builtin glitch).
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
    // quote; the keyword is emitted from the content base so the range
    // covers `else`, not `"els`.
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

/// The braced case-list form of a plain (non-`-regexp`)
/// `switch` must recurse each body as a script so the commands inside are
/// highlighted, rather than treating the whole `{ pat body … }` list as one
/// opaque body.
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
