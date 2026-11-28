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

use tcl_bigip::graph::{GraphContext, build_bigip_object_graph};
use tcl_bigip::parser::driver::parse_bigip_conf;

#[test]
fn build_bigip_object_graph_links_virtual_to_its_references() {
    crate::install();
    // A virtual that references a pool (single ObjectRef, legacy path) and
    // an iRule (a `rules { … }` list, pilot path) — both defined.
    let src = concat!(
        "ltm pool /Common/web {\n",
        "  members {\n    /Common/n:80 { address 10.0.0.1 }\n  }\n}\n",
        "ltm rule /Common/r {\n  when HTTP_REQUEST { }\n}\n",
        "ltm virtual /Common/vs {\n  pool /Common/web\n  rules {\n    /Common/r\n  }\n}\n",
    );
    let cfg = parse_bigip_conf(src, "Common");
    let ctx = GraphContext::new();
    let uri = "file:///c.conf".to_string();
    let graph = build_bigip_object_graph(
        &[(uri.clone(), src.to_string())],
        &[(uri.clone(), &cfg)],
        &ctx,
    );

    // One input source → one node bucket; nodes for the pool, rule, virtual.
    assert_eq!(graph.nodes_by_uri.len(), 1);
    let nodes = &graph.nodes_by_uri[0].1;
    let find = |object_type: &str| {
        nodes
            .iter()
            .find(|n| n.object_type == object_type)
            .unwrap_or_else(|| panic!("missing {object_type} node"))
    };
    let vs = find("virtual");
    let pool = find("pool");
    let rule = find("rule");

    let edge = |from: &str, to: &str| {
        graph
            .edges
            .iter()
            .any(|e| e.source_id == from && e.target_id == to)
    };
    // Forward edges from the virtual to both referenced objects.
    assert!(edge(&vs.node_id, &pool.node_id), "virtual → pool edge");
    assert!(edge(&vs.node_id, &rule.node_id), "virtual → rule edge");
    // The rules-list edge records the property it originated from.
    assert!(
        graph.edges.iter().any(|e| e.source_id == vs.node_id
            && e.target_id == rule.node_id
            && e.via_property.contains("rules")),
        "rule edge is attributed to the `rules` property"
    );
}

#[test]
fn address_fixture_links_literal_and_named_service_references() {
    crate::install();
    let source = include_str!(
        "../../../../docs/work/name-handling/handover/oo-dispatch-probe/bigip-addresses/addresses.conf"
    );
    let config = parse_bigip_conf(source, "Common");
    let uri = "file:///addresses.conf".to_owned();
    let graph = build_bigip_object_graph(
        &[(uri.clone(), source.to_owned())],
        &[(uri, &config)],
        &GraphContext::new(),
    );
    let node_id = |identifier: &str| {
        graph.nodes_by_uri[0]
            .1
            .iter()
            .find(|node| node.identifier == identifier)
            .unwrap_or_else(|| panic!("missing node {identifier}"))
            .node_id
            .as_str()
    };
    for (from, to) in [
        ("/Common/p_v4", "/Common/10.0.0.1"),
        ("/Common/p_v4", "/Common/10.0.0.1%2"),
        ("/Common/app.app/app_pool", "/Common/app.app/app_node"),
        ("/Common/vs_v4", "/Common/10.0.0.10"),
        ("/Common/vs_any", "/Common/0.0.0.0"),
        ("/Common/p_v6", "/Common/2001:db8::1"),
        ("/Common/p_v6", "/Common/2001:db8::1%2"),
        ("/Common/p_http", "/Common/10.0.0.1"),
        ("/Common/vs_v6", "/Common/2001:db8::10"),
        ("/Common/vs_rd", "/Common/10.0.0.10%2"),
        ("/Common/vs_any6", "/Common/any6"),
        ("/T1/p_rel", "/T1/node_t1"),
    ] {
        assert!(
            graph
                .edges
                .iter()
                .any(|edge| edge.source_id == node_id(from) && edge.target_id == node_id(to)),
            "missing {from} to {to}"
        );
    }
}

#[test]
fn sources_without_a_config_are_skipped() {
    crate::install();
    let src = "ltm pool /Common/p {\n}\n";
    let cfg = parse_bigip_conf(src, "Common");
    let ctx = GraphContext::new();
    // Two sources, but only one has a matching config entry.
    let graph = build_bigip_object_graph(
        &[
            ("file:///has.conf".to_string(), src.to_string()),
            ("file:///orphan.conf".to_string(), src.to_string()),
        ],
        &[("file:///has.conf".to_string(), &cfg)],
        &ctx,
    );
    assert_eq!(graph.nodes_by_uri.len(), 1);
    assert_eq!(graph.nodes_by_uri[0].0, "file:///has.conf");
}
