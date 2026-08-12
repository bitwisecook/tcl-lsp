# Data-Flow Graph

## Overview

The data-flow graph extracts structured def-use and alias information
from compiled Tcl and exposes it for visualisation, AI tooling, and
CLI exploration.  It is built on top of def-use chains and memory-SSA.

## Problem

Downstream consumers (compiler explorer, MCP tools, AI skills) need
a unified view of data flow within functions — definitions, uses,
phi merges, and alias relationships — in a format suitable for
serialisation (JSON) and diagram generation (Mermaid).

## Graph Model

```rust
pub struct DataFlowGraph {
    pub functions: Vec<FunctionDataFlowGraph>,
}

pub struct FunctionDataFlowGraph {
    pub function_name: String,
    pub nodes: Vec<DataFlowNode>,   // SSA value definitions
    pub edges: Vec<DataFlowEdge>,   // def→use relationships
    pub aliases: Vec<AliasInfo>,    // alias pairs from memory-SSA
    pub total_defs: u32,
    pub total_uses: u32,
    pub dead_defs: u32,
    pub aliased_vars: u32,
}

pub struct DataFlowNode {
    pub name: String,           // variable name
    pub version: u32,           // SSA version
    pub block: String,          // defining block
    pub def_kind: String,       // "statement", "phi", "parameter"
    pub statement_index: i32,
    pub lattice: String,        // e.g. "CONST(42)", "OVERDEFINED"
    pub type_info: String,      // e.g. "INT", "STRING"
    pub is_dead: bool,          // no uses
    pub use_count: u32,
}

pub struct DataFlowEdge {
    pub from_name: String,      // source SSA value
    pub from_version: u32,
    pub to_block: String,       // destination site
    pub to_statement_index: i32,
    pub edge_kind: EdgeKind,    // Direct | Phi | Alias | Clobber
    pub to_name: String,
    pub to_version: i32,
}

pub struct AliasInfo {
    pub local_name: String,     // e.g. "local_x"
    pub local_kind: String,     // e.g. "UPVAR"
    pub target_name: String,    // e.g. "caller_x"
    pub target_kind: String,
    pub reason: String,         // "upvar", "global", "variable"
}
```

## Serialisation Formats

- **`dataflow_graph_to_dict()`** — JSON-serialisable dict for the
  compiler explorer and MCP tool responses.
- **`dataflow_graph_to_mermaid()`** — Mermaid flowchart string for
  AI skills and documentation.  Node IDs are sanitised to
  alphanumerics and underscores.

## How It Consumes Def-Use Chains and Memory-SSA

1. **Nodes** are built from `DefUseResult.chains` — one node per
   SSA definition, annotated with lattice value and type from
   `FunctionAnalysis`.
2. **Edges** are built from `DefUseChain.uses` — phi incoming edges
   become "phi" edges, statement operands become "direct" edges.
3. **Aliases** are built from `MemorySSAFunction.alias_sets` — each
   alias set becomes an `AliasInfo` with deterministic field assignment
   (locations are sorted by kind, name, qualifier before assigning
   `local_name` and `target_name`).

## Consumer Contracts

| Consumer | Entry Point | Format |
|----------|-------------|--------|
| Compiler Explorer | `rust/tcl-explorer/src/serialise.rs` | Structured JSON payload |
| MCP tools | Native MCP compiler consumers | Registry/compiler facts |
| CLI | `rust/tcl-cli` Explorer command | Shared Explorer payload |

## Module Location

- **Source**: `rust/tcl-compiler/src/dataflow_graph.rs` and
  `rust/tcl-explorer/src/serialise.rs`.
- **Entry point**: the retained `CompilationUnit` dataflow facts consumed by
  the Explorer serialiser.
- **Tests**: Rust compiler and `tcl-explorer` crate tests.
