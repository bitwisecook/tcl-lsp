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

//! Consolidated integration suite for the query language.
//!
//! One binary for all fifteen suites. Every `tests/*.rs` is otherwise its own
//! binary statically linking the whole dependency graph, and six of these files
//! held a single test each — six full link steps to run six tests. The suites
//! are independent and share no state, so nothing is lost by sharing a process
//! and the parallel pool gets wider rather than narrower.
//!
//! The files live under `tests/integration/` — a non-target subdirectory, so
//! cargo does not re-discover them as binaries of their own — and `#[path]`
//! points at them, the arrangement `tcl-lsp-server/tests/e2e.rs` established.
//! Test names are module-qualified (`renderers::…`, `output::…`), so a filter
//! that used to name a binary now names a module and reads the same.

#[path = "integration/edit_plan.rs"]
mod edit_plan;
#[path = "integration/eval.rs"]
mod eval;
#[path = "integration/files.rs"]
mod files;
#[path = "integration/front_end.rs"]
mod front_end;
#[path = "integration/graph.rs"]
mod graph;
#[path = "integration/hardening.rs"]
mod hardening;
#[path = "integration/math.rs"]
mod math;
#[path = "integration/net.rs"]
mod net;
#[path = "integration/output.rs"]
mod output;
#[path = "integration/probes.rs"]
mod probes;
#[path = "integration/projection.rs"]
mod projection;
#[path = "integration/regex_str.rs"]
mod regex_str;
#[path = "integration/renderers.rs"]
mod renderers;
#[path = "integration/time.rs"]
mod time;
#[path = "integration/value2.rs"]
mod value2;
