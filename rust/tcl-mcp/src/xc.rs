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

//! `xc_translate` — iRule → F5 Distributed Cloud (XC) static translation,
//! backed by the `f5-xc` crate (model + terraform / JSON-API renderers).

use f5_xc::model::{TranslateStatus, TranslationItem};
use serde_json::{Map, Value, json};

const NAMESPACE: &str = "default";
const LB_NAME: &str = "translated-lb";

fn items_where(
    result: &f5_xc::XCTranslationResult,
    status: TranslateStatus,
) -> impl Iterator<Item = &TranslationItem> {
    result.items.iter().filter(move |i| i.status == status)
}

/// Translate `source` to XC constructs and render terraform / JSON-API per
/// `output_format` (`"terraform"` | `"json"` | `"both"`, default `"both"`).
pub fn xc_translate(args: &Value) -> Value {
    let source = args.get("source").and_then(Value::as_str).unwrap_or("");
    let output_format = match args.get("output_format").and_then(Value::as_str) {
        Some(f) if !f.is_empty() => f,
        _ => "both",
    };
    let result = f5_xc::translate_irule(source);

    let mut output = Map::new();
    if output_format == "terraform" || output_format == "both" {
        output.insert(
            "terraform".to_owned(),
            json!(f5_xc::render_terraform(&result, NAMESPACE, LB_NAME)),
        );
    }
    if output_format == "json" || output_format == "both" {
        output.insert(
            "json_api".to_owned(),
            f5_xc::render_json(&result, NAMESPACE, LB_NAME),
        );
    }
    output.insert("coverage_pct".to_owned(), json!(result.coverage_pct));

    let translatable: Vec<Value> = items_where(&result, TranslateStatus::Translated)
        .map(|i| json!({ "command": i.irule_command, "kind": i.kind.as_str(), "xc_description": i.xc_description }))
        .collect();
    let untranslatable: Vec<Value> = items_where(&result, TranslateStatus::Untranslatable)
        .map(|i| json!({ "command": i.irule_command, "reason": i.xc_description, "suggestion": i.note, "diagnostic_code": i.diagnostic_code }))
        .collect();
    let partial: Vec<Value> = items_where(&result, TranslateStatus::Partial)
        .map(|i| json!({ "command": i.irule_command, "reason": i.xc_description, "suggestion": i.note, "diagnostic_code": i.diagnostic_code }))
        .collect();
    let advisory: Vec<Value> = items_where(&result, TranslateStatus::Advisory)
        .map(|i| json!({ "command": i.irule_command, "xc_description": i.xc_description, "note": i.note }))
        .collect();

    output.insert("translatable".to_owned(), Value::Array(translatable));
    output.insert("untranslatable".to_owned(), Value::Array(untranslatable));
    output.insert("partial".to_owned(), Value::Array(partial));
    output.insert("advisory".to_owned(), Value::Array(advisory));

    output.insert(
        "summary".to_owned(),
        json!(format!(
            "Coverage: {:.1}% — {} translatable, {} partial, {} untranslatable, {} advisory",
            result.coverage_pct,
            result.translatable_count(),
            result.partial_count(),
            result.untranslatable_count(),
            result.advisory_count(),
        )),
    );

    Value::Object(output)
}
