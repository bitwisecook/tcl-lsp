//! Canonical JSON snapshots of the F5 registries and graphs.
//!
//! Rust port of the snapshot builders in Python `tooling/registry_snapshot.py`.
//! These produce deterministic, byte-for-byte snapshots of the registry data,
//! serialised with `json.dumps`-parity (`indent=2`, `sort_keys=True`).
//!
//! Ported snapshots:
//!
//! - [`profile_graph_snapshot`] — the protocol profile graph and the
//!   protocol-namespace map (`profile_graph_snapshot` in Python).
//! - [`object_graph_snapshot`] — the BIG-IP object graph: object kinds,
//!   the header→kind map, and property reference edges (`object_graph_snapshot`).
//!
//! The `commands` and `events` snapshots are **not** ported here: they embed
//! the full event-validity cross-product (`validCommandsDigest` /
//! `validEventsDigest`) and the hover prose catalogue (`summary`), which
//! reflect Python-internal derivation machinery without a clean, byte-identical
//! Rust equivalent. The `f5 registry-dump` verb reports those sections as
//! deferred.

use std::collections::BTreeMap;

use crate::bigip::{BigipPropertySpec, default_registry};
use crate::profiles::ProfileRegistry;

/// Minimal JSON value tree with a `json.dumps(indent=2, sort_keys=True)`-parity
/// serialiser.
///
/// Object keys are always emitted in sorted (byte-wise) order, matching
/// Python's `sort_keys=True`. The serialiser escapes strings exactly like
/// `CPython`'s `json` module with the default `ensure_ascii=True`.
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
    fn s(value: impl Into<String>) -> Json {
        Json::Str(value.into())
    }

    /// Build a `Json::Array` of strings.
    fn str_array<I, T>(items: I) -> Json
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        Json::Array(items.into_iter().map(|v| Json::Str(v.into())).collect())
    }

    /// Serialise to a string matching Python `json.dumps(indent=2,
    /// sort_keys=True)` byte-for-byte (no trailing newline).
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
                // BTreeMap already iterates in sorted key order, matching
                // Python's `sort_keys=True`.
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

/// Escape `value` as a JSON string literal exactly like `CPython`'s `json` module
/// with the default `ensure_ascii=True`: the standard short escapes, `\uXXXX`
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
                // ensure_ascii=True: emit \uXXXX, surrogate-pairing astral
                // code points (matching `CPython`'s behaviour).
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
///
/// Mirrors Python `profile_graph_snapshot()`.
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
        // The Rust crate stores `tls_shared` for the shared TLS/persistence
        // layer (PERSIST / SSL_PERSISTENCE) to drive its infrastructure logic;
        // the Python registry the snapshot mirrors reports both as `tls`.
        let layer = if spec.layer == "tls_shared" {
            "tls"
        } else {
            spec.layer
        };
        entry.insert("layer".to_owned(), Json::s(layer));
        entry.insert("side".to_owned(), Json::s(spec.side));
        entry.insert(
            "requires".to_owned(),
            Json::str_array(sorted(spec.requires)),
        );
        entry.insert(
            "conflicts".to_owned(),
            Json::str_array(sorted(spec.conflicts)),
        );
        entry.insert(
            "capabilities".to_owned(),
            Json::str_array(sorted(spec.capabilities)),
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
///
/// Mirrors Python `object_graph_snapshot()`.
#[must_use]
pub fn object_graph_snapshot() -> Json {
    let reg = default_registry();

    // objectKinds: sorted by kind name; each kind is `_jsonable(spec)` —
    // the BigipObjectKindSpec dataclass, whose keys `sort_keys=True` then
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

    // propertyReferences: mirrors PROPERTY_REFERENCE_SPECS — for every
    // (module, object_type) header, the properties carrying references,
    // keyed by property name. The Python snapshot iterates
    // sorted((module, object_type)) then sorted(property_name); within a
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

/// Build the sorted `propertyReferences` list, reproducing Python's
/// `PROPERTY_REFERENCE_SPECS` assembly + snapshot ordering.
fn build_property_references(specs: &[&'static crate::bigip::BigipObjectSpec]) -> Vec<Json> {
    // (module, object_type) -> (property name -> appended specs).
    // BTreeMap gives the sorted iteration the snapshot relies on; the inner
    // Vec preserves the registration (append) order Python uses for the spec
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

/// Sorted copy of a `&[&str]` slice as owned `String`s.
fn sorted(items: &[&str]) -> Vec<String> {
    let mut v: Vec<String> = items.iter().map(|s| (*s).to_owned()).collect();
    v.sort_unstable();
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_escaping_matches_python() {
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
