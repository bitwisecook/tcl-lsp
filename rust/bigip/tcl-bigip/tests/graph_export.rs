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

//! Byte-stable fixtures for `export_graph` (DOT / JSON / Mermaid), checked
//! against captured expected output on a fixture (so the serialised text
//! matches exactly).

use tcl_bigip::graph::{GraphContext, build_bigip_object_graph, export_graph};
use tcl_bigip::parser::parse_bigip_conf;

#[test]
fn graph_export() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let source = std::fs::read_to_string(format!("{dir}/graph_export.conf")).expect("read config");
    let cfg = parse_bigip_conf(&source, "Common");
    let uri = "test://config".to_owned();
    let sources = [(uri.clone(), source)];
    let configs = [(uri, &cfg)];

    let ctx = GraphContext::new();
    let graph = build_bigip_object_graph(&sources, &configs, &ctx);

    for fmt in ["dot", "json", "mermaid"] {
        let want = std::fs::read_to_string(format!("{dir}/graph_export.{fmt}.golden"))
            .expect("read golden");
        let export = export_graph(&graph, fmt, &[], false, None).expect("export");
        assert_eq!(
            export.text, want,
            "{fmt} export differs from the expected fixture"
        );
        assert_eq!(export.node_count, 5, "{fmt} node_count");
        assert_eq!(export.edge_count, 8, "{fmt} edge_count");
    }
}

#[test]
fn graph_export_seeded_subgraph() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let source = std::fs::read_to_string(format!("{dir}/graph_export.conf")).expect("read config");
    let cfg = parse_bigip_conf(&source, "Common");
    let uri = "test://config".to_owned();
    let sources = [(uri.clone(), source)];
    let configs = [(uri, &cfg)];

    let ctx = GraphContext::new();
    let graph = build_bigip_object_graph(&sources, &configs, &ctx);

    // BFS from the virtual, depth 1 → the virtual and its pool, in visit order.
    let seeds = ["/Common/web_vs".to_owned()];
    let export = export_graph(&graph, "json", &seeds, false, Some(1)).expect("export");
    let want = std::fs::read_to_string(format!("{dir}/graph_export_seed.json.golden"))
        .expect("read golden");
    assert_eq!(
        export.text, want,
        "seeded subgraph export differs from the expected fixture"
    );
    assert_eq!(export.node_count, 2);
    assert_eq!(export.edge_count, 2);
}

#[test]
fn graph_export_rejects_unknown_format() {
    let ctx = GraphContext::new();
    let graph = build_bigip_object_graph(&[], &[], &ctx);
    assert!(export_graph(&graph, "svg", &[], false, None).is_err());
}
