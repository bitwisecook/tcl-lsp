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

//! `tcl-vm` integration tests assigned to CI shard 5, one module per
//! former test binary, linked once instead of 15 times
//! (`scripts/dev/rust-test-binary-shards.tsv`). Add a test file here as a
//! module; a new top-level `tests/*.rs` is a binary of its own.

mod builtins_e2e;
mod clock_compilation_e2e;
mod cmd_event_e2e;
mod command_mutation_deopt_e2e;
mod cross_version_escapes_e2e;
mod dict_canonicalisation_parity;
mod library_bootstrap_presence;
mod native_byte_literals;
mod native_variable_declaration;
mod opcode_c_parity;
mod opcode_catch_parity;
mod safe_interp_e2e;
mod string_map_identity_receipt;
mod tailcall_uplevel_e2e;
mod variable_container_models;
