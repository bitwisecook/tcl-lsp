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

//! Native port of `tests/lsp_e2e/test_vscode_parity_e2e.py`.
//!
//! End-to-end regression coverage for the VS Code parity gaps. Every test here
//! corresponds to a VS Code extension test that failed against the native Rust
//! server (the extension suite exercises the packaged server over the full LSP
//! protocol, but the backend-neutral `lsp_e2e` suite previously did not cover
//! these paths — so a Rust-only regression slipped through green). Each test
//! drives the *same* server behaviour directly over JSON-RPC so the gap is
//! caught here too, without needing the VS Code test host.
//!
//! Grouped by the fix area:
//!   * diagnostics — W100 inside command substitutions, W216 brace-then-paren
//!   * completion — command-context var binders, `::`-qualified globals, dialect
//!   * commands   — `setDialect` / `compilerExplorer`
//!   * config     — folding / optimiser / diagnostics-master toggles
//!   * capabilities — typeHierarchy advertised, pull-diagnostics opt-in
//!   * navigation — method-parameter go-to-definition
//!   * code actions — code-lens showReferences wiring, brace-expr refactor

use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};

use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::time::Duration;

/// The sorted set of `code` strings carried by `diags` (mirrors Python
/// `_codes`).
fn codes(diags: &[Value]) -> Vec<String> {
    let mut out: Vec<String> = diags
        .iter()
        .map(|d| match d.get("code") {
            Some(Value::String(s)) => s.clone(),
            Some(other) => other.to_string(),
            None => "None".to_owned(),
        })
        .collect();
    out.sort();
    out
}

/// Whether any diagnostic carries `code`.
fn has_code(diags: &[Value], code: &str) -> bool {
    diags.iter().any(|d| code_str(d).as_deref() == Some(code))
}

/// A diagnostic's `code`, normalised to a `String`.
fn code_str(d: &Value) -> Option<String> {
    match d.get("code") {
        Some(Value::String(s)) => Some(s.clone()),
        Some(other) => Some(other.to_string()),
        None => None,
    }
}

/// A diagnostic's `message` text (empty if absent).
fn message(d: &Value) -> &str {
    d.get("message").and_then(Value::as_str).unwrap_or("")
}

/// The `label`s of a completion result (mirrors Python `_labels`).
fn labels(result: &Value) -> Vec<String> {
    completion_labels(result)
}

/// A `Range` value built from `(line, character)` start/end tuples.
fn range(start: (u32, u32), end: (u32, u32)) -> Value {
    json!({
        "start": { "line": start.0, "character": start.1 },
        "end": { "line": end.0, "character": end.1 },
    })
}

/// Every edit carried by a code action's `WorkspaceEdit` (mirrors Python
/// `_edits_of`).
fn edits_of(action: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    let edit = action.get("edit").cloned().unwrap_or(Value::Null);
    if let Some(changes) = edit.get("changes").and_then(Value::as_object) {
        for edits in changes.values() {
            if let Some(arr) = edits.as_array() {
                out.extend(arr.iter().cloned());
            }
        }
    }
    if let Some(dcs) = edit.get("documentChanges").and_then(Value::as_array) {
        for dc in dcs {
            if let Some(arr) = dc.get("edits").and_then(Value::as_array) {
                out.extend(arr.iter().cloned());
            }
        }
    }
    out
}

// ── diagnostics ──────────────────────────────────────────────────────────────

// -- TestDiagnosticsParity -----------------------------------------------

#[test]
fn test_w100_fires_for_expr_in_command_substitution() {
    // `set x [expr $a + $b]` — the unbraced expr lives inside a `[…]` command
    // substitution (an argument value), which the analyser must descend for the
    // W100 check.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(has_code(
        &lsp.open_ready(&uri, "set x [expr $a + $b]\n"),
        "W100"
    ));
}

#[test]
fn test_w216_fires_inside_command_substitution() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = concat!(
        "set arr(name) hello\n",
        "set foo bar\n",
        "set indirect [puts ${arr}(name)]\n",
        "set indirect2 [puts ${arr($foo)}]\n",
        "set \"funny name\" 1\n",
        "set indirect3 [puts ${funny name($foo)}]\n",
    );
    let diags = lsp.open_ready(&uri, src);
    let w216: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("W216"))
        .collect();
    assert!(w216.len() >= 2);
    let msgs = w216
        .iter()
        .map(|d| message(d))
        .collect::<Vec<_>>()
        .join(" / ");
    assert!(msgs.contains("scalar") && msgs.contains("literal")); // ${arr}(name) message
    assert!(msgs.contains("does not substitute")); // ${arr($foo)} message
}

#[test]
fn test_w216_quick_fixes() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set arr(name) 1\nset foo bar\nset r [puts ${arr}(name)]\n";
    let diags = lsp.open_ready(&uri, src);
    let w216: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("W216") && message(d).contains("scalar"))
        .collect();
    assert!(!w216.is_empty());
    let rng = w216[0]["range"].clone();
    let start = (
        u32::try_from(rng["start"]["line"].as_u64().unwrap()).unwrap(),
        u32::try_from(rng["start"]["character"].as_u64().unwrap()).unwrap(),
    );
    let end = (
        u32::try_from(rng["end"]["line"].as_u64().unwrap()).unwrap(),
        u32::try_from(rng["end"]["character"].as_u64().unwrap()).unwrap(),
    );
    let actions = lsp.code_actions(&uri, range(start, end), Value::Array(diags.clone()));
    let new_texts: Vec<String> = actions
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .flat_map(edits_of)
        .filter_map(|e| e.get("newText").and_then(Value::as_str).map(str::to_owned))
        .collect();
    assert!(new_texts.iter().any(|t| t == "$arr(name)"));
}

// ── completion ───────────────────────────────────────────────────────────────

// -- TestCompletionParity ------------------------------------------------

const COMMAND_CONTEXTS: &str = concat!(
    "proc t {} {\n",
    "    lassign {1 2 3} la lb lc\n",
    "    regexp {(\\w+)=(\\w+)} foo=bar rf rk rv\n",
    "    regsub {foo} foobar baz rout\n",
    "    scan \"1 2 3\" \"%d %d %d\" sx sy sz\n",
    "    puts $\n",
    "}",
);

#[test]
fn test_command_context_binders_are_completed() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = format!("{COMMAND_CONTEXTS}\n");
    lsp.open_ready(&uri, &src);
    let ls: BTreeSet<String> =
        labels(&lsp.completion(&uri, 5, u32::try_from("    puts $".len()).unwrap()))
            .into_iter()
            .collect();
    for v in [
        "$la", "$lb", "$lc", "$rf", "$rk", "$rv", "$rout", "$sx", "$sy", "$sz",
    ] {
        assert!(ls.contains(v), "missing {v}: {ls:?}");
    }
}

#[test]
fn test_global_is_colon_qualified_from_namespace_proc() {
    // From inside `::myns::helper`, the global `foo-bar` is only reachable as
    // `$::foo-bar` (a bare `$foo-bar` there is a local lookup), and the hyphen
    // forces the brace form.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let lines = [
        "set ::globalvar 9",
        "set \"foo-bar\" 1",
        "namespace eval ::myns {",
        "    proc helper {arg} {",
        "        puts $",
        "    }",
        "}",
    ];
    lsp.open_ready(&uri, &format!("{}\n", lines.join("\n")));
    let result = lsp.completion(&uri, 4, u32::try_from("        puts $".len()).unwrap());
    let items = completion_items(&result);
    let item = items
        .iter()
        .find(|i| i.get("label").and_then(Value::as_str) == Some("$::foo-bar"));
    assert!(item.is_some(), "missing $::foo-bar: {:?}", labels(&result));
    let item = item.unwrap();
    let inserted = item
        .get("textEdit")
        .and_then(|te| te.get("newText"))
        .and_then(Value::as_str)
        .or_else(|| item.get("insertText").and_then(Value::as_str));
    assert_eq!(inserted, Some("${::foo-bar}"));
}

#[test]
fn test_shebang_dialect_hides_newer_commands() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "#!/usr/bin/env tclsh8.5\ntr\n");
    let ls: BTreeSet<String> = labels(&lsp.completion(&uri, 1, 2)).into_iter().collect();
    assert!(!ls.contains("try"));
}

#[test]
fn test_directive_dialect_hides_newer_commands() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "# tcl-dialect: tcl8.4\ntr\n");
    let ls: BTreeSet<String> = labels(&lsp.completion(&uri, 1, 2)).into_iter().collect();
    assert!(!ls.contains("try"));
}

// ── commands ─────────────────────────────────────────────────────────────────

// -- TestCommandParity ---------------------------------------------------

#[test]
fn test_set_dialect_returns_success() {
    let mut lsp = Lsp::tcl();
    let r = lsp.execute_command("tcl-lsp.setDialect", json!(["tcl8.6"]));
    assert!(r.is_object());
    assert!(r.get("success").and_then(Value::as_bool).is_some());
    assert_eq!(r.get("success").and_then(Value::as_bool), Some(true));
}

#[test]
fn test_compiler_explorer_valid_source() {
    let mut lsp = Lsp::tcl();
    let r = lsp.execute_command(
        "tcl-lsp.compilerExplorer",
        json!(["set x 10\nputs $x\n", "tcl8.6"]),
    );
    assert!(r.is_object());
    assert!(r.get("error").is_none_or(Value::is_null));
    assert!(r.get("ir").is_some());
}

#[test]
fn test_compiler_explorer_empty_source_is_error() {
    let mut lsp = Lsp::tcl();
    let r = lsp.execute_command("tcl-lsp.compilerExplorer", json!(["", "tcl8.6"]));
    assert!(r.is_object());
    let err = r.get("error");
    assert!(err.is_some() && !err.unwrap().is_null());
}

// ── config toggles ───────────────────────────────────────────────────────────

// -- TestConfigToggleParity ----------------------------------------------

#[test]
fn test_folding_toggle_suppresses_ranges() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc f {} {\n  set x 1\n  set y 2\n}\n");
    // present by default (Python asserts the raw result is truthy — a non-empty
    // `FoldingRange[]`; these use `startLine`/`endLine`, not position `Range`s).
    assert!(
        lsp.folding_range(&uri)
            .as_array()
            .is_some_and(|a| !a.is_empty())
    );
    // foldingRange is a plain request with nothing to block on, so it can race
    // ahead of the async didChangeConfiguration apply. Don't poll the effective
    // config — instead wait for the message that proves the toggle landed: a
    // config change reschedules every open doc, so the server re-publishes this
    // document's diagnostics *after* applying the config. Waiting for that
    // publish orders the folding request behind the apply.
    lsp.clear_notifications();
    lsp.apply_configuration(json!({ "features": { "folding": false } }));
    lsp.await_diagnostics_version(&uri, Some(1), Duration::from_secs(15));
    // An authoritative empty set (not `None`) so the client does not fall back
    // to built-in indentation folding.
    assert_eq!(lsp.folding_range(&uri), json!([]));
}

#[test]
fn test_optimiser_toggle_suppresses_o_codes() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "if {$x == \"foo\"} { puts yes }\n";
    let base = lsp.open_ready(&uri, src);
    assert!(codes(&base).iter().any(|c| c.starts_with('O')));
    lsp.clear_notifications();
    lsp.apply_configuration(json!({ "optimiser": { "enabled": false } }));
    let diags = lsp.await_diagnostics_version(&uri, Some(1), Duration::from_secs(15));
    assert!(!codes(&diags).iter().any(|c| c.starts_with('O')));
}

#[test]
fn test_hint_only_optimisation_diagnostic_carries_no_apply_payload() {
    // An O102 rewrite whose use site is a bare `$var` argv word gets a
    // precise, applicable per-argv span and carries `data.replacement` /
    // `startOffset` / `endOffset` for a client to auto-apply. A use nested
    // inside a larger construct (here, `$a` inside `[expr {...}]`) falls
    // back to a *hint-only* diagnostic spanning the whole consuming
    // statement — that span plus the bare-literal replacement is not a
    // safe text splice (it would replace `set v [expr {$a * 2}]` with the
    // standalone fragment `10`), so the diagnostic must carry no `data`
    // payload at all: a client that blindly applied `data` on every
    // diagnostic would otherwise corrupt the statement.
    // O102 is `constant_folding` category, outside the default `readability`
    // editor profile — opt into `standard` so it surfaces.
    let mut lsp = Lsp::with_config(json!({ "optimiser": { "profile": "standard" } }));
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set a 5\nset v [expr {$a * 2}]\nputs $v\n");
    let hint = diags
        .iter()
        .find(|d| code_str(d).as_deref() == Some("O102"))
        .unwrap_or_else(|| panic!("expected an O102 diagnostic, got {:?}", codes(&diags)));
    assert!(
        hint.get("data").is_none(),
        "hint-only O102 must carry no data payload, got {hint:?}"
    );
}

#[test]
fn test_applicable_optimisation_diagnostic_carries_apply_payload() {
    // Control for the hint-only case above: a bare `$var` argv-word use
    // gets a precise, applicable O102 rewrite, which *does* carry a
    // `data` payload a client can use to apply the fix directly.
    let mut lsp = Lsp::with_config(json!({ "optimiser": { "profile": "standard" } }));
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set n 7\nputs $n\n");
    let applicable = diags
        .iter()
        .find(|d| code_str(d).as_deref() == Some("O102"))
        .unwrap_or_else(|| panic!("expected an O102 diagnostic, got {:?}", codes(&diags)));
    let data = applicable
        .get("data")
        .unwrap_or_else(|| panic!("expected a data payload, got {applicable:?}"));
    assert_eq!(data.get("replacement").and_then(Value::as_str), Some("7"));
}

#[test]
fn test_diagnostics_master_switch_clears_all() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(!lsp.open_ready(&uri, "catch {error e}\n").is_empty()); // non-empty by default
    lsp.clear_notifications();
    lsp.apply_configuration(json!({ "features": { "diagnostics": false } }));
    assert_eq!(
        lsp.await_diagnostics_version(&uri, Some(1), Duration::from_secs(15)),
        Vec::<Value>::new()
    );
}

#[test]
fn test_optimiser_code_override_does_not_leak() {
    // A one-off `optimiser.O100 = true` must not persist after it is cleared:
    // rebuilding the override map authoritatively reverts to the profile.
    let mut lsp = Lsp::tcl();
    let clean = "set x 10\nset y 20\nset sum [expr {$x + $y}]\n";
    lsp.apply_configuration(json!({ "optimiser": { "O100": true } }));
    // (Python enters and immediately exits the config_session, restoring the
    // profile; here each test owns its server, so re-assert the default profile
    // to mirror the restore.)
    lsp.apply_configuration(json!({ "optimiser": { "enabled": true } }));
    let uri = unique_uri("tcl");
    assert!(!codes(&lsp.open_ready(&uri, clean)).contains(&"O100".to_owned()));
}

// ── capabilities ─────────────────────────────────────────────────────────────

// -- TestCapabilityParity ------------------------------------------------

#[test]
fn test_type_hierarchy_advertised() {
    let lsp = Lsp::tcl();
    let caps = lsp
        .initialize_result()
        .get("capabilities")
        .cloned()
        .unwrap_or(Value::Null);
    let thp = caps.get("typeHierarchyProvider");
    assert!(
        thp.is_some_and(|v| !v.is_null() && v != &Value::Bool(false)),
        "typeHierarchyProvider not advertised: {caps:?}"
    );
}

#[test]
fn test_pull_diagnostics_not_advertised_by_default() {
    let lsp = Lsp::tcl();
    let caps = lsp
        .initialize_result()
        .get("capabilities")
        .cloned()
        .unwrap_or(Value::Null);
    assert!(
        caps.get("diagnosticProvider").is_none_or(Value::is_null),
        "diagnosticProvider should not be advertised: {caps:?}"
    );
}

// ── navigation ───────────────────────────────────────────────────────────────

// -- TestNavigationParity ------------------------------------------------

#[test]
fn test_method_parameter_definition_resolves_to_name() {
    let lines = [
        "oo::class create C {",
        "    method greet {name greeting} {",
        "        return \"$greeting, $name\"",
        "    }",
        "}",
    ];
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, &format!("{}\n", lines.join("\n")));
    let idx = lines[2].rfind("name").unwrap();
    let res = lsp.definition(&uri, 2, u32::try_from(idx).unwrap() + 1);
    let locs = locations(&res);
    assert!(!locs.is_empty());
    let rng = &locs[0].range;
    assert_eq!(rng["start"]["line"].as_i64(), Some(1)); // the `name` param on the method line
    assert_eq!(rng["start"]["line"], rng["end"]["line"]);
    let start_ch = rng["start"]["character"].as_i64().unwrap();
    let end_ch = rng["end"]["character"].as_i64().unwrap();
    assert!(end_ch - start_ch <= i64::try_from("greeting".len()).unwrap());
}

// ── code actions / lenses ────────────────────────────────────────────────────

// -- TestCodeActionParity ------------------------------------------------

#[test]
fn test_code_lens_resolves_show_references_command() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc greet {} { return 1 }\ngreet\ngreet\n");
    let lenses = lsp.code_lens(&uri);
    let lenses = lenses.as_array().cloned().unwrap_or_default();
    // Reference-count lenses are returned WITHOUT a command so the client
    // resolves them; resolve then wires the clickable showReferences command.
    let unresolved: Vec<&Value> = lenses
        .iter()
        .filter(|lz| {
            lz.get("command").is_none_or(Value::is_null)
                && lz.get("data").is_some_and(|d| !d.is_null())
        })
        .collect();
    assert!(
        !unresolved.is_empty(),
        "expected an unresolved reference lens, got {lenses:?}"
    );
    let resolved = lsp.code_lens_resolve(unresolved[0].clone());
    let cmd = resolved.get("command").cloned().unwrap_or(Value::Null);
    assert_eq!(
        cmd.get("command").and_then(Value::as_str),
        Some("tcl-lsp.showReferences")
    );
    let args = cmd.get("arguments");
    let args = args.and_then(Value::as_array);
    assert!(args.is_some());
    let args = args.unwrap();
    assert_eq!(args.len(), 3);
    assert!(args[0].is_string());
}

#[test]
fn test_brace_expr_refactor_offered() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "expr $a + $b\n");
    // VS Code invokes refactors at the caret (column 0 on the `expr` word).
    let actions = lsp.code_actions(&uri, range((0, 0), (0, 0)), Value::Array(diags));
    let actions = actions.as_array().cloned().unwrap_or_default();
    let brace = actions.iter().find(|a| {
        a.get("title")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("Brace expr")
    });
    assert!(
        brace.is_some(),
        "no Brace expr action: {:?}",
        actions
            .iter()
            .map(|a| a.get("title").cloned())
            .collect::<Vec<_>>()
    );
    let brace = brace.unwrap();
    assert_eq!(
        brace.get("kind").and_then(Value::as_str),
        Some("refactor.rewrite")
    );
    assert!(
        edits_of(brace)
            .iter()
            .any(|e| e.get("newText").and_then(Value::as_str) == Some("{$a + $b}"))
    );
}

// ── follow-ups from PR #733 review (Codex bot) ───────────────────────────────

// -- TestReviewFollowups -------------------------------------------------

#[test]
fn test_local_shadowing_global_stays_bare() {
    // A proc-local `foo` shadows the global `foo`, so completion must keep `$foo`
    // (the local) and NOT rewrite it to `$::foo` (the global).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set foo global\nproc p {} {\n    set foo local\n    puts $\n}\n";
    lsp.open_ready(&uri, src);
    let ls: BTreeSet<String> =
        labels(&lsp.completion(&uri, 3, u32::try_from("    puts $".len()).unwrap()))
            .into_iter()
            .collect();
    assert!(ls.contains("$foo") && !ls.contains("$::foo"), "{ls:?}");
}

#[test]
fn test_var_binders_fire_inside_command_substitutions() {
    // `regexp` inside an `if` condition and `scan` inside a `set` value are `[…]`
    // substitutions; their output vars must still be bound.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = concat!(
        "proc p {s} {\n",
        "    if {[regexp {(\\w+)} $s m]} {\n",
        "        set n [scan $s \"%d\" x]\n",
        "        puts $\n",
        "    }\n",
        "}\n",
    );
    lsp.open_ready(&uri, src);
    let ls: BTreeSet<String> =
        labels(&lsp.completion(&uri, 3, u32::try_from("        puts $".len()).unwrap()))
            .into_iter()
            .collect();
    assert!(ls.contains("$m") && ls.contains("$x"), "{ls:?}");
}

#[test]
fn test_dialect_reresolves_after_edit() {
    // Adding a `# tcl-dialect:` directive to an already-open buffer must take
    // effect without reopening: `try` (8.6+) disappears from completion.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "tr\n");
    let ls: BTreeSet<String> = labels(&lsp.completion(&uri, 0, 2)).into_iter().collect();
    assert!(ls.contains("try"));
    lsp.replace_document(&uri, 2, "# tcl-dialect: tcl8.4\ntr\n");
    lsp.await_diagnostics_version(&uri, Some(2), Duration::from_secs(10));
    let ls: BTreeSet<String> = labels(&lsp.completion(&uri, 1, 2)).into_iter().collect();
    assert!(!ls.contains("try"));
}
