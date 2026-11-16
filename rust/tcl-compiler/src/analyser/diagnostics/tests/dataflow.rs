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

//! The CFG and SSA checks: dead stores, unused bindings, read-before-set, existence folds, constant conditions and value domains.

use super::*;

#[path = "dataflow/constant_conditions.rs"]
mod constant_conditions;
#[path = "dataflow/dead_stores.rs"]
mod dead_stores;
#[path = "dataflow/existence_folds.rs"]
mod existence_folds;
#[path = "dataflow/irules_cells.rs"]
mod irules_cells;
#[path = "dataflow/method_bodies.rs"]
mod method_bodies;
#[path = "dataflow/phi_undefined_reads.rs"]
mod phi_undefined_reads;
#[path = "dataflow/read_before_set.rs"]
mod read_before_set;
#[path = "dataflow/unused_bindings.rs"]
mod unused_bindings;
#[path = "dataflow/value_domains.rs"]
mod value_domains;
#[path = "dataflow/variable_lifecycle.rs"]
mod variable_lifecycle;

// info exists / array exists

fn codes_for(src: &str) -> Vec<String> {
    let mut a = Analyser::new();
    a.emit_document_cfg_ssa_diagnostics_for_test(src);
    a.result
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect()
}

/// [`codes_for`] under one release rather than the lenient span, which
/// includes 8.4 (no `TclOO`) and so proves nothing about a method body.
fn codes_for_in(src: &str, dialect: &str) -> Vec<String> {
    let mut a = Analyser::new();
    a.profile = tcl_dialect::DialectProfile::find(dialect).unwrap();
    a.emit_document_cfg_ssa_diagnostics_for_test(src);
    a.result
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect()
}

/// Helper: W210 codes for a snippet.
fn w210_codes(src: &str) -> Vec<String> {
    w210_codes_in_dialect(src, "tcl")
}

fn w210_codes_in_dialect(src: &str, dialect: &str) -> Vec<String> {
    let mut a = Analyser::new();
    a.analyse(src, dialect)
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W210)
        .map(|d| d.message.clone())
        .collect()
}

/// The lifecycle diagnostics a program draws, as `(code, message)`.
fn lifecycle_findings(src: &str) -> Vec<(DiagCode, String)> {
    Analyser::new()
        .analyse(src, "tcl8.6")
        .diagnostics
        .into_iter()
        .filter(|d| matches!(d.code, DiagCode::W210 | DiagCode::W213))
        .map(|d| (d.code, d.message))
        .collect()
}
