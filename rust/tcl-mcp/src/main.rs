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

//! Native Rust MCP server for the tcl-lsp analysis engine.
//!
//! Built on the official [`rmcp`] SDK (`JSON-RPC` 2.0 over `stdio`). Every tool
//! handler calls the Rust analysis crates (`tcl-lsp-core`, `tcl-compiler`,
//! `tcl-registry`) **directly**. Tool results carry JSON payloads in the MCP
//! text content blocks.

use std::sync::Arc;

use rmcp::ServiceExt;
use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CacheScope, CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ErrorData,
    ListToolsResult, MetaObject, PaginatedRequestParams, ProtocolVersion, ServerCapabilities,
    ServerConfig, Tool,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::transport::stdio;
use serde_json::{Value, json};

mod bigip;
mod datagroup;
mod diag_meta;
mod environment;
mod fakecmp;
mod irule_gen;
mod irule_test;
mod spec_import;
mod spectcl;
mod tk;
mod tools;
mod xc;

#[derive(Clone)]
struct TclMcp;

impl ServerHandler for TclMcp {
    fn get_info(&self) -> ServerConfig {
        let mut info = ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(instructions());
        // Keep a stable server identity so existing MCP clients see no change.
        "tcl-lsp".clone_into(&mut info.server_info.name);
        // The release version from the tag, not the manifest's 0.1.0.
        tcl_version::VERSION.clone_into(&mut info.server_info.version);
        info
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, ErrorData>> {
        let tools = tools::tool_schemas()
            .into_iter()
            .map(|(name, description, schema)| {
                let object = match schema {
                    Value::Object(map) => map,
                    _ => serde_json::Map::new(),
                };
                Tool::new(name, description, Arc::new(object))
            })
            .collect();
        std::future::ready(Ok(tool_list_result(
            tools,
            context
                .protocol_version()
                .is_some_and(|version| version >= ProtocolVersion::V_2026_07_28),
        )))
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CallToolResponse, ErrorData>> {
        // rmcp 3 widened the `call_tool` return type from `CallToolResult` to
        // the `CallToolResponse` enum, so a handler can also answer
        // "input required" or "task materialised". Every tool here answers
        // synchronously, so they all take the `Complete` arm — which is what
        // `From<CallToolResult>` gives.
        let args = request.arguments.map_or_else(|| json!({}), Value::Object);
        std::future::ready(Ok(tool_result(&request.name, &args).into()))
    }
}

fn tool_result(name: &str, args: &Value) -> CallToolResult {
    let mut result = match tools::run_tool(name, args) {
        Ok(payload) => CallToolResult::success(vec![ContentBlock::text(payload.to_string())]),
        Err(payload) => CallToolResult::error(vec![ContentBlock::text(payload.to_string())]),
    };
    let revision = tcl_version::SOURCE_REVISION;
    let timestamp = env!("TCL_MCP_BUILD_TIMESTAMP")
        .parse::<u64>()
        .expect("build script emits Unix seconds");
    let mut meta = MetaObject::new();
    meta.0.insert(
        "io.github.bitwisecook/tcl-lsp-build".into(),
        json!({
            "builtAtUnixSeconds": timestamp,
            "sourceRevision": (!revision.is_empty()).then_some(revision),
            "version": tcl_version::VERSION,
        }),
    );
    result.meta = Some(meta);
    result
}

/// The server blurb clients show before any tool call — the selectable
/// environments named by their compact `short_name` labels, read from the live
/// registry so a new environment advertises itself here without an edit.
fn instructions() -> String {
    let selectable = tcl_registry::model::selectable_environments();
    let families: Vec<&str> = selectable
        .iter()
        .map(|environment| environment.short_name.as_ref())
        .collect();
    format!(
        "Static analysis of every selectable Tcl dialect ({}): graphs, dataflow/SSA, \
         refactors, optimiser, WASM, dialect detection — served natively over the Rust engine.",
        families.join(", ")
    )
}

fn tool_list_result(tools: Vec<Tool>, supports_cache_hints: bool) -> ListToolsResult {
    let mut result = ListToolsResult::with_all_items(tools);
    if supports_cache_hints {
        // MCP 2026-07-28 requires both cache fields on list results.  The tool
        // catalogue is static and contains no user-specific data, but keep a
        // zero TTL so clients continue to refresh it exactly as they did for
        // older protocol revisions.
        result.ttl_ms = Some(0);
        result.cache_scope = Some(CacheScope::Public);
    }
    result
}

/// Every Tokio worker thread's stack budget — see the matching constant
/// and doc comment in `tcl-lsp-server/src/main.rs` for why this must be
/// generous: the analyser's and CFG builder's depth-capped recursions
/// (256 levels) need more stack than Tokio's 2 MiB worker-thread default
/// provides, and MCP tool handlers run analysis the same way the LSP
/// server does.
const WORKER_STACK_SIZE: usize = 64 * 1024 * 1024;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tcl_registry::install_shipped().expect("the shipped surface set installs");
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(WORKER_STACK_SIZE)
        .build()?
        .block_on(serve())
}

async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    let service = TclMcp.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

#[cfg(test)]
mod protocol_tests {
    use super::*;
    use tcl_dialect::model::EnvironmentRegistry;

    #[test]
    fn instructions_name_every_selectable_environment() {
        let text = instructions();
        for environment in EnvironmentRegistry::compiled_selectable() {
            assert!(text.contains(environment.short_name.as_ref()), "{text}");
        }
        assert!(text.contains("Jim"), "{text}");
    }

    #[test]
    fn tool_list_cache_hints_follow_the_negotiated_protocol() {
        let modern = serde_json::to_value(tool_list_result(Vec::new(), true)).unwrap();
        assert_eq!(modern["resultType"], "complete");
        assert_eq!(modern["ttlMs"], 0);
        assert_eq!(modern["cacheScope"], "public");

        let legacy = serde_json::to_value(tool_list_result(Vec::new(), false)).unwrap();
        assert!(legacy.get("ttlMs").is_none());
        assert!(legacy.get("cacheScope").is_none());
    }

    fn wire_payload(name: &str, args: &Value) -> (bool, Value) {
        let result = serde_json::to_value(tool_result(name, args)).unwrap();
        let is_error = result["isError"].as_bool().unwrap();
        let payload = serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        (is_error, payload)
    }

    #[test]
    fn tool_failures_and_refusals_have_distinct_wire_results() {
        for (name, args) in [
            (
                "analyze",
                json!({"source": "set x 1", "dialect": "klingon"}),
            ),
            ("irule_with_context", json!({"config_paths": ["\u{0}"]})),
            ("hud_chain", json!({"config_paths": ["\u{0}"]})),
        ] {
            let (is_error, payload) = wire_payload(name, &args);
            assert!(is_error, "{payload}");
            assert!(payload["error"].is_string(), "{payload}");
        }

        let (is_error, payload) = wire_payload(
            "if_to_switch",
            &json!({"source": "set x 1", "line": 0, "character": 0}),
        );
        assert!(!is_error, "{payload}");
        assert!(payload["refused"]["reason"].is_string(), "{payload}");
        assert!(payload.get("error").is_none(), "{payload}");
    }

    #[test]
    fn facts_and_diagnostics_share_the_lone_cr_line_model() {
        let args = json!({"source": "set x 1\rproc p {} { set }", "dialect": "tcl9.0"});
        let (_, facts) = wire_payload("symbols", &args);
        let (_, diagnostics) = wire_payload("analyze", &args);
        let proc = facts["symbols"]
            .as_array()
            .unwrap()
            .iter()
            .find(|symbol| symbol["name"] == "p")
            .unwrap();
        let arity = diagnostics["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .find(|finding| matches!(finding["code"].as_str(), Some("E002" | "E003")))
            .unwrap();
        assert_eq!(proc["range"]["start"]["line"], 1, "{facts}");
        assert_eq!(arity["range"]["start"]["line"], 1, "{diagnostics}");
    }

    #[test]
    fn package_release_gates_survive_the_mcp_wire_on_the_pinned_ladder() {
        use tcl_dialect::TclVersion;

        // On the pinned Tcl interpreters, these commands first succeed on
        // 8.5, 8.6 and 9.0 respectively. Tcl 8.4 provides Tcl 8.4 while its
        // patchlevel is 8.4.20. Later interpreters provide their patchlevel.
        let features = [
            (1, "W200", TclVersion::V8_5),
            (2, "W138", TclVersion::V8_6),
            (3, "W138", TclVersion::V9_0),
        ];
        for release in TclVersion::ALL {
            let provided = if release == TclVersion::V8_4 {
                release.version_string()
            } else {
                release.patchlevel()
            };
            let source = format!(
                "package require -exact Tcl {{{provided}}}\n\
                 binary format cu 5\nformat %b 5\nformat %llu 5\n"
            );
            let (is_error, payload) = wire_payload(
                "analyze",
                &json!({"source": source, "dialect": release.dialect_profile_name()}),
            );
            assert!(!is_error, "{release:?}: {payload}");
            let diagnostics = payload["diagnostics"].as_array().unwrap();
            for (line, code, introduced) in features {
                let requirement = format!("requires Tcl {}", introduced.version_string());
                let count = diagnostics
                    .iter()
                    .filter(|finding| {
                        finding["code"] == code
                            && finding["range"]["start"]["line"] == line
                            && finding["message"]
                                .as_str()
                                .is_some_and(|message| message.contains(&requirement))
                    })
                    .count();
                assert_eq!(
                    count,
                    usize::from(release < introduced),
                    "{release:?}: {payload}"
                );
            }
        }
    }

    #[test]
    fn written_tcl_requirements_set_only_their_guaranteed_floor_on_the_wire() {
        for (dialect, requirement, warns) in [
            ("tcl8.4", "-exact Tcl {09.0b1}", true),
            ("tcl8.6", "-exact Tcl {09.0b1}", false),
            ("tcl8.6", r"-exact Tcl 9.\u0030b1", false),
            ("tcl8.6", "Tcl {9.0-9.1} {9.1-}", false),
            ("tcl8.6", "Tcl {9.0-9.1} {8.6}", true),
            ("tcl8.6", "Tcl {9..0}", true),
            ("tcl8.6", "Tcl $version", true),
        ] {
            let source = format!("package require {requirement}\nformat %llu 5\n");
            let (is_error, payload) =
                wire_payload("analyze", &json!({"source": source, "dialect": dialect}));
            assert!(!is_error, "{requirement}: {payload}");
            let diagnostics = payload["diagnostics"].as_array().unwrap();
            let warnings: Vec<_> = diagnostics
                .iter()
                .filter(|finding| {
                    finding["code"] == "W138"
                        && finding["message"]
                            .as_str()
                            .is_some_and(|message| message.contains("requires Tcl 9.0"))
                })
                .collect();
            assert_eq!(
                warnings.len(),
                usize::from(warns),
                "{requirement}: {payload}"
            );
            if let Some(warning) = warnings.first() {
                assert_eq!(
                    warning["range"]["start"]["line"], 1,
                    "{requirement}: {warning}"
                );
            }
        }
    }

    #[test]
    fn tool_results_include_build_provenance() {
        let args = json!({"source": "set answer 42", "dialect": "tcl8.6"});
        let expected = tools::run_tool("analyze", &args).unwrap();
        for (name, is_error, payload) in [
            ("analyze", false, expected),
            ("unknown", true, json!({"error": "Unknown tool: unknown"})),
        ] {
            let result = serde_json::to_value(tool_result(name, &args)).unwrap();
            let stamp = &result["_meta"]["io.github.bitwisecook/tcl-lsp-build"];
            assert_eq!(stamp["version"], tcl_version::VERSION);
            assert_eq!(
                stamp["sourceRevision"],
                json!(
                    (!tcl_version::SOURCE_REVISION.is_empty())
                        .then_some(tcl_version::SOURCE_REVISION)
                )
            );
            assert!(stamp["builtAtUnixSeconds"].is_u64());
            assert_eq!(result["isError"], is_error);
            let text = result["content"][0]["text"].as_str().unwrap();
            assert_eq!(serde_json::from_str::<Value>(text).unwrap(), payload);
        }
    }
}

#[cfg(test)]
tcl_registry::testing::install_shipped!();
