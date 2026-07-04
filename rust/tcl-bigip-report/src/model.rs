//! Build a structured report model from BIG-IP configs using the query engine.
//!
//! A faithful port of `f5report.report`. Every fact is pulled from the native
//! `f5-query` engine ([`crate::query`]) — the parsing, object projection and the
//! `referenced_by` reference-graph walk that powers the orphan / dependency
//! analysis. This module only shapes the engine's output into a
//! template-friendly model (a `serde_json` object, so it embeds verbatim in the
//! report for the client-side topology / listener / console views).

use std::collections::{BTreeSet, HashMap};

use regex::Regex;
use serde_json::{Map, Value as J};

use crate::jutil::{barr, bbool, bstr, sarr, truthy};
use crate::query::{Source, query};

/// The engine version string embedded in the report header.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

// Object containers the report walks, in display order. Each is an `f5-query`
// container path under a config root.
const CONTAINERS: &[(&str, &str)] = &[
    ("virtuals", ".ltm.virtual"),
    ("pools", ".ltm.pool"),
    ("nodes", ".ltm.node"),
    ("monitors", ".ltm.monitor"),
    ("rules", ".ltm.rule"),
    ("dataGroups", ".ltm.\"data-group\""),
    ("profiles", ".ltm.profile"),
    ("snatpools", ".ltm.snatpool"),
    ("persistence", ".ltm.persistence"),
    ("policies", ".ltm.policy"),
    ("virtualAddresses", ".ltm.\"virtual-address\""),
];

// Leaf object types that are *referenced* by something else; an empty referrer
// set means the object is orphaned. Virtuals / virtual-addresses are entry
// points, so they are never treated as orphans.
const REFERABLE: &[&str] = &[
    "pools",
    "nodes",
    "monitors",
    "rules",
    "profiles",
    "dataGroups",
    "snatpools",
];

fn container_path(key: &str) -> &'static str {
    CONTAINERS
        .iter()
        .find(|(k, _)| *k == key)
        .map_or("", |(_, p)| *p)
}

// --- small helpers -----------------------------------------------------------

/// Python `str(x)` for the scalar shapes that appear in projected values.
fn py_str(v: &J) -> String {
    match v {
        J::Null => "None".to_string(),
        J::Bool(true) => "True".to_string(),
        J::Bool(false) => "False".to_string(),
        J::String(s) => s.clone(),
        J::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

/// Unwrap engine `ObjectRef` dicts (`{kind, full-path, fields}`) to `fields`.
fn fields_of(rows: Vec<J>) -> Vec<Map<String, J>> {
    rows.into_iter()
        .filter_map(|r| match r {
            J::Object(mut m) if m.contains_key("fields") => match m.remove("fields") {
                Some(J::Object(f)) => Some(f),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

/// Strip trailing ` { ... }` context the projection appends to some refs.
fn clean_path(value: &str) -> String {
    value.split(" {").next().unwrap_or(value).trim().to_string()
}

/// `map[key]` as a cleaned path string.
fn clean_field(m: &Map<String, J>, key: &str) -> String {
    clean_path(bstr(m, key))
}

/// `/Common/x` -> `x`.
fn leaf(s: &str) -> &str {
    s.rsplit('/').next().unwrap_or(s)
}

fn partition_of(full_path: &str) -> String {
    if full_path.starts_with('/') {
        full_path.split('/').nth(1).unwrap_or("").to_string()
    } else {
        String::new()
    }
}

/// Split a BIG-IP destination into `(address, port)`.
fn split_dest(dest: &str) -> (String, String) {
    if dest.is_empty() {
        return (String::new(), String::new());
    }
    let leaf = dest.rsplit('/').next().unwrap_or(dest);
    let colons = leaf.matches(':').count();
    if colons >= 2 {
        // IPv6: address holds the colons, port after a dot.
        if let Some(idx) = leaf.rfind('.') {
            return (leaf[..idx].to_string(), leaf[idx + 1..].to_string());
        }
        return (leaf.to_string(), String::new());
    }
    if let Some(idx) = leaf.rfind(':') {
        return (leaf[..idx].to_string(), leaf[idx + 1..].to_string());
    }
    (leaf.to_string(), String::new())
}

/// A string array field, cleaned per element.
fn clean_arr(m: &Map<String, J>, key: &str) -> Vec<J> {
    sarr(m, key)
        .iter()
        .map(|p| J::String(clean_path(p)))
        .collect()
}

/// Map every object's full-path to the full-paths that reference it (the
/// engine's `referenced_by` graph builtin, surfaced verbatim).
fn refmap(sources: &[Source], container: &str) -> HashMap<String, Vec<J>> {
    let expr = format!("{container}[] | {{p: .\"full-path\", by: referenced_by(.)}}");
    let mut out: HashMap<String, Vec<J>> = HashMap::new();
    let rows = match query(&expr, sources) {
        Ok(r) => r,
        Err(_) => return out,
    };
    for r in rows {
        if let J::Object(m) = r {
            let p = m.get("p").and_then(J::as_str).unwrap_or("").to_string();
            let by = match m.get("by") {
                Some(J::Array(a)) => a.clone(),
                _ => Vec::new(),
            };
            out.insert(p, by);
        }
    }
    out
}

fn used_by(used: &HashMap<String, Vec<J>>, fp: &str) -> Vec<J> {
    used.get(fp).cloned().unwrap_or_default()
}

// --- per-type shaping --------------------------------------------------------

fn shape_virtual(f: &Map<String, J>) -> J {
    let (addr, port) = split_dest(bstr(f, "destination"));
    let fp = bstr(f, "full-path");
    let disabled = bbool(f, "disabled") || bstr(f, "state") == "disabled";

    let mut v = Map::new();
    v.insert("name".into(), J::String(bstr(f, "name").into()));
    v.insert("fullPath".into(), J::String(fp.into()));
    v.insert("partition".into(), J::String(partition_of(fp)));
    v.insert(
        "destination".into(),
        J::String(bstr(f, "destination").into()),
    );
    v.insert("destAddr".into(), J::String(addr));
    v.insert("destPort".into(), J::String(port));
    v.insert("mask".into(), J::String(bstr(f, "mask").into()));
    v.insert("pool".into(), J::String(clean_field(f, "pool")));
    v.insert("profiles".into(), J::Array(clean_arr(f, "profiles")));
    v.insert("rules".into(), J::Array(clean_arr(f, "rules")));
    v.insert("persist".into(), J::Array(clean_arr(f, "persist")));
    v.insert("policies".into(), J::Array(clean_arr(f, "policies")));
    v.insert("snatpool".into(), J::String(clean_field(f, "snatpool")));
    v.insert(
        "sourceXlate".into(),
        J::String(bstr(f, "source-address-translation").into()),
    );
    v.insert(
        "ipProtocol".into(),
        J::String(bstr(f, "ip-protocol").into()),
    );
    v.insert("source".into(), J::String(bstr(f, "source").into()));
    v.insert("vlans".into(), J::Array(clean_arr(f, "vlans")));
    v.insert("vlansEnabled".into(), J::Bool(bbool(f, "vlans-enabled")));
    v.insert("vlansDisabled".into(), J::Bool(bbool(f, "vlans-disabled")));
    v.insert(
        "description".into(),
        J::String(bstr(f, "description").into()),
    );
    v.insert("disabled".into(), J::Bool(disabled));
    let listener = crate::graph::parse_listener(&v);
    v.insert("listener".into(), listener);
    J::Object(v)
}

fn shape_pool(f: &Map<String, J>, used: &HashMap<String, Vec<J>>) -> J {
    let mut members = Vec::new();
    for m in barr(f, "members") {
        let mf: Map<String, J> = match m {
            J::Object(mm) => mm
                .get("fields")
                .and_then(J::as_object)
                .cloned()
                .unwrap_or_default(),
            _ => Map::new(),
        };
        let name = bstr(&mf, "name").to_string();
        let port = match mf.get("port") {
            Some(v) if truthy(v) => py_str(v),
            _ => {
                if name.contains(':') {
                    name.rsplit(':').next().unwrap_or("").to_string()
                } else {
                    String::new()
                }
            }
        };
        let mut mo = Map::new();
        mo.insert("name".into(), J::String(name));
        mo.insert("address".into(), J::String(bstr(&mf, "address").into()));
        mo.insert("port".into(), J::String(port));
        mo.insert("monitor".into(), J::String(clean_field(&mf, "monitor")));
        mo.insert(
            "ratio".into(),
            mf.get("ratio").cloned().unwrap_or(J::String(String::new())),
        );
        mo.insert(
            "priorityGroup".into(),
            mf.get("priority-group")
                .cloned()
                .unwrap_or(J::String(String::new())),
        );
        mo.insert(
            "connectionLimit".into(),
            mf.get("connection-limit")
                .cloned()
                .unwrap_or(J::String(String::new())),
        );
        mo.insert("state".into(), J::String(bstr(&mf, "state").into()));
        mo.insert(
            "description".into(),
            J::String(bstr(&mf, "description").into()),
        );
        members.push(J::Object(mo));
    }
    let fp = bstr(f, "full-path");
    let member_count = members.len();
    let mut p = Map::new();
    p.insert("name".into(), J::String(bstr(f, "name").into()));
    p.insert("fullPath".into(), J::String(fp.into()));
    p.insert("monitor".into(), J::String(clean_field(f, "monitor")));
    p.insert(
        "lbMode".into(),
        J::String(bstr(f, "load-balancing-mode").into()),
    );
    p.insert("members".into(), J::Array(members));
    p.insert("memberCount".into(), J::from(member_count));
    p.insert("usedBy".into(), J::Array(used_by(used, fp)));
    J::Object(p)
}

fn shape_node(f: &Map<String, J>, used: &HashMap<String, Vec<J>>) -> J {
    let fp = bstr(f, "full-path");
    let mut o = Map::new();
    o.insert("name".into(), J::String(bstr(f, "name").into()));
    o.insert("fullPath".into(), J::String(fp.into()));
    o.insert("address".into(), J::String(bstr(f, "address").into()));
    o.insert("monitor".into(), J::String(clean_field(f, "monitor")));
    o.insert("usedBy".into(), J::Array(used_by(used, fp)));
    J::Object(o)
}

fn shape_monitor(f: &Map<String, J>, used: &HashMap<String, Vec<J>>) -> J {
    let fp = bstr(f, "full-path");
    let mut o = Map::new();
    o.insert("name".into(), J::String(bstr(f, "name").into()));
    o.insert("fullPath".into(), J::String(fp.into()));
    o.insert("type".into(), J::String(bstr(f, "type").into()));
    o.insert(
        "interval".into(),
        f.get("interval")
            .cloned()
            .unwrap_or(J::String(String::new())),
    );
    o.insert(
        "timeout".into(),
        f.get("timeout")
            .cloned()
            .unwrap_or(J::String(String::new())),
    );
    o.insert("send".into(), J::String(bstr(f, "send").into()));
    o.insert("recv".into(), J::String(bstr(f, "recv").into()));
    o.insert("usedBy".into(), J::Array(used_by(used, fp)));
    J::Object(o)
}

fn shape_rule(f: &Map<String, J>, used: &HashMap<String, Vec<J>>) -> J {
    let body = bstr(f, "body").to_string();
    let re = Regex::new(r"\bwhen\s+([A-Z][A-Z0-9_]+)").expect("valid when regex");
    let events: BTreeSet<String> = re.captures_iter(&body).map(|c| c[1].to_string()).collect();
    let fp = bstr(f, "full-path");
    // `.refs` is the engine's synthesised iRule reference sub-object.
    let refs: Map<String, J> = match f.get("refs") {
        Some(J::Object(m)) => m
            .get("fields")
            .and_then(J::as_object)
            .cloned()
            .unwrap_or_default(),
        _ => Map::new(),
    };
    let line_count = if body.is_empty() {
        0
    } else {
        body.matches('\n').count() + 1
    };
    let mut o = Map::new();
    o.insert("name".into(), J::String(bstr(f, "name").into()));
    o.insert("fullPath".into(), J::String(fp.into()));
    o.insert("lineCount".into(), J::from(line_count));
    o.insert(
        "events".into(),
        J::Array(events.into_iter().map(J::String).collect()),
    );
    o.insert("body".into(), J::String(body.clone()));
    o.insert(
        "bodyHtml".into(),
        J::String(tcl_lexer::highlight_tcl(&body)),
    );
    // IR-based control-flow flowchart (Mermaid); empty when there's nothing to
    // draw. The report renders it lazily when the iRule row is expanded.
    o.insert(
        "flowchart".into(),
        J::String(tcl_diagram::irule_flowchart_mermaid(
            &body,
            tcl_registry::registry_for_dialect("f5-irules"),
        )),
    );
    o.insert("usedBy".into(), J::Array(used_by(used, fp)));
    o.insert("refPools".into(), J::Array(clean_arr(&refs, "pools")));
    o.insert(
        "refDataGroups".into(),
        J::Array(clean_arr(&refs, "data-groups")),
    );
    o.insert(
        "dynamicActions".into(),
        J::Array(crate::graph::irule_dynamic_actions(&body)),
    );
    J::Object(o)
}

fn shape_data_group(f: &Map<String, J>, used: &HashMap<String, Vec<J>>) -> J {
    let records = barr(f, "records");
    let fp = bstr(f, "full-path");
    let record_count = records.len();
    let shown: Vec<J> = records
        .iter()
        .take(200)
        .map(|r| J::String(py_str(r)))
        .collect();
    let mut o = Map::new();
    o.insert("name".into(), J::String(bstr(f, "name").into()));
    o.insert("fullPath".into(), J::String(fp.into()));
    o.insert("type".into(), J::String(bstr(f, "type").into()));
    o.insert("recordCount".into(), J::from(record_count));
    o.insert("records".into(), J::Array(shown));
    o.insert("usedBy".into(), J::Array(used_by(used, fp)));
    J::Object(o)
}

fn shape_profile(f: &Map<String, J>, used: &HashMap<String, Vec<J>>) -> J {
    let ptype = bstr(f, "type").replace("ProfileType.", "");
    let fp = bstr(f, "full-path");
    let ciphers = {
        let c = bstr(f, "ciphers");
        if c.is_empty() {
            bstr(f, "cipher-group").to_string()
        } else {
            c.to_string()
        }
    };
    let mut o = Map::new();
    o.insert("name".into(), J::String(bstr(f, "name").into()));
    o.insert("fullPath".into(), J::String(fp.into()));
    o.insert("type".into(), J::String(ptype));
    o.insert("parent".into(), J::String(clean_field(f, "defaults-from")));
    o.insert("ciphers".into(), J::String(ciphers));
    o.insert("cert".into(), J::String(clean_field(f, "cert")));
    o.insert("key".into(), J::String(clean_field(f, "key")));
    o.insert("chain".into(), J::String(clean_field(f, "chain")));
    o.insert("usedBy".into(), J::Array(used_by(used, fp)));
    J::Object(o)
}

fn policy_sub(x: &J) -> Map<String, J> {
    match x {
        J::Object(m) => m
            .get("fields")
            .and_then(J::as_object)
            .cloned()
            .unwrap_or_else(|| m.clone()),
        _ => Map::new(),
    }
}

fn shape_policy(f: &Map<String, J>) -> J {
    let mut rules = Vec::new();
    for r in barr(f, "rules") {
        let rf = policy_sub(r);
        let mut conds = Vec::new();
        for c in barr(&rf, "conditions") {
            let cf = policy_sub(c);
            let mut cm = Map::new();
            cm.insert("operand".into(), J::String(bstr(&cf, "operand").into()));
            cm.insert("selector".into(), J::String(bstr(&cf, "selector").into()));
            cm.insert("operator".into(), J::String(bstr(&cf, "operator").into()));
            cm.insert("values".into(), J::Array(barr(&cf, "values").to_vec()));
            cm.insert("negate".into(), J::Bool(bbool(&cf, "negate")));
            cm.insert(
                "caseInsensitive".into(),
                J::Bool(bbool(&cf, "case-insensitive")),
            );
            conds.push(J::Object(cm));
        }
        let mut acts = Vec::new();
        for a in barr(&rf, "actions") {
            let af = policy_sub(a);
            let mut am = Map::new();
            am.insert("target".into(), J::String(bstr(&af, "target").into()));
            am.insert("verb".into(), J::String(bstr(&af, "verb").into()));
            am.insert("pool".into(), J::String(clean_field(&af, "pool")));
            am.insert("location".into(), J::String(bstr(&af, "location").into()));
            am.insert("host".into(), J::String(bstr(&af, "host").into()));
            am.insert("path".into(), J::String(bstr(&af, "path").into()));
            am.insert("value".into(), J::String(bstr(&af, "value").into()));
            am.insert("name".into(), J::String(bstr(&af, "name").into()));
            acts.push(J::Object(am));
        }
        let mut rm = Map::new();
        rm.insert("name".into(), J::String(bstr(&rf, "name").into()));
        rm.insert(
            "ordinal".into(),
            rf.get("ordinal").cloned().unwrap_or(J::from(0)),
        );
        rm.insert("conditions".into(), J::Array(conds));
        rm.insert("actions".into(), J::Array(acts));
        rules.push(J::Object(rm));
    }
    let fp = bstr(f, "full-path");
    let mut o = Map::new();
    o.insert("name".into(), J::String(bstr(f, "name").into()));
    o.insert("fullPath".into(), J::String(fp.into()));
    o.insert("strategy".into(), J::String(bstr(f, "strategy").into()));
    o.insert("rules".into(), J::Array(rules));
    J::Object(o)
}

// --- model assembly ----------------------------------------------------------

fn device_name(uri: &str, source: &str) -> String {
    let re = Regex::new(r"hostname\s+(\S+)").expect("valid hostname regex");
    if let Some(c) = re.captures(source) {
        return c[1].to_string();
    }
    let base = uri.rsplit('/').next().unwrap_or(uri);
    if base.contains('.') {
        base.rsplitn(2, '.').last().unwrap_or(base).to_string()
    } else {
        base.to_string()
    }
}

fn insight(level: &str, text: String) -> J {
    let mut o = Map::new();
    o.insert("level".into(), J::String(level.into()));
    o.insert("text".into(), J::String(text));
    J::Object(o)
}

fn insights(device: &Map<String, J>) -> Vec<J> {
    let mut out = Vec::new();
    let orphans = device
        .get("orphans")
        .and_then(J::as_object)
        .cloned()
        .unwrap_or_default();
    for kind in ["pools", "nodes", "rules", "monitors", "profiles"] {
        let n = orphans.get(kind).and_then(J::as_array).map_or(0, Vec::len);
        if n > 0 {
            out.push(insight(
                "warn",
                format!("{n} orphaned {kind} (defined, referenced by nothing — no iRule can attach them either)"),
            ));
        }
    }
    // Possible orphans: no static reference, but an iRule selects that type
    // dynamically, so they cannot be *proven* unused.
    let possible = device
        .get("possibleOrphans")
        .and_then(J::as_object)
        .cloned()
        .unwrap_or_default();
    for kind in ["pools", "nodes", "snatpools"] {
        let n = possible.get(kind).and_then(J::as_array).map_or(0, Vec::len);
        if n > 0 {
            out.push(insight(
                "info",
                format!(
                    "{n} {kind} have no static reference but an iRule selects {kind} dynamically — can't be proven unused"
                ),
            ));
        }
    }
    let empty_pools: Vec<&str> = barr(device, "pools")
        .iter()
        .filter_map(J::as_object)
        .filter(|p| p.get("memberCount").and_then(J::as_u64) == Some(0))
        .map(|p| bstr(p, "name"))
        .collect();
    if !empty_pools.is_empty() {
        let mut preview = empty_pools
            .iter()
            .take(6)
            .copied()
            .collect::<Vec<_>>()
            .join(", ");
        if empty_pools.len() > 6 {
            preview.push('…');
        }
        out.push(insight(
            "warn",
            format!("{} pool(s) with no members: {preview}", empty_pools.len()),
        ));
    }
    let no_pool_vs: Vec<&str> = barr(device, "virtuals")
        .iter()
        .filter_map(J::as_object)
        .filter(|v| bstr(v, "pool").is_empty() && barr(v, "policies").is_empty())
        .map(|v| bstr(v, "name"))
        .collect();
    if !no_pool_vs.is_empty() {
        out.push(insight(
            "info",
            format!(
                "{} virtual server(s) with no default pool (forwarding / policy-driven)",
                no_pool_vs.len()
            ),
        ));
    }
    let disabled_vs = barr(device, "virtuals")
        .iter()
        .filter_map(J::as_object)
        .filter(|v| bbool(v, "disabled"))
        .count();
    if disabled_vs > 0 {
        out.push(insight(
            "info",
            format!("{disabled_vs} disabled virtual server(s)"),
        ));
    }
    let ssl = barr(device, "profiles")
        .iter()
        .filter_map(J::as_object)
        .filter(|p| bstr(p, "type").contains("SSL"))
        .count();
    if ssl > 0 {
        out.push(insight("info", format!("{ssl} SSL profile(s) in use")));
    }
    if out.is_empty() {
        out.push(insight(
            "ok",
            "No orphaned objects or empty pools detected".to_string(),
        ));
    }
    out
}

fn collect_device(uri: &str, source: &str) -> J {
    let sources: Vec<Source> = vec![(uri.to_string(), source.to_string())];

    // One reference-graph walk per referable container, up front.
    let refmaps: HashMap<&str, HashMap<String, Vec<J>>> = REFERABLE
        .iter()
        .map(|name| (*name, refmap(&sources, container_path(name))))
        .collect();

    let empty_refmap: HashMap<String, Vec<J>> = HashMap::new();

    let re_tmsh = Regex::new(r"#TMSH-VERSION:\s*(\S+)").expect("valid tmsh regex");
    let tmsh = re_tmsh
        .captures(source)
        .map(|c| c[1].to_string())
        .unwrap_or_default();

    let mut device = Map::new();
    device.insert("uri".into(), J::String(uri.into()));
    device.insert("name".into(), J::String(device_name(uri, source)));
    device.insert("tmshVersion".into(), J::String(tmsh));

    for (key, container) in CONTAINERS {
        let rows = fields_of(query(&format!("{container}[]"), &sources).unwrap_or_default());
        let used = refmaps.get(key).unwrap_or(&empty_refmap);
        let shaped: Vec<J> = match *key {
            "virtuals" => rows.iter().map(shape_virtual).collect(),
            "pools" => rows.iter().map(|f| shape_pool(f, used)).collect(),
            "nodes" => rows.iter().map(|f| shape_node(f, used)).collect(),
            "monitors" => rows.iter().map(|f| shape_monitor(f, used)).collect(),
            "rules" => rows.iter().map(|f| shape_rule(f, used)).collect(),
            "dataGroups" => rows.iter().map(|f| shape_data_group(f, used)).collect(),
            "profiles" => rows.iter().map(|f| shape_profile(f, used)).collect(),
            "policies" => rows.iter().map(shape_policy).collect(),
            // snatpools / persistence / virtual-addresses: keep the projected
            // fields, tidy up the name/full-path for display; carry `usedBy`.
            _ => rows
                .iter()
                .map(|f| {
                    let fp = bstr(f, "full-path");
                    let mut o = Map::new();
                    o.insert("name".into(), J::String(bstr(f, "name").into()));
                    o.insert("fullPath".into(), J::String(fp.into()));
                    o.insert("usedBy".into(), J::Array(used_by(used, fp)));
                    o.insert("fields".into(), J::Object(f.clone()));
                    J::Object(o)
                })
                .collect(),
        };
        device.insert((*key).into(), J::Array(shaped));
    }

    // Orphans: a referable leaf object is *confirmed* orphaned only when it has
    // an empty referrer set AND no iRule could dynamically attach its type. When
    // an iRule selects that type dynamically (`pool $x`, `pool [class match …]`)
    // we cannot prove the object is unreachable, so it is a *possible* orphan.
    let risk: std::collections::BTreeSet<&'static str> = device
        .get("rules")
        .and_then(J::as_array)
        .map(|rules| crate::graph::dynamic_attach_risk(rules))
        .unwrap_or_default();
    device.insert(
        "orphanRisk".into(),
        J::Array(risk.iter().map(|t| J::String((*t).into())).collect()),
    );

    let mut orphans = Map::new();
    let mut possible = Map::new();
    for name in REFERABLE {
        let at_risk = risk.contains(name);
        // Annotate each object with its orphan status, then collect the sets.
        if let Some(J::Array(objs)) = device.get_mut(*name) {
            for o in objs.iter_mut() {
                if let Some(om) = o.as_object_mut() {
                    let empty = om
                        .get("usedBy")
                        .and_then(J::as_array)
                        .is_none_or(Vec::is_empty);
                    let status = if !empty {
                        ""
                    } else if at_risk {
                        "possible"
                    } else {
                        "orphan"
                    };
                    om.insert("orphanStatus".into(), J::String(status.into()));
                }
            }
        }
        if let Some(J::Array(objs)) = device.get(*name) {
            let confirmed: Vec<J> = objs
                .iter()
                .filter_map(J::as_object)
                .filter(|o| bstr(o, "orphanStatus") == "orphan")
                .map(|o| J::String(bstr(o, "name").into()))
                .collect();
            let maybe: Vec<J> = objs
                .iter()
                .filter_map(J::as_object)
                .filter(|o| bstr(o, "orphanStatus") == "possible")
                .map(|o| J::String(bstr(o, "name").into()))
                .collect();
            orphans.insert((*name).into(), J::Array(confirmed));
            possible.insert((*name).into(), J::Array(maybe));
        }
    }
    device.insert("orphans".into(), J::Object(orphans));
    device.insert("possibleOrphans".into(), J::Object(possible));

    // Propagate each iRule's dynamic (runtime) actions onto the virtuals that
    // attach it.
    let mut rule_actions: HashMap<String, Vec<J>> = HashMap::new();
    let mut rule_by_name: HashMap<String, String> = HashMap::new();
    if let Some(J::Array(rules)) = device.get("rules") {
        for r in rules {
            if let Some(rm) = r.as_object() {
                let fp = bstr(rm, "fullPath").to_string();
                rule_actions.insert(fp.clone(), barr(rm, "dynamicActions").to_vec());
                rule_by_name.insert(leaf(&fp).to_string(), fp);
            }
        }
    }
    if let Some(J::Array(virtuals)) = device.get_mut("virtuals") {
        for v in virtuals.iter_mut() {
            let rule_refs: Vec<String> = v
                .as_object()
                .map(|vm| sarr(vm, "rules").iter().map(|s| (*s).to_string()).collect())
                .unwrap_or_default();
            let mut acts: Vec<J> = Vec::new();
            for rule_ref in &rule_refs {
                let fp = if rule_actions.contains_key(rule_ref) {
                    rule_ref.clone()
                } else {
                    rule_by_name
                        .get(leaf(rule_ref))
                        .cloned()
                        .unwrap_or_default()
                };
                if let Some(actions) = rule_actions.get(&fp) {
                    for a in actions {
                        if let Some(am) = a.as_object() {
                            let mut na = am.clone();
                            na.insert("rule".into(), J::String(leaf(rule_ref).into()));
                            acts.push(J::Object(na));
                        }
                    }
                }
            }
            if let Some(vm) = v.as_object_mut() {
                vm.insert("dynamicProfiles".into(), J::Array(acts));
            }
        }
    }

    device.insert("graph".into(), crate::graph::build_graph(&device));
    // The raw SCF/config text, embedded so the in-browser wasm query console can
    // run live queries against this exact device.
    device.insert("configText".into(), J::String(source.into()));

    // Certificate inventory (SSL cert expiry & inventory tab).
    let certs = crate::certs::collect_certs(&sources, &device);
    device.insert("certificates".into(), certs);

    // Secret inventory (Secrets tab). Values are clear text only when the
    // config was decrypted with the f5mku master key upstream.
    device.insert(
        "secrets".into(),
        J::Array(crate::secrets::collect_secrets(source)),
    );

    // Counts.
    let mut counts = Map::new();
    for (key, _) in CONTAINERS {
        let n = device.get(*key).and_then(J::as_array).map_or(0, Vec::len);
        counts.insert((*key).into(), J::from(n));
    }
    let pool_members: usize = barr(&device, "pools")
        .iter()
        .filter_map(J::as_object)
        .map(|p| p.get("memberCount").and_then(J::as_u64).unwrap_or(0) as usize)
        .sum();
    counts.insert("poolMembers".into(), J::from(pool_members));
    let orphan_total: usize = device.get("orphans").and_then(J::as_object).map_or(0, |o| {
        o.values().filter_map(J::as_array).map(Vec::len).sum()
    });
    counts.insert("orphans".into(), J::from(orphan_total));
    let cert_total = device
        .get("certificates")
        .and_then(J::as_array)
        .map_or(0, Vec::len);
    counts.insert("certificates".into(), J::from(cert_total));
    let secret_total = device
        .get("secrets")
        .and_then(J::as_array)
        .map_or(0, Vec::len);
    counts.insert("secrets".into(), J::from(secret_total));
    device.insert("counts".into(), J::Object(counts));

    let ins = insights(&device);
    device.insert("insights".into(), J::Array(ins));
    J::Object(device)
}

/// Build the full report model from loaded `(uri, text)` sources.
pub fn collect_model(sources: &[Source], title: &str) -> J {
    let devices: Vec<J> = sources
        .iter()
        .map(|(uri, src)| collect_device(uri, src))
        .collect();

    let mut totals: Map<String, J> = Map::new();
    for d in &devices {
        if let Some(counts) = d.get("counts").and_then(J::as_object) {
            for (k, v) in counts {
                let add = v.as_u64().unwrap_or(0);
                let cur = totals.get(k).and_then(J::as_u64).unwrap_or(0);
                totals.insert(k.clone(), J::from(cur + add));
            }
        }
    }

    let container_order: Vec<J> = CONTAINERS
        .iter()
        .map(|(k, _)| J::String((*k).into()))
        .collect();

    let mut model = Map::new();
    model.insert("title".into(), J::String(title.into()));
    model.insert("engine_version".into(), J::String(ENGINE_VERSION.into()));
    model.insert("devices".into(), J::Array(devices));
    model.insert("totals".into(), J::Object(totals));
    model.insert("container_order".into(), J::Array(container_order));
    J::Object(model)
}
