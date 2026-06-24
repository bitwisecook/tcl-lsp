//! Graph-backed reference builtins (`category="graph"`).
//!
//! `refs(obj)` returns the objects an object references (forward edges);
//! `referenced_by(obj)` returns the objects that reference it (reverse edges);
//! `references_to(path)` is the path-keyed reverse query; and
//! `check_partition_visibility()` audits every reference against the F5
//! partition-visibility rules. All four walk the same BIG-IP reference graph
//! `f5 grep` / `compute_grep` build — here reproduced by walking the cached
//! [`ObjectGraph`](tcl_bigip::graph::ObjectGraph) edges directly.
//!
//! These are registered as [`Builtin::Ctx`](super::Builtin::Ctx) because they
//! need the parsed config(s) to walk: the evaluator hands them
//! `&mut EvalContext`, from which they resolve the relevant [`Root`] (the
//! object's originating config in non-merge mode).

use std::collections::BTreeSet;
use std::rc::Rc;

use tcl_bigip::graph::{ObjectGraph, ObjectNode};
use tcl_bigip::value::ObjectPath;

use crate::errors::QueryError;
use crate::eval::{EvalContext, Root};
use crate::value::Value;

use super::{BuiltinSpec, as_str, type_name};

/// Walk direction over the reference graph, for the one-hop forward/reverse
/// queries the builtins use.
#[derive(Clone, Copy)]
enum Direction {
    Forward,
    Reverse,
}

/// Sort key: `(kind or "", identifier)`.
fn sort_key(node: &ObjectNode) -> (&str, &str) {
    (node.kind.unwrap_or(""), node.identifier.as_str())
}

/// One-hop neighbour walk over the cached graph, reproducing the relevant
/// slice of `compute_grep` for `max_depth=1`, `recurse=True`, `use_exact=True`.
///
/// Returns the *related* nodes (depth 1) for every seed whose identifier
/// equals `pattern`, in `compute_grep`'s `related` order:
/// `(depth, kind, identifier)` — and since every related node here is at
/// depth 1, that collapses to `(kind, identifier)`. Seeds are excluded from
/// the result, matching the BFS's seed/related split.
fn related_one_hop<'g>(
    graph: &'g ObjectGraph,
    pattern: &str,
    direction: Direction,
) -> Vec<&'g ObjectNode> {
    // Flatten the per-uri node lists and index nodes by id.
    let mut all_nodes: Vec<&ObjectNode> = Vec::new();
    for (_uri, nodes) in &graph.nodes_by_uri {
        for node in nodes {
            all_nodes.push(node);
        }
    }
    let node_by_id =
        |id: &str| -> Option<&ObjectNode> { all_nodes.iter().copied().find(|n| n.node_id == id) };

    // Seeds: nodes whose identifier matches exactly, sorted by `sort_key`.
    // The seed *order* does not affect the related set (related is re-sorted),
    // but it is sorted for determinism.
    let mut seeds: Vec<&ObjectNode> = all_nodes
        .iter()
        .copied()
        .filter(|n| n.identifier == pattern)
        .collect();
    seeds.sort_by(|a, b| sort_key(a).cmp(&sort_key(b)));

    let seed_ids: BTreeSet<&str> = seeds.iter().map(|n| n.node_id.as_str()).collect();

    // One-hop BFS expansion: collect every neighbour reached from a seed in
    // the requested direction. `compute_grep` records first-visit depth and
    // skips already-visited ids; with `max_depth=1` only depth-1 neighbours
    // (the seeds' direct neighbours) are collected.
    let mut visited: BTreeSet<&str> = seed_ids.clone();
    let mut related: Vec<&ObjectNode> = Vec::new();
    for seed in &seeds {
        let mut neighbours: Vec<&str> = Vec::new();
        match direction {
            Direction::Forward => {
                for edge in &graph.edges {
                    if edge.source_id == seed.node_id {
                        neighbours.push(edge.target_id.as_str());
                    }
                }
            }
            Direction::Reverse => {
                for edge in &graph.edges {
                    if edge.target_id == seed.node_id {
                        neighbours.push(edge.source_id.as_str());
                    }
                }
            }
        }
        for nid in neighbours {
            if visited.contains(nid) {
                continue;
            }
            visited.insert(nid);
            if let Some(node) = node_by_id(nid) {
                related.push(node);
            }
        }
    }

    // Related ids sorted by (depth, sort-key); all
    // depths are 1 here, so this is `sort_key` order.
    related.sort_by(|a, b| sort_key(a).cmp(&sort_key(b)));
    related
}

/// Resolve the [`Root`] that owns `config_uri`: the active root if its uri
/// matches, else a named root.
/// Used to validate that an object came from a config (and, in non-merge
/// mode, to pick the source-scoped graph). Merge mode widens the graph to
/// every loaded source via [`EvalContext::merged_graph`] instead.
fn root_for_uri<'c>(ctx: &'c EvalContext, config_uri: &str) -> Result<&'c Rc<Root>, QueryError> {
    if config_uri.is_empty() {
        return Err(QueryError::builtin(
            "graph builtins require an object loaded from a config",
        ));
    }
    if ctx.root.uri == config_uri {
        return Ok(&ctx.root);
    }
    if let Some(root) = ctx.named_roots.get(config_uri) {
        return Ok(root);
    }
    // In merge mode the object may originate from a sibling source that is
    // only bound as a merge root (not as a `$name`); accept it.
    if ctx.merge_mode
        && let Some(root) = ctx.merge_roots.iter().find(|r| r.uri == config_uri)
    {
        return Ok(root);
    }
    Err(QueryError::builtin(
        "graph builtins called outside an active query context",
    ))
}

/// Shared body for `refs` / `referenced_by`: the object-relative graph walk.
fn object_relative_refs(
    name: &str,
    args: &[Value],
    ctx: &EvalContext,
    direction: Direction,
) -> Result<Value, QueryError> {
    let Value::ObjectRef(obj) = &args[0] else {
        return Err(QueryError::builtin(format!(
            "{name}: argument 1 must be an object, got {}",
            type_name(&args[0])
        )));
    };
    let root = root_for_uri(ctx, &obj.config_uri)?;
    // Merge mode joins every loaded source's graph so a reference in one
    // file resolves into an object in another; otherwise stay scoped to
    // the originating source.
    let graph = if ctx.merge_mode {
        ctx.merged_graph()
    } else {
        root.graph()
    };
    let related = related_one_hop(&graph, &obj.full_path, direction);
    let out: Vec<Value> = related
        .into_iter()
        .filter(|node| node.identifier != obj.full_path)
        .map(|node| Value::Str(node.identifier.clone()))
        .collect();
    Ok(Value::List(out))
}

/// `refs(obj)` — forward edges (the objects `obj` references).
fn bi_refs(args: &[Value], ctx: &mut EvalContext) -> Result<Value, QueryError> {
    object_relative_refs("refs", args, ctx, Direction::Forward)
}

/// `referenced_by(obj)` — reverse edges (the objects that reference `obj`).
fn bi_referenced_by(args: &[Value], ctx: &mut EvalContext) -> Result<Value, QueryError> {
    object_relative_refs("referenced_by", args, ctx, Direction::Reverse)
}

/// `references_to(path)` — reverse query keyed by a literal full-path string.
/// Identity-shaped (exact seed), de-duplicated, in graph order.
fn bi_references_to(args: &[Value], ctx: &mut EvalContext) -> Result<Value, QueryError> {
    let target = as_str(&args[0], "references_to", 1)?;
    let target = target.trim();
    if target.is_empty() {
        return Ok(Value::List(Vec::new()));
    }
    let root = &ctx.root;
    let graph = root.graph();
    let related = related_one_hop(&graph, target, Direction::Reverse);
    let mut seen: Vec<String> = Vec::new();
    for node in related {
        if node.identifier == target {
            continue;
        }
        if !seen.iter().any(|s| s == &node.identifier) {
            seen.push(node.identifier.clone());
        }
    }
    Ok(Value::List(seen.into_iter().map(Value::Str).collect()))
}

/// `check_partition_visibility()` — every reference whose referrer partition
/// can't see the target partition, as sorted `"<referrer> -> <target>"`
/// strings. Performs a forward-grep-per-object audit; the referrer
/// set is exactly the typed graph nodes (generic objects carry namespaced
/// keys that never appear as graph-node identifiers).
fn bi_check_partition_visibility(
    _args: &[Value],
    ctx: &mut EvalContext,
) -> Result<Value, QueryError> {
    let root = &ctx.root;
    let graph = root.graph();

    // Distinct referrer identifiers, in sorted order (the final result is
    // sorted regardless, so order here only affects nothing observable).
    let mut referrers: BTreeSet<&str> = BTreeSet::new();
    for (_uri, nodes) in &graph.nodes_by_uri {
        for node in nodes {
            referrers.insert(node.identifier.as_str());
        }
    }

    let mut violations: BTreeSet<String> = BTreeSet::new();
    for referrer_path in referrers {
        let Some(referrer) = ObjectPath::try_parse(referrer_path) else {
            continue;
        };
        for node in related_one_hop(&graph, referrer_path, Direction::Forward) {
            if node.identifier == referrer_path {
                continue;
            }
            let Some(target) = ObjectPath::try_parse(&node.identifier) else {
                continue;
            };
            if referrer.partition().can_see(target.partition()) {
                continue;
            }
            violations.insert(format!("{referrer_path} -> {}", node.identifier));
        }
    }
    Ok(Value::List(
        violations.into_iter().map(Value::Str).collect(),
    ))
}

/// Compute a rule's forward references (`max_depth=1`) classified into
/// `(pools, persists, data_groups)`.
/// Each bucket is de-duplicated and sorted.
#[must_use]
pub(crate) fn extract_rule_refs(
    full_path: &str,
    root: &Rc<Root>,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let graph = root.graph();
    let related = related_one_hop(&graph, full_path, Direction::Forward);
    let mut pools: BTreeSet<String> = BTreeSet::new();
    let mut persists: BTreeSet<String> = BTreeSet::new();
    let mut data_groups: BTreeSet<String> = BTreeSet::new();
    for node in related {
        if node.identifier == full_path {
            continue;
        }
        if node.module == "ltm" && node.object_type == "pool" {
            pools.insert(node.identifier.clone());
        } else if node.module == "ltm" && node.object_type.starts_with("persistence") {
            persists.insert(node.identifier.clone());
        } else if node.object_type.starts_with("data-group") {
            data_groups.insert(node.identifier.clone());
        }
    }
    (
        pools.into_iter().collect(),
        persists.into_iter().collect(),
        data_groups.into_iter().collect(),
    )
}

/// Distinct reverse-referrer full-paths of `target` over the root graph — a
/// one-hop reverse, exact-match walk (the same the `rename` builtin uses). The
/// seed (`target` itself) is excluded.
#[must_use]
pub(crate) fn reverse_referrers(target: &str, root: &Rc<Root>) -> Vec<String> {
    let graph = root.graph();
    let mut out: Vec<String> = Vec::new();
    for node in related_one_hop(&graph, target, Direction::Reverse) {
        if node.identifier == target {
            continue;
        }
        if !out.iter().any(|s| s == &node.identifier) {
            out.push(node.identifier.clone());
        }
    }
    out
}

/// Registry entries for the graph builtins (all [`Builtin::Ctx`]).
pub(crate) fn registrations() -> Vec<(&'static str, BuiltinSpec)> {
    vec![
        super::ctx_broadcast("refs", "graph", 1, Some(1), bi_refs),
        super::ctx_broadcast("referenced_by", "graph", 1, Some(1), bi_referenced_by),
        super::ctx("references_to", "graph", 1, Some(1), bi_references_to),
        super::ctx(
            "check_partition_visibility",
            "graph",
            0,
            Some(0),
            bi_check_partition_visibility,
        ),
    ]
}
