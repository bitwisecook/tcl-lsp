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

//! Refactor and source actions: extract and inline proc, expression rewrites, address conversions, docstring generation and where a refactor is offered.

use super::*;

#[test]
fn test_extract_proc_available_without_diagnostics() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set value 42\nputs $value\nputs done\n");
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((1, 0), (2, 0)),
        json!([]),
        &["refactor.extract"],
    );
    assert!(
        titles(&actions)
            .iter()
            .any(|t| t.to_lowercase().contains("extract"))
    );
}

#[test]
fn test_extract_proc_snippets() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set value 42\nputs $value\nputs done\n");
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((1, 0), (2, 0)),
        json!([]),
        &["refactor.extract"],
    );
    let extract = kinds(&actions, "refactor.extract");
    assert_ne!(extract, [] as [serde_json::Value; 0]);
    let snippets = new_texts(&json!(extract));
    assert!(
        snippets
            .iter()
            .any(|s| s.contains("proc extracted_proc {value}"))
    );
    assert!(snippets.iter().any(|s| s.contains("extracted_proc $value")));
}

/// The `refactor.extract` actions named "Extract selection into proc" for a
/// selection of `source`, requested through the real server.
fn extract_proc_actions(lsp: &mut Lsp, uri: &str, from: (u32, u32), to: (u32, u32)) -> Vec<Value> {
    let actions = code_actions_only(lsp, uri, range(from, to), json!([]), &["refactor.extract"]);
    kinds(&actions, "refactor.extract")
        .into_iter()
        .filter(|a| action_title(a) == "Extract selection into proc")
        .collect()
}

#[test]
fn test_extract_proc_carries_a_caller_write_through_upvar() {
    // The original prints `1` then `after=1`. Moving the `set x 1` into a
    // proc *local* would leave the caller with its old value, printing
    // `after=0`. The generated proc must take the
    // variable by name and re-bind it with `upvar 1`, so the assignment
    // lands back in the caller's frame.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 0\nset x 1\nputs $x\nputs \"after=$x\"\n");
    let extract = extract_proc_actions(&mut lsp, &uri, (1, 0), (3, 0));
    assert_eq!(extract.len(), 1, "{extract:?}");
    let snippets = new_texts(&json!(extract));
    assert!(
        snippets.iter().any(|s| s.contains("upvar 1 $xName x")),
        "the write must reach the caller's frame: {snippets:?}"
    );
    assert!(
        snippets.iter().any(|s| s.trim_end() == "extracted_proc x"),
        "the call passes the variable's name, not its value: {snippets:?}"
    );
}

#[test]
fn test_extract_proc_refuses_a_selection_containing_return() {
    // `return` acts on the call frame: inside a new proc it would return from
    // that proc rather than from the enclosing one.  Refused with a reason.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc f {} {\n    set x 1\n    return $x\n}\n");
    let extract = extract_proc_actions(&mut lsp, &uri, (2, 0), (3, 0));
    assert_eq!(extract.len(), 1, "{extract:?}");
    let reason = extract[0]
        .get("disabled")
        .and_then(|d| d.get("reason"))
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("expected a refusal reason: {:?}", extract[0]));
    assert!(reason.contains("call frame"), "{reason}");
}

#[test]
fn test_extract_proc_refuses_a_selection_inside_a_namespace_eval() {
    // The extracted proc would be created in a different namespace from the
    // one the moved code ran in.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "namespace eval app {\n    set x 1\n    puts $x\n}\n");
    let extract = extract_proc_actions(&mut lsp, &uri, (2, 0), (3, 0));
    assert_eq!(extract.len(), 1, "{extract:?}");
    let reason = extract[0]
        .get("disabled")
        .and_then(|d| d.get("reason"))
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("expected a refusal reason: {:?}", extract[0]));
    assert!(reason.contains("namespace"), "{reason}");
}

#[test]
fn test_inline_proc_binds_a_declared_default() {
    // C Tcl 9 prints `hello world`: the omitted argument takes the parameter's
    // declared default.  A textual splice must not emit `puts "hello $name"`,
    // which errors on an unset variable.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let source = "proc greet {{name world}} { puts $name }\ngreet\n";
    lsp.open_ready(&uri, source);
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((1, 1), (1, 1)),
        json!([]),
        &["refactor.inline"],
    );
    let inline = kinds(&actions, "refactor.inline");
    assert_eq!(inline.len(), 1, "{actions:?}");
    assert!(
        new_texts(&json!(inline)).iter().any(|s| s == "puts world"),
        "the default must bind: {inline:?}"
    );
}

#[test]
fn test_inline_proc_refuses_a_braced_argument_with_a_reason() {
    // `greet {a b}` passes the *value* `a b`.  Splicing the written word made
    // the body print the braces, so the transform refuses and explains.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc greet {name} { puts \"hello $name\" }\ngreet {a b}\n",
    );
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((1, 1), (1, 1)),
        json!([]),
        &["refactor.inline"],
    );
    let inline = kinds(&actions, "refactor.inline");
    assert_eq!(inline.len(), 1, "{actions:?}");
    let reason = inline[0]
        .get("disabled")
        .and_then(|d| d.get("reason"))
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("expected a refusal reason: {:?}", inline[0]));
    assert!(reason.contains("plain word"), "{reason}");
}

#[test]
fn test_extract_proc_attaches_rename_command() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set value 42\nputs $value\nputs done\n");
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((1, 0), (2, 0)),
        json!([]),
        &["refactor.extract"],
    );
    // The extract-*proc* action carries the rename; extract-variable shares
    // the `refactor.extract` kind, so select by title rather than by index.
    let extract: Vec<Value> = kinds(&actions, "refactor.extract")
        .into_iter()
        .filter(|a| action_title(a) == "Extract selection into proc")
        .collect();
    assert_eq!(extract.len(), 1, "{actions:?}");
    let cmd = extract[0].get("command").cloned().unwrap_or(Value::Null);
    assert!(!cmd.is_null());
    assert_eq!(
        cmd.get("command").and_then(Value::as_str),
        Some("tclLsp.renameSymbolAtPosition")
    );
    let args = cmd
        .get("arguments")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let line = args[0].as_i64().unwrap();
    let start = args[1].as_i64().unwrap();
    let end = args[2].as_i64().unwrap();
    // The definition is inserted above the selection (line 1), not at line 0:
    // placing it at the top of the file would put it before any `package
    // require` / `namespace` prologue.
    assert_eq!(line, 1);
    assert_eq!(start, i64::try_from("proc ".len()).unwrap());
    assert_eq!(end, start + i64::try_from("extracted_proc".len()).unwrap());
}

#[test]
fn test_inline_proc_available() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc greet {name} { puts $name }\ngreet Bob\n");
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((1, 1), (1, 1)),
        json!([]),
        &["refactor.inline"],
    );
    let inline = kinds(&actions, "refactor.inline");
    assert_ne!(inline, [] as [serde_json::Value; 0]);
    assert!(
        new_texts(&json!(inline))
            .iter()
            .any(|s| s.contains("puts Bob"))
    );
}

#[test]
fn test_inline_proc_refuses_a_returning_proc_with_a_reason() {
    // `return` acts on the call frame: inlined into the caller it would
    // return from the *caller*, not from the proc.  The action is surfaced
    // greyed out with LSP's `disabled.reason` rather than silently omitted,
    // so the user can tell "cannot be done here" from "is broken".
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc wrap {x} { return $x }\nwrap value\n");
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((1, 1), (1, 1)),
        json!([]),
        &["refactor.inline"],
    );
    let inline = kinds(&actions, "refactor.inline");
    assert_eq!(inline.len(), 1, "{actions:?}");
    let reason = inline[0]
        .get("disabled")
        .and_then(|d| d.get("reason"))
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("expected a refusal reason: {:?}", inline[0]));
    assert!(reason.contains("call frame"), "{reason}");
    assert!(
        new_texts(&json!(inline)).is_empty(),
        "a refused action carries no edits: {inline:?}"
    );
}

#[test]
fn test_demorgan_forward() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "if {!($a && $b)} { puts yes }\n");
    let dm: Vec<Value> = rewrite(&mut lsp, &uri, (0, 4), (0, 15))
        .into_iter()
        .filter(|a| action_title(a).contains("De Morgan"))
        .collect();
    assert_eq!(dm.len(), 1);
    assert!(
        new_texts(&json!(dm))
            .iter()
            .any(|s| s.contains("!$a || !$b"))
    );
}

#[test]
fn test_demorgan_reverse() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "if {!$a || !$b} { puts yes }\n");
    let dm: Vec<Value> = rewrite(&mut lsp, &uri, (0, 4), (0, 14))
        .into_iter()
        .filter(|a| action_title(a).contains("De Morgan"))
        .collect();
    assert_eq!(dm.len(), 1);
    assert!(
        new_texts(&json!(dm))
            .iter()
            .any(|s| s.contains("!($a && $b)"))
    );
}

#[test]
fn test_invert_expression() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "if {$a == $b} { puts yes }\n");
    let inv: Vec<Value> = rewrite(&mut lsp, &uri, (0, 4), (0, 12))
        .into_iter()
        .filter(|a| action_title(a).contains("Invert"))
        .collect();
    assert_eq!(inv.len(), 1);
    assert!(
        new_texts(&json!(inv))
            .iter()
            .any(|s| s.contains("$a != $b"))
    );
}

#[test]
fn test_no_actions_for_non_expression_selection() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "puts hello\n");
    let acts = rewrite(&mut lsp, &uri, (0, 0), (0, 4));
    assert!(
        !acts
            .iter()
            .any(|a| action_title(a).contains("De Morgan") || action_title(a).contains("Invert"))
    );
}

#[test]
fn test_no_actions_for_empty_selection() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "if {$a == $b} { puts yes }\n");
    let acts = rewrite(&mut lsp, &uri, (0, 5), (0, 5));
    assert!(
        !acts
            .iter()
            .any(|a| action_title(a).contains("De Morgan") || action_title(a).contains("Invert"))
    );
}

#[test]
fn test_invert_subexpression_in_compound() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "if {$a == $b && $c < $d} { puts yes }\n");
    let inv: Vec<Value> = rewrite(&mut lsp, &uri, (0, 16), (0, 23))
        .into_iter()
        .filter(|a| action_title(a).contains("Invert"))
        .collect();
    assert_eq!(inv.len(), 1);
}

#[test]
fn test_ipv4_offers_ipv6_mapped() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set addr 10.0.0.1\n");
    let actions = lsp.code_actions(&uri, range((0, 10), (0, 10)), json!([]));
    assert!(titles(&actions).iter().any(|t| t.contains("IPv6-mapped")));
    assert!(new_texts(&actions).iter().any(|s| s.contains("::ffff:")));
}

#[test]
fn test_ipv6_mapped_offers_ipv4() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set addr ::ffff:192.168.1.1\n");
    let actions = lsp.code_actions(&uri, range((0, 12), (0, 12)), json!([]));
    assert!(titles(&actions).iter().any(|t| t.contains("IPv4")));
    assert!(
        new_texts(&actions)
            .iter()
            .any(|s| s.contains("192.168.1.1"))
    );
}

#[test]
fn test_non_mapped_ipv6_no_ipv4_action() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set addr ::1\n");
    let actions = lsp.code_actions(&uri, range((0, 10), (0, 10)), json!([]));
    assert!(
        !titles(&json!(kinds(&actions, "refactor")))
            .iter()
            .any(|t| t.contains("IPv4"))
    );
}

#[test]
fn test_ipv4_with_prefix_preserves_suffix() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set net 10.0.0.0/8\n");
    let actions = lsp.code_actions(&uri, range((0, 10), (0, 10)), json!([]));
    assert!(
        new_texts(&json!(kinds(&actions, "refactor")))
            .iter()
            .any(|s| s.contains("/8"))
    );
}

#[test]
fn test_non_ip_word_no_action() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set name hello\n");
    let actions = lsp.code_actions(&uri, range((0, 10), (0, 10)), json!([]));
    assert_eq!(kinds(&actions, "refactor"), [] as [serde_json::Value; 0]);
}

#[test]
fn test_cursor_past_end_of_line_does_not_crash() {
    // The request must succeed (no server-side IndexError); an empty result
    // comes back as JSON null over the wire rather than [].
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 1\n");
    let result = lsp.code_actions(&uri, range((0, 999), (0, 999)), json!([]));
    assert!(result.is_null() || result.is_array());
}

#[test]
fn test_cursor_on_empty_line_does_not_crash() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 1\n\nset y 2\n");
    let result = lsp.code_actions(&uri, range((1, 0), (1, 0)), json!([]));
    assert!(result.is_null() || result.is_array());
}

#[test]
fn test_cursor_past_last_line_does_not_crash() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 1\n");
    let result = lsp.code_actions(&uri, range((99, 0), (99, 0)), json!([]));
    assert!(result.is_null() || result.is_array());
}

//
// The generate-docstring source action is gated by the resolved
// `tclLsp.formatting.docstringStyle` setting. Its documented
// default is `"none"` — "do not generate or reformat docstrings" — which
// `Lsp::tcl()` reproduces (it sends no `formatting` section at all, so the
// server falls back to that default). Tests that exercise the *generation*
// mechanics therefore opt into `"preceding"` explicitly via
// `Lsp::with_config`; `test_none_is_the_default_and_offers_no_action` pins
// the gate itself.

#[test]
fn test_none_is_the_default_and_offers_no_action() {
    // TN-shape: with no `docstringStyle` configured (`Lsp::tcl()`'s
    // baseline), the resolved setting is `none` and the source action is
    // not offered at all, even for an undocumented proc.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc greet {name} { puts $name }\n");
    let actions = lsp.code_actions(&uri, range((0, 0), (0, 0)), json!([]));
    let sa = kinds(&actions, "source");
    assert!(
        sa.iter()
            .all(|a| !action_title(a).to_lowercase().contains("docstring")),
        "docstringStyle defaults to none; no generate action expected, got {sa:?}",
    );
}

#[test]
fn test_offered_for_undocumented_proc() {
    let mut lsp = Lsp::with_config(json!({ "formatting": { "docstringStyle": "preceding" } }));
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc greet {name} { puts $name }\n");
    let actions = lsp.code_actions(&uri, range((0, 0), (0, 0)), json!([]));
    let sa = kinds(&actions, "source");
    let doc: Vec<Value> = sa
        .into_iter()
        .filter(|a| action_title(a).to_lowercase().contains("docstring"))
        .collect();
    assert_eq!(doc.len(), 1);
    assert!(action_title(&doc[0]).contains("'greet'"));
    // Preceding placement: the edit lands on the declaration line itself.
    let range = first_edit_range(&doc[0]);
    assert_eq!(range["start"]["line"], 0);
    assert_eq!(range["start"]["character"], 0);
}

#[test]
fn test_not_offered_for_documented_proc() {
    let mut lsp = Lsp::with_config(json!({ "formatting": { "docstringStyle": "preceding" } }));
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "# Already documented\nproc greet {name} { puts $name }\n",
    );
    let actions = lsp.code_actions(&uri, range((1, 0), (1, 0)), json!([]));
    let sa = kinds(&actions, "source");
    assert!(
        sa.iter()
            .all(|a| !action_title(a).to_lowercase().contains("docstring"))
    );
}

#[test]
fn test_generated_edit_contains_param_tags() {
    let mut lsp = Lsp::with_config(json!({ "formatting": { "docstringStyle": "preceding" } }));
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc add {a b} { expr {$a + $b} }\n");
    let actions = lsp.code_actions(&uri, range((0, 0), (0, 0)), json!([]));
    let doc: Vec<Value> = kinds(&actions, "source")
        .into_iter()
        .filter(|a| action_title(a).to_lowercase().contains("docstring"))
        .collect();
    let snippets = new_texts(&json!(doc));
    assert!(snippets.iter().any(|s| s.contains("@param a")));
    assert!(snippets.iter().any(|s| s.contains("@param b")));
}

#[test]
fn test_body_style_inserts_inside_the_proc_not_on_the_decl_line() {
    // TP-shape: `docstringStyle: "body"` moves the insertion point off the
    // declaration line and into the body, matching the resolved config
    // rather than always using `Preceding`.
    let mut lsp = Lsp::with_config(json!({ "formatting": { "docstringStyle": "body" } }));
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc greet {name} {\n    puts $name\n}\n");
    let actions = lsp.code_actions(&uri, range((0, 0), (0, 0)), json!([]));
    let doc: Vec<Value> = kinds(&actions, "source")
        .into_iter()
        .filter(|a| action_title(a).to_lowercase().contains("docstring"))
        .collect();
    assert_eq!(doc.len(), 1);
    let edit_range = first_edit_range(&doc[0]);
    assert_eq!(edit_range["start"]["line"], 0);
    assert!(
        edit_range["start"]["character"].as_u64().unwrap() > 0,
        "body placement must not sit at column 0 of the decl line: {edit_range:?}",
    );
}

/// Refactor code actions must reach control flow inside an
/// `apply` lambda body.  `apply`'s literal is `ArgRole::LambdaLiteral`, so
/// the refactor descent has to split it and walk element 1; re-segmenting
/// the whole `{argList body}` blob would read `{m}` as a command name and
/// leave every `body_words`-backed action (if-to-switch, switch-to-dict,
/// brace-expr, inline-variable, extract-to-datagroup) unavailable
/// in there.  tclsh 8.6/9.0 confirm this lambda really does run the
/// `if` it wraps.
#[test]
fn test_if_to_switch_offered_inside_an_apply_lambda_body() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc handler {} {\n    apply {{m} {\n        if {$m eq \"GET\"} {\n            puts get\n        } elseif {$m eq \"POST\"} {\n            puts post\n        }\n    }} $x\n}\n";
    lsp.open_ready(&uri, src);
    // Cursor on the `if` inside the lambda body.
    let actions = lsp.code_actions(&uri, range((2, 8), (2, 8)), json!([]));
    assert!(
        titles(&actions)
            .iter()
            .any(|t| t.to_lowercase().contains("switch")),
        "the if-to-switch refactor must be offered inside the lambda: {actions:?}"
    );
}

/// TN: a lambda body written as a **quoted** list
/// element with escapes is backslash-decoded before `apply` evaluates it,
/// so its source slice is not the script that runs — the source says `$m eq
/// \"GET\"` where the real body says `$m eq "GET"` (tclsh 8.6 / 9.0.4 both
/// run this lambda and answer `get` / `post`).  Re-parsing the slice in
/// place offers a rewrite built from text that never executes, and lands
/// its edits inside a string literal, so the descent must take only
/// `{braced}` body elements.
#[test]
fn test_no_refactor_offered_inside_an_escaped_quoted_lambda_body() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "apply {{m} \"if {$m eq \\\"GET\\\"} {return get} elseif {$m eq \\\"POST\\\"} {return post}\"} GET\n";
    lsp.open_ready(&uri, src);
    // Cursor on the `if` word of the quoted body element.
    let actions = lsp.code_actions(&uri, range((0, 12), (0, 12)), json!([]));
    assert!(
        kinds(&actions, "refactor").is_empty(),
        "an escaped quoted body element is not script at these offsets: {:?}",
        titles(&actions)
    );
}

/// TN: the lambda's *argument list* is a plain word list, not code, so a
/// cursor on it offers no control-flow refactor even when a parameter
/// happens to be spelled like a command.
#[test]
fn test_no_refactor_offered_on_the_lambda_arg_list() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "apply {{if} {\n    puts $if\n}} 1\n";
    lsp.open_ready(&uri, src);
    // Cursor on the `if` parameter name in the argument list (line 0).
    let actions = lsp.code_actions(&uri, range((0, 8), (0, 8)), json!([]));
    assert!(
        !titles(&actions)
            .iter()
            .any(|t| t.to_lowercase().contains("switch")),
        "a parameter word named `if` is not an `if` command: {actions:?}"
    );
}
