//! Serialisation of the canonical red-green CST for the `cst` view.
//!
//! `DOCUMENT` → `COMMAND` → `WORD` → fragment, each interior node carrying
//! its absolute range (and a word's `single`/`braced`/`quoted`/`expand`
//! shape), each leaf its inner `text` versus lossless `raw` and inner-end
//! range, and each opaque `{…}` / `[…]` leaf its descended child document
//! (flagged `terminated`). Built from the same `build_document` the
//! analyser and lowering consume.

use serde_json::{Value, json};

use tcl_compiler::parsing::syntax::build::build_document;
use tcl_compiler::parsing::syntax::descend::descend_token;
use tcl_compiler::parsing::syntax::green::SyntaxKind;
use tcl_compiler::parsing::syntax::red::{SyntaxElement, SyntaxNode, SyntaxToken, SyntaxTree};
use tcl_lexer::{LexerConfig, SourceMap, TokenType};

/// Maximum descent depth, matching `depth < 24` guard.
const MAX_DEPTH: u32 = 24;

/// Token-type name in uppercase enum-member form
/// (`Str` → `"STR"`, `Cmd` → `"CMD"`, …).
fn token_type_name(kind: TokenType) -> String {
    format!("{kind:?}").to_uppercase()
}

/// `SyntaxKind` name in uppercase form (`Command` → `"COMMAND"`).
fn kind_name(kind: SyntaxKind) -> String {
    format!("{kind:?}").to_uppercase()
}

/// Whether a leaf token's word is opaque (a braced `{…}` or bracketed
/// `[…]` body the explorer can descend into).
fn is_opaque(kind: TokenType) -> bool {
    matches!(kind, TokenType::Str | TokenType::Cmd)
}

fn leaf_dict(
    tok: &SyntaxToken<'_>,
    is_marker: bool,
    sm: &SourceMap<'_>,
    depth: u32,
    config: LexerConfig,
) -> Value {
    let mut entry = json!({
        "kind": token_type_name(tok.token_type()),
        "isMarker": is_marker,
        "text": tok.text(),
        "raw": tok.raw(),
        "inQuote": tok.in_quote(),
        "startOffset": tok.start_position().offset,
        // End offsets are exclusive; the red layer's end is the lexer
        // inner-end, so +1 covers the last inner char.
        "endOffset": tok.end_position().offset + 1,
    });
    if depth < MAX_DEPTH && is_opaque(tok.token_type()) && !tok.text().is_empty() {
        // Descend with the document's dialect config so nested `{…}` / `[…]`
        // bodies tokenise the same way the top level does.
        let descended = descend_token(sm, tok.to_token(), config);
        let root = descended.root();
        entry["terminated"] = json!(descended.is_terminated());
        entry["child"] = node_dict(&root, descended.tree(), sm, depth + 1, config);
    }
    entry
}

fn node_dict(
    node: &SyntaxNode<'_>,
    tree: &SyntaxTree,
    sm: &SourceMap<'_>,
    depth: u32,
    config: LexerConfig,
) -> Value {
    let kind = node.kind();
    let markers = node.expand_markers();
    let children: Vec<SyntaxElement<'_>> = node.children().collect();
    let frags: Vec<&SyntaxToken<'_>> = children
        .iter()
        .filter_map(|c| match c {
            SyntaxElement::Token(t) => Some(t),
            SyntaxElement::Node(_) => None,
        })
        .collect();

    let mut tags: Vec<&str> = Vec::new();
    let mut label = String::new();
    let (start, end) = match kind {
        SyntaxKind::Command => {
            let toks = node.tokens();
            let first = &toks[0];
            let start = first.start_position().offset;
            let end_inc = tree
                .position_at(first.raw_start() + node.green().range_end_rel.unwrap_or(0))
                .offset;
            sm.source()
                .get(start as usize..(end_inc as usize + 1))
                .unwrap_or("")
                .clone_into(&mut label);
            (start, end_inc + 1)
        }
        SyntaxKind::Word => {
            let first = markers.first().or_else(|| frags.first().copied());
            let start = first.map_or(0, |t| t.start_position().offset);
            let end = frags.last().map_or(start, |t| t.end_position().offset + 1);
            if !markers.is_empty() {
                tags.push("expand");
            }
            if frags.len() == 1 {
                tags.push("single");
            }
            if frags
                .first()
                .is_some_and(|t| t.token_type() == TokenType::Str)
            {
                tags.push("braced");
            }
            if frags.first().is_some_and(|t| t.raw().starts_with('"')) {
                tags.push("quoted");
            }
            (start, end)
        }
        // DOCUMENT (the region the tree is anchored over).
        SyntaxKind::Document => {
            let base = tree.position_at(0).offset;
            (base, base + u32::try_from(tree.text().len()).unwrap_or(0))
        }
    };

    let mut kids: Vec<Value> = markers
        .iter()
        .map(|m| leaf_dict(m, true, sm, depth, config))
        .collect();
    for child in &children {
        match child {
            SyntaxElement::Token(t) => kids.push(leaf_dict(t, false, sm, depth, config)),
            SyntaxElement::Node(n) => kids.push(node_dict(n, tree, sm, depth, config)),
        }
    }

    json!({
        "kind": kind_name(kind),
        "label": label,
        "tags": tags,
        "startOffset": start,
        "endOffset": end,
        "children": kids,
    })
}

/// Serialise the `cst` view for `source`, tokenising with `config` so the
/// tree reflects the document's dialect (`{*}` expansion, iRules brace
/// separator).
#[must_use]
pub fn serialise_cst(source: &str, config: LexerConfig) -> Value {
    let (green, _warnings) = build_document(source, config);
    let tree = SyntaxTree::new(green);
    let sm = SourceMap::new(source);
    node_dict(&tree.root(), &tree, &sm, 0, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_root_holds_commands() {
        let cst = serialise_cst("set x 1", LexerConfig::default());
        assert_eq!(cst["kind"], "DOCUMENT");
        let cmd = &cst["children"][0];
        assert_eq!(cmd["kind"], "COMMAND");
        assert_eq!(cmd["label"], "set x 1");
    }

    /// Find a braced `STR` leaf with a descended child document.
    fn has_descended_brace(node: &Value) -> bool {
        for child in node["children"].as_array().into_iter().flatten() {
            if child.get("child").is_some() && child["kind"] == "STR" {
                assert_eq!(child["child"]["kind"], "DOCUMENT");
                return true;
            }
            if child.get("children").is_some() && has_descended_brace(child) {
                return true;
            }
        }
        false
    }

    #[test]
    fn opaque_braced_word_descends_into_child_document() {
        let cst = serialise_cst("if {$x} { puts hi }", LexerConfig::default());
        assert!(
            has_descended_brace(&cst),
            "expected a descended braced body"
        );
    }

    #[test]
    fn word_tags_mark_braced_and_quoted() {
        let cst = serialise_cst("set x \"hi\"", LexerConfig::default());
        // The command's words carry shape tags.
        let cmd = &cst["children"][0];
        let words: Vec<&Value> = cmd["children"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["kind"] == "WORD")
            .collect();
        let quoted = words
            .iter()
            .any(|w| w["tags"].as_array().unwrap().iter().any(|t| t == "quoted"));
        assert!(quoted, "expected a quoted word tag");
    }
}
