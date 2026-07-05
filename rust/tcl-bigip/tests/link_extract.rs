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

//! BIG-IP linked object graph extraction tests.
//!
//! These exercise `extract_linked_bigip_objects` — a BFS wrapper that, given
//! one or more cursor offsets, resolves the *root* object containing each
//! offset and walks the linked-object graph **bidirectionally** (incoming +
//! outgoing reference edges) up to `max_depth`, annotating each reached node
//! with its BFS `depth` and a `sourceOrigin` tag derived from the config URI's
//! basename.
//!
//! The crate exposes the underlying node/edge graph
//! (`tcl_bigip::graph::build_bigip_object_graph`, returning `ObjectNode` /
//! `ObjectEdge`) but NOT the `extract_linked_bigip_objects` BFS wrapper. The
//! BFS, root-at-offset resolution, and source-origin tagging are pure graph
//! traversal over the public `ObjectNode` / `ObjectEdge` fields, so they are
//! reproduced here in the test harness — no `src/` changes were needed. The
//! structural assertions (which nodes are reached, which edges connect them)
//! are made directly against the graph, matching the `result["nodes"]` /
//! `result["edges"]` shape.
//!
//! Behaviours covered, one `#[test]` each:
//!  * bidirectional BFS reaches incoming (vs→pool) and outgoing (pool↑) links,
//!    with monotone depth;
//!  * cross-`bigip.conf`-file reference resolution;
//!  * `gtm rule`→`gtm pool` stays in the gtm domain, no `ltm_pool` edge;
//!  * multi-cursor seeds merge into one graph with multiple roots;
//!  * two cursors in one object dedupe to a single root;
//!  * multi-cursor across files;
//!  * `sourceOrigin` base/synced/script tags;
//!  * firewall rule-list → address-list ref is graph-visible via the registry
//!    pilot path.

use std::collections::{HashMap, HashSet, VecDeque};

use tcl_bigip::graph::{
    GraphContext, ObjectEdge, ObjectGraph, ObjectNode, build_bigip_object_graph,
};
use tcl_bigip::parser::parse_bigip_conf;

// ---------------------------------------------------------------------------
// Harness: a faithful reimplementation of `extract_linked_bigip_objects`'s BFS
// over the public graph, returning enough of its result shape for assertions.
// ---------------------------------------------------------------------------

/// One reached node, mirroring an entry of `result["nodes"]`.
struct ReachedNode<'a> {
    node: &'a ObjectNode,
    depth: usize,
}

/// A seed root, mirroring an entry of `result["roots"]`.
#[derive(Clone)]
struct Root {
    id: String,
    uri: String,
    /// Mirrors `roots[].header` / `rootHeader`; carried for result-shape
    /// fidelity though no assertion reads it.
    #[allow(dead_code)]
    header: String,
}

/// Mirror of the relevant subset of `extract_linked_bigip_objects`'s dict.
struct LinkResult<'a> {
    /// Backward-compat primary root id (== `roots[0].id`).
    root: String,
    /// Deduplicated seed roots, in seed order.
    roots: Vec<Root>,
    /// Reached nodes, sorted by `(depth, id)`.
    nodes: Vec<ReachedNode<'a>>,
    /// Edges whose endpoints are both reached.
    edges: Vec<&'a ObjectEdge>,
}

impl<'a> LinkResult<'a> {
    fn node_by_header_prefix(&self, prefix: &str) -> &ReachedNode<'a> {
        self.nodes
            .iter()
            .find(|rn| rn.node.header.starts_with(prefix))
            .unwrap_or_else(|| panic!("missing node with header prefix: {prefix:?}"))
    }

    fn edge_pairs(&self) -> HashSet<(String, String)> {
        self.edges
            .iter()
            .map(|e| (e.source_id.clone(), e.target_id.clone()))
            .collect()
    }

    fn headers(&self) -> HashSet<String> {
        self.nodes.iter().map(|rn| rn.node.header.clone()).collect()
    }
}

/// `_source_origin_for_uri`: classify a URI by its basename.
fn source_origin_for_uri(uri: &str) -> Option<&'static str> {
    let path = uri.split('?').next().unwrap_or(uri);
    let path = path.split('#').next().unwrap_or(path);
    let basename = path.rsplit('/').next().unwrap_or(path);
    match basename {
        "bigip_base.conf" => Some("base"),
        "bigip.conf" | "bigip_gtm.conf" | "bigip_user.conf" => Some("synced"),
        "bigip_script.conf" => Some("script"),
        _ => None,
    }
}

/// `_find_root_at_offset`: the smallest node in `uri` whose
/// `[header_start_offset, end_offset]` span contains `offset`.
fn find_root_at_offset<'a>(
    graph: &'a ObjectGraph,
    uri: &str,
    offset: usize,
) -> Option<&'a ObjectNode> {
    graph
        .nodes_by_uri
        .iter()
        .find(|(u, _)| u == uri)
        .map(|(_, nodes)| nodes)?
        .iter()
        .filter(|n| n.header_start_offset <= offset && offset <= n.end_offset)
        .min_by_key(|n| n.end_offset - n.start_offset)
}

/// Reimplements `extract_linked_bigip_objects`: build the graph, resolve a
/// deduplicated root per seed, run a bidirectional BFS bounded by `max_depth`,
/// and return the reached nodes/edges. Returns `None` when there are no seeds,
/// an unknown seed uri, or no resolvable root.
fn extract_linked<'a>(
    graph: &'a ObjectGraph,
    seeds: &[(&str, usize)],
    max_depth: usize,
) -> Option<LinkResult<'a>> {
    const MAX_NODES: usize = 250;

    if seeds.is_empty() {
        return None;
    }
    // Every seed uri must be present in the built graph (a uri only appears if
    // it had a config).
    for (uri, _) in seeds {
        if !graph.nodes_by_uri.iter().any(|(u, _)| u == uri) {
            return None;
        }
    }

    // Resolve roots, deduplicated by node id, in seed order.
    let mut roots: Vec<&ObjectNode> = Vec::new();
    let mut seen_root_ids: HashSet<&str> = HashSet::new();
    for (uri, offset) in seeds {
        if let Some(root) = find_root_at_offset(graph, uri, *offset)
            && seen_root_ids.insert(root.node_id.as_str())
        {
            roots.push(root);
        }
    }
    if roots.is_empty() {
        return None;
    }

    let all_nodes: HashMap<&str, &ObjectNode> = graph
        .nodes_by_uri
        .iter()
        .flat_map(|(_, ns)| ns.iter())
        .map(|n| (n.node_id.as_str(), n))
        .collect();

    let mut outgoing: HashMap<&str, Vec<&ObjectEdge>> = HashMap::new();
    let mut incoming: HashMap<&str, Vec<&ObjectEdge>> = HashMap::new();
    for edge in &graph.edges {
        outgoing
            .entry(edge.source_id.as_str())
            .or_default()
            .push(edge);
        incoming
            .entry(edge.target_id.as_str())
            .or_default()
            .push(edge);
    }

    // Bidirectional BFS from all roots at depth 0.
    let mut depths: HashMap<String, usize> = HashMap::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    for r in &roots {
        depths.insert(r.node_id.clone(), 0);
        queue.push_back(r.node_id.clone());
    }
    while let Some(node_id) = queue.pop_front() {
        if depths.len() >= MAX_NODES {
            break;
        }
        let depth = depths[&node_id];
        if depth >= max_depth {
            continue;
        }
        let mut neighbours: Vec<&str> = Vec::new();
        for e in outgoing.get(node_id.as_str()).into_iter().flatten() {
            neighbours.push(e.target_id.as_str());
        }
        for e in incoming.get(node_id.as_str()).into_iter().flatten() {
            neighbours.push(e.source_id.as_str());
        }
        for neighbour in neighbours {
            if depths.contains_key(neighbour) {
                continue;
            }
            depths.insert(neighbour.to_owned(), depth + 1);
            queue.push_back(neighbour.to_owned());
            if depths.len() >= MAX_NODES {
                break;
            }
        }
    }

    let visited: HashSet<String> = depths.keys().cloned().collect();

    // Reached nodes, sorted by (depth, id).
    let mut sorted: Vec<(String, usize)> = depths.into_iter().collect();
    sorted.sort_by(|a, b| (a.1, &a.0).cmp(&(b.1, &b.0)));
    let nodes: Vec<ReachedNode> = sorted
        .into_iter()
        .map(|(id, depth)| ReachedNode {
            node: all_nodes[id.as_str()],
            depth,
        })
        .collect();

    let edges: Vec<&ObjectEdge> = graph
        .edges
        .iter()
        .filter(|e| visited.contains(&e.source_id) && visited.contains(&e.target_id))
        .collect();

    let root_list: Vec<Root> = roots
        .iter()
        .map(|r| Root {
            id: r.node_id.clone(),
            uri: r.uri.clone(),
            header: r.header.clone(),
        })
        .collect();

    Some(LinkResult {
        root: root_list[0].id.clone(),
        roots: root_list,
        nodes,
        edges,
    })
}

/// Build a graph from `(uri, source)` pairs (each parsed with the `Common`
/// default partition).
fn build_graph(sources: &[(&str, &str)]) -> (ObjectGraph, GraphContext) {
    let ctx = GraphContext::new();
    let cfgs: Vec<(String, _)> = sources
        .iter()
        .map(|(uri, src)| ((*uri).to_owned(), parse_bigip_conf(src, "Common")))
        .collect();
    let src_pairs: Vec<(String, String)> = sources
        .iter()
        .map(|(uri, src)| ((*uri).to_owned(), (*src).to_owned()))
        .collect();
    let cfg_refs: Vec<(String, &_)> = cfgs.iter().map(|(u, c)| (u.clone(), c)).collect();
    let graph = build_bigip_object_graph(&src_pairs, &cfg_refs, &ctx);
    (graph, ctx)
}

/// Byte offset of `needle` within `hay`, plus `add` (mirrors
/// `source.index(...) + N` cursor positioning).
fn offset_of(hay: &str, needle: &str, add: usize) -> usize {
    hay.find(needle)
        .unwrap_or_else(|| panic!("substring not found: {needle:?}"))
        + add
}

// ---------------------------------------------------------------------------
// Tests.
// ---------------------------------------------------------------------------

#[test]
fn extract_linked_objects_traverses_incoming_and_outgoing_links() {
    let source = "ltm node /Common/web1 {\n\
        \x20   address 10.0.0.11\n\
        }\n\
        ltm pool /Common/web_pool {\n\
        \x20   members {\n\
        \x20       /Common/web1:80 { }\n\
        \x20   }\n\
        \x20   monitor /Common/http\n\
        }\n\
        ltm profile http /Common/custom_http {\n\
        \x20   defaults-from /Common/http\n\
        }\n\
        ltm rule /Common/r1 {\n\
        \x20   when HTTP_REQUEST {\n\
        \x20       pool /Common/web_pool\n\
        \x20   }\n\
        }\n\
        ltm virtual /Common/www_vs {\n\
        \x20   destination /Common/192.0.2.10:443\n\
        \x20   pool /Common/web_pool\n\
        \x20   profiles {\n\
        \x20       /Common/custom_http { }\n\
        \x20   }\n\
        \x20   rules {\n\
        \x20       /Common/r1\n\
        \x20   }\n\
        }\n";
    let uri = "file:///bigip.conf";
    let (graph, _ctx) = build_graph(&[(uri, source)]);

    let offset = offset_of(source, "ltm pool /Common/web_pool", 5);
    let result = extract_linked(&graph, &[(uri, offset)], 4).expect("result is not None");

    let pool = result.node_by_header_prefix("ltm pool /Common/web_pool");
    let vs = result.node_by_header_prefix("ltm virtual /Common/www_vs");
    let profile = result.node_by_header_prefix("ltm profile http /Common/custom_http");
    let rule = result.node_by_header_prefix("ltm rule /Common/r1");

    // The pool (containing the cursor) is the root, at depth 0.
    assert_eq!(pool.node.node_id, result.root);
    // The virtual references the pool (incoming, 1 hop); the profile is a
    // further hop out from the virtual; the rule references the pool (1 hop).
    assert!(vs.depth >= 1, "vs depth {} >= 1", vs.depth);
    assert!(profile.depth >= 2, "profile depth {} >= 2", profile.depth);
    assert!(rule.depth >= 1, "rule depth {} >= 1", rule.depth);

    let pairs = result.edge_pairs();
    assert!(
        pairs.contains(&(vs.node.node_id.clone(), pool.node.node_id.clone())),
        "virtual → pool edge"
    );
    assert!(
        pairs.contains(&(vs.node.node_id.clone(), profile.node.node_id.clone())),
        "virtual → profile edge"
    );
    assert!(
        pairs.contains(&(rule.node.node_id.clone(), pool.node.node_id.clone())),
        "rule → pool edge"
    );
}

#[test]
fn extract_linked_objects_resolves_across_bigip_conf_files() {
    let base_source = "ltm pool /Common/web_pool {\n\
        \x20   members {\n\
        \x20       /Common/web1:80 { }\n\
        \x20   }\n\
        }\n";
    let app_source = "ltm virtual /Common/www_vs {\n\
        \x20   destination /Common/192.0.2.10:443\n\
        \x20   pool /Common/web_pool\n\
        }\n";
    let base_uri = "file:///bigip_base.conf";
    let app_uri = "file:///bigip.conf";

    let (graph, _ctx) = build_graph(&[(base_uri, base_source), (app_uri, app_source)]);

    let offset = offset_of(base_source, "ltm pool /Common/web_pool", 8);
    let result = extract_linked(&graph, &[(base_uri, offset)], 3).expect("result is not None");

    let pool = result.node_by_header_prefix("ltm pool /Common/web_pool");
    let vs = result.node_by_header_prefix("ltm virtual /Common/www_vs");
    assert_eq!(pool.node.uri, base_uri);
    assert_eq!(vs.node.uri, app_uri);

    let pairs = result.edge_pairs();
    assert!(
        pairs.contains(&(vs.node.node_id.clone(), pool.node.node_id.clone())),
        "cross-file virtual → pool edge"
    );
}

#[test]
fn extract_linked_objects_keeps_gtm_rule_pool_links_in_gtm_domain() {
    let source = "ltm pool /Common/app_pool {\n\
        \x20   members {\n\
        \x20       /Common/10.0.0.11:80 { }\n\
        \x20   }\n\
        }\n\
        gtm pool a /Common/app_pool { }\n\
        gtm rule /Common/geo_rule {\n\
        \x20   when DNS_REQUEST {\n\
        \x20       pool /Common/app_pool\n\
        \x20   }\n\
        }\n";
    let uri = "file:///bigip_gtm.conf";
    let (graph, _ctx) = build_graph(&[(uri, source)]);

    let offset = offset_of(source, "gtm rule /Common/geo_rule", 5);
    let result = extract_linked(&graph, &[(uri, offset)], 2).expect("result is not None");

    let rule = result.node_by_header_prefix("gtm rule /Common/geo_rule");
    let gtm_pool = result.node_by_header_prefix("gtm pool a /Common/app_pool");

    let pairs = result.edge_pairs();
    assert!(
        pairs.contains(&(rule.node.node_id.clone(), gtm_pool.node.node_id.clone())),
        "gtm rule → gtm pool edge"
    );

    // The gtm rule's pool reference must resolve within the gtm domain: no
    // edge from the rule should resolve through the `ltm_pool` kind.
    let rule_edges: Vec<&&ObjectEdge> = result
        .edges
        .iter()
        .filter(|e| e.source_id == rule.node.node_id)
        .collect();
    assert!(
        !rule_edges.iter().any(|e| e.via_kind == "ltm_pool"),
        "no gtm rule edge should resolve via ltm_pool; got {:?}",
        rule_edges.iter().map(|e| &e.via_kind).collect::<Vec<_>>()
    );
}

#[test]
fn extract_linked_objects_multi_cursor_merges_graphs() {
    // Multiple cursor offsets seed the BFS from separate roots.
    let source = "ltm pool /Common/pool_a {\n\
        \x20   members {\n\
        \x20       /Common/10.0.0.1:80 { }\n\
        \x20   }\n\
        }\n\
        ltm pool /Common/pool_b {\n\
        \x20   members {\n\
        \x20       /Common/10.0.0.2:80 { }\n\
        \x20   }\n\
        }\n\
        ltm rule /Common/irule_d {\n\
        \x20   when HTTP_REQUEST {\n\
        \x20       pool /Common/pool_a\n\
        \x20   }\n\
        }\n\
        ltm virtual /Common/vs_a {\n\
        \x20   pool /Common/pool_a\n\
        \x20   rules {\n\
        \x20       /Common/irule_d\n\
        \x20   }\n\
        }\n\
        ltm virtual /Common/vs_b {\n\
        \x20   pool /Common/pool_b\n\
        }\n";
    let uri = "file:///bigip.conf";
    let (graph, _ctx) = build_graph(&[(uri, source)]);

    // Cursors in vs_a and irule_d – their combined graphs cover both.
    let offset_vs = offset_of(source, "ltm virtual /Common/vs_a", 5);
    let offset_irule = offset_of(source, "ltm rule /Common/irule_d", 5);

    let result =
        extract_linked(&graph, &[(uri, offset_vs), (uri, offset_irule)], 4).expect("not None");

    // Both roots should appear.
    let root_ids: HashSet<&str> = result.roots.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(root_ids.len(), 2);

    let headers = result.headers();
    assert!(headers.contains("ltm virtual /Common/vs_a"));
    assert!(headers.contains("ltm rule /Common/irule_d"));
    assert!(headers.contains("ltm pool /Common/pool_a"));

    // Backward compat: root points to the first seed.
    assert_eq!(result.root, result.roots[0].id);
}

#[test]
fn extract_linked_objects_multi_cursor_deduplicates_same_object() {
    // Two cursors inside the same object produce a single root.
    let source = "ltm pool /Common/my_pool {\n\
        \x20   members {\n\
        \x20       /Common/10.0.0.1:80 { }\n\
        \x20   }\n\
        }\n";
    let uri = "file:///bigip.conf";
    let (graph, _ctx) = build_graph(&[(uri, source)]);

    let offset_a = offset_of(source, "ltm pool", 5);
    let offset_b = offset_of(source, "members", 2);

    let result = extract_linked(&graph, &[(uri, offset_a), (uri, offset_b)], 4).expect("not None");
    assert_eq!(result.roots.len(), 1);
}

#[test]
fn extract_linked_objects_multi_cursor_across_files() {
    // Cursors in different config files produce a merged graph.
    let base_source = "ltm pool /Common/web_pool {\n\
        \x20   members {\n\
        \x20       /Common/web1:80 { }\n\
        \x20   }\n\
        }\n";
    let app_source = "ltm virtual /Common/www_vs {\n\
        \x20   destination /Common/192.0.2.10:443\n\
        \x20   pool /Common/web_pool\n\
        }\n";
    let base_uri = "file:///bigip_base.conf";
    let app_uri = "file:///bigip.conf";

    let (graph, _ctx) = build_graph(&[(base_uri, base_source), (app_uri, app_source)]);

    let result = extract_linked(
        &graph,
        &[
            (base_uri, offset_of(base_source, "ltm pool", 5)),
            (app_uri, offset_of(app_source, "ltm virtual", 5)),
        ],
        3,
    )
    .expect("not None");

    assert_eq!(result.roots.len(), 2);
    let root_uris: HashSet<&str> = result.roots.iter().map(|r| r.uri.as_str()).collect();
    assert!(root_uris.contains(base_uri));
    assert!(root_uris.contains(app_uri));
}

#[test]
fn extract_linked_objects_source_origin_annotation() {
    // Nodes carry a sourceOrigin tag derived from their URI filename.
    let base_source = "ltm pool /Common/web_pool {\n\
        \x20   members {\n\
        \x20       /Common/web1:80 { }\n\
        \x20   }\n\
        }\n";
    let app_source = "ltm virtual /Common/www_vs {\n\
        \x20   pool /Common/web_pool\n\
        }\n";
    let script_source = "ltm rule /Common/my_rule {\n\
        \x20   when HTTP_REQUEST {\n\
        \x20       pool /Common/web_pool\n\
        \x20   }\n\
        }\n";
    let base_uri = "file:///config/bigip_base.conf";
    let app_uri = "file:///config/bigip.conf";
    let script_uri = "file:///config/bigip_script.conf";

    let (graph, _ctx) = build_graph(&[
        (base_uri, base_source),
        (app_uri, app_source),
        (script_uri, script_source),
    ]);

    let result = extract_linked(
        &graph,
        &[(app_uri, offset_of(app_source, "ltm virtual", 5))],
        4,
    )
    .expect("not None");

    let origin_by_header: HashMap<String, Option<&'static str>> = result
        .nodes
        .iter()
        .map(|rn| (rn.node.header.clone(), source_origin_for_uri(&rn.node.uri)))
        .collect();

    assert_eq!(
        origin_by_header.get("ltm virtual /Common/www_vs"),
        Some(&Some("synced"))
    );
    assert_eq!(
        origin_by_header.get("ltm pool /Common/web_pool"),
        Some(&Some("base"))
    );
    assert_eq!(
        origin_by_header.get("ltm rule /Common/my_rule"),
        Some(&Some("script"))
    );
}

#[test]
fn extract_linked_objects_finds_firewall_rule_address_list_via_registry() {
    // `security firewall rule-list.rules` carries typed firewall-rule bodies
    // whose destination address-list refs must surface as graph edges, so the
    // rule-list is reachable from the address-list (the registry/pilot path).
    let source = "security firewall address-list /Common/web_servers { }\n\
        security firewall rule-list /Common/rl_web {\n\
        \x20   rules {\n\
        \x20       allow_https {\n\
        \x20           action accept\n\
        \x20           destination { address-lists { /Common/web_servers } }\n\
        \x20       }\n\
        \x20   }\n\
        }\n";
    let uri = "file:///bigip.conf";
    let (graph, _ctx) = build_graph(&[(uri, source)]);

    let offset = offset_of(
        source,
        "security firewall address-list /Common/web_servers",
        5,
    );
    let result = extract_linked(&graph, &[(uri, offset)], 4).expect("not None");

    let headers = result.headers();
    assert!(
        headers.contains("security firewall rule-list /Common/rl_web"),
        "rule-list must be reachable from the address-list it references; got headers: {headers:?}"
    );
}
