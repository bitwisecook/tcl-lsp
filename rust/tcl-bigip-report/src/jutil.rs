//! Small accessors over `serde_json` maps, so the port of the Python model
//! shaping reads close to the original `dict.get(...)` code.

use serde_json::{Map, Value as J};

/// Python-style truthiness of a JSON value (`None`/`""`/`0`/`[]`/`{}` → false).
pub(crate) fn truthy(v: &J) -> bool {
    match v {
        J::Null => false,
        J::Bool(b) => *b,
        J::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        J::String(s) => !s.is_empty(),
        J::Array(a) => !a.is_empty(),
        J::Object(o) => !o.is_empty(),
    }
}

/// `map[key]` as `&str`, or `""` when absent / not a string.
pub(crate) fn bstr<'a>(map: &'a Map<String, J>, key: &str) -> &'a str {
    map.get(key).and_then(J::as_str).unwrap_or("")
}

/// `bool(map.get(key))` with Python truthiness.
pub(crate) fn bbool(map: &Map<String, J>, key: &str) -> bool {
    map.get(key).is_some_and(truthy)
}

/// `map[key]` as an array slice, or empty when absent / not an array.
pub(crate) fn barr<'a>(map: &'a Map<String, J>, key: &str) -> &'a [J] {
    match map.get(key) {
        Some(J::Array(a)) => a.as_slice(),
        _ => &[],
    }
}

/// `map[key]` as a list of `&str` (non-string members skipped).
pub(crate) fn sarr<'a>(map: &'a Map<String, J>, key: &str) -> Vec<&'a str> {
    barr(map, key).iter().filter_map(J::as_str).collect()
}
