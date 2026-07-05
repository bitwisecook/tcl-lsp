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

//! Differential tests for the **complete** BIG-IP graph edge walk on a
//! realistic `bigip.conf` — the pilot value-spec dispatch, the legacy
//! token-scan fallback, AND the iRule body walker.
//! Companion to `graph_pilot.rs`, which exercises the compound pilot specs on a
//! synthetic fixture; this pins the full walk on real config. The graph
//! reproduces the expected edge set exactly. Self-contained — no external
//! fixture generator at test time.

use tcl_bigip::graph::{GraphContext, build_bigip_object_graph};
use tcl_bigip::parser::parse_bigip_conf;

#[test]
fn graph_edges() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let source = std::fs::read_to_string(format!("{dir}/bigip.conf")).expect("read config");
    let golden = std::fs::read_to_string(format!("{dir}/graph_edges.golden.tsv")).expect("golden");

    let cfg = parse_bigip_conf(&source, "Common");
    let uri = "test://config".to_owned();
    let sources = [(uri.clone(), source)];
    let configs = [(uri, &cfg)];

    let ctx = GraphContext::new();
    let graph = build_bigip_object_graph(&sources, &configs, &ctx);

    let got: Vec<String> = graph
        .edges
        .iter()
        .map(|e| {
            format!(
                "{}\t{}\t{}\t{}",
                e.source_id, e.target_id, e.via_property, e.via_kind
            )
        })
        .collect();
    let want: Vec<&str> = golden.lines().collect();

    // Exact ordered comparison against the expected edge set.
    assert_eq!(got, want, "graph edges differ from the expected set");
}
