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

//! Push diagnostics, end-to-end against the packaged server. The server
//! advertises no pull provider, so these assert on the `publishDiagnostics` the
//! server pushes after analysis, keyed by version.

use crate::common::{Lsp, scaled_timeout, unique_uri};

use serde_json::Value;
use std::collections::BTreeSet;
use std::time::Duration;

#[path = "diagnostics/arity.rs"]
mod arity;
#[path = "diagnostics/caller_frame_injection.rs"]
mod caller_frame_injection;
#[path = "diagnostics/code_canaries.rs"]
mod code_canaries;
#[path = "diagnostics/command_resolution.rs"]
mod command_resolution;
#[path = "diagnostics/constant_conditions.rs"]
mod constant_conditions;
#[path = "diagnostics/dialect_gating.rs"]
mod dialect_gating;
#[path = "diagnostics/method_bodies.rs"]
mod method_bodies;
#[path = "diagnostics/publication.rs"]
mod publication;
#[path = "diagnostics/shimmer.rs"]
mod shimmer;
#[path = "diagnostics/style_checks.rs"]
mod style_checks;
#[path = "diagnostics/subcommand_dispatch.rs"]
mod subcommand_dispatch;
#[path = "diagnostics/syntax_errors.rs"]
mod syntax_errors;
#[path = "diagnostics/variable_flow.rs"]
mod variable_flow;

/// The set of `code` strings carried by `diags`.
fn codes(diags: &[Value]) -> BTreeSet<String> {
    diags
        .iter()
        .map(|d| match d.get("code") {
            Some(Value::String(s)) => s.clone(),
            Some(other) => other.to_string(),
            None => "None".to_owned(),
        })
        .collect()
}

/// Whether any diagnostic carries `code`.
fn has_code(diags: &[Value], code: &str) -> bool {
    codes(diags).contains(code)
}

/// The set of start lines for diagnostics carrying `code`.
fn on_line(diags: &[Value], code: &str) -> BTreeSet<i64> {
    diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some(code) && d.get("range").is_some())
        .filter_map(|d| {
            d.get("range")
                .and_then(|r| r.get("start"))
                .and_then(|s| s.get("line"))
                .and_then(Value::as_i64)
        })
        .collect()
}

/// A diagnostic's `code`, normalised to a `String` (stringifying non-strings).
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

/// Diagnostics from `diags` carrying `code`.
fn with_code(diags: &[Value], code: &str) -> Vec<Value> {
    diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some(code))
        .cloned()
        .collect()
}

// -- E004 malformed `if` — end-to-end. Each message/range is
// cross-checked against tclsh 8.6 and Tcl 9.0.4's `Tcl_IfObjCmd` source
// in the unit-level truth table (`tcl-registry`'s
// `commands::tcl::if_::tests`, `tcl-compiler`'s `analyser::state::tests`
// `tp_*` / `fp_*` / `tn_*` cases); this layer only asserts the
// diagnostic survives the full LSP round trip (server → JSON-RPC →
// `publishDiagnostics`) with the right code, message, and — critically
// — a *tight* range, not the whole statement.

/// A diagnostic's `range` as `((start_line, start_char), (end_line, end_char))`.
fn diag_range(d: &Value) -> ((i64, i64), (i64, i64)) {
    let get = |path: &[&str]| -> i64 {
        let mut v = d.get("range").expect("diagnostic has a range");
        for p in path {
            v = v.get(p).unwrap_or(&Value::Null);
        }
        v.as_i64().unwrap_or(-1)
    };
    (
        (get(&["start", "line"]), get(&["start", "character"])),
        (get(&["end", "line"]), get(&["end", "character"])),
    )
}

// One canary per analysis family, locked to the server's published output.

// W210 read-before-set, control-flow modelling family.

// End-to-end coverage for three W210 soundness hazards, checked against
// real tclsh 9.0.3 as ground truth.
