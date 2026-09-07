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

| Durable artefact | Explorer surface |
| --- | --- |
| Lowered IR and module CFG | IR, CFG |
| Scalar SSA and phi/uses/defs | SSA |
| Dominators, frontiers, and dominator tree | Dominators |
| SCCP lattice, executable blocks/edges, constant branches | SCCP |
| Def-use chains and optimiser-authoritative dead stores | Liveness |
| Type, interval/range, bounds, rendered-property, full taint lattice, and memory-SSA facts | Existing views, Taint Lattice, Data Flow |
| Interprocedural summaries and caller/unit scope | Interprocedural, Unit Scope |
| Cross-event connection scope summaries and race sets | Connection Scope |
| TclOO methods and synthetic body units | Semantic view |
| Target-neutral executable IR and registry resolution outcomes | Executable Semantics |
| Typed executable/source/proof declines | Executable Semantics |
| Optimiser pass records and source rewrites | Pass Pipeline, Optimisations |
| ASM | Tcl ASM |
| World-state SSA | World SSA |
| WASM | WASM |
| Backend selection plans/proofs | WASM `codegenPlan` (when the canonical WASM pipeline runs) |

## Native editor panes are additive

An editor host may also render a tree view into one of its own editor panes,
through the `tcl-lsp.compilerExplorerView` command: it returns
`{text, lines}` from `tcl_explorer::render::render_view_mapped` — the same
renderer `tcl explore --text` and the TUI use — where `lines[n]` is the source
span the pane's line `n` points at, so a caret in the pane navigates back into
the user's file.

This never *replaces* a web pane. The rule above stands: every in-scope
artefact keeps its renderer in both web shells. Three panes could not be a flat
buffer anyway — the CFG's lane-routed edge overlay, the WASM branch-lane
gutter, and the optimiser diff's bracket connectors are drawn, not written.

`render_view_mapped` is the only producer of the line map, and it appends to
the text and the map in the same place, so the two cannot drift; the alignment
is asserted per view in `render.rs`'s tests.
