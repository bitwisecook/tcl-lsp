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

//! Lookup verbs: `command-info` (registry metadata).
//!
//! Drives the command registry in `tcl-registry` to report command metadata.

use std::path::Path;

use serde::Serialize;
use tcl_cli_support::{OutputTarget, registry_for_dialect, write_text_output};
use tcl_dialect::DialectProfile;
use tcl_registry::ProfileQueries;

/// JSON payload when the command resolves (fields are emitted in a fixed order).
#[derive(Serialize)]
struct FoundPayload {
    found: bool,
    command: String,
    dialect: String,
    summary: String,
    synopsis: Vec<String>,
    switches: Vec<String>,
    #[serde(rename = "validEvents")]
    valid_events: Vec<String>,
}

/// JSON payload when the command is unknown.
#[derive(Serialize)]
struct NotFoundPayload {
    found: bool,
    command: String,
    dialect: String,
}

/// `tcl command-info` — look up registry metadata for one command.
pub fn run_command_info(
    command: &str,
    dialect: &DialectProfile,
    json: bool,
    output: Option<&Path>,
) -> anyhow::Result<u8> {
    let query = command.trim();
    if query.is_empty() {
        anyhow::bail!("command name is required");
    }
    let registry = registry_for_dialect(dialect.name);
    let target = OutputTarget::from_arg(output);

    // Exact match, then a case-insensitive fallback (mirrors
    // lookup_command_info). Resolution is dialect-true (§5.1): a command
    // that exists in the data but is unavailable under `dialect` — banned
    // in iRules, version-gated above the profile's base — reports
    // not-found for that dialect.
    let resolved_name: Option<String> = if dialect.resolve_command(&registry, query).is_some() {
        Some(query.to_owned())
    } else {
        let lowered = query.to_lowercase();
        registry
            .command_names()
            .find(|c| {
                c.to_lowercase() == lowered && dialect.resolve_command(&registry, c).is_some()
            })
            .map(str::to_owned)
    };

    let Some(resolved_name) = resolved_name else {
        if json {
            let payload = NotFoundPayload {
                found: false,
                command: query.to_owned(),
                dialect: dialect.name.to_owned(),
            };
            write_text_output(
                &target,
                &tcl_cli_support::ensure_ascii(&serde_json::to_string_pretty(&payload)?),
            )?;
        } else {
            write_text_output(
                &target,
                &format!("command not found: {query} (dialect={})", dialect.name),
            )?;
        }
        return Ok(1);
    };

    let spec = dialect
        .resolve_command(&registry, &resolved_name)
        .expect("resolved command spec");
    let summary = spec
        .hover
        .as_ref()
        .map_or(String::new(), |h| h.summary.to_owned());
    let synopsis: Vec<String> = spec
        .hover
        .as_ref()
        .map(|h| h.synopsis.iter().map(|s| (*s).to_owned()).collect())
        .unwrap_or_default();
    // §5.2 option gating (intersects + version ceiling) — the same rule
    // hover/completion/the snapshot use.
    let mut switches: Vec<String> = dialect
        .available_option_names(spec)
        .into_iter()
        .map(str::to_owned)
        .collect();
    switches.sort();
    // NOTE: iRules `validEvents` resolution (event_requires → events_matching)
    // is not implemented; non-iRules dialects resolve to an empty list anyway.
    let valid_events: Vec<String> = Vec::new();

    if json {
        let payload = FoundPayload {
            found: true,
            command: resolved_name.clone(),
            dialect: dialect.name.to_owned(),
            summary,
            synopsis,
            switches,
            valid_events,
        };
        write_text_output(
            &target,
            &tcl_cli_support::ensure_ascii(&serde_json::to_string_pretty(&payload)?),
        )?;
        return Ok(0);
    }

    let mut lines = vec![
        format!("command: {resolved_name}"),
        format!("dialect: {}", dialect.name),
    ];
    if !summary.is_empty() {
        lines.push(format!("summary: {summary}"));
    }
    for item in &synopsis {
        lines.push(format!("synopsis: {item}"));
    }
    if !switches.is_empty() {
        lines.push(format!("switches: {}", switches.join(", ")));
    }
    if !valid_events.is_empty() {
        let shown: Vec<&str> = valid_events.iter().take(20).map(String::as_str).collect();
        lines.push(format!(
            "valid events ({}): {}",
            valid_events.len(),
            shown.join(", ")
        ));
    }
    write_text_output(&target, &lines.join("\n"))?;
    Ok(0)
}
