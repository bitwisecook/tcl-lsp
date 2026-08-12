# Compiler Explorer coverage contract

The Explorer is a presentation of durable compiler artefacts, not a second
compiler. A stage is in scope when it is retained on `CompilationUnit`,
`FunctionUnit`, or a stable compiler-owned analysis result and can explain a
user-visible transformation, diagnostic, or proof. Private work-list locals,
allocator state, memo-cache keys, and backend payloads that are not retained on
the compilation unit are out of scope.

Every in-scope artefact must have a serialised payload, a tab descriptor, and a
renderer in the shared CLI/TUI and web front-ends. The Rust test
`durable_compiler_views_are_present_and_cover_extra_units` is the witness for
the payload/tab half. `reconcileExplorerViews` consumes the same `meta.views`
descriptors in both web shells and supplies a generic JSON renderer for stable
data-shaped views, so a new descriptor cannot silently have no web pane.

## Inventory

| Durable artefact | Explorer surface | Status |
| --- | --- | --- |
| Lowered IR and module CFG | IR, CFG | Included |
| Scalar SSA and phi/uses/defs | SSA | Included |
| Dominators, frontiers, and dominator tree | Dominators | Included by this tranche |
| SCCP lattice, executable blocks/edges, constant branches | SCCP | Included by this tranche |
| Def-use chains and optimiser-authoritative dead stores | Liveness | Included by this tranche |
| Type, interval/range, bounds, rendered-property, full taint lattice, and memory-SSA facts | Existing views, Taint Lattice, Data Flow | Included |
| Interprocedural summaries and caller/unit scope | Interprocedural, Unit Scope | Included |
| Cross-event connection scope summaries and race sets | Connection Scope | Included by this tranche |
| TclOO methods and synthetic body units | Semantic view | Included by this tranche |
| Target-neutral executable IR and registry resolution outcomes | Executable Semantics | Included by this tranche |
| Typed executable/source/proof declines | Executable Semantics | Included by this tranche |
| Optimiser pass records and source rewrites | Pass Pipeline, Optimisations | Included |
| ASM | Tcl ASM | Included |
| World-state SSA | World SSA | Included |
| WASM | WASM | Included |
| Backend selection plans/proofs | WASM `codegenPlan` | Included when the canonical WASM pipeline runs |
