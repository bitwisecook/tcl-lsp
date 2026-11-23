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

//! Per-type shaping of engine objects into report rows: LTM, GTM, firewall, NAT, profiles and policies.

use super::{clean_arr, clean_field, clean_path, event_registry, partition_of, py_str, used_by};
use crate::jutil::{barr, bbool, bstr, sarr, truthy};
use serde_json::{Map, Value as J};
use std::collections::{BTreeSet, HashMap};
use tcl_bigip::value::Destination;
use tcl_registry::profile_defaults::{BigipVersion, profile_field_defaults};

// --- per-type shaping --------------------------------------------------------

pub(super) fn shape_virtual(f: &Map<String, J>) -> J {
    let (addr, port) =
        Destination::try_parse(bstr(f, "destination")).map_or_default(|destination| {
            let address = match destination.address {
                tcl_bigip::value::Address::Ip(_) => destination
                    .ip_address()
                    .map_or_else(String::new, |address| address.to_string()),
                _ => destination.address.to_string(),
            };
            (address, destination.port.spelling)
        });
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

pub(super) fn shape_pool(f: &Map<String, J>, used: &HashMap<String, Vec<J>>) -> J {
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

pub(super) fn shape_node(f: &Map<String, J>, used: &HashMap<String, Vec<J>>) -> J {
    let fp = bstr(f, "full-path");
    let mut o = Map::new();
    o.insert("name".into(), J::String(bstr(f, "name").into()));
    o.insert("fullPath".into(), J::String(fp.into()));
    o.insert("address".into(), J::String(bstr(f, "address").into()));
    o.insert("monitor".into(), J::String(clean_field(f, "monitor")));
    o.insert("usedBy".into(), J::Array(used_by(used, fp)));
    J::Object(o)
}

pub(super) fn shape_monitor(f: &Map<String, J>, used: &HashMap<String, Vec<J>>) -> J {
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

pub(super) fn shape_rule(f: &Map<String, J>, used: &HashMap<String, Vec<J>>) -> J {
    let body = bstr(f, "body").to_string();
    // Parse through the shared event-handler owner, then order into canonical
    // firing order rather than alphabetical order (which scrambles the
    // lifecycle, e.g. CLIENTSSL_HANDSHAKE ahead of CLIENT_ACCEPTED).
    let command_registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let identities = tcl_compiler::realm::document_realm_bindings(
        &body,
        tcl_dialect::DialectProfile::irules(),
        command_registry,
    );
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
            tcl_lexer::LexerConfig::from_grammar(
                tcl_registry::model::resolve_environment("f5-irules").grammar(),
            ),
        )),
    );
    // IR-based control-flow graph ({nodes, edges} JSON for elkjs); empty when
    // there's nothing to draw. The report renders it lazily when the iRule row
    // is expanded.
    o.insert(
        "flowchart".into(),
        J::String(tcl_diagram::irule_flowchart_graph(
            &body,
            tcl_registry::model::ingress::static_context_for("f5-irules").commands(),
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

pub(super) fn shape_data_group(f: &Map<String, J>, used: &HashMap<String, Vec<J>>) -> J {
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

pub(super) fn shape_gtm_wideip(f: &Map<String, J>, used: &HashMap<String, Vec<J>>) -> J {
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

pub(super) fn shape_gtm_pool(f: &Map<String, J>, used: &HashMap<String, Vec<J>>) -> J {
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

pub(super) fn shape_gtm_server(f: &Map<String, J>, used: &HashMap<String, Vec<J>>) -> J {
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

pub(super) fn shape_gtm_datacenter(f: &Map<String, J>) -> J {
    let fp = bstr(f, "full-path");
    let mut o = Map::new();
    o.insert("name".into(), J::String(bstr(f, "name").into()));
    o.insert("fullPath".into(), J::String(fp.into()));
    o.insert("location".into(), J::String(bstr(f, "location").into()));
    o.insert("contact".into(), J::String(bstr(f, "contact").into()));
    o.insert("state".into(), J::String(bstr(f, "state").into()));
    J::Object(o)
}

pub(super) fn shape_gtm_listener(f: &Map<String, J>) -> J {
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

pub(super) fn shape_fw_policy(f: &Map<String, J>) -> J {
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

pub(super) fn shape_fw_rule_list(f: &Map<String, J>) -> J {
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

pub(super) fn shape_fw_address_list(f: &Map<String, J>) -> J {
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

pub(super) fn shape_fw_port_list(f: &Map<String, J>) -> J {
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

pub(super) fn shape_nat_policy(f: &Map<String, J>) -> J {
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
pub(super) fn shape_nat_translation(f: &Map<String, J>) -> J {
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

pub(super) fn shape_profile(
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

pub(super) fn shape_policy(f: &Map<String, J>) -> J {
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
