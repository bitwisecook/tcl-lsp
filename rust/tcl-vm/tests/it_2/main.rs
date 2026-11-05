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

//! `tcl-vm` integration tests assigned to CI shard 2, one module per
//! former test binary, linked once instead of 22 times
//! (`scripts/dev/rust-test-binary-shards.tsv`). Add a test file here as a
//! module; a new top-level `tests/*.rs` is a binary of its own.

mod cmd_collections_e2e;
mod cmd_coro_e2e;
mod cmd_info_prefix_e2e;
mod cmd_string_e2e;
mod cross_interp_reentry_e2e;
mod cross_version_expr_surface_e2e;
mod cross_version_numerals_e2e;
mod dictionary_worker_receipt;
mod encoding_e2e;
mod env_host_sync_e2e;
mod expanded_invoke_e2e;
mod expr_surface_e2e;
mod integer_argument_reads_e2e;
mod namespace_colon_runs_e2e;
mod numeric_tower_e2e;
mod opcode_dispatch_coverage;
mod package_coro_e2e;
mod regexp_e2e;
mod string_map_worker_receipt;
mod switch_integer_e2e;
mod wasm_coro_e2e;
mod word_parts_owner_e2e;
