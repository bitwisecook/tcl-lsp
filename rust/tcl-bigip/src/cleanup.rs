//! Cleanup analysis (powers `f5 cleanup` / `clean`).
//!
//! Walks the forward reference graph from every `ltm virtual` / `gtm wide-IP`
//! root; any in-scope, deletable object not reached is a cleanup candidate. The
//! candidates are emitted in reverse-topological order so each `tmsh delete`
//! runs after the objects that reference its target are already gone.

use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::BuildHasher;

use crate::graph::{ObjectEdge, ObjectGraph, ObjectNode};
use crate::range::Range;
use crate::stats::{deletable_kinds, is_root_kind};

/// One BIG-IP object proposed for deletion.
pub struct CleanupCandidate {
    /// Stable graph node id.
    pub node_id: String,
    /// Source URI.
    pub uri: String,
    /// tmsh module word.
    pub module: String,
    /// tmsh object-type word(s).
    pub object_type: String,
    /// Object full-path / identifier.
    pub full_path: String,
    /// Resolved registry kind.
    pub kind: Option<&'static str>,
    /// The `delete <module> <type> <path>` command.
    pub delete_command: String,
    /// Why it's a candidate.
    pub reason: String,
    /// Source span.
    pub range: Range,
}

/// Result of [`compute_cleanup`].
pub struct CleanupReport {
    /// Candidates in reverse-topological (delete) order.
    pub candidates: Vec<CleanupCandidate>,
    /// Kept node ids (reachable from a root), sorted.
    pub kept: Vec<String>,
    /// Root node ids, sorted.
    pub roots: Vec<String>,
    /// Per-kind candidate counts, in first-appearance order.
    pub summary: Vec<(String, usize)>,
    /// The rendered tmsh delete script.
    pub tmsh_script: String,
}

fn build_delete_command(node: &ObjectNode) -> String {
    format!(
        "delete {} {} {}",
        node.module, node.object_type, node.identifier
    )
}

fn matches_keep_filter<S: BuildHasher>(
    node: &ObjectNode,
    keep_paths: &HashSet<String, S>,
    keep_partitions: &[String],
) -> bool {
    keep_paths.contains(&node.identifier)
        || keep_partitions
            .iter()
            .any(|prefix| node.identifier.starts_with(prefix))
}

/// Compute the cleanup report. `source_uris` should be the sorted input URIs
/// (used only in the script header).
#[must_use]
pub fn compute_cleanup<S: BuildHasher>(
    graph: &ObjectGraph,
    source_uris: &[String],
    keep_paths: &HashSet<String, S>,
    keep_partitions: &[String],
) -> CleanupReport {
    let all_nodes: Vec<&ObjectNode> = graph
        .nodes_by_uri
        .iter()
        .flat_map(|(_uri, nodes)| nodes.iter())
        .collect();

    let mut outgoing: HashMap<&str, Vec<&ObjectEdge>> = HashMap::new();
    for edge in &graph.edges {
        outgoing
            .entry(edge.source_id.as_str())
            .or_default()
            .push(edge);
    }

    // Roots: every virtual / wide-IP, in node order.
    let root_ids: Vec<&str> = all_nodes
        .iter()
        .filter(|n| is_root_kind(n.kind))
        .map(|n| n.node_id.as_str())
        .collect();

    // BFS reachability from the roots over outgoing edges.
    let mut kept: HashSet<&str> = root_ids.iter().copied().collect();
    let mut queue: VecDeque<&str> = root_ids.iter().copied().collect();
    while let Some(nid) = queue.pop_front() {
        if let Some(edges) = outgoing.get(nid) {
            for edge in edges {
                if kept.insert(edge.target_id.as_str()) {
                    queue.push_back(edge.target_id.as_str());
                }
            }
        }
    }

    let deletable = deletable_kinds();
    // Candidates: unreached, in ltm/gtm scope, deletable kind, not kept-filtered.
    let candidates: HashMap<&str, &ObjectNode> = all_nodes
        .iter()
        .copied()
        .filter(|node| {
            !kept.contains(node.node_id.as_str())
                && matches!(node.module.as_str(), "ltm" | "gtm")
                && node.kind.is_some_and(|k| deletable.contains(k))
                && !matches_keep_filter(node, keep_paths, keep_partitions)
        })
        .map(|n| (n.node_id.as_str(), n))
        .collect();

    let order = topological_order(&candidates, &graph.edges);

    let mut candidate_list = Vec::new();
    let mut summary_order: Vec<String> = Vec::new();
    let mut summary_counts: HashMap<String, usize> = HashMap::new();
    for nid in &order {
        let node = candidates[nid.as_str()];
        candidate_list.push(CleanupCandidate {
            node_id: node.node_id.clone(),
            uri: node.uri.clone(),
            module: node.module.clone(),
            object_type: node.object_type.clone(),
            full_path: node.identifier.clone(),
            kind: node.kind,
            delete_command: build_delete_command(node),
            reason: "not reachable from any virtual server / wide-IP".to_owned(),
            range: node.range,
        });
        let key = node.kind.unwrap_or("<unknown>").to_owned();
        if let Some(c) = summary_counts.get_mut(&key) {
            *c += 1;
        } else {
            summary_order.push(key.clone());
            summary_counts.insert(key, 1);
        }
    }
    let summary: Vec<(String, usize)> = summary_order
        .iter()
        .map(|k| (k.clone(), summary_counts[k]))
        .collect();

    let tmsh_script = format_tmsh_script(&candidate_list, &summary, root_ids.len(), source_uris);

    let mut kept_sorted: Vec<String> = kept.iter().map(|s| (*s).to_owned()).collect();
    kept_sorted.sort();
    let mut roots_sorted: Vec<String> = root_ids.iter().map(|s| (*s).to_owned()).collect();
    roots_sorted.sort();

    CleanupReport {
        candidates: candidate_list,
        kept: kept_sorted,
        roots: roots_sorted,
        summary,
        tmsh_script,
    }
}

/// Topologically sort candidates so referrers precede referents, with a
/// deterministic `(kind, identifier)` tie-break.
fn topological_order(candidates: &HashMap<&str, &ObjectNode>, edges: &[ObjectEdge]) -> Vec<String> {
    let mut in_degree: HashMap<&str, usize> = candidates.keys().map(|k| (*k, 0usize)).collect();
    let mut out_adj: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut seen: HashSet<(&str, &str)> = HashSet::new();
    for edge in edges {
        let (s, t) = (edge.source_id.as_str(), edge.target_id.as_str());
        if !candidates.contains_key(s) || !candidates.contains_key(t) || s == t {
            continue;
        }
        if seen.insert((s, t)) {
            out_adj.entry(s).or_default().push(t);
            *in_degree.get_mut(t).expect("target in candidates") += 1;
        }
    }

    let sort_key = |nid: &str| -> (&'static str, String) {
        let node = candidates[nid];
        (node.kind.unwrap_or(""), node.identifier.clone())
    };

    let mut ready: Vec<&str> = in_degree
        .iter()
        .filter(|&(_, &d)| d == 0)
        .map(|(n, _)| *n)
        .collect();
    ready.sort_by_key(|n| sort_key(n));
    let mut queue: VecDeque<&str> = ready.into_iter().collect();
    let mut order: Vec<&str> = Vec::new();
    while let Some(nid) = queue.pop_front() {
        order.push(nid);
        if let Some(neighbours) = out_adj.get(nid) {
            let mut sorted_neighbours = neighbours.clone();
            sorted_neighbours.sort_by_key(|n| sort_key(n));
            for neighbour in sorted_neighbours {
                let d = in_degree.get_mut(neighbour).expect("neighbour tracked");
                *d -= 1;
                if *d == 0 {
                    queue.push_back(neighbour);
                }
            }
        }
    }

    if order.len() < candidates.len() {
        let order_set: HashSet<&str> = order.iter().copied().collect();
        let mut leftover: Vec<&str> = candidates
            .keys()
            .copied()
            .filter(|n| !order_set.contains(n))
            .collect();
        leftover.sort_by_key(|n| sort_key(n));
        order.extend(leftover);
    }
    order.into_iter().map(str::to_owned).collect()
}

fn format_tmsh_script(
    candidates: &[CleanupCandidate],
    summary: &[(String, usize)],
    root_count: usize,
    source_uris: &[String],
) -> String {
    let sources = if source_uris.is_empty() {
        "(none)".to_owned()
    } else {
        source_uris.join(", ")
    };
    if candidates.is_empty() {
        return format!(
            "# tcl-lsp BIG-IP cleanup\n# Sources: {sources}\n# Roots (kept): {root_count} virtual server(s) / wide-IP(s)\n# Candidates: none — every object is referenced.\n"
        );
    }
    let mut lines = vec![
        "# tcl-lsp BIG-IP cleanup".to_owned(),
        format!("# Sources: {sources}"),
        format!("# Roots (kept): {root_count} virtual server(s) / wide-IP(s)"),
        format!("# Candidates: {} unreferenced object(s)", candidates.len()),
    ];
    let mut sorted_summary = summary.to_vec();
    sorted_summary.sort_by(|a, b| a.0.cmp(&b.0));
    for (kind, count) in &sorted_summary {
        lines.push(format!("#   {kind}: {count}"));
    }
    lines.push("#".to_owned());
    lines.push("# Review each delete before running.  Order is reverse-".to_owned());
    lines.push("# topological: referencing objects are deleted before the".to_owned());
    lines.push("# objects they reference, so tmsh accepts each command in turn.".to_owned());
    lines.push(String::new());
    for candidate in candidates {
        lines.push(candidate.delete_command.clone());
    }
    lines.push(String::new());
    lines.join("\n")
}

/// 2-space-indented JSON, built
/// by hand for a deterministic key order.
#[must_use]
pub fn report_to_json(report: &CleanupReport) -> String {
    use std::fmt::Write as _;

    use crate::jsonfmt::json_string as q;
    let mut out = String::from("{\n  \"candidates\": [");
    for (i, c) in report.candidates.iter().enumerate() {
        out.push_str(if i == 0 { "\n" } else { ",\n" });
        let kind = c.kind.map_or_else(|| "null".to_owned(), q);
        let _ = write!(
            out,
            "    {{\n      \"nodeId\": {},\n      \"uri\": {},\n      \"module\": {},\n      \"objectType\": {},\n      \"fullPath\": {},\n      \"kind\": {kind},\n      \"deleteCommand\": {},\n      \"reason\": {},\n      \"range\": {{\n        \"start\": {{\n          \"line\": {},\n          \"character\": {}\n        }},\n        \"end\": {{\n          \"line\": {},\n          \"character\": {}\n        }}\n      }}\n    }}",
            q(&c.node_id),
            q(&c.uri),
            q(&c.module),
            q(&c.object_type),
            q(&c.full_path),
            q(&c.delete_command),
            q(&c.reason),
            c.range.start.line,
            c.range.start.character,
            c.range.end.line,
            c.range.end.character,
        );
    }
    out.push_str(if report.candidates.is_empty() {
        "],\n"
    } else {
        "\n  ],\n"
    });

    let str_list = |items: &[String]| -> String {
        if items.is_empty() {
            "[]".to_owned()
        } else {
            let inner: Vec<String> = items.iter().map(|s| format!("    {}", q(s))).collect();
            format!("[\n{}\n  ]", inner.join(",\n"))
        }
    };
    let _ = writeln!(out, "  \"kept\": {},", str_list(&report.kept));
    let _ = writeln!(out, "  \"roots\": {},", str_list(&report.roots));

    if report.summary.is_empty() {
        out.push_str("  \"summary\": {},\n");
    } else {
        let inner: Vec<String> = report
            .summary
            .iter()
            .map(|(k, v)| format!("    {}: {v}", q(k)))
            .collect();
        let _ = writeln!(out, "  \"summary\": {{\n{}\n  }},", inner.join(",\n"));
    }
    let _ = writeln!(out, "  \"tmshScript\": {}", q(&report.tmsh_script));
    out.push_str("}\n");
    out
}
