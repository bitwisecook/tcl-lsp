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

//! Derive the object graph, listener-matching fields and iRule dynamic actions.
//!
//! A faithful port of `f5report.graph`. It consumes the already-shaped
//! per-device model from [`crate::model`] and produces:
//!
//! * a **graph** of typed nodes and edges (including pools referenced *inside*
//!   iRules and pool-member → node links) that the topology explorer renders;
//! * per-virtual **listener** fields the listener-matching table ranks by
//!   specificity;
//! * per-iRule **dynamic actions** surfaced on the virtuals that attach a rule.

use std::collections::{HashMap, HashSet};

use regex::Regex;
use serde_json::{Map, Value as J};
use tcl_bigip::value::{IPAddress, Network, Port, WildcardAddress};

use crate::jutil::{barr, bbool, bstr, sarr};

/// Node type -> short prefix used in stable object ids (`vs:/Common/x`),
/// in display order.
pub(crate) const NODE_TYPES: &[(&str, &str)] = &[
    ("virtuals", "vs"),
    ("pools", "pool"),
    ("nodes", "node"),
    ("monitors", "mon"),
    ("rules", "rule"),
    ("profiles", "prof"),
    ("persistence", "persist"),
    ("policies", "policy"),
    ("snatpools", "snat"),
    ("dataGroups", "dg"),
];

// --- route domain / listener parsing ----------------------------------------

/// Extract BIG-IP listener-matching fields from a shaped virtual.
pub(crate) fn parse_listener(v: &Map<String, J>) -> J {
    let raw = bstr(v, "destAddr");
    let literal = IPAddress::try_parse(raw);
    let wildcard = WildcardAddress::try_parse(raw);
    let rd = literal.as_ref().and_then(|ip| ip.route_domain).unwrap_or(0);
    let is_v6 =
        literal.as_ref().is_some_and(IPAddress::is_ipv6) || wildcard == Some(WildcardAddress::Any6);
    let any_addr = wildcard.is_some() || literal.as_ref().is_some_and(IPAddress::is_unspecified);
    let dest_addr = literal.map_or_else(|| raw.to_owned(), |ip| ip.addr.to_string());
    let full_bits = if is_v6 { 128 } else { 32 };
    let prefix = if any_addr {
        0
    } else {
        Network::prefix_from_mask(bstr(v, "mask"), is_v6).unwrap_or(full_bits)
    };
    let port = Port::try_parse_service(bstr(v, "destPort")).map_or(0, |port| port.port);

    let src_default = if is_v6 { "::/0" } else { "0.0.0.0/0" };
    let src_field = bstr(v, "source");
    let src = if src_field.is_empty() {
        src_default
    } else {
        src_field
    };
    let source_network = Network::try_parse(src);
    let src_prefix = source_network.as_ref().map_or(0, Network::prefix_length);
    let src = source_network.map_or_else(|| src.to_owned(), |net| net.canonical_cidr());

    let vlans: Vec<J> = sarr(v, "vlans")
        .iter()
        .map(|x| J::String(leaf(x).to_string()))
        .collect();

    let ip_proto = bstr(v, "ipProtocol");
    let protocol = if ip_proto.is_empty() { "any" } else { ip_proto };

    let mut o = Map::new();
    o.insert(
        "family".into(),
        J::String(if is_v6 { "IPv6" } else { "IPv4" }.into()),
    );
    o.insert("address".into(), J::String(dest_addr));
    o.insert("anyAddr".into(), J::Bool(any_addr));
    o.insert("prefix".into(), J::from(prefix));
    o.insert("maxPrefix".into(), J::from(full_bits));
    o.insert("port".into(), J::from(port));
    o.insert(
        "portRaw".into(),
        J::String(if port == 0 {
            "any".to_string()
        } else {
            port.to_string()
        }),
    );
    o.insert("protocol".into(), J::String(protocol.to_string()));
    o.insert("routeDomain".into(), J::from(rd));
    o.insert("source".into(), J::String(src));
    o.insert("sourcePrefix".into(), J::from(src_prefix));
    o.insert("vlans".into(), J::Array(vlans));
    o.insert("vlansEnabled".into(), J::Bool(bbool(v, "vlansEnabled")));
    o.insert("vlansDisabled".into(), J::Bool(bbool(v, "vlansDisabled")));
    J::Object(o)
}

// --- iRule dynamic actions ---------------------------------------------------

struct DynCmd {
    re: Regex,
    category: &'static str,
    effect: &'static str,
}

fn dynamic_cmds() -> Vec<DynCmd> {
    // (regex, category, human effect). Matched case-insensitively.
    let spec: &[(&str, &str, &str)] = &[
        (
            r"(?i)\bSSL::disable(?:\s+(clientside|serverside))?",
            "SSL",
            "SSL::disable",
        ),
        (
            r"(?i)\bSSL::enable(?:\s+(clientside|serverside))?",
            "SSL",
            "SSL::enable",
        ),
        (r"(?i)\bHTTP::disable\b", "HTTP", "HTTP::disable"),
        (r"(?i)\bHTTP::enable\b", "HTTP", "HTTP::enable"),
        (
            r"(?i)\bCOMPRESS::disable\b",
            "Compression",
            "COMPRESS::disable",
        ),
        (
            r"(?i)\bCOMPRESS::enable\b",
            "Compression",
            "COMPRESS::enable",
        ),
        (r"(?i)\bCACHE::disable\b", "Cache", "CACHE::disable"),
        (r"(?i)\bCACHE::enable\b", "Cache", "CACHE::enable"),
        (r"(?i)\bWAM::(?:enable|disable)\b", "WebAccel", "WAM"),
        (r"(?i)\bLB::detach\b", "Pool", "LB::detach"),
        (
            r"(?i)\bpersist\s+(none|uie|cookie|source_addr|dest_addr|hash|ssl|sip|carp|universal)",
            "Persistence",
            "persist",
        ),
        (r"(?i)\bsnatpool\s+(\S+)", "SNAT", "snatpool"),
        (r"(?i)\bsnat\s+(automap|none|\S+)", "SNAT", "snat"),
        (r"(?i)\bnode\s+(\d[\d.:a-fA-F]+)", "Node", "node override"),
        (r"(?i)\bvirtual\s+(\S+)", "Virtual", "virtual target"),
    ];
    spec.iter()
        .map(|(re, category, effect)| DynCmd {
            re: Regex::new(re).expect("valid dynamic-cmd regex"),
            category,
            effect,
        })
        .collect()
}

/// The attachable object types considered for dynamic (iRule) reachability:
/// `pool` (LB target), `node` (direct node), `snatpool` (SNAT). Literal iRule
/// references (`pool /Common/x`) are already captured by the engine's
/// `referenced_by` (`.refs`), so only a *non-literal* argument counts here.
pub(crate) const ATTACH_TYPES: &[&str] = &["pools", "nodes", "snatpools"];

/// One dynamic attachment an iRule performs: the reconstructed name pattern, the
/// rule that builds it, and the partition contexts the rule executes in (the
/// partitions of the virtuals it is attached to).
pub(crate) struct Attach {
    pub rule: String,
    /// Partitions the rule runs in (partitions of its attaching virtuals). Empty
    /// when the rule is not attached to any virtual — it never executes, so it
    /// makes nothing reachable.
    pub ctx: std::collections::BTreeSet<String>,
    pub pattern: tcl_diagram::AttachPattern,
}

/// Partition-aware reachability: can `pat`, evaluated by a rule running in the
/// `ctx` partitions, build a name that resolves to the given object?
///
/// BIG-IP name resolution for an *unqualified* object name in an iRule is
/// relative to the partition of the **virtual server** carrying the traffic
/// (with `/Common` always visible as a fallback) — not the iRule's own folder.
/// So a `/Common` iRule doing `pool "web_[HTTP::host]"` attached to a virtual in
/// `/TenantA` selects `/TenantA/web_*` (or `/Common/web_*`), never `/TenantB/…`.
///
/// * an `unconstrained` pattern (`pool $x`) could be any name in any partition;
/// * a pattern rooted at `/partition/…` is absolute — match the full path;
/// * an unqualified pattern reaches only objects in a context partition (or
///   `/Common`) whose leaf name matches.
pub(crate) fn pattern_reaches(
    pat: &tcl_diagram::AttachPattern,
    obj_leaf: &str,
    obj_full: &str,
    obj_partition: &str,
    ctx: &std::collections::BTreeSet<String>,
) -> bool {
    if pat.unconstrained {
        return true;
    }
    if pat.prefix.starts_with('/') {
        return pat.matches(obj_full);
    }
    let visible = obj_partition == "Common" || ctx.contains(obj_partition);
    visible && pat.matches(obj_leaf)
}

/// Build, per attachable object type, the [`Attach`] records an iRule could
/// dynamically construct, tagged with the partition contexts the rule runs in.
///
/// Rather than an all-or-nothing "risk" heuristic that demotes an *entire*
/// object type the moment any rule attaches it dynamically, each
/// attach expression is reconstructed (via [`tcl_diagram::attach_reach`]) into a
/// prefix / contained / suffix name pattern, resolved *per partition*. A pool
/// called `db_backend` stays a *confirmed* orphan even when a rule does
/// `pool "web_[HTTP::host]"`, and a `/TenantB/web_pool` stays orphaned when the
/// rule only runs on `/TenantA` virtuals.
pub(crate) fn attach_index(
    rules: &[J],
    rule_ctx: &HashMap<String, std::collections::BTreeSet<String>>,
) -> std::collections::BTreeMap<&'static str, Vec<Attach>> {
    let mut index: std::collections::BTreeMap<&'static str, Vec<Attach>> =
        std::collections::BTreeMap::new();
    for r in rules {
        let Some(rm) = r.as_object() else { continue };
        let body = bstr(rm, "body");
        if body.is_empty() {
            continue;
        }
        let fp = bstr(rm, "fullPath").to_string();
        let name = bstr(rm, "name");
        let rule_name = if name.is_empty() { &fp } else { name }.to_string();
        let ctx = rule_ctx.get(&fp).cloned().unwrap_or_default();
        let reach = tcl_diagram::attach_reach(body);
        for ty in ATTACH_TYPES {
            for pat in reach.patterns_for(ty) {
                index.entry(*ty).or_default().push(Attach {
                    rule: rule_name.clone(),
                    ctx: ctx.clone(),
                    pattern: pat.clone(),
                });
            }
        }
    }
    index
}

/// The `{rule, pattern}` records whose name-pattern could attach the given
/// object, resolved per the rule's partition context. Only rules that are
/// actually attached to a virtual (non-empty context) can reach anything.
/// Returns an empty vec when the object is provably unreachable by every rule.
pub(crate) fn attach_matches(
    attaches: &[Attach],
    leaf: &str,
    full: &str,
    partition: &str,
    address: &str,
) -> Vec<J> {
    let mut out: Vec<J> = Vec::new();
    let mut seen: HashSet<(String, String)> = HashSet::new();
    for a in attaches {
        if a.ctx.is_empty() {
            continue; // unattached rule never executes
        }
        let hit = pattern_reaches(&a.pattern, leaf, full, partition, &a.ctx)
            || (!address.is_empty()
                && pattern_reaches(&a.pattern, address, address, partition, &a.ctx));
        if !hit {
            continue;
        }
        let glob = a.pattern.glob();
        if !seen.insert((a.rule.clone(), glob.clone())) {
            continue;
        }
        let mut o = Map::new();
        o.insert("rule".into(), J::String(a.rule.clone()));
        o.insert("pattern".into(), J::String(glob));
        out.push(J::Object(o));
    }
    out
}

/// Return the profile/processing-changing commands a rule issues.
pub(crate) fn irule_dynamic_actions(body: &str) -> Vec<J> {
    if body.is_empty() {
        return Vec::new();
    }
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut out: Vec<J> = Vec::new();
    for cmd in dynamic_cmds() {
        let has_group = cmd.re.captures_len() > 1;
        for caps in cmd.re.captures_iter(body) {
            let arg = if has_group {
                caps.get(1).map_or("", |m| m.as_str())
            } else {
                ""
            }
            .to_string();
            let key = (cmd.effect.to_string(), arg.clone());
            if seen.contains(&key) {
                continue;
            }
            seen.insert(key);
            let mut o = Map::new();
            o.insert("category".into(), J::String(cmd.category.into()));
            o.insert("effect".into(), J::String(cmd.effect.into()));
            o.insert("arg".into(), J::String(arg));
            out.push(J::Object(o));
        }
    }
    out
}

// --- object graph ------------------------------------------------------------

fn oid(kind: &str, full_path: &str) -> String {
    format!("{kind}:{full_path}")
}

/// `/Common/x` -> `x`; the leaf after the last `/`.
fn leaf(s: &str) -> &str {
    s.rsplit('/').next().unwrap_or(s)
}

fn partition_of(full_path: &str) -> &str {
    if full_path.starts_with('/') {
        full_path.split('/').nth(1).unwrap_or("")
    } else {
        ""
    }
}

fn split_monitors(monitor: &str) -> Vec<String> {
    if monitor.is_empty() {
        return Vec::new();
    }
    // `/Common/http and /Common/tcp` -> ['/Common/http', '/Common/tcp'].
    let re = Regex::new(r"\s+and\s+|\s+or\s+|\s*,\s*|\s+min\s+\d+\s+of\s*")
        .expect("valid monitor split");
    re.split(monitor)
        .map(str::trim)
        .filter(|p| p.starts_with('/'))
        .map(str::to_string)
        .collect()
}

/// Build a typed node/edge graph for one device.
pub(crate) fn build_graph(device: &Map<String, J>) -> J {
    // nodes indexed by oid (insertion-ordered).
    let mut nodes: Vec<J> = Vec::new();
    let mut node_index: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut edges: Vec<J> = Vec::new();
    let mut edge_seen: HashSet<(String, String, String)> = HashSet::new();

    // index nodes by full-path and (for members) by address.
    let mut node_by_path: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut node_by_addr: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    macro_rules! add_node {
        ($kind:expr, $full_path:expr, $name:expr, $orphan:expr) => {{
            let id = oid($kind, $full_path);
            if !node_index.contains_key(&id) {
                let display = if $name.is_empty() {
                    leaf($full_path).to_string()
                } else {
                    $name.to_string()
                };
                let mut n = Map::new();
                n.insert("oid".into(), J::String(id.clone()));
                n.insert("type".into(), J::String($kind.to_string()));
                n.insert("name".into(), J::String(display));
                n.insert("fullPath".into(), J::String($full_path.to_string()));
                n.insert(
                    "partition".into(),
                    J::String(partition_of($full_path).to_string()),
                );
                if let Some(orphan) = $orphan {
                    n.insert("orphan".into(), J::Bool(orphan));
                }
                node_index.insert(id.clone(), nodes.len());
                nodes.push(J::Object(n));
            }
            id
        }};
    }

    let add_edge = |edges: &mut Vec<J>,
                    edge_seen: &mut HashSet<(String, String, String)>,
                    node_index: &std::collections::HashMap<String, usize>,
                    src: &str,
                    dst: &str,
                    kind: &str,
                    label: &str| {
        if src.is_empty()
            || dst.is_empty()
            || !node_index.contains_key(src)
            || !node_index.contains_key(dst)
        {
            return;
        }
        let key = (src.to_string(), dst.to_string(), kind.to_string());
        if edge_seen.contains(&key) {
            return;
        }
        edge_seen.insert(key);
        let mut e = Map::new();
        e.insert("from".into(), J::String(src.to_owned()));
        e.insert("to".into(), J::String(dst.to_string()));
        e.insert("kind".into(), J::String(kind.to_string()));
        e.insert("label".into(), J::String(label.to_string()));
        edges.push(J::Object(e));
    };

    // register every referable object first
    for (key, prefix) in NODE_TYPES {
        if let Some(J::Array(objs)) = device.get(*key) {
            for o in objs {
                let om = match o.as_object() {
                    Some(m) => m,
                    None => continue,
                };
                let fp = bstr(om, "fullPath");
                if fp.is_empty() {
                    continue;
                }
                // Confirmed orphan only (an object an iRule might dynamically
                // attach carries `orphanStatus == "possible"`, not "orphan").
                let orphan = if om.contains_key("orphanStatus") {
                    Some(bstr(om, "orphanStatus") == "orphan")
                } else if om.contains_key("usedBy") {
                    Some(barr(om, "usedBy").is_empty())
                } else {
                    None
                };
                let id = add_node!(prefix, fp, bstr(om, "name"), orphan);
                node_by_path.insert(fp.to_string(), id.clone());
                if *key == "nodes" {
                    let addr = bstr(om, "address");
                    if !addr.is_empty() {
                        node_by_addr.insert(addr.to_string(), id);
                    }
                }
            }
        }
    }

    // virtual edges
    if let Some(J::Array(virtuals)) = device.get("virtuals") {
        for v in virtuals {
            let vm = match v.as_object() {
                Some(m) => m,
                None => continue,
            };
            let vid = oid("vs", bstr(vm, "fullPath"));
            let pool = bstr(vm, "pool");
            if !pool.is_empty() {
                add_edge(
                    &mut edges,
                    &mut edge_seen,
                    &node_index,
                    &vid,
                    &oid("pool", pool),
                    "pool",
                    "pool",
                );
            }
            for r in sarr(vm, "rules") {
                add_edge(
                    &mut edges,
                    &mut edge_seen,
                    &node_index,
                    &vid,
                    &oid("rule", r),
                    "rule",
                    "irule",
                );
            }
            for p in sarr(vm, "profiles") {
                add_edge(
                    &mut edges,
                    &mut edge_seen,
                    &node_index,
                    &vid,
                    &oid("prof", p),
                    "profile",
                    "profile",
                );
            }
            for p in sarr(vm, "persist") {
                add_edge(
                    &mut edges,
                    &mut edge_seen,
                    &node_index,
                    &vid,
                    &oid("persist", p),
                    "persist",
                    "persist",
                );
            }
            for p in sarr(vm, "policies") {
                add_edge(
                    &mut edges,
                    &mut edge_seen,
                    &node_index,
                    &vid,
                    &oid("policy", p),
                    "policy",
                    "policy",
                );
            }
            let snat = bstr(vm, "snatpool");
            if !snat.is_empty() {
                add_edge(
                    &mut edges,
                    &mut edge_seen,
                    &node_index,
                    &vid,
                    &oid("snat", snat),
                    "snat",
                    "snat",
                );
            }
        }
    }

    // pool edges: members -> nodes, pool -> monitors
    if let Some(J::Array(pools)) = device.get("pools") {
        for p in pools {
            let pm = match p.as_object() {
                Some(m) => m,
                None => continue,
            };
            let pid = oid("pool", bstr(pm, "fullPath"));
            if let Some(J::Array(members)) = pm.get("members") {
                for m in members {
                    let mm = match m.as_object() {
                        Some(x) => x,
                        None => continue,
                    };
                    let name = bstr(mm, "name");
                    let npath = name.rsplitn(2, ':').last().unwrap_or(name); // name.rsplit(":",1)[0]
                    let target = node_by_path
                        .get(npath)
                        .or_else(|| node_by_addr.get(bstr(mm, "address")))
                        .cloned();
                    if let Some(tid) = target {
                        add_edge(
                            &mut edges,
                            &mut edge_seen,
                            &node_index,
                            &pid,
                            &tid,
                            "member",
                            bstr(mm, "port"),
                        );
                    }
                }
            }
            for mon in split_monitors(bstr(pm, "monitor")) {
                if node_by_path.contains_key(&mon) {
                    add_edge(
                        &mut edges,
                        &mut edge_seen,
                        &node_index,
                        &pid,
                        &oid("mon", &mon),
                        "monitor",
                        "monitor",
                    );
                }
            }
        }
    }

    // iRule edges: pools referenced *inside* the rule body (engine .refs)
    if let Some(J::Array(rules)) = device.get("rules") {
        for r in rules {
            let rm = match r.as_object() {
                Some(m) => m,
                None => continue,
            };
            let rid = oid("rule", bstr(rm, "fullPath"));
            for pool in sarr(rm, "refPools") {
                add_edge(
                    &mut edges,
                    &mut edge_seen,
                    &node_index,
                    &rid,
                    &oid("pool", pool),
                    "pool-irule",
                    "pool (iRule)",
                );
            }
            for dg in sarr(rm, "refDataGroups") {
                add_edge(
                    &mut edges,
                    &mut edge_seen,
                    &node_index,
                    &rid,
                    &oid("dg", dg),
                    "datagroup",
                    "data-group",
                );
            }
        }
    }

    // Mark built-in / system (default) nodes so diagrams can render them as
    // terminal leaves — shown where directly linked, but never traversed through
    // (a shared default profile must not bridge one app's graph into another's).
    let mut default_paths: HashSet<String> = HashSet::new();
    for (key, _) in NODE_TYPES {
        if let Some(J::Array(objs)) = device.get(*key) {
            for o in objs {
                if let Some(om) = o.as_object()
                    && om.get("isDefault").and_then(J::as_bool) == Some(true)
                {
                    default_paths.insert(bstr(om, "fullPath").to_string());
                }
            }
        }
    }
    for n in &mut nodes {
        if let Some(nm) = n.as_object_mut()
            && default_paths.contains(bstr(nm, "fullPath"))
        {
            nm.insert("isDefault".into(), J::Bool(true));
        }
    }

    let mut g = Map::new();
    g.insert("nodes".into(), J::Array(nodes));
    g.insert("edges".into(), J::Array(edges));
    J::Object(g)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listener_preserves_ipv6_service_and_route_domain_source_prefix() {
        let input = serde_json::json!({
            "destAddr": "2001:db8::53%2",
            "destPort": "http",
            "mask": "ffff:ffff:ffff:ffff::",
            "source": "2001:db8:1::%2/48"
        });
        let listener = parse_listener(input.as_object().unwrap());
        assert_eq!(listener["family"], "IPv6");
        assert_eq!(listener["routeDomain"], 2);
        assert_eq!(listener["port"], 80);
        assert_eq!(listener["prefix"], 64);
        assert_eq!(listener["sourcePrefix"], 48);
        assert_eq!(listener["source"], "2001:db8:1::/48");
    }
}
