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
use tcl_dialect::DialectProfile;
use tcl_registry::command_snapshot::{command_registry_snapshot, command_registry_snapshots};

/// The plain-Tcl-version dialects `--all-dialects` snapshots, in the catalog's
/// stable sorted-name order.
///
/// The predicate is the catalog's own "this profile is a plain Tcl release"
/// fact ([`DialectProfile::const_fold_version`], `Some` only for the versioned
/// Tcl profiles and `None` for every vendor dialect), so a new release added to
/// the catalog is snapshotted without a second list to update — which is how
/// the hand-written array came to be missing `tcl9.1`.
fn tcl_dialects() -> Vec<&'static str> {
    DialectProfile::all()
        .iter()
        .filter(|profile| profile.const_fold_version().is_some())
        .map(|profile| profile.name)
        .collect()
}

/// `tcl registry-dump` — dump the command registry for one dialect (or
/// every Tcl dialect with `--all-dialects`) as canonical JSON.
pub fn run_registry_dump(
    dialect: &DialectProfile,
    all_dialects: bool,
    output: Option<&Path>,
) -> anyhow::Result<u8> {
    let target = OutputTarget::from_arg(output);
    // `build_default` already carries every Tcl dialect's commands, so the
    // `tcl8.6` registry serves every Tcl dialect (and `--all-dialects`).
    let json = if all_dialects {
        // T3: the single-`tcl8.6`-registry shortcut is the payload ledger
        // row T3 retires (P1); the name itself resolves through the seam.
        let registry =
            registry_for_dialect(tcl_cli_support::environment::profile_for_dialect("tcl8.6").name);
        command_registry_snapshots(&registry, &tcl_dialects())
    } else {
        let registry = registry_for_dialect(dialect.name);
        command_registry_snapshot(&registry, dialect.name)
    };
    write_text_output(&target, &json.dumps_indent2())?;
    Ok(0)
}
