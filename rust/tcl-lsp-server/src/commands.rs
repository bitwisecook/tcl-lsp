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

//! The `workspace/executeCommand` handlers.

use super::*;

mod bigip_objects;
mod bigip_rules;
mod catalogue_queries;
mod config_reports;
mod dialect_selection;
mod document_views;
mod fix_all;
mod minimize_diagnostic;
mod source_transforms;
mod xc_translate;

#[cfg(test)]
pub(super) use catalogue_queries::{
    TclInstallationSelectionUnavailable, tcl_installation_selection_paths,
};
#[cfg(test)]
pub(super) use config_reports::{config_ini_path, non_ascii_mode_str};
pub(super) use document_views::sha256_digest_hex;
