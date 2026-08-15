# Data-Flow Graph

## Overview

The data-flow graph extracts structured def-use and alias information
from compiled Tcl and exposes it for visualisation, AI tooling, and
CLI exploration.  It is built on top of def-use chains and memory-SSA.

Memory-SSA is optional, so the alias half is only populated when the caller
ran `with_memory_ssa` first — which both production callers
(`tcl-explorer/src/lib.rs`, `tcl-lsp-core/src/graphs.rs`) do.

## Problem

Downstream consumers (the compiler explorer, the `tcl dataflow` CLI verb,
the LSP graph request) need a unified view of data flow within functions —
definitions, uses, phi merges, and alias relationships — in a shape that
serialises straightforwardly to JSON.

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
    pub lattice: String,        // "CONST(42)", "CONSTSET(1,2)", "OVERDEFINED",
                                // "UNKNOWN", or "" when no entry
    pub type_info: String,      // the type-lattice *kind*: "UNKNOWN" | "KNOWN"
                                // | "SHIMMERED" | "OVERDEFINED", or ""
    pub is_dead: bool,          // no uses
    pub use_count: u32,
}

pub struct DataFlowEdge {
    pub from_name: String,      // source SSA value
    pub from_version: u32,
    pub to_block: String,       // destination site
    pub to_statement_index: i32, // -1 for phi-incoming and terminator uses
    pub edge_kind: EdgeKind,    // Direct | Phi
    pub to_name: String,        // phi edges only
    pub to_version: i32,        // phi edges only; -1 otherwise
}

pub struct AliasInfo {
    pub local_name: String,     // e.g. "local_x"
    pub local_kind: String,     // the `MemoryLocationKind` Debug form,
                                // e.g. "Local", "Upvar"
    pub target_name: String,    // e.g. "caller_x"
    pub target_kind: String,    // e.g. "Global", "NamespaceVar"
    pub reason: String,         // the memory-SSA reason, e.g. "caller-frame-cell"
}
```

`extract_function_dataflow` emits `Phi` for a `UseKind::PhiIncoming` use and
`Direct` for every other use, including terminator-condition uses. Alias and
clobber information is represented separately by `AliasInfo` and Memory-SSA
operations, rather than as scalar def-use edges.

## Serialisation

The graph types are plain Rust structs with no serialisation of their own.
Each consumer projects them:

- `tcl-lsp-core/src/graphs.rs` — `function_dataflow_json` renders the
  camelCase wire dict (`defKind`, `statementIndex`, `typeInfo`, `fromName`,
  `edgeKind`, …) that the `tcl dataflow` CLI verb and the LSP graph request
  return.
- `tcl-explorer/src/serialise.rs` — `serialise_dataflow` builds the
  explorer's `dataflow` view payload from `extract_function_dataflow`.

There is no Mermaid renderer for this graph.

## How It Consumes Def-Use Chains and Memory-SSA

1. **Nodes** are built from `DefUseResult.chains` — one node per
   SSA definition, in sorted key order (name, then version) so the output is
   deterministic.  Each is annotated with lattice value and type from the
   `FunctionUnit` (`rust/tcl-compiler/src/compilation_unit.rs`, built by
   `CompilationUnit::build_for()`): the lattice value comes from
   `SccpResult.values` (the return of `sccp()`, carried as
   `FunctionUnit.sccp`) and the type from `FunctionUnit.types`.  Both are
   `Option` arguments to `extract_function_dataflow()`, so a node renders
   without them when those analyses have not run.

   Those two maps are keyed by `ssa::ValueKey` (interned `Symbol`), while the
   chains are keyed by name, so the extractor resolves each chain's name back
   through `SsaFunction::var_symbol`.  A name with no SSA symbol yields an
   empty display column rather than a wrong one.
2. **Edges** are built from `DefUseChain.uses` — a `UseKind::PhiIncoming` use
   becomes a `Phi` edge carrying the receiving phi's name and version;
   every other use, including a terminator-condition read, becomes a
   `Direct` edge with `to_statement_index` from the use site.
3. **Aliases** are built from `MemorySsaFunction.alias_sets` — each
   alias set of two or more locations becomes an `AliasInfo`, with the
   locations sorted by `(kind, name, qualifier)` first so `local_name` and
   `target_name` are assigned deterministically.  A one-location set
   contributes nothing.

`extract_dataflow_graph` merges per-function results from a
`FunctionInputs` slice into a module-level `DataFlowGraph`, ordered `::top`
first and then alphabetically, so the whole payload is byte-comparable
across runs.

## Consumer Contracts

| Consumer | Entry point | Format |
|----------|-------------|--------|
| Compiler Explorer `dataflow` view | `serialise_dataflow` (`rust/tcl-explorer/src/serialise.rs`) | Explorer JSON payload |
| `tcl dataflow` CLI verb (alias `dataflow-graph`) and the LSP graph request | `dataflow_graph` (`rust/tcl-lsp-core/src/graphs.rs`) | camelCase wire dict |

## Module Location

- **Source**: `rust/tcl-compiler/src/dataflow_graph.rs`
  (`extract_function_dataflow`, `extract_dataflow_graph`).
- **Projections**: `rust/tcl-explorer/src/serialise.rs`,
  `rust/tcl-lsp-core/src/graphs.rs`.
- **Tests**: `rust/tcl-compiler/tests/dataflow.rs` plus the in-module tests
  in `dataflow_graph.rs` and the `tcl-explorer` crate tests.
