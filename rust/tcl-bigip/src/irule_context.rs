//! iRule context engine.
//!
//! For one `ltm rule`, [`build_irule_context`] walks the rule body and resolves
//! every BIG-IP object it references (pools, data groups, persistence, SNAT
//! pools, profiles, monitors, nodes, called rules), plus a one-hop transitive
//! expansion (pool members → nodes; pool monitor → monitor). The resulting
//! [`IruleContextBundle`] renders to JSON ([`context_bundle_to_json`] /
//! [`bundles_to_json`]) or a Tcl-flavoured text block
//! ([`context_bundle_to_text`]).
//!
//! Like [`crate::lint`], this is a *sibling* of the query engine: it walks the
//! parsed model directly and reuses `tcl-irules` (the object-reference walker)
//! and the typed model. It does **not** use the query DSL.

use std::collections::HashSet;

use tcl_irules::extract_irules_object_references;
use tcl_registry::CommandRegistry;

use crate::jsonfmt::json_string;
use crate::model::{
    BigipDataGroup, BigipMonitor, BigipNode, BigipPersistence, BigipPool, BigipProfile, BigipRule,
    BigipSnatPool, ModelObject,
};
use crate::parser::driver::{BigipConfig, Placed};
use crate::range::Range;

// ── Minimal insertion-ordered JSON (with `null`) ─────────────────────
//
// Matches the format of `json.dumps(value, indent=2, ensure_ascii=True)`
// without `sort_keys` (object keys keep insertion order). A sibling of
// `convert::Json`, extended with a `Null` variant for the `None` fields the
// context dict carries.

/// A minimal insertion-ordered JSON value. Object keys preserve insertion
/// order; serialised with `json.dumps(indent=2)`-parity.
enum Json {
    /// JSON `null`.
    Null,
    /// A JSON string.
    Str(String),
    /// A JSON integer.
    Int(i64),
    /// A JSON array.
    Array(Vec<Json>),
    /// A JSON object (insertion-ordered key/value pairs).
    Object(Vec<(String, Json)>),
}

impl Json {
    fn write(&self, out: &mut String, level: usize) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Str(s) => out.push_str(&json_string(s)),
            Json::Int(i) => out.push_str(&i.to_string()),
            Json::Array(items) => {
                if items.is_empty() {
                    out.push_str("[]");
                    return;
                }
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    indent(out, level + 1);
                    item.write(out, level + 1);
                }
                indent(out, level);
                out.push(']');
            }
            Json::Object(entries) => {
                if entries.is_empty() {
                    out.push_str("{}");
                    return;
                }
                out.push('{');
                for (i, (key, val)) in entries.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    indent(out, level + 1);
                    out.push_str(&json_string(key));
                    out.push_str(": ");
                    val.write(out, level + 1);
                }
                indent(out, level);
                out.push('}');
            }
        }
    }

    /// Serialise as `json.dumps(value, indent=2)` would (no trailing newline).
    fn dumps_indent2(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0);
        out
    }
}

fn indent(out: &mut String, level: usize) {
    out.push('\n');
    for _ in 0..level * 2 {
        out.push(' ');
    }
}

/// Render an optional non-empty string as a JSON string, else `null`.
fn str_or_null(s: &str) -> Json {
    if s.is_empty() {
        Json::Null
    } else {
        Json::Str(s.to_owned())
    }
}

// ── The bundle ───────────────────────────────────────────────────────

/// A resolved iRule plus every BIG-IP object it references. Each per-kind
/// list keeps the
/// reference-walk insertion order; `unresolved` keeps first-seen kind order;
/// `source_slices` keeps insertion order.
pub struct IruleContextBundle {
    /// The iRule the bundle was built for.
    pub rule: BigipRule,
    /// Pools referenced by the rule (and, when transitive, none added here).
    pub pools: Vec<BigipPool>,
    /// Data groups referenced by the rule.
    pub data_groups: Vec<BigipDataGroup>,
    /// Persistence profiles referenced by the rule.
    pub persistence: Vec<BigipPersistence>,
    /// SNAT pools referenced by the rule.
    pub snat_pools: Vec<BigipSnatPool>,
    /// Profiles referenced by the rule.
    pub profiles: Vec<BigipProfile>,
    /// Monitors referenced by the rule (or, transitively, via pools).
    pub monitors: Vec<BigipMonitor>,
    /// Nodes referenced by the rule (or, transitively, via pool members).
    pub nodes: Vec<BigipNode>,
    /// Other rules the rule calls.
    pub rules: Vec<BigipRule>,
    /// Reference names that did not resolve, keyed by kind (first-seen order).
    pub unresolved: Vec<(String, Vec<String>)>,
    /// Original config-text slice per referenced object full-path.
    pub source_slices: Vec<(String, String)>,
}

/// An order-preserving, insert-if-absent map: the first insertion fixes the
/// position, later inserts of the same key are
/// no-ops (object values are deterministic per full-path key, so this matches
/// both `d[key] = obj` and `d.setdefault(key, obj)`).
struct OrderedMap<T> {
    entries: Vec<(String, T)>,
    seen: HashSet<String>,
}

impl<T> OrderedMap<T> {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            seen: HashSet::new(),
        }
    }

    fn insert(&mut self, key: String, value: T) {
        if self.seen.insert(key.clone()) {
            self.entries.push((key, value));
        }
    }

    fn into_values(self) -> Vec<T> {
        self.entries.into_iter().map(|(_, v)| v).collect()
    }
}

// ── Reference-kind classification ───────────────────────────────────

/// Map an object reference's kinds to a coarse bucket, or `None` to skip.
/// Shared with the `f5 irule trace` verb.
#[must_use]
pub fn classify_kind(kinds: &[&str]) -> Option<&'static str> {
    if kinds.contains(&"ltm_pool") {
        return Some("pool");
    }
    if kinds
        .iter()
        .any(|&k| k.starts_with("ltm_data_group") || k == "sys_file_data_group")
    {
        return Some("data-group");
    }
    if kinds.iter().any(|&k| k.starts_with("ltm_persistence")) {
        return Some("persistence");
    }
    if kinds.contains(&"ltm_snatpool") {
        return Some("snat-pool");
    }
    if kinds.iter().any(|&k| k.starts_with("ltm_monitor")) {
        return Some("monitor");
    }
    if kinds.iter().any(|&k| k.starts_with("ltm_profile")) {
        return Some("profile");
    }
    if kinds.contains(&"ltm_node") {
        return Some("node");
    }
    if kinds.contains(&"ltm_rule") {
        return Some("rule");
    }
    None
}

// ── Per-kind views over the merged config ────────────────────────────

/// The per-kind object lists the resolver reasons about, in config source
/// order (so the `resolve_name` suffix-match fallback is deterministic).
struct ConfigView<'a> {
    default_partition: &'a str,
    pools: Vec<(&'a str, &'a BigipPool)>,
    data_groups: Vec<(&'a str, &'a BigipDataGroup)>,
    persistence: Vec<(&'a str, &'a BigipPersistence)>,
    snat_pools: Vec<(&'a str, &'a BigipSnatPool)>,
    profiles: Vec<(&'a str, &'a BigipProfile)>,
    monitors: Vec<(&'a str, &'a BigipMonitor)>,
    nodes: Vec<(&'a str, &'a BigipNode)>,
    rules: Vec<(&'a str, &'a BigipRule)>,
}

impl<'a> ConfigView<'a> {
    fn build(config: &'a BigipConfig) -> Self {
        let mut view = ConfigView {
            default_partition: &config.default_partition,
            pools: Vec::new(),
            data_groups: Vec::new(),
            persistence: Vec::new(),
            snat_pools: Vec::new(),
            profiles: Vec::new(),
            monitors: Vec::new(),
            nodes: Vec::new(),
            rules: Vec::new(),
        };
        for placed in &config.objects {
            let Placed {
                full_path, object, ..
            } = placed;
            let key = full_path.as_str();
            match object {
                ModelObject::Pool(p) => view.pools.push((key, p)),
                ModelObject::DataGroup(d) => view.data_groups.push((key, d)),
                ModelObject::Persistence(p) => view.persistence.push((key, p)),
                ModelObject::SnatPool(s) => view.snat_pools.push((key, s)),
                ModelObject::Profile(p) => view.profiles.push((key, p)),
                ModelObject::Monitor(m) => view.monitors.push((key, m)),
                ModelObject::Node(n) => view.nodes.push((key, n)),
                ModelObject::Rule(r) => view.rules.push((key, r)),
                _ => {}
            }
        }
        view
    }
}

/// Resolve a possibly-short `name` to an index into `entries`: exact, then
/// `default_partition`-qualified,
/// then `/Common/`, then a suffix match against any partition.
fn resolve_in<T>(name: &str, entries: &[(&str, &T)], default_partition: &str) -> Option<usize> {
    if let Some(i) = entries.iter().position(|e| e.0 == name) {
        return Some(i);
    }
    if !name.starts_with('/') {
        // `(default_partition or "Common").strip("/")`
        let base = if default_partition.is_empty() {
            "Common"
        } else {
            default_partition
        };
        let partition = base.trim_matches('/');
        if !partition.is_empty() {
            let candidate = format!("/{partition}/{name}");
            if let Some(i) = entries.iter().position(|e| e.0 == candidate) {
                return Some(i);
            }
        }
        if partition != "Common" {
            let candidate = format!("/Common/{name}");
            if let Some(i) = entries.iter().position(|e| e.0 == candidate) {
                return Some(i);
            }
        }
    }
    let suffix = format!("/{name}");
    entries.iter().position(|e| e.0.ends_with(&suffix))
}

// ── Source slicing ──────────────────────────────────────────────────

/// Return the source text that contains an object: `sources[config_origin]`
/// when present, else the sole source when there is exactly one, else `None`.
#[must_use]
pub fn origin_source<'a>(
    sources: &'a [(String, String)],
    config_origin: Option<&str>,
) -> Option<&'a str> {
    if let Some(origin) = config_origin
        && let Some((_, src)) = sources.iter().find(|(k, _)| k == origin)
    {
        return Some(src.as_str());
    }
    if sources.len() == 1 {
        return Some(sources[0].1.as_str());
    }
    None
}

/// Resolve an object reference of `kind` named `name` against `config`,
/// returning the resolved full-path. Shares the `resolve_name`-over-model
/// resolver with [`build_irule_context`]; consumed by the `f5 irule trace`
/// verb.
#[must_use]
pub fn resolve_reference(config: &BigipConfig, kind: &str, name: &str) -> Option<String> {
    let view = ConfigView::build(config);
    let dp = view.default_partition;
    match kind {
        "pool" => resolve_in(name, &view.pools, dp).map(|i| view.pools[i].0.to_owned()),
        "data-group" => {
            resolve_in(name, &view.data_groups, dp).map(|i| view.data_groups[i].0.to_owned())
        }
        "persistence" => {
            resolve_in(name, &view.persistence, dp).map(|i| view.persistence[i].0.to_owned())
        }
        "snat-pool" => {
            resolve_in(name, &view.snat_pools, dp).map(|i| view.snat_pools[i].0.to_owned())
        }
        "monitor" => resolve_in(name, &view.monitors, dp).map(|i| view.monitors[i].0.to_owned()),
        "profile" => resolve_in(name, &view.profiles, dp).map(|i| view.profiles[i].0.to_owned()),
        "node" => resolve_in(name, &view.nodes, dp).map(|i| view.nodes[i].0.to_owned()),
        "rule" => resolve_in(name, &view.rules, dp).map(|i| view.rules[i].0.to_owned()),
        _ => None,
    }
}

/// Slice the original config text for an object's `range`: the line-span
/// `[start.line, end.line]`, right-stripped with
/// a single trailing newline. Returns `None` when out of bounds.
fn slice_for(range: Option<&Range>, source: Option<&str>) -> Option<String> {
    let source = source?;
    let rng = range?;
    // `source.splitlines(keepends=True)` over `\n` boundaries (the line model
    // the parser's range line numbers index into).
    let mut lines: Vec<&str> = Vec::new();
    let mut start = 0usize;
    let bytes = source.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'\n' {
            lines.push(&source[start..=i]);
            start = i + 1;
        }
    }
    if start < source.len() {
        lines.push(&source[start..]);
    }
    let start_line = rng.start.line as usize;
    let end_line = rng.end.line as usize;
    if end_line >= lines.len() {
        return None;
    }
    let chunk: String = lines[start_line..=end_line].concat();
    Some(format!("{}\n", chunk.trim_end()))
}

// ── build_irule_context ──────────────────────────────────────────────

/// Walk `rule` and return every BIG-IP object it references.
///
/// `merged` is the config (or merged union of configs) references resolve
/// against. `transitive` expands one level deeper (pool members → nodes, pool
/// monitor → monitor). `src_text` is the original config text used to record
/// each object's source slice (typically `origin_source(sources, origin)`);
/// pass `None` to skip slicing. `registry` is the iRules command registry used
/// by the object-reference walker.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn build_irule_context(
    rule: &BigipRule,
    merged: &BigipConfig,
    transitive: bool,
    src_text: Option<&str>,
    registry: &CommandRegistry,
) -> IruleContextBundle {
    let view = ConfigView::build(merged);

    let mut pools: OrderedMap<BigipPool> = OrderedMap::new();
    let mut data_groups: OrderedMap<BigipDataGroup> = OrderedMap::new();
    let mut persistence: OrderedMap<BigipPersistence> = OrderedMap::new();
    let mut snat_pools: OrderedMap<BigipSnatPool> = OrderedMap::new();
    let mut profiles: OrderedMap<BigipProfile> = OrderedMap::new();
    let mut monitors: OrderedMap<BigipMonitor> = OrderedMap::new();
    let mut nodes: OrderedMap<BigipNode> = OrderedMap::new();
    let mut rules: OrderedMap<BigipRule> = OrderedMap::new();
    let mut unresolved: OrderedMap<Vec<String>> = OrderedMap::new();
    let mut source_slices: Vec<(String, String)> = Vec::new();
    let mut slice_seen: HashSet<String> = HashSet::new();

    let mut record_slice = |full_path: &str, range: Option<&Range>| {
        if src_text.is_none() {
            return;
        }
        if full_path.is_empty() {
            return;
        }
        if let Some(chunk) = slice_for(range, src_text)
            && slice_seen.insert(full_path.to_owned())
        {
            source_slices.push((full_path.to_owned(), chunk));
        }
    };

    // The rule's own slice.
    record_slice(&rule.full_path, rule.range.as_ref());

    let mut push_unresolved = |kind: &str, name: &str| {
        if let Some((_, names)) = unresolved.entries.iter_mut().find(|(k, _)| k == kind) {
            names.push(name.to_owned());
        } else {
            unresolved.insert(kind.to_owned(), vec![name.to_owned()]);
        }
    };

    let mut seen: HashSet<(&str, String)> = HashSet::new();
    for reference in extract_irules_object_references(&rule.source, None, registry) {
        let Some(kind) = classify_kind(&reference.kinds) else {
            continue;
        };
        if !seen.insert((kind, reference.name.clone())) {
            continue;
        }
        let name = reference.name.as_str();
        match kind {
            "pool" => {
                if let Some(i) = resolve_in(name, &view.pools, view.default_partition) {
                    let (path, obj) = view.pools[i];
                    pools.insert(path.to_owned(), obj.clone());
                    record_slice(path, obj.range.as_ref());
                } else {
                    push_unresolved(kind, name);
                }
            }
            "data-group" => {
                if let Some(i) = resolve_in(name, &view.data_groups, view.default_partition) {
                    let (path, obj) = view.data_groups[i];
                    data_groups.insert(path.to_owned(), obj.clone());
                    record_slice(path, obj.range.as_ref());
                } else {
                    push_unresolved(kind, name);
                }
            }
            "persistence" => {
                if let Some(i) = resolve_in(name, &view.persistence, view.default_partition) {
                    let (path, obj) = view.persistence[i];
                    persistence.insert(path.to_owned(), obj.clone());
                    record_slice(path, obj.range.as_ref());
                } else {
                    push_unresolved(kind, name);
                }
            }
            "snat-pool" => {
                if let Some(i) = resolve_in(name, &view.snat_pools, view.default_partition) {
                    let (path, obj) = view.snat_pools[i];
                    snat_pools.insert(path.to_owned(), obj.clone());
                    record_slice(path, obj.range.as_ref());
                } else {
                    push_unresolved(kind, name);
                }
            }
            "profile" => {
                if let Some(i) = resolve_in(name, &view.profiles, view.default_partition) {
                    let (path, obj) = view.profiles[i];
                    profiles.insert(path.to_owned(), obj.clone());
                    record_slice(path, obj.range.as_ref());
                } else {
                    push_unresolved(kind, name);
                }
            }
            "monitor" => {
                if let Some(i) = resolve_in(name, &view.monitors, view.default_partition) {
                    let (path, obj) = view.monitors[i];
                    monitors.insert(path.to_owned(), obj.clone());
                    record_slice(path, obj.range.as_ref());
                } else {
                    push_unresolved(kind, name);
                }
            }
            "node" => {
                if let Some(i) = resolve_in(name, &view.nodes, view.default_partition) {
                    let (path, obj) = view.nodes[i];
                    nodes.insert(path.to_owned(), obj.clone());
                    record_slice(path, obj.range.as_ref());
                } else {
                    push_unresolved(kind, name);
                }
            }
            "rule" => {
                if let Some(i) = resolve_in(name, &view.rules, view.default_partition) {
                    let (path, obj) = view.rules[i];
                    // Recursive / self-reference: the rule body is already part
                    // of the bundle; don't record it (and don't mark unresolved).
                    if path == rule.full_path {
                        continue;
                    }
                    rules.insert(path.to_owned(), obj.clone());
                    record_slice(path, obj.range.as_ref());
                } else {
                    push_unresolved(kind, name);
                }
            }
            _ => {}
        }
    }

    if transitive {
        // Pool members → nodes; pool monitor → monitor object.
        let referenced_pools: Vec<BigipPool> =
            pools.entries.iter().map(|(_, p)| p.clone()).collect();
        for pool in &referenced_pools {
            for member in member_iter(pool) {
                let node_name = member
                    .name
                    .rsplit_once(':')
                    .map_or(member.name.as_str(), |(n, _)| n);
                if let Some(i) = resolve_in(node_name, &view.nodes, view.default_partition) {
                    let (path, obj) = view.nodes[i];
                    nodes.insert(path.to_owned(), obj.clone());
                    record_slice(path, obj.range.as_ref());
                }
            }
            if !pool.monitor.is_empty() {
                let first = pool.monitor.split(' ').next().unwrap_or("");
                if let Some(i) = resolve_in(first, &view.monitors, view.default_partition) {
                    let (path, obj) = view.monitors[i];
                    monitors.insert(path.to_owned(), obj.clone());
                    record_slice(path, obj.range.as_ref());
                }
            }
        }
    }

    IruleContextBundle {
        rule: rule.clone(),
        pools: pools.into_values(),
        data_groups: data_groups.into_values(),
        persistence: persistence.into_values(),
        snat_pools: snat_pools.into_values(),
        profiles: profiles.into_values(),
        monitors: monitors.into_values(),
        nodes: nodes.into_values(),
        rules: rules.into_values(),
        unresolved: unresolved.entries,
        source_slices,
    }
}

/// Iterate a pool's typed members.
fn member_iter(pool: &BigipPool) -> impl Iterator<Item = &crate::model::BigipPoolMember> {
    pool.members.items.iter().filter_map(|item| {
        if let crate::value::ListItemValue::PoolMember(m) = &item.value {
            Some(m)
        } else {
            None
        }
    })
}

// ── JSON rendering ──────────────────────────────────────────────────

fn summarise_pool(pool: &BigipPool) -> Json {
    let members: Vec<Json> = member_iter(pool)
        .map(|m| {
            Json::Object(vec![
                ("name".to_owned(), Json::Str(m.name.clone())),
                (
                    "address".to_owned(),
                    m.address
                        .as_ref()
                        .map_or(Json::Null, |a| Json::Str(a.to_string())),
                ),
                (
                    "port".to_owned(),
                    if m.port == 0 {
                        Json::Null
                    } else {
                        Json::Int(m.port)
                    },
                ),
                ("monitor".to_owned(), str_or_null(&m.monitor)),
            ])
        })
        .collect();
    Json::Object(vec![
        ("fullPath".to_owned(), Json::Str(pool.full_path.clone())),
        ("monitor".to_owned(), str_or_null(&pool.monitor)),
        (
            "loadBalancingMode".to_owned(),
            str_or_null(&pool.load_balancing_mode),
        ),
        ("members".to_owned(), Json::Array(members)),
    ])
}

fn summarise_data_group(dg: &BigipDataGroup) -> Json {
    Json::Object(vec![
        ("fullPath".to_owned(), Json::Str(dg.full_path.clone())),
        (
            "kind".to_owned(),
            Json::Str(dg.kind.py_name().to_ascii_lowercase()),
        ),
        ("valueType".to_owned(), str_or_null(&dg.value_type)),
        (
            "recordCount".to_owned(),
            Json::Int(i64::try_from(dg.records.len()).unwrap_or(i64::MAX)),
        ),
        (
            "records".to_owned(),
            Json::Array(dg.records.iter().map(|r| Json::Str(r.clone())).collect()),
        ),
    ])
}

fn summarise_persistence(p: &BigipPersistence) -> Json {
    Json::Object(vec![
        ("fullPath".to_owned(), Json::Str(p.full_path.clone())),
        ("type".to_owned(), str_or_null(&p.persistence_type)),
    ])
}

fn summarise_snat_pool(s: &BigipSnatPool) -> Json {
    Json::Object(vec![
        ("fullPath".to_owned(), Json::Str(s.full_path.clone())),
        (
            "members".to_owned(),
            Json::Array(s.members.paths().into_iter().map(Json::Str).collect()),
        ),
    ])
}

fn summarise_profile(p: &BigipProfile) -> Json {
    Json::Object(vec![
        ("fullPath".to_owned(), Json::Str(p.full_path.clone())),
        (
            "type".to_owned(),
            Json::Str(p.profile_type.py_name().to_ascii_lowercase()),
        ),
    ])
}

fn summarise_monitor(m: &BigipMonitor) -> Json {
    Json::Object(vec![
        ("fullPath".to_owned(), Json::Str(m.full_path.clone())),
        ("type".to_owned(), str_or_null(&m.monitor_type)),
    ])
}

fn summarise_node(n: &BigipNode) -> Json {
    Json::Object(vec![
        ("fullPath".to_owned(), Json::Str(n.full_path.clone())),
        (
            "address".to_owned(),
            n.address
                .as_ref()
                .map_or(Json::Null, |a| Json::Str(a.to_string())),
        ),
    ])
}

fn rule_dict(r: &BigipRule) -> Json {
    Json::Object(vec![
        ("fullPath".to_owned(), Json::Str(r.full_path.clone())),
        ("name".to_owned(), Json::Str(r.name.clone())),
        ("source".to_owned(), Json::Str(r.source.clone())),
    ])
}

/// Sorted-unique copy of `names`.
fn sorted_unique(names: &[String]) -> Vec<String> {
    let mut v: Vec<String> = names
        .iter()
        .cloned()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    v.sort();
    v
}

fn bundle_to_json(bundle: &IruleContextBundle) -> Json {
    let arr = |items: Vec<Json>| Json::Array(items);
    Json::Object(vec![
        ("rule".to_owned(), rule_dict(&bundle.rule)),
        (
            "pools".to_owned(),
            arr(bundle.pools.iter().map(summarise_pool).collect()),
        ),
        (
            "dataGroups".to_owned(),
            arr(bundle
                .data_groups
                .iter()
                .map(summarise_data_group)
                .collect()),
        ),
        (
            "persistence".to_owned(),
            arr(bundle
                .persistence
                .iter()
                .map(summarise_persistence)
                .collect()),
        ),
        (
            "snatPools".to_owned(),
            arr(bundle.snat_pools.iter().map(summarise_snat_pool).collect()),
        ),
        (
            "profiles".to_owned(),
            arr(bundle.profiles.iter().map(summarise_profile).collect()),
        ),
        (
            "monitors".to_owned(),
            arr(bundle.monitors.iter().map(summarise_monitor).collect()),
        ),
        (
            "nodes".to_owned(),
            arr(bundle.nodes.iter().map(summarise_node).collect()),
        ),
        (
            "rules".to_owned(),
            arr(bundle.rules.iter().map(rule_dict).collect()),
        ),
        (
            "unresolved".to_owned(),
            Json::Object(
                bundle
                    .unresolved
                    .iter()
                    .map(|(kind, names)| {
                        (
                            kind.clone(),
                            Json::Array(sorted_unique(names).into_iter().map(Json::Str).collect()),
                        )
                    })
                    .collect(),
            ),
        ),
        (
            "sourceSlices".to_owned(),
            Json::Object(
                bundle
                    .source_slices
                    .iter()
                    .map(|(path, slice)| (path.clone(), Json::Str(slice.clone())))
                    .collect(),
            ),
        ),
    ])
}

/// Serialise a single bundle as `json.dumps(context_bundle_to_dict(bundle),
/// indent=2)` (no trailing newline).
#[must_use]
pub fn context_bundle_to_json(bundle: &IruleContextBundle) -> String {
    bundle_to_json(bundle).dumps_indent2()
}

/// Serialise many bundles as `json.dumps({"bundles": [...]}, indent=2)` (no
/// trailing newline) — the stdout/single-file multi-bundle form.
#[must_use]
pub fn bundles_to_json(bundles: &[IruleContextBundle]) -> String {
    Json::Object(vec![(
        "bundles".to_owned(),
        Json::Array(bundles.iter().map(bundle_to_json).collect()),
    )])
    .dumps_indent2()
}

// ── Text rendering ──────────────────────────────────────────────────

fn render_pool_text(pool: &BigipPool) -> String {
    let mut lines = vec![format!("ltm pool {} {{", pool.full_path)];
    if !pool.load_balancing_mode.is_empty() {
        lines.push(format!(
            "    load-balancing-mode {}",
            pool.load_balancing_mode
        ));
    }
    if pool
        .members
        .items
        .iter()
        .any(|item| matches!(item.value, crate::value::ListItemValue::PoolMember(_)))
    {
        lines.push("    members {".to_owned());
        for m in member_iter(pool) {
            lines.push(format!("        {} {{", m.name));
            if let Some(a) = m.address.as_ref() {
                lines.push(format!("            address {a}"));
            }
            if !m.monitor.is_empty() {
                lines.push(format!("            monitor {}", m.monitor));
            }
            lines.push("        }".to_owned());
        }
        lines.push("    }".to_owned());
    }
    if !pool.monitor.is_empty() {
        lines.push(format!("    monitor {}", pool.monitor));
    }
    lines.push("}".to_owned());
    format!("{}\n", lines.join("\n"))
}

fn render_data_group_text(dg: &BigipDataGroup) -> String {
    let kind = dg.kind.py_name().to_ascii_lowercase();
    let mut lines = vec![format!("ltm data-group {kind} {} {{", dg.full_path)];
    if !dg.value_type.is_empty() {
        lines.push(format!("    type {}", dg.value_type));
    }
    if !dg.records.is_empty() {
        lines.push("    records {".to_owned());
        for record in &dg.records {
            lines.push(format!("        {record} {{ }}"));
        }
        lines.push("    }".to_owned());
    }
    lines.push("}".to_owned());
    format!("{}\n", lines.join("\n"))
}

fn render_persistence_text(p: &BigipPersistence) -> String {
    let ty = if p.persistence_type.is_empty() {
        "<unknown>"
    } else {
        &p.persistence_type
    };
    format!("ltm persistence {ty} {} {{ }}\n", p.full_path)
}

fn render_snat_pool_text(s: &BigipSnatPool) -> String {
    let members = s.members.paths();
    let mut lines = vec![format!("ltm snatpool {} {{", s.full_path)];
    if !members.is_empty() {
        lines.push("    members {".to_owned());
        for member in &members {
            lines.push(format!("        {member}"));
        }
        lines.push("    }".to_owned());
    }
    lines.push("}".to_owned());
    format!("{}\n", lines.join("\n"))
}

fn render_profile_text(p: &BigipProfile) -> String {
    format!(
        "ltm profile {} {} {{ }}\n",
        p.profile_type.py_name().to_ascii_lowercase(),
        p.full_path
    )
}

fn render_monitor_text(m: &BigipMonitor) -> String {
    let ty = if m.monitor_type.is_empty() {
        "<unknown>"
    } else {
        &m.monitor_type
    };
    format!("ltm monitor {ty} {} {{ }}\n", m.full_path)
}

fn render_node_text(n: &BigipNode) -> String {
    let addr = n
        .address
        .as_ref()
        .map_or(String::new(), ToString::to_string);
    format!("ltm node {} {{ address {addr} }}\n", n.full_path)
}

/// Prefer a real source slice; fall back to the synthetic stanza.
fn render_object_text(bundle: &IruleContextBundle, full_path: &str, fallback: String) -> String {
    bundle
        .source_slices
        .iter()
        .find(|(path, _)| path == full_path)
        .map_or(fallback, |(_, slice)| slice.clone())
}

/// Render `bundle` as a single Tcl-flavoured text block.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn context_bundle_to_text(bundle: &IruleContextBundle) -> String {
    let mut parts: Vec<String> = Vec::new();

    parts.push(format!("# ===== iRule {} =====", bundle.rule.full_path));
    if let Some((_, slice)) = bundle
        .source_slices
        .iter()
        .find(|(path, _)| *path == bundle.rule.full_path)
    {
        parts.push(slice.trim_end().to_owned());
    } else {
        parts.push(format!("ltm rule {} {{", bundle.rule.full_path));
        parts.push(bundle.rule.source.trim_end().to_owned());
        parts.push("}".to_owned());
    }

    // (kind label, entries) in the fixed Python order.
    let mut sections: Vec<(&str, Vec<(String, String)>)> = Vec::new();
    sections.push((
        "pool",
        bundle
            .pools
            .iter()
            .map(|p| {
                (
                    p.full_path.clone(),
                    render_object_text(bundle, &p.full_path, render_pool_text(p)),
                )
            })
            .collect(),
    ));
    sections.push((
        "data-group",
        bundle
            .data_groups
            .iter()
            .map(|d| {
                (
                    d.full_path.clone(),
                    render_object_text(bundle, &d.full_path, render_data_group_text(d)),
                )
            })
            .collect(),
    ));
    sections.push((
        "persistence",
        bundle
            .persistence
            .iter()
            .map(|p| {
                (
                    p.full_path.clone(),
                    render_object_text(bundle, &p.full_path, render_persistence_text(p)),
                )
            })
            .collect(),
    ));
    sections.push((
        "snat-pool",
        bundle
            .snat_pools
            .iter()
            .map(|s| {
                (
                    s.full_path.clone(),
                    render_object_text(bundle, &s.full_path, render_snat_pool_text(s)),
                )
            })
            .collect(),
    ));
    sections.push((
        "profile",
        bundle
            .profiles
            .iter()
            .map(|p| {
                (
                    p.full_path.clone(),
                    render_object_text(bundle, &p.full_path, render_profile_text(p)),
                )
            })
            .collect(),
    ));
    sections.push((
        "monitor",
        bundle
            .monitors
            .iter()
            .map(|m| {
                (
                    m.full_path.clone(),
                    render_object_text(bundle, &m.full_path, render_monitor_text(m)),
                )
            })
            .collect(),
    ));
    sections.push((
        "node",
        bundle
            .nodes
            .iter()
            .map(|n| {
                (
                    n.full_path.clone(),
                    render_object_text(bundle, &n.full_path, render_node_text(n)),
                )
            })
            .collect(),
    ));
    sections.push((
        "rule",
        bundle
            .rules
            .iter()
            .map(|r| {
                let fallback =
                    format!("ltm rule {} {{\n{}\n}}\n", r.full_path, r.source.trim_end());
                (
                    r.full_path.clone(),
                    render_object_text(bundle, &r.full_path, fallback),
                )
            })
            .collect(),
    ));

    for (kind, entries) in &sections {
        for (full_path, text) in entries {
            parts.push(format!("\n# ===== referenced {kind} {full_path} ====="));
            parts.push(text.trim_end().to_owned());
        }
    }

    if !bundle.unresolved.is_empty() {
        parts.push("\n# ===== unresolved =====".to_owned());
        let mut kinds: Vec<&String> = bundle.unresolved.iter().map(|(k, _)| k).collect();
        kinds.sort();
        for kind in kinds {
            let names = bundle
                .unresolved
                .iter()
                .find(|(k, _)| k == kind)
                .map_or(&[][..], |(_, n)| n.as_slice());
            for name in sorted_unique(names) {
                parts.push(format!("# {kind}: {name}"));
            }
        }
    }

    format!("{}\n", parts.join("\n"))
}
