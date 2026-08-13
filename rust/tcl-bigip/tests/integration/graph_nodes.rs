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

//! Differential tests for the BIG-IP graph **node** extraction via
//! `build_bigip_object_graph`.
//!
//! Builds nodes for a representative `bigip.conf` and asserts each node's
//! identity, resolved kind, byte offsets, and source range match a golden
//! captured from the graph builder. Self-contained — no external fixture
//! generator at test time. (Edges are a separate, later increment.)

use tcl_bigip::graph::{GraphContext, build_objects_for_source};

#[test]
fn graph_nodes() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let source = std::fs::read_to_string(format!("{dir}/bigip.conf")).expect("read config");
    let golden = std::fs::read_to_string(format!("{dir}/graph_nodes.golden.tsv")).expect("golden");

    let ctx = GraphContext::new();
    let mut nodes = build_objects_for_source("test://config", &source, &ctx);
    nodes.sort_by_key(|n| n.start_offset);

    let expected: Vec<&str> = golden.lines().collect();
    assert_eq!(
        nodes.len(),
        expected.len(),
        "node count differs: got {}, want {}",
        nodes.len(),
        expected.len()
    );

    for (node, want) in nodes.iter().zip(expected) {
        let f: Vec<&str> = want.split('\t').collect();
        let got = [
            node.module.clone(),
            node.object_type.clone(),
            node.identifier.clone(),
            node.kind.unwrap_or("-").to_owned(),
            node.start_offset.to_string(),
            node.end_offset.to_string(),
            node.header_start_offset.to_string(),
            node.range.start.line.to_string(),
            node.range.start.character.to_string(),
            node.range.end.line.to_string(),
            node.range.end.character.to_string(),
            node.node_id.clone(),
        ];
        assert_eq!(
            got.as_slice(),
            f.as_slice(),
            "node mismatch for {:?}",
            node.node_id
        );
    }
}
