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

//! `tcl-compiler` integration tests assigned to CI shard 5, one module per
//! former test binary, linked once instead of 13 times
//! (`scripts/dev/rust-test-binary-shards.tsv`). Add a test file here as a
//! module; a new top-level `tests/*.rs` is a binary of its own.

#[path = "../support/body_identity.rs"]
mod body_identity;
#[path = "../support/function_identity.rs"]
mod function_identity;

mod cfg;
mod checks;
mod irules_spec_examples_self_consistent;
mod lsp_external_taint_validation;
mod optimiser_coverage;
mod pipeline_coverage;
mod recovery_positions;
mod slot_dispatch;
mod tcl_namespace_probe;
mod unused_var_tricky_features;
mod var_escape_cfg_catch;
mod var_escape_cfg_tree_arms;
mod var_escape_typeinfer;
