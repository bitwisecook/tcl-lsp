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

//! Consolidated integration suite for the BIG-IP configuration model.
//!
//! One binary for all nine suites. Every `tests/*.rs` is otherwise its own
//! binary statically linking the whole dependency graph, and five of these
//! files held a single test each — five full link steps to run five tests. The
//! suites are independent and share no state, so nothing is lost by sharing a
//! process and the parallel pool gets wider rather than narrower.
//!
//! The files live under `tests/integration/` — a non-target subdirectory, so
//! cargo does not re-discover them as binaries of their own — and `#[path]`
//! points at them, the arrangement `tcl-lsp-server/tests/e2e.rs` established.
//! Test names are module-qualified (`graph_edges::…`, `link_extract::…`), so a
//! filter that used to name a binary now names a module and reads the same.

#[path = "integration/cleanup.rs"]
mod cleanup;
#[path = "integration/conf_wrapped_irules.rs"]
mod conf_wrapped_irules;
#[path = "integration/graph_edges.rs"]
mod graph_edges;
#[path = "integration/graph_export.rs"]
mod graph_export;
#[path = "integration/graph_nodes.rs"]
mod graph_nodes;
#[path = "integration/graph_pilot.rs"]
mod graph_pilot;
#[path = "integration/irule_context.rs"]
mod irule_context;
#[path = "integration/link_extract.rs"]
mod link_extract;
#[path = "integration/resolve_kind.rs"]
mod resolve_kind;
#[path = "integration/stats.rs"]
mod stats;
