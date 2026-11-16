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

#[path = "commands/catalogue_queries.rs"]
mod catalogue_queries;
#[path = "commands/config_reports.rs"]
mod config_reports;
#[path = "commands/dialect_selection.rs"]
mod dialect_selection;
#[path = "commands/document_views.rs"]
mod document_views;
#[path = "commands/fix_all.rs"]
mod fix_all;
#[path = "commands/source_transforms.rs"]
mod source_transforms;
#[path = "commands/xc_translate.rs"]
mod xc_translate;
