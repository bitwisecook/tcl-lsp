//! iRule (Tcl) structural analysis for reports, the CLI, and the MCP.
//!
//! [`diagram_data`] projects a parsed script's [`tcl_compiler`] IR into a
//! frontend-agnostic `{events, procedures}` structure (decision / action /
//! assign / return / switch / loop nodes). [`irule_flowchart_mermaid`] turns
//! that into a deterministic Mermaid `flowchart` — an offline, no-LLM diagram
//! for the BIG-IP report's iRule view, the CLI, and the MCP.
//!
//! [`attach_reach`] / [`irule_attach_patterns`] reconstruct the object-name
//! patterns (prefix / contained / suffix) an iRule could build for a dynamic
//! `pool` / `node` / `snatpool` attachment, so the report's orphan analysis can
//! filter candidate objects down to only the ones a rule could actually reach.
//!
//! This lives in its own crate (not `tcl-lsp-core`): it depends only on the
//! compiler + lexer + registry, so any tool — including the wasm report
//! generator — can use it without pulling in the LSP feature layer.

mod attach;
mod data;
mod mermaid;

pub use attach::{AttachPattern, AttachReach, attach_reach, irule_attach_patterns};
pub use data::diagram_data;
pub use mermaid::irule_flowchart_mermaid;
