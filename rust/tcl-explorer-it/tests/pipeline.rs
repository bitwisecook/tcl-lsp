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

mod support;

use tcl_explorer::run_pipeline;

#[test]
fn run_pipeline_top_level_only() {
    support::install_surfaces();
    let result = run_pipeline("set x 1\nset y 2", "tcl8.6");
    let snapshots = result.snapshots();
    // Only the top-level function exists for a script with no procs.
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].name, "::top");
    assert!(result.total_blocks() >= 1);
}

#[test]
fn run_pipeline_collects_procedures_in_order() {
    support::install_surfaces();
    let src = "proc beta {} { return 2 }\nproc alpha {} { return 1 }\nalpha";
    let result = run_pipeline(src, "tcl8.6");
    let names: Vec<&str> = result.snapshots().iter().map(|s| s.name).collect();
    // Top-level first, then procedures sorted by qualified name.
    assert_eq!(names[0], "::top");
    let procs = &names[1..];
    assert!(procs.contains(&"::alpha"));
    assert!(procs.contains(&"::beta"));
    let mut sorted = procs.to_vec();
    sorted.sort_unstable();
    assert_eq!(procs, sorted.as_slice());
}

#[test]
fn unparseable_dialect_does_not_panic() {
    support::install_surfaces();
    let result = run_pipeline("set x 1", "not-a-real-dialect");
    assert_eq!(result.dialect, "not-a-real-dialect");
    assert!(!result.snapshots().is_empty());
}
