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

//! Drift gate between the projection's `PathRef` targets and the registry's
//! reference data.
//!
//! Every `path_ref(…, "ltm pool")` in `projection.rs` states which kind a
//! property names. `rust/tcl-registry`'s `REFERENCE_EDGES` states the same
//! thing for the same properties. Nothing has forced the two to agree, and
//! they have not: reviews of the projection have caught a property aimed at
//! `apm policy customization-source` when the config names an `apm policy
//! customization-group`, and a kind missing from the projection entirely
//! while the registry named it as a target.
//!
//! This test observes the targets rather than reading the source: it projects
//! one object of every covered kind out of the committed fixtures and walks
//! the resulting values for `PathRef`s. Every `(owner kind, property, target)`
//! it finds must either agree with the registry or be recorded in
//! [`ACCEPTED_DIVERGENCE`] with the reason.
//!
//! The registry cannot yet *drive* the projection — it carries no reference
//! data for most of these properties — so the exception list is long. It is
//! the work inventory for closing that gap: every entry removed from it is a
//! property the two layers now agree on.

use std::collections::{BTreeMap, BTreeSet};

use tcl_bigip::parser::parse_bigip_conf;
use tcl_bigip_query::eval::Root;
use tcl_bigip_query::projection::root_container;
use tcl_bigip_query::value::Value;
use tcl_registry::bigip::{default_registry, reference_targets};

const FIXTURES: &[(&str, &str)] = &[
    (
        "bigip.conf",
        include_str!("../../../../samples/bigip/bigip.conf"),
    ),
    (
        "bigip_base.conf",
        include_str!("../../../../samples/bigip/bigip_base.conf"),
    ),
    (
        "ltm.conf",
        include_str!("../../../../samples/for_f5_query/ltm.conf"),
    ),
    (
        "gtm.conf",
        include_str!("../../../../samples/for_f5_query/gtm.conf"),
    ),
    (
        "apm.conf",
        include_str!("../../../../samples/for_f5_query/apm.conf"),
    ),
    (
        "lab_localhost.conf",
        include_str!("../../../../samples/for_f5_query/sysadmin/lab_localhost.conf"),
    ),
    (
        "lab_platform.conf",
        include_str!("../../../../samples/for_f5_query/sysadmin/lab_platform.conf"),
    ),
    (
        "tier3-reaggregator.conf",
        include_str!("../../../../samples/for_f5_query/multitier/tier3-reaggregator.conf"),
    ),
    (
        "device-01.bigip.conf",
        include_str!("../../../../rust/bigip-report-gen/python/tests/data/device-01.bigip.conf"),
    ),
];

/// `(owner kind, property, target)` triples the registry does not confirm,
/// each with why it stands. Grouped by cause.
const ACCEPTED_DIVERGENCE: &[(&str, &str, &str, &str)] = &[
    // The registry records no reference edge for the property at all. These
    // are gaps in the registry data, not in the projection: the target is
    // what a real config names, verified against the committed fixtures.
    ("cm device", "cert", "cm cert", "registry has no edge"),
    ("cm device", "key", "cm key", "registry has no edge"),
    (
        "cm device-group",
        "devices",
        "cm device",
        "registry has no edge",
    ),
    ("cm ha-group", "pools", "ltm pool", "registry has no edge"),
    (
        "cm traffic-group",
        "default-device",
        "cm device",
        "registry has no edge",
    ),
    (
        "cm traffic-group",
        "ha-group",
        "cm ha-group",
        "registry has no edge",
    ),
    (
        "cm trust-domain",
        "ca-cert",
        "cm cert",
        "registry has no edge",
    ),
    (
        "cm trust-domain",
        "ca-devices",
        "cm device",
        "registry has no edge",
    ),
    (
        "cm trust-domain",
        "ca-key",
        "cm key",
        "registry has no edge",
    ),
    (
        "cm trust-domain",
        "trust-group",
        "cm device-group",
        "registry has no edge",
    ),
    (
        "gtm listener",
        "profiles",
        "ltm profile",
        "registry has no edge",
    ),
    (
        "gtm server",
        "datacenter",
        "gtm datacenter",
        "registry has no edge",
    ),
    (
        "ltm monitor",
        "cert",
        "sys file ssl-cert",
        "registry has no edge",
    ),
    (
        "ltm monitor",
        "key",
        "sys file ssl-key",
        "registry has no edge",
    ),
    (
        "ltm profile",
        "ca-file",
        "sys file ssl-cert",
        "registry has no edge",
    ),
    (
        "ltm profile",
        "cert",
        "sys file ssl-cert",
        "registry has no edge",
    ),
    (
        "ltm profile",
        "chain",
        "sys file ssl-cert",
        "registry has no edge",
    ),
    (
        "ltm profile",
        "cipher-group",
        "ltm cipher group",
        "registry has no edge",
    ),
    (
        "ltm profile",
        "key",
        "sys file ssl-key",
        "registry has no edge",
    ),
    ("ltm profile", "pool", "ltm pool", "registry has no edge"),
    (
        "ltm profile",
        "proxy-ca-cert",
        "sys file ssl-cert",
        "registry has no edge",
    ),
    (
        "ltm profile",
        "proxy-ca-key",
        "sys file ssl-key",
        "registry has no edge",
    ),
    (
        "ltm profile",
        "publisher",
        "sys log-config publisher",
        "registry has no edge",
    ),
    ("ltm virtual", "auth", "ltm profile", "registry has no edge"),
    (
        "ltm virtual",
        "clone-pools",
        "ltm pool",
        "registry has no edge",
    ),
    (
        "ltm virtual",
        "rate-class",
        "ltm rate-class",
        "registry has no edge",
    ),
    (
        "ltm virtual",
        "per-flow-request-access-policy",
        "apm policy access-policy",
        "registry has no edge",
    ),
    (
        "net route-domain",
        "parent",
        "net route-domain",
        "registry has no edge",
    ),
    (
        "net route-domain",
        "vlans",
        "net vlan",
        "registry has no edge",
    ),
    (
        "net stp",
        "interfaces",
        "net interface",
        "registry has no edge",
    ),
    (
        "net vlan",
        "interfaces",
        "net interface",
        "registry has no edge",
    ),
    (
        "security firewall address-list",
        "address-lists",
        "security firewall address-list",
        "registry has no edge",
    ),
    (
        "security firewall policy",
        "rule-lists",
        "security firewall rule-list",
        "registry has no edge",
    ),
    (
        "security nat policy",
        "rule-lists",
        "security nat rule-list",
        "registry has no edge",
    ),
    (
        "sys file ssl-cert",
        "issuer-cert",
        "sys file ssl-cert",
        "registry has no edge",
    ),
    (
        "sys folder",
        "device-group",
        "cm device-group",
        "registry has no edge",
    ),
    // The registry has no spec for the owner kind, so it can carry no edges
    // for it. `apm policy access-policy` / `policy-item` are parsed by the
    // model and projected here, but absent from the registry catalogue.
    (
        "apm policy access-policy",
        "default-ending",
        "apm policy policy-item",
        "owner kind absent from the registry",
    ),
    (
        "apm policy access-policy",
        "items",
        "apm policy policy-item",
        "owner kind absent from the registry",
    ),
    (
        "apm policy access-policy",
        "start-item",
        "apm policy policy-item",
        "owner kind absent from the registry",
    ),
    (
        "apm policy policy-item",
        "agents",
        "apm policy agent",
        "owner kind absent from the registry",
    ),
    // The registry's edge is wrong about TMSH. Left as-is pending a registry
    // correction; changing the projection to match would break navigation.
    (
        "gtm datacenter",
        "prober-pool",
        "gtm prober-pool",
        "registry says ltm_pool; a datacenter's prober-pool is a gtm prober-pool",
    ),
    (
        "gtm server",
        "prober-pool",
        "gtm prober-pool",
        "registry says ltm_pool; a server's prober-pool is a gtm prober-pool",
    ),
    (
        "gtm listener",
        "pool",
        "gtm pool",
        "registry says ltm_pool; needs a TMSH ruling before either side moves",
    ),
    // A route's `interface` names the VLAN or tunnel it egresses on — the
    // same target set as `ltm virtual`'s `transparent-nexthop`, which already
    // resolves against `net vlan`. The registry records `net_interface`.
    // `modules::net_vlan_interfaces_and_route_interface_deref` pins the
    // behaviour against a real stanza.
    (
        "net route",
        "interface",
        "net vlan",
        "registry says net_interface; TMSH takes a VLAN or tunnel path",
    ),
];

/// Display spellings for a registry kind: the exact `module object-type`,
/// plus the family prefix the projection merges specific types into
/// (`ltm profile client-ssl` is reachable as `ltm profile`).
fn displays_for(reg_kind: &str) -> Vec<String> {
    let reg = default_registry();
    let Some(spec) = reg.get(reg_kind) else {
        return Vec::new();
    };
    let Some(module) = spec.kind_spec.module else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for object_type in spec.kind_spec.object_types {
        out.push(format!("{module} {object_type}"));
        if let Some((family, _)) = object_type.split_once(' ') {
            out.push(format!("{module} {family}"));
        }
    }
    out
}

/// Collect `(property, target)` for every `PathRef` reachable from *value*,
/// descending through lists and plain objects but not through `ObjectRef`s,
/// which carry their own kind and are visited in their own right.
fn collect_refs(property: &str, value: &Value, out: &mut BTreeSet<(String, String)>) {
    match value {
        Value::PathRef(r) => {
            out.insert((property.to_owned(), r.expected_kind.clone()));
        }
        Value::List(items) | Value::Stream(items) => {
            for item in items {
                collect_refs(property, item, out);
            }
        }
        Value::Object(map) => {
            for (key, v) in map {
                collect_refs(key, v, out);
            }
        }
        _ => {}
    }
}

/// Project one object of every kind the fixtures carry, and gather each
/// kind's `(property, target)` pairs.
fn observed_targets() -> BTreeMap<String, BTreeSet<(String, String)>> {
    let mut out: BTreeMap<String, BTreeSet<(String, String)>> = BTreeMap::new();
    for (name, source) in FIXTURES {
        let config = parse_bigip_conf(source, "Common");
        let root = Root::bigip(*name, (*source).to_owned(), config);
        let Value::Container(container) = root_container(&root) else {
            panic!("{name}: a BIG-IP root must project a container");
        };
        // Walk `<root>` → module → kind → objects, forcing every container so
        // the projection builds each kind's fields.
        for (_module, module_value) in container.entries() {
            let Value::Container(module_container) = module_value else {
                continue;
            };
            for (_label, kind_value) in module_container.entries() {
                let Value::Container(kind_container) = kind_value else {
                    continue;
                };
                for (_path, object) in kind_container.entries() {
                    let Value::ObjectRef(obj) = object else {
                        continue;
                    };
                    let entry = out.entry(obj.kind.clone()).or_default();
                    for (property, value) in &obj.fields {
                        collect_refs(property, value, entry);
                    }
                }
            }
        }
    }
    out
}

#[test]
fn projection_pathref_targets_agree_with_the_registry() {
    let reg = default_registry();
    let accepted: BTreeSet<(&str, &str, &str)> = ACCEPTED_DIVERGENCE
        .iter()
        .map(|(kind, property, target, _why)| (*kind, *property, *target))
        .collect();

    let observed = observed_targets();
    assert!(
        !observed.is_empty(),
        "the fixtures projected no objects at all"
    );

    let mut unexplained = Vec::new();
    let mut confirmed = 0usize;
    let mut seen: BTreeSet<(String, String, String)> = BTreeSet::new();
    for (kind, refs) in &observed {
        for (property, target) in refs {
            // An empty target is a deliberate "not a ref" (`ltm pool`'s
            // member monitor when unset); nothing to check.
            if target.is_empty() {
                continue;
            }
            seen.insert((kind.clone(), property.clone(), target.clone()));
            let candidates = reg.candidate_registry_kinds_for_display(kind);
            let registry_targets: BTreeSet<&'static str> = candidates
                .iter()
                .flat_map(|rk| reference_targets(rk, property).iter().copied())
                .collect();
            let displays: BTreeSet<String> = registry_targets
                .iter()
                .flat_map(|rk| displays_for(rk))
                .collect();
            if displays.contains(target) {
                confirmed += 1;
                continue;
            }
            if accepted.contains(&(kind.as_str(), property.as_str(), target.as_str())) {
                continue;
            }
            let found: Vec<&String> = displays.iter().take(4).collect();
            unexplained.push(format!(
                "  {kind}.{property} -> {target:?}; registry says {found:?}"
            ));
        }
    }

    assert!(
        unexplained.is_empty(),
        "{} projection reference target(s) the registry does not confirm and \
         ACCEPTED_DIVERGENCE does not explain.\nEither correct the target, add \
         the missing edge to rust/tcl-registry/src/bigip/references.rs, or record \
         it with a reason:\n{}",
        unexplained.len(),
        unexplained.join("\n")
    );

    // An exception the fixtures never exercise cannot be judged: a scalar ref
    // always materialises a `PathRef`, but an empty list-valued one
    // materialises nothing, and a kind no fixture carries is never projected
    // at all. Report the blind spot rather than failing on it — the gate is
    // only as wide as the fixtures.
    let uncovered: Vec<String> = ACCEPTED_DIVERGENCE
        .iter()
        .filter(|(kind, property, target, _)| {
            !seen.contains(&(
                (*kind).to_owned(),
                (*property).to_owned(),
                (*target).to_owned(),
            ))
        })
        .map(|(kind, property, target, _)| format!("{kind}.{property} -> {target}"))
        .collect();
    if !uncovered.is_empty() {
        eprintln!(
            "note: {} recorded divergence(s) sit outside fixture coverage and \
             were not checked (an empty list-valued ref materialises no PathRef, \
             and a kind no fixture carries is never projected):\n  {}",
            uncovered.len(),
            uncovered.join("\n  ")
        );
    }

    assert!(
        confirmed > 0,
        "no projection target was confirmed by the registry, which means the \
         lookup itself is broken rather than the data merely being sparse"
    );
}
