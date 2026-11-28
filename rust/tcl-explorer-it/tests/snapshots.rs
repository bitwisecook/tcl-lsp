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

use tcl_explorer::{FunctionSnapshotKind, run_pipeline};

mod support;

#[test]
fn snapshots_retain_duplicate_procedures_and_named_top_separately() {
    support::install_surfaces();
    let result = run_pipeline(
        "proc p {} {return FIRST}; proc p {} {return LAST}; proc ::top {} {return NAMED}",
        "tcl8.6",
    );
    let snapshots = result.snapshots();
    let procedure_rows: Vec<_> = snapshots
        .iter()
        .filter(|row| row.kind == FunctionSnapshotKind::Procedure)
        .collect();
    assert_eq!(procedure_rows.len(), 3);
    assert_eq!(
        procedure_rows
            .iter()
            .filter(|row| row.name == "::p")
            .count(),
        2
    );
    assert_eq!(
        procedure_rows
            .iter()
            .filter(|row| row.name == "::top")
            .count(),
        1
    );
    assert!(snapshots[0].unit.body_id.is_none());
    let ids: std::collections::HashSet<_> = procedure_rows
        .iter()
        .map(|row| row.unit.body_id.expect("retained procedure body"))
        .collect();
    assert_eq!(ids.len(), 3);
}
