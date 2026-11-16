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

//! Code actions for the Tcl dialect, end-to-end against the packaged server.
//! The realistic flow drives diagnostics back into the `codeAction` context —
//! exactly what an editor does — so quick-fix offers are exercised against real
//! (or, where a specific range matters, fabricated) diagnostics.
//!
//! The F5 iRules-only code actions (collect-bootstrap, the IRULE3001/3002
//! wrap-with-encode taint fixes, profile headers) run against the dedicated
//! iRules server in `irules.rs`. T102 (tainted data in option position) is
//! dialect-general — not an iRules-specific check — so its `--`-insertion
//! fix is exercised here against the plain `tcl` dialect.

use crate::common::helpers::assert_fix_applies;
use crate::common::{Lsp, unique_uri};

use serde_json::{Value, json};

#[path = "code_actions/analyser_quick_fixes.rs"]
mod analyser_quick_fixes;
#[path = "code_actions/quick_fixes.rs"]
mod quick_fixes;
#[path = "code_actions/refactor_actions.rs"]
mod refactor_actions;

// Local helpers.

/// An LSP `Range` from `(line, char)` tuples.
fn range(start: (u32, u32), end: (u32, u32)) -> Value {
    json!({
        "start": { "line": start.0, "character": start.1 },
        "end": { "line": end.0, "character": end.1 },
    })
}

/// Request code actions with an explicit `only` filter (the harness's
/// `code_actions` has no `only` parameter, so build the request directly).
fn code_actions_only(lsp: &mut Lsp, uri: &str, rng: Value, diags: Value, only: &[&str]) -> Value {
    let mut params = json!({ "textDocument": { "uri": uri }, "context": { "only": only } });
    params["range"] = rng;
    params["context"]["diagnostics"] = diags;
    lsp.request("textDocument/codeAction", params)
}

/// Every `newText` in a code-action list's workspace edits (`_new_texts`).
fn new_texts(actions: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let empty = Vec::new();
    for action in actions.as_array().unwrap_or(&empty) {
        let edit = action.get("edit").cloned().unwrap_or(Value::Null);
        if let Some(changes) = edit.get("changes").and_then(Value::as_object) {
            for edits in changes.values() {
                for e in edits.as_array().unwrap_or(&empty) {
                    if let Some(t) = e.get("newText").and_then(Value::as_str) {
                        out.push(t.to_owned());
                    }
                }
            }
        }
        if let Some(doc_changes) = edit.get("documentChanges").and_then(Value::as_array) {
            for change in doc_changes {
                for e in change
                    .get("edits")
                    .and_then(Value::as_array)
                    .unwrap_or(&empty)
                {
                    if let Some(t) = e.get("newText").and_then(Value::as_str) {
                        out.push(t.to_owned());
                    }
                }
            }
        }
    }
    out
}

/// The `range` of the first workspace edit found on `action` — checks
/// `edit.changes` first, then `edit.documentChanges`, same dual path as
/// [`new_texts`].
fn first_edit_range(action: &Value) -> Value {
    let edit = action.get("edit").cloned().unwrap_or(Value::Null);
    if let Some(r) = edit
        .get("changes")
        .and_then(Value::as_object)
        .and_then(|m| m.values().next())
        .and_then(Value::as_array)
        .and_then(|edits| edits.first())
        .map(|e| e["range"].clone())
    {
        return r;
    }
    edit.get("documentChanges")
        .and_then(Value::as_array)
        .and_then(|dc| dc.first())
        .and_then(|change| change.get("edits"))
        .and_then(Value::as_array)
        .and_then(|edits| edits.first())
        .map(|e| e["range"].clone())
        .expect("action carries a workspace edit")
}

/// The `title` of every action (`_titles`).
fn titles(actions: &Value) -> Vec<String> {
    actions.as_array().map_or_default(|a| {
        a.iter()
            .map(|x| {
                x.get("title")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned()
            })
            .collect()
    })
}

/// The `title` of a single action.
fn action_title(action: &Value) -> &str {
    action.get("title").and_then(Value::as_str).unwrap_or("")
}

/// Actions whose `kind` starts with `kind_prefix` (`_kinds`).
fn kinds(actions: &Value, kind_prefix: &str) -> Vec<Value> {
    actions.as_array().map_or_default(|a| {
        a.iter()
            .filter(|x| {
                x.get("kind")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .starts_with(kind_prefix)
            })
            .cloned()
            .collect()
    })
}

/// A synthetic diagnostic.
fn diag(code: &str, message: &str, start: (u32, u32), end: (u32, u32)) -> Value {
    json!({
        "range": {
            "start": { "line": start.0, "character": start.1 },
            "end": { "line": end.0, "character": end.1 },
        },
        "code": code,
        "message": message,
        "source": "tcl-lsp",
    })
}

/// Diagnostics from `diags` carrying `code`.
fn with_code(diags: &[Value], code: &str) -> Vec<Value> {
    diags
        .iter()
        .filter(|d| d.get("code").and_then(Value::as_str) == Some(code))
        .cloned()
        .collect()
}

/// The `_rewrite` helper: request `refactor.rewrite`-only actions.
fn rewrite(lsp: &mut Lsp, uri: &str, start: (u32, u32), end: (u32, u32)) -> Vec<Value> {
    let actions = code_actions_only(
        lsp,
        uri,
        range(start, end),
        json!([]),
        &["refactor.rewrite"],
    );
    kinds(&actions, "refactor.rewrite")
}
