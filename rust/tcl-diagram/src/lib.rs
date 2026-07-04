//! Control-flow diagram extraction from the Tcl / iRules IR.
//!
//! [`diagram_data`] projects a parsed script's [`tcl_compiler`] IR into a
//! frontend-agnostic `{events, procedures}` structure (decision / action /
//! assign / return / switch / loop nodes). [`irule_flowchart_mermaid`] turns
//! that into a deterministic Mermaid `flowchart` — an offline, no-LLM diagram
//! for the BIG-IP report's iRule view, the CLI, and the MCP.
//!
//! This lives in its own crate (not `tcl-lsp-core`): it depends only on the
//! compiler + registry, so any tool — including the wasm report generator — can
//! use it without pulling in the LSP feature layer.

mod data;
mod mermaid;

pub use data::diagram_data;
pub use mermaid::irule_flowchart_mermaid;
