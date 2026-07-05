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

//! Canonical-JSON serialisation of an [`AplModel`], matching the schema
//! the bridge reconstructs the `AplModel` structs from.

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
