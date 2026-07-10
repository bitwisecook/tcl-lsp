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
//! `tcl-registry`) **directly**. The tool set + wire results are kept stable so
//! existing MCP clients are unaffected.

use std::sync::Arc;

use rmcp::ServiceExt;
use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ContentBlock, ErrorData, ListToolsResult,
    PaginatedRequestParams, ServerCapabilities, ServerInfo, Tool,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::transport::stdio;
use serde_json::{Value, json};

mod bigip;
mod datagroup;
mod diag_meta;
mod fakecmp;
mod irule_gen;
mod irule_test;
mod tk;
mod tools;
mod xc;

#[derive(Clone)]
struct TclMcp;

impl ServerHandler for TclMcp {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(
                "Tcl/iRules static analysis: graphs, dataflow/SSA, refactors, optimiser, \
                 WASM, dialect detection — served natively over the Rust engine.",
            );
        // Keep a stable server identity so existing MCP clients see no change.
        "tcl-lsp".clone_into(&mut info.server_info.name);
        // The release version from the tag, not the manifest's 0.1.0.
        tcl_version::VERSION.clone_into(&mut info.server_info.version);
        info
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
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
        Ok(ListToolsResult {
            tools,
            ..Default::default()
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let args = request.arguments.map_or_else(|| json!({}), Value::Object);
        match tools::dispatch(&request.name, &args) {
            Some(result) => Ok(CallToolResult::success(vec![ContentBlock::text(
                result.to_string(),
            )])),
            None => Ok(CallToolResult::error(vec![ContentBlock::text(
                json!({ "error": format!("Unknown tool: {}", request.name) }).to_string(),
            )])),
        }
    }
}

/// `--help` / `--version`, handled before the stdio transport takes over.
///
/// Without this the binary reads `initialize` from stdin the moment it starts,
/// so `tcl-mcp --help` answered `ConnectionClosed("initialize request")` on a
/// closed pipe. An MCP server is spawned by its client and speaks JSON-RPC on
/// stdio, so it has no CLI to speak of — but a released binary should still be
/// able to say what it is and which version it is.
///
/// Returns `true` when a flag was handled and the caller should exit.
fn handle_cli_flags() -> bool {
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                println!(
                    "tcl-lsp MCP server {version}\n\
                     \n\
                     Tcl/iRules static analysis served over the Model Context Protocol:\n\
                     graphs, dataflow/SSA, refactors, optimiser, WASM, dialect detection.\n\
                     \n\
                     Speaks JSON-RPC on stdio and is normally spawned by an MCP client\n\
                     (Claude Code, Zed, the VS Code extension) rather than run by hand.\n\
                     \n\
                     Options:\n\
                     \x20 -h, --help       Print this help and exit\n\
                     \x20 -V, --version    Print the version and exit",
                    version = tcl_version::VERSION,
                );
                return true;
            }
            "-V" | "--version" => {
                println!("tcl-mcp {}", tcl_version::VERSION);
                return true;
            }
            _ => {}
        }
    }
    false
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if handle_cli_flags() {
        return Ok(());
    }
    let service = TclMcp.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
