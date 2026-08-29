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

//! Canonical JSON snapshots of the F5 registries and graphs.
//!
//! These produce deterministic JSON snapshots of the registry data,
//! serialised with two-space indentation and sorted keys.
//!
//! Snapshots:
//!
//! - [`profile_graph_snapshot`] — the protocol profile graph and the
//!   protocol-namespace map.
//! - [`object_graph_snapshot`] — the BIG-IP object graph: object kinds,
//!   the header→kind map, and property reference edges.
//! - [`event_graph_snapshot`] — the iRules event graph: per-event protocol
//!   props (with the `transport` string/list/null remapping), firing order,
//!   flow chains, and the content-addressed valid-command digests.
//!
//! The `commands` snapshot is **not** produced here: it embeds the full
//! per-command traits/scalars dicts and the hover prose catalogue
//! (`summary`), which have no clean, stable serialisation in this module.
//! The `f5 registry-dump` verb reports that section (and the `all`
//! aggregate containing it) as unavailable.

use std::collections::BTreeMap;

use crate::bigip::{BigipPropertySpec, default_registry};
use crate::lifecycle::Lifecycle;
use crate::profiles::ProfileRegistry;

/// Minimal JSON value tree with a two-space-indented, key-sorted
/// serialiser.
///
/// Object keys are always emitted in sorted (byte-wise) order. The
/// serialiser escapes non-ASCII as `\uXXXX` (ASCII-only output).
#[derive(Debug, Clone)]
pub enum Json {
    /// JSON `null`.
    Null,
    /// JSON boolean.
    Bool(bool),
    /// JSON integer (snapshot data has no fractional numbers).
    Int(i64),
    /// JSON string.
    Str(String),
    /// JSON array.
    Array(Vec<Json>),
    /// JSON object. Keys are sorted at serialisation time.
    Object(BTreeMap<String, Json>),
}

impl Json {
    /// Build a `Json::Str` from any displayable string.
    pub fn s(value: impl Into<String>) -> Json {
        Json::Str(value.into())
    }

    /// Build a `Json::Array` of strings.
    pub fn str_array<I, T>(items: I) -> Json
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        Json::Array(items.into_iter().map(|v| Json::Str(v.into())).collect())
    }

    /// Serialise to a string as JSON with a 2-space indent and keys sorted
    /// alphabetically (no trailing newline).
    #[must_use]
    pub fn dumps_indent2(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0);
        out
    }

    fn write(&self, out: &mut String, indent: usize) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Int(n) => out.push_str(&n.to_string()),
            Json::Str(s) => write_json_string(out, s),
            Json::Array(items) => {
                if items.is_empty() {
                    out.push_str("[]");
                    return;
                }
                out.push('[');
                let child = indent + 2;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push('\n');
                    push_spaces(out, child);
                    item.write(out, child);
                }
                out.push('\n');
                push_spaces(out, indent);
                out.push(']');
            }
            Json::Object(map) => {
                if map.is_empty() {
                    out.push_str("{}");
                    return;
                }
                out.push('{');
                let child = indent + 2;
                // BTreeMap already iterates in sorted key order.
                for (i, (key, value)) in map.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push('\n');
                    push_spaces(out, child);
                    write_json_string(out, key);
                    out.push_str(": ");
                    value.write(out, child);
                }
                out.push('\n');
                push_spaces(out, indent);
                out.push('}');
            }
        }
    }
}

fn push_spaces(out: &mut String, n: usize) {
    for _ in 0..n {
        out.push(' ');
    }
}

/// Escape `value` as a JSON string literal with ASCII-only output
/// (escape non-ASCII as `\uXXXX`): the standard short escapes, `\uXXXX`
/// for every other control character, and `\uXXXX` (with surrogate pairs for
/// astral code points) for every non-ASCII character.
fn write_json_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                push_u_escape(out, c as u32);
            }
            c if (c as u32) < 0x7f => out.push(c),
            c => {
                // ASCII-only output: emit \uXXXX, surrogate-pairing astral
                // code points.
                let cp = c as u32;
                if cp <= 0xffff {
                    push_u_escape(out, cp);
                } else {
                    let v = cp - 0x1_0000;
                    let high = 0xd800 + (v >> 10);
                    let low = 0xdc00 + (v & 0x3ff);
                    push_u_escape(out, high);
                    push_u_escape(out, low);
                }
            }
        }
    }
    out.push('"');
}

fn push_u_escape(out: &mut String, code: u32) {
    use std::fmt::Write;
    let _ = write!(out, "\\u{code:04x}");
}

/// Snapshot of the protocol profile graph and protocol-namespace map.
#[must_use]
pub fn profile_graph_snapshot() -> Json {
    let reg = ProfileRegistry::build();

    let mut profiles = BTreeMap::new();
    let mut names = reg.all_profile_names();
    names.sort_unstable();
    for name in names {
        let spec = reg.get_profile(name).expect("listed profile resolves");
        let mut entry = BTreeMap::new();
        entry.insert("name".to_owned(), Json::s(spec.name));
        insert_lifecycle(&mut entry, spec.lifecycle());
        // The registry stores `tls_shared` for the shared TLS/persistence
        // layer (PERSIST / SSL_PERSISTENCE) to drive its infrastructure logic;
        // both are reported as `tls`.
        let layer = if spec.layer == "tls_shared" {
            "tls"
        } else {
            spec.layer
        };
        entry.insert("layer".to_owned(), Json::s(layer));
        entry.insert("side".to_owned(), Json::s(spec.side));
        // Published as the two flat lists the snapshot has always carried;
        // the registry itself holds one relation list (R12) and projects the
        // two directions back out here.
        entry.insert(
            "requires".to_owned(),
            Json::str_array(sorted(&spec.inferred_parents())),
        );
        entry.insert(
            "conflicts".to_owned(),
            Json::str_array(sorted(&spec.forbidden_peers())),
        );
        profiles.insert(spec.name.to_owned(), Json::Object(entry));
    }

    let mut namespaces = BTreeMap::new();
    let mut prefixes = reg.all_namespace_prefixes();
    prefixes.sort_unstable();
    for prefix in prefixes {
        let spec = reg
            .get_namespace(prefix)
            .expect("listed namespace resolves");
        let mut entry = BTreeMap::new();
        entry.insert("prefix".to_owned(), Json::s(spec.prefix));
        entry.insert(
            "profiles".to_owned(),
            Json::str_array(sorted(spec.profiles)),
        );
        entry.insert("layer".to_owned(), Json::s(spec.layer));
        entry.insert("side".to_owned(), Json::s(spec.side));
        entry.insert(
            "side_selectable".to_owned(),
            Json::Bool(spec.side_selectable),
        );
        namespaces.insert(spec.prefix.to_owned(), Json::Object(entry));
    }

    let mut root = BTreeMap::new();
    root.insert("schema".to_owned(), Json::s("tcl-lsp/registry/profiles/v1"));
    root.insert("profiles".to_owned(), Json::Object(profiles));
    root.insert("protocolNamespaces".to_owned(), Json::Object(namespaces));
    Json::Object(root)
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
        entry.insert("kind".to_owned(), Json::s(ks.kind));
        entry.insert(
            "table_name".to_owned(),
            ks.table_name.map_or(Json::Null, Json::s),
        );
        entry.insert(
            "resolver_name".to_owned(),
            ks.resolver_name.map_or(Json::Null, Json::s),
        );
        entry.insert("module".to_owned(), ks.module.map_or(Json::Null, Json::s));
        entry.insert(
            "object_types".to_owned(),
            Json::str_array(ks.object_types.iter().copied()),
        );
        object_kinds.push(Json::Object(entry));
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
            entry.insert("module".to_owned(), Json::s(module));
            entry.insert("objectType".to_owned(), Json::s(object_type));
            entry.insert("kind".to_owned(), Json::s(kind));
            Json::Object(entry)
        })
        .collect();

    // propertyReferences: for every
    // (module, object_type) header, the properties carrying references,
    // keyed by property name. The snapshot iterates
    // sorted by (module, object_type) then property name; within a
    // property name the spec tuple preserves registration (append) order.
    let property_references = build_property_references(reg.specs());

    let mut root = BTreeMap::new();
    root.insert("schema".to_owned(), Json::s("tcl-lsp/registry/objects/v1"));
    root.insert(
        "objectKindCount".to_owned(),
        Json::Int(i64::try_from(object_kinds.len()).unwrap_or(i64::MAX)),
    );
    root.insert("objectKinds".to_owned(), Json::Array(object_kinds));
    root.insert("headerKindMap".to_owned(), Json::Array(header_kind_map));
    root.insert(
        "propertyReferences".to_owned(),
        Json::Array(property_references),
    );
    Json::Object(root)
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
                entry.insert("module".to_owned(), Json::s(module.clone()));
                entry.insert("objectType".to_owned(), Json::s(object_type.clone()));
                entry.insert("section".to_owned(), Json::s(section.clone()));
                entry.insert("property".to_owned(), Json::s(prop.name));
                entry.insert("valueType".to_owned(), Json::s(prop.value_type.as_str()));
                entry.insert("required".to_owned(), Json::Bool(prop.required));
                entry.insert(
                    "references".to_owned(),
                    Json::str_array(sorted(prop.references)),
                );
                entry.insert(
                    "enumValues".to_owned(),
                    Json::str_array(sorted(prop.enum_values)),
                );
                entry.insert(
                    "usageFlags".to_owned(),
                    Json::str_array(sorted(prop.usage_flags)),
                );
                out.push(Json::Object(entry));
            }
        }
    }
    out
}

/// Serialise a [`Lifecycle`] into a snapshot object under the three
/// canonical keys. Every registry surface names the fields the same way and
/// uses the same null semantics: `null` means the lifecycle never reached
/// that state, and `retiredVersion` is the **exclusive** first release
/// without the entity.
fn insert_lifecycle(obj: &mut BTreeMap<String, Json>, lifecycle: Lifecycle) {
    obj.insert(
        "introducedVersion".to_owned(),
        lifecycle.introduced.map_or(Json::Null, Json::s),
    );
    obj.insert(
        "deprecatedVersion".to_owned(),
        lifecycle.deprecated.map_or(Json::Null, Json::s),
    );
    obj.insert(
        "retiredVersion".to_owned(),
        lifecycle.retired.map_or(Json::Null, Json::s),
    );
}

/// Sorted copy of a `&[&str]` slice as owned `String`s.
fn sorted(items: &[&str]) -> Vec<String> {
    let mut v: Vec<String> = items.iter().map(|s| (*s).to_owned()).collect();
    v.sort_unstable();
    v
}

/// Stable content digest of a string list: sort the
/// items, join with newlines, and SHA-256 — `"sha256:" + hexdigest`. The two
/// heaviest registry facts (the per-event valid-command cross-product) are
/// content-addressed this way so the snapshot stays small.
fn digest_list(items: &[String]) -> String {
    use sha2::{Digest, Sha256};

    let mut sorted_items: Vec<&str> = items.iter().map(String::as_str).collect();
    sorted_items.sort_unstable();
    let joined = sorted_items.join("\n");
    let digest = Sha256::digest(joined.as_bytes());
    // `sha2` 0.11 returns a `hybrid-array` `Array`, which — unlike the
    // `generic-array` one 0.10 returned — does not implement `LowerHex`, so
    // `{digest:x}` no longer compiles. Hand-rolled rather than pulling in a hex
    // crate for one call site: this string is part of the committed registry
    // snapshot, so the encoding must stay exactly lowercase, zero-padded, two
    // characters per byte.
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push(char::from_digit(u32::from(byte >> 4), 16).expect("nibble is < 16"));
        hex.push(char::from_digit(u32::from(byte & 0x0f), 16).expect("nibble is < 16"));
    }
    format!("sha256:{hex}")
}

/// Serialise an [`EventProps`](crate::events::EventProps) as a JSON
/// object (the 9 fields; `sort_keys` reorders them
/// at emit time). `transport` is remapped: empty → `null`, one → a string,
/// many → a list.
fn event_props_json(props: &crate::events::EventProps) -> Json {
    let transport = match props.transport {
        [] => Json::Null,
        [one] => Json::s(*one),
        many => Json::str_array(sorted(many)),
    };
    let mut obj = BTreeMap::new();
    // Declared BIG-IP lifecycle: explicit introduction data, or the 15.0 axis
    // baseline; `null` deprecated/retired = the state was never reached.
    insert_lifecycle(&mut obj, props.lifecycle());
    obj.insert("client_side".to_owned(), Json::Bool(props.client_side));
    obj.insert("server_side".to_owned(), Json::Bool(props.server_side));
    obj.insert("transport".to_owned(), transport);
    obj.insert(
        "implied_profiles".to_owned(),
        Json::str_array(sorted(props.implied_profiles)),
    );
    obj.insert("flow".to_owned(), Json::Bool(props.flow));
    obj.insert("hot".to_owned(), Json::Bool(props.hot));
    obj.insert("common".to_owned(), Json::Bool(props.common));
    obj.insert(
        "setup_event".to_owned(),
        props.setup_event.map_or(Json::Null, Json::s),
    );
    Json::Object(obj)
}

/// Snapshot of the iRules event graph: per-event protocol properties, firing
/// order, flow chains, and the commands valid in each event.
///
/// The per-event valid-command list is content-addressed (`validCommandsDigest`)
/// rather than emitted verbatim.
#[must_use]
pub fn event_graph_snapshot() -> Json {
    use crate::events::EventRegistry;

    let dialect = "f5-irules";
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    // The profile-stamped registry: the §9 operator-head exclusion and the
    // specs' own surfaces govern the per-event valid-command digests, exactly
    // as they govern `event-info`.
    let cmds = crate::cache::registry_for_profile(tcl_dialect::DialectProfile::irules());

    // Sorted event names.
    let mut names = events.all_event_names();
    names.sort_unstable();

    let mut event_items: Vec<Json> = Vec::with_capacity(names.len());
    for name in &names {
        // The dump keeps the digest-stable no-filter view; per-version
        // dumps opt in by passing a pin.
        let info = cmds.event_info(name, &events, &profiles, None);
        let mut entry = BTreeMap::new();
        entry.insert("event".to_owned(), Json::s(*name));
        entry.insert(
            "props".to_owned(),
            events.get_props(name).map_or(Json::Null, event_props_json),
        );
        entry.insert(
            "orderIndex".to_owned(),
            events.event_index(name).map_or(Json::Null, |i| {
                Json::Int(i64::try_from(i).unwrap_or(i64::MAX))
            }),
        );
        entry.insert("known".to_owned(), Json::Bool(info.known));
        insert_lifecycle(&mut entry, info.lifecycle);
        entry.insert(
            "lifecycleState".to_owned(),
            Json::s(info.lifecycle_state.as_str()),
        );
        entry.insert("multiplicity".to_owned(), Json::s(info.multiplicity));
        entry.insert("side".to_owned(), Json::s(info.side));
        entry.insert(
            "transport".to_owned(),
            info.transport.as_deref().map_or(Json::Null, Json::s),
        );
        entry.insert(
            "impliedProfiles".to_owned(),
            Json::str_array(info.implied_profiles.iter().map(|s| (*s).to_owned())),
        );
        entry.insert(
            "validCommandCount".to_owned(),
            Json::Int(i64::try_from(info.valid_command_count()).unwrap_or(i64::MAX)),
        );
        entry.insert(
            "validCommandsDigest".to_owned(),
            Json::s(digest_list(&info.valid_commands)),
        );
        event_items.push(Json::Object(entry));
    }

    // One entry per event with its sorted profiles, in MASTER_ORDER.
    let master_order: Vec<Json> = events
        .master_order()
        .iter()
        .map(|entry| {
            let mut obj = BTreeMap::new();
            obj.insert("event".to_owned(), Json::s(entry.event));
            obj.insert(
                "profiles".to_owned(),
                Json::str_array(sorted(entry.profile_gates)),
            );
            Json::Object(obj)
        })
        .collect();

    // The flow chains, serialised as a map keyed by chain id.
    let mut flow_chains = BTreeMap::new();
    for chain in events.flow_chains() {
        let steps: Vec<Json> = chain
            .steps
            .iter()
            .map(|step| {
                let mut s = BTreeMap::new();
                s.insert("event".to_owned(), Json::s(step.event));
                s.insert("phase".to_owned(), Json::s(step.phase));
                s.insert("conditional".to_owned(), Json::Bool(step.conditional));
                s.insert("condition_note".to_owned(), Json::s(step.condition_note));
                Json::Object(s)
            })
            .collect();
        let mut c = BTreeMap::new();
        c.insert("chain_id".to_owned(), Json::s(chain.chain_id));
        c.insert("description".to_owned(), Json::s(chain.description));
        c.insert(
            "profiles".to_owned(),
            Json::str_array(sorted(chain.profiles)),
        );
        c.insert("steps".to_owned(), Json::Array(steps));
        c.insert("notes".to_owned(), Json::s(chain.notes));
        flow_chains.insert(chain.chain_id.to_owned(), Json::Object(c));
    }

    let mut root = BTreeMap::new();
    root.insert("schema".to_owned(), Json::s("tcl-lsp/registry/events/v1"));
    root.insert("dialect".to_owned(), Json::s(dialect));
    root.insert(
        "eventCount".to_owned(),
        Json::Int(i64::try_from(event_items.len()).unwrap_or(i64::MAX)),
    );
    root.insert("events".to_owned(), Json::Array(event_items));
    root.insert("masterOrder".to_owned(), Json::Array(master_order));
    root.insert("flowChains".to_owned(), Json::Object(flow_chains));
    Json::Object(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_escaping_is_correct() {
        assert_eq!(Json::s("a/b").dumps_indent2(), "\"a/b\"");
        assert_eq!(Json::s("x\"y").dumps_indent2(), "\"x\\\"y\"");
        assert_eq!(Json::s("uni\u{e9}").dumps_indent2(), "\"uni\\u00e9\"");
        assert_eq!(Json::s("\u{1f600}").dumps_indent2(), "\"\\ud83d\\ude00\"");
        assert_eq!(Json::s("\u{7f}").dumps_indent2(), "\"\\u007f\"");
        assert_eq!(Json::s("\u{01}").dumps_indent2(), "\"\\u0001\"");
    }

    #[test]
    fn empty_containers_inline() {
        assert_eq!(Json::Array(vec![]).dumps_indent2(), "[]");
        assert_eq!(Json::Object(BTreeMap::new()).dumps_indent2(), "{}");
    }

    #[test]
    fn profile_snapshot_has_expected_shape() {
        let snap = profile_graph_snapshot();
        let Json::Object(root) = &snap else {
            panic!("root is an object");
        };
        assert!(root.contains_key("profiles"));
        assert!(root.contains_key("protocolNamespaces"));
    }

    #[test]
    fn object_snapshot_kind_count() {
        let snap = object_graph_snapshot();
        let Json::Object(root) = &snap else {
            panic!("root is an object");
        };
        assert!(matches!(root.get("objectKindCount"), Some(Json::Int(798))));
    }
}
