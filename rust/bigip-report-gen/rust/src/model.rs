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

//! Build a structured report model from BIG-IP configs using the query engine.
//!
//! A faithful port of `f5report.report`. Every fact is pulled from the native
//! `f5-query` engine ([`crate::query`]) — the parsing, object projection and the
//! `referenced_by` reference-graph walk that powers the orphan / dependency
//! analysis. This module only shapes the engine's output into a
//! template-friendly model (a `serde_json` object, so it embeds verbatim in the
//! report for the client-side topology / listener / console views).

use std::collections::{BTreeSet, HashMap};
use std::sync::OnceLock;

use regex::Regex;
use serde_json::{Map, Value as J};
use tcl_registry::events::EventRegistry;
use tcl_registry::profile_defaults::{BigipVersion, profile_field_defaults};

use crate::jutil::{barr, bbool, bstr, sarr, truthy};
use crate::query::{Source, query};

/// Whether `uri` names an iApp presentation source. The LSP recognises the
/// same conventional names; reports use the URI because a UCS/SCF source has
/// no separate language-id field.
fn is_iapp_presentation_uri(uri: &str) -> bool {
    let basename = uri.rsplit('/').next().unwrap_or(uri);
    basename.eq_ignore_ascii_case("presentation")
        || basename
            .rsplit_once('.')
            .is_some_and(|(_, ext)| ext.eq_ignore_ascii_case("apl"))
}

/// Whether `uri` names an iApp implementation that can be paired with a
/// presentation in the same directory.
fn is_iapp_implementation_uri(uri: &str) -> bool {
    let basename = uri.rsplit('/').next().unwrap_or(uri);
    basename.eq_ignore_ascii_case("implementation")
        || basename.rsplit_once('.').is_some_and(|(_, ext)| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "iapp" | "iappimpl" | "impl"
            )
        })
}

fn uri_directory(uri: &str) -> &str {
    uri.rsplit_once('/').map_or("", |(dir, _)| dir)
}

fn iapp_peer<'a>(uri: &str, sources: &'a [Source], presentation: bool) -> Option<&'a str> {
    sources
        .iter()
        .find(|(candidate_uri, _)| {
            uri_directory(candidate_uri) == uri_directory(uri)
                && if presentation {
                    is_iapp_implementation_uri(candidate_uri)
                } else {
                    is_iapp_presentation_uri(candidate_uri)
                }
        })
        .map(|(_, source)| source.as_str())
}

/// Evidence describing whether a report had the complete iApp pair needed for
/// cross-file validation. It is deliberately data, rather than a silent
/// absence of IAPP7001/7002, so a reviewer can distinguish "clean" from
/// "could not inspect".
#[must_use]
pub fn collect_iapp_diagnostic_evidence(uri: &str, sources: &[Source]) -> J {
    let presentation = is_iapp_presentation_uri(uri);
    let implementation = is_iapp_implementation_uri(uri);
    let peer = if presentation || implementation {
        iapp_peer(uri, sources, presentation)
    } else {
        None
    };
    match (presentation, implementation, peer.is_some()) {
        (true, false, true) => serde_json::json!({
            "state": "complete",
            "message": "Presentation and implementation were supplied; cross-file iApp checks are complete.",
        }),
        (true, false, false) => serde_json::json!({
            "state": "presentation_only",
            "message": "Only the iApp presentation was supplied; implementation-reference checks (IAPP7001 and IAPP7002) could not be evaluated.",
        }),
        (false, true, true) => serde_json::json!({
            "state": "complete",
            "message": "Implementation and presentation were supplied; cross-file iApp checks are complete.",
        }),
        (false, true, false) => serde_json::json!({
            "state": "implementation_only",
            "message": "Only the iApp implementation was supplied; presentation-field checks (IAPP7001) could not be evaluated.",
        }),
        _ => J::Null,
    }
}

fn f5_diagnostic_json(diagnostic: &tcl_bigip::validator::ConfigDiagnostic) -> J {
    use tcl_bigip::validator::DiagSeverity;

    serde_json::json!({
        "code": diagnostic.code,
        "message": diagnostic.message,
        "severity": match diagnostic.severity {
            DiagSeverity::Warning => "warning",
            DiagSeverity::Hint => "hint",
        },
        "line": diagnostic.range.start.line + 1,
        "column": diagnostic.range.start.character + 1,
        "tab": diagnostic.subject.report_tab(),
    })
}

/// Run the shared BIG-IP config validator and expose every finding to the
/// report. The complete list is retained as `configDiagnostics`; routed lists
/// let the template render findings in the tab where they are actionable.
#[must_use]
pub fn collect_config_diagnostics(source: &str) -> J {
    J::Array(
        tcl_bigip::validator::validate_bigip_source(source, "Common")
            .iter()
            .map(f5_diagnostic_json)
            .collect(),
    )
}

/// Collect model and iApp diagnostics for one report source. The conventional
/// iApp presentation/implementation filenames let a multi-file report perform
/// every IAPP7001–7003 check offline; partial input remains explicit through
/// [`collect_iapp_diagnostic_evidence`].
#[must_use]
pub fn collect_report_diagnostics(uri: &str, source: &str, sources: &[Source]) -> J {
    let mut diagnostics = tcl_bigip::validator::validate_bigip_source(source, "Common");
    if is_iapp_presentation_uri(uri) {
        let model = tcl_bigip::apl::parse_apl(source);
        let refs = iapp_peer(uri, sources, true).map(tcl_bigip::apl::extract_iapp_var_refs);
        diagnostics.extend(tcl_bigip::apl::validate_iapp_presentation(
            &model,
            refs.as_deref(),
            "f5-iapps",
        ));
    } else if is_iapp_implementation_uri(uri)
        && let Some(presentation) = iapp_peer(uri, sources, false)
    {
        let model = tcl_bigip::apl::parse_apl(presentation);
        let refs = tcl_bigip::apl::extract_iapp_var_refs(source);
        diagnostics.extend(tcl_bigip::apl::validate_iapp_implementation(
            &refs,
            Some(&model),
            "f5-iapps",
        ));
    }
    J::Array(diagnostics.iter().map(f5_diagnostic_json).collect())
}

fn add_config_diagnostics(
    device: &mut Map<String, J>,
    uri: &str,
    source: &str,
    sources: &[Source],
) {
    let J::Array(all) = collect_report_diagnostics(uri, source, sources) else {
        unreachable!("config diagnostics are always an array")
    };
    let mut routed: HashMap<String, Vec<J>> = HashMap::new();
    for item in &all {
        let tab = item.get("tab").and_then(J::as_str).unwrap_or("virtuals");
        routed.entry(tab.to_owned()).or_default().push(item.clone());
    }

    for subject in tcl_bigip::validator::ConfigDiagnosticSubject::ALL {
        let tab = subject.report_tab();
        let key = match tab {
            "virtuals" => "virtualDiagnostics",
            "rules" => "ruleDiagnostics",
            "pools" => "poolDiagnostics",
            "dataGroups" => "dataGroupDiagnostics",
            "apps" => "appDiagnostics",
            "objectIndex" => "objectDiagnostics",
            _ => unreachable!("every configuration diagnostic subject has a report tab"),
        };
        device.insert(key.into(), J::Array(routed.remove(tab).unwrap_or_default()));
    }
    device.insert("configDiagnostics".into(), J::Array(all));
    device.insert(
        "iappDiagnosticEvidence".into(),
        collect_iapp_diagnostic_evidence(uri, sources),
    );
}

/// The engine version string embedded in the report header.
///
/// `tcl_version::VERSION`, not `CARGO_PKG_VERSION`: the workspace manifest
/// carries `0.1.0` and is never bumped (releases are tag-only), so the manifest
/// version made every report's header read `query engine v0.1.0` regardless of
/// what was actually shipped. `tcl-version` resolves the tag, plus the commit as
/// build metadata (`2.1.8+g4c5a8f86`).
pub const ENGINE_VERSION: &str = tcl_version::VERSION;

/// The short git commit hash the generator was built from, stamped by
/// `build.rs`. `"unknown"` when the build carried no git metadata.
pub const GIT_HASH: &str = env!("GIT_HASH");

/// The single `git describe --tags` version (nearest `v*` tag + commits-since +
/// short hash, e.g. `v1.2.3-4-gabcdef0`) shown in the footer. Stamped by
/// `build.rs`; `"unknown"` when the build carried no git metadata.
pub const GIT_DESCRIBE: &str = env!("GIT_DESCRIBE");

/// The iRule event registry — the source of truth for canonical event firing
/// order. Built once (the data is compiled into the binary) and reused across
/// every shaped rule. (Profile *traffic* order lives in the shared
/// `tcl_bigip_query::builtins::f5profile` engine core.)
fn event_registry() -> &'static EventRegistry {
    static R: OnceLock<EventRegistry> = OnceLock::new();
    R.get_or_init(EventRegistry::build)
}

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

// GTM object containers the report walks, in display order. A GTM (DNS) tier
// fronts the LTM tiers; surfacing these lets the report show the wide-IP ->
// pool -> server chain and link the GTM to the downstream LTM virtuals.
const GTM_CONTAINERS: &[(&str, &str)] = &[
    ("gtmWideips", ".gtm.wideip"),
    ("gtmPools", ".gtm.pool"),
    ("gtmServers", ".gtm.server"),
    ("gtmDatacenters", ".gtm.datacenter"),
    ("gtmListeners", ".gtm.listener"),
];

// AFM security-firewall + NAT object containers. Surfacing these gives the
// report a firewall / NAT posture view: the policies and rule-lists, the
// address-/port-lists they match on, and the NAT policies and translations.
const SECURITY_CONTAINERS: &[(&str, &str)] = &[
    ("firewallPolicies", ".security.\"firewall-policy\""),
    ("firewallRuleLists", ".security.\"firewall-rule-list\""),
    (
        "firewallAddressLists",
        ".security.\"firewall-address-list\"",
    ),
    ("firewallPortLists", ".security.\"firewall-port-list\""),
    ("natPolicies", ".security.\"nat-policy\""),
    (
        "natSourceTranslations",
        ".security.\"nat-source-translation\"",
    ),
    (
        "natDestinationTranslations",
        ".security.\"nat-destination-translation\"",
    ),
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

/// Object-list keys that carry a displayed, partition-scoped object (used to tag
/// each object with its partition for the partition filter).
const DISPLAY_KEYS: &[&str] = &[
    "virtuals",
    "pools",
    "nodes",
    "monitors",
    "rules",
    "dataGroups",
    "profiles",
    "policies",
    "snatpools",
    "persistence",
    "certificates",
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

/// Decompose a BIG-IP full path `/partition/seg1/…/name` into
/// `(partition, [folder segments], name)`. Folder segments are the parts
/// between the partition and the leaf name (empty when the object lives in the
/// partition root). Returns `None` for a non-`/`-rooted or partition-less path.
fn split_full_path(full_path: &str) -> Option<(&str, Vec<&str>, &str)> {
    let rest = full_path.strip_prefix('/')?;
    let parts: Vec<&str> = rest.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() < 2 {
        return None; // need at least partition + name
    }
    let partition = parts[0];
    let name = *parts.last().unwrap();
    let segments = parts[1..parts.len() - 1].to_vec();
    Some((partition, segments, name))
}

/// The folder path of an object (`/Common/App_X`), or `""` when it sits in the
/// partition root. Used to display and group objects by folder.
fn folder_of(full_path: &str) -> String {
    match split_full_path(full_path) {
        Some((partition, segments, _)) if !segments.is_empty() => {
            format!("/{partition}/{}", segments.join("/"))
        }
        _ => String::new(),
    }
}

/// The "application folder" key of an object: the partition plus its first
/// sub-folder segment — the folder BIG-IP treats as an application boundary
/// (an iApp materialises its objects under a `<name>.app` folder, so this also
/// captures iApps). Returns `None` for objects that live directly in the
/// partition root (no sub-folder) — those are deliberately NOT grouped into an
/// app, so a partition with no sub-folders yields no folder-apps.
fn app_folder_of(full_path: &str) -> Option<(String, String)> {
    let (partition, segments, _) = split_full_path(full_path)?;
    let first = segments.first()?;
    Some((partition.to_owned(), (*first).to_owned()))
}

/// Singular object-kind label for a [`DISPLAY_KEYS`] container name.
fn singular_kind(display_key: &str) -> &'static str {
    match display_key {
        "virtuals" => "virtual",
        "pools" => "pool",
        "nodes" => "node",
        "monitors" => "monitor",
        "rules" => "rule",
        "dataGroups" => "data-group",
        "profiles" => "profile",
        "policies" => "policy",
        "snatpools" => "snatpool",
        "persistence" => "persistence",
        "certificates" => "certificate",
        _ => "object",
    }
}

/// Derive applications from folder grouping.
///
/// An application is the set of objects that share an **application folder**
/// (partition + first sub-folder segment, via [`app_folder_of`]). Objects that
/// live directly in the partition root are deliberately left ungrouped — so a
/// partition with no sub-folders produces no apps (no catch-all). iApps are
/// captured for free, because their objects live under a `<name>.app` folder.
///
/// Membership is the folder itself; the report's ref graph / drawer still show
/// how the members connect. Each app: `{ name, partition, folder, source,
/// entryPoints, members:[{kind, name, fullPath, partition}], memberCount }`.
fn build_apps(device: &Map<String, J>) -> J {
    use std::collections::BTreeMap;

    struct Acc {
        members: Vec<J>,
        entry_points: Vec<J>,
    }
    let mut apps: BTreeMap<(String, String), Acc> = BTreeMap::new();

    for key in DISPLAY_KEYS {
        let kind = singular_kind(key);
        if let Some(J::Array(objs)) = device.get(*key) {
            for o in objs.iter().filter_map(J::as_object) {
                let fp = bstr(o, "fullPath");
                let Some((part, seg)) = app_folder_of(fp) else {
                    continue;
                };
                let acc = apps.entry((part.clone(), seg)).or_insert_with(|| Acc {
                    members: Vec::new(),
                    entry_points: Vec::new(),
                });
                let mut m = Map::new();
                m.insert("kind".into(), J::String(kind.into()));
                m.insert("name".into(), J::String(bstr(o, "name").into()));
                m.insert("fullPath".into(), J::String(fp.into()));
                m.insert("partition".into(), J::String(part));
                if *key == "virtuals" {
                    acc.entry_points.push(J::String(fp.into()));
                }
                acc.members.push(J::Object(m));
            }
        }
    }

    // BTreeMap iterates sorted by (partition, folder segment) — stable output.
    let out: Vec<J> = apps
        .into_iter()
        .map(|((part, seg), acc)| {
            // An iApp materialises its objects under a `<name>.app` folder
            // (always lowercase `.app` in TMOS).
            let stripped = seg.strip_suffix(".app");
            let is_iapp = stripped.is_some();
            let display_name = stripped.unwrap_or(&seg).to_owned();
            let mut a = Map::new();
            a.insert("name".into(), J::String(display_name));
            a.insert("partition".into(), J::String(part.clone()));
            a.insert("folder".into(), J::String(format!("/{part}/{seg}")));
            a.insert(
                "source".into(),
                J::String(if is_iapp { "iapp" } else { "folder" }.into()),
            );
            a.insert("memberCount".into(), J::from(acc.members.len()));
            a.insert("entryPoints".into(), J::Array(acc.entry_points));
            a.insert("members".into(), J::Array(acc.members));
            J::Object(a)
        })
        .collect();

    J::Array(out)
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
    // Parse through the shared event-handler owner, then order into canonical
    // firing order rather than alphabetical order (which scrambles the
    // lifecycle, e.g. CLIENTSSL_HANDSHAKE ahead of CLIENT_ACCEPTED).
    let command_registry = tcl_registry::registry_for_dialect("f5-irules");
    let identities =
        tcl_compiler::head_identity::command_head_identities(&body, "f5-irules", command_registry);
    let discovered: BTreeSet<String> =
        tcl_registry::events::top_level_when_handlers_with_registry_and_head_resolver(
            &body,
            command_registry,
            &identities,
        )
        .into_iter()
        .map(|handler| handler.event)
        .collect();
    let events = event_registry().order_events(&discovered.into_iter().collect::<Vec<_>>());
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
        J::String(tcl_lexer::highlight_tcl_with_config(
            &body,
            tcl_lexer::LexerConfig::for_dialect("f5-irules"),
        )),
    );
    // IR-based control-flow graph ({nodes, edges} JSON for elkjs); empty when
    // there's nothing to draw. The report renders it lazily when the iRule row
    // is expanded.
    o.insert(
        "flowchart".into(),
        J::String(tcl_diagram::irule_flowchart_graph(
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
    // The reconstructed dynamic-attach name patterns and their resolved objects
    // are attached later as `referencedObjects` (once the device's object lists
    // and per-virtual partition contexts are known — see
    // `annotate_rule_reachability`).
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

// --- GTM shaping -------------------------------------------------------------

/// `map[key]` as an array of plain strings (skips non-strings).
fn str_array(m: &Map<String, J>, key: &str) -> Vec<J> {
    sarr(m, key)
        .iter()
        .map(|s| J::String((*s).into()))
        .collect()
}

fn shape_gtm_wideip(f: &Map<String, J>, used: &HashMap<String, Vec<J>>) -> J {
    let fp = bstr(f, "full-path");
    let mut o = Map::new();
    o.insert("name".into(), J::String(bstr(f, "name").into()));
    o.insert("fullPath".into(), J::String(fp.into()));
    o.insert(
        "recordType".into(),
        J::String(bstr(f, "record-type").into()),
    );
    o.insert("pools".into(), J::Array(clean_arr(f, "pools")));
    o.insert("aliases".into(), J::Array(str_array(f, "aliases")));
    o.insert(
        "poolLbMode".into(),
        J::String(bstr(f, "pool-lb-mode").into()),
    );
    o.insert("state".into(), J::String(bstr(f, "state").into()));
    o.insert(
        "description".into(),
        J::String(bstr(f, "description").into()),
    );
    o.insert("usedBy".into(), J::Array(used_by(used, fp)));
    J::Object(o)
}

fn shape_gtm_pool(f: &Map<String, J>, used: &HashMap<String, Vec<J>>) -> J {
    let members: Vec<J> = barr(f, "members")
        .iter()
        .filter_map(J::as_object)
        .map(|mm| {
            let mut mo = Map::new();
            mo.insert("name".into(), J::String(bstr(mm, "name").into()));
            mo.insert(
                "servicePort".into(),
                J::String(bstr(mm, "service-port").into()),
            );
            mo.insert("ratio".into(), J::String(bstr(mm, "ratio").into()));
            mo.insert(
                "staticTarget".into(),
                J::String(bstr(mm, "static-target").into()),
            );
            mo.insert("state".into(), J::String(bstr(mm, "state").into()));
            J::Object(mo)
        })
        .collect();
    let fp = bstr(f, "full-path");
    let member_count = members.len();
    let mut o = Map::new();
    o.insert("name".into(), J::String(bstr(f, "name").into()));
    o.insert("fullPath".into(), J::String(fp.into()));
    o.insert(
        "recordType".into(),
        J::String(bstr(f, "record-type").into()),
    );
    o.insert(
        "lbMode".into(),
        J::String(bstr(f, "load-balancing-mode").into()),
    );
    o.insert("monitor".into(), J::String(clean_field(f, "monitor")));
    o.insert("members".into(), J::Array(members));
    o.insert("memberCount".into(), J::from(member_count));
    o.insert("state".into(), J::String(bstr(f, "state").into()));
    o.insert("usedBy".into(), J::Array(used_by(used, fp)));
    J::Object(o)
}

fn shape_gtm_server(f: &Map<String, J>, used: &HashMap<String, Vec<J>>) -> J {
    let fp = bstr(f, "full-path");
    let mut o = Map::new();
    o.insert("name".into(), J::String(bstr(f, "name").into()));
    o.insert("fullPath".into(), J::String(fp.into()));
    o.insert("datacenter".into(), J::String(clean_field(f, "datacenter")));
    o.insert("product".into(), J::String(bstr(f, "product").into()));
    o.insert("monitor".into(), J::String(clean_field(f, "monitor")));
    o.insert("addresses".into(), J::Array(str_array(f, "addresses")));
    // The raw virtual-server destinations (e.g. `10.2.0.20:443`) — the report's
    // architecture layer resolves these against downstream devices' served
    // virtual addresses to link this GTM to the LTM tiers it balances.
    o.insert(
        "virtualServers".into(),
        J::Array(str_array(f, "virtual-servers")),
    );
    o.insert("state".into(), J::String(bstr(f, "state").into()));
    o.insert("usedBy".into(), J::Array(used_by(used, fp)));
    J::Object(o)
}

fn shape_gtm_datacenter(f: &Map<String, J>) -> J {
    let fp = bstr(f, "full-path");
    let mut o = Map::new();
    o.insert("name".into(), J::String(bstr(f, "name").into()));
    o.insert("fullPath".into(), J::String(fp.into()));
    o.insert("location".into(), J::String(bstr(f, "location").into()));
    o.insert("contact".into(), J::String(bstr(f, "contact").into()));
    o.insert("state".into(), J::String(bstr(f, "state").into()));
    J::Object(o)
}

fn shape_gtm_listener(f: &Map<String, J>) -> J {
    let fp = bstr(f, "full-path");
    let mut o = Map::new();
    o.insert("name".into(), J::String(bstr(f, "name").into()));
    o.insert("fullPath".into(), J::String(fp.into()));
    o.insert("address".into(), J::String(bstr(f, "address").into()));
    o.insert("port".into(), J::String(bstr(f, "port").into()));
    o.insert("pool".into(), J::String(clean_field(f, "pool")));
    o.insert("state".into(), J::String(bstr(f, "state").into()));
    J::Object(o)
}

// --- Security (firewall + NAT) shaping ---------------------------------------

/// Shape a firewall-rule endpoint (source / destination) into flat string lists.
fn shape_fw_endpoint(m: &Map<String, J>) -> J {
    let mut o = Map::new();
    o.insert("addresses".into(), J::Array(str_array(m, "addresses")));
    o.insert(
        "addressLists".into(),
        J::Array(clean_arr(m, "address-lists")),
    );
    o.insert("ports".into(), J::Array(str_array(m, "ports")));
    o.insert("portLists".into(), J::Array(clean_arr(m, "port-lists")));
    J::Object(o)
}

fn shape_fw_rule(m: &Map<String, J>) -> J {
    let empty = Map::new();
    let src = m.get("source").and_then(J::as_object).unwrap_or(&empty);
    let dst = m
        .get("destination")
        .and_then(J::as_object)
        .unwrap_or(&empty);
    let mut o = Map::new();
    o.insert("name".into(), J::String(bstr(m, "name").into()));
    o.insert("action".into(), J::String(bstr(m, "action").into()));
    o.insert(
        "ipProtocol".into(),
        J::String(bstr(m, "ip-protocol").into()),
    );
    o.insert("log".into(), J::Bool(bbool(m, "log")));
    o.insert("source".into(), shape_fw_endpoint(src));
    o.insert("destination".into(), shape_fw_endpoint(dst));
    o.insert("ruleList".into(), J::String(clean_field(m, "rule-list")));
    J::Object(o)
}

fn shape_fw_policy(f: &Map<String, J>) -> J {
    let mut o = Map::new();
    o.insert("name".into(), J::String(bstr(f, "name").into()));
    o.insert("fullPath".into(), J::String(bstr(f, "full-path").into()));
    o.insert("rules".into(), J::Array(str_array(f, "rules")));
    o.insert("ruleLists".into(), J::Array(clean_arr(f, "rule-lists")));
    o.insert(
        "description".into(),
        J::String(bstr(f, "description").into()),
    );
    J::Object(o)
}

fn shape_fw_rule_list(f: &Map<String, J>) -> J {
    let rules: Vec<J> = barr(f, "rules")
        .iter()
        .filter_map(J::as_object)
        .map(shape_fw_rule)
        .collect();
    let rule_count = rules.len();
    let mut o = Map::new();
    o.insert("name".into(), J::String(bstr(f, "name").into()));
    o.insert("fullPath".into(), J::String(bstr(f, "full-path").into()));
    o.insert("rules".into(), J::Array(rules));
    o.insert("ruleCount".into(), J::from(rule_count));
    o.insert(
        "description".into(),
        J::String(bstr(f, "description").into()),
    );
    J::Object(o)
}

fn shape_fw_address_list(f: &Map<String, J>) -> J {
    let mut o = Map::new();
    o.insert("name".into(), J::String(bstr(f, "name").into()));
    o.insert("fullPath".into(), J::String(bstr(f, "full-path").into()));
    o.insert("addresses".into(), J::Array(str_array(f, "addresses")));
    o.insert(
        "addressLists".into(),
        J::Array(clean_arr(f, "address-lists")),
    );
    o.insert("fqdns".into(), J::Array(str_array(f, "fqdns")));
    o.insert(
        "description".into(),
        J::String(bstr(f, "description").into()),
    );
    J::Object(o)
}

fn shape_fw_port_list(f: &Map<String, J>) -> J {
    let mut o = Map::new();
    o.insert("name".into(), J::String(bstr(f, "name").into()));
    o.insert("fullPath".into(), J::String(bstr(f, "full-path").into()));
    o.insert("ports".into(), J::Array(str_array(f, "ports")));
    o.insert(
        "description".into(),
        J::String(bstr(f, "description").into()),
    );
    J::Object(o)
}

fn shape_nat_policy(f: &Map<String, J>) -> J {
    let mut o = Map::new();
    o.insert("name".into(), J::String(bstr(f, "name").into()));
    o.insert("fullPath".into(), J::String(bstr(f, "full-path").into()));
    o.insert("rules".into(), J::Array(str_array(f, "rules")));
    o.insert("ruleLists".into(), J::Array(clean_arr(f, "rule-lists")));
    o.insert(
        "description".into(),
        J::String(bstr(f, "description").into()),
    );
    J::Object(o)
}

/// Shared shaper for the two NAT translation kinds (same projected shape).
fn shape_nat_translation(f: &Map<String, J>) -> J {
    let mut o = Map::new();
    o.insert("name".into(), J::String(bstr(f, "name").into()));
    o.insert("fullPath".into(), J::String(bstr(f, "full-path").into()));
    o.insert("type".into(), J::String(bstr(f, "type").into()));
    o.insert("addresses".into(), J::Array(str_array(f, "addresses")));
    o.insert("ports".into(), J::Array(str_array(f, "ports")));
    o.insert(
        "description".into(),
        J::String(bstr(f, "description").into()),
    );
    J::Object(o)
}

fn shape_profile(
    f: &Map<String, J>,
    used: &HashMap<String, Vec<J>>,
    bigip_version: Option<BigipVersion>,
) -> J {
    let ptype = bstr(f, "type").replace("ProfileType.", "");
    let fp = bstr(f, "full-path");
    let mut default_fields = Map::new();
    for (field, value) in profile_field_defaults(&ptype, bigip_version) {
        default_fields.insert(field.to_owned(), J::String(value.to_owned()));
    }
    let mut effective_fields = default_fields.clone();
    for (field, value) in f {
        if matches!(field.as_str(), "name" | "full-path" | "type") {
            continue;
        }
        if let Some(value) = value.as_str().filter(|value| !value.is_empty()) {
            effective_fields.insert(field.clone(), J::String(clean_path(value)));
        }
    }
    let ciphers = {
        let ciphers = bstr(&effective_fields, "ciphers");
        let cipher_group = bstr(&effective_fields, "cipher-group");
        if ciphers.is_empty() || ciphers == "none" {
            cipher_group.to_owned()
        } else {
            ciphers.to_owned()
        }
    };
    let mut o = Map::new();
    o.insert("name".into(), J::String(bstr(f, "name").into()));
    o.insert("fullPath".into(), J::String(fp.into()));
    o.insert("type".into(), J::String(ptype));
    o.insert("parent".into(), J::String(clean_field(f, "defaults-from")));
    o.insert("ciphers".into(), J::String(ciphers));
    o.insert(
        "cert".into(),
        J::String(clean_field(&effective_fields, "cert")),
    );
    o.insert(
        "key".into(),
        J::String(clean_field(&effective_fields, "key")),
    );
    o.insert(
        "chain".into(),
        J::String(clean_field(&effective_fields, "chain")),
    );
    o.insert("defaultFields".into(), J::Object(default_fields));
    o.insert("effectiveFields".into(), J::Object(effective_fields));
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
    if let Some(lifecycle) = device.get("releaseLifecycle").and_then(J::as_object) {
        let level = bstr(lifecycle, "level");
        if matches!(level, "warn" | "danger") {
            out.push(insight(level, bstr(lifecycle, "text").to_owned()));
        }
    }
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
                    "{n} {kind} have no static reference but an iRule could build a matching name dynamically — can't be proven unused"
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

/// Singular label for an attachable object-type key.
fn attach_singular(ty: &str) -> &'static str {
    match ty {
        "pools" => "pool",
        "nodes" => "node",
        "snatpools" => "snatpool",
        _ => "object",
    }
}

fn json_ref(ty: &str, full_path: &str) -> J {
    // Stable object id prefix matching the topology graph / navigation index.
    let prefix = match ty {
        "pool" => "pool",
        "node" => "node",
        "snatpool" => "snat",
        "data-group" => "dg",
        "irule" => "rule",
        other => other,
    };
    let mut o = Map::new();
    o.insert("type".into(), J::String(ty.into()));
    o.insert("name".into(), J::String(leaf(full_path).into()));
    o.insert("fullPath".into(), J::String(full_path.into()));
    o.insert("oid".into(), J::String(format!("{prefix}:{full_path}")));
    J::Object(o)
}

/// Full paths of built-in / system objects: the default profiles, monitors and
/// `_sys_*` objects that ship with TMOS, declared in `profile_base.conf` /
/// `low_profile_base.conf` (often as bare names the engine prefixes with
/// `/Common/`). Recognised from the `# config/<member>` section headers the UCS
/// extractor writes into the SCF.
fn default_object_paths(config_text: &str) -> std::collections::HashSet<String> {
    const DEFAULT_MEMBERS: &[&str] = &["config/profile_base.conf", "config/low_profile_base.conf"];
    let mut out = std::collections::HashSet::new();
    let mut in_default = false;
    for line in config_text.lines() {
        if let Some(rest) = line.strip_prefix("# ") {
            let name = rest.trim();
            if name.starts_with("config/") {
                in_default = DEFAULT_MEMBERS.contains(&name);
            }
            continue;
        }
        if !in_default {
            continue;
        }
        // A top-level declaration starts at column 0 with a lowercase letter and
        // opens a brace; the object name is the last token before the first `{`
        // (handles both `x /Common/y {` and one-liner `x y { }`).
        if line.starts_with(|c: char| c.is_ascii_lowercase())
            && let Some(brace) = line.find('{')
            && let Some(name) = line[..brace].split_whitespace().last()
        {
            out.insert(if name.starts_with('/') {
                name.to_string()
            } else {
                format!("/Common/{name}")
            });
        }
    }
    out
}

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
fn link_proc_calls(device: &mut Map<String, J>) {
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
type RuleCtx = HashMap<String, BTreeSet<String>>;
/// Per-rule attaching virtuals, as `(virtual full path, virtual partition)`.
type RuleVirtuals = HashMap<String, Vec<(String, String)>>;

/// Map each iRule to the partitions it executes in (partitions of its attaching
/// virtuals) and the attaching virtuals themselves. Virtuals may reference a
/// rule by leaf name or full path.
fn build_rule_contexts(device: &Map<String, J>) -> (RuleCtx, RuleVirtuals) {
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
fn annotate_rule_reachability(device: &mut Map<String, J>, rule_vs: &RuleVirtuals) {
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

/// Order every virtual server's attached-profile list into BIG-IP protocol
/// stack order (transport → TLS → application → …), resolved from the profile
/// registry's `layer` metadata. A config lists profiles in an arbitrary order
/// (often alphabetical, so `/Common/http` precedes `/Common/tcp`), but the
/// device processes them by layer — the report should show that order, so a
/// listener reads TCP → … → HTTP, not HTTP → TCP.
fn order_virtual_profiles(device: &mut Map<String, J>) {
    // Full path → projected profile type (e.g. "/Common/http" → "HTTP").
    let mut type_of: HashMap<String, String> = HashMap::new();
    if let Some(J::Array(profiles)) = device.get("profiles") {
        for p in profiles {
            if let Some(pm) = p.as_object() {
                let fp = bstr(pm, "fullPath").to_string();
                if !fp.is_empty() {
                    type_of.insert(fp, bstr(pm, "type").to_string());
                }
            }
        }
    }
    if let Some(J::Array(virtuals)) = device.get_mut("virtuals") {
        for v in virtuals.iter_mut() {
            let Some(vm) = v.as_object_mut() else {
                continue;
            };
            let Some(J::Array(profs)) = vm.get_mut("profiles") else {
                continue;
            };
            let names: Vec<String> = profs
                .iter()
                .filter_map(|p| p.as_str().map(str::to_owned))
                .collect();
            if names.len() != profs.len() {
                continue; // non-string entries: leave the list untouched
            }
            // Delegate to the shared f5-query traffic-order core. The config's
            // typed profile inventory is authoritative (matched by full path or
            // by leaf, so partition-relative refs like `my_http` still resolve),
            // and the core falls back to well-known default-profile names (e.g.
            // `/Common/tcp`) that a config never re-declares.
            let ordered =
                tcl_bigip_query::builtins::f5profile::order_profiles_with_types(&names, &type_of);
            *profs = ordered.into_iter().map(J::String).collect();
        }
    }
}

fn collect_device(
    uri: &str,
    source: &str,
    all_sources: &[Source],
    cert_pems: &HashMap<String, String>,
    files: &[J],
    analysis_time: i64,
) -> J {
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
    let bigip_version = BigipVersion::parse(&tmsh);

    let mut device = Map::new();
    device.insert("uri".into(), J::String(uri.into()));
    device.insert("name".into(), J::String(device_name(uri, source)));
    device.insert("tmshVersion".into(), J::String(tmsh));
    device.insert(
        "releaseLifecycle".into(),
        crate::bigip_release_lifecycle(
            device
                .get("tmshVersion")
                .and_then(J::as_str)
                .unwrap_or_default(),
        ),
    );
    add_config_diagnostics(&mut device, uri, source, all_sources);

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
            "profiles" => rows
                .iter()
                .map(|f| shape_profile(f, used, bigip_version))
                .collect(),
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

    // GTM object inventory (the DNS tier). Shaped separately from the LTM
    // containers — these feed the GTM section and the cross-device architecture
    // linker (a GTM server's virtual-server destinations point at LTM virtuals).
    let gtm_used: HashMap<String, Vec<J>> = HashMap::new();
    for (key, container) in GTM_CONTAINERS {
        let rows = fields_of(query(&format!("{container}[]"), &sources).unwrap_or_default());
        let shaped: Vec<J> = match *key {
            "gtmWideips" => rows
                .iter()
                .map(|f| shape_gtm_wideip(f, &gtm_used))
                .collect(),
            "gtmPools" => rows.iter().map(|f| shape_gtm_pool(f, &gtm_used)).collect(),
            "gtmServers" => rows
                .iter()
                .map(|f| shape_gtm_server(f, &gtm_used))
                .collect(),
            "gtmDatacenters" => rows.iter().map(shape_gtm_datacenter).collect(),
            "gtmListeners" => rows.iter().map(shape_gtm_listener).collect(),
            _ => Vec::new(),
        };
        device.insert((*key).into(), J::Array(shaped));
    }

    // AFM firewall + NAT inventory (the security posture view).
    for (key, container) in SECURITY_CONTAINERS {
        let rows = fields_of(query(&format!("{container}[]"), &sources).unwrap_or_default());
        let shaped: Vec<J> = match *key {
            "firewallPolicies" => rows.iter().map(shape_fw_policy).collect(),
            "firewallRuleLists" => rows.iter().map(shape_fw_rule_list).collect(),
            "firewallAddressLists" => rows.iter().map(shape_fw_address_list).collect(),
            "firewallPortLists" => rows.iter().map(shape_fw_port_list).collect(),
            "natPolicies" => rows.iter().map(shape_nat_policy).collect(),
            "natSourceTranslations" | "natDestinationTranslations" => {
                rows.iter().map(shape_nat_translation).collect()
            }
            _ => Vec::new(),
        };
        device.insert((*key).into(), J::Array(shaped));
    }

    // Order each virtual's attached-profile list into protocol-stack order
    // now that both the virtuals and the typed profile inventory exist.
    order_virtual_profiles(&mut device);

    // Tag built-in / system objects (the default profiles, monitors and
    // `_sys_*` iRules that ship with TMOS — from `profile_base.conf` /
    // `low_profile_base.conf`) so they can be hidden by default and kept out of
    // orphan analysis, counts and diagrams.
    let defaults = default_object_paths(source);
    for key in DISPLAY_KEYS {
        if let Some(J::Array(objs)) = device.get_mut(*key) {
            for o in objs.iter_mut() {
                if let Some(om) = o.as_object_mut() {
                    let fp = bstr(om, "fullPath");
                    let is_def = defaults.contains(fp) || leaf(fp).starts_with("_sys_");
                    om.insert("isDefault".into(), J::Bool(is_def));
                }
            }
        }
    }

    // Link cross-iRule `call <rule>::<proc>` references before orphan analysis
    // so a proc-library iRule is counted as used by its callers.
    link_proc_calls(&mut device);

    // Orphans: a referable leaf object is *confirmed* orphaned only when it has
    // an empty referrer set AND no iRule could dynamically attach an object of
    // its *name*. Rather than demote a whole type the moment any rule attaches
    // it dynamically, each attach expression is reconstructed into a
    // prefix/contained/suffix name pattern (`pool "web_[HTTP::host]"` → `web_*`);
    // an object is a *possible* orphan only when some rule's pattern could build
    // its name, and stays a *confirmed* orphan otherwise.
    let rules_arr = device
        .get("rules")
        .and_then(J::as_array)
        .cloned()
        .unwrap_or_default();
    // Rule execution contexts: the partitions of the virtuals each rule is
    // attached to. A `/Common` iRule attached to virtuals in several partitions
    // resolves its unqualified names *per partition*, so orphan reachability is
    // partition-aware.
    let (rule_ctx, rule_vs) = build_rule_contexts(&device);
    let attach_idx = crate::graph::attach_index(&rules_arr, &rule_ctx);
    // orphanRisk: the types some rule attaches dynamically at all (summary /
    // topology use); the per-object filtering below is what actually classifies.
    device.insert(
        "orphanRisk".into(),
        J::Array(attach_idx.keys().map(|t| J::String((*t).into())).collect()),
    );
    // Surface the reconstructed patterns themselves so the report can explain
    // *why* an object is only a possible orphan.
    let mut attach_patterns = Map::new();
    for (ty, pats) in &attach_idx {
        let arr: Vec<J> = pats
            .iter()
            .map(|a| {
                let mut o = a.pattern.to_json();
                if let Some(om) = o.as_object_mut() {
                    om.insert("rule".into(), J::String(a.rule.clone()));
                }
                o
            })
            .collect();
        attach_patterns.insert((*ty).into(), J::Array(arr));
    }
    device.insert("attachPatterns".into(), J::Object(attach_patterns));

    let no_patterns: Vec<crate::graph::Attach> = Vec::new();
    let mut orphans = Map::new();
    let mut possible = Map::new();
    for name in REFERABLE {
        let attaches = attach_idx
            .get(name)
            .map_or(no_patterns.as_slice(), Vec::as_slice);
        // Annotate each object with its orphan status, then collect the sets.
        if let Some(J::Array(objs)) = device.get_mut(*name) {
            for o in objs.iter_mut() {
                if let Some(om) = o.as_object_mut() {
                    let empty = om
                        .get("usedBy")
                        .and_then(J::as_array)
                        .is_none_or(Vec::is_empty);
                    let status = if om.get("isDefault").and_then(J::as_bool) == Some(true) {
                        // Built-in/system objects are never "orphans".
                        ""
                    } else if !empty {
                        ""
                    } else if attaches.is_empty() {
                        "orphan"
                    } else {
                        // Partition-aware match of the object against each rule's
                        // reconstructed pattern in its execution context.
                        let leaf_name = bstr(om, "name").to_string();
                        let fp = bstr(om, "fullPath").to_string();
                        let part = partition_of(&fp);
                        let addr = bstr(om, "address").to_string();
                        let matches =
                            crate::graph::attach_matches(attaches, &leaf_name, &fp, &part, &addr);
                        if matches.is_empty() {
                            "orphan"
                        } else {
                            om.insert("orphanMatches".into(), J::Array(matches));
                            "possible"
                        }
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

    // Per-iRule reachability tables: statically referenced objects plus the
    // objects each dynamic filter could select, resolved per attaching-virtual
    // partition (VS-specific).
    annotate_rule_reachability(&mut device, &rule_vs);

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
    let certs = crate::certs::collect_certs(&sources, &device, cert_pems, analysis_time);
    device.insert("certificates".into(), certs);

    // Full offline TLS assurance: effective profile inheritance/defaults,
    // multi-certificate SNI variants, verified chains, client trust and a
    // transparent SSL Labs-style estimate. Only compact results and source
    // provenance enter the HTML; the generated report remains self-contained.
    device.insert(
        "tls".into(),
        crate::tls::collect_tls(source, bigip_version, cert_pems, analysis_time),
    );

    // Secret inventory (Secrets tab). Values are clear text only when the
    // config was decrypted with the f5mku master key upstream.
    device.insert(
        "secrets".into(),
        J::Array(crate::secrets::collect_secrets(source)),
    );

    // APM access-profile walk (APM tab): follow every `apm profile access` out
    // to its policy, items, agents and resources. Read from the config text —
    // the query projection is LTM-only.
    device.insert(
        "apmProfiles".into(),
        crate::apm::collect_apm(source, &device),
    );

    // Forensic file inventory + ATT&CK-mapped checklist (Forensics tab), built
    // from the UCS members the entry point extracted (empty for a bare
    // bigip.conf) plus a web-shell scan of this device's iRules.
    let rule_slice = device
        .get("rules")
        .and_then(J::as_array)
        .cloned()
        .unwrap_or_default();
    device.insert(
        "forensics".into(),
        crate::forensics::collect_forensics(files, &rule_slice),
    );

    // Offline security-posture findings (Security tab): default credentials,
    // SNMP/password-policy weaknesses, plaintext secrets, exposed private
    // keys, and shell-access review. See `crate::security` for the rule
    // table, the crypt verification it relies on, and the documented limits.
    device.insert(
        "security".into(),
        crate::security::collect_security(source, files),
    );

    // Tag every displayed object with its partition (from the full path) and
    // collect the device's partition set, so the report can filter to a
    // partition while always keeping shared /Common objects visible.
    let mut partitions: BTreeSet<String> = BTreeSet::new();
    for key in DISPLAY_KEYS {
        if let Some(J::Array(objs)) = device.get_mut(*key) {
            for o in objs.iter_mut() {
                if let Some(om) = o.as_object_mut() {
                    let existing = bstr(om, "partition").to_string();
                    let part = if existing.is_empty() {
                        partition_of(bstr(om, "fullPath"))
                    } else {
                        existing
                    };
                    if !part.is_empty() {
                        partitions.insert(part.clone());
                    }
                    om.insert("partition".into(), J::String(part));
                    // Folder path (`/Common/App_X`, empty for partition-root),
                    // for display and the apps grouping.
                    om.insert("folder".into(), J::String(folder_of(bstr(om, "fullPath"))));
                }
            }
        }
    }
    device.insert(
        "partitions".into(),
        J::Array(partitions.into_iter().map(J::String).collect()),
    );

    // Auto-detected applications, grouped by folder (iApps included via their
    // `.app` folders). Objects in the partition root are left ungrouped.
    let apps = build_apps(&device);
    device.insert("apps".into(), apps);

    // Counts. Built-in / system (default) objects are excluded — the chips count
    // the estate's own objects, not the ~260 TMOS defaults.
    let mut counts = Map::new();
    for (key, _) in CONTAINERS {
        let n = device.get(*key).and_then(J::as_array).map_or(0, |objs| {
            objs.iter()
                .filter_map(J::as_object)
                .filter(|o| o.get("isDefault").and_then(J::as_bool) != Some(true))
                .count()
        });
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
    let diagnostic_total = device
        .get("configDiagnostics")
        .and_then(J::as_array)
        .map_or(0, Vec::len);
    counts.insert("configDiagnostics".into(), J::from(diagnostic_total));
    let apps_total = device.get("apps").and_then(J::as_array).map_or(0, Vec::len);
    counts.insert("apps".into(), J::from(apps_total));
    let cert_total = device
        .get("certificates")
        .and_then(J::as_array)
        .map_or(0, Vec::len);
    counts.insert("certificates".into(), J::from(cert_total));
    let tls_total = device
        .get("tls")
        .and_then(|tls| tls.get("endpoints"))
        .and_then(J::as_array)
        .map_or(0, Vec::len);
    counts.insert("tls".into(), J::from(tls_total));
    let secret_total = device
        .get("secrets")
        .and_then(J::as_array)
        .map_or(0, Vec::len);
    counts.insert("secrets".into(), J::from(secret_total));
    // The Security tab's badge is the count of *actionable* findings
    // (confirmed / could-not-inspect) — clear/not-applicable results still
    // populate the tab (positive assurance) but don't inflate the badge.
    let security_actionable = device
        .get("security")
        .and_then(|s| s.get("actionable"))
        .and_then(J::as_u64)
        .unwrap_or(0);
    let tls_actionable = device
        .get("tls")
        .and_then(|tls| tls.get("findings"))
        .and_then(J::as_array)
        .map_or(0, |findings| {
            findings
                .iter()
                .filter(|finding| {
                    matches!(
                        finding.get("severity").and_then(J::as_str),
                        Some("warning" | "error" | "critical")
                    )
                })
                .count() as u64
        });
    counts.insert(
        "security".into(),
        J::from(security_actionable + tls_actionable),
    );
    let apm_total = device
        .get("apmProfiles")
        .and_then(J::as_array)
        .map_or(0, Vec::len);
    counts.insert("apmProfiles".into(), J::from(apm_total));
    // Aggregate GTM + firewall/NAT object counts (for the tab badges).
    let len_of = |keys: &[&str]| -> usize {
        keys.iter()
            .map(|k| device.get(*k).and_then(J::as_array).map_or(0, Vec::len))
            .sum()
    };
    let gtm_total = len_of(&[
        "gtmWideips",
        "gtmPools",
        "gtmServers",
        "gtmDatacenters",
        "gtmListeners",
    ]);
    counts.insert("gtm".into(), J::from(gtm_total));
    let firewall_total = len_of(&[
        "firewallPolicies",
        "firewallRuleLists",
        "firewallAddressLists",
        "firewallPortLists",
        "natPolicies",
        "natSourceTranslations",
        "natDestinationTranslations",
    ]);
    counts.insert("firewall".into(), J::from(firewall_total));
    let file_total = device
        .get("forensics")
        .and_then(|f| f.get("files"))
        .and_then(J::as_array)
        .map_or(0, Vec::len);
    counts.insert("files".into(), J::from(file_total));
    device.insert("counts".into(), J::Object(counts));

    let ins = insights(&device);
    device.insert("insights".into(), J::Array(ins));
    J::Object(device)
}

/// Build the full report model from loaded `(uri, text)` sources.
#[must_use]
pub fn collect_model(sources: &[Source], title: &str) -> J {
    collect_model_full(sources, title, &HashMap::new(), &HashMap::new(), None)
}

/// [`collect_model`] with certificate PEMs recovered from the UCS filestore.
///
/// `cert_pems` is keyed **by source URI** and then by a `sys file ssl-cert` or
/// `sys file ssl-key` `cache-path` (as it appears in the stanza) → the PEM text
/// of that member, read out of the archive by the caller (the CLI / wasm entry
/// points, which have the raw UCS bytes). Private-key material is reduced to
/// SPKI match evidence and is never copied into the returned model. Scoping by
/// source URI keeps a filestore
/// `cache-path` shared across two UCS files in one report from resolving to the
/// wrong device's certificate. The certs tab parses these to fill
/// metadata-free stanzas and reconstruct the trust chain; an empty map falls
/// back to config metadata only.
#[must_use]
#[allow(clippy::implicit_hasher)] // the caller (wasm/CLI) always uses std HashMap
pub fn collect_model_with_certs(
    sources: &[Source],
    title: &str,
    cert_pems: &HashMap<String, HashMap<String, String>>,
) -> J {
    collect_model_full(sources, title, cert_pems, &HashMap::new(), None)
}

/// [`collect_model_with_certs`] plus an optional *architecture manifest* (a Tcl
/// script; see [`tcl_bigip_query::architecture`]).
///
/// The report always auto-detects how the loaded devices relate as tiers (an
/// upstream device whose pool member / GTM server address is served by another
/// device's virtual is one tier up). `manifest` overrides each device's
/// role/tier and can declare links explicitly; `None` (or empty) uses pure
/// auto-detection. A malformed manifest is reported inside the model rather than
/// failing the build.
#[must_use]
#[allow(clippy::implicit_hasher)] // the caller (wasm/CLI) always uses std HashMap
pub fn collect_model_with_architecture(
    sources: &[Source],
    title: &str,
    cert_pems: &HashMap<String, HashMap<String, String>>,
    manifest: Option<&str>,
) -> J {
    collect_model_full(sources, title, cert_pems, &HashMap::new(), manifest)
}

/// [`collect_model_with_certs`] plus the per-device UCS **file inventory** that
/// powers the Forensics tab, and an optional architecture manifest.
///
/// `files` is keyed by source URI → the list of that device's extracted UCS
/// members (each a JSON object `{path, size, sha256, isText, content?}`, from
/// [`tcl_bigip_io::list_ucs_members`] / `read_ucs_member` at the wasm/CLI
/// entry). Both `cert_pems` and `files` are source-scoped so nothing bleeds
/// between devices in a multi-UCS report. `manifest` is the optional Tcl
/// architecture manifest. Empty maps / `None` reproduce [`collect_model`] exactly.
#[must_use]
#[allow(clippy::implicit_hasher)] // the caller (wasm/CLI) always uses std HashMap
pub fn collect_model_full(
    sources: &[Source],
    title: &str,
    cert_pems: &HashMap<String, HashMap<String, String>>,
    files: &HashMap<String, Vec<J>>,
    manifest: Option<&str>,
) -> J {
    let analysis_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_secs()).ok())
        .unwrap_or(0);
    let empty_pems = HashMap::new();
    let empty_files: Vec<J> = Vec::new();
    let devices: Vec<J> = sources
        .iter()
        .map(|(uri, src)| {
            collect_device(
                uri,
                src,
                sources,
                cert_pems.get(uri).unwrap_or(&empty_pems),
                files.get(uri).map_or(&empty_files, |v| v.as_slice()),
                analysis_time,
            )
        })
        .collect();

    let architecture = tcl_bigip_query::build_architecture(&devices, manifest);
    let enrichment = crate::enrich::build_enrichment(&devices, &architecture);

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
    model.insert("git_hash".into(), J::String(GIT_HASH.into()));
    model.insert("version".into(), J::String(GIT_DESCRIBE.into()));
    // Backend badge shown in the footer ("rust" here, "py" from the Python
    // generator's collect_model). Drives the `{{ backend }}` template variable.
    model.insert("backend".into(), J::String("rust".into()));
    model.insert("devices".into(), J::Array(devices));
    model.insert("totals".into(), J::Object(totals));
    model.insert("container_order".into(), J::Array(container_order));
    model.insert("architecture".into(), architecture);
    model.insert("enrichment".into(), enrichment);
    J::Object(model)
}

#[cfg(test)]
mod app_tests {
    use super::*;

    #[test]
    fn folder_helpers_split_paths() {
        assert_eq!(folder_of("/Common/appA/vs_a"), "/Common/appA");
        assert_eq!(folder_of("/Common/webstore.app/vs"), "/Common/webstore.app");
        // partition-root object has no folder
        assert_eq!(folder_of("/Common/root_vs"), "");
        assert_eq!(folder_of("not-a-path"), "");
        assert_eq!(
            app_folder_of("/Common/appA/pool_a"),
            Some(("Common".to_owned(), "appA".to_owned()))
        );
        // deeper nesting still groups under the first sub-folder
        assert_eq!(
            app_folder_of("/Common/appA/sub/x"),
            Some(("Common".to_owned(), "appA".to_owned()))
        );
        // root object is not grouped
        assert_eq!(app_folder_of("/Common/root_vs"), None);
    }

    #[test]
    fn build_apps_groups_by_folder_and_skips_root() {
        let mut device = Map::new();
        let mk = |name: &str, fp: &str| {
            let mut o = Map::new();
            o.insert("name".into(), J::String(name.into()));
            o.insert("fullPath".into(), J::String(fp.into()));
            J::Object(o)
        };
        device.insert(
            "virtuals".into(),
            J::Array(vec![
                mk("root_vs", "/Common/root_vs"),
                mk("vs_a", "/Common/appA/vs_a"),
                mk("web_vs", "/Common/store.app/web_vs"),
            ]),
        );
        device.insert(
            "pools".into(),
            J::Array(vec![mk("pool_a", "/Common/appA/pool_a")]),
        );

        let J::Array(apps) = build_apps(&device) else {
            panic!("expected array");
        };
        // appA (folder) + store (iapp) — the root vs is NOT an app.
        assert_eq!(apps.len(), 2);
        let a = apps[0].as_object().unwrap();
        assert_eq!(bstr(a, "name"), "appA");
        assert_eq!(bstr(a, "source"), "folder");
        assert_eq!(a.get("memberCount").and_then(J::as_u64), Some(2));
        let b = apps[1].as_object().unwrap();
        assert_eq!(bstr(b, "name"), "store"); // ".app" stripped
        assert_eq!(bstr(b, "source"), "iapp");
    }

    #[test]
    fn rule_events_use_top_level_rooted_normalised_owner() {
        let mut fields = Map::new();
        fields.insert("name".into(), J::String("r".into()));
        fields.insert("full-path".into(), J::String("/Common/r".into()));
        fields.insert(
            "body".into(),
            J::String("::when http_request { if {1} { :::when client_data {} } }".into()),
        );
        let shaped = shape_rule(&fields, &HashMap::new());
        let mut events: Vec<&str> = shaped["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(J::as_str)
            .collect();
        events.sort_unstable();
        assert_eq!(events, ["HTTP_REQUEST"]);
    }

    #[test]
    fn rule_events_ignore_when_text_stored_as_data() {
        let mut fields = Map::new();
        fields.insert("name".into(), J::String("r".into()));
        fields.insert("full-path".into(), J::String("/Common/r".into()));
        fields.insert(
            "body".into(),
            J::String("set payload {when CLIENT_DATA {}}\nset q \"when SERVER_DATA {}\"\nwhen HTTP_REQUEST {}".into()),
        );
        let shaped = shape_rule(&fields, &HashMap::new());
        assert_eq!(
            shaped["events"],
            J::Array(vec![J::String("HTTP_REQUEST".into())])
        );
    }
}

#[cfg(test)]
mod config_diagnostic_tests {
    use super::*;

    #[test]
    fn report_model_preserves_and_routes_validator_output() {
        let source = "ltm pool /Common/empty {\n}\n\
                      ltm data-group internal /Common/bad {\n  type ip\n  records {\n    nope { }\n  }\n}\n\
                      ltm rule /Common/a {\n  when HTTP_REQUEST { one }\n}\n\
                      ltm rule /Common/b {\n  when HTTP_REQUEST { two }\n}\n\
                      ltm virtual /Common/vs {\n  rules { /Common/a /Common/b }\n}\n";
        let mut device = Map::new();
        let sources = vec![("memory://config.bigip.conf".to_owned(), source.to_owned())];
        add_config_diagnostics(&mut device, "memory://config.bigip.conf", source, &sources);

        let codes = |key: &str| {
            device[key]
                .as_array()
                .expect("diagnostic array")
                .iter()
                .filter_map(|item| item["code"].as_str())
                .collect::<Vec<_>>()
        };
        assert!(codes("poolDiagnostics").contains(&"BIGIP6008"));
        assert!(codes("dataGroupDiagnostics").contains(&"BIGIP6011"));
        assert!(codes("virtualDiagnostics").contains(&"BIGIP6012"));
        assert_eq!(
            device["configDiagnostics"].as_array().map(Vec::len),
            Some(
                device["virtualDiagnostics"].as_array().unwrap().len()
                    + device["ruleDiagnostics"].as_array().unwrap().len()
                    + device["poolDiagnostics"].as_array().unwrap().len()
                    + device["dataGroupDiagnostics"].as_array().unwrap().len()
                    + device["appDiagnostics"].as_array().unwrap().len()
                    + device["objectDiagnostics"].as_array().unwrap().len()
            )
        );
    }

    #[test]
    fn registry_diagnostics_route_to_the_object_index() {
        let source = "ltm virtual /Common/vs { fallback-persistence /Common/missing }\n";
        let mut device = Map::new();
        let sources = vec![("memory://config.bigip.conf".to_owned(), source.to_owned())];
        add_config_diagnostics(&mut device, "memory://config.bigip.conf", source, &sources);
        assert!(
            device["objectDiagnostics"]
                .as_array()
                .expect("object diagnostics")
                .iter()
                .any(|diagnostic| diagnostic["code"] == "BIGIP6013")
        );
    }

    #[test]
    fn iapp_diagnostics_are_global_routed_to_apps_and_mark_partial_evidence() {
        let presentation =
            "#include \"missing.inc\"\nsection basic {\n  string addr\n  string port\n}\n";
        let implementation = "set ok $::basic__addr\nset missing $::basic__missing\n";
        let sources = vec![
            (
                "memory://iapp/presentation.apl".to_owned(),
                presentation.to_owned(),
            ),
            (
                "memory://iapp/implementation".to_owned(),
                implementation.to_owned(),
            ),
        ];

        let model = collect_model(&sources, "iApp diagnostics");
        let devices = model["devices"].as_array().expect("devices");
        let presentation_device = devices
            .iter()
            .find(|device| device["uri"] == "memory://iapp/presentation.apl")
            .expect("presentation device");
        let implementation_device = devices
            .iter()
            .find(|device| device["uri"] == "memory://iapp/implementation")
            .expect("implementation device");
        let codes = |device: &J| {
            device["configDiagnostics"]
                .as_array()
                .expect("global diagnostics")
                .iter()
                .filter_map(|item| item["code"].as_str().map(str::to_owned))
                .collect::<Vec<_>>()
        };
        assert!(codes(presentation_device).contains(&"IAPP7002".to_owned()));
        assert!(codes(presentation_device).contains(&"IAPP7003".to_owned()));
        assert!(codes(implementation_device).contains(&"IAPP7001".to_owned()));
        assert!(
            presentation_device["appDiagnostics"]
                .as_array()
                .expect("apps diagnostics")
                .iter()
                .all(|item| item["tab"] == "apps")
        );
        assert_eq!(
            presentation_device["iappDiagnosticEvidence"]["state"],
            "complete"
        );

        let partial_sources = vec![(
            "memory://iapp/presentation.apl".to_owned(),
            presentation.to_owned(),
        )];
        let partial = collect_model(&partial_sources, "partial iApp diagnostics");
        assert_eq!(
            partial["devices"][0]["iappDiagnosticEvidence"]["state"],
            "presentation_only"
        );
    }
}

#[cfg(test)]
mod engine_version_tests {
    /// The report header renders `query engine v{{ engine_version }}`, and read
    /// `0.1.0` — the never-bumped workspace manifest version — for every release.
    ///
    /// The guard is "comes from `tcl-version`", not "looks like a tag": CI checks
    /// out with `fetch-depth: 1` and no tags, so no tag is reachable there and the
    /// resolved version legitimately falls back to the manifest base (`0.1.0`,
    /// plus the commit). Asserting a tag-shaped string would fail on every PR.
    #[test]
    fn engine_version_comes_from_tcl_version_not_cargo_pkg_version() {
        assert_eq!(
            super::ENGINE_VERSION,
            tcl_version::VERSION,
            "ENGINE_VERSION must be sourced from tcl-version (which resolves the \
             release tag), not from CARGO_PKG_VERSION (the workspace manifest's \
             permanent 0.1.0 placeholder)"
        );
    }

    /// Whatever the base resolves to, the commit must always be present — that is
    /// the whole point of stamping it separately from `git describe`.
    #[test]
    fn engine_version_carries_the_commit() {
        let v = super::ENGINE_VERSION;
        assert!(
            v.contains("+g") || tcl_version::COMMIT.is_empty(),
            "expected the commit as build metadata, got {v:?}"
        );
    }
}
