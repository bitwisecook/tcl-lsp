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

use crate::tools::{ToolResult, refused, tool_error};

use serde_json::{Value, json};
use tcl_bigip::irule_context::{
    build_irule_context, context_bundle_to_json, context_bundle_to_text,
};
use tcl_bigip::model::ModelObject;
use tcl_bigip::parser::{BigipConfig, parse_bigip_conf};

/// Parse inline `config_text` plus any `config_paths` files, keeping each
/// entry's origin and raw text.
fn load_configs(args: &Value) -> Result<Vec<(String, String, BigipConfig)>, Value> {
    let mut configs: Vec<(String, String, BigipConfig)> = Vec::new();
    if let Some(text) = args
        .get("config_text")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    {
        configs.push((
            "<config_text>".to_owned(),
            text.to_owned(),
            parse_bigip_conf(text, "Common"),
        ));
    }
    if let Some(paths) = args.get("config_paths").and_then(Value::as_array) {
        for path in paths.iter().filter_map(Value::as_str) {
            match std::fs::read_to_string(path) {
                Ok(text) => {
                    let cfg = parse_bigip_conf(&text, "Common");
                    configs.push((path.to_owned(), text, cfg));
                }
                Err(e) => return Err(tool_error(format_args!("reading {path}: {e}"))),
            }
        }
    }
    if configs.is_empty() {
        return Err(tool_error(
            "no config provided; pass config_text or config_paths",
        ));
    }
    Ok(configs)
}

fn merge_loaded(configs: &[(String, String, BigipConfig)]) -> BigipConfig {
    let config_refs: Vec<(String, &BigipConfig)> = configs
        .iter()
        .map(|(origin, _text, cfg)| (origin.clone(), cfg))
        .collect();
    tcl_bigip::lint::merge_configs(&config_refs)
}

/// `hud_chain` — the HUD chain of each virtual server: its profiles as HUD
/// filters, the attached iRules in rule-list order, and every `when` handler
/// merged into one table per event, ordered by priority and rule position.
pub fn hud_chain(args: &Value) -> ToolResult {
    let configs = load_configs(args)?;
    let merged = merge_loaded(&configs);
    let virtual_name = args.get("virtual").and_then(Value::as_str).unwrap_or("");
    let rule_filter = args.get("rule_path").and_then(Value::as_str).unwrap_or("");

    let mut chains = if virtual_name.is_empty() {
        tcl_bigip::hud_chain::hud_chains(&merged)
    } else {
        match tcl_bigip::hud_chain::hud_chain(&merged, virtual_name) {
            Some(chain) => vec![chain],
            None => {
                return Ok(refused(format_args!(
                    "no virtual server {virtual_name} in config"
                )));
            }
        }
    };
    if !rule_filter.is_empty() {
        chains.retain(|chain| {
            chain.rules.iter().any(|rule| {
                rule.reference == rule_filter || rule.full_path.as_deref() == Some(rule_filter)
            })
        });
    }
    if chains.is_empty() {
        return Ok(refused(if rule_filter.is_empty() {
            "no virtual servers in config".to_owned()
        } else {
            format!("no virtual server attaches {rule_filter}")
        }));
    }
    let chains = serde_json::to_value(&chains)
        .map_err(|error| tool_error(format_args!("serialising HUD chains: {error}")))?;
    Ok(json!({ "chains": chains }))
}

/// `irule_with_context` — bundle each iRule in a BIG-IP config with the objects
/// it references (pools, data-groups, profiles, monitors, …). Mirrors the
/// `f5 irule context` pipeline: parse + merge the config(s), build one bundle
/// per `ltm rule`, render to JSON or Tcl-flavoured text.
pub fn irule_with_context(args: &Value) -> ToolResult {
    let rule_filter = args.get("rule_path").and_then(Value::as_str).unwrap_or("");
    let format = args.get("format").and_then(Value::as_str).unwrap_or("json");
    let transitive = args
        .get("transitive")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    let configs = load_configs(args)?;

    // Merge once so cross-file references resolve.
    let merged = merge_loaded(&configs);
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
        return Ok(refused("no iRules found in config"));
    }

    let bundle_values: Vec<Value> = bundles.into_iter().map(|(_path, value)| value).collect();
    Ok(json!({ "format": format, "bundles": bundle_values }))
}

/// `explain_flow` — narrate a captured session against a BIG-IP config: matched
/// virtual server, profiles, HTTP/TLS, ordered iRule events, pool decision, and
/// (optionally) a simulated outcome. Over `f5_cli::explain_flow_value`.
pub fn explain_flow(args: &Value) -> ToolResult {
    let pcap_path = args.get("pcap_path").and_then(Value::as_str).unwrap_or("");
    if pcap_path.is_empty() {
        return Err(tool_error("pcap_path is required"));
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
        std::fs::write(&p, text)
            .map_err(|error| tool_error(format_args!("staging config text: {error}")))?;
        paths.push(p.clone());
        tempfile = Some(p);
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
    result.map_err(tool_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONF: &str = "\
ltm rule /Common/r1 {
when HTTP_REQUEST { set u [HTTP::uri] }
}
ltm rule /Common/r2 {
when HTTP_REQUEST priority 100 { set u x }
}
ltm virtual /Common/vs1 {
    destination /Common/10.0.0.1:80
    profiles { /Common/http { } /Common/tcp { } }
    rules { /Common/r1 /Common/r2 }
}
ltm virtual /Common/vs2 {
    destination /Common/10.0.0.2:80
    profiles { /Common/tcp { } }
}
";

    #[test]
    fn hud_chain_merges_handlers_across_rules() {
        let out = hud_chain(&json!({ "config_text": CONF, "virtual": "vs1" })).unwrap();
        let chains = out["chains"].as_array().unwrap();
        assert_eq!(chains.len(), 1);
        let chain = &chains[0];
        assert_eq!(chain["virtualServer"], "/Common/vs1");
        assert_eq!(chain["filters"][0]["profileType"], "TCP");
        assert_eq!(chain["filters"][1]["profileType"], "HTTP");
        assert_eq!(chain["filters"][1]["typeSource"], "baseObject");
        let handlers = &chain["events"][0]["handlers"];
        assert_eq!(chain["events"][0]["event"], "HTTP_REQUEST");
        assert_eq!(handlers[0]["rule"], "/Common/r2");
        assert_eq!(handlers[0]["priority"], 100);
        assert_eq!(handlers[1]["rule"], "/Common/r1");
        assert_eq!(chain["events"][0]["raised"], true);
    }

    #[test]
    fn hud_chain_filters_by_attached_rule() {
        let out = hud_chain(&json!({ "config_text": CONF, "rule_path": "/Common/r1" })).unwrap();
        let names: Vec<_> = out["chains"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["virtualServer"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["/Common/vs1"]);

        let all = hud_chain(&json!({ "config_text": CONF })).unwrap();
        assert_eq!(all["chains"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn hud_chain_refuses_unknown_targets_and_errors_without_config() {
        let missing = hud_chain(&json!({ "config_text": CONF, "virtual": "nope" })).unwrap();
        assert!(missing["refused"]["reason"].is_string());
        let unattached =
            hud_chain(&json!({ "config_text": CONF, "rule_path": "/Common/zz" })).unwrap();
        assert!(unattached["refused"]["reason"].is_string());
        assert!(hud_chain(&json!({})).is_err());
    }
}
