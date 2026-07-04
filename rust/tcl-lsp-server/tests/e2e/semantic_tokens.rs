//! Native port of `tests/lsp_e2e/test_semantic_tokens_e2e.py`.
//!
//! Semantic tokens, end-to-end against the packaged server. A fresh document is
//! opened per case and `textDocument/semanticTokens/full` is requested once,
//! then the delta-encoded `data` is decoded and the legend (advertised in
//! `initialize`) maps type/modifier indices back to names — so a legend or
//! encoder drift in the shipped artifact is caught here.


use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};

use serde_json::Value;

/// A decoded token with its resolved type-name (like the pytest `_typed`).
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
    let raw = lsp.semantic_tokens(uri);
    decode_semantic_tokens(&raw)
        .into_iter()
        .map(|t| TypedToken {
            line: t.line,
            char: t.char,
            length: t.length,
            ttype: legend[t.ttype as usize].clone(),
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
    let line = source.split('\n').nth(tok.line as usize).unwrap_or("");
    let start = tok.char as usize;
    let end = (tok.char + tok.length) as usize;
    // Slicing by UTF-16-ish char offset; the covered() cases here are all ASCII.
    line.get(start..end).unwrap_or("")
}

/// The bit mask for a modifier name.
fn modifier_bit(mods: &[String], name: &str) -> i64 {
    let idx = mods.iter().position(|m| m == name).expect("modifier in legend");
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
        ("crlf", "proc p {} {\r\n    set x 1\r\n}\r\n"),
        (
            "multibyte_string",
            "set greeting \"héllo wörld café\"\nputs $greeting\n",
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
fn test_variable() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "set x $y\n");
    let types: Vec<String> = typed(&mut lsp, &lg, &uri).into_iter().map(|t| t.ttype).collect();
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
    let data = lsp.semantic_tokens(&uri)["data"].as_array().cloned().unwrap_or_default();
    assert_eq!(data.len() % 5, 0);
}

#[test]
fn test_empty_source() {
    let mut lsp = Lsp::tcl();
    let uri = open_doc(&mut lsp, "");
    assert_eq!(
        lsp.semantic_tokens(&uri)["data"],
        Value::Array(vec![])
    );
}

#[test]
fn test_string_in_quotes() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "puts \"hello\"\n");
    let types: Vec<String> = typed(&mut lsp, &lg, &uri).into_iter().map(|t| t.ttype).collect();
    assert!(types.iter().any(|t| t == "function"));
    assert!(types.iter().any(|t| t == "string"));
}

#[test]
fn test_braced_string() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "puts {hello world}\n");
    let types: Vec<String> = typed(&mut lsp, &lg, &uri).into_iter().map(|t| t.ttype).collect();
    assert!(types.iter().any(|t| t == "string"));
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
    let types: std::collections::BTreeSet<String> =
        typed(&mut lsp, &lg, &uri).into_iter().map(|t| t.ttype).collect();
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
    assert!(tokens.iter().any(|t| t.ttype == "function" && t.length == 7));
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
    let has_set = tokens.iter().any(|t| t.ttype == "function" && t.length == 3);
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
    let types: std::collections::BTreeSet<String> =
        typed(&mut lsp, &lg, &uri).into_iter().map(|t| t.ttype).collect();
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
    let has_puts = tokens.iter().any(|t| t.ttype == "function" && t.length == 4);
    assert!(has_puts, "foreachperm body not recursed: {tokens:?}");
    let has_var = tokens.iter().any(|t| t.ttype == "variable");
    assert!(has_var, "foreachperm var not recursed: {tokens:?}");
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
    assert_eq!(covered(source, kw), "else", "{:?} {:?}", kw, covered(source, kw));
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
    let cc: Vec<&TypedToken> = tokens.iter().filter(|t| t.ttype == "regexpCharClass").collect();
    assert!(cc.iter().any(|t| t.length == 2));
    assert!(tokens.iter().any(|t| t.ttype == "regexpQuantifier"));
}

#[test]
fn test_switch_regexp_braced_case_list() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "switch -regexp $x { {^a} {puts a} {^b} {puts b} }\n");
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
    assert_eq!(events[0].length, "HTTP_REQUEST".len() as i64);
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
    assert!(decs.iter().any(|t| t.length == "-nocase".len() as i64));
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
    assert!(tokens.iter().any(|t| t.ttype == "namespace" && t.length == "oo::".len() as i64));
    assert!(tokens.iter().any(|t| t.ttype == "keyword" && t.length == "class".len() as i64));
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
    assert_eq!(ns[0].length, "::".len() as i64);
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
    assert_eq!(tokens.iter().filter(|t| t.ttype == "formatPercent").count(), 2);
    assert_eq!(tokens.iter().filter(|t| t.ttype == "formatSpec").count(), 2);
}

#[test]
fn test_format_flags_width_precision() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "format \"%-10.5f\" 3.14159\n");
    let tokens = typed(&mut lsp, &lg, &uri);
    assert_eq!(tokens.iter().filter(|t| t.ttype == "formatPercent").count(), 1);
    assert_eq!(tokens.iter().filter(|t| t.ttype == "formatFlag").count(), 2);
    assert_eq!(tokens.iter().filter(|t| t.ttype == "formatSpec").count(), 1);
    assert_eq!(tokens.iter().filter(|t| t.ttype == "formatWidth").count(), 2);
}

#[test]
fn test_clock_format_specifiers() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "clock format $t -format \"%Y-%m-%d\"\n");
    let tokens = typed(&mut lsp, &lg, &uri);
    assert_eq!(tokens.iter().filter(|t| t.ttype == "clockPercent").count(), 3);
    assert_eq!(tokens.iter().filter(|t| t.ttype == "clockSpec").count(), 3);
}

#[test]
fn test_clock_format_locale_modifier() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "clock format $t -format \"%EY\"\n");
    let tokens = typed(&mut lsp, &lg, &uri);
    assert_eq!(tokens.iter().filter(|t| t.ttype == "clockPercent").count(), 1);
    assert_eq!(tokens.iter().filter(|t| t.ttype == "clockModifier").count(), 1);
    assert_eq!(tokens.iter().filter(|t| t.ttype == "clockSpec").count(), 1);
}

#[test]
fn test_clock_without_format_option() {
    let mut lsp = Lsp::tcl();
    let lg = legend(&lsp);
    let uri = open_doc(&mut lsp, "clock format $t -gmt true\n");
    let tokens = typed(&mut lsp, &lg, &uri);
    assert_eq!(tokens.iter().filter(|t| t.ttype == "clockPercent").count(), 0);
}
