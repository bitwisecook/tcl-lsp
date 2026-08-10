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

//! Flow-sensitive per-SSA-version var-escape analysis.
//!
//! Components:
//!
//! * [`state`]: `CfgEscapeResult` + `CfgState`.
//! * `collect_known_names_from_cfg`.
//! * per-call handlers (cfg variants).
//! * barrier handlers + `escape_every_name_touched_tree`.
//! * `handle_call` dispatcher + value/expr scans.
//! * `handle_statement` + `walk_block` + `block_order` +
//!   `analyse_cfg_function` entry point.

pub mod handlers;
pub mod known_names;
pub mod state;
pub mod walker;

pub use known_names::collect_known_names_from_cfg;
pub use state::{CfgEscapeResult, CfgState};
pub use walker::{analyse_cfg_function, analyse_cfg_function_with_registry};
