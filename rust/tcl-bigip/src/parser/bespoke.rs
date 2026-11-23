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

//! Bespoke structural per-kind parsers — the structured-field half that
//! the generated scalar parsers leave at `Default`.
//!
//! The generated scalar parser is invoked first for the scalar half; this
//! module then fills the structured fields (typed value-layer values,
//! keyed-block lists, sub-records) and overrides any scalar field the
//! generator gets wrong (e.g. unquoted monitor/profile fields, the
//! `data-group` `type` key, the `gtm wideip` `last-resort-pool` prefix
//! strip).
//!
//! Offset-sensitive fields (pool-member `field_offsets`, virtual
//! `pool_range`) need the absolute source position of the block, so the
//! driver threads a [`BespokeCtx`] carrying the source text, its line
//! index, and the block's `start_offset`.

use std::collections::HashMap;

use tcl_lexer::LineIndex;
use tcl_registry::bigip::BigipRegistry;

use crate::model::r#gen::parsers::{
    parse_bigip_apm_oauth_db_instance, parse_bigip_apm_policy_agent, parse_bigip_apm_policy_item,
    parse_bigip_auth_partition, parse_bigip_cm_device, parse_bigip_cm_device_group,
    parse_bigip_cm_traffic_group, parse_bigip_cm_trust_domain, parse_bigip_data_group,
    parse_bigip_gtm_datacenter, parse_bigip_gtm_pool, parse_bigip_gtm_prober_pool,
    parse_bigip_gtm_rule, parse_bigip_gtm_server, parse_bigip_gtm_wideip,
    parse_bigip_ltm_dns_cache_resolver, parse_bigip_monitor, parse_bigip_net_dns_resolver,
    parse_bigip_net_interface, parse_bigip_net_port_list, parse_bigip_net_route,
    parse_bigip_net_route_domain, parse_bigip_net_self, parse_bigip_net_stp, parse_bigip_net_vlan,
    parse_bigip_node, parse_bigip_pem_listener, parse_bigip_pem_policy,
    parse_bigip_pem_service_chain_endpoint, parse_bigip_persistence, parse_bigip_policy,
    parse_bigip_pool, parse_bigip_profile, parse_bigip_rule, parse_bigip_security_firewall_policy,
    parse_bigip_security_firewall_port_list, parse_bigip_security_firewall_rule_list,
    parse_bigip_security_log_profile, parse_bigip_security_nat_policy,
    parse_bigip_security_packet_filter_policy, parse_bigip_snat_pool,
    parse_bigip_sys_file_ssl_cert, parse_bigip_sys_ntp, parse_bigip_sys_provision,
    parse_bigip_sys_snmp, parse_bigip_virtual_address, parse_bigip_virtual_server,
};
use crate::model::{
    BigipApmOauthDbInstance, BigipApmPolicyAgent, BigipApmPolicyItem, BigipAuthPartition,
    BigipCmDevice, BigipCmDeviceGroup, BigipCmTrafficGroup, BigipCmTrustDomain, BigipDataGroup,
    BigipGtmDatacenter, BigipGtmPool, BigipGtmPoolMember, BigipGtmProberPool, BigipGtmRule,
    BigipGtmServer, BigipGtmTopology, BigipGtmWideip, BigipLtmDnsCacheResolver, BigipMonitor,
    BigipNetDnsResolver, BigipNetInterface, BigipNetPortList, BigipNetRoute, BigipNetRouteDomain,
    BigipNetSelf, BigipNetStp, BigipNetVlan, BigipNode, BigipPemListener, BigipPemPolicy,
    BigipPemServiceChainEndpoint, BigipPersistence, BigipPolicy, BigipPolicyAction,
    BigipPolicyCondition, BigipPolicyRule, BigipPool, BigipPoolMember, BigipProfile, BigipRule,
    BigipSecurityFirewallPolicy, BigipSecurityFirewallPortList, BigipSecurityFirewallRuleList,
    BigipSecurityLogProfile, BigipSecurityNatPolicy, BigipSecurityPacketFilterPolicy,
    BigipSnatPool, BigipSysFileSslCert, BigipSysNtp, BigipSysNtpRestrict, BigipSysProvision,
    BigipSysSnmp, BigipSysSnmpDiskMonitor, BigipSysSnmpProcessMonitor, BigipSysSnmpTrap,
    BigipSysSnmpUser, BigipVirtualAddress, BigipVirtualServer, DataGroupType, ProfileType,
};
use crate::range::Range;
use crate::value::bigip_list::SourceSpan;
use crate::value::grammar::{ConfigurationAddress, read_address_property};
use crate::value::{
    BigipList, FirewallRule, ListItem, ListItemValue, ListSyntax, Network, PersistenceAttachment,
    ProfileAttachment,
};

use super::helpers::{
    Property, parse_keyed_block_entries, parse_keyed_block_entries_with_offsets, parse_list_block,
    parse_properties, parse_properties_with_spans, unquote,
};
use super::scalar::{description, props_map, state_flag};

/// Source context threaded from the driver so offset-sensitive structured
/// fields (pool-member `field_offsets`, virtual `pool_range`) can be
/// lifted from body-relative spans to absolute source positions.
#[derive(Clone, Copy)]
pub struct BespokeCtx<'a> {
    /// The full configuration source.
    pub source: &'a str,
    /// Its line index (for offset -> position resolution).
    pub line_index: &'a LineIndex,
    /// Absolute byte offset of the block's opening `{`.
    pub block_start: usize,
}

// Small helpers for primitives not in `scalar`/`helpers`.

/// Convert a byte offset into a code-point index (the offset convention
/// for non-ASCII sources).
fn byte_to_cp(source: &str, byte: usize) -> usize {
    source
        .get(..byte.min(source.len()))
        .map_or(0, |s| s.chars().count())
}

/// Strip a single layer of surrounding `{ ... }` from a sub-block value.
fn strip_outer_braces(text: &str) -> &str {
    let s = text.trim();
    let s = s.strip_prefix('{').unwrap_or(s);
    s.strip_suffix('}').unwrap_or(s).trim()
}

/// Strip surrounding double quotes after trimming.
fn strip_quotes(text: &str) -> String {
    let s = text.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        s[1..s.len() - 1].to_owned()
    } else {
        s.to_owned()
    }
}

/// For a block of anonymous sub-blocks like `{ { ip 10.0.0.1 } { ip
/// 10.0.0.2 } }`, extract `prop_name` from every direct sub-block in
/// document order.
fn collect_named_property_from_anon_subblocks(braced: &str, prop_name: &str) -> Vec<String> {
    let inner = strip_outer_braces(braced);
    let bytes = inner.as_bytes();
    let length = bytes.len();
    let mut out = Vec::new();
    let mut pos = 0;
    while pos < length {
        while pos < length && matches!(bytes[pos], b' ' | b'\t' | b'\n' | b'\r') {
            pos += 1;
        }
        if pos >= length || bytes[pos] != b'{' {
            break;
        }
        let block_start = pos;
        pos += 1;
        let mut depth = 1;
        while pos < length && depth > 0 {
            match bytes[pos] {
                b'{' => depth += 1,
                b'}' => depth -= 1,
                _ => {}
            }
            pos += 1;
        }
        let block_text = &inner[block_start..pos];
        let sub_inner = strip_outer_braces(block_text);
        if let Some((_, p)) = parse_properties_with_spans(sub_inner)
            .iter()
            .find(|(k, _)| k == prop_name)
        {
            out.push(p.value.clone());
        }
    }
    out
}

/// For a block like `{ 1 { ip ... } 2 { ip ... } }`, return the
/// `prop_name` value from every direct sub-block in document order.
fn collect_named_property_from_subblocks(braced: &str, prop_name: &str) -> Vec<String> {
    let inner = strip_outer_braces(braced);
    let mut out = Vec::new();
    for (_, prop) in parse_properties_with_spans(inner) {
        if !prop.value.starts_with('{') {
            continue;
        }
        let sub_inner = strip_outer_braces(&prop.value);
        if let Some((_, p)) = parse_properties_with_spans(sub_inner)
            .iter()
            .find(|(k, _)| k == prop_name)
        {
            out.push(p.value.clone());
        }
    }
    out
}

/// `(start, end)` of the first non-whitespace / non-`#{}` token.
fn first_scalar_token_span(value: &str) -> Option<(usize, usize)> {
    let bytes = value.as_bytes();
    let mut i = 0;
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r' | b'#' | b'{' | b'}') {
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }
    let start = i;
    while i < bytes.len() && !matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r' | b'#' | b'{' | b'}')
    {
        i += 1;
    }
    Some((start, i))
}

/// Quote-aware tokeniser for policy `values { ... }` lists.
fn parse_policy_values_block(braced: &str) -> Vec<String> {
    let inner = strip_outer_braces(braced);
    let bytes = inner.as_bytes();
    let mut out = Vec::new();
    let mut pos = 0;
    while pos < bytes.len() {
        let ch = bytes[pos];
        if matches!(ch, b' ' | b'\t' | b'\n' | b'\r') {
            pos += 1;
            continue;
        }
        if ch == b'"' {
            pos += 1;
            let mut buf = String::new();
            while pos < bytes.len() && bytes[pos] != b'"' {
                if bytes[pos] == b'\\' && pos + 1 < bytes.len() {
                    // Advance past the backslash, then by the escaped char's
                    // full UTF-8 width so multibyte chars don't split.
                    let c = inner[pos + 1..].chars().next().unwrap();
                    buf.push(c);
                    pos += 1 + c.len_utf8();
                    continue;
                }
                // Advance by the char's full UTF-8 width, not one byte, or the
                // next slice would land inside a multibyte char and panic.
                let c = inner[pos..].chars().next().unwrap();
                buf.push(c);
                pos += c.len_utf8();
            }
            if pos < bytes.len() && bytes[pos] == b'"' {
                pos += 1;
            }
            out.push(buf);
            continue;
        }
        let start = pos;
        while pos < bytes.len() && !matches!(bytes[pos], b' ' | b'\t' | b'\n' | b'\r') {
            pos += 1;
        }
        out.push(inner[start..pos].to_owned());
    }
    out
}

/// The known property keys for a `(module, object_type)` pair, used by
/// [`split_compact_props`].
fn property_names_for(module: &str, object_type: &str) -> Vec<String> {
    let registry = BigipRegistry::build();
    registry
        .get_by_header(module, object_type)
        .map_or_else(Vec::new, |spec| {
            spec.properties.iter().map(|p| p.name.to_owned()).collect()
        })
}

/// Quote-aware tokeniser for inline sibling-property splitting.
///
/// A double-quoted run is kept as a single token (quotes preserved) so a
/// multi-word scalar value like `"TLS enabled for clients"` is never
/// re-split on its inner whitespace, and its inner words are never
/// re-interpreted as property names. Backslash escapes are passed through
/// verbatim so slicing never lands mid-char.
fn tokenize_quote_aware(value: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut token = String::new();
    let mut in_quote = false;
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                in_quote = !in_quote;
                token.push(c);
            }
            '\\' => {
                token.push(c);
                if let Some(n) = chars.next() {
                    token.push(n);
                }
            }
            c if !in_quote && c.is_whitespace() => {
                if !token.is_empty() {
                    out.push(std::mem::take(&mut token));
                }
            }
            c => token.push(c),
        }
    }
    if !token.is_empty() {
        out.push(token);
    }
    out
}

/// Net brace depth of a token, ignoring braces inside a quoted run so a
/// quoted scalar containing `{`/`}` cannot desynchronise depth tracking.
fn brace_delta(tok: &str) -> i64 {
    let mut depth: i64 = 0;
    let mut in_quote = false;
    let mut chars = tok.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => in_quote = !in_quote,
            '\\' => {
                chars.next();
            }
            '{' if !in_quote => depth += 1,
            '}' if !in_quote => depth -= 1,
            _ => {}
        }
    }
    depth
}

/// Re-split a read-to-EOL value containing inline sibling properties.
/// Returns the additional inline pairs plus an optional `__head__` (the
/// owning key's real first-token value).
///
/// Splitting is quote-aware: a quoted multi-word scalar value is kept
/// intact and its inner words are not matched against `known`.
fn split_inline_keys(value: &str, known: &[String]) -> HashMap<String, String> {
    let mut out: HashMap<String, String> = HashMap::new();
    let tokens = tokenize_quote_aware(value);
    if tokens.is_empty() {
        return out;
    }
    let key_set: std::collections::HashSet<&str> = known.iter().map(String::as_str).collect();
    let mut current_key: Option<&str> = None;
    let mut current_value: Vec<&str> = vec![tokens[0].as_str()];
    let mut brace_depth: i64 = brace_delta(&tokens[0]);
    let mut head: Option<String> = None;
    for tok in &tokens[1..] {
        if brace_depth > 0 {
            current_value.push(tok);
            brace_depth += brace_delta(tok);
            continue;
        }
        if key_set.contains(tok.as_str()) {
            let joined = current_value.join(" ");
            if let Some(k) = current_key {
                out.insert(k.to_owned(), joined);
            } else {
                head = Some(joined);
            }
            current_key = Some(tok);
            current_value = Vec::new();
            continue;
        }
        current_value.push(tok);
        brace_depth += brace_delta(tok);
    }
    let Some(k) = current_key else {
        return HashMap::new();
    };
    out.insert(k.to_owned(), current_value.join(" "));
    if let Some(h) = head {
        out.insert("__head__".to_owned(), h);
    }
    out
}

/// Parse a body's top-level props, splitting inline sibling pairs against
/// `known`.
fn split_compact_props(body: &str, known: &[String]) -> HashMap<String, String> {
    let base = parse_properties(body);
    let mut out: HashMap<String, String> = base.iter().cloned().collect();
    for (key, value) in &base {
        if value.is_empty() || !value.contains(' ') {
            continue;
        }
        let inline = split_inline_keys(value, known);
        if inline.is_empty() {
            continue;
        }
        let mut inline = inline;
        if let Some(head) = inline.remove("__head__") {
            out.insert(key.clone(), head);
        }
        for (ik, iv) in inline {
            out.entry(ik).or_insert(iv);
        }
    }
    out
}

/// Map a profile type string to a [`ProfileType`].
fn classify_profile(type_str: &str) -> ProfileType {
    match type_str.to_lowercase().as_str() {
        "aimcp" => ProfileType::Aimcp,
        "http" | "http2" | "http-compression" | "http-proxy-connect" | "web-acceleration" => {
            ProfileType::Http
        }
        "tcp" => ProfileType::Tcp,
        "udp" => ProfileType::Udp,
        "client-ssl" => ProfileType::ClientSsl,
        "server-ssl" => ProfileType::ServerSsl,
        "ftp" => ProfileType::Ftp,
        "dns" => ProfileType::Dns,
        "sip" => ProfileType::Sip,
        "diameter" => ProfileType::Diameter,
        "fix" => ProfileType::Fix,
        "radius" => ProfileType::Radius,
        "mqtt" => ProfileType::Mqtt,
        "websocket" => ProfileType::Websocket,
        "stream" => ProfileType::Stream,
        "sse" => ProfileType::Sse,
        "html" => ProfileType::Html,
        "json" => ProfileType::Json,
        "rewrite" => ProfileType::Rewrite,
        "fasthttp" => ProfileType::Fasthttp,
        "fastl4" => ProfileType::Fastl4,
        "one-connect" => ProfileType::OneConnect,
        _ => ProfileType::Other,
    }
}

/// Policy condition operand vocabulary.
const POLICY_OPERANDS: &[&str] = &[
    "http-method",
    "geoip",
    "http-cookie",
    "http-version",
    "ssl-extension",
    "http-header",
    "http-host",
    "http-user-agent",
    "tcp",
    "http-referer",
    "http-status",
    "ssl-cert",
    "http-uri",
];

/// Policy condition selector vocabulary.
const POLICY_SELECTORS: &[&str] = &[
    "extension",
    "scheme",
    "query",
    "path",
    "method",
    "version",
    "port",
    "host",
    "all",
    "address",
];

/// Policy condition operator vocabulary.
const POLICY_OPERATORS: &[&str] = &[
    "less",
    "equals",
    "less-or-equal",
    "exists",
    "missing",
    "greater",
    "contains",
    "matches",
    "greater-or-equal",
    "ends-with",
    "starts-with",
];

/// Policy event vocabulary.
const POLICY_EVENTS: &[&str] = &[
    "websocket-request",
    "response",
    "client-accepted",
    "server-connected",
    "ssl-client-hello",
    "proxy-response",
    "ssl-server-hello",
    "proxy-request",
    "request",
    "websocket-response",
];

/// Policy action target vocabulary.
const POLICY_ACTION_TARGETS: &[&str] = &[
    "http-cookie",
    "shutdown",
    "http-header",
    "http-uri",
    "http-host",
    "tcp",
    "http-reply",
    "forward",
    "log",
];

/// Policy action verb vocabulary.
const POLICY_ACTION_VERBS: &[&str] = &[
    "redirect", "enable", "reset", "insert", "rewrite", "disable", "drop", "select", "replace",
    "remove",
];

/// NTP restrict flag keys.
const NTP_RESTRICT_FLAG_KEYS: &[&str] = &[
    "ignore",
    "kod",
    "limited",
    "low-priority-trap",
    "no-modify",
    "no-peer",
    "no-query",
    "no-serve-packets",
    "no-trap",
    "no-trust",
];

// pool

/// Parse a pool member's `{ ... }` body, returning props + absolute field
/// spans.
fn parse_pool_member_body_with_spans(
    braced_body: &str,
    base_offset: usize,
) -> (HashMap<String, String>, HashMap<String, (usize, usize)>) {
    let mut inner = braced_body;
    let mut inner_start = 0;
    if let Some(stripped) = inner.strip_prefix('{') {
        inner = stripped;
        inner_start = 1;
    }
    if let Some(stripped) = inner.strip_suffix('}') {
        inner = stripped;
    }
    let props_with_spans = parse_properties_with_spans(inner);
    let mut values = HashMap::new();
    let mut spans = HashMap::new();
    for (key, prop) in &props_with_spans {
        values.insert(key.clone(), prop.value.clone());
        if let (Some(vs), Some(ve)) = (prop.value_start, prop.value_end) {
            spans.insert(
                key.clone(),
                (
                    base_offset + inner_start + vs,
                    base_offset + inner_start + ve,
                ),
            );
        }
    }
    (values, spans)
}

/// Extract pool members from a `members { ... }` block.
fn parse_pool_members(braced: &str, base_offset: usize) -> Vec<BigipPoolMember> {
    let mut members = Vec::new();
    let mut inner = braced;
    let mut inner_start_in_braced = 0;
    if let Some(stripped) = inner.strip_prefix('{') {
        inner = stripped;
        inner_start_in_braced = 1;
    }
    if let Some(stripped) = inner.strip_suffix('}') {
        inner = stripped;
    }
    let bytes = inner.as_bytes();
    let length = bytes.len();
    let mut pos = 0;
    while pos < length {
        while pos < length && matches!(bytes[pos], b' ' | b'\t' | b'\n' | b'\r') {
            pos += 1;
        }
        if pos >= length {
            break;
        }
        let name_start = pos;
        while pos < length && !matches!(bytes[pos], b' ' | b'\t' | b'\n' | b'\r' | b'{') {
            pos += 1;
        }
        let name = inner[name_start..pos].to_owned();
        if name.is_empty() {
            pos += 1;
            continue;
        }
        while pos < length && matches!(bytes[pos], b' ' | b'\t') {
            pos += 1;
        }
        let mut body_text = "";
        let mut body_abs_offset = 0;
        if pos < length && bytes[pos] == b'{' {
            let body_start = pos;
            pos += 1;
            let mut depth = 1;
            while pos < length && depth > 0 {
                match bytes[pos] {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    _ => {}
                }
                pos += 1;
            }
            body_text = &inner[body_start..pos];
            body_abs_offset = base_offset + inner_start_in_braced + body_start;
        }
        let (props, field_offsets) = if body_text.is_empty() {
            (HashMap::new(), HashMap::new())
        } else {
            parse_pool_member_body_with_spans(body_text, body_abs_offset)
        };
        let addr = props.get("address").cloned().unwrap_or_default();
        let port = match read_address_property("ltm", "pool", "members", &name) {
            Some(ConfigurationAddress::Member(key)) => i64::from(key.port.port),
            _ => props
                .get("port")
                .and_then(|value| crate::value::Port::try_parse_service(value))
                .map_or(0, |port| i64::from(port.port)),
        };
        let addr_typed =
            match crate::value::read_address_field("ltm", "pool", &["members", "address"], &addr) {
                Some(ConfigurationAddress::Address(address)) => Some(address),
                _ => None,
            };
        members.push(BigipPoolMember {
            name,
            address: addr_typed,
            port,
            monitor: props.get("monitor").cloned().unwrap_or_default(),
            description: unquote(props.get("description").map_or("", String::as_str)).to_owned(),
            state: state_flag(&props),
            ratio: props.get("ratio").cloned().unwrap_or_default(),
            priority_group: props.get("priority-group").cloned().unwrap_or_default(),
            connection_limit: props.get("connection-limit").cloned().unwrap_or_default(),
            rate_limit: props.get("rate-limit").cloned().unwrap_or_default(),
            field_offsets,
        });
    }
    members
}

/// Parse an `ltm pool` block.
#[must_use]
pub fn parse_pool(full_path: &str, body: &str, range: Range, ctx: BespokeCtx) -> BigipPool {
    let mut obj = parse_bigip_pool(full_path, body, range);
    let props_with_spans = parse_properties_with_spans(body);
    let members_prop = props_with_spans.iter().find(|(k, _)| k == "members");
    let mut members: Vec<BigipPoolMember> = Vec::new();
    if let Some((_, prop)) = members_prop
        && !prop.value.is_empty()
    {
        let members_abs = ctx.block_start + 1 + prop.value_start.unwrap_or(0);
        members = parse_pool_members(&prop.value, members_abs);
        // `field_offsets` are absolute source offsets; the canonical form
        // stores them as code-point indices, so convert the
        // byte offsets to code points for non-ASCII fidelity.
        for m in &mut members {
            for span in m.field_offsets.values_mut() {
                *span = (
                    byte_to_cp(ctx.source, span.0),
                    byte_to_cp(ctx.source, span.1),
                );
            }
        }
    }
    obj.members = BigipList {
        items: members
            .into_iter()
            .map(|m| {
                // The typed member carries address / port / field_offsets;
                // the item key is the member name. Per-item lexical spans
                // stay at their defaults.
                let key = m.name.clone();
                let mut item = ListItem::new(ListItemValue::PoolMember(m));
                item.key = key;
                item
            })
            .collect(),
        syntax: ListSyntax::KeyedBlock,
        ..Default::default()
    };
    obj
}

// snatpool

/// Parse an `ltm snatpool` block.
#[must_use]
pub fn parse_snatpool(full_path: &str, body: &str, range: Range) -> BigipSnatPool {
    let mut obj = parse_bigip_snat_pool(full_path, body, range);
    let props = props_map(body);
    let members = props
        .get("members")
        .filter(|m| !m.is_empty())
        .map_or_default(|m| parse_list_block(m));
    obj.members = BigipList {
        items: members.into_iter().map(ListItem::new).collect(),
        syntax: ListSyntax::BracedSpaceSeparated,
        ..Default::default()
    };
    obj
}

// virtual

/// Re-glue a `(key, body)` pair into the per-item source the inner spec's
/// parse consumes (`"<key> { <body> }"`).
fn keyed_item_text(key: &str, body: &str) -> String {
    let body = body.trim();
    if body.is_empty() {
        format!("{key} {{ }}")
    } else {
        format!("{key} {{ {body} }}")
    }
}

/// Build a keyed-block attachment list (`profiles` / `persist`). The
/// closure maps `(path, body)` to the typed list item value.
///
/// Each item's lexical spans (`raw` / `body` / `range` / `key_range` /
/// `body_range`) follow the keyed-block parse: offsets are relative to
/// `block` (the property value, `base_offset == 0`), `raw` is the re-glued
/// `keyed_item_text`, and `body_range` is `None` when the body span is
/// empty.
fn keyed_attachment_list<F>(block: &str, make: F) -> BigipList
where
    F: Fn(&str, &str) -> ListItemValue,
{
    let entries = parse_keyed_block_entries_with_offsets(block);
    BigipList {
        items: entries
            .into_iter()
            .map(
                |(name, body, key_start, key_end, body_start, body_end, item_start, item_end)| {
                    let value = make(&name, &body);
                    let mut item = ListItem::new(value);
                    item.raw = keyed_item_text(&name, &body);
                    item.body = body;
                    item.range = Some(SourceSpan::new(item_start, item_end));
                    item.key_range = Some(SourceSpan::new(key_start, key_end));
                    item.body_range = if body_end > body_start {
                        Some(SourceSpan::new(body_start, body_end))
                    } else {
                        None
                    };
                    item.key = name;
                    item
                },
            )
            .collect(),
        syntax: ListSyntax::KeyedBlock,
        raw: block.trim().to_owned(),
        ..Default::default()
    }
}

/// Parse an `ltm virtual` block.
#[must_use]
pub fn parse_virtual(
    full_path: &str,
    body: &str,
    range: Range,
    ctx: BespokeCtx,
) -> BigipVirtualServer {
    let known = property_names_for("ltm", "virtual");
    let props = split_compact_props(body, &known);
    let props_with_spans = parse_properties_with_spans(body);

    // Start from the generated scalar parser, then override compact-aware
    // scalars + every structured field.
    let mut obj = parse_bigip_virtual_server(full_path, body, range);

    obj.pool = props.get("pool").cloned().unwrap_or_default();
    obj.snatpool = props.get("snatpool").cloned().unwrap_or_default();

    // pool_range: first scalar token span of the pool property value.
    obj.pool_range = None;
    if let Some((_, pool_prop)) = props_with_spans.iter().find(|(k, _)| k == "pool")
        && let (Some(vs), Some(ve)) = (pool_prop.value_start, pool_prop.value_end)
    {
        let raw_value = &body[vs..ve];
        if let Some((ts, te)) = first_scalar_token_span(raw_value) {
            let body_base = ctx.block_start + 1;
            let abs_start = body_base + vs + ts;
            let abs_end = body_base + vs + te;
            obj.pool_range = Some(range_from_token_offsets(ctx, abs_start, abs_end));
        }
    }

    obj.destination = match read_address_property(
        "ltm",
        "virtual",
        "destination",
        props.get("destination").map_or("", String::as_str),
    ) {
        Some(ConfigurationAddress::Destination(destination)) => Some(destination),
        _ => None,
    };

    // rules / policies / vlans: braced space-separated.
    obj.rules = braced_space_list(props.get("rules"));
    obj.policies = braced_space_list(props.get("policies"));
    obj.vlans = braced_space_list(props.get("vlans"));

    // profiles: keyed-block of ProfileAttachment.
    obj.profiles = props
        .get("profiles")
        .filter(|b| !b.is_empty())
        .map_or_default(|block| {
            keyed_attachment_list(block, |path, b| {
                ListItemValue::Profile(ProfileAttachment::from_raw(path, strip_outer_braces(b)))
            })
        });

    // persist: keyed-block of PersistenceAttachment.
    obj.persist = props
        .get("persist")
        .filter(|b| !b.is_empty())
        .map_or_default(|block| {
            keyed_attachment_list(block, |path, b| {
                ListItemValue::Persistence(PersistenceAttachment::from_raw(
                    path,
                    strip_outer_braces(b),
                ))
            })
        });

    // source-address-translation: parse the inner block for type / pool.
    obj.source_address_translation = String::new();
    if let Some(sat_block) = props.get("source-address-translation")
        && !sat_block.is_empty()
    {
        let sat_props: HashMap<String, String> = parse_properties(strip_outer_braces(sat_block))
            .into_iter()
            .collect();
        obj.source_address_translation = sat_props.get("type").cloned().unwrap_or_default();
        if obj.snatpool.is_empty() {
            obj.snatpool = sat_props.get("pool").cloned().unwrap_or_default();
        }
    }

    // Compact-aware scalar / bare-flag overrides (registry-split props).
    obj.description = description(&props);
    obj.mask = props.get("mask").cloned().unwrap_or_default();
    obj.source = props.get("source").cloned().unwrap_or_default();
    obj.ip_protocol = props.get("ip-protocol").cloned().unwrap_or_default();
    obj.state = state_flag(&props);
    obj.dhcp_relay = props.contains_key("dhcp-relay");
    obj.internal = props.contains_key("internal");
    obj.ip_forward = props.contains_key("ip-forward");
    obj.l2_forward = props.contains_key("l2-forward");
    obj.reject = props.contains_key("reject");
    obj.vlans_disabled = props.contains_key("vlans-disabled");
    obj.vlans_enabled = props.contains_key("vlans-enabled");

    // auth / traffic-classes / clone-pools tuples.
    obj.auth_profiles = list_from(props.get("auth"));
    obj.traffic_classes = list_from(props.get("traffic-classes"));
    obj.clone_pools = list_from(props.get("clone-pools"));

    obj
}

/// A `braced-space-separated` `BigipList` from an optional block value.
fn braced_space_list(block: Option<&String>) -> BigipList {
    let items = block
        .filter(|b| !b.is_empty())
        .map_or_default(|b| parse_list_block(b));
    BigipList {
        items: items.into_iter().map(ListItem::new).collect(),
        syntax: ListSyntax::BracedSpaceSeparated,
        ..Default::default()
    }
}

/// A `Vec<String>` from an optional brace-list value.
fn list_from(block: Option<&String>) -> Vec<String> {
    block
        .filter(|b| !b.is_empty())
        .map_or_default(|b| parse_list_block(b))
}

// node / virtual-address

/// Parse an `ltm node` block.
#[must_use]
pub fn parse_node(full_path: &str, body: &str, range: Range) -> BigipNode {
    let mut obj = parse_bigip_node(full_path, body, range);
    let props = props_map(body);
    let mut fqdn_raw = String::new();
    if let Some(fqdn_block) = props.get("fqdn")
        && fqdn_block.starts_with('{')
    {
        let fqdn_props: HashMap<String, String> = parse_properties(strip_outer_braces(fqdn_block))
            .into_iter()
            .collect();
        fqdn_raw = fqdn_props.get("name").cloned().unwrap_or_default();
    }
    let addr_raw = props.get("address").cloned().unwrap_or_default();
    obj.address = match read_address_property("ltm", "node", "address", &addr_raw) {
        Some(ConfigurationAddress::Address(address)) => Some(address),
        _ => None,
    };
    obj.fqdn = match crate::value::read_address_field("ltm", "node", &["fqdn", "name"], &fqdn_raw) {
        Some(ConfigurationAddress::Fqdn(fqdn)) => Some(fqdn),
        _ => None,
    };
    obj
}

/// Parse an `ltm virtual-address` block.
#[must_use]
pub fn parse_virtual_address(full_path: &str, body: &str, range: Range) -> BigipVirtualAddress {
    let mut obj = parse_bigip_virtual_address(full_path, body, range);
    let known = property_names_for("ltm", "virtual-address");
    let props = split_compact_props(body, &known);
    obj.address = match read_address_property(
        "ltm",
        "virtual-address",
        "address",
        props.get("address").map_or("", String::as_str),
    ) {
        Some(ConfigurationAddress::Address(crate::value::Address::Ip(address))) => Some(address),
        Some(ConfigurationAddress::Address(crate::value::Address::Wildcard(wildcard))) => {
            wildcard.unspecified_ip()
        }
        _ => None,
    };
    // Compact-aware scalar overrides.
    obj.mask = props.get("mask").cloned().unwrap_or_default();
    obj
}

// monitor / profile / persistence

/// Parse an `ltm/gtm monitor` block. `monitor_type` is the sub-type after
/// `monitor `.
#[must_use]
pub fn parse_monitor(
    full_path: &str,
    body: &str,
    monitor_type: &str,
    range: Range,
) -> BigipMonitor {
    let mut obj = parse_bigip_monitor(full_path, body, range);
    let props = props_map(body);
    monitor_type.clone_into(&mut obj.monitor_type);
    // These quoted text fields are unquoted.
    let uq = |k: &str| unquote(props.get(k).map_or("", String::as_str)).to_owned();
    obj.send = uq("send");
    obj.recv = uq("recv");
    obj.recv_disable = uq("recv-disable");
    obj.base = uq("base");
    obj.filter = uq("filter");
    obj.args = uq("args");
    obj.headers = uq("headers");
    obj.request = uq("request");
    obj.response = uq("response");
    obj
}

/// Parse an `ltm profile <type>` block. `profile_type_str` is the sub-type
/// after `profile `.
#[must_use]
pub fn parse_profile(
    full_path: &str,
    profile_type_str: &str,
    body: &str,
    range: Range,
) -> BigipProfile {
    let mut obj = parse_bigip_profile(full_path, body, range);
    let props = props_map(body);
    obj.profile_type = classify_profile(profile_type_str);
    let uq = |k: &str| unquote(props.get(k).map_or("", String::as_str)).to_owned();
    obj.lws_separator = uq("lws-separator");
    obj.server_agent_name = uq("server-agent-name");
    obj.ciphers = uq("ciphers");
    obj.cert_extension_includes = uq("cert-extension-includes");
    obj.options = uq("options");
    obj.server_name = uq("server-name");
    obj.source = uq("source");
    obj.target = uq("target");
    obj
}

/// Parse an `ltm persistence <type>` block. `persistence_type` is the
/// sub-type after `persistence `.
#[must_use]
pub fn parse_persistence(
    full_path: &str,
    persistence_type: &str,
    body: &str,
    range: Range,
) -> BigipPersistence {
    let mut obj = parse_bigip_persistence(full_path, body, range);
    let props = props_map(body);
    persistence_type.clone_into(&mut obj.persistence_type);
    unquote(props.get("cookie-name").map_or("", String::as_str)).clone_into(&mut obj.cookie_name);
    obj
}

// data-group

/// Parse an `ltm data-group internal|external` block. `kind` comes from the
/// header.
#[must_use]
pub fn parse_data_group(
    full_path: &str,
    body: &str,
    kind: DataGroupType,
    range: Range,
) -> BigipDataGroup {
    let mut obj = parse_bigip_data_group(full_path, body, range);
    let props = props_map(body);
    obj.kind = kind;
    obj.value_type = props.get("type").cloned().unwrap_or_default();
    obj.records = props
        .get("records")
        .filter(|r| !r.is_empty())
        .map_or_default(|r| parse_list_block(r));
    obj
}

// policy

/// Parse a single `conditions { N { ... } }` body.
fn parse_policy_condition(index: i64, body: &str) -> BigipPolicyCondition {
    let props = parse_properties(body);
    let mut operand = String::new();
    let mut selector = String::new();
    let mut operator = "equals".to_owned();
    let mut negate = false;
    let mut case_insensitive = false;
    let mut event = String::new();
    let mut name = String::new();
    let mut values: Vec<String> = Vec::new();
    for (key, value) in &props {
        if !value.is_empty() {
            if key == "values" {
                values = parse_policy_values_block(value);
            } else if key == "name" || key == "tm-name" {
                name = strip_quotes(value);
            }
            continue;
        }
        if POLICY_OPERANDS.contains(&key.as_str()) && operand.is_empty() {
            operand.clone_from(key);
        } else if POLICY_SELECTORS.contains(&key.as_str()) && selector.is_empty() {
            selector.clone_from(key);
        } else if POLICY_OPERATORS.contains(&key.as_str()) {
            operator.clone_from(key);
        } else if POLICY_EVENTS.contains(&key.as_str()) {
            event.clone_from(key);
        } else if key == "not" {
            negate = true;
        } else if key == "case-insensitive" {
            case_insensitive = true;
        }
    }
    BigipPolicyCondition {
        index,
        operand,
        selector,
        operator,
        values,
        name,
        negate,
        case_insensitive,
        event,
    }
}

/// Parse a single `actions { N { ... } }` body.
fn parse_policy_action(index: i64, body: &str) -> BigipPolicyAction {
    let props = parse_properties(body);
    let mut target = String::new();
    let mut verb = String::new();
    let mut pool = String::new();
    let mut location = String::new();
    let mut name = String::new();
    let mut value = String::new();
    let mut path = String::new();
    let mut query = String::new();
    let mut host = String::new();
    let mut event = String::new();
    for (key, val) in &props {
        if !val.is_empty() {
            match key.as_str() {
                "pool" => val.trim().clone_into(&mut pool),
                "location" => location = strip_quotes(val),
                "name" | "tm-name" => name = strip_quotes(val),
                "value" => value = strip_quotes(val),
                "path" => path = strip_quotes(val),
                "query" => query = strip_quotes(val),
                "host" => host = strip_quotes(val),
                _ => {}
            }
            continue;
        }
        if POLICY_ACTION_TARGETS.contains(&key.as_str()) && target.is_empty() {
            target.clone_from(key);
        } else if POLICY_ACTION_VERBS.contains(&key.as_str()) && verb.is_empty() {
            verb.clone_from(key);
        } else if POLICY_EVENTS.contains(&key.as_str()) {
            event.clone_from(key);
        }
    }
    BigipPolicyAction {
        index,
        target,
        verb,
        pool,
        location,
        name,
        value,
        path,
        query,
        host,
        event,
    }
}

/// Parse a single `rules { name { ... } }` body.
fn parse_policy_rule(name: &str, body: &str) -> BigipPolicyRule {
    let props_with_spans = parse_properties_with_spans(body);
    let props: HashMap<&str, &Property> = props_with_spans
        .iter()
        .map(|(k, p)| (k.as_str(), p))
        .collect();
    let ordinal = props
        .get("ordinal")
        .map_or(0, |p| p.value.trim().parse().unwrap_or(0));

    let mut conditions: Vec<BigipPolicyCondition> = Vec::new();
    if let Some(cond_prop) = props.get("conditions")
        && !cond_prop.value.is_empty()
    {
        for (idx_str, prop) in parse_properties_with_spans(strip_outer_braces(&cond_prop.value)) {
            if let Ok(idx) = idx_str.parse::<i64>() {
                conditions.push(parse_policy_condition(idx, strip_outer_braces(&prop.value)));
            }
        }
    }

    let mut actions: Vec<BigipPolicyAction> = Vec::new();
    if let Some(act_prop) = props.get("actions")
        && !act_prop.value.is_empty()
    {
        for (idx_str, prop) in parse_properties_with_spans(strip_outer_braces(&act_prop.value)) {
            if let Ok(idx) = idx_str.parse::<i64>() {
                actions.push(parse_policy_action(idx, strip_outer_braces(&prop.value)));
            }
        }
    }

    conditions.sort_by_key(|c| c.index);
    actions.sort_by_key(|a| a.index);
    BigipPolicyRule {
        name: name.to_owned(),
        ordinal,
        conditions,
        actions,
    }
}

/// Parse an `ltm policy` block.
#[must_use]
pub fn parse_policy(full_path: &str, body: &str, range: Range) -> BigipPolicy {
    let mut obj = parse_bigip_policy(full_path, body, range);
    let props = props_map(body);

    let mut strategy = props.get("strategy").cloned().unwrap_or_default();
    if strategy.trim().is_empty() {
        "first-match".clone_into(&mut strategy);
    }
    let strategy = strategy.trim();
    strategy
        .rsplit('/')
        .next()
        .unwrap_or(strategy)
        .clone_into(&mut obj.strategy);

    obj.requires = list_from(props.get("requires"));
    obj.controls = list_from(props.get("controls"));

    let mut rules: Vec<BigipPolicyRule> = Vec::new();
    if let Some(rules_block) = props.get("rules")
        && !rules_block.is_empty()
    {
        for (rule_name, prop) in parse_properties_with_spans(strip_outer_braces(rules_block)) {
            rules.push(parse_policy_rule(
                &rule_name,
                strip_outer_braces(&prop.value),
            ));
        }
    }
    rules.sort_by(|a, b| (a.ordinal, &a.name).cmp(&(b.ordinal, &b.name)));
    obj.rules = rules;
    obj
}

// rule

/// Parse an `ltm rule` block.
#[must_use]
pub fn parse_rule(full_path: &str, body: &str, range: Range) -> BigipRule {
    let mut obj = parse_bigip_rule(full_path, body, range);
    body.trim().clone_into(&mut obj.source);
    obj.description = String::new();
    obj
}

// gtm pool / wideip / topology

/// Parse a `gtm pool <record-type>` block. `record_type` is the sub-type
/// after `pool `.
#[must_use]
pub fn parse_gtm_pool(
    full_path: &str,
    body: &str,
    record_type: &str,
    range: Range,
) -> BigipGtmPool {
    let mut obj = parse_bigip_gtm_pool(full_path, body, range);
    let props = props_map(body);
    record_type.clone_into(&mut obj.record_type);
    let mut members: Vec<BigipGtmPoolMember> = Vec::new();
    if let Some(members_block) = props.get("members") {
        for (entry_name, entry_body) in parse_keyed_block_entries(members_block) {
            let ep = props_map(&entry_body);
            members.push(BigipGtmPoolMember {
                name: entry_name,
                description: description(&ep),
                state: state_flag(&ep),
                member_order: ep.get("member-order").cloned().unwrap_or_default(),
                order: ep.get("order").cloned().unwrap_or_default(),
                service_port: ep.get("service-port").cloned().unwrap_or_default(),
                ratio: ep.get("ratio").cloned().unwrap_or_default(),
                monitor: ep.get("monitor").cloned().unwrap_or_default(),
                depends_on: ep.get("depends-on").cloned().unwrap_or_default(),
                limit_max_bps: ep.get("limit-max-bps").cloned().unwrap_or_default(),
                limit_max_connections: ep.get("limit-max-connections").cloned().unwrap_or_default(),
                limit_max_pps: ep.get("limit-max-pps").cloned().unwrap_or_default(),
                static_target: ep.get("static-target").cloned().unwrap_or_default(),
            });
        }
    }
    obj.members = members;
    // Prefer ``qos-kilobytes-second`` over ``qos-kbps``.
    let kbps = props
        .get("qos-kilobytes-second")
        .filter(|v| !v.is_empty())
        .or_else(|| props.get("qos-kbps"))
        .cloned()
        .unwrap_or_default();
    obj.qos_kbps = kbps;
    obj
}

/// Parse a `gtm wideip <record-type>` block. `record_type` is the sub-type
/// after `wideip `.
#[must_use]
pub fn parse_gtm_wideip(
    full_path: &str,
    body: &str,
    record_type: &str,
    range: Range,
) -> BigipGtmWideip {
    let mut obj = parse_bigip_gtm_wideip(full_path, body, range);
    let props = props_map(body);
    record_type.clone_into(&mut obj.record_type);
    obj.pools = list_from(props.get("pools"));
    obj.aliases = list_from(props.get("aliases"));
    // ``last-resort-pool`` is emitted with the record-type prefix.
    obj.last_resort_pool = String::new();
    if let Some(raw) = props.get("last-resort-pool") {
        let mut parts = raw.splitn(2, char::is_whitespace);
        let first = parts.next().unwrap_or("");
        match parts.next() {
            Some(rest) if !first.is_empty() => {
                rest.trim_start().clone_into(&mut obj.last_resort_pool);
            }
            _ => obj.last_resort_pool.clone_from(raw),
        }
    }
    obj
}

/// Parse a `gtm topology` stanza. `identifier` is the multi-token condition
/// from the header.
#[must_use]
pub fn parse_gtm_topology(identifier: &str, body: &str, range: Range) -> BigipGtmTopology {
    let props = props_map(body);
    BigipGtmTopology {
        name: identifier.to_owned(),
        full_path: identifier.to_owned(),
        description: description(&props),
        order: props.get("order").cloned().unwrap_or_default(),
        score: props.get("score").cloned().unwrap_or_default(),
        range: Some(range),
    }
}

// sys ntp / snmp

/// Parse a `sys ntp` block.
#[must_use]
pub fn parse_sys_ntp(full_path: &str, body: &str, range: Range) -> BigipSysNtp {
    let mut obj = parse_bigip_sys_ntp(full_path, body, range);
    let props_with_spans = parse_properties_with_spans(body);
    let lookup = |key: &str| {
        props_with_spans
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, p)| p)
    };
    obj.servers = lookup("servers").map_or_else(Vec::new, |p| parse_list_block(&p.value));
    obj.timezone = lookup("timezone").map_or_else(String::new, |p| p.value.clone());
    let mut restrict: Vec<BigipSysNtpRestrict> = Vec::new();
    if let Some(p) = lookup("restrict") {
        for (entry_name, entry_body) in parse_keyed_block_entries(&p.value) {
            let ep = props_map(&entry_body);
            let flags = NTP_RESTRICT_FLAG_KEYS
                .iter()
                .filter(|f| ep.get(**f).map(String::as_str) == Some("enabled"))
                .map(|f| (*f).to_owned())
                .collect();
            restrict.push(BigipSysNtpRestrict {
                name: entry_name,
                address: ep.get("address").cloned().unwrap_or_default(),
                mask: ep.get("mask").cloned().unwrap_or_default(),
                default_entry: ep.get("default-entry").cloned().unwrap_or_default(),
                flags,
                description: description(&ep),
            });
        }
    }
    obj.restrict = restrict;
    obj
}

/// Parse a `sys snmp` block.
#[must_use]
pub fn parse_sys_snmp(full_path: &str, body: &str, range: Range) -> BigipSysSnmp {
    let mut obj = parse_bigip_sys_snmp(full_path, body, range);
    let props_with_spans = parse_properties_with_spans(body);
    let plain: HashMap<String, String> = props_with_spans
        .iter()
        .map(|(k, p)| (k.clone(), p.value.clone()))
        .collect();
    let lookup = |key: &str| {
        props_with_spans
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, p)| p)
    };

    obj.agent_addresses =
        lookup("agent-addresses").map_or_else(Vec::new, |p| parse_list_block(&p.value));
    obj.communities = lookup("communities").map_or_else(Vec::new, |p| {
        parse_keyed_block_entries(&p.value)
            .into_iter()
            .map(|(k, _)| k)
            .collect()
    });
    unquote(plain.get("sys-contact").map_or("", String::as_str)).clone_into(&mut obj.sys_contact);
    unquote(plain.get("sys-location").map_or("", String::as_str)).clone_into(&mut obj.sys_location);
    obj.sys_services = plain.get("sys-services").cloned().unwrap_or_default();
    obj.trap_community = plain.get("trap-community").cloned().unwrap_or_default();

    if let Some(p) = lookup("users") {
        obj.users = parse_keyed_block_entries(&p.value)
            .into_iter()
            .map(|(name, b)| {
                let ep = props_map(&b);
                BigipSysSnmpUser {
                    name,
                    username: ep.get("username").cloned().unwrap_or_default(),
                    security_level: ep.get("security-level").cloned().unwrap_or_default(),
                    auth_protocol: ep.get("auth-protocol").cloned().unwrap_or_default(),
                    privacy_protocol: ep.get("privacy-protocol").cloned().unwrap_or_default(),
                    oid_subset: ep.get("oid-subset").cloned().unwrap_or_default(),
                    description: description(&ep),
                }
            })
            .collect();
    }
    if let Some(p) = lookup("traps") {
        obj.traps = parse_keyed_block_entries(&p.value)
            .into_iter()
            .map(|(name, b)| {
                let ep = props_map(&b);
                BigipSysSnmpTrap {
                    name,
                    host: ep.get("host").cloned().unwrap_or_default(),
                    port: ep.get("port").cloned().unwrap_or_default(),
                    version: ep.get("version").cloned().unwrap_or_default(),
                    community: ep.get("community").cloned().unwrap_or_default(),
                    security_name: ep.get("security-name").cloned().unwrap_or_default(),
                    security_level: ep.get("security-level").cloned().unwrap_or_default(),
                    auth_protocol: ep.get("auth-protocol").cloned().unwrap_or_default(),
                    privacy_protocol: ep.get("privacy-protocol").cloned().unwrap_or_default(),
                    network: ep.get("network").cloned().unwrap_or_default(),
                    description: description(&ep),
                }
            })
            .collect();
    }
    if let Some(p) = lookup("process-monitors") {
        obj.process_monitors = parse_keyed_block_entries(&p.value)
            .into_iter()
            .map(|(name, b)| {
                let ep = props_map(&b);
                BigipSysSnmpProcessMonitor {
                    name,
                    process: ep.get("process").cloned().unwrap_or_default(),
                    max_processes: ep.get("max-processes").cloned().unwrap_or_default(),
                    min_processes: ep.get("min-processes").cloned().unwrap_or_default(),
                    description: description(&ep),
                }
            })
            .collect();
    }
    if let Some(p) = lookup("disk-monitors") {
        obj.disk_monitors = parse_keyed_block_entries(&p.value)
            .into_iter()
            .map(|(name, b)| {
                let ep = props_map(&b);
                BigipSysSnmpDiskMonitor {
                    name,
                    partition: ep.get("partition").cloned().unwrap_or_default(),
                    min_space: ep.get("min-space").cloned().unwrap_or_default(),
                    description: description(&ep),
                }
            })
            .collect();
    }
    obj
}

// net route / self

/// Parse a `net route` block.
#[must_use]
pub fn parse_net_route(full_path: &str, body: &str, range: Range) -> BigipNetRoute {
    let mut obj = parse_bigip_net_route(full_path, body, range);
    let known = property_names_for("net", "route");
    let props = split_compact_props(body, &known);
    obj.network = match read_address_property(
        "net",
        "route",
        "network",
        props.get("network").map_or("", String::as_str),
    ) {
        Some(ConfigurationAddress::Network(network)) => Some(network),
        _ => None,
    };
    obj.is_default_route = obj.network.as_ref().is_some_and(Network::is_default);
    obj.gw = match read_address_property(
        "net",
        "route",
        "gw",
        props.get("gw").map_or("", String::as_str),
    ) {
        Some(ConfigurationAddress::Address(crate::value::Address::Ip(gateway))) => Some(gateway),
        _ => None,
    };
    obj.pool = props.get("pool").cloned().unwrap_or_default();
    obj.mtu = props.get("mtu").cloned().unwrap_or_default();
    obj.blackhole = props.contains_key("blackhole");
    obj.interface = props.get("interface").cloned().unwrap_or_default();
    obj
}

/// Parse a `net self` block.
#[must_use]
pub fn parse_net_self(full_path: &str, body: &str, range: Range) -> BigipNetSelf {
    let mut obj = parse_bigip_net_self(full_path, body, range);
    let known = property_names_for("net", "self");
    let plain = split_compact_props(body, &known);
    let props_with_spans = parse_properties_with_spans(body);

    let mut allow_service: Vec<String> = Vec::new();
    if let Some((_, p)) = props_with_spans.iter().find(|(k, _)| k == "allow-service") {
        if p.value.starts_with('{') {
            allow_service = parse_list_block(&p.value);
        } else if !p.value.is_empty() {
            allow_service = vec![p.value.clone()];
        }
    }
    obj.allow_service = allow_service;

    obj.address = match read_address_property(
        "net",
        "self",
        "address",
        plain.get("address").map_or("", String::as_str),
    ) {
        Some(ConfigurationAddress::Network(network)) => Some(network),
        _ => None,
    };
    obj.vlan = plain.get("vlan").cloned().unwrap_or_default();
    obj.traffic_group = plain.get("traffic-group").cloned().unwrap_or_default();
    obj
}

// security firewall rule-list

/// Parse a `security firewall rule-list` block.
///
/// Note: the model has no `rule_objects` field, so only the `rules` name
/// tuple is surfaced.
#[must_use]
pub fn parse_security_firewall_rule_list(
    full_path: &str,
    body: &str,
    range: Range,
) -> BigipSecurityFirewallRuleList {
    let mut obj = parse_bigip_security_firewall_rule_list(full_path, body, range);
    let props_with_spans = parse_properties_with_spans(body);
    if let Some((_, p)) = props_with_spans.iter().find(|(k, _)| k == "rules") {
        let entries = parse_keyed_block_entries(&p.value);
        obj.rules = entries.iter().map(|(k, _)| k.clone()).collect();
        obj.rule_objects = entries
            .iter()
            .map(|(name, rbody)| FirewallRule::from_raw(name, rbody))
            .collect();
    }
    obj
}

/// `security firewall policy` — `rules` + `rule_lists` walked out of the
/// nested `rules { ... }` block.
#[must_use]
pub fn parse_security_firewall_policy(
    full_path: &str,
    body: &str,
    range: Range,
) -> BigipSecurityFirewallPolicy {
    let mut obj = parse_bigip_security_firewall_policy(full_path, body, range);
    let (names, refs) = firewall_rules_summary(&props_map(body));
    obj.rules = names;
    obj.rule_lists = refs;
    obj
}

/// Walk a firewall `rules { ... }` block → `(rule_names, rule_list_refs)`.
fn firewall_rules_summary(props: &HashMap<String, String>) -> (Vec<String>, Vec<String>) {
    let raw = props.get("rules").map_or("", String::as_str);
    if raw.is_empty() || !raw.starts_with('{') {
        return (Vec::new(), Vec::new());
    }
    let inner = strip_outer_braces(raw);
    let mut names = Vec::new();
    let mut refs = Vec::new();
    for (key, prop) in parse_properties_with_spans(inner) {
        names.push(key);
        if prop.value.starts_with('{') {
            let sub = parse_properties(strip_outer_braces(&prop.value));
            if let Some((_, v)) = sub.iter().find(|(k, _)| k == "rule-list") {
                refs.push(v.clone());
            }
        }
    }
    (names, refs)
}

/// `gtm server` — `addresses` / `virtual_servers` flattened out of the
/// nested `devices` / `virtual-servers` numbered sub-blocks (the generated
/// scalar parser mis-reads them as flat lists, so override both).
#[must_use]
pub fn parse_gtm_server(full_path: &str, body: &str, range: Range) -> BigipGtmServer {
    let mut obj = parse_bigip_gtm_server(full_path, body, range);
    let props = props_map(body);
    let mut addresses = Vec::new();
    if let Some(devices) = props.get("devices") {
        for (_, dev) in parse_properties_with_spans(strip_outer_braces(devices)) {
            if !dev.value.starts_with('{') {
                continue;
            }
            let dev_props = parse_properties(strip_outer_braces(&dev.value));
            if let Some((_, addrs)) = dev_props.iter().find(|(k, _)| k == "addresses") {
                addresses.extend(parse_list_block(addrs));
            }
        }
    }
    let mut virtual_servers = Vec::new();
    if let Some(vss) = props.get("virtual-servers") {
        for (_, vs) in parse_properties_with_spans(strip_outer_braces(vss)) {
            if !vs.value.starts_with('{') {
                continue;
            }
            let vs_props = parse_properties(strip_outer_braces(&vs.value));
            if let Some((_, dest)) = vs_props.iter().find(|(k, _)| k == "destination") {
                virtual_servers.push(dest.clone());
            }
        }
    }
    obj.addresses = addresses;
    obj.virtual_servers = virtual_servers;
    obj
}

/// `apm policy item` — `caption` is a quoted string whose quotes are
/// stripped. The rest is handled by the scalar parser.
#[must_use]
pub fn parse_apm_policy_item(full_path: &str, body: &str, range: Range) -> BigipApmPolicyItem {
    let mut obj = parse_bigip_apm_policy_item(full_path, body, range);
    let props = props_map(body);
    obj.caption = props
        .get("caption")
        .map_or_else(String::new, |v| strip_quotes(v));
    obj
}

// Per-kind structured-list parsers (cm / gtm / ltm / net / pem / security / sys)

/// Apply `parse_list_block` to a property's value when present, else empty.
fn list_block_of(props: &HashMap<String, String>, key: &str) -> Vec<String> {
    props
        .get(key)
        .filter(|v| !v.is_empty())
        .map_or_default(|v| parse_list_block(v))
}

/// The ordered top-level keys of a braced sub-block value.
fn subblock_keys(value: &str) -> Vec<String> {
    parse_properties_with_spans(strip_outer_braces(value))
        .into_iter()
        .map(|(k, _)| k)
        .collect()
}

/// `apm oauth db-instance` — `purge_time` is a quoted string whose quotes
/// are stripped. The rest is handled by the scalar parser.
#[must_use]
pub fn parse_apm_oauth_db_instance(
    full_path: &str,
    body: &str,
    range: Range,
) -> BigipApmOauthDbInstance {
    let mut obj = parse_bigip_apm_oauth_db_instance(full_path, body, range);
    obj.purge_time = props_map(body)
        .get("purge-time")
        .map_or_else(String::new, |v| strip_quotes(v));
    obj
}

/// `cm device` — `unicast_address` flattened out of anon-or-numbered
/// `ip` sub-blocks; `comment`/`contact`/`location` quote-stripped.
#[must_use]
pub fn parse_cm_device(full_path: &str, body: &str, range: Range) -> BigipCmDevice {
    let mut obj = parse_bigip_cm_device(full_path, body, range);
    let props = props_map(body);
    obj.comment = props
        .get("comment")
        .map_or_else(String::new, |v| strip_quotes(v));
    obj.contact = props
        .get("contact")
        .map_or_else(String::new, |v| strip_quotes(v));
    obj.location = props
        .get("location")
        .map_or_else(String::new, |v| strip_quotes(v));
    obj.unicast_address = props.get("unicast-address").map_or_else(Vec::new, |ua| {
        let anon = collect_named_property_from_anon_subblocks(ua, "ip");
        if anon.is_empty() {
            collect_named_property_from_subblocks(ua, "ip")
        } else {
            anon
        }
    });
    obj
}

/// `cm device-group` — `devices` list.
#[must_use]
pub fn parse_cm_device_group(full_path: &str, body: &str, range: Range) -> BigipCmDeviceGroup {
    let mut obj = parse_bigip_cm_device_group(full_path, body, range);
    obj.devices = list_block_of(&props_map(body), "devices");
    obj
}

/// `cm traffic-group` — `ha_order` list.
#[must_use]
pub fn parse_cm_traffic_group(full_path: &str, body: &str, range: Range) -> BigipCmTrafficGroup {
    let mut obj = parse_bigip_cm_traffic_group(full_path, body, range);
    obj.ha_order = list_block_of(&props_map(body), "ha-order");
    obj
}

/// `cm trust-domain` — `ca_devices` list.
#[must_use]
pub fn parse_cm_trust_domain(full_path: &str, body: &str, range: Range) -> BigipCmTrustDomain {
    let mut obj = parse_bigip_cm_trust_domain(full_path, body, range);
    obj.ca_devices = list_block_of(&props_map(body), "ca-devices");
    obj
}

/// `gtm datacenter` — `contact`/`location` quote-stripped.
#[must_use]
pub fn parse_gtm_datacenter(full_path: &str, body: &str, range: Range) -> BigipGtmDatacenter {
    let mut obj = parse_bigip_gtm_datacenter(full_path, body, range);
    let props = props_map(body);
    obj.contact = props
        .get("contact")
        .map_or_else(String::new, |v| strip_quotes(v));
    obj.location = props
        .get("location")
        .map_or_else(String::new, |v| strip_quotes(v));
    obj
}

/// `gtm prober-pool` — `members` list.
#[must_use]
pub fn parse_gtm_prober_pool(full_path: &str, body: &str, range: Range) -> BigipGtmProberPool {
    let mut obj = parse_bigip_gtm_prober_pool(full_path, body, range);
    obj.members = list_block_of(&props_map(body), "members");
    obj
}

/// `gtm rule` — Tcl body stored verbatim in `source`.
#[must_use]
pub fn parse_gtm_rule(full_path: &str, body: &str, range: Range) -> BigipGtmRule {
    let mut obj = parse_bigip_gtm_rule(full_path, body, range);
    body.trim().clone_into(&mut obj.source);
    obj
}

/// `ltm dns cache resolver` — `forward_zones` keys.
#[must_use]
pub fn parse_ltm_dns_cache_resolver(
    full_path: &str,
    body: &str,
    range: Range,
) -> BigipLtmDnsCacheResolver {
    let mut obj = parse_bigip_ltm_dns_cache_resolver(full_path, body, range);
    let props = props_map(body);
    obj.forward_zones = props
        .get("forward-zones")
        .filter(|raw| raw.starts_with('{'))
        .map_or_else(Vec::new, |raw| subblock_keys(raw));
    obj
}

/// `net dns-resolver` — `forward_zones` + `nameservers` lists.
#[must_use]
pub fn parse_net_dns_resolver(full_path: &str, body: &str, range: Range) -> BigipNetDnsResolver {
    let mut obj = parse_bigip_net_dns_resolver(full_path, body, range);
    let props = props_map(body);
    obj.forward_zones = list_block_of(&props, "forward-zones");
    obj.nameservers = list_block_of(&props, "nameservers");
    obj
}

/// `net interface` — `name = full_path` (bare slot/port token).
#[must_use]
pub fn parse_net_interface(full_path: &str, body: &str, range: Range) -> BigipNetInterface {
    let mut obj = parse_bigip_net_interface(full_path, body, range);
    full_path.clone_into(&mut obj.name);
    obj
}

/// `net port-list` — `ports` list.
#[must_use]
pub fn parse_net_port_list(full_path: &str, body: &str, range: Range) -> BigipNetPortList {
    let mut obj = parse_bigip_net_port_list(full_path, body, range);
    obj.ports = list_block_of(&props_map(body), "ports");
    obj
}

/// `net route-domain` — `vlans` + `routing_protocol` lists.
#[must_use]
pub fn parse_net_route_domain(full_path: &str, body: &str, range: Range) -> BigipNetRouteDomain {
    let mut obj = parse_bigip_net_route_domain(full_path, body, range);
    let props = props_map(body);
    obj.vlans = list_block_of(&props, "vlans");
    obj.routing_protocol = list_block_of(&props, "routing-protocol");
    obj
}

/// `net stp` — `interfaces` + `vlans` lists.
#[must_use]
pub fn parse_net_stp(full_path: &str, body: &str, range: Range) -> BigipNetStp {
    let mut obj = parse_bigip_net_stp(full_path, body, range);
    let props = props_map(body);
    obj.interfaces = list_block_of(&props, "interfaces");
    obj.vlans = list_block_of(&props, "vlans");
    obj
}

/// `net vlan` — `interfaces` list.
#[must_use]
pub fn parse_net_vlan(full_path: &str, body: &str, range: Range) -> BigipNetVlan {
    let mut obj = parse_bigip_net_vlan(full_path, body, range);
    obj.interfaces = list_block_of(&props_map(body), "interfaces");
    obj
}

/// `pem listener` — `virtual_servers` list.
#[must_use]
pub fn parse_pem_listener(full_path: &str, body: &str, range: Range) -> BigipPemListener {
    let mut obj = parse_bigip_pem_listener(full_path, body, range);
    obj.virtual_servers = list_block_of(&props_map(body), "virtual-servers");
    obj
}

/// `pem policy` — `rules` sub-block keys.
#[must_use]
pub fn parse_pem_policy(full_path: &str, body: &str, range: Range) -> BigipPemPolicy {
    let mut obj = parse_bigip_pem_policy(full_path, body, range);
    obj.rules = props_map(body)
        .get("rules")
        .map_or_else(Vec::new, |v| subblock_keys(v));
    obj
}

/// `pem service-chain-endpoint` — `service_endpoints` sub-block keys.
#[must_use]
pub fn parse_pem_service_chain_endpoint(
    full_path: &str,
    body: &str,
    range: Range,
) -> BigipPemServiceChainEndpoint {
    let mut obj = parse_bigip_pem_service_chain_endpoint(full_path, body, range);
    obj.service_endpoints = props_map(body)
        .get("service-endpoints")
        .map_or_else(Vec::new, |v| subblock_keys(v));
    obj
}

/// `security firewall port-list` — `ports` list.
#[must_use]
pub fn parse_security_firewall_port_list(
    full_path: &str,
    body: &str,
    range: Range,
) -> BigipSecurityFirewallPortList {
    let mut obj = parse_bigip_security_firewall_port_list(full_path, body, range);
    obj.ports = list_block_of(&props_map(body), "ports");
    obj
}

/// `security log profile` — `application_data` from the `application`
/// key.
#[must_use]
pub fn parse_security_log_profile(
    full_path: &str,
    body: &str,
    range: Range,
) -> BigipSecurityLogProfile {
    let mut obj = parse_bigip_security_log_profile(full_path, body, range);
    obj.application_data = props_map(body)
        .get("application")
        .cloned()
        .unwrap_or_default();
    obj
}

/// `security nat policy` — firewall `rules` + `rule_lists` summary.
#[must_use]
pub fn parse_security_nat_policy(
    full_path: &str,
    body: &str,
    range: Range,
) -> BigipSecurityNatPolicy {
    let mut obj = parse_bigip_security_nat_policy(full_path, body, range);
    let (names, refs) = firewall_rules_summary(&props_map(body));
    obj.rules = names;
    obj.rule_lists = refs;
    obj
}

/// `security packet-filter policy` — firewall `rules` summary (no
/// rule-lists surfaced).
#[must_use]
pub fn parse_security_packet_filter_policy(
    full_path: &str,
    body: &str,
    range: Range,
) -> BigipSecurityPacketFilterPolicy {
    let mut obj = parse_bigip_security_packet_filter_policy(full_path, body, range);
    let (names, _) = firewall_rules_summary(&props_map(body));
    obj.rules = names;
    obj
}

/// `apm policy agent <subtype>` — the subtype names the agent's kind and
/// lives in the stanza header, not in the body.
#[must_use]
pub fn parse_apm_policy_agent(
    full_path: &str,
    body: &str,
    agent_type: &str,
    range: Range,
) -> BigipApmPolicyAgent {
    let mut obj = parse_bigip_apm_policy_agent(full_path, body, range);
    agent_type.clone_into(&mut obj.agent_type);
    obj
}

/// `sys file ssl-cert` — `cert_validators` list.
#[must_use]
pub fn parse_sys_file_ssl_cert(full_path: &str, body: &str, range: Range) -> BigipSysFileSslCert {
    let mut obj = parse_bigip_sys_file_ssl_cert(full_path, body, range);
    obj.cert_validators = list_block_of(&props_map(body), "cert-validators");
    obj
}

/// `sys provision` — `name = full_path` (bare module name).
#[must_use]
pub fn parse_sys_provision(full_path: &str, body: &str, range: Range) -> BigipSysProvision {
    let mut obj = parse_bigip_sys_provision(full_path, body, range);
    full_path.clone_into(&mut obj.name);
    obj
}

/// `auth partition` — `name = full_path` (cluster-wide identifier).
#[must_use]
pub fn parse_auth_partition(full_path: &str, body: &str, range: Range) -> BigipAuthPartition {
    let mut obj = parse_bigip_auth_partition(full_path, body, range);
    full_path.clone_into(&mut obj.name);
    obj
}

// Range helpers

/// Build an inclusive `Range` from token start/exclusive-end offsets.
fn range_from_token_offsets(ctx: BespokeCtx, start: usize, end_exclusive: usize) -> Range {
    let end = end_exclusive.max(start + 1) - 1;
    Range::from_offsets(ctx.source, ctx.line_index, start, end)
}

#[cfg(test)]
mod tests;
