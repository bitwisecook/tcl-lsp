//! Canonical-JSON serialisation of an [`AplModel`], matching the schema
//! `dialects.f5.bigip.parser._rust_bridge.rebuild_apl`
//! reconstructs the `AplModel` dataclasses from.

use serde_json::{Map, Value, json};

use super::model::{AplField, AplModel, AplSection, AplTable};
use crate::canonical::range as range_value;
use crate::range::Range;

fn opt_range(r: Option<Range>) -> Value {
    r.map_or(Value::Null, range_value)
}

fn field_value(f: &AplField) -> Value {
    json!({
        "name": f.name,
        "qualified_name": f.qualified_name,
        "field_type": f.field_type,
        "is_required": f.is_required,
        "range": range_value(f.range),
    })
}

fn pairs<T>(items: &[(String, T)], f: impl Fn(&T) -> Value) -> Value {
    Value::Array(
        items
            .iter()
            .map(|(k, v)| Value::Array(vec![Value::String(k.clone()), f(v)]))
            .collect(),
    )
}

fn section_value(s: &AplSection) -> Value {
    json!({
        "name": s.name,
        "qualified_name": s.qualified_name,
        "fields": pairs(&s.fields, field_value),
        "range": opt_range(s.range),
    })
}

fn table_value(t: &AplTable) -> Value {
    json!({
        "name": t.name,
        "qualified_name": t.qualified_name,
        "columns": pairs(&t.columns, field_value),
        "range": opt_range(t.range),
    })
}

/// Serialise an [`AplModel`] to the canonical JSON document.
#[must_use]
pub fn model_to_canonical(model: &AplModel) -> Value {
    let mut out = Map::new();
    out.insert("sections".to_owned(), pairs(&model.sections, section_value));
    out.insert("tables".to_owned(), pairs(&model.tables, table_value));
    out.insert(
        "defines".to_owned(),
        pairs(&model.defines, |v| Value::String(v.clone())),
    );
    out.insert(
        "includes".to_owned(),
        Value::Array(
            model
                .includes
                .iter()
                .map(|i| json!({"path": i.path, "line": i.line, "resolved": i.resolved}))
                .collect(),
        ),
    );
    out.insert(
        "all_fields".to_owned(),
        pairs(&model.all_fields, field_value),
    );
    Value::Object(out)
}
