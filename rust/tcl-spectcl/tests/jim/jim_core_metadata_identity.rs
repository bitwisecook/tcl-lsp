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

//! Canonical compiled-core descriptor identity and metadata accessor purity.

use tcl_registry::CommandSpec;

fn same_rows(left: &[&'static CommandSpec], right: &[&'static CommandSpec]) {
    assert_eq!(left.len(), right.len());
    assert!(left.iter().zip(right).all(|(a, b)| std::ptr::eq(*a, *b)));
}

/// The compiled-in rows carry their canonical procedure descriptor, every
/// read hands out the same rows, and the installed Jim surface is those rows.
#[test]
fn compiled_core_metadata_is_canonical_and_the_installed_surface_shares_it() {
    crate::install();
    let rows = tcl_spectcl::core_surfaces::builtin_commands();
    let proc_row = rows.iter().find(|row| row.name == "proc").unwrap();
    assert!(proc_row.procedure_definition.is_some());
    assert!(proc_row.native_compilation.is_none());

    let foreign = tcl_spectcl::loader::evaluate_pack(include_str!(
        "../../core-surfaces/jim-own-surface.tclspec"
    ));
    let foreign_proc = foreign
        .commands
        .iter()
        .find(|command| command.spec.name == "proc")
        .unwrap()
        .spec;
    assert!(foreign_proc.procedure_definition.is_none());
    assert!(!std::ptr::eq(foreign_proc, *proc_row));

    let installed = crate::install().specs_of("jim");
    same_rows(&rows, installed);
    let reader = std::thread::spawn(tcl_spectcl::core_surfaces::builtin_commands);
    same_rows(&rows, &reader.join().unwrap());
    same_rows(&rows, &tcl_spectcl::core_surfaces::builtin_commands());
}
