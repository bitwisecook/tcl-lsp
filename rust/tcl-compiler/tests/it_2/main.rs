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

//! `tcl-compiler` integration tests assigned to CI shard 2, one module per
//! former test binary, linked once instead of 14 times
//! (`scripts/dev/rust-test-binary-shards.tsv`). Add a test file here as a
//! module; a new top-level `tests/*.rs` is a binary of its own.

#[path = "../support/body_identity.rs"]
mod body_identity;

mod alias_scoping;
mod core_analyses;
mod dialect_threading;
mod differential_word_expr;
mod entry_contract_abstention;
mod intervals;
mod irules_event_context;
mod lsp_external_validation;
mod phi_undef_memo;
mod regex_source_oracle;
mod static_name_consumers;
mod value_transfer_witnesses;
mod var_escape_cfg;
mod w210_empty_loop_source;
