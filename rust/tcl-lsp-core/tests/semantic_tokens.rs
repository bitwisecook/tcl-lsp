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
