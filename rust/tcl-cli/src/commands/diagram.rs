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

//! `diagram` verb: extract structured control-flow data from the IR.
//!
//! The extraction itself lives in [`tcl_diagram`] (shared with the
//! `tcl_lsp_py` facade); this verb is the CLI I/O + text-rendering wrapper
//! that combines the inputs, resolves the registry, and prints the
//! `{events, procedures}` flow tree the VS Code `/diagram` command forwards
//! to the LLM.

use serde_json::Value;
use tcl_cli_support::{
    OutputTarget, combine_sources, combined_effective_dialect, ensure_ascii, read_input_documents,
    registry_for_dialect, write_text_output,
};
use tcl_diagram as diagram;

use crate::cli::InputArgs;

/// `tcl diagram` — extract control-flow diagram data from the IR.
pub fn run_diagram(input: &InputArgs, json_out: bool) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let dialect = combined_effective_dialect(&documents, input.dialect.as_deref());
    let source = combine_sources(&documents);
    let registry = registry_for_dialect(&dialect);
    let data = diagram::diagram_data_for_dialect(&source, registry, &dialect);

    let target = OutputTarget::from_arg(input.output.as_deref());

    if json_out {
        let text = ensure_ascii(&serde_json::to_string_pretty(&data)?);
        write_text_output(&target, &text)?;
        return Ok(0);
    }

    let empty: Vec<Value> = Vec::new();
    let events = data
        .get("events")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let procedures = data
        .get("procedures")
        .and_then(Value::as_array)
        .unwrap_or(&empty);

    let mut lines = vec![format!(
        "diagram: events={} procedures={}",
        events.len(),
        procedures.len()
    )];
    if !events.is_empty() {
        lines.push("events:".to_owned());
        for event in events {
            let name = event.get("name").and_then(Value::as_str).unwrap_or("?");
            let multiplicity = event
                .get("multiplicity")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let flow_count = event
                .get("flow")
                .and_then(Value::as_array)
                .map_or(0, Vec::len);
            lines.push(format!("  {name} ({multiplicity}) nodes={flow_count}"));
        }
    }
    if !procedures.is_empty() {
        lines.push("procedures:".to_owned());
        for proc in procedures {
            let name = proc.get("name").and_then(Value::as_str).unwrap_or("?");
            let params = proc
                .get("params")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .map(|v| v.as_str().map_or_else(|| v.to_string(), str::to_owned))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            let flow_count = proc
                .get("flow")
                .and_then(Value::as_array)
                .map_or(0, Vec::len);
            lines.push(format!("  {name}({params}) nodes={flow_count}"));
        }
    }
    write_text_output(&target, &lines.join("\n"))?;
    Ok(0)
}
