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

//! The JSON payload a translation is reported as.
//!
//! One shape serves every caller: the `xc_translate` MCP tool, the
//! `tcl-lsp.xcTranslate` workspace command the editors invoke, and anything
//! else that renders a translation. Callers differ only in the transport, so
//! the field set lives here rather than being re-derived per consumer.

use serde_json::{Map, Value, json};

use crate::model::{TranslateStatus, TranslationItem, XCTranslationResult};

/// XC namespace the renderers are given.
pub const DEFAULT_NAMESPACE: &str = "default";
/// XC load-balancer name the renderers are given.
pub const DEFAULT_LB_NAME: &str = "translated-lb";

/// Which renderings a payload carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// Terraform HCL only.
    Terraform,
    /// ves.io JSON API only.
    Json,
    /// Both renderings.
    #[default]
    Both,
    /// One pasteable F5 XC Console document per object.
    Console,
}

impl OutputFormat {
    /// The wire spelling a client sends (`"terraform"` | `"json"` |
    /// `"console"` | `"both"`). Anything else — including an absent or empty
    /// argument — is [`OutputFormat::Both`], so a client that omits the
    /// argument still gets a complete payload.
    ///
    /// `"both"` stays terraform plus JSON API: the Console documents are a
    /// separate rendering of the same objects, and a client that asked for
    /// everything before this existed should not start receiving them.
    #[must_use]
    pub fn from_arg(arg: Option<&str>) -> Self {
        match arg {
            Some("terraform") => Self::Terraform,
            Some("json") => Self::Json,
            Some("console") => Self::Console,
            _ => Self::Both,
        }
    }

    fn wants_console(self) -> bool {
        matches!(self, Self::Console)
    }

    fn wants_terraform(self) -> bool {
        matches!(self, Self::Terraform | Self::Both)
    }

    fn wants_json(self) -> bool {
        matches!(self, Self::Json | Self::Both)
    }
}

fn items_where(
    result: &XCTranslationResult,
    status: TranslateStatus,
) -> impl Iterator<Item = &TranslationItem> {
    result.items.iter().filter(move |i| i.status == status)
}

fn item_value(item: &TranslationItem) -> Value {
    json!({
        "status": item.status.as_str(),
        "kind": item.kind.as_str(),
        "command": item.irule_command,
        "xc_description": item.xc_description,
        "note": item.note,
        "diagnostic_code": item.diagnostic_code,
    })
}

/// Render `result` as the reporting payload.
///
/// Beyond the requested renderings (`terraform`, `json_api`) it carries
/// `coverage_pct`, the per-status counts, the full `items` list, the
/// per-status item lists, and a one-line `summary`.
#[must_use]
pub fn translation_payload(
    result: &XCTranslationResult,
    namespace: &str,
    lb_name: &str,
    output_format: OutputFormat,
) -> Value {
    let mut output = Map::new();
    if output_format.wants_terraform() {
        output.insert(
            "terraform".to_owned(),
            json!(crate::render_terraform(result, namespace, lb_name)),
        );
    }
    if output_format.wants_json() {
        output.insert(
            "json_api".to_owned(),
            crate::render_json(result, namespace, lb_name),
        );
    }
    if output_format.wants_console() {
        output.insert(
            "console_objects".to_owned(),
            Value::Array(
                crate::render_console_objects(result, namespace, lb_name)
                    .into_iter()
                    .map(|o| {
                        json!({
                            "object_type": o.object_type,
                            "name": o.name,
                            "source_path": o.source_path,
                            "namespace": o.namespace,
                            "document": o.document,
                        })
                    })
                    .collect(),
            ),
        );
    }
    output.insert("coverage_pct".to_owned(), json!(result.coverage_pct));

    let translatable: Vec<Value> = items_where(result, TranslateStatus::Translated)
        .map(|i| json!({ "command": i.irule_command, "kind": i.kind.as_str(), "xc_description": i.xc_description }))
        .collect();
    let untranslatable: Vec<Value> = items_where(result, TranslateStatus::Untranslatable)
        .map(|i| json!({ "command": i.irule_command, "reason": i.xc_description, "suggestion": i.note, "diagnostic_code": i.diagnostic_code }))
        .collect();
    let partial: Vec<Value> = items_where(result, TranslateStatus::Partial)
        .map(|i| json!({ "command": i.irule_command, "reason": i.xc_description, "suggestion": i.note, "diagnostic_code": i.diagnostic_code }))
        .collect();
    let advisory: Vec<Value> = items_where(result, TranslateStatus::Advisory)
        .map(|i| json!({ "command": i.irule_command, "xc_description": i.xc_description, "note": i.note }))
        .collect();

    output.insert("translatable".to_owned(), Value::Array(translatable));
    output.insert("untranslatable".to_owned(), Value::Array(untranslatable));
    output.insert("partial".to_owned(), Value::Array(partial));
    output.insert("advisory".to_owned(), Value::Array(advisory));

    // The flat, status-tagged list the editors filter client-side.
    output.insert(
        "items".to_owned(),
        Value::Array(result.items.iter().map(item_value).collect()),
    );
    output.insert(
        "translatable_count".to_owned(),
        json!(result.translatable_count()),
    );
    output.insert("partial_count".to_owned(), json!(result.partial_count()));
    output.insert(
        "untranslatable_count".to_owned(),
        json!(result.untranslatable_count()),
    );
    output.insert("advisory_count".to_owned(), json!(result.advisory_count()));

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

#[cfg(test)]
mod tests {
    use super::*;

    const IRULE: &str = "when HTTP_REQUEST {\n    if { [HTTP::uri] starts_with \"/api\" } {\n        pool api_pool\n    }\n}\n";

    fn payload(format: OutputFormat) -> Value {
        translation_payload(
            &crate::translate_irule(IRULE),
            DEFAULT_NAMESPACE,
            DEFAULT_LB_NAME,
            format,
        )
    }

    #[test]
    fn both_carries_terraform_and_json() {
        let out = payload(OutputFormat::Both);
        assert!(
            out["terraform"]
                .as_str()
                .is_some_and(|hcl| hcl.contains("volterra_origin_pool")),
            "{out}"
        );
        assert!(out["json_api"].is_object(), "{out}");
    }

    #[test]
    fn single_format_omits_the_other_rendering() {
        let terraform = payload(OutputFormat::Terraform);
        assert!(terraform.get("terraform").is_some());
        assert!(terraform.get("json_api").is_none(), "{terraform}");

        let api = payload(OutputFormat::Json);
        assert!(api.get("json_api").is_some());
        assert!(api.get("terraform").is_none(), "{api}");
    }

    #[test]
    fn items_are_status_tagged_and_counted() {
        let out = payload(OutputFormat::Both);
        let items = out["items"].as_array().expect("items array");
        assert!(!items.is_empty(), "{out}");
        assert!(
            items
                .iter()
                .all(|i| i.get("status").and_then(Value::as_str).is_some()),
            "{out}"
        );
        let translated = items.iter().filter(|i| i["status"] == "translated").count();
        assert_eq!(Value::from(translated), out["translatable_count"], "{out}");
        assert!(out["coverage_pct"].as_f64().is_some(), "{out}");
    }

    #[test]
    fn console_objects_are_one_pasteable_document_each() {
        let out = payload(OutputFormat::Console);
        let objects = out["console_objects"].as_array().expect("console objects");
        let types: Vec<&str> = objects
            .iter()
            .filter_map(|o| o["object_type"].as_str())
            .collect();
        assert!(types.contains(&"origin_pool"), "{out}");
        assert!(types.contains(&"http_loadbalancer"), "{out}");

        for object in objects {
            let document = object["document"].as_object().expect("document");
            // A create takes `{metadata, spec}` and nothing else, so anything
            // extra here is something the Console editor would reject.
            let mut keys: Vec<&str> = document.keys().map(String::as_str).collect();
            keys.sort_unstable();
            assert_eq!(keys, ["metadata", "spec"], "{object}");

            let metadata = document["metadata"].as_object().expect("metadata");
            let mut meta_keys: Vec<&str> = metadata.keys().map(String::as_str).collect();
            meta_keys.sort_unstable();
            assert_eq!(
                meta_keys,
                [
                    "annotations",
                    "description",
                    "disable",
                    "labels",
                    "name",
                    "namespace"
                ],
                "{object}"
            );
            assert_eq!(metadata["namespace"], Value::from(DEFAULT_NAMESPACE));
            assert_eq!(metadata["name"], object["name"]);
        }
    }

    #[test]
    fn console_document_names_are_valid_xc_names() {
        // A partition-qualified pool is the ordinary BIG-IP spelling, and
        // `/` is not legal in a DNS-1035 name, so a document carrying the
        // raw path would be rejected on paste.
        let result = crate::translate_irule("when HTTP_REQUEST {\n  pool /Common/web-pool\n}\n");
        let out = translation_payload(
            &result,
            DEFAULT_NAMESPACE,
            DEFAULT_LB_NAME,
            OutputFormat::Console,
        );
        for object in out["console_objects"].as_array().expect("objects") {
            let name = object["document"]["metadata"]["name"]
                .as_str()
                .expect("metadata name");
            assert!(
                name.starts_with(|c: char| c.is_ascii_lowercase())
                    && name
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
                    && !name.ends_with('-')
                    && name.len() <= 63,
                "not a DNS-1035 name: {name:?}"
            );
        }

        // The pool keeps its partition, and its source path is recoverable.
        let pool = out["console_objects"]
            .as_array()
            .expect("objects")
            .iter()
            .find(|o| o["object_type"] == "origin_pool")
            .expect("origin pool");
        assert!(
            pool["document"]["metadata"]["name"]
                .as_str()
                .is_some_and(|n| n.starts_with("common-web-pool-")),
            "{pool}"
        );
        assert_eq!(pool["source_path"], Value::from("/Common/web-pool"));
        assert_eq!(
            pool["document"]["metadata"]["description"],
            Value::from("Translated from BIG-IP /Common/web-pool")
        );
    }

    #[test]
    fn console_format_carries_neither_other_rendering() {
        let out = payload(OutputFormat::Console);
        assert!(out.get("terraform").is_none(), "{out}");
        assert!(out.get("json_api").is_none(), "{out}");
        // The coverage report travels with every format.
        assert!(out["coverage_pct"].as_f64().is_some(), "{out}");
    }

    #[test]
    fn both_stays_terraform_and_json_only() {
        // A client that asked for everything before the Console rendering
        // existed must not start receiving it.
        let out = payload(OutputFormat::Both);
        assert!(out.get("console_objects").is_none(), "{out}");
    }

    #[test]
    fn unknown_format_argument_falls_back_to_both() {
        assert_eq!(OutputFormat::from_arg(None), OutputFormat::Both);
        assert_eq!(OutputFormat::from_arg(Some("")), OutputFormat::Both);
        assert_eq!(OutputFormat::from_arg(Some("yaml")), OutputFormat::Both);
        assert_eq!(
            OutputFormat::from_arg(Some("terraform")),
            OutputFormat::Terraform
        );
        assert_eq!(OutputFormat::from_arg(Some("json")), OutputFormat::Json);
        assert_eq!(
            OutputFormat::from_arg(Some("console")),
            OutputFormat::Console
        );
    }
}
