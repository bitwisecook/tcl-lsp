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

// Minimal insertion-ordered JSON (with `null`)
//
// Matches a 2-space-indented, ASCII-escaped JSON encoding that keeps
// object keys in insertion order (not sorted). A sibling of
// `convert::Json`, extended with a `Null` variant for the `None` fields the
// context dict carries.

/// A minimal insertion-ordered JSON value. Object keys preserve insertion
/// order; serialised with 2-space indentation.
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

    /// Serialise as 2-space-indented JSON (no trailing newline).
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

// The bundle

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

impl<T> Default for OrderedMap<T> {
    fn default() -> Self {
        Self::new()
    }
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

// Reference-kind classification

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

// Per-kind views over the merged config

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

// Source slicing

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

// build_irule_context

/// Walk `rule` and return every BIG-IP object it references.
///
/// `merged` is the config (or merged union of configs) references resolve
/// against. `transitive` expands one level deeper (pool members → nodes, pool
/// monitor → monitor). `src_text` is the original config text used to record
/// each object's source slice (typically `origin_source(sources, origin)`);
/// pass `None` to skip slicing. `registry` is the iRules command registry used
/// by the object-reference walker.
#[must_use]
pub fn build_irule_context(
    rule: &BigipRule,
    merged: &BigipConfig,
    transitive: bool,
    src_text: Option<&str>,
    registry: &CommandRegistry,
) -> IruleContextBundle {
    let view = ConfigView::build(merged);
    let mut builder = ContextBuilder::new(src_text);

    // The rule's own slice.
    builder.record_slice(&rule.full_path, rule.range.as_ref());

    let mut seen: HashSet<(&str, String)> = HashSet::new();
    for reference in extract_irules_object_references(&rule.source, None, registry) {
        let Some(kind) = classify_kind(&reference.kinds) else {
            continue;
        };
        if !seen.insert((kind, reference.name.clone())) {
            continue;
        }
        builder.collect_reference(kind, reference.name.as_str(), &view, rule);
    }

    if transitive {
        builder.collect_transitive(&view);
    }

    builder.finish(rule)
}

/// Anything carrying an optional source `Range`, so [`ContextBuilder`] can
/// record a slice for it generically.
trait HasRange {
    fn range(&self) -> Option<&Range>;
}

macro_rules! impl_has_range {
    ($($ty:ty),* $(,)?) => {
        $(impl HasRange for $ty {
            fn range(&self) -> Option<&Range> {
                self.range.as_ref()
            }
        })*
    };
}
impl_has_range!(
    BigipPool,
    BigipDataGroup,
    BigipPersistence,
    BigipSnatPool,
    BigipProfile,
    BigipMonitor,
    BigipNode,
    BigipRule,
);

/// Accumulates the resolved per-kind object maps, unresolved names, and source
/// slices while [`build_irule_context`] walks a rule's references.
struct ContextBuilder<'a> {
    src_text: Option<&'a str>,
    pools: OrderedMap<BigipPool>,
    data_groups: OrderedMap<BigipDataGroup>,
    persistence: OrderedMap<BigipPersistence>,
    snat_pools: OrderedMap<BigipSnatPool>,
    profiles: OrderedMap<BigipProfile>,
    monitors: OrderedMap<BigipMonitor>,
    nodes: OrderedMap<BigipNode>,
    rules: OrderedMap<BigipRule>,
    unresolved: OrderedMap<Vec<String>>,
    source_slices: Vec<(String, String)>,
    slice_seen: HashSet<String>,
}

impl<'a> ContextBuilder<'a> {
    fn new(src_text: Option<&'a str>) -> Self {
        Self {
            src_text,
            pools: OrderedMap::new(),
            data_groups: OrderedMap::new(),
            persistence: OrderedMap::new(),
            snat_pools: OrderedMap::new(),
            profiles: OrderedMap::new(),
            monitors: OrderedMap::new(),
            nodes: OrderedMap::new(),
            rules: OrderedMap::new(),
            unresolved: OrderedMap::new(),
            source_slices: Vec::new(),
            slice_seen: HashSet::new(),
        }
    }

    fn record_slice(&mut self, full_path: &str, range: Option<&Range>) {
        if self.src_text.is_none() || full_path.is_empty() {
            return;
        }
        if let Some(chunk) = slice_for(range, self.src_text)
            && self.slice_seen.insert(full_path.to_owned())
        {
            self.source_slices.push((full_path.to_owned(), chunk));
        }
    }

    fn push_unresolved(&mut self, kind: &str, name: &str) {
        if let Some((_, names)) = self.unresolved.entries.iter_mut().find(|(k, _)| k == kind) {
            names.push(name.to_owned());
        } else {
            self.unresolved
                .insert(kind.to_owned(), vec![name.to_owned()]);
        }
    }

    /// Resolve `name` in `list`; on hit insert a clone into `map` and record its
    /// slice, returning the matched full-path. On miss, return `None`.
    fn resolve_into<T: Clone + HasRange>(
        &mut self,
        map: &mut OrderedMap<T>,
        list: &[(&str, &T)],
        name: &str,
        partition: &str,
    ) -> Option<String> {
        let i = resolve_in(name, list, partition)?;
        let (path, obj) = list[i];
        map.insert(path.to_owned(), obj.clone());
        let range = obj.range().copied();
        self.record_slice(path, range.as_ref());
        Some(path.to_owned())
    }

    /// Resolve a single classified reference into the matching kind map (or mark
    /// it unresolved). `rule` is the rule being built, used to skip self-refs.
    fn collect_reference(&mut self, kind: &str, name: &str, view: &ConfigView, rule: &BigipRule) {
        let part = view.default_partition;
        let hit = match kind {
            "pool" => {
                let mut m = std::mem::take(&mut self.pools);
                let r = self.resolve_into(&mut m, &view.pools, name, part);
                self.pools = m;
                r
            }
            "data-group" => {
                let mut m = std::mem::take(&mut self.data_groups);
                let r = self.resolve_into(&mut m, &view.data_groups, name, part);
                self.data_groups = m;
                r
            }
            "persistence" => {
                let mut m = std::mem::take(&mut self.persistence);
                let r = self.resolve_into(&mut m, &view.persistence, name, part);
                self.persistence = m;
                r
            }
            "snat-pool" => {
                let mut m = std::mem::take(&mut self.snat_pools);
                let r = self.resolve_into(&mut m, &view.snat_pools, name, part);
                self.snat_pools = m;
                r
            }
            "profile" => {
                let mut m = std::mem::take(&mut self.profiles);
                let r = self.resolve_into(&mut m, &view.profiles, name, part);
                self.profiles = m;
                r
            }
            "monitor" => {
                let mut m = std::mem::take(&mut self.monitors);
                let r = self.resolve_into(&mut m, &view.monitors, name, part);
                self.monitors = m;
                r
            }
            "node" => {
                let mut m = std::mem::take(&mut self.nodes);
                let r = self.resolve_into(&mut m, &view.nodes, name, part);
                self.nodes = m;
                r
            }
            "rule" => return self.collect_rule_reference(name, view, rule),
            _ => return,
        };
        if hit.is_none() {
            self.push_unresolved(kind, name);
        }
    }

    /// Rule references need a self-reference guard before insertion.
    fn collect_rule_reference(&mut self, name: &str, view: &ConfigView, rule: &BigipRule) {
        let Some(i) = resolve_in(name, &view.rules, view.default_partition) else {
            self.push_unresolved("rule", name);
            return;
        };
        let (path, obj) = view.rules[i];
        // Recursive / self-reference: the rule body is already part of the
        // bundle; don't record it (and don't mark unresolved).
        if path == rule.full_path {
            return;
        }
        self.rules.insert(path.to_owned(), obj.clone());
        self.record_slice(path, obj.range.as_ref());
    }

    /// Expand one level deeper: pool members → nodes; pool monitor → monitor.
    fn collect_transitive(&mut self, view: &ConfigView) {
        let referenced_pools: Vec<BigipPool> =
            self.pools.entries.iter().map(|(_, p)| p.clone()).collect();
        let part = view.default_partition;
        for pool in &referenced_pools {
            for member in member_iter(pool) {
                let node_name = member
                    .name
                    .rsplit_once(':')
                    .map_or(member.name.as_str(), |(n, _)| n);
                let mut m = std::mem::take(&mut self.nodes);
                let _ = self.resolve_into(&mut m, &view.nodes, node_name, part);
                self.nodes = m;
            }
            if !pool.monitor.is_empty() {
                let first = pool.monitor.split(' ').next().unwrap_or("");
                let mut m = std::mem::take(&mut self.monitors);
                let _ = self.resolve_into(&mut m, &view.monitors, first, part);
                self.monitors = m;
            }
        }
    }

    fn finish(self, rule: &BigipRule) -> IruleContextBundle {
        IruleContextBundle {
            rule: rule.clone(),
            pools: self.pools.into_values(),
            data_groups: self.data_groups.into_values(),
            persistence: self.persistence.into_values(),
            snat_pools: self.snat_pools.into_values(),
            profiles: self.profiles.into_values(),
            monitors: self.monitors.into_values(),
            nodes: self.nodes.into_values(),
            rules: self.rules.into_values(),
            unresolved: self.unresolved.entries,
            source_slices: self.source_slices,
        }
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

// JSON rendering

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

/// Serialise a single bundle as 2-space-indented JSON (no trailing newline).
#[must_use]
pub fn context_bundle_to_json(bundle: &IruleContextBundle) -> String {
    bundle_to_json(bundle).dumps_indent2()
}

/// Serialise many bundles as a 2-space-indented `{"bundles": [...]}` object (no
/// trailing newline) — the stdout/single-file multi-bundle form.
#[must_use]
pub fn bundles_to_json(bundles: &[IruleContextBundle]) -> String {
    Json::Object(vec![(
        "bundles".to_owned(),
        Json::Array(bundles.iter().map(bundle_to_json).collect()),
    )])
    .dumps_indent2()
}

// Text rendering

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

/// Build the `(full_path, rendered_text)` entries for one referenced-object
/// section: each object prefers its real source slice, falling back to the
/// synthetic stanza produced by `fallback`.
fn section_entries<T>(
    bundle: &IruleContextBundle,
    objects: &[T],
    full_path: impl Fn(&T) -> &str,
    fallback: impl Fn(&T) -> String,
) -> Vec<(String, String)> {
    objects
        .iter()
        .map(|obj| {
            let path = full_path(obj);
            (
                path.to_owned(),
                render_object_text(bundle, path, fallback(obj)),
            )
        })
        .collect()
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

    // (kind label, entries) in the fixed order.
    for (kind, entries) in &referenced_sections(bundle) {
        for (full_path, text) in entries {
            parts.push(format!("\n# ===== referenced {kind} {full_path} ====="));
            parts.push(text.trim_end().to_owned());
        }
    }

    if !bundle.unresolved.is_empty() {
        push_unresolved_section(bundle, &mut parts);
    }

    format!("{}\n", parts.join("\n"))
}

/// The `(kind label, entries)` referenced-object sections in fixed order.
fn referenced_sections(bundle: &IruleContextBundle) -> Vec<(&'static str, Vec<(String, String)>)> {
    vec![
        (
            "pool",
            section_entries(
                bundle,
                &bundle.pools,
                |p| p.full_path.as_str(),
                render_pool_text,
            ),
        ),
        (
            "data-group",
            section_entries(
                bundle,
                &bundle.data_groups,
                |d| d.full_path.as_str(),
                render_data_group_text,
            ),
        ),
        (
            "persistence",
            section_entries(
                bundle,
                &bundle.persistence,
                |p| p.full_path.as_str(),
                render_persistence_text,
            ),
        ),
        (
            "snat-pool",
            section_entries(
                bundle,
                &bundle.snat_pools,
                |s| s.full_path.as_str(),
                render_snat_pool_text,
            ),
        ),
        (
            "profile",
            section_entries(
                bundle,
                &bundle.profiles,
                |p| p.full_path.as_str(),
                render_profile_text,
            ),
        ),
        (
            "monitor",
            section_entries(
                bundle,
                &bundle.monitors,
                |m| m.full_path.as_str(),
                render_monitor_text,
            ),
        ),
        (
            "node",
            section_entries(
                bundle,
                &bundle.nodes,
                |n| n.full_path.as_str(),
                render_node_text,
            ),
        ),
        (
            "rule",
            section_entries(
                bundle,
                &bundle.rules,
                |r| r.full_path.as_str(),
                |r| format!("ltm rule {} {{\n{}\n}}\n", r.full_path, r.source.trim_end()),
            ),
        ),
    ]
}

/// Append the `# ===== unresolved =====` block (kinds sorted, names unique).
fn push_unresolved_section(bundle: &IruleContextBundle, parts: &mut Vec<String>) {
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
