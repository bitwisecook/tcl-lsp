//! Shared helpers for the generated scalar per-kind parsers — the
//! property-extraction primitives the Python parsers build on
//! (`_parse_properties`, `_description`, `_state_flag`, `_list_field`,
//! and the `full_path.rsplit("/")[-1]` name convention).

#![allow(clippy::implicit_hasher)]

use std::collections::HashMap;

use super::helpers::{parse_list_block, parse_properties, unquote};

/// Build a `key -> value` map from a block body, last-wins on duplicate
/// keys (mirrors Python `_parse_properties` returning a dict).
#[must_use]
pub fn props_map(body: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for (k, v) in parse_properties(body) {
        map.insert(k, v);
    }
    map
}

/// Leaf name of a full path: the segment after the last `/`. Mirrors
/// `full_path.rsplit("/", 1)[-1]` (an empty path yields `""`).
#[must_use]
pub fn name_leaf(full_path: &str) -> String {
    full_path.rsplit('/').next().unwrap_or(full_path).to_owned()
}

/// The unquoted `description` property, or empty. Mirrors `_description`.
#[must_use]
pub fn description(props: &HashMap<String, String>) -> String {
    props
        .get("description")
        .map(|v| unquote(v).to_owned())
        .unwrap_or_default()
}

/// `"enabled"` / `"disabled"` for a bare state flag, else `""`. Mirrors
/// `_state_flag`.
#[must_use]
pub fn state_flag(props: &HashMap<String, String>) -> String {
    if props.contains_key("enabled") {
        "enabled".to_owned()
    } else if props.contains_key("disabled") {
        "disabled".to_owned()
    } else {
        String::new()
    }
}

/// A brace-delimited list field, or empty. Mirrors `_list_field`:
/// `key { a b c }` yields the items; a bare `key value` yields a single
/// entry; absent yields empty.
#[must_use]
pub fn list_field(props: &HashMap<String, String>, key: &str) -> Vec<String> {
    match props.get(key) {
        None => Vec::new(),
        Some(raw) if raw.is_empty() => Vec::new(),
        Some(raw) if raw.starts_with('{') => parse_list_block(raw),
        Some(raw) => vec![raw.clone()],
    }
}

/// A scalar string property, or empty (mirrors `props.get(key, "")`).
#[must_use]
pub fn get_str(props: &HashMap<String, String>, key: &str) -> String {
    props.get(key).cloned().unwrap_or_default()
}

/// Whether a bare flag property is present (mirrors `"key" in props`).
#[must_use]
pub fn get_bool(props: &HashMap<String, String>, key: &str) -> bool {
    props.contains_key(key)
}

/// A scalar integer property, or `0` when absent / unparseable.
#[must_use]
pub fn get_int(props: &HashMap<String, String>, key: &str) -> i64 {
    props.get(key).and_then(|v| v.parse().ok()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use crate::model::r#gen::parsers::parse_bigip_ltm_nat;
    use crate::range::Range;

    #[test]
    fn scalar_parser_matches_python_ltm_nat() {
        // Body + expected field values captured from the live Python
        // `_parse_ltm_nat` (see the differential corpus).
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
