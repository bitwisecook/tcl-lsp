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

//! `tcl explore` — compiler-explorer views over aggregated input.
//!
//! Drives the `tcl-explorer` pipeline + serialiser (the same contract the
//! GUI / TUI / editor panels consume). `--json` emits the full
//! machine-readable JSON; the default is a compact per-view summary. The
//! JSON path exposes every view.

use serde_json::Value;

use tcl_cli_support::{
    OutputTarget, combine_sources, combined_effective_dialect, read_input_documents,
    resolve_use_colour, write_text_output,
};

use crate::cli::{ColourArgs, InputArgs};

/// Handle `tcl explore`.
pub fn run_explore(
    input: &InputArgs,
    show: &[String],
    json: bool,
    text: bool,
    tui: bool,
    colour: &ColourArgs,
) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let dialect = combined_effective_dialect(&documents, input.dialect_profile()?);
    let source = combine_sources(&documents);

    // `tcl-explorer` deliberately resolves the process's active registry so
    // its pipeline matches the LSP. Initialising the CLI registry publishes
    // the discovered project pack set (and its hook plan) on that interface.
    let _registry = tcl_cli_support::registry_for_dialect(dialect.name);

    if tui {
        return run_tui(&source, dialect);
    }

    let result = tcl_explorer::run_pipeline(&source, dialect.name);
    let value = tcl_explorer::serialise_result(&result);

    let target = OutputTarget::from_arg(input.output.as_deref());
    let output = if json {
        serde_json::to_string_pretty(&value)?
    } else if text {
        let use_colour = resolve_use_colour(colour.colour, colour.no_colour, &target);
        tcl_explorer::render::render_all(&value, show, use_colour)
    } else {
        render_summary(&value, show)
    };
    write_text_output(&target, &output)?;
    Ok(0)
}

/// Launch the interactive TUI (feature `tui`), or explain how to enable it.
#[cfg(feature = "tui")]
fn run_tui(source: &str, dialect: &'static tcl_dialect::DialectProfile) -> anyhow::Result<u8> {
    crate::tui::run(source, dialect)?;
    Ok(0)
}

#[cfg(not(feature = "tui"))]
fn run_tui(_source: &str, _dialect: &'static tcl_dialect::DialectProfile) -> anyhow::Result<u8> {
    anyhow::bail!("the TUI is not compiled in — rebuild with `--features tui`");
}

/// A one-line description of a view's payload.
fn summarise(value: &Value) -> String {
    match value {
        Value::Array(a) => format!("{} item(s)", a.len()),
        Value::Object(o) => format!("{} field(s)", o.len()),
        Value::String(s) => format!("{} char(s)", s.chars().count()),
        Value::Null => "(none)".to_owned(),
        other => other.to_string(),
    }
}

/// A compact per-view summary of the serialised result, optionally filtered
/// to the views named in `show` (case-insensitive substring match).
fn render_summary(value: &Value, show: &[String]) -> String {
    let Some(obj) = value.as_object() else {
        return "compiler explorer: no result".to_owned();
    };
    let mut lines = vec!["Compiler explorer summary".to_owned()];
    let mut keys: Vec<&String> = obj.keys().collect();
    keys.sort();
    for key in keys {
        if !show.is_empty()
            && !show
                .iter()
                .any(|s| key.to_lowercase().contains(&s.to_lowercase()))
        {
            continue;
        }
        lines.push(format!("  {key}: {}", summarise(&obj[key])));
    }
    lines.join("\n")
}
