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

//! The `explain-flow` verb — trace each flow in a PCAP through the BIG-IP
//! config.
//!
//! Covers the always-available built-in walker (static) path end to end: flow
//! extraction (`tcl_bigip::flow`), session pairing, virtual-server matching,
//! the matched-session detail (profile chain, iRule event chain, LTM policy
//! trace via `tcl_bigip::policy_eval`, resolved plan), the reset-cause
//! narrative, and both the text and `--json` report renderers. The `--tshark`
//! / `--keylog` / `--tshark-filter` L7-enrichment paths are wired through
//! `tcl_bigip::flow::tshark`. The `--simulate` (iRule VM) path is not yet
//! implemented.

use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;

use indexmap::IndexMap;
use regex::Regex;
use serde_json::{Map, Value};
use tcl_bigip::flow::{
    Connection, Flow, Session, enrich_with_tshark, extract_flows, extract_flows_via_tshark,
    pair_sessions, tshark_available,
};
use tcl_bigip::model::{
    BigipPolicy, BigipProfile, BigipRule, BigipVirtualServer, ModelObject, ProfileType,
};
use tcl_bigip::parser::BigipConfig;
use tcl_bigip::policy_eval::{PolicyDecision, evaluate_policy, request_state_from_session};
use tcl_cli_support::{OutputTarget, write_text_output};

use crate::commands::explain;

/// Canonical lifecycle order for L4/SSL/HTTP events on one request/response
/// cycle.
const EVENT_ORDER: &[&str] = &[
    "RULE_INIT",
    "CLIENT_ACCEPTED",
    "CLIENTSSL_HANDSHAKE",
    "CLIENTSSL_CLIENTHELLO",
    "CLIENTSSL_SERVERHELLO_SEND",
    "CLIENTSSL_DATA",
    "HTTP_REQUEST",
    "HTTP_REQUEST_DATA",
    "HTTP_REQUEST_SEND",
    "LB_SELECTED",
    "LB_FAILED",
    "SERVER_CONNECTED",
    "SERVERSSL_HANDSHAKE",
    "SERVERSSL_DATA",
    "HTTP_RESPONSE",
    "HTTP_RESPONSE_CONTINUE",
    "HTTP_RESPONSE_DATA",
    "CLIENT_DATA",
    "SERVER_DATA",
    "HTTP_DISCONNECT",
    "CLIENT_CLOSED",
    "SERVER_CLOSED",
];

/// iRule command invocation inside `[ ... ]`.
static HUD_TOKEN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[\s*(HTTP::|SSL::|IP::|TCP::|LB::|SNI\b)([^\]\n]*?)\s*\]").expect("hud regex")
});

/// One HUD annotation: `(source-line excerpt, command, captured value)`.
type Annotation = (String, String, String);
/// One iRule event body: `(rule path, event name, body text)`.
type EventBlock = (String, String, String);
/// Per-event annotation group: `(rule path, event name, annotations)`.
type EventAnnotation = (String, String, Vec<Annotation>);

/// Per-session explanation: which VS matched, the event chain, RST analysis.
/// Unpopulated fields default to empty.
#[derive(Default)]
struct SessionExplain {
    session: Option<SessionHolder>,
    matched_vs: String,
    /// Partition short-name (rendered in the `--json` output).
    matched_partition: String,
    profile_chain: Vec<String>,
    pool_selected: String,
    snat_observed: String,
    event_sequence: Vec<String>,
    event_blocks: Vec<EventBlock>,
    event_annotations: Vec<EventAnnotation>,
    ltm_policies: Vec<String>,
    policy_decisions: Vec<PolicyDecision>,
    apm_profile: String,
    gtm_wide_ips: Vec<String>,
    explain_text: String,
    reset_analysis: String,
    /// The dynamic iRule-simulation outcome (`--simulate`), if requested.
    simulated: Option<SimOutcome>,
}

/// The dynamic outcome of running a matched session's iRule(s) under the
/// embedded TMM-sim orchestrator on `tcl-vm`. The simulation itself lives in
/// the shared, config-agnostic [`tcl_irule_test::simulate_irule`].
use tcl_irule_test::SimOutcome;

/// Owns the [`Session`] referenced by a [`SessionExplain`].
struct SessionHolder(Session);

impl SessionExplain {
    fn session(&self) -> &Session {
        &self.session.as_ref().expect("session present").0
    }
}

/// The whole-report result of [`compute_explain_flow`]. The `used_tshark` /
/// `keylog_path` / `tshark_filter` fields record the L7-enrichment path taken:
/// the built-in walker alone (`used_tshark = false`), the `--tshark` /
/// `--keylog` overlay, or a `--tshark-filter` extraction.
struct ExplainFlowReport {
    pcap_path: String,
    flow_count: usize,
    session_count: usize,
    matched_count: usize,
    used_tshark: bool,
    keylog_path: String,
    tshark_filter: String,
    sessions: Vec<SessionExplain>,
}

/// Parse a VS destination string like `/Common/10.0.0.1:443` or `/p/[::1]:80`,
/// returning the canonical address and port.
fn parse_destination(dest: &str, re: &Regex) -> Option<(String, u32)> {
    let caps = re.captures(dest.trim())?;
    let addr = caps
        .name("addr")?
        .as_str()
        .trim_matches(|c| c == '[' || c == ']');
    let port_raw = caps.name("port")?.as_str();
    let canonical = canonicalise_ip(addr)?;
    let port = if port_raw == "any" {
        0
    } else {
        port_raw.parse::<u32>().ok()?
    };
    Some((canonical, port))
}

/// `str(ipaddress.ip_address(addr))` — canonical form, or `None` if invalid.
fn canonicalise_ip(addr: &str) -> Option<String> {
    addr.parse::<std::net::IpAddr>()
        .ok()
        .map(|ip| ip.to_string())
}

/// Find the virtual server whose destination matches `(dst_ip, dst_port)`,
/// iterating VSes in source order.
fn match_virtual(cfg: &BigipConfig, dst_ip: &str, dst_port: u16, re: &Regex) -> Option<String> {
    let flow_ip = canonicalise_ip(dst_ip).unwrap_or_else(|| dst_ip.to_owned());
    for placed in &cfg.objects {
        let ModelObject::VirtualServer(vs) = &placed.object else {
            continue;
        };
        let dest_text = vs
            .destination
            .as_ref()
            .map_or_else(String::new, ToString::to_string);
        let Some((vs_addr, vs_port)) = parse_destination(&dest_text, re) else {
            continue;
        };
        if vs_addr != flow_ip {
            continue;
        }
        if vs_port == 0 || vs_port == u32::from(dst_port) {
            return Some(vs.full_path.clone());
        }
    }
    None
}

/// Fetch the parsed virtual server with the given full path.
fn find_virtual<'a>(cfg: &'a BigipConfig, full_path: &str) -> Option<&'a BigipVirtualServer> {
    cfg.objects.iter().find_map(|p| match &p.object {
        ModelObject::VirtualServer(vs) if vs.full_path == full_path => Some(vs),
        _ => None,
    })
}

/// All object full-paths in a given `Placed` table, in source order.
fn table_keys<'a>(cfg: &'a BigipConfig, table: &str) -> impl Iterator<Item = &'a str> {
    cfg.objects.iter().filter_map(move |p| {
        if p.table_name == table {
            Some(p.full_path.as_str())
        } else {
            None
        }
    })
}

/// Resolve a possibly-short name to a full path in `table`, trying exact,
/// partition-qualified, `/Common/`, and suffix matches in turn.
fn resolve_name(cfg: &BigipConfig, name: &str, table: &str) -> Option<String> {
    if table_keys(cfg, table).any(|k| k == name) {
        return Some(name.to_owned());
    }
    if !name.starts_with('/') {
        let partition = {
            let p = if cfg.default_partition.is_empty() {
                "Common"
            } else {
                cfg.default_partition.as_str()
            };
            p.trim_matches('/').to_owned()
        };
        if !partition.is_empty() {
            let candidate = format!("/{partition}/{name}");
            if table_keys(cfg, table).any(|k| k == candidate) {
                return Some(candidate);
            }
        }
        if partition != "Common" {
            let candidate = format!("/Common/{name}");
            if table_keys(cfg, table).any(|k| k == candidate) {
                return Some(candidate);
            }
        }
    }
    let suffix = format!("/{name}");
    table_keys(cfg, table)
        .find(|k| k.ends_with(&suffix))
        .map(ToOwned::to_owned)
}

fn resolve_profile(cfg: &BigipConfig, name: &str) -> Option<String> {
    resolve_name(cfg, name, "profiles")
}

fn find_profile<'a>(cfg: &'a BigipConfig, full_path: &str) -> Option<&'a BigipProfile> {
    cfg.objects.iter().find_map(|p| match &p.object {
        ModelObject::Profile(prof) if prof.full_path == full_path => Some(prof),
        _ => None,
    })
}

/// Resolve a generic BIG-IP object key by identifier/name. Returns the
/// `generic_objects` key.
fn resolve_generic_object(
    cfg: &BigipConfig,
    name: &str,
    module: Option<&str>,
    object_types: Option<&[&str]>,
) -> Option<String> {
    let clean = name.trim();
    if clean.is_empty() {
        return None;
    }
    for (key, obj) in &cfg.generic_objects {
        if module.is_some_and(|m| obj.module != m) {
            continue;
        }
        if object_types.is_some_and(|types| !types.contains(&obj.object_type.as_str())) {
            continue;
        }
        // The `ident == clean` exact-match is already covered by the leading
        // term, so it is not repeated inside the unqualified branch.
        let ident = obj.identifier.as_str();
        let matches = ident == clean
            || (clean.starts_with('/') && ident.ends_with(clean))
            || (!clean.starts_with('/') && ident.ends_with(&format!("/{clean}")));
        if matches {
            return Some(key.clone());
        }
    }
    None
}

/// Return the LTM policy paths attached to *vs*, in attach order.
fn ltm_policies_for(cfg: &BigipConfig, vs: &BigipVirtualServer) -> Vec<String> {
    let policies = vs.policies.paths();
    if policies.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<String> = Vec::new();
    for reference in policies {
        if let Some(resolved) =
            resolve_generic_object(cfg, &reference, Some("ltm"), Some(&["policy"]))
        {
            let obj = cfg
                .generic_objects
                .iter()
                .find(|(k, _)| *k == resolved)
                .map(|(_, o)| o);
            let label = obj
                .filter(|o| !o.identifier.is_empty())
                .map_or(resolved, |o| o.identifier.clone());
            out.push(label);
        } else {
            out.push(format!("{reference} (unresolved)"));
        }
    }
    out
}

fn find_policy<'a>(cfg: &'a BigipConfig, full_path: &str) -> Option<&'a BigipPolicy> {
    cfg.objects.iter().find_map(|p| match &p.object {
        ModelObject::Policy(policy) if policy.full_path == full_path => Some(policy),
        _ => None,
    })
}

/// Evaluate every parsed LTM policy attached to the matched VS against the
/// captured request state.
fn evaluate_attached_policies(
    cfg: &BigipConfig,
    attached_paths: &[String],
    session: &Session,
) -> Vec<PolicyDecision> {
    if attached_paths.is_empty() {
        return Vec::new();
    }
    let state = request_state_from_session(session);
    let mut out: Vec<PolicyDecision> = Vec::new();
    for path in attached_paths {
        if path.ends_with("(unresolved)") {
            continue;
        }
        if let Some(policy) = find_policy(cfg, path) {
            out.push(evaluate_policy(policy, &state));
        }
    }
    out
}

/// Locate the APM access profile attached to *vs*.
fn apm_profile_for(cfg: &BigipConfig, vs: &BigipVirtualServer) -> String {
    for pref in vs.profiles.paths() {
        let resolved = resolve_profile(cfg, &pref).unwrap_or_else(|| pref.clone());
        if resolved.contains("/access") || resolved.ends_with("access") {
            return resolved;
        }
        for (_key, obj) in &cfg.generic_objects {
            if obj.module == "apm"
                && (resolved == obj.identifier || resolved.ends_with(&obj.identifier))
            {
                return resolved;
            }
        }
    }
    String::new()
}

/// Return every GTM wide-IP identifier in *cfg* (a global inventory).
fn gtm_wide_ips_in_config(cfg: &BigipConfig) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (key, obj) in &cfg.generic_objects {
        if obj.module != "gtm" {
            continue;
        }
        if obj.object_type.starts_with("wideip")
            || obj.object_type == "wideip-a"
            || obj.object_type.contains("wideip")
        {
            out.push(if obj.identifier.is_empty() {
                key.clone()
            } else {
                obj.identifier.clone()
            });
        }
    }
    out
}

fn find_rule<'a>(cfg: &'a BigipConfig, full_path: &str) -> Option<&'a BigipRule> {
    cfg.objects.iter().find_map(|p| match &p.object {
        ModelObject::Rule(rule) if rule.full_path == full_path => Some(rule),
        _ => None,
    })
}

/// The deduplicated profile types attached to *vs* (only resolved, present
/// profiles count).
fn profile_types_for_virtual(cfg: &BigipConfig, vs: &BigipVirtualServer) -> Vec<ProfileType> {
    let mut out: Vec<ProfileType> = Vec::new();
    for pref in vs.profiles.paths() {
        if let Some(resolved) = resolve_profile(cfg, &pref)
            && let Some(prof) = find_profile(cfg, &resolved)
            && !out.contains(&prof.profile_type)
        {
            out.push(prof.profile_type);
        }
    }
    out
}

/// Return `{event_name: body}` for every `when EVENT { … }` block. Brace-aware
/// extractor; malformed (unbalanced) blocks are skipped. The last body wins on
/// a duplicate event, with the first-seen position preserved.
fn extract_event_blocks(rule_source: &str) -> IndexMap<String, String> {
    let mut blocks: IndexMap<String, String> = IndexMap::new();
    for block in tcl_irules::when_blocks(rule_source) {
        blocks.insert(
            block.event,
            rule_source[block.body_span.as_range()].to_owned(),
        );
    }
    blocks
}

/// Pick the events from the rule that would plausibly fire for the connection,
/// in lifecycle order.
fn expected_event_sequence(
    cfg: &BigipConfig,
    vs: &BigipVirtualServer,
    conn: &Connection,
    rule_events: &[&str],
) -> Vec<String> {
    let types = profile_types_for_virtual(cfg, vs);
    let has_http = types.contains(&ProfileType::Http);
    let has_client_ssl = types.contains(&ProfileType::ClientSsl);
    let has_server_ssl = types.contains(&ProfileType::ServerSsl);

    let client = &conn.client;
    let server = conn.server.as_ref();
    let saw_tls = client.tls_clienthello || has_client_ssl;
    let saw_http_req = client.http_request_seen || (has_http && client.proto == 6);
    let saw_http_resp = server
        .is_some_and(|s| s.http_response_seen || !s.http_response_code.is_empty())
        || client.http_response_seen;
    let saw_close =
        client.tcp_fin || client.tcp_rst || server.is_some_and(|s| s.tcp_fin || s.tcp_rst);

    let mut candidate: Vec<&str> = vec!["RULE_INIT", "CLIENT_ACCEPTED"];
    if saw_tls {
        candidate.extend(["CLIENTSSL_CLIENTHELLO", "CLIENTSSL_HANDSHAKE"]);
    }
    if saw_http_req {
        candidate.extend(["HTTP_REQUEST", "HTTP_REQUEST_DATA"]);
    }
    candidate.extend(["LB_SELECTED", "SERVER_CONNECTED"]);
    if has_server_ssl {
        candidate.push("SERVERSSL_HANDSHAKE");
    }
    if saw_http_req {
        candidate.push("HTTP_REQUEST_SEND");
    }
    if saw_http_resp {
        candidate.extend(["HTTP_RESPONSE", "HTTP_RESPONSE_DATA"]);
    }
    if saw_close {
        candidate.extend(["CLIENT_CLOSED", "SERVER_CLOSED"]);
    }

    let rule_set: HashSet<&str> = rule_events.iter().copied().collect();
    let selected: Vec<String> = candidate
        .iter()
        .filter(|ev| rule_set.contains(**ev))
        .map(|ev| (*ev).to_owned())
        .collect();
    let selected_set: HashSet<&str> = selected.iter().map(String::as_str).collect();
    let mut extra: Vec<String> = rule_events
        .iter()
        .filter(|ev| !selected_set.contains(**ev) && !EVENT_ORDER.contains(*ev))
        .map(|ev| (*ev).to_owned())
        .collect();
    extra.sort();
    let mut out = selected;
    out.extend(extra);
    out
}

/// Resolve an iRule command token like `HTTP::host` against *conn*.
// One flat arm per iRule command token; splitting the dispatch would obscure it.
#[allow(clippy::too_many_lines)]
fn hud_value_for(token: &str, conn: &Connection) -> Option<String> {
    let f = &conn.client;
    let s = conn.server.as_ref();
    let ne = |x: &str| -> Option<String> {
        if x.is_empty() {
            None
        } else {
            Some(x.to_owned())
        }
    };
    let t = token;
    if t == "HTTP::host" {
        return ne(&f.http_host);
    }
    if t == "HTTP::method" {
        return ne(&f.http_method);
    }
    if t == "HTTP::uri" {
        return ne(&f.http_uri);
    }
    if t == "HTTP::path" {
        return ne(&f.http_path);
    }
    if t == "HTTP::query" {
        return ne(&f.http_query);
    }
    if t == "HTTP::version" {
        return ne(&f.http_request_version);
    }
    if t == "HTTP::status" {
        return ne(s.map_or("", |x| x.http_response_code.as_str()));
    }
    if t.starts_with("HTTP::header ") {
        let name = t["HTTP::header".len()..]
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .to_lowercase();
        if let Some(v) = f.request_header(&name) {
            return Some(v.to_owned());
        }
        if let Some(server) = s
            && let Some(v) = server.response_header(&name)
        {
            return Some(v.to_owned());
        }
        return None;
    }
    if t == "HTTP::cookie" {
        return ne(&f.http_cookie);
    }
    if t == "HTTP::user_agent" {
        return ne(&f.http_user_agent);
    }
    if t == "SSL::extensions" {
        let mut bits: Vec<String> = Vec::new();
        if !f.tls_sni.is_empty() {
            bits.push(format!("sni={}", f.tls_sni));
        }
        if !f.tls_alpn.is_empty() {
            bits.push(format!("alpn={}", f.tls_alpn));
        }
        return if bits.is_empty() {
            None
        } else {
            Some(bits.join(", "))
        };
    }
    if t == "SSL::cipher" {
        return ne(&f.tls_chosen_cipher);
    }
    if t == "SSL::cipher version" {
        return ne(&f.tls_chosen_version);
    }
    if t == "SSL::cert subject" {
        return ne(&f.tls_cert_subject);
    }
    if t == "SSL::cert" {
        return ne(&f.tls_cert_subject);
    }
    if t.to_uppercase() == "SNI" || t == "SSL::extensions servername" {
        return ne(&f.tls_sni);
    }
    if t == "IP::client_addr" {
        return Some(f.src_ip.clone());
    }
    if t == "IP::local_addr" {
        return Some(f.dst_ip.clone());
    }
    if t == "IP::remote_addr" {
        return Some(f.dst_ip.clone());
    }
    if t == "TCP::client_port" {
        return (f.src_port != 0).then(|| f.src_port.to_string());
    }
    if t == "TCP::local_port" {
        return (f.dst_port != 0).then(|| f.dst_port.to_string());
    }
    if t == "TCP::remote_port" {
        return (f.dst_port != 0).then(|| f.dst_port.to_string());
    }
    // LB::server is filled by the caller from the back side.
    None
}

/// Return `[(line_excerpt, command, captured_value), …]` for *body*, one
/// annotation per line.
fn annotate_event_block(body: &str, conn: &Connection) -> Vec<Annotation> {
    let mut out: Vec<Annotation> = Vec::new();
    for line in py_splitlines(body) {
        let stripped = line.trim();
        if stripped.is_empty() || stripped.starts_with('#') {
            continue;
        }
        for caps in HUD_TOKEN_RE.captures_iter(line) {
            let ns = caps.get(1).expect("group 1").as_str();
            let rest = caps.get(2).expect("group 2").as_str().trim();
            let full = if ns == "SNI" {
                "SNI".to_owned()
            } else {
                format!("{ns}{rest}")
            };
            if let Some(value) = hud_value_for(&full, conn) {
                out.push((stripped.to_owned(), full, value));
                break;
            }
        }
    }
    out
}

/// Produce a human-readable narrative of why the session ended.
fn analyse_reset(session: &Session) -> String {
    let mut parts: Vec<String> = Vec::new();
    let front = &session.front;
    let back = &session.back;
    let causes = session.reset_causes();

    if front.client.tcp_rst {
        parts.push(format!(
            "client\u{2192}VIP RST after {} bytes ({}x)",
            front.client.tcp_rst_after_bytes, front.client.tcp_rst_count
        ));
    }
    if let Some(server) = &front.server
        && server.tcp_rst
    {
        parts.push(format!(
            "VIP\u{2192}client RST after {} bytes ({}x)",
            server.tcp_rst_after_bytes, server.tcp_rst_count
        ));
    }
    if let Some(back) = back {
        if back.client.tcp_rst {
            parts.push(format!(
                "TMM\u{2192}server RST after {} bytes ({}x)",
                back.client.tcp_rst_after_bytes, back.client.tcp_rst_count
            ));
        }
        if let Some(server) = &back.server
            && server.tcp_rst
        {
            parts.push(format!(
                "server\u{2192}TMM RST after {} bytes ({}x)",
                server.tcp_rst_after_bytes, server.tcp_rst_count
            ));
        }
    }

    if parts.is_empty() {
        let front_fin = front.client.tcp_fin || front.server.as_ref().is_some_and(|s| s.tcp_fin);
        if front_fin {
            return "graceful FIN teardown (no RST)".to_owned();
        }
        return "no termination observed in capture".to_owned();
    }

    if causes.is_empty() {
        parts.push("no F5 reset cause string in trailer (LOW/MED TLV absent or opaque)".to_owned());
    } else {
        parts.push(format!("F5 reset cause(s): {}", causes.join(" | ")));
    }

    if front.client.tls_alert_seen {
        parts.push(format!("client TLS alert: {}", front.client.tls_alert_desc));
    }
    if let Some(server) = &front.server
        && server.tls_alert_seen
    {
        parts.push(format!("server TLS alert: {}", server.tls_alert_desc));
    }

    parts.join(" ; ")
}

/// The orchestrator profile labels (TCP / CLIENTSSL / HTTP / …) for `vs`.
fn profiles_for_orchestrator(cfg: &BigipConfig, vs: &BigipVirtualServer) -> Vec<String> {
    let types = profile_types_for_virtual(cfg, vs);
    let mut out: Vec<String> = Vec::new();
    if types.contains(&ProfileType::Tcp) || !vs.profiles.paths().is_empty() {
        out.push("TCP".to_owned());
    }
    if types.contains(&ProfileType::Udp) {
        out.push("UDP".to_owned());
    }
    if types.contains(&ProfileType::ClientSsl) {
        out.push("CLIENTSSL".to_owned());
    }
    if types.contains(&ProfileType::ServerSsl) {
        out.push("SERVERSSL".to_owned());
    }
    if types.contains(&ProfileType::Http) {
        out.push("HTTP".to_owned());
    }
    if out.is_empty() {
        out.push("TCP".to_owned());
    }
    out
}

/// The last `/`-separated segment of a full path (`/Common/web` → `web`).
fn last_segment(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Run the matched VS's iRule(s) under the embedded orchestrator with the
/// captured request state. Extracts the iRule sources, orchestrator profiles,
/// pools, and request from the config/session, then delegates to the shared,
/// config-agnostic [`tcl_irule_test::simulate_irule`] (best-effort: any failure
/// lands in [`SimOutcome::error`] so the static report still renders).
fn simulate_session(cfg: &BigipConfig, vs: &BigipVirtualServer, session: &Session) -> SimOutcome {
    // Collect every attached iRule's source, resolving short refs.
    let sources: Vec<String> = vs
        .rules
        .paths()
        .iter()
        .filter_map(|rref| {
            let resolved = resolve_name(cfg, rref, "rules").unwrap_or_else(|| rref.clone());
            find_rule(cfg, &resolved).map(|rule| rule.source.clone())
        })
        .collect();
    if sources.is_empty() {
        return SimOutcome {
            error: String::from("no iRules attached to VS — nothing to simulate"),
            ..SimOutcome::default()
        };
    }

    let profiles = profiles_for_orchestrator(cfg, vs);

    // Every config pool (a `pool foo` call inside an iRule resolves through
    // this set) as `(short-name, member-node-names)`.
    let pools: Vec<(String, Vec<String>)> = cfg
        .objects
        .iter()
        .filter_map(|placed| {
            let ModelObject::Pool(pool) = &placed.object else {
                return None;
            };
            let members: Vec<String> = pool
                .members
                .paths()
                .iter()
                .filter(|m| !m.is_empty())
                .map(|m| last_segment(m).to_owned())
                .collect();
            (!members.is_empty()).then(|| (last_segment(&pool.full_path).to_owned(), members))
        })
        .collect();

    let front = &session.front.client;
    let request = tcl_irule_test::SimRequest {
        method: front.http_method.clone(),
        uri: front.http_uri.clone(),
        host: front.http_host.clone(),
        sni: front.tls_sni.clone(),
        headers: front.http_request_headers.clone(),
    };

    tcl_irule_test::simulate_irule(&sources, &profiles, &pools, Some(&request))
}

#[allow(
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::too_many_arguments
)] // flow extraction + per-session matching, read top-to-bottom
fn compute_explain_flow(
    pcap_display: &str,
    pcap_path: &Path,
    pcap_bytes: &[u8],
    configs: &[BigipConfig],
    show_event_bodies: bool,
    max_event_body_lines: usize,
    use_tshark: bool,
    keylog_path: &str,
    tshark_filter: &str,
    simulate: bool,
) -> Result<ExplainFlowReport, String> {
    let dest_re = Regex::new(
        r"^(?P<path>/[^/\s]+/)?(?P<addr>\[[^\]]+\]|[0-9a-fA-F\.:]+)(?:%\d+)?:(?P<port>\d+|any)$",
    )
    .expect("static destination regex");

    // Flow extraction has three paths:
    //   * `--tshark-filter` → tshark is the canonical flow source;
    //   * `--tshark` / `--keylog` → built-in walker + tshark L7 overlay;
    //   * neither → built-in walker alone.
    let mut used_tshark = false;
    let flows = if tshark_filter.is_empty() {
        let mut flows = extract_flows(pcap_bytes).map_err(|e| e.to_string())?;
        if use_tshark && tshark_available() {
            used_tshark = enrich_with_tshark(&mut flows, pcap_path, keylog_path, "");
        }
        flows
    } else {
        let flows = extract_flows_via_tshark(pcap_path, keylog_path, tshark_filter);
        used_tshark = !flows.is_empty() || tshark_available();
        flows
    };
    let flow_count = flows.len();
    let mut sessions = pair_sessions(&flows);
    let session_count = sessions.len();

    // Sort: paired (front+back) first, then by descending front packet count.
    sessions.sort_by(|a, b| {
        let rank = |s: &Session| u8::from(s.back.is_none());
        let pkts = |s: &Session| -> i128 {
            let c = i128::from(s.front.client.packets);
            let sv = s.front.server.as_ref().map_or(0, |f| i128::from(f.packets));
            -(c + sv)
        };
        rank(a).cmp(&rank(b)).then_with(|| pkts(a).cmp(&pkts(b)))
    });

    let mut session_explains: Vec<SessionExplain> = Vec::new();
    let mut matched = 0usize;

    for session in sessions {
        let front = &session.front;

        let mut vs_path: Option<String> = None;
        let mut cfg_hit: Option<&BigipConfig> = None;
        for cfg in configs {
            let mut hit = match_virtual(cfg, &front.client.dst_ip, front.client.dst_port, &dest_re);
            if hit.is_none()
                && let Some(server) = &front.server
            {
                hit = match_virtual(cfg, &server.src_ip, server.src_port, &dest_re);
            }
            if hit.is_some() {
                vs_path = hit;
                cfg_hit = Some(cfg);
                break;
            }
        }

        let (Some(vs_path), Some(cfg_hit)) = (vs_path, cfg_hit) else {
            let reset = analyse_reset(&session);
            session_explains.push(SessionExplain {
                session: Some(SessionHolder(session)),
                reset_analysis: reset,
                ..Default::default()
            });
            continue;
        };

        matched += 1;
        let Some(vs) = find_virtual(cfg_hit, &vs_path) else {
            // The matcher returned a path it can no longer fetch (shouldn't
            // happen); fall back to the bare match line.
            let reset = analyse_reset(&session);
            session_explains.push(SessionExplain {
                session: Some(SessionHolder(session)),
                matched_vs: vs_path,
                reset_analysis: reset,
                ..Default::default()
            });
            continue;
        };
        let partition = if vs_path.starts_with('/') {
            vs_path.split('/').nth(1).unwrap_or("").to_owned()
        } else {
            String::new()
        };

        let mut profile_chain: Vec<String> = Vec::new();
        for pref in vs.profiles.paths() {
            let resolved = resolve_profile(cfg_hit, &pref).unwrap_or_else(|| pref.clone());
            if let Some(prof) = find_profile(cfg_hit, &resolved) {
                profile_chain.push(format!(
                    "{resolved} ({})",
                    prof.profile_type.py_name().to_lowercase()
                ));
            } else {
                profile_chain.push(format!("{pref} (unresolved)"));
            }
        }

        // iRule event chain: ordered firing sequence, verbatim bodies, and
        // HUD annotations tying iRule commands to captured connection state.
        let mut event_sequence: Vec<String> = Vec::new();
        let mut event_blocks: Vec<EventBlock> = Vec::new();
        let mut event_annotations: Vec<EventAnnotation> = Vec::new();
        for rref in vs.rules.paths() {
            let resolved_rule =
                resolve_name(cfg_hit, &rref, "rules").unwrap_or_else(|| rref.clone());
            let Some(rule) = find_rule(cfg_hit, &resolved_rule) else {
                continue;
            };
            let blocks = extract_event_blocks(&rule.source);
            let rule_events: Vec<&str> = blocks.keys().map(String::as_str).collect();
            let ordered_events = expected_event_sequence(cfg_hit, vs, front, &rule_events);
            for ev in ordered_events {
                event_sequence.push(format!("{resolved_rule}::{ev}"));
                let full_body = &blocks[&ev];
                let mut ann = annotate_event_block(full_body, front);
                // Synthesise LB::server from a paired back side, if referenced.
                if let Some(back) = &session.back {
                    let bc = &back.client;
                    let lb_value = format!("{}:{}", bc.dst_ip, bc.dst_port);
                    if !ann.iter().any(|a| a.1.starts_with("LB::")) {
                        for line in py_splitlines(full_body) {
                            if line.contains("LB::server") {
                                ann.push((
                                    line.trim().to_owned(),
                                    "LB::server".to_owned(),
                                    lb_value,
                                ));
                                break;
                            }
                        }
                    }
                }
                if !ann.is_empty() {
                    event_annotations.push((resolved_rule.clone(), ev.clone(), ann));
                }
                if show_event_bodies {
                    let body_lines = py_splitlines(full_body);
                    let body = if body_lines.len() > max_event_body_lines {
                        format!(
                            "{}\n... (truncated)",
                            body_lines[..max_event_body_lines].join("\n")
                        )
                    } else {
                        full_body.clone()
                    };
                    event_blocks.push((resolved_rule.clone(), ev.clone(), body));
                }
            }
        }

        let ltm_policies = ltm_policies_for(cfg_hit, vs);
        let policy_decisions = evaluate_attached_policies(cfg_hit, &ltm_policies, &session);
        let apm = apm_profile_for(cfg_hit, vs);
        let gtm = gtm_wide_ips_in_config(cfg_hit);

        // Pool selection + SNAT inferred from the back-side flow if present.
        let mut pool_selected = String::new();
        let mut snat_observed = String::new();
        if let Some(back) = &session.back {
            let bc = &back.client;
            pool_selected = format!("{}:{}", bc.dst_ip, bc.dst_port);
            if bc.src_ip != session.front.client.src_ip {
                snat_observed = format!("{}:{}", bc.src_ip, bc.src_port);
            }
        }

        let explain_report =
            explain::compute_explain(cfg_hit, &vs.full_path, Some(explain::ExplainKind::Virtual));
        let explain_full_path = explain::full_path_of(
            &explain_report,
            cfg_hit,
            Some(explain::ExplainKind::Virtual),
        );
        let explain_text = explain::format_text(&explain_report, &explain_full_path);

        let reset = analyse_reset(&session);

        // Dynamic iRule simulation (`--simulate`): run the matched VS's iRules
        // under the embedded orchestrator with this session's captured request.
        let simulated = simulate.then(|| simulate_session(cfg_hit, vs, &session));

        session_explains.push(SessionExplain {
            session: Some(SessionHolder(session)),
            matched_vs: vs.full_path.clone(),
            matched_partition: partition,
            profile_chain,
            pool_selected,
            snat_observed,
            event_sequence,
            event_blocks,
            event_annotations,
            ltm_policies,
            policy_decisions,
            apm_profile: apm,
            gtm_wide_ips: gtm,
            explain_text,
            reset_analysis: reset,
            simulated,
        });
    }

    Ok(ExplainFlowReport {
        pcap_path: pcap_display.to_owned(),
        flow_count,
        session_count,
        matched_count: matched,
        used_tshark,
        keylog_path: keylog_path.to_owned(),
        tshark_filter: tshark_filter.to_owned(),
        sessions: session_explains,
    })
}

/// Render the text report.
// One straight-line report builder (header + per-session sections); splitting
// it into helpers would scatter the output layout.
#[allow(clippy::too_many_lines)]
fn format_report(report: &ExplainFlowReport) -> String {
    let pcap_path = &report.pcap_path;
    let sessions = &report.sessions;
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("explain-flow: {pcap_path}"));
    let matched = sessions.iter().filter(|s| !s.matched_vs.is_empty()).count();
    let keylog = if report.keylog_path.is_empty() {
        String::new()
    } else {
        format!(" | keylog: {}", report.keylog_path)
    };
    let filter = if report.tshark_filter.is_empty() {
        String::new()
    } else {
        format!(" | filter: {}", py_repr(&report.tshark_filter))
    };
    lines.push(format!(
        "  sessions: {} | matched: {} | tshark: {}{keylog}{filter}",
        sessions.len(),
        matched,
        if report.used_tshark { "yes" } else { "no" },
    ));
    lines.push(String::new());

    for (i, se) in sessions.iter().enumerate() {
        let s = se.session();
        lines.push(format!("[session {}] {}", i + 1, s.proto_name()));
        lines.push(format!("  front: {}", s.front.summary()));
        if let Some(back) = &s.back {
            lines.push(format!("  back:  {}", back.summary()));
        }
        if se.matched_vs.is_empty() {
            lines.push("  (no virtual server matched this destination)".to_owned());
            if !se.reset_analysis.is_empty() {
                lines.push(format!("  termination: {}", se.reset_analysis));
            }
            lines.push(String::new());
            continue;
        }
        lines.push(format!("  matched virtual: {}", se.matched_vs));
        if !se.pool_selected.is_empty() {
            lines.push(format!(
                "  pool member chosen (observed): {}",
                se.pool_selected
            ));
        }
        if !se.snat_observed.is_empty() {
            lines.push(format!("  SNAT applied (observed): {}", se.snat_observed));
        }
        if !se.profile_chain.is_empty() {
            lines.push("  profiles (in attach order):".to_owned());
            for p in &se.profile_chain {
                lines.push(format!("    - {p}"));
            }
        }
        if !se.ltm_policies.is_empty() {
            lines.push("  ltm policies:".to_owned());
            for p in &se.ltm_policies {
                lines.push(format!("    - {p}"));
            }
        }
        if !se.policy_decisions.is_empty() {
            lines.push("  ltm policy decisions:".to_owned());
            for pd in &se.policy_decisions {
                lines.push(format!(
                    "    --- {} (strategy: {}) ---",
                    pd.policy, pd.strategy
                ));
                for rd in &pd.rules {
                    let state = if rd.fired {
                        "FIRED"
                    } else if rd.matched {
                        "matched"
                    } else {
                        "no-match"
                    };
                    lines.push(format!(
                        "      rule {} (ord {}): {state}",
                        py_repr(&rd.rule),
                        rd.ordinal
                    ));
                    for ct in &rd.conditions {
                        use std::fmt::Write as _;
                        let mut target = ct.operand.clone();
                        if !ct.name.is_empty() {
                            let _ = write!(target, "[{}]", ct.name);
                        } else if !ct.selector.is_empty() {
                            let _ = write!(target, ".{}", ct.selector);
                        }
                        let prefix = if ct.matched { "      - " } else { "        " };
                        if ct.note.is_empty() {
                            lines.push(format!(
                                "{prefix}{target} {} {} -> actual={} match={}",
                                ct.operator,
                                py_list_repr(&ct.expected),
                                py_repr(&ct.actual),
                                py_bool(ct.matched)
                            ));
                        } else {
                            lines.push(format!(
                                "{prefix}{target} {} {} (skipped: {})",
                                ct.operator,
                                py_list_repr(&ct.expected),
                                ct.note
                            ));
                        }
                    }
                    for at in &rd.actions {
                        lines.push(
                            format!("        action: {} {} {}", at.target, at.verb, at.value)
                                .trim_end()
                                .to_owned(),
                        );
                    }
                }
            }
        }
        if !se.apm_profile.is_empty() {
            lines.push(format!("  apm: {}", se.apm_profile));
        }
        if !se.gtm_wide_ips.is_empty() {
            lines.push(
                "  gtm wide-ips in config (global inventory; may or may not point at this VS):"
                    .to_owned(),
            );
            for w in &se.gtm_wide_ips {
                lines.push(format!("    - {w}"));
            }
        }
        if !se.event_sequence.is_empty() {
            lines.push("  expected iRule event firing order:".to_owned());
            for ev in &se.event_sequence {
                lines.push(format!("    -> {ev}"));
            }
        }
        if !se.event_blocks.is_empty() {
            lines.push("  iRule event bodies (path through):".to_owned());
            for (rule, ev, body) in &se.event_blocks {
                lines.push(format!("    --- {rule} :: when {ev} ---"));
                for body_line in py_splitlines(body) {
                    lines.push(format!("      {body_line}"));
                }
            }
        }
        if !se.event_annotations.is_empty() {
            lines.push("  HUD-state captured for iRule commands:".to_owned());
            for (rule, ev, anns) in &se.event_annotations {
                lines.push(format!("    --- {rule} :: when {ev} ---"));
                for (line_excerpt, cmd, value) in anns {
                    lines.push(format!("      [{cmd}] = {}", py_repr(value)));
                    lines.push(format!("          ({line_excerpt})"));
                }
            }
        }
        // Captured request (front-side client flow).
        let fc = &s.front.client;
        if fc.http_request_seen || fc.tls_clienthello {
            lines.push("  captured request:".to_owned());
            if !fc.http_method.is_empty() || !fc.http_uri.is_empty() {
                let method = if fc.http_method.is_empty() {
                    "?"
                } else {
                    &fc.http_method
                };
                let req_line = format!("{method} {} {}", fc.http_uri, fc.http_request_version);
                lines.push(format!("    request line: {}", req_line.trim()));
            }
            if !fc.http_host.is_empty() {
                lines.push(format!("    host: {}", fc.http_host));
            }
            if !fc.http_user_agent.is_empty() {
                lines.push(format!("    user-agent: {}", fc.http_user_agent));
            }
            if !fc.http_referer.is_empty() {
                lines.push(format!("    referer: {}", fc.http_referer));
            }
            if !fc.http_cookie.is_empty() {
                lines.push(format!("    cookie: {}", fc.http_cookie));
            }
            if !fc.tls_sni.is_empty() {
                use std::fmt::Write as _;
                let mut tls_line = format!("    tls: SNI={}", fc.tls_sni);
                if !fc.tls_chosen_version.is_empty() || !fc.tls_version.is_empty() {
                    let v = if fc.tls_chosen_version.is_empty() {
                        &fc.tls_version
                    } else {
                        &fc.tls_chosen_version
                    };
                    let _ = write!(tls_line, " version={v}");
                }
                if !fc.tls_chosen_cipher.is_empty() {
                    let _ = write!(tls_line, " cipher={}", fc.tls_chosen_cipher);
                }
                if !fc.tls_alpn.is_empty() {
                    let _ = write!(tls_line, " alpn={}", fc.tls_alpn);
                }
                lines.push(tls_line);
            }
            if !fc.tls_cert_subject.is_empty() {
                lines.push(format!("    server cert subject: {}", fc.tls_cert_subject));
            }
        }
        // Captured response (server-side flow).
        if let Some(sc) = &s.front.server
            && (sc.http_response_seen || !sc.http_response_code.is_empty())
        {
            lines.push("  captured response:".to_owned());
            let status = format!("{} {}", sc.http_response_code, sc.http_response_phrase);
            let status = status.trim();
            let status = if status.is_empty() { "(none)" } else { status };
            lines.push(format!("    status: {status}"));
            if !sc.http_response_content_type.is_empty() {
                lines.push(format!(
                    "    content-type: {}",
                    sc.http_response_content_type
                ));
            }
            if !sc.http_response_content_length.is_empty() {
                lines.push(format!(
                    "    content-length: {}",
                    sc.http_response_content_length
                ));
            }
        }
        // Dynamic iRule-simulation outcome (`--simulate`).
        if let Some(sim) = &se.simulated {
            lines.push("  iRule simulation:".to_owned());
            if sim.error.is_empty() {
                lines.push(format!(
                    "    pool: {}",
                    if sim.pool.is_empty() {
                        "(none)"
                    } else {
                        &sim.pool
                    }
                ));
                if !sim.node.is_empty() {
                    lines.push(format!("    node: {}", sim.node));
                }
                if sim.response_committed {
                    lines.push("    response: committed by iRule".to_owned());
                }
                for (category, action, value) in &sim.decisions {
                    let value = if value.is_empty() {
                        String::new()
                    } else {
                        format!(" {value}")
                    };
                    lines.push(format!("    decision: {category} {action}{value}"));
                }
                for log in &sim.logs {
                    lines.push(format!("    log: {log}"));
                }
            } else {
                lines.push(format!("    error: {}", sim.error));
            }
        }
        lines.push(format!("  termination: {}", se.reset_analysis));
        lines.push("  resolved plan:".to_owned());
        for explain_line in py_splitlines(&se.explain_text) {
            lines.push(format!("    {explain_line}"));
        }
        lines.push(String::new());
    }

    let joined = lines.join("\n");
    format!("{}\n", joined.trim_end())
}

/// Split a string on Unicode line boundaries: break on each recognised
/// line-boundary character, treat `\r\n` as one break, and emit no trailing
/// empty segment when the string ends on a boundary.
fn py_splitlines(s: &str) -> Vec<&str> {
    fn is_boundary(c: char) -> bool {
        matches!(
            c,
            '\n' | '\r'
                | '\u{0B}'
                | '\u{0C}'
                | '\u{1C}'
                | '\u{1D}'
                | '\u{1E}'
                | '\u{85}'
                | '\u{2028}'
                | '\u{2029}'
        )
    }
    let mut out: Vec<&str> = Vec::new();
    let mut start = 0usize;
    let mut chars = s.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if !is_boundary(c) {
            continue;
        }
        out.push(&s[start..i]);
        let mut end = i + c.len_utf8();
        if c == '\r'
            && let Some(&(j, '\n')) = chars.peek()
        {
            end = j + '\n'.len_utf8();
            chars.next();
        }
        start = end;
    }
    if start < s.len() {
        out.push(&s[start..]);
    }
    out
}

/// Render a `bool` as the capitalised `True`/`False` tokens the HUD uses.
fn py_bool(b: bool) -> &'static str {
    if b { "True" } else { "False" }
}

/// Render a list of strings in the `['a', 'b']` repr form.
fn py_list_repr(values: &[String]) -> String {
    let inner: Vec<String> = values.iter().map(|v| py_repr(v)).collect();
    format!("[{}]", inner.join(", "))
}

/// Render a string in repr form for the HUD annotations
/// (single-quoted, with `\\`, `\'`, `\n`, `\r`, `\t` escapes).
fn py_repr(s: &str) -> String {
    let use_double = s.contains('\'') && !s.contains('"');
    let quote = if use_double { '"' } else { '\'' };
    let mut out = String::with_capacity(s.len() + 2);
    out.push(quote);
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

/// Build an insertion-ordered JSON object (relies on the `serde_json`
/// `preserve_order` feature so keys serialise in insertion order).
fn obj(pairs: Vec<(&str, Value)>) -> Value {
    let mut map = Map::new();
    for (k, v) in pairs {
        map.insert(k.to_owned(), v);
    }
    Value::Object(map)
}

fn str_array(items: &[String]) -> Value {
    Value::Array(items.iter().map(|s| Value::String(s.clone())).collect())
}

fn headers_value(headers: &[(String, String)]) -> Value {
    let mut map = Map::new();
    for (k, v) in headers {
        map.insert(k.clone(), Value::String(v.clone()));
    }
    Value::Object(map)
}

/// Serialise a [`Flow`] to its JSON object form.
fn flow_to_value(f: &Flow) -> Value {
    obj(vec![
        ("src_ip", Value::String(f.src_ip.clone())),
        ("src_port", Value::from(f.src_port)),
        ("dst_ip", Value::String(f.dst_ip.clone())),
        ("dst_port", Value::from(f.dst_port)),
        ("proto", Value::String(f.proto_name())),
        ("packets", Value::from(f.packets)),
        ("bytes", Value::from(f.bytes_total)),
        ("tcp_syn", Value::Bool(f.tcp_syn)),
        ("tcp_synack", Value::Bool(f.tcp_synack)),
        ("tcp_fin", Value::Bool(f.tcp_fin)),
        ("tcp_rst", Value::Bool(f.tcp_rst)),
        ("tcp_rst_count", Value::from(f.tcp_rst_count)),
        ("tcp_rst_after_bytes", Value::from(f.tcp_rst_after_bytes)),
        ("tls_clienthello", Value::Bool(f.tls_clienthello)),
        ("tls_sni", Value::String(f.tls_sni.clone())),
        ("tls_version", Value::String(f.tls_version.clone())),
        (
            "tls_chosen_version",
            Value::String(f.tls_chosen_version.clone()),
        ),
        (
            "tls_chosen_cipher",
            Value::String(f.tls_chosen_cipher.clone()),
        ),
        ("tls_alpn", Value::String(f.tls_alpn.clone())),
        ("tls_alert_seen", Value::Bool(f.tls_alert_seen)),
        ("tls_alert_desc", Value::String(f.tls_alert_desc.clone())),
        ("http_method", Value::String(f.http_method.clone())),
        ("http_host", Value::String(f.http_host.clone())),
        ("http_uri", Value::String(f.http_uri.clone())),
        ("http_response_seen", Value::Bool(f.http_response_seen)),
        ("http_path", Value::String(f.http_path.clone())),
        ("http_query", Value::String(f.http_query.clone())),
        (
            "http_request_version",
            Value::String(f.http_request_version.clone()),
        ),
        ("http_user_agent", Value::String(f.http_user_agent.clone())),
        ("http_cookie", Value::String(f.http_cookie.clone())),
        ("http_referer", Value::String(f.http_referer.clone())),
        (
            "http_request_content_type",
            Value::String(f.http_request_content_type.clone()),
        ),
        (
            "http_request_content_length",
            Value::String(f.http_request_content_length.clone()),
        ),
        (
            "http_request_headers",
            headers_value(&f.http_request_headers),
        ),
        (
            "http_response_code",
            Value::String(f.http_response_code.clone()),
        ),
        (
            "http_response_phrase",
            Value::String(f.http_response_phrase.clone()),
        ),
        (
            "http_response_content_type",
            Value::String(f.http_response_content_type.clone()),
        ),
        (
            "http_response_content_length",
            Value::String(f.http_response_content_length.clone()),
        ),
        (
            "http_response_headers",
            headers_value(&f.http_response_headers),
        ),
        (
            "tls_cert_subject",
            Value::String(f.tls_cert_subject.clone()),
        ),
        ("tls_cert_issuer", Value::String(f.tls_cert_issuer.clone())),
        (
            "tls_supported_groups",
            Value::String(f.tls_supported_groups.clone()),
        ),
        (
            "tls_signature_algos",
            Value::String(f.tls_signature_algos.clone()),
        ),
        ("f5_reset_causes", str_array(&f.f5_reset_causes)),
        ("peer_remote_ip", Value::String(f.peer_remote_ip.clone())),
        ("peer_remote_port", Value::from(f.peer_remote_port)),
        ("peer_local_ip", Value::String(f.peer_local_ip.clone())),
        ("peer_local_port", Value::from(f.peer_local_port)),
    ])
}

/// Serialise an optional [`Connection`] to JSON (`None` → JSON null).
fn conn_to_value(conn: Option<&Connection>) -> Value {
    match conn {
        None => Value::Null,
        Some(c) => obj(vec![
            ("client", flow_to_value(&c.client)),
            (
                "server",
                c.server.as_ref().map_or(Value::Null, flow_to_value),
            ),
        ]),
    }
}

/// Serialise a [`PolicyDecision`] to its JSON object form.
fn policy_decision_to_value(pd: &PolicyDecision) -> Value {
    let rules: Vec<Value> = pd
        .rules
        .iter()
        .map(|rd| {
            let conditions: Vec<Value> = rd
                .conditions
                .iter()
                .map(|ct| {
                    obj(vec![
                        ("operand", Value::String(ct.operand.clone())),
                        ("selector", Value::String(ct.selector.clone())),
                        ("operator", Value::String(ct.operator.clone())),
                        ("expected", str_array(&ct.expected)),
                        ("actual", Value::String(ct.actual.clone())),
                        ("matched", Value::Bool(ct.matched)),
                        ("name", Value::String(ct.name.clone())),
                        ("negate", Value::Bool(ct.negate)),
                        ("event", Value::String(ct.event.clone())),
                        ("note", Value::String(ct.note.clone())),
                    ])
                })
                .collect();
            let actions: Vec<Value> = rd
                .actions
                .iter()
                .map(|at| {
                    obj(vec![
                        ("target", Value::String(at.target.clone())),
                        ("verb", Value::String(at.verb.clone())),
                        ("value", Value::String(at.value.clone())),
                    ])
                })
                .collect();
            obj(vec![
                ("rule", Value::String(rd.rule.clone())),
                ("ordinal", Value::from(rd.ordinal)),
                ("matched", Value::Bool(rd.matched)),
                ("fired", Value::Bool(rd.fired)),
                ("conditions", Value::Array(conditions)),
                ("actions", Value::Array(actions)),
            ])
        })
        .collect();
    obj(vec![
        ("policy", Value::String(pd.policy.clone())),
        ("strategy", Value::String(pd.strategy.clone())),
        ("rules", Value::Array(rules)),
    ])
}

/// Serialise one per-session entry to its JSON object form.
fn session_to_value(se: &SessionExplain) -> Value {
    let s = se.session();
    let sim = se.simulated.as_ref();
    let session = obj(vec![
        ("front", conn_to_value(Some(&s.front))),
        ("back", conn_to_value(s.back.as_ref())),
    ]);
    let event_blocks: Vec<Value> = se
        .event_blocks
        .iter()
        .map(|(rule, event, body)| {
            obj(vec![
                ("rule", Value::String(rule.clone())),
                ("event", Value::String(event.clone())),
                ("body", Value::String(body.clone())),
            ])
        })
        .collect();
    let event_annotations: Vec<Value> = se
        .event_annotations
        .iter()
        .map(|(rule, event, anns)| {
            let annotations: Vec<Value> = anns
                .iter()
                .map(|(line, cmd, value)| {
                    obj(vec![
                        ("line", Value::String(line.clone())),
                        ("command", Value::String(cmd.clone())),
                        ("value", Value::String(value.clone())),
                    ])
                })
                .collect();
            obj(vec![
                ("rule", Value::String(rule.clone())),
                ("event", Value::String(event.clone())),
                ("annotations", Value::Array(annotations)),
            ])
        })
        .collect();
    let policy_decisions: Vec<Value> = se
        .policy_decisions
        .iter()
        .map(policy_decision_to_value)
        .collect();
    obj(vec![
        ("session", session),
        ("matched_vs", Value::String(se.matched_vs.clone())),
        ("partition", Value::String(se.matched_partition.clone())),
        ("profile_chain", str_array(&se.profile_chain)),
        ("pool_selected", Value::String(se.pool_selected.clone())),
        ("snat_observed", Value::String(se.snat_observed.clone())),
        ("event_sequence", str_array(&se.event_sequence)),
        ("event_blocks", Value::Array(event_blocks)),
        ("event_annotations", Value::Array(event_annotations)),
        ("ltm_policies", str_array(&se.ltm_policies)),
        ("policy_decisions", Value::Array(policy_decisions)),
        ("apm_profile", Value::String(se.apm_profile.clone())),
        ("gtm_wide_ips", str_array(&se.gtm_wide_ips)),
        ("explain_text", Value::String(se.explain_text.clone())),
        ("reset_analysis", Value::String(se.reset_analysis.clone())),
        // `--simulate` outcome (all empty / false when not requested).
        (
            "simulated_pool",
            Value::String(sim.map(|s| s.pool.clone()).unwrap_or_default()),
        ),
        (
            "simulated_node",
            Value::String(sim.map(|s| s.node.clone()).unwrap_or_default()),
        ),
        (
            "simulated_response_committed",
            Value::Bool(sim.is_some_and(|s| s.response_committed)),
        ),
        (
            "simulated_logs",
            str_array(sim.map_or(&[][..], |s| s.logs.as_slice())),
        ),
        (
            "simulated_decisions",
            Value::Array(sim.map_or_else(Vec::new, |s| {
                s.decisions
                    .iter()
                    .map(|(category, action, value)| {
                        obj(vec![
                            ("category", Value::String(category.clone())),
                            ("action", Value::String(action.clone())),
                            ("value", Value::String(value.clone())),
                        ])
                    })
                    .collect()
            })),
        ),
        (
            "simulation_error",
            Value::String(sim.map(|s| s.error.clone()).unwrap_or_default()),
        ),
    ])
}

/// Serialise the full report as 2-space-indented JSON.
fn report_to_value(report: &ExplainFlowReport) -> Value {
    let sessions: Vec<Value> = report.sessions.iter().map(session_to_value).collect();
    obj(vec![
        ("pcap_path", Value::String(report.pcap_path.clone())),
        ("flow_count", Value::from(report.flow_count)),
        ("session_count", Value::from(report.session_count)),
        ("matched_count", Value::from(report.matched_count)),
        ("used_tshark", Value::Bool(report.used_tshark)),
        ("keylog_path", Value::String(report.keylog_path.clone())),
        ("tshark_filter", Value::String(report.tshark_filter.clone())),
        ("sessions", Value::Array(sessions)),
    ])
}

fn report_to_json(report: &ExplainFlowReport) -> Result<String, String> {
    let value = report_to_value(report);
    let pretty = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
    Ok(format!("{}\n", tcl_cli_support::ensure_ascii(&pretty)))
}

/// Options for [`explain_flow_value`] — the request config as one value so the
/// entry point stays a clean two-argument call.
#[derive(Debug, Default, Clone)]
pub struct ExplainFlowOptions<'a> {
    /// BIG-IP config files to load + merge for VS / iRule resolution.
    pub paths: &'a [std::path::PathBuf],
    /// Force the tshark overlay (also implied by `keylog` / `tshark_filter`).
    pub tshark: bool,
    /// TLS keylog file, forwarded to tshark for decryption.
    pub keylog: Option<&'a Path>,
    /// Extra tshark display filter.
    pub tshark_filter: Option<&'a str>,
    /// Run the iRule simulation pass.
    pub simulate: bool,
    /// Omit per-event iRule body listings.
    pub no_event_bodies: bool,
    /// Cap on event-body lines shown per event.
    pub max_event_lines: usize,
}

/// Compute the explain-flow report for a pcap + BIG-IP configs and return it as
/// a JSON value (the `f5 explain-flow --json` shape). For embedders — the
/// native MCP `explain_flow` tool calls this directly.
///
/// # Errors
/// Returns a message when the pcap isn't a readable file or config loading /
/// flow computation fails.
pub fn explain_flow_value(pcap: &Path, options: &ExplainFlowOptions<'_>) -> Result<Value, String> {
    if !pcap.is_file() {
        return Err(format!("not a file: {}", pcap.display()));
    }
    let use_tshark = options.tshark || options.keylog.is_some() || options.tshark_filter.is_some();
    let keylog_path = options
        .keylog
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tshark_filter = options.tshark_filter.unwrap_or("");

    let opts = crate::cli::PassphraseArgs::default().to_options();
    let path_strs: Vec<String> = options
        .paths
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    let configs: Vec<BigipConfig> = tcl_bigip_io::load_paths(&path_strs, &opts)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|loaded| loaded.config)
        .collect();

    let pcap_bytes = std::fs::read(pcap).map_err(|e| e.to_string())?;
    let report = compute_explain_flow(
        &pcap.display().to_string(),
        pcap,
        &pcap_bytes,
        &configs,
        !options.no_event_bodies,
        options.max_event_lines,
        use_tshark,
        &keylog_path,
        tshark_filter,
        options.simulate,
    )?;
    Ok(report_to_value(&report))
}

/// Entry point for `f5 explain-flow` (built-in walker / static path).
// Arg list and bool flags mirror the verb's CLI options one-for-one; a struct
// would just re-wrap the same one-shot parameters.
#[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
pub fn run_explain_flow(
    pcap: &Path,
    paths: &[std::path::PathBuf],
    tshark: bool,
    keylog: Option<&Path>,
    tshark_filter: Option<&str>,
    simulate: bool,
    no_event_bodies: bool,
    max_event_lines: usize,
    json: bool,
    output: Option<&Path>,
) -> anyhow::Result<u8> {
    if !pcap.is_file() {
        anyhow::bail!("not a file: {}", pcap.display());
    }
    // `--tshark-filter` implies tshark; `--tshark` / `--keylog` request the
    // overlay. `keylog` is forwarded to tshark only when it points at a file.
    let use_tshark = tshark || keylog.is_some() || tshark_filter.is_some();
    let keylog_path = keylog
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tshark_filter = tshark_filter.unwrap_or("");

    let opts = crate::cli::PassphraseArgs::default().to_options();
    let path_strs: Vec<String> = paths
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    let configs: Vec<BigipConfig> = tcl_bigip_io::load_paths(&path_strs, &opts)
        .map_err(|e| anyhow::anyhow!("{e}"))?
        .into_iter()
        .map(|loaded| loaded.config)
        .collect();

    let pcap_bytes = std::fs::read(pcap)?;
    let report = compute_explain_flow(
        &pcap.display().to_string(),
        pcap,
        &pcap_bytes,
        &configs,
        !no_event_bodies,
        max_event_lines,
        use_tshark,
        &keylog_path,
        tshark_filter,
        simulate,
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;

    let rendered = if json {
        report_to_json(&report).map_err(|e| anyhow::anyhow!("{e}"))?
    } else {
        format_report(&report)
    };

    let target = OutputTarget::from_arg(output);
    write_text_output(&target, &rendered)?;

    Ok(u8::from(report.matched_count == 0))
}
