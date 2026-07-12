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

//! F5 iApp APL (Application Presentation Language) — structural parser
//! and object model.

pub mod canonical;
pub mod iapp_diagnostics;
pub mod iapp_vars;
pub mod model;
pub mod parser;
pub mod tokens;

pub use canonical::model_to_canonical;
pub use iapp_diagnostics::{validate_iapp_implementation, validate_iapp_presentation};
pub use iapp_vars::{IappVarRef, extract_iapp_var_refs};
pub use model::{
    AplField, AplInclude, AplModel, AplSection, AplTable, apl_name_to_tcl_var, tcl_var_to_apl_name,
};
pub use parser::parse_apl;
pub use tokens::{AplToken, AplTokenKind, embedded_tcl_regions, tokenise_apl};
