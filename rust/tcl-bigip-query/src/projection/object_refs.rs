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

//! Object references: the stanza an object came from, its source range and the byte slots of its scalar fields.

use super::project_fields;
use crate::eval::Root;
use crate::value::{FieldSlot, ObjectRef, Value};
use indexmap::IndexMap;
use std::rc::Rc;
use tcl_bigip::model::ModelObject;

// ObjectRef building

pub(super) fn build_object_ref(
    kind: &str,
    full_path: &str,
    obj: &ModelObject,
    root: &Rc<Root>,
) -> Value {
    let cache_key = (kind.to_owned(), full_path.to_owned());
    if let Some(cached) = root.object_cache.borrow().get(&cache_key) {
        return Value::ObjectRef(Rc::clone(cached));
    }

    let fields = project_fields(kind, obj, root);
    let stanza_slot = stanza_slot_for(obj, root);
    let field_slots = collect_field_slots(obj, &fields, root);

    let object_ref = Rc::new(ObjectRef {
        kind: kind.to_owned(),
        full_path: full_path.to_owned(),
        fields,
        field_slots,
        stanza_slot,
        config_uri: root.uri.clone(),
    });
    root.object_cache
        .borrow_mut()
        .insert(cache_key, Rc::clone(&object_ref));
    Value::ObjectRef(object_ref)
}

/// Build the whole-stanza slot from the object's range, extending back to
/// the header's line start.
fn stanza_slot_for(obj: &ModelObject, root: &Rc<Root>) -> Option<FieldSlot> {
    let rng = model_range(obj)?;
    let start = rng.start.offset as usize;
    let end = rng.end.offset as usize;
    let header_start = scan_back_to_line_start(&root.source, start);
    let raw_text = root.source.get(header_start..end)?.to_owned();
    Some(FieldSlot {
        source_uri: root.uri.clone(),
        start: header_start,
        end,
        raw_text,
    })
}

fn scan_back_to_line_start(source: &str, offset: usize) -> usize {
    let bytes = source.as_bytes();
    let mut i = offset.min(bytes.len());
    while i > 0 && bytes[i - 1] != b'\n' {
        i -= 1;
    }
    i
}

fn model_range(obj: &ModelObject) -> Option<tcl_bigip::range::Range> {
    match obj {
        ModelObject::VirtualServer(o) => o.range,
        ModelObject::VirtualAddress(o) => o.range,
        ModelObject::Pool(o) => o.range,
        ModelObject::Node(o) => o.range,
        ModelObject::Rule(o) => o.range,
        ModelObject::Profile(o) => o.range,
        ModelObject::Monitor(o) => o.range,
        ModelObject::Persistence(o) => o.range,
        ModelObject::SnatPool(o) => o.range,
        ModelObject::Policy(o) => o.range,
        ModelObject::DataGroup(o) => o.range,
        ModelObject::LtmSnatTranslation(o) => o.range,
        ModelObject::NetRoute(o) => o.range,
        ModelObject::NetVlan(o) => o.range,
        ModelObject::NetSelf(o) => o.range,
        ModelObject::NetRouteDomain(o) => o.range,
        ModelObject::NetPortList(o) => o.range,
        ModelObject::NetInterface(o) => o.range,
        ModelObject::NetDnsResolver(o) => o.range,
        ModelObject::NetTunnel(o) => o.range,
        ModelObject::NetStp(o) => o.range,
        ModelObject::SysDns(o) => o.range,
        ModelObject::SysNtp(o) => o.range,
        ModelObject::SysSnmp(o) => o.range,
        ModelObject::SysGlobalSettings(o) => o.range,
        ModelObject::SysProvision(o) => o.range,
        ModelObject::SysFolder(o) => o.range,
        ModelObject::SysFileSslCert(o) => o.range,
        ModelObject::SysFileSslKey(o) => o.range,
        ModelObject::SysManagementRoute(o) => o.range,
        ModelObject::CmCert(o) => o.range,
        ModelObject::CmKey(o) => o.range,
        ModelObject::CmDevice(o) => o.range,
        ModelObject::CmDeviceGroup(o) => o.range,
        ModelObject::CmTrafficGroup(o) => o.range,
        ModelObject::CmTrustDomain(o) => o.range,
        ModelObject::CmHaGroup(o) => o.range,
        ModelObject::ApmPolicyAccessPolicy(o) => o.range,
        ModelObject::ApmPolicyItem(o) => o.range,
        ModelObject::ApmPolicyAgent(o) => o.range,
        ModelObject::ApmPolicyCustomizationSource(o) => o.range,
        ModelObject::ApmOauthDbInstance(o) => o.range,
        ModelObject::ApmEphemeralAuthSshSecurityConfig(o) => o.range,
        ModelObject::ApmReportDefaultReport(o) => o.range,
        ModelObject::GtmDatacenter(o) => o.range,
        ModelObject::GtmServer(o) => o.range,
        ModelObject::GtmPool(o) => o.range,
        ModelObject::GtmWideip(o) => o.range,
        ModelObject::GtmListener(o) => o.range,
        ModelObject::SecurityFirewallPolicy(o) => o.range,
        ModelObject::SecurityFirewallRuleList(o) => o.range,
        ModelObject::SecurityFirewallAddressList(o) => o.range,
        ModelObject::SecurityFirewallPortList(o) => o.range,
        ModelObject::SecurityNatPolicy(o) => o.range,
        ModelObject::SecurityNatSourceTranslation(o) => o.range,
        ModelObject::SecurityNatDestinationTranslation(o) => o.range,
        _ => None,
    }
}

// Field-slot byte-range discovery

/// Locate each top-level scalar property's value span inside its stanza.
///
/// Returns a map of TMSH field name → [`FieldSlot`] for every field that
/// appears as a single-line `key value` property in the source *and* is a
/// known projected field. Compound list / sub-block values and identity
/// fields (whose location is the header) are simply absent.
fn collect_field_slots(
    obj: &ModelObject,
    fields: &IndexMap<String, Value>,
    root: &Rc<Root>,
) -> IndexMap<String, FieldSlot> {
    let Some(rng) = model_range(obj) else {
        return IndexMap::new();
    };
    let body_start = rng.start.offset as usize;
    let body_end = rng.end.offset as usize;
    let Some(body_text) = root.source.get(body_start..body_end) else {
        return IndexMap::new();
    };
    let mut slots: IndexMap<String, FieldSlot> = IndexMap::new();
    for (key, value_start, value_end, value_text) in iter_top_level_scalar_slots(body_text) {
        if !fields.contains_key(&key) {
            continue;
        }
        slots.insert(
            key,
            FieldSlot {
                source_uri: root.uri.clone(),
                start: body_start + value_start,
                end: body_start + value_end,
                raw_text: value_text,
            },
        );
    }
    slots
}

/// Yield `(key, value_start, value_end, value_text)` for scalar lines at the
/// stanza's top brace-depth.
fn iter_top_level_scalar_slots(body: &str) -> Vec<(String, usize, usize, String)> {
    let target_depth = i32::from(body.trim_start().starts_with('{'));
    let mut depth = 0i32;
    let mut line_start = 0usize;
    let mut out = Vec::new();
    for line in split_keep_ends(body) {
        let line_end = line_start + line.len();
        let stripped = line.trim();
        if !stripped.is_empty()
            && !stripped.starts_with('#')
            && depth == target_depth
            && let Some(parsed) = parse_scalar_slot_line(line, line_start)
        {
            out.push(parsed);
        }
        depth = brace_depth_after_line(line, depth);
        line_start = line_end;
    }
    out
}

/// Split *body* into lines keeping the trailing `\n` — splits on the
/// `\n`-only line endings SCF uses, keeping the newline on each line.
fn split_keep_ends(body: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let bytes = body.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'\n' {
            out.push(&body[start..=i]);
            start = i + 1;
        }
    }
    if start < body.len() {
        out.push(&body[start..]);
    }
    out
}

/// Parse one top-level scalar property line.
fn parse_scalar_slot_line(line: &str, line_start: usize) -> Option<(String, usize, usize, String)> {
    let content = line.strip_suffix('\n').unwrap_or(line);
    let bytes = content.as_bytes();
    let mut pos = 0usize;
    while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
        pos += 1;
    }
    if pos >= bytes.len() || matches!(bytes[pos], b'{' | b'}' | b'#') {
        return None;
    }
    let key_start = pos;
    while pos < bytes.len() && !matches!(bytes[pos], b' ' | b'\t' | b'{' | b'}') {
        pos += 1;
    }
    let key = &content[key_start..pos];
    if key.is_empty() {
        return None;
    }
    while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
        pos += 1;
    }
    if pos >= bytes.len() {
        return None;
    }
    let value = content[pos..].trim_end_matches([' ', '\t']);
    if value.is_empty() || value.contains('{') || value.contains('}') {
        return None;
    }
    let value_start = line_start + pos;
    let value_end = value_start + value.len();
    Some((key.to_owned(), value_start, value_end, value.to_owned()))
}

/// Update brace depth for one line, ignoring quoted strings.
fn brace_depth_after_line(line: &str, depth: i32) -> i32 {
    let mut depth = depth;
    let mut in_quote = false;
    let mut escaped = false;
    for ch in line.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' && in_quote {
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_quote = !in_quote;
            continue;
        }
        if in_quote {
            continue;
        }
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth = (depth - 1).max(0);
        }
    }
    depth
}
