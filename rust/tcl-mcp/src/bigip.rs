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

//! BIG-IP–specific MCP tools backed by the `tcl-bigip` / `f5-cli` engines.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tcl_bigip::irule_context::{
    build_irule_context, context_bundle_to_json, context_bundle_to_text,
};
use tcl_bigip::model::ModelObject;
use tcl_bigip::parser::{BigipConfig, parse_bigip_conf};

/// `irule_with_context` — bundle each iRule in a BIG-IP config with the objects
/// it references (pools, data-groups, profiles, monitors, …). Mirrors the
/// `f5 irule context` pipeline: parse + merge the config(s), build one bundle
/// per `ltm rule`, render to JSON or Tcl-flavoured text.
pub fn irule_with_context(args: &Value) -> Value {
    let config_text = args
        .get("config_text")
        .and_then(Value::as_str)
        .unwrap_or("");
    let rule_filter = args.get("rule_path").and_then(Value::as_str).unwrap_or("");
    let format = args.get("format").and_then(Value::as_str).unwrap_or("json");
    let transitive = args
        .get("transitive")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    // Resolve inputs: inline `config_text` plus any `config_paths` files.
    // Each entry keeps its raw text so the context builder can slice rule
    // bodies by source range.
    let mut configs: Vec<(String, String, BigipConfig)> = Vec::new();
    if !config_text.is_empty() {
        configs.push((
            "<config_text>".to_owned(),
            config_text.to_owned(),
            parse_bigip_conf(config_text, "Common"),
        ));
    }
    if let Some(paths) = args.get("config_paths").and_then(Value::as_array) {
        for path in paths.iter().filter_map(Value::as_str) {
            match std::fs::read_to_string(path) {
                Ok(text) => {
                    let cfg = parse_bigip_conf(&text, "Common");
                    configs.push((path.to_owned(), text, cfg));
                }
                Err(e) => return json!({ "error": format!("reading {path}: {e}") }),
            }
        }
    }
    if configs.is_empty() {
        return json!({ "error": "no config provided; pass config_text or config_paths" });
    }

    // Merge once so cross-file references resolve.
    let config_refs: Vec<(String, &BigipConfig)> = configs
        .iter()
        .map(|(origin, _text, cfg)| (origin.clone(), cfg))
        .collect();
    let merged = tcl_bigip::lint::merge_configs(&config_refs);
    let registry = crate::environment::store_for_dialect("f5-irules");

    // One bundle per `ltm rule`, optionally filtered by full path.
    let mut bundles: Vec<(String, Value)> = Vec::new();
    for (_origin, text, cfg) in &configs {
        for placed in &cfg.objects {
            if placed.table_name != "rules" {
                continue;
            }
            let ModelObject::Rule(rule) = &placed.object else {
                continue;
            };
            if !rule_filter.is_empty() && placed.full_path != rule_filter {
                continue;
            }
            let bundle =
                build_irule_context(rule, &merged, transitive, Some(text.as_str()), registry);
            let value = if format == "text" {
                json!({ "rule": placed.full_path, "text": context_bundle_to_text(&bundle) })
            } else {
                // `context_bundle_to_json` returns a JSON string; embed it as a
                // structured value (fall back to a stub on the impossible parse
                // failure).
                serde_json::from_str(&context_bundle_to_json(&bundle))
                    .unwrap_or_else(|_| json!({ "rule": placed.full_path }))
            };
            bundles.push((placed.full_path.clone(), value));
        }
    }
    if bundles.is_empty() {
        return json!({ "error": "no iRules found in config" });
    }

    let bundle_values: Vec<Value> = bundles.into_iter().map(|(_path, value)| value).collect();
    json!({ "format": format, "bundles": bundle_values })
}

/// `explain_flow` — narrate a captured session against a BIG-IP config: matched
/// virtual server, profiles, HTTP/TLS, ordered iRule events, pool decision, and
/// (optionally) a simulated outcome. Over `f5_cli::explain_flow_value`.
pub fn explain_flow(args: &Value) -> Value {
    let pcap_path = args.get("pcap_path").and_then(Value::as_str).unwrap_or("");
    if pcap_path.is_empty() {
        return json!({ "error": "pcap_path is required" });
    }

    // Config file paths, plus inline `config_text` staged to a temp file.
    let mut paths: Vec<PathBuf> = Vec::new();
    let mut tempfile: Option<PathBuf> = None;
    if let Some(text) = args
        .get("config_text")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    {
        let mut p = std::env::temp_dir();
        p.push(format!("tcl_mcp_bigip_{}.conf", std::process::id()));
        if std::fs::write(&p, text).is_ok() {
            paths.push(p.clone());
            tempfile = Some(p);
        }
    }
    if let Some(arr) = args.get("config_paths").and_then(Value::as_array) {
        paths.extend(arr.iter().filter_map(Value::as_str).map(PathBuf::from));
    }

    let keylog = args
        .get("keylog_path")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty());
    let tshark_filter = args
        .get("tshark_filter")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty());
    let options = f5_cli::ExplainFlowOptions {
        paths: &paths,
        tshark: args
            .get("use_tshark")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        keylog: keylog.map(Path::new),
        tshark_filter,
        simulate: args
            .get("simulate")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        no_event_bodies: false,
        max_event_lines: args
            .get("max_event_body_lines")
            .and_then(Value::as_u64)
            .and_then(|n| usize::try_from(n).ok())
            .unwrap_or(8),
    };

    let result = f5_cli::explain_flow_value(Path::new(pcap_path), &options);
    if let Some(p) = tempfile {
        let _ = std::fs::remove_file(p);
    }
    result.unwrap_or_else(|e| json!({ "error": e }))
}
