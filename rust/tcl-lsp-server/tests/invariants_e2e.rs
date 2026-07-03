//! Native port of `tests/lsp_e2e/test_invariants_e2e.py`.
//!
//! Universal invariants + adversarial robustness, end-to-end against the server.
//! These assert properties that must hold for *every* provider on *every*
//! document: well-formed ranges, disjoint `WorkspaceEdit`s, and that hostile
//! inputs never wedge/crash the server or emit a malformed span.

mod common;

use common::helpers::*;
use common::{Lsp, unique_uri};

use serde_json::{Value, json};

/// A corpus spanning clean code, every multibyte/EOL hazard, and the recovery
/// paths. Reused by the range-invariant and robustness batteries.
fn corpus() -> Vec<(&'static str, String)> {
    let large: String = (0..400)
        .map(|i| format!("proc p{i} {{a b}} {{ return [expr {{$a + $b}}] }}\n"))
        .collect();
    let mut v: Vec<(&'static str, String)> = vec![
        (
            "clean",
            "proc greet {name} {\n    puts \"Hello $name\"\n}\ngreet World\n".to_owned(),
        ),
        ("vars", "set x 1\nset y $x\nputs [expr {$x + $y}]\n".to_owned()),
        (
            "multibyte",
            "set s \"café résumé naïve\"\nputs $s\nproc f {s} { return $s }\nf $s\n".to_owned(),
        ),
        ("emoji", "set e \"😀 🚀 🐫 done\"\nputs $e\nset n 1\n".to_owned()),
        ("unicode_ident", "set 日本語 1\nputs ${日本語}\n".to_owned()),
        ("crlf", "proc p {} {\r\n    set x 1\r\n}\r\np\r\n".to_owned()),
        ("bom", "\u{feff}puts hello\nset x 1\n".to_owned()),
        (
            "unterminated_bracket",
            "set x [foo bar\nproc recovered {} {}\nputs hi\n".to_owned(),
        ),
        (
            "unterminated_brace",
            "proc p {} {\n  set y [foo\n}\nset z 1\n".to_owned(),
        ),
        ("unterminated_quote", "set s \"open\nputs done\n".to_owned()),
        ("deep_nesting", "set x [a [b [c [d [e [f [g\nputs tail\n".to_owned()),
        ("empty", "".to_owned()),
        ("blank_lines", "\n\n\n".to_owned()),
        ("only_comment", "# just a comment\n".to_owned()),
        ("large", large),
    ];
    v.sort_by(|a, b| a.0.cmp(b.0));
    v
}

/// Run a broad request battery against `uri`; return `(name, result)` pairs.
///
/// Every request must return (not raise / not time out). Positions are chosen to
/// land on real tokens in the smaller corpus entries and are harmless elsewhere.
fn exercise_all_providers(lsp: &mut Lsp, uri: &str) -> Vec<(&'static str, Value)> {
    vec![
        ("documentSymbol", lsp.document_symbols(uri)),
        ("foldingRange", lsp.folding_range(uri)),
        ("semanticTokens", lsp.semantic_tokens(uri)),
        ("hover@0,1", lsp.hover(uri, 0, 1)),
        ("definition@0,1", lsp.definition(uri, 0, 1)),
        ("references@0,5", lsp.references(uri, 0, 5, true)),
        ("highlight@0,5", lsp.document_highlight(uri, 0, 5)),
        ("completion@0,1", lsp.completion(uri, 0, 1)),
        ("signatureHelp@1,8", lsp.signature_help(uri, 1, 8)),
        (
            "selectionRange@0,1",
            lsp.selection_range(uri, json!([{ "line": 0, "character": 1 }])),
        ),
        (
            "codeAction@0,0-1,0",
            lsp.code_actions(
                uri,
                json!({
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": 1, "character": 0 },
                }),
                json!([]),
            ),
        ),
        ("documentLink", lsp.document_links(uri)),
        ("codeLens", lsp.code_lens(uri)),
    ]
}

// -- TestRangeInvariants -------------------------------------------------

/// Every Range from every provider is well-formed, over the whole corpus.
#[test]
fn test_all_provider_ranges_well_formed() {
    let mut lsp = Lsp::tcl();
    for (name, source) in corpus() {
        let uri = unique_uri("tcl");
        lsp.open_ready(&uri, &source);
        let results = exercise_all_providers(&mut lsp, &uri);
        let mut violations: Vec<String> = Vec::new();
        for (req, res) in &results {
            violations.extend(range_violations(res, &source, &format!("{name}/{req}"), false));
        }
        assert!(
            violations.is_empty(),
            "malformed ranges:\n{}",
            violations.join("\n")
        );
    }
}

// -- TestWorkspaceEditInvariants -----------------------------------------

#[test]
fn test_rename_edits_do_not_overlap() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc greet {name} { puts $name }\ngreet a\ngreet b\ngreet c\n";
    lsp.open_ready(&uri, src);
    let edit = lsp.rename(&uri, 0, 6, "welcome");
    assert!(workspace_edit_violations(&edit, "rename").is_empty());
}

#[test]
fn test_multisite_variable_rename_edits_are_disjoint() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc p {} {\n    set count 0\n    incr count\n    incr count\n    return $count\n}\n";
    lsp.open_ready(&uri, src);
    let edit = lsp.rename(&uri, 1, 8, "counter");
    assert!(workspace_edit_violations(&edit, "var-rename").is_empty());
}

#[test]
fn test_code_action_edits_do_not_overlap() {
    // W100 (unbraced expr) offers a safe wrap fix; its edits must be disjoint.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "if $a { puts yes }\nif $b { puts no }\n");
    let actions = lsp.code_actions(
        &uri,
        json!({
            "start": { "line": 0, "character": 0 },
            "end": { "line": 2, "character": 0 },
        }),
        json!([]),
    );
    let actions = actions.as_array().cloned().unwrap_or_default();
    for action in &actions {
        let edit = action.get("edit").cloned().unwrap_or(Value::Null);
        assert!(workspace_edit_violations(&edit, "code-action").is_empty());
    }
}

// -- TestRobustnessAdversarial -------------------------------------------

/// Hostile inputs never hang/crash the server and never yield bad spans.
#[test]
fn test_server_survives_and_responds() {
    let mut lsp = Lsp::tcl();
    for (name, source) in corpus() {
        let uri = unique_uri("tcl");
        // `open_ready` itself proves the analysis pipeline didn't wedge.
        lsp.open_ready(&uri, &source);
        // The full battery must return for every adversarial input (the request
        // helpers panic on a JSON-RPC error or time out, so reaching the end is
        // the assertion) and every span must be well-formed.
        let results = exercise_all_providers(&mut lsp, &uri);
        let mut violations: Vec<String> = Vec::new();
        for (req, res) in &results {
            violations.extend(range_violations(res, &source, &format!("{name}/{req}"), false));
        }
        assert!(
            violations.is_empty(),
            "malformed ranges on adversarial input:\n{}",
            violations.join("\n")
        );
    }
}

#[test]
fn test_server_still_responsive_after_adversarial_burst() {
    // After hammering hostile documents, a normal request on a fresh doc must
    // still answer correctly — proves no document poisoned global state.
    let mut lsp = Lsp::tcl();
    let by_name: std::collections::HashMap<&str, String> = corpus().into_iter().collect();
    for name in ["deep_nesting", "unterminated_quote", "emoji", "large"] {
        let u = unique_uri("tcl");
        let src = by_name[name].clone();
        lsp.open_ready(&u, &src);
        let _ = exercise_all_providers(&mut lsp, &u);
    }
    let sane = unique_uri("tcl");
    lsp.open_ready(&sane, "puts hello\n");
    assert!(hover_text(&lsp.hover(&sane, 0, 2)).contains("puts"));
}
