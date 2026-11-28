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

//! The modules share one test binary.

#[path = "integration/edit_plan.rs"]
mod edit_plan;
#[path = "integration/eval.rs"]
mod eval;
#[path = "integration/files.rs"]
mod files;
#[path = "integration/front_end.rs"]
mod front_end;
#[path = "integration/hardening.rs"]
mod hardening;
#[path = "integration/math.rs"]
mod math;
#[path = "integration/modules.rs"]
mod modules;
#[path = "integration/net.rs"]
mod net;
#[path = "integration/output.rs"]
mod output;
#[path = "integration/probes.rs"]
mod probes;
#[path = "integration/regex_str.rs"]
mod regex_str;
#[path = "integration/renderers.rs"]
mod renderers;
#[path = "integration/time.rs"]
mod time;
#[path = "integration/value2.rs"]
mod value2;
