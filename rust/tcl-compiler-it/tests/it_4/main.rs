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

//! `tcl-compiler`'s tests that need a crate above it. Each module keeps the
//! path it had in `tcl-compiler`, so a test's name is unchanged.

#[path = "../../../tcl-compiler/tests/support/body_identity.rs"]
mod body_identity;

mod analyser;
mod caller_frame_effects;
mod cfg;
mod dataflow;
mod dispatch_proof_architecture;
mod fix_safety;
mod lsp_external_binding_reports;
mod lsp_values_release;
mod mro_lattice_adversarial;
mod name_lookup_issue_witnesses;
mod optimiser;
mod slot_allocation;
mod source_role_boundaries;
mod span_char_boundaries;
mod taint_depth;
mod value_transfer_witnesses;
mod wasm_api_architecture;

/// Installs the shipped command surfaces; every test calls it before its
/// first registry read.
fn install_surfaces() {
    tcl_registry::install_shipped().expect("the shipped surface set installs");
}
