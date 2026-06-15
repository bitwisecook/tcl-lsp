//! JSON serialisation of an [`ExplorerResult`] into the explorer contract
//! shape (`docs/design/contracts/wasm-explorer-view.md` for the `wasm`
//! slice; the rest is the de-facto contract `explorer-core.js` reads).
//!
//! Faithful port of `tooling/cli/serialise.py`, brought up one view-family
//! at a time (EXP-1b..N). Each `serialise_*` helper mirrors the matching
//! Python `_serialise_*` and is verified against it by the differential
//! parity test. `serialise_result` assembles the top-level object; keys
//! not yet ported are simply absent (the parity harness compares only the
//! keys present on both sides as families land).

use serde_json::{Map, Value, json};

use tcl_registry::available_dialects;

use crate::ExplorerResult;
use crate::views::{Severity, VIEW_META};

/// Serialise the `meta` view: dialect list, view-tab table, and the
/// severity vocabulary. Mirrors `_serialise_meta`.
#[must_use]
pub fn serialise_meta() -> Value {
    let dialects: Vec<Value> = available_dialects()
        .iter()
        .map(|d| Value::String((*d).to_owned()))
        .collect();
    let views: Vec<Value> = VIEW_META
        .iter()
        .map(|&(id, label, group)| json!({ "id": id, "label": label, "group": group }))
        .collect();
    let severities: Vec<Value> = Severity::ALL
        .iter()
        .map(|s| Value::String(s.as_str().to_owned()))
        .collect();
    json!({
        "dialects": dialects,
        "views": views,
        "severities": severities,
    })
}

/// Serialise a full pipeline result to the explorer contract JSON.
///
/// Currently emits the ported view families; subsequent EXP-* increments
/// add one family per step. The argument is accepted now so the signature
/// is stable as views that read `result` land.
#[must_use]
pub fn serialise_result(_result: &ExplorerResult) -> Value {
    let mut out = Map::new();
    out.insert("meta".to_owned(), serialise_meta());
    Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_pipeline;

    #[test]
    fn meta_lists_all_dialects_views_and_severities() {
        let meta = serialise_meta();
        assert_eq!(meta["dialects"].as_array().unwrap().len(), 14);
        assert_eq!(meta["views"].as_array().unwrap().len(), 24);
        assert_eq!(meta["severities"], json!(["error", "warning", "info"]));
        // First view tab matches the Python table head.
        assert_eq!(
            meta["views"][0],
            json!({ "id": "greentree", "label": "Green Tree", "group": "compiler" })
        );
    }

    #[test]
    fn serialise_result_includes_meta() {
        let result = run_pipeline("set x 1", "tcl8.6");
        let value = serialise_result(&result);
        assert!(value.get("meta").is_some());
    }
}
