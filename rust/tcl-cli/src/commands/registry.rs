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

//! `registry-dump` verb: serialise the command registry as canonical JSON.
//!
//! Drives the snapshot builders in `tcl_registry::command_snapshot`. The output is
//! canonical 2-space-indented JSON with keys sorted.

use std::path::Path;

use tcl_cli_support::{OutputTarget, registry_for_dialect, write_text_output};
use tcl_registry::command_snapshot::{command_registry_snapshot, command_registry_snapshots};

/// The Tcl dialects `--all-dialects` snapshots, in stable order.
const TCL_DIALECTS: [&str; 4] = ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0"];

/// `tcl registry-dump` — dump the command registry for one dialect (or
/// every Tcl dialect with `--all-dialects`) as canonical JSON.
pub fn run_registry_dump(
    dialect: &str,
    all_dialects: bool,
    output: Option<&Path>,
) -> anyhow::Result<u8> {
    let target = OutputTarget::from_arg(output);
    // `build_default` already carries every Tcl dialect's commands, so the
    // `tcl8.6` registry serves all four Tcl dialects (and `--all-dialects`).
    let json = if all_dialects {
        let registry = registry_for_dialect("tcl8.6");
        command_registry_snapshots(&registry, &TCL_DIALECTS)
    } else {
        let registry = registry_for_dialect(dialect);
        command_registry_snapshot(&registry, dialect)
    };
    write_text_output(&target, &json.dumps_indent2())?;
    Ok(0)
}
