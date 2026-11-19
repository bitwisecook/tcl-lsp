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
    let span = model_range(obj)
        .and_then(|range| {
            root.stanza_sources
                .get_or_init(|| tcl_bigip::range::StanzaSourceIndex::new(&root.source))
                .at(range.start)
        })
        .filter(|span| span.closed);
    let stanza_slot = span.map(|span| {
        let mut extent = span.range();
        let preceding = &root.source[..extent.start];
        let line = preceding.rfind('\n').map_or(0, |offset| offset + 1);
        if preceding[line..]
            .bytes()
            .all(|byte| matches!(byte, b' ' | b'\t'))
        {
            extent.start = line;
        }
        FieldSlot {
            source_uri: root.uri.clone(),
            start: extent.start,
            end: extent.end,
            raw_text: root.source[extent].to_owned(),
        }
    });
    let field_slots = span.map_or_else(IndexMap::new, |span| {
        collect_field_slots(span, &fields, root)
    });

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
    span: &tcl_bigip::parser::BlockSpan,
    fields: &IndexMap<String, Value>,
    root: &Root,
) -> IndexMap<String, FieldSlot> {
    let body = &root.source[span.body.clone()];
    let tokens: Vec<_> = tcl_bigip::parser::shell_tokens(body).collect();
    tcl_bigip::parser::parse_properties_with_spans(body)
        .into_iter()
        .filter_map(|(name, property)| {
            let start = property.value_start?;
            let end = property.value_end?;
            if !fields.contains_key(&name) || body[start..end].contains('\n') {
                return None;
            }
            let from = tokens.partition_point(|token| token.range.end <= start);
            let to = tokens.partition_point(|token| token.range.start < end);
            let value = tokens.get(from..to)?;
            if value
                .iter()
                .any(|t| !t.complete || t.kind != tcl_bigip::parser::ShellTokenKind::Word)
            {
                return None;
            }
            let first = value.first()?.range.start;
            let last = value.last()?.range.end;
            if first != start || last > end {
                return None;
            }
            Some((
                name,
                FieldSlot {
                    source_uri: root.uri.clone(),
                    start: span.body.start + first,
                    end: span.body.start + last,
                    raw_text: body[first..last].to_owned(),
                },
            ))
        })
        .collect()
}
