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

//! Native port of `tests/lsp_e2e/test_registry_contract_e2e.py`.
//!
//! The LSP front-end must surface the same registry as the golden fixtures.
//! These drive the real `workspace/executeCommand` registry-lookup handlers
//! (`listIruleEvents`, `describeIruleEvent`, `describeIruleCommand`,
//! `listSubcommands`) against the committed presence CSVs, so a Rust server
//! passes them unchanged.


use crate::common::Lsp;

use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::PathBuf;

/// The committed baseline directory, resolved relative to this crate's manifest.
/// These golden CSVs live in the crate (`tests/fixtures/registry/`) so the
/// registry-contract test is self-contained now that the repo-root `tests/`
/// tree is gone.
fn baseline_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/registry")
}

/// A parsed CSV row keyed by header name. The registry baselines use plain,
/// unquoted comma-separated values, so a straight split is faithful.
type Row = std::collections::HashMap<String, String>;

/// Parse `name` under the baseline dir into header-keyed rows.
fn read_csv(name: &str) -> Vec<Row> {
    let path = baseline_dir().join(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let mut lines = text.lines();
    let header: Vec<String> = lines
        .next()
        .expect("csv header")
        .split(',')
        .map(str::to_owned)
        .collect();
    lines
        .filter(|l| !l.is_empty())
        .map(|line| {
            let fields: Vec<&str> = line.split(',').collect();
            header
                .iter()
                .cloned()
                .zip(fields.iter().map(ToString::to_string))
                .collect()
        })
        .collect()
}

/// The known-event rows (`known == "true"`).
fn known_events() -> Vec<Row> {
    read_csv("events.csv")
        .into_iter()
        .filter(|e| e.get("known").map(String::as_str) == Some("true"))
        .collect()
}

/// The sorted `f5-irules` command names, sampled every 7th (`_IRULES[::7]`).
fn command_sample() -> Vec<String> {
    let mut irules: Vec<String> = read_csv("commands.csv")
        .into_iter()
        .filter(|r| r.get("dialect").map(String::as_str) == Some("f5-irules"))
        .filter_map(|r| r.get("command").cloned())
        .collect();
    irules.sort();
    irules.into_iter().step_by(7).collect()
}

// -- tests ---------------------------------------------------------------

#[test]
fn list_irule_events_matches_known_events() {
    let mut lsp = Lsp::irules();
    let data = lsp.execute_command("tcl-lsp.listIruleEvents", json!([]));
    let served: BTreeSet<String> = data["events"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect())
        .unwrap_or_default();
    let expected: BTreeSet<String> = known_events()
        .iter()
        .filter_map(|e| e.get("event").cloned())
        .collect();
    let missing: Vec<&String> = expected.difference(&served).collect();
    let extra: Vec<&String> = served.difference(&expected).collect();
    assert!(
        served == expected,
        "missing={missing:?} extra={extra:?}"
    );
}

#[test]
fn describe_every_known_event_count_matches_csv() {
    let mut lsp = Lsp::irules();
    let mut failures: Vec<String> = Vec::new();
    for entry in known_events() {
        let event = entry.get("event").cloned().unwrap_or_default();
        let data = lsp.execute_command("tcl-lsp.describeIruleEvent", json!([event]));
        let valid_count = match data.get("validCommandCount") {
            Some(Value::Number(n)) => n.to_string(),
            Some(Value::String(s)) => s.clone(),
            other => format!("{other:?}"),
        };
        if valid_count != *entry.get("valid_commands").unwrap() {
            failures.push(format!(
                "{event}: lsp={valid_count} csv={}",
                entry.get("valid_commands").unwrap()
            ));
        }
        let deprecated = match data.get("deprecated") {
            Some(Value::Bool(b)) => b.to_string(),
            Some(Value::String(s)) => s.to_lowercase(),
            other => format!("{other:?}"),
        };
        if deprecated != *entry.get("deprecated").unwrap() {
            failures.push(format!("{event}: deprecated mismatch"));
        }
    }
    assert!(failures.is_empty(), "{}", failures[..failures.len().min(40)].join("\n"));
}

#[test]
fn describe_sampled_irule_commands_are_found() {
    let mut lsp = Lsp::irules();
    let mut failures: Vec<String> = Vec::new();
    for command in command_sample() {
        let data = lsp.execute_command("tcl-lsp.describeIruleCommand", json!([command]));
        if data.get("found") != Some(&Value::Bool(true)) {
            failures.push(command);
        }
    }
    assert!(
        failures.is_empty(),
        "describeIruleCommand not found for: {:?}",
        &failures[..failures.len().min(20)]
    );
}

#[test]
fn list_subcommands_covers_known_ensembles() {
    // A handful of well-known core ensembles and the subcommands the LSP must
    // surface for them (`listSubcommands` resolves dialect-agnostically).
    let ensembles: &[(&str, &[&str])] = &[
        ("string", &["length", "range", "map", "tolower", "trim"]),
        ("dict", &["get", "set", "keys", "exists", "merge"]),
        ("info", &["body", "exists", "commands", "vars"]),
        ("file", &["exists", "join", "dirname", "extension"]),
        ("clock", &["seconds", "format", "scan"]),
        ("array", &["get", "set", "names", "exists"]),
    ];
    let mut lsp = Lsp::tcl();
    let mut failures: Vec<String> = Vec::new();
    for (command, expected) in ensembles {
        let data = lsp.execute_command("tcl-lsp.listSubcommands", json!([command]));
        let served: BTreeSet<String> = data["subcommands"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|s| s.get("name").and_then(Value::as_str).map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        let missing: Vec<&str> = expected
            .iter()
            .filter(|e| !served.contains(**e))
            .copied()
            .collect();
        if !missing.is_empty() {
            let mut sorted = missing;
            sorted.sort_unstable();
            failures.push(format!("{command}: LSP missing subcommands {sorted:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
