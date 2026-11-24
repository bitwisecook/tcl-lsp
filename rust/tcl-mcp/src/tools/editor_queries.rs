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

//! The tools that answer what an editor asks: command and event lookups, hover, completion, definition, references and rename.

use super::{IRULES_DIALECT, analyse, arg_str, arg_u32, lsp_range_json, registry, resolve_dialect};
use serde_json::{Value, json};
use tcl_registry::events::EventRegistry;
use tcl_registry::profiles::ProfileRegistry;

// ── Registry info tools ───────────────────────────────────────────────

pub(super) fn event_info(args: &Value) -> Value {
    let event = arg_str(args, "event_name");
    let reg = registry(IRULES_DIALECT);
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    let info = reg.event_info(event, &events, &profiles, None);
    let valid_command_count = info.valid_commands.len();
    json!({
        "event": info.event,
        "known": info.known,
        // The three lifecycle releases, same names and null semantics as the
        // CLI/query/snapshot surfaces; `retired_version` is exclusive.
        "introduced_version": info.lifecycle.introduced,
        "deprecated_version": info.lifecycle.deprecated,
        "retired_version": info.lifecycle.retired,
        "lifecycle_state": info.lifecycle_state.as_str(),
        "multiplicity": info.multiplicity,
        "description": info.description,
        "side": info.side,
        "transport": info.transport,
        "implied_profiles": info.implied_profiles,
        "valid_commands": info.valid_commands,
        "valid_command_count": valid_command_count,
    })
}

pub(super) fn command_info(args: &Value) -> Value {
    let command = arg_str(args, "command_name");
    let reg = registry(IRULES_DIALECT);
    // Query availability through the fixed iRules context over this store.
    let irules = crate::environment::context_for_dialect(IRULES_DIALECT);
    let Some(spec) = irules.resolve_spec(&reg, command) else {
        return json!({ "command": command, "found": false });
    };
    // Optional BIG-IP target release: report the declared version range
    // (explicit introduction data, or the 15.0 axis baseline) and whether
    // the command exists at the target.
    let bigip_version = args
        .get("bigip_version")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let keyed_range = irules.keyed_version_range(spec);
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    let mut obj = json!({ "command": command, "found": true });
    let m = obj.as_object_mut().expect("json object");
    if let Some(h) = spec.hover {
        if !h.summary.is_empty() {
            m.insert("summary".to_owned(), json!(h.summary));
        }
        if !h.synopsis.is_empty() {
            m.insert("synopsis".to_owned(), json!(h.synopsis));
        }
    }
    let switches = irules.available_option_names(spec);
    if !switches.is_empty() {
        m.insert("switches".to_owned(), json!(switches));
    }
    let valid_events = reg.irules_events_for_command(command, &events, &profiles);
    if !valid_events.is_empty() {
        m.insert("valid_events".to_owned(), json!(valid_events));
    }
    if let Some((min, max)) = keyed_range {
        m.insert("bigip_min_version".to_owned(), json!(min));
        m.insert("bigip_max_version".to_owned(), json!(max));
        if let Some(target) = bigip_version {
            m.insert(
                "available_at_bigip_version".to_owned(),
                json!(tcl_registry::version::within_range(target, min, max)),
            );
        }
    }
    obj
}

// ── LSP-feature tools ─────────────────────────────────────────────────

pub(super) const SOURCE_URI: &str = "file:///source.tcl";

pub(super) fn hover(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysis = analyse(source, &dialect);
    let context = analysis
        .resolved_input
        .as_ref()
        .map(tcl_compiler::analyser::ResolvedAnalysisInput::context_registry);
    let commands = context.as_ref().map(|context| context.commands().as_ref());
    match tcl_lsp_core::hover::hover(
        source,
        arg_u32(args, "line"),
        arg_u32(args, "character"),
        &analysis,
        commands,
    ) {
        Some(h) => json!({ "contents": h.value }),
        None => Value::Null,
    }
}

pub(super) fn complete(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysis = analyse(source, &dialect);
    let context = analysis
        .resolved_input
        .as_ref()
        .map(tcl_compiler::analyser::ResolvedAnalysisInput::context_registry);
    let commands = context.as_ref().map(|context| context.commands().as_ref());
    let items = tcl_lsp_core::completion::completions(
        source,
        arg_u32(args, "line"),
        arg_u32(args, "character"),
        &analysis,
        commands,
        None,
        tcl_lsp_core::profile_for_dialect(&dialect),
    );
    let items_json: Vec<Value> = items
        .iter()
        .map(|i| {
            let mut obj = json!({ "label": i.label, "kind": completion_kind_str(i.kind), "insert_text": i.insert_text });
            let m = obj.as_object_mut().expect("json object");
            if let Some(d) = &i.detail {
                m.insert("detail".to_owned(), json!(d));
            }
            if let Some(d) = &i.documentation {
                m.insert("documentation".to_owned(), json!(d));
            }
            if let Some(s) = &i.sort_text {
                m.insert("sort_text".to_owned(), json!(s));
            }
            obj
        })
        .collect();
    json!({ "items": items_json, "total": items_json.len() })
}

fn completion_kind_str(k: tcl_lsp_core::completion::CompletionKind) -> &'static str {
    use tcl_lsp_core::completion::CompletionKind::{EnumValue, Function, Snippet, Variable};
    match k {
        Variable => "variable",
        Function => "function",
        EnumValue => "enum_member",
        Snippet => "snippet",
    }
}

pub(super) fn goto_definition(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysis = analyse(source, &dialect);
    let ranges = tcl_lsp_core::definition::definition(
        source,
        arg_u32(args, "line"),
        arg_u32(args, "character"),
        &analysis,
    );
    let locations: Vec<Value> = ranges
        .iter()
        .map(|r| json!({ "uri": SOURCE_URI, "range": lsp_range_json(r) }))
        .collect();
    json!({ "locations": locations })
}

pub(super) fn find_references(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysis = analyse(source, &dialect);
    let ranges = tcl_lsp_core::references::references(
        source,
        tcl_lsp_core::profile_for_dialect(&dialect),
        arg_u32(args, "line"),
        arg_u32(args, "character"),
        &analysis,
        true,
    );
    let refs: Vec<Value> = ranges
        .iter()
        .map(|r| json!({ "uri": SOURCE_URI, "range": lsp_range_json(r) }))
        .collect();
    json!({ "references": refs, "total": refs.len() })
}

pub(super) fn rename(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysis = analyse(source, &dialect);
    let context = analysis
        .resolved_input
        .as_ref()
        .map(tcl_compiler::analyser::ResolvedAnalysisInput::context_registry);
    let commands = context.as_ref().map(|context| context.commands().as_ref());
    let outcome = tcl_lsp_core::rename::rename_outcome(
        source,
        tcl_lsp_core::profile_for_dialect(&dialect),
        arg_u32(args, "line"),
        arg_u32(args, "character"),
        arg_str(args, "new_name"),
        &analysis,
        commands,
    );
    let edits = match outcome {
        Ok(edits) => edits,
        Err(refusal) => return json!({ "refused": { "reason": refusal.reason } }),
    };
    let edits_json: Vec<Value> = edits
        .iter()
        .map(|e| json!({ "range": lsp_range_json(&e.range), "new_text": e.new_text }))
        .collect();
    json!({ "edits": edits_json, "total": edits.len() })
}

#[cfg(test)]
mod refusal_tests {
    use super::*;

    #[test]
    fn unresolved_command_rename_reports_a_reason_instead_of_null() {
        let source = include_str!(
            "../../../../docs/work/name-handling/handover/oo-dispatch-probe/editor-actions/workspace/nosource.tcl"
        );
        let result = rename(
            &json!({ "source": source, "dialect": "tcl9.0", "line": 1, "character": 26, "new_name": "scaled" }),
        );
        let reason = result["refused"]["reason"]
            .as_str()
            .expect("visible refusal reason");
        assert!(reason.contains("unavailable"), "{reason}");
        assert!(!result.is_null());
    }

    #[test]
    fn inapplicable_rename_is_a_visible_refusal() {
        let result = rename(
            &json!({ "source": "# comment\n", "dialect": "tcl9.0", "line": 0, "character": 3, "new_name": "renamed" }),
        );
        assert!(
            result["refused"]["reason"]
                .as_str()
                .is_some_and(|reason| !reason.is_empty())
        );
    }
}
