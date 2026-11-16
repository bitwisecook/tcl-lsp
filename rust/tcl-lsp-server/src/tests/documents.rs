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

//! Open documents: path admission, the document map, edit order, content changes and reads.

use super::*;

#[path = "documents/content_changes.rs"]
mod content_changes;
#[path = "documents/edit_order.rs"]
mod edit_order;
#[path = "documents/path_admission.rs"]
mod path_admission;
#[path = "documents/per_uri_caches.rs"]
mod per_uri_caches;
#[path = "documents/reads.rs"]
mod reads;
#[path = "documents/stall_report.rs"]
mod stall_report;
#[path = "documents/store.rs"]
mod store;
#[path = "documents/unpark_watchdog.rs"]
mod unpark_watchdog;
