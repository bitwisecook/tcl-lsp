//! Tests for the LSP semantic-tokens provider.
//! Verifies `semantic_tokens::full` classifies source spans (function/keyword/
//! variable/number/string/comment) correctly.
//!
//! C-Tcl proof: the classification mirrors Tcl's parse — `puts`/`set`/`proc`/
//! `if` are real commands (`info commands`), `$y` is a variable, `42` is a
//! number, and `#…` in command position is a comment.

use tcl_lsp_core::semantic_tokens::{full, legend_token_types};
use tcl_registry::registry_for_dialect;

#[derive(Debug)]
struct Tok {
    line: u32,
    character: u32,
    length: u32,
    ttype: String,
}

/// Decode the LSP delta-encoded token stream into absolute (line, char,
/// length, type-name) tuples.
fn decode(source: &str, dialect: &str) -> Vec<Tok> {
    let registry = registry_for_dialect(dialect);
    let st = full(source, dialect, registry);
    let legend = legend_token_types();
    let mut out = Vec::new();
    let (mut line, mut character) = (0u32, 0u32);
    for chunk in st.data.chunks(5) {
        let (dl, dc, length, ty) = (chunk[0], chunk[1], chunk[2], chunk[3]);
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
        });
    }
    out
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
fn comment_is_one_token() {
    let t = decode("# hello world", "tcl8.6");
    assert_eq!(t[0].ttype, "comment");
}

#[test]
fn comment_with_namespace_qualifiers_stays_one_comment() {
    // `::` inside a comment must NOT split into namespace tokens.
    let source = "# TCP::collect / TCP::payload / TCP::release";
    let t = decode(source, "f5-irules");
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].ttype, "comment");
    assert_eq!(t[0].length as usize, source.chars().count());
}

#[test]
fn multiline_comment_header_all_comments() {
    let source =
        "# Flow:\n#   1. CLIENT_ACCEPTED -> TCP::collect\n#   2. CLIENT_DATA -> TCP::payload\n";
    let t = decode(source, "f5-irules");
    assert!(
        t.iter().all(|x| x.ttype == "comment"),
        "expected all comment tokens, got {t:?}"
    );
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

// -- TclOO definition-body highlighting (issue #747) -----------------------
//
// C-Tcl proof: `oo::configurable`, `oo::abstract`, and `oo::singleton` are
// real Tcl 9.0 metaclasses (`info commands oo::*`) that manufacture classes
// with the same `create name ?body?` shape as `oo::class`.  Inside such a
// definition body the words `superclass` / `self` / `property` / `method`
// are TclOO definition sub-keywords, and a `method`/`constructor` body holds
// ordinary Tcl code — so `set x 1` inside it is a real command call.

#[test]
fn oo_configurable_body_keywords_are_highlighted() {
    // Regression for #747: `superclass` / `self` / `property` / `method`
    // inside an `oo::configurable` body must highlight as keywords, exactly
    // as they already do inside `oo::class`.
    let src = concat!(
        "oo::configurable create Widget {\n",
        "    superclass Base\n",
        "    self mixin M\n",
        "    property color\n",
        "    method paint {} { set x 1 }\n",
        "}\n",
    );
    for kw in ["superclass", "self", "property", "method"] {
        assert_eq!(
            kind_of_word(src, "tcl9.0", kw).as_deref(),
            Some("keyword"),
            "`{kw}` inside oo::configurable body must be a keyword",
        );
    }
}

#[test]
fn oo_configurable_method_body_is_recursed() {
    // The method body `{ set x 1 }` must be tokenised (not emitted as one
    // opaque string): `set` → function, `1` → number.
    let src = concat!(
        "oo::configurable create Widget {\n",
        "    method paint {} { set x 1 }\n",
        "}\n",
    );
    let toks = decode(src, "tcl9.0");
    assert!(
        toks.iter()
            .any(|t| t.ttype == "function" && tok_text(src, t) == "set"),
        "method body should tokenise `set` as a function: {toks:?}",
    );
    assert!(
        toks.iter().any(|t| t.ttype == "number"),
        "method body should tokenise the `1` literal as a number: {toks:?}",
    );
}

#[test]
fn oo_class_method_body_is_recursed() {
    // Regression for #747 (comment: "even with oo::class it stops at the
    // method body"): the `method`/`constructor` body inside an oo::class
    // block must be recursed, not left as one opaque string.
    let src = concat!(
        "oo::class create C {\n",
        "    constructor {} { set y 2 }\n",
        "    method m {} { puts hello }\n",
        "}\n",
    );
    let toks = decode(src, "tcl9.0");
    // `puts` (a builtin) inside the method body → function.
    assert!(
        toks.iter()
            .any(|t| t.ttype == "function" && tok_text(src, t) == "puts"),
        "method body should tokenise `puts`: {toks:?}",
    );
    // `set` inside the constructor body → function; `2` → number.
    assert!(
        toks.iter()
            .any(|t| t.ttype == "function" && tok_text(src, t) == "set"),
        "constructor body should tokenise `set`: {toks:?}",
    );
    assert!(
        toks.iter().any(|t| t.ttype == "number"),
        "constructor body should tokenise the `2` literal: {toks:?}",
    );
}

#[test]
fn oo_abstract_and_singleton_bodies_are_recursed() {
    for metaclass in ["oo::abstract", "oo::singleton"] {
        let src = format!("{metaclass} create C {{\n    method m {{}} {{ set z 3 }}\n}}\n");
        assert_eq!(
            kind_of_word(&src, "tcl9.0", "method").as_deref(),
            Some("keyword"),
            "`method` inside {metaclass} body must be a keyword",
        );
        let toks = decode(&src, "tcl9.0");
        assert!(
            toks.iter()
                .any(|t| t.ttype == "function" && tok_text(&src, t) == "set"),
            "{metaclass}: method body should tokenise `set`: {toks:?}",
        );
    }
}

#[test]
fn oo_property_accessor_bodies_are_recursed() {
    // `property NAME -get { BODY } -set { BODY }` accessor bodies inside a
    // configurable class must be recursed too.
    let src = concat!(
        "oo::configurable create Widget {\n",
        "    property color -get { return red } -set { set color $value }\n",
        "}\n",
    );
    let toks = decode(src, "tcl9.0");
    // `return` is itself a language keyword; its presence inside the `-get`
    // body proves the accessor body was recursed.
    assert!(
        toks.iter()
            .any(|t| t.ttype == "keyword" && tok_text(src, t) == "return"),
        "`-get` accessor body should tokenise `return`: {toks:?}",
    );
    assert!(
        toks.iter()
            .any(|t| t.ttype == "function" && tok_text(src, t) == "set"),
        "`-set` accessor body should tokenise `set`: {toks:?}",
    );
    // The `$value` variable substitution inside the `-set` body is recursed.
    assert!(
        toks.iter().any(|t| t.ttype == "variable"),
        "`-set` accessor body should tokenise `$value`: {toks:?}",
    );
}

#[test]
fn top_level_method_is_not_an_oo_body() {
    // Context guard: a top-level user proc named `method` (outside any OO
    // definition body) must NOT be treated as an OO method definition.  A
    // bare `method a b {c}` call at top level is a plain command — its last
    // braced word must stay an opaque string, not be recursed as a body.
    let src = "method a b { this is not code }\n";
    let toks = decode(src, "tcl8.6");
    // No command inside the braced word should be tokenised as a function;
    // `this`/`is`/`not`/`code` must never surface as separate command heads.
    assert!(
        !toks.iter().any(|t| t.ttype == "function"),
        "top-level `method` braced arg must not be recursed as a body: {toks:?}",
    );
}

#[test]
fn oo_define_body_form_recurses_method_bodies() {
    // The `oo::define Cls { … }` script form is an outer OO definition body
    // too — method bodies inside it must recurse.
    let src = concat!("oo::define Cls {\n", "    method m {} { puts hi }\n", "}\n",);
    assert_eq!(
        kind_of_word(src, "tcl9.0", "method").as_deref(),
        Some("keyword"),
    );
    let toks = decode(src, "tcl9.0");
    assert!(
        toks.iter()
            .any(|t| t.ttype == "function" && tok_text(src, t) == "puts"),
        "oo::define body method should recurse: {toks:?}",
    );
}
