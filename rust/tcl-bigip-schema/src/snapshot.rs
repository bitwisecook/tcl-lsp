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

//! Deterministic BIG-IP object-graph snapshot data.

use crate::bigip::{BigipPropertySpec, default_registry};
use serde_json::Value as Json;
use std::collections::BTreeMap;

fn string(value: impl Into<String>) -> Json {
    Json::String(value.into())
}
fn strings<I, T>(items: I) -> Json
where
    I: IntoIterator<Item = T>,
    T: Into<String>,
{
    Json::Array(items.into_iter().map(|v| string(v)).collect())
}
fn sorted(items: &[&str]) -> Vec<String> {
    let mut values: Vec<_> = items.iter().map(|s| (*s).to_owned()).collect();
    values.sort_unstable();
    values
}

/// Snapshot of the BIG-IP object graph: object kinds and reference edges.
#[must_use]
pub fn object_graph_snapshot() -> Json {
    let reg = default_registry();

    // objectKinds: sorted by kind name; each kind is the serialised
    // `BigipObjectKindSpec` fields, whose keys the serialiser then
    // re-sorts alphabetically.
    let mut specs: Vec<_> = reg.specs().to_vec();
    specs.sort_by_key(|s| s.kind_spec.kind);

    let mut object_kinds = Vec::with_capacity(specs.len());
    for spec in &specs {
        let ks = spec.kind_spec;
        let mut entry = BTreeMap::new();
        entry.insert("kind".to_owned(), string(ks.kind));
        entry.insert(
            "table_name".to_owned(),
            ks.table_name.map_or(Json::Null, string),
        );
        entry.insert(
            "resolver_name".to_owned(),
            ks.resolver_name.map_or(Json::Null, string),
        );
        entry.insert("module".to_owned(), ks.module.map_or(Json::Null, string));
        entry.insert(
            "object_types".to_owned(),
            strings(ks.object_types.iter().copied()),
        );
        object_kinds.push(Json::Object(entry.into_iter().collect()));
    }

    // headerKindMap: every (module, object_type) -> kind, sorted by the
    // (module, object_type) key tuple.
    let mut header_entries: Vec<(&str, &str, &str)> = Vec::new();
    for spec in reg.specs() {
        for &(module, object_type) in spec.header_types {
            header_entries.push((module, object_type, spec.kind_spec.kind));
        }
    }
    header_entries.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));
    let header_kind_map = header_entries
        .into_iter()
        .map(|(module, object_type, kind)| {
            let mut entry = BTreeMap::new();
            entry.insert("module".to_owned(), string(module));
            entry.insert("objectType".to_owned(), string(object_type));
            entry.insert("kind".to_owned(), string(kind));
            Json::Object(entry.into_iter().collect())
        })
        .collect();

    // propertyReferences: for every
    // (module, object_type) header, the properties carrying references,
    // keyed by property name. The snapshot iterates
    // sorted by (module, object_type) then property name; within a
    // property name the spec tuple preserves registration (append) order.
    let property_references = build_property_references(reg.specs());

    let mut root = BTreeMap::new();
    root.insert("schema".to_owned(), string("tcl-lsp/registry/objects/v1"));
    root.insert("objectKindCount".to_owned(), Json::from(object_kinds.len()));
    root.insert("objectKinds".to_owned(), Json::Array(object_kinds));
    root.insert("headerKindMap".to_owned(), Json::Array(header_kind_map));
    root.insert(
        "propertyReferences".to_owned(),
        Json::Array(property_references),
    );
    Json::Object(root.into_iter().collect())
}

/// Build the sorted `propertyReferences` list: the property-reference
/// assembly plus snapshot ordering.
fn build_property_references(specs: &[&'static crate::bigip::BigipObjectSpec]) -> Vec<Json> {
    // (module, object_type) -> (property name -> appended specs).
    // BTreeMap gives the sorted iteration the snapshot relies on; the inner
    // Vec preserves the registration (append) order used for the spec
    // tuple of a repeated property name.
    type SectionMap = BTreeMap<String, Vec<&'static BigipPropertySpec>>;
    let mut by_header: BTreeMap<(String, String), SectionMap> = BTreeMap::new();

    for spec in specs {
        for &(module, object_type) in spec.header_types {
            let section_map = by_header
                .entry((module.to_owned(), object_type.to_owned()))
                .or_default();
            for prop in spec.properties {
                if prop.references.is_empty() {
                    continue;
                }
                section_map
                    .entry(prop.name.to_owned())
                    .or_default()
                    .push(prop);
            }
        }
    }

    let mut out = Vec::new();
    for ((module, object_type), section_map) in &by_header {
        for (section, props) in section_map {
            for prop in props {
                let mut entry = BTreeMap::new();
                entry.insert("module".to_owned(), string(module.clone()));
                entry.insert("objectType".to_owned(), string(object_type.clone()));
                entry.insert("section".to_owned(), string(section.clone()));
                entry.insert("property".to_owned(), string(prop.name));
                entry.insert("valueType".to_owned(), string(prop.value_type.as_str()));
                entry.insert("required".to_owned(), Json::Bool(prop.required));
                entry.insert("references".to_owned(), strings(sorted(prop.references)));
                entry.insert("enumValues".to_owned(), strings(sorted(prop.enum_values)));
                entry.insert("usageFlags".to_owned(), strings(sorted(prop.usage_flags)));
                out.push(Json::Object(entry.into_iter().collect()));
            }
        }
    }
    out
}
