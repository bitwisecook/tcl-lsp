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

//! iRule links: resolved rule references, procedure calls, rule contexts and reachability.

use super::{attach_singular, json_ref, leaf, partition_of};
use crate::jutil::{barr, bstr, sarr};
use serde_json::{Map, Value as J};
use std::collections::{BTreeSet, HashMap};

/// Resolve a cross-iRule `call <rule>::proc` target name to an actual iRule full
/// path, preferring the caller's partition, then `/Common`.
fn resolve_rule(
    name: &str,
    caller_part: &str,
    known: &std::collections::HashSet<String>,
    leaf_to_fp: &HashMap<String, Vec<String>>,
) -> Option<String> {
    if name.starts_with('/') && known.contains(name) {
        return Some(name.to_string());
    }
    let fps = leaf_to_fp.get(leaf(name))?;
    if let Some(fp) = fps.iter().find(|f| partition_of(f) == caller_part) {
        return Some(fp.clone());
    }
    if let Some(fp) = fps.iter().find(|f| partition_of(f) == "Common") {
        return Some(fp.clone());
    }
    fps.first().cloned()
}

/// Link iRules that call each other via `call <rule>::<proc>` (F5 cross-iRule
/// proc calls): record each caller's resolved callees as `refRules`, and add the
/// caller to each callee's `usedBy` so a proc-library iRule (attached to no
/// virtual, only called from other rules) is linked in rather than flagged
/// orphaned. Run before orphan classification, which reads `usedBy`.
pub(super) fn link_proc_calls(device: &mut Map<String, J>) {
    let mut leaf_to_fp: HashMap<String, Vec<String>> = HashMap::new();
    let mut known: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut bodies: Vec<(String, String, String)> = Vec::new();
    if let Some(J::Array(rules)) = device.get("rules") {
        for r in rules {
            if let Some(rm) = r.as_object() {
                let fp = bstr(rm, "fullPath").to_string();
                if fp.is_empty() {
                    continue;
                }
                leaf_to_fp
                    .entry(leaf(&fp).to_string())
                    .or_default()
                    .push(fp.clone());
                known.insert(fp.clone());
                bodies.push((fp.clone(), partition_of(&fp), bstr(rm, "body").to_string()));
            }
        }
    }

    let mut caller_refs: HashMap<String, Vec<String>> = HashMap::new();
    let mut callee_callers: HashMap<String, Vec<String>> = HashMap::new();
    for (fp, part, body) in &bodies {
        if body.is_empty() {
            continue;
        }
        let mut resolved: Vec<String> = Vec::new();
        for name in tcl_diagram::proc_call_refs(body) {
            if let Some(callee) = resolve_rule(&name, part, &known, &leaf_to_fp) {
                if &callee == fp {
                    continue;
                }
                if !resolved.contains(&callee) {
                    resolved.push(callee.clone());
                }
                callee_callers.entry(callee).or_default().push(fp.clone());
            }
        }
        if !resolved.is_empty() {
            caller_refs.insert(fp.clone(), resolved);
        }
    }

    if let Some(J::Array(rules)) = device.get_mut("rules") {
        for r in rules.iter_mut() {
            if let Some(rm) = r.as_object_mut() {
                let fp = bstr(rm, "fullPath").to_string();
                if let Some(refs) = caller_refs.get(&fp) {
                    rm.insert(
                        "refRules".into(),
                        J::Array(refs.iter().map(|s| J::String(s.clone())).collect()),
                    );
                }
                if let Some(callers) = callee_callers.get(&fp) {
                    let mut used: Vec<J> = barr(rm, "usedBy").to_vec();
                    let existing: std::collections::HashSet<String> = used
                        .iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect();
                    for c in callers {
                        if !existing.contains(c) {
                            used.push(J::String(c.clone()));
                        }
                    }
                    rm.insert("usedBy".into(), J::Array(used));
                }
            }
        }
    }
}

/// Per-rule partition contexts (partitions of the virtuals attaching a rule).
pub(super) type RuleCtx = HashMap<String, BTreeSet<String>>;

/// Per-rule attaching virtuals, as `(virtual full path, virtual partition)`.
pub(super) type RuleVirtuals = HashMap<String, Vec<(String, String)>>;

/// Map each iRule to the partitions it executes in (partitions of its attaching
/// virtuals) and the attaching virtuals themselves. Virtuals may reference a
/// rule by leaf name or full path.
pub(super) fn build_rule_contexts(device: &Map<String, J>) -> (RuleCtx, RuleVirtuals) {
    let mut leaf_to_fp: HashMap<String, String> = HashMap::new();
    let mut known_fp: std::collections::HashSet<String> = std::collections::HashSet::new();
    if let Some(J::Array(rules)) = device.get("rules") {
        for r in rules {
            if let Some(rm) = r.as_object() {
                let fp = bstr(rm, "fullPath").to_string();
                if fp.is_empty() {
                    continue;
                }
                leaf_to_fp.insert(leaf(&fp).to_string(), fp.clone());
                known_fp.insert(fp);
            }
        }
    }
    let mut ctx: HashMap<String, BTreeSet<String>> = HashMap::new();
    let mut vs: HashMap<String, Vec<(String, String)>> = HashMap::new();
    if let Some(J::Array(virtuals)) = device.get("virtuals") {
        for v in virtuals {
            let Some(vm) = v.as_object() else { continue };
            let vfp = bstr(vm, "fullPath").to_string();
            let vpart = bstr(vm, "partition").to_string();
            for rr in sarr(vm, "rules") {
                let fp = if known_fp.contains(rr) {
                    rr.to_string()
                } else if let Some(f) = leaf_to_fp.get(leaf(rr)) {
                    f.clone()
                } else {
                    continue;
                };
                ctx.entry(fp.clone()).or_default().insert(vpart.clone());
                vs.entry(fp).or_default().push((vfp.clone(), vpart.clone()));
            }
        }
    }
    (ctx, vs)
}

/// Attach a per-iRule reachability table: the objects it statically references,
/// plus the objects each reconstructed dynamic filter could select — resolved
/// per attaching-virtual partition, so a `/Common` rule attached across
/// partitions shows a VS-specific candidate set for each.
pub(super) fn annotate_rule_reachability(device: &mut Map<String, J>, rule_vs: &RuleVirtuals) {
    // Snapshot attachable objects (name, fullPath, partition, address) per type.
    let mut snap: HashMap<&'static str, Vec<(String, String, String, String)>> = HashMap::new();
    for ty in crate::graph::ATTACH_TYPES {
        let mut v = Vec::new();
        if let Some(J::Array(objs)) = device.get(*ty) {
            for o in objs {
                if let Some(om) = o.as_object() {
                    let fp = bstr(om, "fullPath").to_string();
                    let part = partition_of(&fp);
                    v.push((
                        bstr(om, "name").to_string(),
                        fp,
                        part,
                        bstr(om, "address").to_string(),
                    ));
                }
            }
        }
        snap.insert(*ty, v);
    }

    let Some(J::Array(rules)) = device.get_mut("rules") else {
        return;
    };
    for r in rules.iter_mut() {
        let Some(rm) = r.as_object_mut() else {
            continue;
        };
        let body = bstr(rm, "body").to_string();
        let fp = bstr(rm, "fullPath").to_string();
        let own_part = partition_of(&fp);

        // Static references (absolute paths from the engine's `.refs`).
        let mut static_refs: Vec<J> = Vec::new();
        for p in sarr(rm, "refPools") {
            static_refs.push(json_ref("pool", p));
        }
        for dg in sarr(rm, "refDataGroups") {
            static_refs.push(json_ref("data-group", dg));
        }
        // Cross-iRule proc-call references (`call <rule>::<proc>`).
        for rr in sarr(rm, "refRules") {
            static_refs.push(json_ref("irule", rr));
        }

        // Dynamic references grouped by the partition of the attaching virtuals.
        let reach = tcl_diagram::attach_reach(&body);
        let has_dynamic = crate::graph::ATTACH_TYPES
            .iter()
            .any(|ty| !reach.patterns_for(ty).is_empty());
        let mut by_part: std::collections::BTreeMap<String, Vec<String>> =
            std::collections::BTreeMap::new();
        if has_dynamic {
            if let Some(vlist) = rule_vs.get(&fp) {
                for (vfp, vpart) in vlist {
                    by_part.entry(vpart.clone()).or_default().push(vfp.clone());
                }
            }
            if by_part.is_empty() {
                // Unattached: still surface the determined filters, resolved
                // informationally in the rule's own partition.
                by_part.insert(own_part.clone(), Vec::new());
            }
        }
        let attached = rule_vs.get(&fp).is_some_and(|v| !v.is_empty());

        let mut groups: Vec<J> = Vec::new();
        for (part, vlist) in &by_part {
            let mut ctx = BTreeSet::new();
            ctx.insert(part.clone());
            let mut filters: Vec<J> = Vec::new();
            for ty in crate::graph::ATTACH_TYPES {
                for pat in reach.patterns_for(ty) {
                    let objs = snap.get(*ty).map_or(&[][..], Vec::as_slice);
                    let mut matched: Vec<J> = Vec::new();
                    for (nm, ofp, opart, addr) in objs {
                        let hit = crate::graph::pattern_reaches(pat, nm, ofp, opart, &ctx)
                            || (!addr.is_empty()
                                && crate::graph::pattern_reaches(pat, addr, addr, opart, &ctx));
                        if hit {
                            matched.push(json_ref(attach_singular(ty), ofp));
                        }
                    }
                    let mut f = pat.to_json();
                    if let Some(fo) = f.as_object_mut() {
                        fo.insert("type".into(), J::String(attach_singular(ty).into()));
                        fo.insert("objects".into(), J::Array(matched));
                    }
                    filters.push(f);
                }
            }
            let mut g = Map::new();
            g.insert("partition".into(), J::String(part.clone()));
            g.insert(
                "virtuals".into(),
                J::Array(vlist.iter().map(|s| J::String(leaf(s).into())).collect()),
            );
            g.insert("attached".into(), J::Bool(attached));
            g.insert("filters".into(), J::Array(filters));
            groups.push(J::Object(g));
        }

        let mut refs = Map::new();
        refs.insert("static".into(), J::Array(static_refs));
        refs.insert("dynamic".into(), J::Array(groups));
        rm.insert("referencedObjects".into(), J::Object(refs));
    }
}
