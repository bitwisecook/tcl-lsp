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

//! Lint engine for `f5 validate` (alias `lint`).
//!
//! This is a *sibling* of the query engine: it walks the parsed
//! [`BigipConfig`] model directly (it does **not** use the query DSL) and
//! reuses the `tcl-bigip` model, [`crate::stats::is_root_kind`],
//! `tcl-irules` (the iRule object-reference walker), and `tcl-registry` (the
//! known-event set). Eight built-in rules run in a fixed registration order;
//! [`run_lint`] yields findings in that order (the per-severity / rule-id /
//! full-path sort lives in the text formatter).

use std::collections::HashSet;

use tcl_dialect::DialectSet;
use tcl_irules::extract_irules_object_references;
use tcl_registry::CommandRegistry;
use tcl_registry::events::EventRegistry;

use crate::model::ModelObject;
use crate::parser::driver::{BigipConfig, Placed};

/// The lint rule categories, in fixed order.
pub const CATEGORIES: [&str; 2] = ["config", "irule"];
/// The finding severities, in fixed order.
pub const SEVERITIES: [&str; 3] = ["error", "warning", "info"];

/// One lint finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Rule id (e.g. `"orphan-monitor"`, `"irule-missing-pool"`).
    pub rule_id: String,
    /// `"error"`, `"warning"`, or `"info"`.
    pub severity: &'static str,
    /// `"config"` or `"irule"`.
    pub category: &'static str,
    /// The object full-path the finding is about.
    pub full_path: String,
    /// Human-readable message.
    pub message: String,
}

// Per-kind views over the model
//
// Config holds one map per object kind, but the Rust model stores everything
// in `config.objects` tagged with its `table_name`. `KindMap` provides those
// per-kind map semantics: iteration in first-seen order, last-wins on a
// full_path collision.

/// An order-preserving, last-wins map keyed by `full_path` — the per-kind
/// view, materialised over the typed model objects.
pub(crate) struct KindMap<'a, T> {
    /// `(full_path, value)` in first-seen order.
    entries: Vec<(&'a str, &'a T)>,
    /// `full_path -> index into entries`.
    index: std::collections::HashMap<&'a str, usize>,
}

impl<T> Default for KindMap<'_, T> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            index: std::collections::HashMap::new(),
        }
    }
}

impl<'a, T> KindMap<'a, T> {
    /// Insert / update with map-assignment semantics: a new key appends
    /// (keeping first-seen position), an existing key updates the value in
    /// place.
    fn insert(&mut self, key: &'a str, value: &'a T) {
        if let Some(&i) = self.index.get(key) {
            self.entries[i].1 = value;
        } else {
            self.index.insert(key, self.entries.len());
            self.entries.push((key, value));
        }
    }

    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn contains_key(&self, key: &str) -> bool {
        self.index.contains_key(key)
    }

    /// Look up the value for `key`.
    pub(crate) fn get(&self, key: &str) -> Option<&'a T> {
        self.index.get(key).map(|&i| self.entries[i].1)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&'a str, &'a T)> + '_ {
        self.entries.iter().copied()
    }
}

/// The per-kind model views the rules reason about, grouped from
/// `config.objects` by `ModelObject` variant.
pub(crate) struct ModelView<'a> {
    pub(crate) default_partition: &'a str,
    pub(crate) pools: KindMap<'a, crate::model::BigipPool>,
    pub(crate) monitors: KindMap<'a, crate::model::BigipMonitor>,
    pub(crate) virtual_servers: KindMap<'a, crate::model::BigipVirtualServer>,
    pub(crate) nodes: KindMap<'a, crate::model::BigipNode>,
    pub(crate) profiles: KindMap<'a, crate::model::BigipProfile>,
    pub(crate) snat_pools: KindMap<'a, crate::model::BigipSnatPool>,
    pub(crate) persistence: KindMap<'a, crate::model::BigipPersistence>,
    pub(crate) rules: KindMap<'a, crate::model::BigipRule>,
    pub(crate) data_groups: KindMap<'a, crate::model::BigipDataGroup>,
    pub(crate) policies: KindMap<'a, crate::model::BigipPolicy>,
    pub(crate) generic_object_keys: &'a [(String, crate::model::BigipGenericObject)],
}

impl<'a> ModelView<'a> {
    /// Build the per-kind views for one config's typed objects, in source
    /// order. Multiple configs are pre-merged into one [`BigipConfig`] by
    /// [`merge_configs`], so a single pass over `config.objects` suffices.
    pub(crate) fn build(config: &'a BigipConfig) -> Self {
        let mut view = ModelView {
            default_partition: &config.default_partition,
            pools: KindMap::default(),
            monitors: KindMap::default(),
            virtual_servers: KindMap::default(),
            nodes: KindMap::default(),
            profiles: KindMap::default(),
            snat_pools: KindMap::default(),
            persistence: KindMap::default(),
            rules: KindMap::default(),
            data_groups: KindMap::default(),
            policies: KindMap::default(),
            generic_object_keys: &config.generic_objects,
        };
        for placed in &config.objects {
            let Placed {
                full_path, object, ..
            } = placed;
            let key = full_path.as_str();
            match object {
                ModelObject::Pool(p) => view.pools.insert(key, p),
                ModelObject::Monitor(m) => view.monitors.insert(key, m),
                ModelObject::VirtualServer(v) => view.virtual_servers.insert(key, v),
                ModelObject::Node(n) => view.nodes.insert(key, n),
                ModelObject::Profile(p) => view.profiles.insert(key, p),
                ModelObject::SnatPool(s) => view.snat_pools.insert(key, s),
                ModelObject::Persistence(p) => view.persistence.insert(key, p),
                ModelObject::Rule(r) => view.rules.insert(key, r),
                ModelObject::DataGroup(d) => view.data_groups.insert(key, d),
                ModelObject::Policy(p) => view.policies.insert(key, p),
                _ => {}
            }
        }
        view
    }
}

/// Resolve a possibly-short `name` to a full-path key in `table`: exact, then
/// partition-qualified against
/// `default_partition`, then `/Common/`, then a suffix match.
pub(crate) fn resolve_name<T>(
    name: &str,
    table: &KindMap<'_, T>,
    default_partition: &str,
) -> Option<String> {
    if table.contains_key(name) {
        return Some(name.to_owned());
    }
    if !name.starts_with('/') {
        let partition = {
            let p = default_partition.trim_matches('/');
            if p.is_empty() { "Common" } else { p }
        };
        let candidate = format!("/{partition}/{name}");
        if table.contains_key(&candidate) {
            return Some(candidate);
        }
        if partition != "Common" {
            let candidate = format!("/Common/{name}");
            if table.contains_key(&candidate) {
                return Some(candidate);
            }
        }
    }
    let suffix = format!("/{name}");
    table
        .iter()
        .find(|(k, _)| k.ends_with(&suffix))
        .map(|(k, _)| k.to_owned())
}

// Config merge

/// Build a single merged [`BigipConfig`] that unions every input, later
/// configs winning on key collision. Only the
/// fields the lint rules read — `objects` and `generic_objects` — are
/// merged; `default_partition` follows the first input. A fresh config would
/// default its partition to `Common`, but `resolve_name` only consults it for
/// non-`/`-prefixed names, which the merged-input path never produces
/// differently across files.
#[must_use]
pub fn merge_configs(configs: &[(String, &BigipConfig)]) -> BigipConfig {
    let mut merged = BigipConfig::default();
    // Track last-wins per (table_name, full_path) for typed objects and per
    // key for generic objects, preserving first-seen order like dict.update.
    let mut obj_index: std::collections::HashMap<(&'static str, String), usize> =
        std::collections::HashMap::new();
    let mut gen_index: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (_uri, cfg) in configs {
        if merged.default_partition.is_empty() {
            merged.default_partition.clone_from(&cfg.default_partition);
        }
        for placed in &cfg.objects {
            let k = (placed.table_name, placed.full_path.clone());
            if let Some(&i) = obj_index.get(&k) {
                merged.objects[i] = placed.clone();
            } else {
                obj_index.insert(k, merged.objects.len());
                merged.objects.push(placed.clone());
            }
        }
        for (key, gobj) in &cfg.generic_objects {
            if let Some(&i) = gen_index.get(key) {
                merged.generic_objects[i] = (key.clone(), gobj.clone());
            } else {
                gen_index.insert(key.clone(), merged.generic_objects.len());
                merged.generic_objects.push((key.clone(), gobj.clone()));
            }
        }
    }
    merged
}

// repr formatting (for `{name!r}` / `{event!r}` messages)

/// The `repr()`-style rendering of a string, as used by `{value!r}`:
/// single-quote-preferred quoting; object paths never contain quotes.
fn py_repr(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    format!("{quote}{s}{quote}")
}

// The eight built-in rules

/// `orphan-monitor` (warning/config): a monitor referenced by no pool or
/// pool-member; `/Common/<factory monitor>` downgrades to `info`.
fn rule_orphan_monitor(view: &ModelView<'_>, out: &mut Vec<Finding>) {
    let mut in_use: HashSet<String> = HashSet::new();
    for (_path, pool) in view.pools.iter() {
        if !pool.monitor.is_empty() {
            let first = pool.monitor.split(' ').next().unwrap_or("");
            if let Some(resolved) = resolve_name(first, &view.monitors, view.default_partition) {
                in_use.insert(resolved);
            }
        }
        for member in member_iter(pool) {
            if !member.monitor.is_empty() {
                let first = member.monitor.split(' ').next().unwrap_or("");
                if let Some(resolved) = resolve_name(first, &view.monitors, view.default_partition)
                {
                    in_use.insert(resolved);
                }
            }
        }
    }
    for (path, monitor) in view.monitors.iter() {
        if in_use.contains(path) {
            continue;
        }
        if path.starts_with("/Common/") && monitor.full_path == path {
            out.push(Finding {
                rule_id: "orphan-monitor".to_owned(),
                severity: "info",
                category: "config",
                full_path: path.to_owned(),
                message: "monitor not referenced by any pool in this config".to_owned(),
            });
        } else {
            out.push(Finding {
                rule_id: "orphan-monitor".to_owned(),
                severity: "warning",
                category: "config",
                full_path: path.to_owned(),
                message: "monitor not referenced by any pool".to_owned(),
            });
        }
    }
}

/// Iterate the typed pool members of a pool.
fn member_iter(
    pool: &crate::model::BigipPool,
) -> impl Iterator<Item = &crate::model::BigipPoolMember> {
    pool.members.items.iter().filter_map(|item| {
        if let crate::value::ListItemValue::PoolMember(m) = &item.value {
            Some(m)
        } else {
            None
        }
    })
}

/// Whether a pool has any members.
fn pool_has_members(pool: &crate::model::BigipPool) -> bool {
    !pool.members.is_empty()
}

/// `empty-pool` (warning/config): a pool with no members.
fn rule_empty_pool(view: &ModelView<'_>, out: &mut Vec<Finding>) {
    for (path, pool) in view.pools.iter() {
        if !pool_has_members(pool) {
            out.push(Finding {
                rule_id: "empty-pool".to_owned(),
                severity: "warning",
                category: "config",
                full_path: path.to_owned(),
                message: "pool has no members".to_owned(),
            });
        }
    }
}

/// `virtual-without-pool` (info/config): a virtual with no default pool and
/// no iRules attached.
fn rule_virtual_without_pool(view: &ModelView<'_>, out: &mut Vec<Finding>) {
    for (path, vs) in view.virtual_servers.iter() {
        if !vs.pool.is_empty() {
            continue;
        }
        if !vs.rules.is_empty() {
            continue;
        }
        out.push(Finding {
            rule_id: "virtual-without-pool".to_owned(),
            severity: "info",
            category: "config",
            full_path: path.to_owned(),
            message: "virtual has no default pool and no iRules attached".to_owned(),
        });
    }
}

/// `pool-without-monitor` (warning/config): a non-empty pool with no
/// pool-level or per-member monitor.
fn rule_pool_without_monitor(view: &ModelView<'_>, out: &mut Vec<Finding>) {
    for (path, pool) in view.pools.iter() {
        if !pool.monitor.is_empty() {
            continue;
        }
        if !pool_has_members(pool) {
            continue; // already covered by empty-pool
        }
        if member_iter(pool).any(|m| !m.monitor.is_empty()) {
            continue;
        }
        out.push(Finding {
            rule_id: "pool-without-monitor".to_owned(),
            severity: "warning",
            category: "config",
            full_path: path.to_owned(),
            message: "pool has no health monitor (pool-level or per-member)".to_owned(),
        });
    }
}

/// The deprecated iRule commands, in registration order.
const DEPRECATED_IRULE_COMMANDS: &[(&str, &str)] = &[
    (
        "X509::extensions",
        "X509::extensions is deprecated; use X509::extensions_count + X509::extensions_get",
    ),
    ("stream::expression", "use STREAM::expression"),
    (
        "log_local0",
        "use 'log local0.<facility> ...' (no underscore form)",
    ),
];

/// `irule-deprecated-command` (warning/irule): a rule body containing a
/// deprecated command substring.
fn rule_irule_deprecated_command(view: &ModelView<'_>, out: &mut Vec<Finding>) {
    for (path, rule) in view.rules.iter() {
        for (cmd, msg) in DEPRECATED_IRULE_COMMANDS {
            if rule.source.contains(cmd) {
                out.push(Finding {
                    rule_id: "irule-deprecated-command".to_owned(),
                    severity: "warning",
                    category: "irule",
                    full_path: path.to_owned(),
                    message: (*msg).to_owned(),
                });
            }
        }
    }
}

/// `irule-empty-when` (info/irule): a `when EVENT { }` block with only
/// whitespace / comments inside.
fn rule_irule_empty_when(
    view: &ModelView<'_>,
    empty_when_re: &regex::Regex,
    out: &mut Vec<Finding>,
) {
    for (path, rule) in view.rules.iter() {
        if empty_when_re.is_match(&rule.source) {
            out.push(Finding {
                rule_id: "irule-empty-when".to_owned(),
                severity: "info",
                category: "irule",
                full_path: path.to_owned(),
                message: "iRule contains a `when` block with no statements".to_owned(),
            });
        }
    }
}

/// `irule-unknown-event` (warning/irule): a `when EVENT` naming an event the
/// f5-irules registry does not know.
fn rule_irule_unknown_event(
    view: &ModelView<'_>,
    when_re: &regex::Regex,
    events: &EventRegistry,
    out: &mut Vec<Finding>,
) {
    for (path, rule) in view.rules.iter() {
        // Collect `_WHEN_RE` matches into a set; dedup first-seen so a
        // repeated event fires at most one finding per rule.
        let mut seen: HashSet<&str> = HashSet::new();
        for cap in when_re.captures_iter(&rule.source) {
            let event = cap.get(1).map_or("", |m| m.as_str());
            if !seen.insert(event) {
                continue;
            }
            if !events.is_known(event) {
                out.push(Finding {
                    rule_id: "irule-unknown-event".to_owned(),
                    severity: "warning",
                    category: "irule",
                    full_path: path.to_owned(),
                    message: format!("iRule references unknown event {}", py_repr(event)),
                });
            }
        }
    }
}

/// Whether `cfg` carries surrounding-context objects: any non-rule typed
/// object, or any generic object
/// keyed by something other than `ltm::rule::*`.
fn has_config_context(view: &ModelView<'_>) -> bool {
    if !view.pools.is_empty()
        || !view.data_groups.is_empty()
        || !view.persistence.is_empty()
        || !view.snat_pools.is_empty()
        || !view.monitors.is_empty()
        || !view.profiles.is_empty()
        || !view.nodes.is_empty()
        || !view.virtual_servers.is_empty()
        || !view.policies.is_empty()
    {
        return true;
    }
    view.generic_object_keys
        .iter()
        .any(|(key, _)| !key.starts_with("ltm::rule::"))
}

/// Map an iRule reference's candidate kinds to a coarse bucket.
fn classify_reference_kind(kinds: &[&str]) -> Option<&'static str> {
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

/// Resolve a classified reference against `view`.
fn resolve_reference(view: &ModelView<'_>, kind: &str, name: &str) -> Option<String> {
    let part = view.default_partition;
    match kind {
        "pool" => resolve_name(name, &view.pools, part),
        "data-group" => resolve_name(name, &view.data_groups, part),
        "persistence" => resolve_name(name, &view.persistence, part),
        "snat-pool" => resolve_name(name, &view.snat_pools, part),
        "monitor" => resolve_name(name, &view.monitors, part),
        "profile" => resolve_name(name, &view.profiles, part),
        "node" => resolve_name(name, &view.nodes, part),
        "rule" => resolve_name(name, &view.rules, part),
        _ => None,
    }
}

/// `irule-missing-<kind>` (warning/irule): an iRule object reference that
/// fails to resolve against the surrounding config. Skipped entirely when
/// the config carries no surrounding context.
fn rule_irule_missing_object(
    view: &ModelView<'_>,
    irules_registry: &CommandRegistry,
    out: &mut Vec<Finding>,
) {
    if !has_config_context(view) {
        return;
    }
    for (path, rule) in view.rules.iter() {
        let mut seen: HashSet<(String, String)> = HashSet::new();
        for reference in extract_irules_object_references(&rule.source, None, irules_registry) {
            let Some(kind) = classify_reference_kind(&reference.kinds) else {
                continue;
            };
            // `persist <kind> {insert|lookup} <session-key>` surfaces a
            // heuristic candidate at argument_index >= 2 that is not a real
            // persistence reference — skip it (the canonical form lives at
            // argument_index 0).
            if kind == "persistence"
                && reference.command.eq_ignore_ascii_case("persist")
                && reference.argument_index >= 2
            {
                continue;
            }
            if resolve_reference(view, kind, &reference.name).is_some() {
                continue;
            }
            let key = (kind.to_owned(), reference.name.clone());
            if !seen.insert(key) {
                continue;
            }
            out.push(Finding {
                rule_id: format!("irule-missing-{kind}"),
                severity: "warning",
                category: "irule",
                full_path: path.to_owned(),
                message: format!(
                    "iRule references {kind} {} (via `{}`) but no such object exists",
                    py_repr(&reference.name),
                    reference.command
                ),
            });
        }
    }
}

// run_lint

/// Run every registered rule against the union of `configs` and return the
/// findings in registration order. `sources` is unused
/// by the built-in rules but kept for API compatibility. `category` limits to
/// one rule category; `severity` filters the emitted findings.
#[must_use]
pub fn run_lint(
    configs: &[(String, &BigipConfig)],
    _sources: &[(String, &str)],
    category: Option<&str>,
    severity: Option<&str>,
) -> Vec<Finding> {
    // Merge when more than one config; otherwise use the single config.
    let owned;
    let view = if configs.len() > 1 {
        owned = merge_configs(configs);
        ModelView::build(&owned)
    } else {
        // Exactly one config (the verb always passes >= 1).
        ModelView::build(configs[0].1)
    };

    let when_re = regex::Regex::new(r"\bwhen\s+([A-Z][A-Z0-9_]*)\b").expect("static regex");
    let empty_when_re = regex::Regex::new(r"when\s+[A-Z_][A-Z0-9_]*\s*\{\s*(?:#[^\n]*\n\s*)*\}")
        .expect("static regex");
    let events = EventRegistry::build();
    let mut irules_registry = CommandRegistry::build_default();
    irules_registry.load_dialect(DialectSet::IRULES);

    let mut findings: Vec<Finding> = Vec::new();

    // The config-category rules, then the irule-category rules, in the fixed
    // registration order.
    let run_config = matches!(category, None | Some("config"));
    let run_irule = matches!(category, None | Some("irule"));

    if run_config {
        rule_orphan_monitor(&view, &mut findings);
        rule_empty_pool(&view, &mut findings);
        rule_virtual_without_pool(&view, &mut findings);
        rule_pool_without_monitor(&view, &mut findings);
    }
    if run_irule {
        rule_irule_deprecated_command(&view, &mut findings);
        rule_irule_empty_when(&view, &empty_when_re, &mut findings);
        rule_irule_unknown_event(&view, &when_re, &events, &mut findings);
        rule_irule_missing_object(&view, &irules_registry, &mut findings);
    }

    if let Some(sev) = severity {
        findings.retain(|f| f.severity == sev);
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_bigip_conf;

    fn lint(src: &str) -> Vec<Finding> {
        let cfg = parse_bigip_conf(src, "Common");
        run_lint(&[("u".to_owned(), &cfg)], &[], None, None)
    }

    #[test]
    fn empty_pool_and_pool_without_monitor() {
        let src = "\
ltm pool /Common/empty { }
ltm pool /Common/nomon {
    members {
        /Common/n1:80 { address 10.0.0.1 }
    }
}
ltm node /Common/n1 { address 10.0.0.1 }
";
        let findings = lint(src);
        assert!(findings.iter().any(|f| f.rule_id == "empty-pool"
            && f.full_path == "/Common/empty"
            && f.severity == "warning"));
        assert!(
            findings
                .iter()
                .any(|f| f.rule_id == "pool-without-monitor" && f.full_path == "/Common/nomon")
        );
        // Empty pool is NOT also flagged pool-without-monitor.
        assert!(
            !findings
                .iter()
                .any(|f| f.rule_id == "pool-without-monitor" && f.full_path == "/Common/empty")
        );
    }

    #[test]
    fn orphan_monitor_factory_vs_custom() {
        let src = "\
ltm monitor http /Common/factory_mon { }
ltm monitor http /Partition2/custom_mon { }
";
        let cfg = parse_bigip_conf(src, "Partition2");
        let findings = run_lint(&[("u".to_owned(), &cfg)], &[], None, None);
        // /Common/ factory monitor downgrades to info.
        assert!(findings.iter().any(|f| f.rule_id == "orphan-monitor"
            && f.full_path == "/Common/factory_mon"
            && f.severity == "info"));
        assert!(findings.iter().any(|f| f.rule_id == "orphan-monitor"
            && f.full_path == "/Partition2/custom_mon"
            && f.severity == "warning"));
    }

    #[test]
    fn virtual_without_pool_skips_when_irule_attached() {
        let src = "\
ltm virtual /Common/vs_no_pool { }
ltm virtual /Common/vs_with_rule {
    rules {
        /Common/r1
    }
}
ltm rule /Common/r1 { when HTTP_REQUEST { } }
";
        let findings = lint(src);
        assert!(findings.iter().any(|f| f.rule_id == "virtual-without-pool"
            && f.full_path == "/Common/vs_no_pool"));
        assert!(
            !findings
                .iter()
                .any(|f| f.rule_id == "virtual-without-pool"
                    && f.full_path == "/Common/vs_with_rule")
        );
    }

    #[test]
    fn irule_empty_when_and_deprecated() {
        let src = "\
ltm rule /Common/r1 {
    when HTTP_REQUEST { }
    log_local0.info \"hi\"
}
";
        let findings = lint(src);
        assert!(findings.iter().any(|f| f.rule_id == "irule-empty-when"));
        assert!(
            findings
                .iter()
                .any(|f| f.rule_id == "irule-deprecated-command")
        );
    }

    #[test]
    fn unknown_event_fires() {
        let src = "ltm rule /Common/r1 { when NOT_A_REAL_EVENT { log local0. x } }";
        let findings = lint(src);
        assert!(
            findings
                .iter()
                .any(|f| f.rule_id == "irule-unknown-event"
                    && f.message.contains("NOT_A_REAL_EVENT"))
        );
        // A real event does not fire.
        let ok = "ltm rule /Common/r2 { when HTTP_REQUEST { log local0. x } }";
        let cfg = parse_bigip_conf(ok, "Common");
        let f2 = run_lint(&[("u".to_owned(), &cfg)], &[], None, None);
        assert!(!f2.iter().any(|f| f.rule_id == "irule-unknown-event"));
    }
}
