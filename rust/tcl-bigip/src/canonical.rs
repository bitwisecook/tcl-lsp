//! Canonical JSON serialisation matching the reference schema, so a round-trip
//! through `config_to_canonical` reconstructs the exact canonical structs the
//! consumer layer expects.
//!
//! Tags: `{"r":[6]}` Range, `{"S":[2]}` SourceSpan, `{"e":NAME}` enum,
//! `{"s":STR}` scalar value, `{"t":[…]}` tuple, `{"m":{…}}` dict,
//! `{"__t":NAME,"d":{…}}` nested struct, `{"L":{…}}` BigipList.

#![allow(
    clippy::wildcard_imports,
    clippy::doc_markdown,
    clippy::implicit_hasher
)]

use serde_json::{Map, Value, json};

use crate::model::{BigipGenericObject, BigipMinimalObject};
use crate::parser::driver::BigipConfig;
use crate::range::Range;
use crate::value::bigip_list::{BigipList, ListItem, ListItemValue, SourceSpan};

/// A type that serialises to the canonical `"d"` field-map and knows its
/// canonical type name.
pub trait Canon {
    /// The canonical type name (e.g. `"BigipPoolMember"`).
    fn canon_type(&self) -> &'static str;
    /// The canonical `"d"` field map (a JSON object).
    fn canon_fields(&self) -> Value;
}

/// Wrap a [`Canon`] value as a tagged nested struct.
#[must_use]
pub fn tagged<T: Canon + ?Sized>(v: &T) -> Value {
    json!({"__t": v.canon_type(), "d": v.canon_fields()})
}

// Scalar / primitive helpers

/// Canonical form of a [`Range`].
#[must_use]
pub fn range(r: Range) -> Value {
    json!({"r": [r.start.line, r.start.character, r.start.offset,
                 r.end.line, r.end.character, r.end.offset]})
}

/// Canonical form of an optional [`Range`].
#[must_use]
pub fn opt_range(r: Option<Range>) -> Value {
    r.map_or(Value::Null, range)
}

/// Canonical form of a [`SourceSpan`].
#[must_use]
pub fn span(s: SourceSpan) -> Value {
    json!({"S": [s.start, s.end]})
}

/// Canonical form of an optional [`SourceSpan`].
#[must_use]
pub fn opt_span(s: Option<SourceSpan>) -> Value {
    s.map_or(Value::Null, span)
}

/// Canonical form of a typed scalar value (by its `Display`).
#[must_use]
pub fn scalar<T: std::fmt::Display>(v: &T) -> Value {
    json!({"s": v.to_string()})
}

/// Canonical form of an optional typed scalar value.
#[must_use]
pub fn opt_scalar<T: std::fmt::Display>(v: Option<&T>) -> Value {
    v.map_or(Value::Null, |x| scalar(x))
}

/// Canonical form of a `Vec<String>` (a tuple of strings).
#[must_use]
pub fn vec_str(v: &[String]) -> Value {
    json!({"t": v.iter().map(|s| Value::String(s.clone())).collect::<Vec<_>>()})
}

/// Canonical form of a tuple of nested structured values.
#[must_use]
pub fn vec_tagged<T: Canon>(v: &[T]) -> Value {
    json!({"t": v.iter().map(tagged).collect::<Vec<_>>()})
}

/// Canonical form of the pool-member `field_offsets` map.
///
/// The input is a `HashMap`, whose iteration order is randomised per run.
/// With `serde_json/preserve_order` feature-unified on across the workspace,
/// a `serde_json::Map` preserves insertion order, so iterating the `HashMap`
/// directly would leak that random order into the canonical JSON string and
/// make the PyO3 cross-language contract nondeterministic. Sort the keys
/// first so the same input always serialises to the same bytes.
#[must_use]
pub fn field_offsets(m: &std::collections::HashMap<String, (usize, usize)>) -> Value {
    let mut keys: Vec<&String> = m.keys().collect();
    keys.sort();
    let mut obj = Map::new();
    for k in keys {
        let (a, b) = m[k];
        obj.insert(k.clone(), json!({"t": [a, b]}));
    }
    json!({"m": obj})
}

// BigipList

/// Canonical form of a [`BigipList`].
#[must_use]
pub fn list(l: &BigipList) -> Value {
    json!({
        "L": {
            "syntax": l.syntax.as_str(),
            "operator": l.operator,
            "raw": l.raw,
            "range": opt_span(l.range),
            "items": l.items.iter().map(item).collect::<Vec<_>>(),
        }
    })
}

fn item(it: &ListItem) -> Value {
    json!({
        "v": list_value(&it.value),
        "k": it.key,
        "b": it.body,
        "raw": it.raw,
        "range": opt_span(it.range),
        "key_range": opt_span(it.key_range),
        "body_range": opt_span(it.body_range),
    })
}

fn list_value(v: &ListItemValue) -> Value {
    match v {
        ListItemValue::Str(s) => Value::String(s.clone()),
        ListItemValue::Profile(p) => tagged(p),
        ListItemValue::Persistence(p) => tagged(p),
        ListItemValue::CertKeyChain(c) => tagged(c),
        ListItemValue::DataGroupRecord(d) => tagged(d),
        ListItemValue::GtmRegionMember(g) => tagged(g),
        ListItemValue::FirewallRule(f) => tagged(f),
        ListItemValue::MonitorExpression(m) => tagged(m),
        ListItemValue::PoolMember(p) => tagged(p),
    }
}

// Structured value-layer types (ListItem values)

use crate::value::attachments::{PersistenceAttachment, ProfileAttachment};
use crate::value::cert_key_chain::CertKeyChain;
use crate::value::data_group_record::DataGroupRecord;
use crate::value::firewall_rule::{FirewallEndpoint, FirewallRule};
use crate::value::gtm_region_member::GtmRegionMember;
use crate::value::monitor_expression::MonitorExpression;

impl Canon for ProfileAttachment {
    fn canon_type(&self) -> &'static str {
        "ProfileAttachment"
    }
    fn canon_fields(&self) -> Value {
        json!({"path": self.path, "context": self.context, "raw": self.raw})
    }
}

impl Canon for PersistenceAttachment {
    fn canon_type(&self) -> &'static str {
        "PersistenceAttachment"
    }
    fn canon_fields(&self) -> Value {
        json!({"path": self.path, "default": self.default, "raw": self.raw})
    }
}

impl Canon for CertKeyChain {
    fn canon_type(&self) -> &'static str {
        "CertKeyChain"
    }
    fn canon_fields(&self) -> Value {
        json!({
            "name": self.name, "cert": self.cert, "key": self.key,
            "chain": self.chain, "passphrase": self.passphrase, "raw": self.raw,
        })
    }
}

impl Canon for DataGroupRecord {
    fn canon_type(&self) -> &'static str {
        "DataGroupRecord"
    }
    fn canon_fields(&self) -> Value {
        json!({"key": self.key, "data": self.data, "raw": self.raw})
    }
}

impl Canon for GtmRegionMember {
    fn canon_type(&self) -> &'static str {
        "GtmRegionMember"
    }
    fn canon_fields(&self) -> Value {
        json!({"kind": self.kind, "value": self.value, "negated": self.negated, "raw": self.raw})
    }
}

impl Canon for FirewallEndpoint {
    fn canon_type(&self) -> &'static str {
        "FirewallEndpoint"
    }
    fn canon_fields(&self) -> Value {
        json!({
            "port_lists": vec_str(&self.port_lists),
            "ports": vec_str(&self.ports),
            "address_lists": vec_str(&self.address_lists),
            "addresses": vec_str(&self.addresses),
        })
    }
}

impl Canon for FirewallRule {
    fn canon_type(&self) -> &'static str {
        "FirewallRule"
    }
    fn canon_fields(&self) -> Value {
        json!({
            "name": self.name, "action": self.action, "ip_protocol": self.ip_protocol,
            "log": self.log, "source": tagged(&self.source), "destination": tagged(&self.destination),
            "rule_list": self.rule_list, "raw": self.raw,
        })
    }
}

impl Canon for MonitorExpression {
    fn canon_type(&self) -> &'static str {
        "MonitorExpression"
    }
    fn canon_fields(&self) -> Value {
        let spans: Vec<Value> = self
            .monitor_spans
            .iter()
            .map(|(a, b)| json!({"t": [a, b]}))
            .collect();
        json!({
            "mode": self.mode.as_str(),
            "monitors": vec_str(&self.monitors),
            "minimum": self.minimum,
            "raw": self.raw,
            "monitor_spans": {"t": spans},
        })
    }
}

// Top-level config

impl Canon for BigipGenericObject {
    fn canon_type(&self) -> &'static str {
        "BigipGenericObject"
    }
    fn canon_fields(&self) -> Value {
        json!({
            "module": self.module,
            "object_type": self.object_type,
            "identifier": self.identifier,
            "header": self.header,
            "range": opt_range(self.range),
        })
    }
}

impl Canon for BigipMinimalObject {
    fn canon_type(&self) -> &'static str {
        "BigipMinimalObject"
    }
    fn canon_fields(&self) -> Value {
        json!({
            "name": self.name,
            "full_path": self.full_path,
            "kind": self.kind,
            "description": self.description,
            "range": opt_range(self.range),
        })
    }
}

/// Serialise a [`BigipConfig`] to the canonical JSON document the
/// `_rust_bridge.rebuild` step consumes.
#[must_use]
pub fn config_to_canonical(config: &BigipConfig) -> Value {
    let generic: Vec<Value> = config
        .generic_objects
        .iter()
        .map(|(key, obj)| json!([key, obj.canon_fields()]))
        .collect();
    let objects: Vec<Value> = config
        .objects
        .iter()
        .map(|p| {
            json!({
                "table": p.table_name,
                "full_path": p.full_path,
                "fields": p.object.canon_fields(),
            })
        })
        .collect();
    json!({
        "default_partition": config.default_partition,
        "generic_objects": generic,
        "objects": objects,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::field_offsets;

    /// Build a `field_offsets`-shaped map with enough keys that `HashMap`'s
    /// randomised iteration order would surface if it leaked.
    fn sample_map() -> HashMap<String, (usize, usize)> {
        let mut m = HashMap::new();
        for (i, key) in [
            "session",
            "address",
            "state",
            "monitor",
            "ratio",
            "priority-group",
            "connection-limit",
            "rate-limit",
            "fqdn",
            "description",
        ]
        .iter()
        .enumerate()
        {
            m.insert((*key).to_string(), (i, i + 1));
        }
        m
    }

    /// The canonical `field_offsets` JSON must be deterministic — two
    /// independently-seeded `HashMap`s with the same entries must serialise
    /// to byte-identical JSON, otherwise the PyO3 cross-language contract
    /// diverges run-to-run.
    #[test]
    fn field_offsets_key_order_is_deterministic() {
        let a = field_offsets(&sample_map());
        let b = field_offsets(&sample_map());
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap(),
            "field_offsets serialisation must not depend on HashMap iteration order",
        );

        // The same map serialised twice in a row must also match, and the
        // emitted keys must be in sorted order.
        let map = sample_map();
        let first = serde_json::to_string(&field_offsets(&map)).unwrap();
        let second = serde_json::to_string(&field_offsets(&map)).unwrap();
        assert_eq!(first, second);

        let value = field_offsets(&map);
        let inner = value.get("m").and_then(|m| m.as_object()).unwrap();
        let mut sorted: Vec<&String> = inner.keys().collect();
        let observed: Vec<&String> = inner.keys().collect();
        sorted.sort();
        assert_eq!(observed, sorted, "keys must be emitted in sorted order");
    }
}
