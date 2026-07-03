//! Shared helpers for the generated scalar per-kind parsers — the
//! property-extraction primitives they build on (`props_map`, `description`,
//! `state_flag`, `list_field`) and the `full_path.rsplit("/")[-1]` name
//! convention.

use std::collections::HashMap;
use std::hash::BuildHasher;

use super::helpers::{parse_list_block, parse_properties, unquote};

/// Build a `key -> value` map from a block body, last-wins on duplicate
/// keys.
#[must_use]
pub fn props_map(body: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for (k, v) in parse_properties(body) {
        map.insert(k, v);
    }
    map
}

/// Leaf name of a full path: the segment after the last `/` (an empty
/// path yields `""`).
#[must_use]
pub fn name_leaf(full_path: &str) -> String {
    full_path.rsplit('/').next().unwrap_or(full_path).to_owned()
}

/// The unquoted `description` property, or empty.
#[must_use]
pub fn description<S: BuildHasher>(props: &HashMap<String, String, S>) -> String {
    props
        .get("description")
        .map(|v| unquote(v).to_owned())
        .unwrap_or_default()
}

/// `"enabled"` / `"disabled"` for a bare state flag, else `""`.
#[must_use]
pub fn state_flag<S: BuildHasher>(props: &HashMap<String, String, S>) -> String {
    if props.contains_key("enabled") {
        "enabled".to_owned()
    } else if props.contains_key("disabled") {
        "disabled".to_owned()
    } else {
        String::new()
    }
}

/// A brace-delimited list field, or empty: `key { a b c }` yields the
/// items; a bare `key value` yields a single entry; absent yields empty.
#[must_use]
pub fn list_field<S: BuildHasher>(props: &HashMap<String, String, S>, key: &str) -> Vec<String> {
    match props.get(key) {
        None => Vec::new(),
        Some(raw) if raw.is_empty() => Vec::new(),
        Some(raw) if raw.starts_with('{') => parse_list_block(raw),
        Some(raw) => vec![raw.clone()],
    }
}

/// A scalar string property, or empty.
#[must_use]
pub fn get_str<S: BuildHasher>(props: &HashMap<String, String, S>, key: &str) -> String {
    props.get(key).cloned().unwrap_or_default()
}

/// Whether a bare flag property is present.
#[must_use]
pub fn get_bool<S: BuildHasher>(props: &HashMap<String, String, S>, key: &str) -> bool {
    props.contains_key(key)
}

/// A scalar integer property, or `0` when absent / unparseable.
#[must_use]
pub fn get_int<S: BuildHasher>(props: &HashMap<String, String, S>, key: &str) -> i64 {
    props.get(key).and_then(|v| v.parse().ok()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use crate::model::r#gen::parsers::parse_bigip_ltm_nat;
    use crate::range::Range;

    #[test]
    fn scalar_parser_ltm_nat() {
        // Body + expected field values captured as fixtures for the `ltm nat`
        // scalar parse (see the differential corpus).
        let body = "\n    translation-address 1.2.3.4\n    originating-address 5.6.7.8\n    \
                    traffic-group /Common/tg\n    description \"my nat\"\n    \
                    vlans { /Common/v1 /Common/v2 }\n    vlans-enabled\n";
        let nat = parse_bigip_ltm_nat("/Common/n1", body, Range::zero());
        assert_eq!(nat.name, "n1");
        assert_eq!(nat.translation_address, "1.2.3.4");
        assert_eq!(nat.originating_address, "5.6.7.8");
        assert_eq!(nat.traffic_group, "/Common/tg");
        assert_eq!(nat.description, "my nat");
        assert_eq!(nat.vlans, vec!["/Common/v1", "/Common/v2"]);
        assert!(nat.vlans_enabled);
        assert!(!nat.vlans_disabled);
        assert_eq!(nat.state, "");
    }
}
