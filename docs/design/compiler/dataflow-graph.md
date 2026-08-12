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

```
DataFlowGraph
  functions: list[FunctionDataFlowGraph]

FunctionDataFlowGraph
  function_name: str
  nodes: list[DataFlowNode]       # SSA value definitions
  edges: list[DataFlowEdge]       # def→use relationships
  aliases: list[AliasInfo]        # alias pairs from memory-SSA

DataFlowNode
  name: str                       # variable name
  version: int                    # SSA version
  block: str                      # defining block
  def_kind: str                   # "statement", "phi", "parameter"
  lattice: str                    # e.g. "CONST(42)", "OVERDEFINED"
  type_info: str                  # e.g. "INT", "STRING"
  is_dead: bool                   # no uses
  use_count: int

DataFlowEdge
  from_name / from_version        # source SSA value
  to_block / to_statement_index   # destination site
  edge_kind: EdgeKind              # EdgeKind.DIRECT, .PHI, .ALIAS, .CLOBBER

AliasInfo
  local_name / local_kind         # e.g. "local_x" / "UPVAR"
  target_name / target_kind       # e.g. "caller_x" / "UPVAR"
  reason: str                     # "upvar", "global", "variable"
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
