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

//! Aggregate statistics for one or more BIG-IP configs (powers `f5 stats` /
//! `summary`).
//!
//! Counts come off the parsed [`BigipConfig`] tables and the reference graph
//! ([`crate::graph`]); nothing here re-parses the source.

use std::collections::HashMap;

use tcl_registry::bigip::default_registry;

use crate::graph::{ObjectGraph, ObjectNode};
use crate::parser::driver::BigipConfig;

/// Aggregate stats over a set of configs.
/// Count maps keep insertion order so the JSON preserves that key order.
pub struct StatsReport {
    /// `(category, count)` — pool/virtual/node/… in fixed category order.
    pub object_counts: Vec<(String, usize)>,
    /// `(partition, count)` in first-seen order.
    pub partition_counts: Vec<(String, usize)>,
    /// `(rule_path, non_blank_lines)` in source order.
    pub irule_loc: Vec<(String, usize)>,
    /// `(event, count)` in first-seen order.
    pub irule_event_histogram: Vec<(String, usize)>,
    /// Top-N most-referenced `(identifier, incoming_edges)`.
    pub top_referenced: Vec<(String, usize)>,
    /// Count of non-root, deletable-eligible nodes with zero inbound refs.
    pub orphan_count: usize,
    /// Sum of [`Self::object_counts`].
    pub total_objects: usize,
    /// Rendered text report.
    pub text_report: String,
}

/// Whether `kind` is a keep-graph root: `ltm_virtual` or any GTM wide-IP.
#[must_use]
pub fn is_root_kind(kind: Option<&str>) -> bool {
    matches!(kind, Some("ltm_virtual")) || kind.is_some_and(|k| k.starts_with("gtm_wideip"))
}

/// Every `ltm`/`gtm` kind eligible for cleanup: module-scoped, minus the
/// never-delete kinds and the roots.
#[must_use]
pub fn deletable_kinds() -> std::collections::HashSet<&'static str> {
    const NEVER_DELETE: [&str; 2] = ["ltm_virtual", "ltm_virtual_address"];
    let reg = default_registry();
    let mut eligible = std::collections::HashSet::new();
    for spec in reg.specs() {
        let kind = spec.kind_spec.kind;
        if !matches!(spec.kind_spec.module, Some("ltm" | "gtm")) {
            continue;
        }
        if NEVER_DELETE.contains(&kind) || is_root_kind(Some(kind)) {
            continue;
        }
        eligible.insert(kind);
    }
    eligible
}

fn count_irule_lines(body: &str) -> usize {
    body.lines()
        .filter(|raw| {
            let line = raw.trim();
            !line.is_empty() && !line.starts_with('#')
        })
        .count()
}

fn partition_of(full_path: &str) -> String {
    if !full_path.starts_with('/') {
        return "(no partition)".to_owned();
    }
    let parts: Vec<&str> = full_path.splitn(3, '/').collect();
    if parts.len() >= 2 {
        format!("/{}/", parts[1])
    } else {
        "(no partition)".to_owned()
    }
}

/// An insertion-ordered counter: increments preserve first-seen key order.
#[derive(Default)]
struct OrderedCounter {
    order: Vec<String>,
    counts: HashMap<String, usize>,
}

impl OrderedCounter {
    fn add(&mut self, key: &str) {
        if let Some(c) = self.counts.get_mut(key) {
            *c += 1;
        } else {
            self.order.push(key.to_owned());
            self.counts.insert(key.to_owned(), 1);
        }
    }
    fn into_pairs(self) -> Vec<(String, usize)> {
        self.order
            .iter()
            .map(|k| (k.clone(), self.counts[k]))
            .collect()
    }
    /// `Counter.most_common`: count desc, ties in first-seen order.
    fn most_common(&self) -> Vec<(String, usize)> {
        let mut pairs: Vec<(String, usize)> = self
            .order
            .iter()
            .map(|k| (k.clone(), self.counts[k]))
            .collect();
        pairs.sort_by_key(|p| std::cmp::Reverse(p.1)); // stable: ties keep first-seen order
        pairs
    }
}

/// Compute the statistics report over `graph` + `configs`.
#[must_use]
pub fn compute_stats(
    graph: &ObjectGraph,
    configs: &[(String, &BigipConfig)],
    top_n: usize,
) -> StatsReport {
    // Object counts from the typed tables + generic objects.
    let mut table_counts: HashMap<&str, usize> = HashMap::new();
    let mut generic = 0usize;
    for (_uri, cfg) in configs {
        for p in &cfg.objects {
            *table_counts.entry(p.table_name).or_default() += 1;
        }
        generic += cfg.generic_objects.len();
    }
    let object_counts: Vec<(String, usize)> = [
        ("pool", "pools"),
        ("virtual", "virtual_servers"),
        ("node", "nodes"),
        ("profile", "profiles"),
        ("monitor", "monitors"),
        ("data-group", "data_groups"),
        ("snatpool", "snat_pools"),
        ("persistence", "persistence"),
        ("rule", "rules"),
    ]
    .into_iter()
    .map(|(cat, table)| {
        (
            cat.to_owned(),
            table_counts.get(table).copied().unwrap_or(0),
        )
    })
    .chain(std::iter::once(("generic".to_owned(), generic)))
    .collect();

    // Partition breakdown over pool / virtual / node paths, in config order.
    let mut partition_counter = OrderedCounter::default();
    for (_uri, cfg) in configs {
        for table in ["pools", "virtual_servers", "nodes"] {
            for p in cfg.objects.iter().filter(|p| p.table_name == table) {
                partition_counter.add(&partition_of(&p.full_path));
            }
        }
    }

    // iRule LOC + event histogram from the `ltm rule` graph nodes (the `rules`
    // table), in source order.
    let mut irule_loc: Vec<(String, usize)> = Vec::new();
    let mut irule_events = OrderedCounter::default();
    for node in all_nodes(graph) {
        if node.module != "ltm" || node.object_type != "rule" {
            continue;
        }
        irule_loc.push((node.identifier.clone(), count_irule_lines(&node.body)));
        for block in tcl_irules::when_blocks(&node.body) {
            irule_events.add(&block.event);
        }
    }

    // Incoming-edge counts for top-referenced + orphans.
    let mut incoming = OrderedCounter::default();
    for edge in &graph.edges {
        incoming.add(&edge.target_id);
    }
    let by_id: HashMap<&str, &ObjectNode> =
        all_nodes(graph).map(|n| (n.node_id.as_str(), n)).collect();

    let mut top_referenced: Vec<(String, usize)> = Vec::new();
    for (node_id, count) in incoming.most_common() {
        if let Some(node) = by_id.get(node_id.as_str()) {
            top_referenced.push((node.identifier.clone(), count));
            if top_referenced.len() >= top_n {
                break;
            }
        }
    }

    let deletable = deletable_kinds();
    let mut orphan_count = 0usize;
    for node in all_nodes(graph) {
        if is_root_kind(node.kind) {
            continue;
        }
        let Some(kind) = node.kind else { continue };
        if !deletable.contains(kind) {
            continue;
        }
        if incoming.counts.get(&node.node_id).copied().unwrap_or(0) == 0 {
            orphan_count += 1;
        }
    }

    let partition_counts = partition_counter.into_pairs();
    let irule_event_histogram = irule_events.most_common();
    let total_objects: usize = object_counts.iter().map(|(_, c)| c).sum();

    let text_report = format_text(&TextView {
        object_counts: &object_counts,
        partition_counts: &partition_counts,
        irule_loc: &irule_loc,
        irule_events: &irule_events,
        top: &top_referenced,
        orphans: orphan_count,
        total: total_objects,
    });

    StatsReport {
        object_counts,
        partition_counts,
        irule_loc,
        irule_event_histogram,
        top_referenced,
        orphan_count,
        total_objects,
        text_report,
    }
}

fn all_nodes(graph: &ObjectGraph) -> impl Iterator<Item = &ObjectNode> {
    graph
        .nodes_by_uri
        .iter()
        .flat_map(|(_uri, nodes)| nodes.iter())
}

/// Borrowed inputs for [`format_text`]; bundled to keep the arg count low.
struct TextView<'a> {
    object_counts: &'a [(String, usize)],
    partition_counts: &'a [(String, usize)],
    irule_loc: &'a [(String, usize)],
    irule_events: &'a OrderedCounter,
    top: &'a [(String, usize)],
    orphans: usize,
    total: usize,
}

fn format_text(view: &TextView) -> String {
    let &TextView {
        object_counts,
        partition_counts,
        irule_loc,
        irule_events,
        top,
        orphans,
        total,
    } = view;
    let mut lines = vec![format!("objects: {total} total")];
    let mut oc = object_counts.to_vec();
    oc.sort_by(|a, b| a.0.cmp(&b.0));
    for (kind, count) in &oc {
        lines.push(format!("  {kind:<14} {count}"));
    }
    lines.push(String::new());
    lines.push("partitions:".to_owned());
    let mut pc = partition_counts.to_vec();
    pc.sort_by(|a, b| a.0.cmp(&b.0));
    for (partition, count) in &pc {
        lines.push(format!("  {partition:<20} {count}"));
    }
    lines.push(String::new());
    let loc_sum: usize = irule_loc.iter().map(|(_, c)| c).sum();
    lines.push(format!(
        "iRules: {} ({loc_sum} non-blank lines)",
        irule_loc.len()
    ));
    if !irule_loc.is_empty() {
        let mut by_loc = irule_loc.to_vec();
        by_loc.sort_by_key(|p| std::cmp::Reverse(p.1)); // stable: -loc, ties keep order
        for (path, loc) in by_loc.iter().take(10) {
            lines.push(format!("  {path:<40} {loc} loc"));
        }
    }
    let events = irule_events.most_common();
    if !events.is_empty() {
        lines.push(String::new());
        lines.push("iRule events:".to_owned());
        for (event, count) in events.iter().take(15) {
            lines.push(format!("  {event:<32} {count}"));
        }
    }
    lines.push(String::new());
    lines.push("most referenced:".to_owned());
    for (ident, count) in top {
        lines.push(format!("  {ident:<40} {count}"));
    }
    lines.push(String::new());
    lines.push(format!("orphans (no inbound references): {orphans}"));
    lines.join("\n")
}

/// 2-space-indented JSON (built by hand
/// for a deterministic key order, like the graph export).
#[must_use]
pub fn report_to_json(report: &StatsReport) -> String {
    use std::fmt::Write as _;

    use crate::jsonfmt::json_string as q;
    let obj_block = |pairs: &[(String, usize)], indent: &str| -> String {
        if pairs.is_empty() {
            return "{}".to_owned();
        }
        let inner: Vec<String> = pairs
            .iter()
            .map(|(k, v)| format!("{indent}  {}: {v}", q(k)))
            .collect();
        format!("{{\n{}\n{indent}}}", inner.join(",\n"))
    };

    let mut out = String::from("{\n");
    let _ = writeln!(out, "  \"totalObjects\": {},", report.total_objects);
    let _ = writeln!(
        out,
        "  \"objectCounts\": {},",
        obj_block(&report.object_counts, "  ")
    );
    let _ = writeln!(
        out,
        "  \"partitionCounts\": {},",
        obj_block(&report.partition_counts, "  ")
    );
    let _ = writeln!(
        out,
        "  \"iruleLoc\": {},",
        obj_block(&report.irule_loc, "  ")
    );
    let _ = writeln!(
        out,
        "  \"iruleEventHistogram\": {},",
        obj_block(&report.irule_event_histogram, "  ")
    );
    // topReferenced is a list of {identifier, incomingEdges}.
    if report.top_referenced.is_empty() {
        out.push_str("  \"topReferenced\": [],\n");
    } else {
        out.push_str("  \"topReferenced\": [\n");
        let items: Vec<String> = report
            .top_referenced
            .iter()
            .map(|(ident, count)| {
                format!(
                    "    {{\n      \"identifier\": {},\n      \"incomingEdges\": {count}\n    }}",
                    q(ident)
                )
            })
            .collect();
        out.push_str(&items.join(",\n"));
        out.push_str("\n  ],\n");
    }
    let _ = writeln!(out, "  \"orphanCount\": {}", report.orphan_count);
    out.push('}');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_root_kind_matches_virtual_and_wideip() {
        // Cleanup roots (stats.rs had no unit coverage): the LTM virtual and
        // any GTM wide-IP kind; everything else, and None, is a non-root.
        assert!(is_root_kind(Some("ltm_virtual")));
        assert!(is_root_kind(Some("gtm_wideip")));
        assert!(is_root_kind(Some("gtm_wideip_a")));
        assert!(!is_root_kind(Some("ltm_pool")));
        assert!(!is_root_kind(None));
    }

    #[test]
    fn deletable_kinds_excludes_roots_and_never_delete() {
        let kinds = deletable_kinds();
        assert!(!kinds.is_empty());
        assert!(!kinds.contains("ltm_virtual")); // a root
        assert!(!kinds.contains("ltm_virtual_address")); // never-delete
    }
}
