//! Native port of `tests/lsp_e2e/test_editor_features_e2e.py`.
//!
//! Code lens, document links, formatting, and inlay hints — end-to-end against
//! the packaged server. A cross-implementation conformance surface (raw
//! JSON-RPC): the Rust/Zed port must meet this spec.

mod common;

use common::{Lsp, unique_uri};

use serde_json::{Value, json};

/// A whole-document formatter returns a single replace edit; return its text.
fn full_text(edits: &Value) -> Option<String> {
    let arr = edits.as_array()?;
    let first = arr.first()?;
    first
        .get("newText")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// The lens array of a `code_lens` result (never null).
fn lenses(lsp: &mut Lsp, uri: &str) -> Vec<Value> {
    match lsp.code_lens(uri) {
        Value::Array(a) => a,
        _ => Vec::new(),
    }
}

/// A lens's `data.qname`, if present.
fn qname(lens: &Value) -> Option<&str> {
    lens.get("data").and_then(|d| d.get("qname")).and_then(Value::as_str)
}

/// Resolve the lens whose `data.qname` equals `qname`.
fn resolve_for_qname(lsp: &mut Lsp, lenses: &[Value], want: &str) -> Value {
    let lens = lenses
        .iter()
        .find(|l| qname(l) == Some(want))
        .unwrap_or_else(|| panic!("no lens for qname {want:?}"))
        .clone();
    lsp.code_lens_resolve(lens)
}

/// An inlay hint's `label`: a string or a list of `{value}` parts.
fn label_text(hint: &Value) -> String {
    match hint.get("label") {
        Some(Value::Array(parts)) => parts
            .iter()
            .map(|p| p.get("value").and_then(Value::as_str).unwrap_or("").to_owned())
            .collect::<Vec<_>>()
            .join(""),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

/// `(character, label)` of every parameter-kind hint on `line`, sorted.
fn param_labels_on_line(hints: &Value, line: i64) -> Vec<(i64, String)> {
    let mut out = Vec::new();
    for h in hints.as_array().cloned().unwrap_or_default() {
        if h.get("kind").and_then(Value::as_i64) != Some(2) {
            continue;
        }
        let pos = h.get("position").cloned().unwrap_or(Value::Null);
        let char = pos.get("character").and_then(Value::as_i64);
        if pos.get("line").and_then(Value::as_i64) == Some(line) {
            if let Some(c) = char {
                out.push((c, label_text(&h)));
            }
        }
    }
    out.sort();
    out
}

// -- TestCodeLens --------------------------------------------------------

#[test]
fn test_proc_gets_reference_count_lens() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc greet {} { return }\ngreet\ngreet\n");
    let ls = lenses(&mut lsp, &uri);
    let on_proc: Vec<&Value> = ls
        .iter()
        .filter(|l| l["range"]["start"]["line"].as_i64() == Some(0))
        .collect();
    assert!(!on_proc.is_empty(), "{ls:?}");
    assert!(
        on_proc.iter().any(|l| qname(l) == Some("::greet")),
        "{on_proc:?}"
    );
}

#[test]
fn test_no_lens_without_procs() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 1\nputs $x\n");
    assert!(lenses(&mut lsp, &uri).is_empty());
}

#[test]
fn test_count_matches_reference_list_for_forward_call() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "fwd637\nproc fwd637 {} { return }\n");
    let ls = lenses(&mut lsp, &uri);
    let resolved = resolve_for_qname(&mut lsp, &ls, "::fwd637");
    assert_eq!(resolved["command"]["title"], json!("1 reference"));
    // The reference list at the proc name (declaration excluded) agrees.
    let refs = match lsp.references(&uri, 1, 5, false) {
        Value::Array(a) => a,
        _ => Vec::new(),
    };
    assert_eq!(refs.len(), 1, "{refs:?}");
    let title = resolved["command"]["title"].as_str().unwrap_or("");
    assert!(title.starts_with(&refs.len().to_string()), "{title:?}");
}

#[test]
fn test_unresolved_call_scoped_to_namespace() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "namespace eval nsa644 {\n    dup644\n    proc dup644 {} {}\n}\n\
         namespace eval nsb644 {\n    proc dup644 {} {}\n}\n",
    );
    let ls = lenses(&mut lsp, &uri);
    let a = resolve_for_qname(&mut lsp, &ls, "::nsa644::dup644");
    let b = resolve_for_qname(&mut lsp, &ls, "::nsb644::dup644");
    assert_eq!(a["command"]["title"], json!("1 reference"));
    assert_eq!(b["command"]["title"], json!("0 references"));
}

// -- TestDocumentLinks ---------------------------------------------------

#[test]
fn test_source_command_is_linked() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "source other.tcl\nputs done\n");
    let links = match lsp.document_links(&uri) {
        Value::Array(a) => a,
        _ => Vec::new(),
    };
    assert!(
        links.iter().any(|l| l
            .get("tooltip")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("other.tcl")),
        "{links:?}"
    );
}

#[test]
fn test_package_require_is_linked() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "package require Tcl\n");
    let links = match lsp.document_links(&uri) {
        Value::Array(a) => a,
        _ => Vec::new(),
    };
    assert!(
        links
            .iter()
            .any(|l| l["range"]["start"]["line"].as_i64() == Some(0)),
        "{links:?}"
    );
}

// -- TestFormatting ------------------------------------------------------

#[test]
fn test_full_document_formatting_normalises_spacing() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc  f {}    {\nputs   hi\n}\n");
    let formatted = full_text(&lsp.formatting(&uri, 4, true));
    let formatted = formatted.expect("formatting produced no edit");
    assert!(formatted.contains("proc f {} {"), "{formatted:?}");
    assert!(formatted.contains("    puts hi"), "{formatted:?}");
}

#[test]
fn test_already_formatted_is_stable() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {} {\n    puts hi\n}\n";
    lsp.open_ready(&uri, src);
    let edits = lsp.formatting(&uri, 4, true);
    // No change needed → either no edits, or an edit reproducing the text.
    let non_empty = edits.as_array().map(|a| !a.is_empty()).unwrap_or(false);
    if non_empty {
        assert_eq!(full_text(&edits).as_deref(), Some(src));
    }
}

#[test]
fn test_range_formatting_returns_edits() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc  f {}    {\nputs   hi\n}\n");
    let range = json!({
        "start": { "line": 0, "character": 0 },
        "end": { "line": 2, "character": 1 },
    });
    let edits = lsp.range_formatting(&uri, range, 4);
    let arr = edits.as_array().cloned().unwrap_or_default();
    assert!(!arr.is_empty(), "{edits:?}");
    assert!(
        arr[0]["newText"]
            .as_str()
            .unwrap_or("")
            .contains("proc f {} {"),
        "{:?}",
        arr[0]
    );
}

// -- TestInlayHints ------------------------------------------------------

#[test]
fn test_provider_responds_with_a_list() {
    // Inlay hints are gated off by default; the provider must still answer with
    // a well-formed (here empty) list, never an error.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 42\n");
    let result = lsp.inlay_hints(&uri, (0, 0), (1, 0));
    assert!(result.is_array(), "{result:?}");
}

#[test]
fn test_hint_kinds_are_valid_when_present() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc add {a b} { expr {$a + $b} }\nadd 1 2\n");
    for hint in lsp.inlay_hints(&uri, (0, 0), (2, 0)).as_array().cloned().unwrap_or_default() {
        let k = hint.get("kind").and_then(Value::as_i64);
        assert!(k == Some(1) || k == Some(2), "{hint:?}");
    }
}

// -- TestInlayHintOptionalPositionals ------------------------------------
// These run against `Lsp::inlay()` (inlay hints on) so the provider actually
// produces hints — the default server keeps them off.

#[test]
fn test_single_positional_binds_required_string_slot() {
    let mut lsp = Lsp::inlay();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "puts hello\n");
    let labels = param_labels_on_line(&lsp.inlay_hints(&uri, (0, 0), (1, 0)), 0);
    let names: Vec<String> = labels.iter().map(|(_, n)| n.clone()).collect();
    assert!(names.contains(&"string:".to_owned()), "{labels:?}");
    assert!(!names.contains(&"channelId:".to_owned()), "{labels:?}");
}

#[test]
fn test_two_positionals_label_channel_then_string() {
    let mut lsp = Lsp::inlay();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "puts $chan hello\n");
    let labels = param_labels_on_line(&lsp.inlay_hints(&uri, (0, 0), (1, 0)), 0);
    let names: Vec<String> = labels.iter().map(|(_, n)| n.clone()).collect();
    assert_eq!(names, vec!["channelId:".to_owned(), "string:".to_owned()], "{labels:?}");
}

#[test]
fn test_leading_flag_does_not_shift_required_slot() {
    let mut lsp = Lsp::inlay();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "puts -nonewline hello\n");
    let labels = param_labels_on_line(&lsp.inlay_hints(&uri, (0, 0), (1, 0)), 0);
    let names: Vec<String> = labels.iter().map(|(_, n)| n.clone()).collect();
    assert!(names.contains(&"string:".to_owned()), "{names:?}");
    assert!(!names.contains(&"channelId:".to_owned()), "{names:?}");
}

#[test]
fn test_no_documentation_placeholder_labels() {
    let mut lsp = Lsp::inlay();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "lsearch $mylist needle\n");
    let labels = param_labels_on_line(&lsp.inlay_hints(&uri, (0, 0), (1, 0)), 0);
    let names: Vec<String> = labels.iter().map(|(_, n)| n.clone()).collect();
    assert!(names.contains(&"list:".to_owned()), "{names:?}");
    assert!(names.contains(&"pattern:".to_owned()), "{names:?}");
    assert!(
        !names.iter().any(|n| n == "options:" || n == "switches:"),
        "{names:?}"
    );
}

// -- TestInlayToggleIndependenceE2E --------------------------------------
// The single `inlayHints` toggle was split into two independent options, both
// off by default: `inlayTypeHints` and `inlayParameterHints`. Enabling one must
// not turn on the other. Each test owns its server, so `apply_configuration_settle`
// replaces pytest's `config_session` (no restore needed).

#[test]
fn test_parameter_hints_only_produce_labels_no_type_hints() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.apply_configuration_settle(
        json!({ "features": { "inlayTypeHints": false, "inlayParameterHints": true } }),
        &uri,
        |c| {
            c["features"].get("inlayParameterHints") == Some(&json!(true))
                && c["features"].get("inlayTypeHints") == Some(&json!(false))
        },
    );
    lsp.open_ready(&uri, "set x 42\nputs hello\n");
    let hints = lsp.inlay_hints(&uri, (0, 0), (2, 0));
    let arr = hints.as_array().cloned().unwrap_or_default();
    assert!(
        arr.iter().any(|h| h.get("kind").and_then(Value::as_i64) == Some(2)),
        "{arr:?}"
    ); // Parameter present
    assert!(
        arr.iter().all(|h| h.get("kind").and_then(Value::as_i64) != Some(1)),
        "{arr:?}"
    ); // no Type leaked in
}

#[test]
fn test_type_hints_only_emit_no_parameter_labels() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.apply_configuration_settle(
        json!({ "features": { "inlayTypeHints": true, "inlayParameterHints": false } }),
        &uri,
        |c| {
            c["features"].get("inlayTypeHints") == Some(&json!(true))
                && c["features"].get("inlayParameterHints") == Some(&json!(false))
        },
    );
    lsp.open_ready(&uri, "set x 42\nputs hello\n");
    let hints = lsp.inlay_hints(&uri, (0, 0), (2, 0));
    let arr = hints.as_array().cloned().unwrap_or_default();
    assert!(
        arr.iter().any(|h| h.get("kind").and_then(Value::as_i64) == Some(1)),
        "{arr:?}"
    ); // Type present
    assert!(
        arr.iter().all(|h| h.get("kind").and_then(Value::as_i64) != Some(2)),
        "{arr:?}"
    ); // no Parameter leaked in
}

// -- TestInlayLegacyAliasE2E ---------------------------------------------
// The retired `features.inlayHints` key is a backward-compatible alias that
// enables *type* hints only — parameter hints stay off.

#[test]
fn test_legacy_inlay_hints_alias_enables_type_only() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.apply_configuration_settle(
        json!({ "features": { "inlayHints": true } }),
        &uri,
        |c| c["features"].get("inlayTypeHints") == Some(&json!(true)),
    );
    let eff = lsp.effective_config(&uri);
    assert_eq!(
        eff["features"].get("inlayTypeHints"),
        Some(&json!(true)),
        "{:?}",
        eff["features"]
    );
    assert_eq!(
        eff["features"].get("inlayParameterHints"),
        Some(&json!(false)),
        "{:?}",
        eff["features"]
    );
    lsp.open_ready(&uri, "set x 42\nputs hello\n");
    let hints = lsp.inlay_hints(&uri, (0, 0), (2, 0));
    let arr = hints.as_array().cloned().unwrap_or_default();
    assert!(
        arr.iter().any(|h| h.get("kind").and_then(Value::as_i64) == Some(1)),
        "{arr:?}"
    ); // Type present
    assert!(
        arr.iter().all(|h| h.get("kind").and_then(Value::as_i64) != Some(2)),
        "{arr:?}"
    ); // no Parameter
}
