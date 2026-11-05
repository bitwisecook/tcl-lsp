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

//! `tcl-vm` integration tests assigned to CI shard 3, one module per
//! former test binary, linked once instead of 22 times
//! (`scripts/dev/rust-test-binary-shards.tsv`). Add a test file here as a
//! module; a new top-level `tests/*.rs` is a binary of its own.

mod c_error_paths_e2e;
mod cmd_control_e2e;
mod cmd_math_expr_e2e;
mod cmd_oo_e2e;
mod coro_error_trace_e2e;
mod cross_version_info_surface_e2e;
mod cross_version_lset_e2e;
mod embed_api_e2e;
mod error_info_text_e2e;
mod info_worker_e2e;
mod literal_index_encoding_e2e;
mod mathfunc_derivation_e2e;
mod namespace_surface_e2e;
mod release_number_semantics_e2e;
mod release_running_bytecode_e2e;
mod rename_return_source_e2e;
mod return_code_boundary_e2e;
mod run_script;
mod tclsh_parity_e2e;
mod tricky_resolution_e2e;
mod uplevel_coro_e2e;
mod word_value_resolution_e2e;
