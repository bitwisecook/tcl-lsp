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

//! Thin wrapper over the `f5-query` engine that returns results as
//! `serde_json::Value`.
//!
//! The BIG-IP report is built entirely from a handful of `f5-query` DSL
//! expressions — the same engine that powers `f5 query`. This module runs an
//! expression against one or more in-memory configs and converts the engine's
//! [`Value`]s into `serde_json::Value`, using exactly the mapping the `PyO3`
//! binding (`f5report._engine`) uses so the Rust port sees the same shapes the
//! Python generator did:
//!
//! * `PathRef`   → the target full-path string;
//! * `ObjectRef` → `{"kind", "full-path", "fields": {…}}`;
//! * `Container` → `"container(<kind>)"`;
//! * `Drop`/`Null` → `null`.

use serde_json::{Map, Value as J};
use tcl_bigip_query::value::Value;
use tcl_bigip_query::{QueryOptions, run_query};

/// A loaded config source: `(uri, scf_text)`.
pub type Source = (String, String);

/// Error carrying the engine's message (parse / evaluation / load failure).
#[derive(Debug, Clone)]
pub struct ReportError(pub String);

impl std::fmt::Display for ReportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ReportError {}

/// Convert an engine [`Value`] into a `serde_json::Value`.
///
/// Mirrors `f5report._engine::value_to_py` one-for-one.
pub(crate) fn value_to_json(value: &Value) -> J {
    match value {
        Value::Null | Value::Drop => J::Null,
        Value::Bool(b) => J::Bool(*b),
        Value::Int(i) => J::from(*i),
        Value::Float(f) => serde_json::Number::from_f64(*f).map_or(J::Null, J::Number),
        Value::Str(s) => J::String(s.clone()),
        Value::PathRef(p) => J::String(p.full_path.clone()),
        Value::List(items) | Value::Stream(items) => {
            J::Array(items.iter().map(value_to_json).collect())
        }
        Value::Object(map) => {
            let mut out = Map::new();
            for (k, v) in map {
                out.insert(k.clone(), value_to_json(v));
            }
            J::Object(out)
        }
        Value::Container(c) => J::String(format!("container({})", c.kind)),
        Value::ObjectRef(o) => {
            let mut fields = Map::new();
            for (k, v) in &o.fields {
                fields.insert(k.clone(), value_to_json(v));
            }
            let mut dict = Map::new();
            dict.insert("kind".into(), J::String(o.kind.clone()));
            dict.insert("full-path".into(), J::String(o.full_path.clone()));
            dict.insert("fields".into(), J::Object(fields));
            J::Object(dict)
        }
    }
}

/// Run a read-only `f5-query` expression against `sources`, returning a flat
/// list of result values (top-level streams flattened) as `serde_json::Value`.
///
/// Mirrors `f5report._engine::query` (default, non-`per_file` path). A mutating
/// expression is rejected — a report never rewrites its input.
pub(crate) fn query(expr: &str, sources: &[Source]) -> Result<Vec<J>, ReportError> {
    let opts = QueryOptions::default();
    let result = run_query(expr, sources, &opts).map_err(|e| ReportError(e.to_string()))?;
    if result.has_mutation {
        return Err(ReportError(
            "mutating queries are not supported by the report engine (read-only)".into(),
        ));
    }
    let mut out = Vec::new();
    for (_uri, values) in &result.values_per_file {
        for v in values {
            out.push(value_to_json(v));
        }
    }
    Ok(out)
}
