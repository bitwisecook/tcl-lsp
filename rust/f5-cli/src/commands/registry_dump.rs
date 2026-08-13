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

//! The `registry-dump` verb — canonical JSON snapshots of the F5 registries.
//!
//! Serialises the F5 registry / graph snapshots from [`tcl_registry::snapshot`]
//! as two-space-indented, key-sorted JSON.
//!
//! ## Section coverage
//!
//! - `profiles` — emits the profile graph snapshot.
//! - `objects` — emits the object graph snapshot.
//! - `events` — emits the event graph snapshot (the per-event valid-command
//!   list is content-addressed via `validCommandsDigest`).
//! - `commands` — the full per-command `traits`/`scalars` dicts and the hover
//!   prose catalogue (`summary`) for the `f5-irules` dialect, from
//!   [`tcl_registry::command_snapshot`] (the same snapshot the `tcl
//!   registry-dump` verb serialises).
//! - `all` — every section in one object (`commands`/`events`/`objects`/`profiles`).

use std::collections::BTreeMap;
use std::path::Path;

use tcl_cli_support::registry_for_dialect;
use tcl_registry::command_snapshot::command_registry_snapshot;
use tcl_registry::snapshot::{
    Json, event_graph_snapshot, object_graph_snapshot, profile_graph_snapshot,
};

/// The `f5-irules` command-registry snapshot (core Tcl + iRules commands).
fn commands_snapshot() -> Json {
    command_registry_snapshot(&registry_for_dialect("f5-irules"), "f5-irules")
}

/// Run the `registry-dump` verb for `section`, writing to `output`
/// (`None` = stdout).
pub fn run_registry_dump(section: &str, output: Option<&Path>) -> anyhow::Result<u8> {
    let json = match section {
        "profiles" => profile_graph_snapshot(),
        "objects" => object_graph_snapshot(),
        "events" => event_graph_snapshot(),
        "commands" => commands_snapshot(),
        "all" => {
            let mut m = BTreeMap::new();
            m.insert("commands".to_owned(), commands_snapshot());
            m.insert("events".to_owned(), event_graph_snapshot());
            m.insert("objects".to_owned(), object_graph_snapshot());
            m.insert("profiles".to_owned(), profile_graph_snapshot());
            Json::Object(m)
        }
        other => anyhow::bail!("unknown registry-dump section: {other}"),
    };

    // Two-space-indented, key-sorted JSON with a single trailing
    // newline appended (to stdout, or to the file).
    let mut text = json.dumps_indent2();
    text.push('\n');

    if let Some(path) = output {
        std::fs::write(path, &text)
            .map_err(|e| anyhow::anyhow!("failed to write {}: {e}", path.display()))?;
    } else {
        use std::io::Write;
        std::io::stdout()
            .lock()
            .write_all(text.as_bytes())
            .map_err(|e| anyhow::anyhow!("failed to write stdout: {e}"))?;
    }
    Ok(0)
}
