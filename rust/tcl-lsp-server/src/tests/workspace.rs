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

//! Folder scans, disk reindexing, watched files, libraries, rehoming and project-wide facts.

use super::*;

#[path = "workspace/class_factories.rs"]
mod class_factories;
#[path = "workspace/disk_reindex.rs"]
mod disk_reindex;
#[path = "workspace/folder_scan.rs"]
mod folder_scan;
#[path = "workspace/folders.rs"]
mod folders;
#[path = "workspace/library_indexing.rs"]
mod library_indexing;
#[path = "workspace/package_database.rs"]
mod package_database;
#[path = "workspace/project_inputs.rs"]
mod project_inputs;
#[path = "workspace/scan_batches.rs"]
mod scan_batches;
#[path = "workspace/source_files.rs"]
mod source_files;
#[path = "workspace/source_graph.rs"]
mod source_graph;
#[path = "workspace/source_rehoming.rs"]
mod source_rehoming;
#[path = "workspace/warm.rs"]
mod warm;
#[path = "workspace/watched_files.rs"]
mod watched_files;
#[path = "workspace/watched_open_paths.rs"]
mod watched_open_paths;
