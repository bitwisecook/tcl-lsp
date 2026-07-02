//! Native Rust MCP server for the tcl-lsp analysis engine.
//!
//! Built on the official [`rmcp`] SDK (`JSON-RPC` 2.0 over `stdio`). Every tool
//! handler calls the Rust analysis crates (`tcl-lsp-core`, `tcl-compiler`,
//! `tcl-registry`) **directly** — replacing the former Python
//! `ai/mcp/tcl_mcp_server.py` + `PyO3` bridge. The tool set + wire results match
//! what the Python server produced so existing MCP clients are unaffected.

use std::sync::Arc;

use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ContentBlock, ErrorData, ListToolsResult,
    PaginatedRequestParams, ServerCapabilities, ServerInfo, Tool,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::transport::stdio;
use rmcp::ServiceExt;
use serde_json::{Value, json};

mod tools;

#[derive(Clone)]
struct TclMcp;

impl ServerHandler for TclMcp {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(
                "Tcl/iRules static analysis: graphs, dataflow/SSA, refactors, optimiser, \
                 WASM, dialect detection — served natively over the Rust engine.",
            );
        // Keep the same server identity the Python server advertised so existing
        // MCP clients see no change.
        "tcl-lsp".clone_into(&mut info.server_info.name);
        env!("CARGO_PKG_VERSION").clone_into(&mut info.server_info.version);
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
        let args = request
            .arguments
            .map_or_else(|| json!({}), Value::Object);
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = TclMcp.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
