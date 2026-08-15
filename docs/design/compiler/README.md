# Compiler design docs

Focused, high-churn compiler design docs live in this folder. Each file
describes one piece of the pipeline — how it works, what it consumes and
produces, and the contracts that downstream consumers depend on.
Technical jargon is allowed here.

User-facing compiler troubleshooting and how-tos live in
[`docs/kcs/`](../../kcs/README.md).

## Start here

- [common-semantic-compiler.md](common-semantic-compiler.md) — implementation
  contract for the shared semantic IR, value/cell/world SSA, registry
  boundaries, exact completion and trace flow, target-family lowering, and
  current backend contracts.
- [semantic-aot-optimisation.md](semantic-aot-optimisation.md) — default-off
  contract for guarded semantic AOT passes, mixed plans, materialisation,
  native numeric lowering, and Tcl's dynamic dispatch surfaces.
- [compiler-pipeline-overview.md](compiler-pipeline-overview.md) — stage
  map and fact hand-off boundaries.
- [compiler-systems-overview.md](compiler-systems-overview.md) —
  subsystem contract map for quick ownership triage.
- [algorithms.md](algorithms.md) — the classic algorithms the pipeline uses
  (SSA, dominators, SCCP, semi-pruned SSA, interval abstract interpretation,
  GVN, slot allocation, worklist dataflow), their **verified** original
  references, and how each is adapted for Tcl's dynamism.
- [precision-limitations.md](precision-limitations.md) — where the compiler is
  deliberately, or knowingly, less precise than it could be: what is imprecise,
  why it matters, and where the code is.
- [fp-sweep.md](fp-sweep.md) — the `cargo xtask fp-sweep` false-positive audit
  harness: what it runs, why it is dialect-aware, how firings are grouped, and
  where the paired regression tests live.
- [command-oracle-audits.md](command-oracle-audits.md) — per-command Tcl
  oracle queue, evidence availability, and registry verdicts.

## Pipeline stages

- [lexing-segmentation.md](lexing-segmentation.md) — token and command
  segmentation.
- [green-token-tree.md](green-token-tree.md) — **proposal** for a lossless
  token tree, error nodes, and incremental reparse.
- [syntax-tree.md](syntax-tree.md) — the canonical red-green concrete syntax
  tree (lossless, position-independent); the segmenter's byte-identical
  backing and the foundation the formatter, minifier, AOT lowering, and
  per-command tooling are migrating onto.
- [expression-parsing.md](expression-parsing.md) — Pratt parser, braced
  and unbraced expressions.
- [cfg-construction.md](cfg-construction.md) — basic block
  decomposition patterns.
- [ssa-construction.md](ssa-construction.md) — version numbering and
  phi placement.
- [ir-types-lowering.md](ir-types-lowering.md) — IR node selection
  rules.
- [lowering-dispatch.md](lowering-dispatch.md) — argument roles and
  command classification.
- [full-pipeline-walkthrough.md](full-pipeline-walkthrough.md) —
  end-to-end source to bytecode walkthrough.
- [control-flow-patterns.md](control-flow-patterns.md) — if, while,
  for, foreach, and proc compilation.
- [error-recovery.md](error-recovery.md) — ghost delimiter injection for
  malformed input: the recovery loop, the E20x heuristics, the structural-state
  index that vetoes inert insertion points, and the `ArgRole` router.

## Analysis

- [sccp-core-analyses.md](sccp-core-analyses.md) — constant propagation
  and liveness.
- [constant-folding-type-inference.md](constant-folding-type-inference.md)
  — SCCP and type lattice.
- [type-tracking.md](type-tracking.md) — the comprehensive value-type model
  (purity / first-use commitment, union nodes, container element types, the
  numeric tower) with its oracle corpus and phasing.
- [def-use-chains.md](def-use-chains.md) — def-use chain construction
  and consumer contracts.
- [memory-ssa.md](memory-ssa.md) — memory-SSA, alias detection, and
  versioned memory operations.
- [dataflow-graph.md](dataflow-graph.md) — data-flow graph extraction,
  serialisation, and consumer contracts.
- [rendered-value-properties.md](rendered-value-properties.md) — string
  content analysis over SSA.
- [byte-array-corruption.md](byte-array-corruption.md) — the S110
  byte-array-corruption correctness check (binary data forced through
  character-string semantics).
- [taint-analysis.md](taint-analysis.md) — sources, sinks, colours, and
  propagation.
- [var-escape-analysis.md](var-escape-analysis.md) — which Tcl vars stay
  on WASM locals vs spill to the runtime frame.
- [interprocedural-analysis.md](interprocedural-analysis.md) —
  ProcSummary construction.
- [frame-effect-summaries.md](frame-effect-summaries.md) — the per-proc
  caller-frame (`upvar`/`uplevel`) and global-frame (`uplevel #0`)
  effect summaries, their named/opaque/empty lattice, the method-dispatch
  widening, and the conservative limits.
- [interprocedural-call-site-seeding.md](interprocedural-call-site-seeding.md)
  — how a procedure parameter is bound to a caller-uniform literal, which
  indirect calls (`$cmd args`, callback prefixes, `eval $script`) count as
  call sites, and what withdraws the seed module-wide.
- [optimisation-passes.md](optimisation-passes.md) — pass table and
  priorities.

## Infrastructure

- [command-registry.md](command-registry.md) — command metadata, specs,
  arity, and taint hints.
- [data-structure-reference.md](data-structure-reference.md) — pipeline
  types at each stage.
- [connection-scope.md](connection-scope.md) — cross-event variable
  flow in iRules.
- [dialects-events.md](dialects-events.md) — dialect filtering and
  event requirements.
- [event-priority-model.md](event-priority-model.md) — base priority
  and offset model for event handlers.
- [namespace-resolution.md](namespace-resolution.md) — qualified name
  handling.
- [diagnostics-calculation.md](diagnostics-calculation.md) — two-phase
  diagnostic architecture.
- [codegen-internals.md](codegen-internals.md) — LVT, linearisation,
  labels, and peephole optimisation.
- [wasm-codegen.md](wasm-codegen.md) — shared semantic-to-WASM boundary,
  executable-IR generic argv transport, typed semantic declines, the single
  Rust emitter, and the shared runtime ABI.
- [wasm-extensions.md](wasm-extensions.md) — current embedded-script boundary
  and the explicitly future package-driven extension design.
- [ebpf-backend.md](ebpf-backend.md) — BPF-Tcl layering, typed core and BPF-IR,
  current `rbpf` codegen ABI, event/framework capabilities, verified design
  issues, real-world use cases, and the production-kernel roadmap.
- [recursive-descent-depth-limits.md](recursive-descent-depth-limits.md) —
  why deeply-nested Tcl source could crash the analyser (issue #996): the
  depth-cap + generous-stack-budget model every recursive-descent walker
  needs, the inventory of guarded walkers, and the known gaps.

## Side-effects and effect classification

- [side-effects-system.md](side-effects-system.md) — structured
  side-effect hints, classification flow, and how to add hints to
  commands.

## Pipeline contracts

- [lowering-contracts.md](lowering-contracts.md) — lowering guarantees
  consumed by CFG, SSA, and codegen.
- [cfg-ssa-fact-model.md](cfg-ssa-fact-model.md) — core fact model and
  consumption rules.
- [compilation-unit-contracts.md](compilation-unit-contracts.md) —
  compilation unit orchestration and incremental cache expectations.
- [compilation-unit-scope.md](compilation-unit-scope.md) — when a fact
  derived from one file's call sites may be trusted as a fact about every
  caller: cross-file evidence, registry-declared unit boundaries, and the
  interprocedural constant seed's gate.
- [object-type-lattice.md](object-type-lattice.md) — the object-handle →
  class carrier (`ObjectHandleFacts`): the four maps that answer
  "what class does `$obj` hold?", owner attribution per VTA edge, the
  `by_scope` vs `any_scope` soundness directions each consumer must read,
  and the empty-seed fast path's three-part gate.

## Optimisation passes

- [tail-call-recursion-optimisation.md](tail-call-recursion-optimisation.md)
  — tail-call rewriting, recursion-to-loop, and accumulator hints
  (O121–O123).
- [dispatch-stability-proof.md](dispatch-stability-proof.md) — the world-state
  contents/absence lattice and the typed per-site proof that gates stable-call
  CSE (`O105`): the tracks and ledgers, the transfer functions, the
  `DispatchEntryAssumption` entry contract, and the conservative abstentions.
- [optimiser-o124-unused-irule-procs.md](optimiser-o124-unused-irule-procs.md)
  — O124 comment out unused procs in iRules.
- [o125-code-sinking.md](o125-code-sinking.md) — O125 code sinking into
  decision blocks.

## Diagnostics and pass integration

- [pass-fact-ownership-matrix.md](pass-fact-ownership-matrix.md) —
  producer and consumer ownership map for core compiler facts.
- [downstream-pass-contracts.md](downstream-pass-contracts.md) — pass
  ownership, typed finding contracts, and overlap rules.
- [diagnostics-integration.md](diagnostics-integration.md) — aggregation
  and suppression policy boundary.
- [async-diagnostics-tiering.md](async-diagnostics-tiering.md) —
  fast/deep tiering and cancellation expectations.

## Codegen boundary

- [bytecode-boundary.md](bytecode-boundary.md) — what stays in codegen
  and what should move earlier.
- [codegen-module-map.md](codegen-module-map.md) — package module map
  and ownership boundaries.
- [wasm-runtime-primitives.md](wasm-runtime-primitives.md) — Rust
  runtime ABI at the compiler-to-interpreter boundary.
- [wasm-extensions.md](wasm-extensions.md) — the `wasm_stdlib` Cargo feature
  and how the Rust runtime embeds optional Tcl scripts.
- [wasm-target-surfaces.md](wasm-target-surfaces.md) — WASI vs in-browser
  WASM: the capability matrix, browser-target build/wiring gaps, the
  proposed host-import surface, and measured module sizes.
- [aot-command-priority.md](aot-command-priority.md) — real-corpus census
  (issue #1181) ranking which Tcl commands the AOT WASM compiler should
  emit directly next, with a breadth-weighted tiering and what is already
  covered versus what cannot be direct.

## Related KCS how-tos

- [How do I add a compiler pass?](../../kcs/kcs-howto-add-compiler-pass.md)
- [How do I add a command-registry package?](../../kcs/kcs-howto-add-command-registry-package.md)
- [How does array-element SSA typing work?](../../kcs/kcs-howto-array-element-ssa-typing.md)
- [How do I suppress a diagnostic?](../../kcs/kcs-howto-suppress-diagnostics.md)
