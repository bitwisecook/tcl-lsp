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
use tcl_bigip::parser::parse_bigip_conf;

#[test]
fn graph_resolves_references_across_sources() {
    crate::install();
    let pool = "ltm pool /Common/web_pool { }";
    let virtual_server = "ltm virtual /Common/www_vs { pool /Common/web_pool }";
    let sources = [
        ("file:///base.conf".to_owned(), pool.to_owned()),
        ("file:///app.conf".to_owned(), virtual_server.to_owned()),
    ];
    let base = parse_bigip_conf(pool, "Common");
    let app = parse_bigip_conf(virtual_server, "Common");
    let graph = build_bigip_object_graph(
        &sources,
        &[(sources[0].0.clone(), &base), (sources[1].0.clone(), &app)],
        &GraphContext::new(),
    );
    let nodes: Vec<_> = graph
        .nodes_by_uri
        .iter()
        .flat_map(|(_, nodes)| nodes)
        .collect();
    let pool_node = nodes
        .iter()
        .find(|n| n.identifier == "/Common/web_pool")
        .unwrap();
    let virtual_node = nodes
        .iter()
        .find(|n| n.identifier == "/Common/www_vs")
        .unwrap();
    assert!(
        graph
            .edges
            .iter()
            .any(|e| e.source_id == virtual_node.node_id && e.target_id == pool_node.node_id)
    );
}

#[test]
fn gtm_rule_pool_reference_stays_in_gtm_domain() {
    crate::install();
    let source = "ltm pool /Common/app_pool {\n    members {\n        /Common/10.0.0.11:80 { }\n    }\n}\ngtm pool a /Common/app_pool { }\ngtm rule /Common/geo_rule {\n    when DNS_REQUEST {\n        pool /Common/app_pool\n    }\n}\n";
    let config = parse_bigip_conf(source, "Common");
    let uri = "file:///bigip.conf".to_owned();
    let graph = build_bigip_object_graph(
        &[(uri.clone(), source.to_owned())],
        &[(uri, &config)],
        &GraphContext::new(),
    );
    let nodes: Vec<_> = graph
        .nodes_by_uri
        .iter()
        .flat_map(|(_, nodes)| nodes)
        .collect();
    let rule = nodes
        .iter()
        .find(|n| n.identifier == "/Common/geo_rule")
        .unwrap();
    let edges: Vec<_> = graph
        .edges
        .iter()
        .filter(|e| e.source_id == rule.node_id)
        .collect();
    let gtm_pool = nodes
        .iter()
        .find(|n| n.header.starts_with("gtm pool a "))
        .unwrap();
    assert!(edges.iter().any(|e| e.target_id == gtm_pool.node_id));
    assert!(!edges.iter().any(|e| e.via_kind == "ltm_pool"));
}

#[test]
fn firewall_rule_list_references_address_list() {
    crate::install();
    let source = "security firewall address-list /Common/web_servers { }\nsecurity firewall rule-list /Common/rl_web { rules { allow_https { action accept destination { address-lists { /Common/web_servers } } } } }";
    let config = parse_bigip_conf(source, "Common");
    let uri = "file:///bigip.conf".to_owned();
    let graph = build_bigip_object_graph(
        &[(uri.clone(), source.to_owned())],
        &[(uri, &config)],
        &GraphContext::new(),
    );
    let nodes: Vec<_> = graph
        .nodes_by_uri
        .iter()
        .flat_map(|(_, nodes)| nodes)
        .collect();
    let rule = nodes
        .iter()
        .find(|n| n.identifier == "/Common/rl_web")
        .unwrap();
    let addresses = nodes
        .iter()
        .find(|n| n.identifier == "/Common/web_servers")
        .unwrap();
    assert!(
        graph
            .edges
            .iter()
            .any(|e| e.source_id == rule.node_id && e.target_id == addresses.node_id)
    );
}
