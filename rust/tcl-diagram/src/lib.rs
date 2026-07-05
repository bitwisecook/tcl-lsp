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

//! iRule (Tcl) structural analysis for reports, the CLI, and the MCP.
//!
//! [`diagram_data`] projects a parsed script's [`tcl_compiler`] IR into a
//! frontend-agnostic `{events, procedures}` structure (decision / action /
//! assign / return / switch / loop nodes). [`irule_flowchart_mermaid`] turns
//! that into a deterministic Mermaid `flowchart` — an offline, no-LLM diagram
//! for the BIG-IP report's iRule view, the CLI, and the MCP.
//!
//! [`attach_reach`] / [`irule_attach_patterns`] reconstruct the object-name
//! patterns (prefix / contained / suffix) an iRule could build for a dynamic
//! `pool` / `node` / `snatpool` attachment, so the report's orphan analysis can
//! filter candidate objects down to only the ones a rule could actually reach.
//!
//! This lives in its own crate (not `tcl-lsp-core`): it depends only on the
//! compiler + lexer + registry, so any tool — including the wasm report
//! generator — can use it without pulling in the LSP feature layer.

mod attach;
mod data;
mod mermaid;

pub use attach::{
    AttachPattern, AttachReach, attach_reach, irule_attach_patterns, proc_call_refs,
};
pub use data::diagram_data;
pub use mermaid::irule_flowchart_mermaid;
