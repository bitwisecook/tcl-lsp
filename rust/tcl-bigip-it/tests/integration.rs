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

fn install() {
    tcl_registry::install_shipped().expect("install shipped surfaces");
}

#[path = "suite/graph.rs"]
mod graph;
#[path = "suite/hud_chain.rs"]
mod hud_chain;
#[path = "suite/irule_context.rs"]
mod irule_context;
#[path = "suite/lint.rs"]
mod lint;

#[path = "suite/iapp_diagnostics.rs"]
mod iapp_diagnostics;
#[path = "suite/iapp_vars.rs"]
mod iapp_vars;

#[path = "suite/links.rs"]
mod links;
#[path = "suite/validator.rs"]
mod validator;

#[path = "suite/integration_cleanup.rs"]
mod integration_cleanup;

#[path = "suite/integration_graph_edges.rs"]
mod integration_graph_edges;

#[path = "suite/integration_graph_export.rs"]
mod integration_graph_export;

#[path = "suite/integration_graph_nodes.rs"]
mod integration_graph_nodes;

#[path = "suite/integration_graph_pilot.rs"]
mod integration_graph_pilot;

#[path = "suite/integration_irule_context.rs"]
mod integration_irule_context;

#[path = "suite/integration_stats.rs"]
mod integration_stats;

#[path = "suite/refs.rs"]
mod refs;
#[path = "suite/wireshark.rs"]
mod wireshark;

#[path = "suite/graph_references.rs"]
mod graph_references;
