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

use std::sync::{Arc, Barrier};

use tcl_registry::{CommandSpec, core_surface_generation, register_core_surface_specs};

fn same_rows(left: &[&'static CommandSpec], right: &[&'static CommandSpec]) {
    assert_eq!(left.len(), right.len());
    assert!(left.iter().zip(right).all(|(a, b)| std::ptr::eq(*a, *b)));
}

/// This isolated test process reaches the accessor before any bootstrap.
#[test]
fn compiled_core_metadata_is_pure_before_and_after_active_registration() {
    let custom: &'static CommandSpec = Box::leak(Box::new(CommandSpec {
        name: "custom::core_metadata_probe",
        ..CommandSpec::DEFAULT
    }));
    let first_generation = register_core_surface_specs(vec![custom]);
    let rows = tcl_spectcl::core_surfaces::builtin_commands();
    assert_eq!(core_surface_generation(), first_generation);
    assert_eq!(register_core_surface_specs(vec![custom]), first_generation);
    let proc_row = rows.iter().find(|row| row.name == "proc").unwrap();
    assert!(proc_row.procedure_definition.is_some());
    assert!(proc_row.native_compilation.is_none());

    let foreign = tcl_spectcl::loader::evaluate_pack(include_str!(
        "../core-surfaces/jim-own-surface.tclspec"
    ));
    let foreign_proc = foreign
        .commands
        .iter()
        .find(|command| command.spec.name == "proc")
        .unwrap()
        .spec;
    assert!(foreign_proc.procedure_definition.is_none());
    assert!(!std::ptr::eq(foreign_proc, *proc_row));
    assert_eq!(core_surface_generation(), first_generation);

    let canonical_generation = register_core_surface_specs(rows.clone());
    assert_eq!(
        register_core_surface_specs(rows.clone()),
        canonical_generation
    );
    let repeated = tcl_spectcl::core_surfaces::builtin_commands();
    same_rows(&rows, &repeated);
    let replacement_generation = register_core_surface_specs(vec![custom]);
    same_rows(&rows, &tcl_spectcl::core_surfaces::builtin_commands());
    assert_eq!(core_surface_generation(), replacement_generation);
    assert_eq!(
        register_core_surface_specs(vec![custom]),
        replacement_generation
    );

    let next: &'static CommandSpec = Box::leak(Box::new(CommandSpec {
        name: "custom::next_core_metadata_probe",
        ..CommandSpec::DEFAULT
    }));
    let barrier = Arc::new(Barrier::new(2));
    let writer_barrier = Arc::clone(&barrier);
    let writer = std::thread::spawn(move || {
        writer_barrier.wait();
        register_core_surface_specs(vec![next])
    });
    barrier.wait();
    same_rows(&rows, &tcl_spectcl::core_surfaces::builtin_commands());
    let final_generation = writer.join().unwrap();
    assert_eq!(register_core_surface_specs(vec![next]), final_generation);
    same_rows(&rows, &tcl_spectcl::core_surfaces::builtin_commands());
    assert_eq!(core_surface_generation(), final_generation);
}
