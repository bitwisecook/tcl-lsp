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

//! var-escape analysis.
//!
//! Per-proc static analysis tagging each Tcl variable as
//! [`types::EscapeTag::Local`] (stays in a WASM local) or
//! [`types::EscapeTag::Frame`] (must live in the runtime frame
//! so the interpreter or an `upvar` alias can see it by name).
//!
//! Components:
//!
//! * [`types`]: vocabulary + summary types.
//! * [`cfg_propagation`]: intra-procedural rule audit and
//!   flow-sensitive SSA-version propagation.
//! * [`info_subcommands`]: which `info` subcommands
//!   force pessimism.
//! * [`interprocedural`]: thread escapes across call
//!   edges.
//! * [`slot_resolution`]: compile-time slot indices for
//!   proc-locals.

pub mod api;
pub mod cfg_propagation;
pub mod handlers;
pub mod helpers;
pub mod info_subcommands;
pub mod interprocedural;
pub mod known_names;
pub mod slot_resolution;
pub mod state;
pub mod types;
pub mod walker;

pub use api::{
    TOP_LEVEL_QNAME, analyse_var_escape, analyse_var_escape_cu, analyse_var_escape_with_registry,
    cfg_result_to_summary,
};
pub use cfg_propagation::{analyse_cfg_function, analyse_cfg_function_with_registry};
pub use interprocedural::solve_interprocedural_escape;
pub use slot_resolution::{LOCALS_ARRAY_CAP, assign_local_slots, populate_local_slots};
pub use walker::{analyse_script, analyse_script_with_registry};

pub use info_subcommands::{
    FRAME_INSPECTING_SUBCOMMANDS, INTERPRETER_GLOBAL_SUBCOMMANDS,
    is_frame_inspecting_info_subcommand, is_safe_info_subcommand,
};
pub use types::{
    Barrier, BarrierKind, EscapeReason, EscapeReasonKind, EscapeTag, ProcEscapeSummary, join,
};
