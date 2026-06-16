# Design documentation

This folder is the home for technical documentation about how tcl-lsp is
built: architecture, contracts, interfaces, data-structure references,
pipeline internals, and pass/fact ownership. Technical jargon and
specialist terms are allowed here — design docs describe how the system
is structured and why, and assume the reader can read the code.

If you are writing a user-facing answer, a how-to, a Q&A, or a feature
description, it belongs in [`docs/kcs/`](../kcs/README.md) instead. The
rules for the KCS/documentation split live in
[`AGENTS.md`](../../AGENTS.md) under "Knowledge base and documentation".

## Architecture and walkthroughs

- [compiler-architecture.md](compiler-architecture.md) — high-level map of
  the multi-pass compiler pipeline with diagrams and cross-links.
- [common-runtime-emitter-architecture.md](common-runtime-emitter-architecture.md)
  — steering doc reasoning across the whole space (TCLVM bytecode emitter, WASM
  emitter, and the runtimes/VM) to fix the right split and interface shapes: the
  two interface families (emitter vs runtime-state), why the bytecode VM is a
  reified-state runtime, and the WASM migration path (WASM engine untouched).
- [example-script-walkthroughs.md](example-script-walkthroughs.md) — full
  pipeline traces for progressively complex Tcl scripts.
- [code-importing-examples.md](code-importing-examples.md) — reference
  patterns for Tcl code importing (package require, sourcing).
- [incremental-analysis-worker.md](incremental-analysis-worker.md) — the
  persistent per-document analysis worker model: bounded pool + per-uri
  single-writer lock for incremental edits, process pool + serialized
  warm-start seed for the cold build.

> Past project-tracking documents (perf reports, phase trackers,
> migration plans) are kept in [`../archive/`](../archive/) and are
> not part of the current design surface.

## F5 BIG-IP CLI

- [f5-cli-architecture.md](f5-cli-architecture.md) — verb registry,
  reference graph, IP-redaction model, tmsh emitter, file layout, and
  the recipe for adding a new verb.
- [f5-query-engine-internals.md](f5-query-engine-internals.md) —
  internals of the `f5 query` engine: module layout, pipeline,
  invariants, edit-plan apply order, builtin registration,
  extension points.  User-facing reference (grammar, every
  builtin, sample configs, F5 KB cross-references) lives at
  [`docs/references/f5_query/`](../references/f5_query/); the
  alphabetical builtin catalogue there is generated from the
  registry by
  [`scripts/dev/gen_query_builtins_doc.py`](../../scripts/dev/gen_query_builtins_doc.py)
  and asserted up-to-date by CI.
- [bigip-registry-architecture.md](bigip-registry-architecture.md) —
  registry contract for object kinds, value specs (parse / project
  / render / references), source-range fidelity, and the pilot
  migration table that opts properties into the typed dispatch.
- [bigip-list-operator-audit.md](bigip-list-operator-audit.md) —
  every list-valued property without ``list_operators``, classified
  by emission style (real list / sub-block / uncertain), backing
  the curated override layer in
  ``dialects/f5/bigip/registry/specs/_base.py``.
- [f5-query-renderer-contract.md](f5-query-renderer-contract.md) —
  decorator-based renderer plugin registry that powers
  ``f5 q --render NAME``: ``RendererSpec`` shape, source-text
  recovery via ``RENDER_SOURCES`` contextvar, error-mapping rules,
  and CLI / Python API wiring.

## tclpkg package manager

- [tclpkg-architecture.md](tclpkg-architecture.md) — architecture overview,
  contracts, file-path anchors, test anchors.
- [contracts/tclpkg-manifest.md](contracts/tclpkg-manifest.md) — manifest
  directives, safe-mode whitelist, validation rules.
- [contracts/tclpkg-lockfile.md](contracts/tclpkg-lockfile.md) — canonical
  JSON, determinism contract, schema versioning.
- [contracts/tclpkg-resolver.md](contracts/tclpkg-resolver.md) — MVS
  algorithm, replace/exclude semantics.
- [contracts/tclpkg-cache.md](contracts/tclpkg-cache.md) — CAS layout,
  hash canonicalisation, integrity verification.
- [contracts/tclpkg-venv.md](contracts/tclpkg-venv.md) — virtual
  environment layout, activation scripts, tclsh wrapper.
- [contracts/tclpkg-lsp.md](contracts/tclpkg-lsp.md) — project root
  detection, W130–W134 diagnostics, code actions.

## Compiler internals

See [compiler/README.md](compiler/README.md) for the compiler design-doc
index — pipeline stages, analyses, codegen, optimisation passes, and
ownership matrices.

## Runtime internals

- [runtime/namespace-tree.md](runtime/namespace-tree.md) — design for
  the Zig runtime's namespace tree (root, child links, per-ns
  command/variable/path tables) modelled on Tcl 9's `Namespace`
  struct, with per-phase migration plan from the FQN-string
  fallbacks currently in `tcl_procs.zig` / `tcl_ns.zig` (globals
  were folded into `tcl_ns.zig` in P3.4; `tcl_globals.zig` no longer
  exists).
- [runtime/rename-alias.md](runtime/rename-alias.md) — layout + flow
  for `rename` and single-interp `interp alias`, layered on top of the
  namespace tree.  Covers `CMD_ALIAS` flag, `AliasRec`, dispatch
  trampoline, and the compiled-proc name-slot preservation
  caveat.
- [runtime/command-introspection.md](runtime/command-introspection.md)
  — interpreter-wide hidden-commands table, `interp hide` / `expose`
  semantics, the `OFF_EXPORT_NAME_BUCKET` sidecar that unblocks
  compiled-proc rename, and the `info commands` / `info procs` /
  `namespace which -command` walkers.
- [runtime/child-interp.md](runtime/child-interp.md) — child-
  interpreter primitives (`interp create` / `eval` / `exists` /
  `slaves` / `delete`), the `Interp` struct + per-interp hidden
  table, the `enter` / `leave` swap pair for nested eval, and the
  compiler's conservative proc-index flush on `interp create` /
  `eval` / `delete`.
- [runtime/memory-management.md](runtime/memory-management.md) —
  TclObj refcount discipline, `OBJ_STR_CAP` ownership, the
  deferred-free queue, parse-cache invalidation, and the
  bump-allocator → libc-malloc routing rationale.
- [runtime/refcount-contract.md](runtime/refcount-contract.md) —
  ownership categories for every WASM-exported runtime function
  (callee-takes / caller-keeps / borrow), the linter that
  enforces them, and the decision rules for new exports.
- [runtime/leak-sweep-trap-triage.md](runtime/leak-sweep-trap-triage.md) —
  triage clusters for the 29 trapping tcltest files in the leak-
  sweep baseline, suggested order of attack, and the structured
  ``trap_origin`` enrichment that would unblock deeper analysis.
- [runtime/trace-implementation.md](runtime/trace-implementation.md) —
  current ``trace add variable`` no-op gap, the design sketch
  (per-Var TraceList + fire hooks on every mutator), and an
  effort estimate for closing it.
- [runtime/proc-call-and-stack-traces.md](runtime/proc-call-and-stack-traces.md)
  — the proc call protocol: argument binding, exception propagation,
  and ``-errorinfo`` / ``-errorcode`` stack-trace assembly across the
  call stack.
- [runtime/c-api-ownership-contract.md](runtime/c-api-ownership-contract.md)
  — the C Tcl API the runtime ships (``tcl.h`` / ``tclOO.h`` /
  ``tclTomMath.h``) and the refcount-ownership / error-return contract
  every exported C entry point must honour.
- [runtime/c-extension-abi.md](runtime/c-extension-abi.md) — the
  C-Tcl-extension → WASM ABI: how an unmodified C extension is compiled
  and linked against the runtime, with the spike that validated the
  mechanism end-to-end.
- [runtime/tcltest-bringup.md](runtime/tcltest-bringup.md) — the plan for
  running the real Tcl library and ``tcltest`` harness (the
  ``pkgIndex.tcl`` / ``tm`` machinery) against real C Tcl 9 under the
  runtime.
- [runtime/zig-runtime-roadmap.md](runtime/zig-runtime-roadmap.md) —
  phases 4 (residual), 5, and 6 of the Zig WASM runtime, companion to
  the master plan: remaining P4 work and P5/P6 sequencing.

## Rust runtime port

The Python pipeline's port to a native Rust workspace — current state,
where it is headed, and whether the gap is bridgeable. The companion
chunk-by-chunk dispatch story lives in
[`docs/rust-rewrite.md`](../rust-rewrite.md).

- [rust/current-architecture.md](rust/current-architecture.md) — snapshot
  of the Rust workspace as it stands today: crate layout and data flow
  through the native pipeline.
- [rust/target-architecture.md](rust/target-architecture.md) — the target
  design (zero-copy, single-parse, incremental-reuse, MVCC) and how the
  competing goals are reconciled.
- [rust/feasibility.md](rust/feasibility.md) — feasibility analysis:
  whether the target is reachable from the current architecture across
  all six design goals.
- [rust/review-findings.md](rust/review-findings.md) — workspace review
  findings on correctness, performance, and memory, including the
  ``unsafe``-forbidden discipline and where it costs.
- [runtime/rust-runtime-port.md](runtime/rust-runtime-port.md) —
  productionising the C-Tcl-extension-to-WASM port on the Rust runtime:
  status, bootstrapping plan, and the end-to-end build mechanism.

## Optional WASM extensions

- [compiler/wasm-extensions.md](compiler/wasm-extensions.md) —
  contract for shipping optional runtime features the user's
  program requests via ``package require``. Build-flag variant
  runtimes today; deferred Stage 2 plan for separately-merged
  extension WASMs. Includes the file layout for the in-tree
  ``runtime/zig/tcltest/`` port (every ~107 upstream tcltest
  command registered, PORTABLE/PARTIAL ones implemented and
  NOT-PORTABLE ones stubbed with explicit error messages).

## Compiler staircase (S0–S6)

The phased plan to drive the Tcl-WASM AOT compiler from "frames
everywhere" baseline through inlining and SSA-driven optimisations.
Each stage doc lists tasks, file paths, test plans, and acceptance
gates.

- [compiler/wasm-aot-staircase.md](compiler/wasm-aot-staircase.md)
  — overview tying S0 through S6 together: stage status,
  acceptance gates, sequencing rules.
- [compiler/wasm-aot-staircase-s0.md](compiler/wasm-aot-staircase-s0.md)
  — S0 foundations: leak detector, refcount contract,
  deterministic repro for the canonical bug.
- [compiler/wasm-aot-staircase-s1.md](compiler/wasm-aot-staircase-s1.md)
  — S1 frames-everywhere baseline + ``--no-frame-elision``
  kill-switch.
- [compiler/wasm-aot-staircase-s2.md](compiler/wasm-aot-staircase-s2.md)
  — S2 per-proc frame elision with refcount discipline.
- [compiler/wasm-aot-staircase-s3.md](compiler/wasm-aot-staircase-s3.md)
  — S3 escape-analysis tightening + ``pure_leaf`` predicate.
- [compiler/wasm-aot-staircase-s4.md](compiler/wasm-aot-staircase-s4.md)
  — S4 IR-level inlining (catalogue + inliner).
- [compiler/wasm-aot-staircase-s5.md](compiler/wasm-aot-staircase-s5.md)
  — S5 SSA-driven codegen optimisations (LICM + GVN + DCE).
- [compiler/wasm-aot-staircase-s6.md](compiler/wasm-aot-staircase-s6.md)
  — S6 allocation + small-value representation (free-lists,
  inline strings, dict hash side-cache, tagged immediates,
  per-statement arena).

## Contracts and interfaces

See [contracts/](contracts/) for focused notes on module contracts and
cross-module interfaces. Each file answers "who owns this surface, what
are its rules, and what are the failure modes". One contract per file.

- [command-registry-event-model.md](contracts/command-registry-event-model.md)
  — command and event registry ownership rules.
- [registry-contract-tests.md](contracts/registry-contract-tests.md) —
  the language-agnostic registry shape contract, `registry-dump` verbs,
  golden fixtures, and the front-end-driven tests that validate them
  (including the `rust` branch).
- [shared-utility-contracts.md](contracts/shared-utility-contracts.md) —
  shared helper ownership across core and LSP.
- [core-lsp-shared-utility.md](contracts/core-lsp-shared-utility.md) — the
  single home for logic that otherwise drifts between features/passes
  (offset mapping, proc lookup, package ranking, event context,
  word-shape parsing) and the rules that keep it from being
  reimplemented.
- [formatter-engine.md](contracts/formatter-engine.md) — formatter
  idempotency and rewrite contracts.
- [project-layout.md](contracts/project-layout.md) — repository layout
  and dependency direction.
- [release-and-publish.md](contracts/release-and-publish.md) —
  the four-layer build/CI/publish model, the no-marketplace-tokens-in-CI
  invariant, and the release flow.
- [lsp-feature-providers.md](contracts/lsp-feature-providers.md) —
  non-diagnostics LSP provider contracts and failure modes.
- [workspace-indexing.md](contracts/workspace-indexing.md) — workspace
  cache, index, and scanner contracts.
- [package-loading.md](contracts/package-loading.md) — stdlib, tcllib,
  Tk, and iRules cross-file package loading.
- [parsing.md](contracts/parsing.md) — segmentation and recovery
  contracts.
- [lexing.md](contracts/lexing.md) — token and range fidelity rules.
- [lsp-diagnostics-publication.md](contracts/lsp-diagnostics-publication.md)
  — LSP diagnostics publication and suppression model.
- [vm-bytecode-test-boundary.md](contracts/vm-bytecode-test-boundary.md)
  — VM and bytecode identity and fixture boundary guidance.
- [vscode-extension.md](contracts/vscode-extension.md) — VS Code
  extension integration contracts.
- [wasm-explorer-view.md](contracts/wasm-explorer-view.md) — JSON
  shape produced by `WasmModule.to_explorer_json()` and consumed by
  the compiler explorer disassembly panel.
- [differential-fuzzing.md](contracts/differential-fuzzing.md) —
  differential fuzzing oracle and coverage-guided mutation contracts.
- [pipeline-lsp-first.md](contracts/pipeline-lsp-first.md) — pipeline
  layering for LSP use.
- [command-alias-resolution.md](contracts/command-alias-resolution.md)
  — `interp alias` resolution and argument role inheritance.
- [docstring-handling.md](contracts/docstring-handling.md) — proc
  docstring extraction, parsing, and formatting.
- [dialect-stubs.md](contracts/dialect-stubs.md) — dialect command stubs
  and inline stub blocks.
- [proc-arg-traits.md](contracts/proc-arg-traits.md) — proc argument
  trait inference.
- [variable-case-mismatch-suggestions.md](contracts/variable-case-mismatch-suggestions.md)
  — case-mismatch suggestion diagnostics.
- [shimmer-reference-behaviour.md](contracts/shimmer-reference-behaviour.md)
  — shimmer expectations and validation strategy.
- [tcloo-implementation.md](contracts/tcloo-implementation.md) — TclOO
  class hierarchy, VM runtime, and MRO.
- [irule-test-framework.md](contracts/irule-test-framework.md) — iRule
  event orchestrator and TMM simulation.
- [namespace-model.md](contracts/namespace-model.md) — unified
  namespace model across dialects.
- [irule4005-racy-static-cross-event.md](contracts/irule4005-racy-static-cross-event.md)
  — IRULE4005 racy `static::` cross-event contract.
- [dialect-detection.md](contracts/dialect-detection.md) — dialect
  detection priority chain.
- [xdg-config.md](contracts/xdg-config.md) — XDG configuration file
  format reference.
- [config-precedence.md](contracts/config-precedence.md) — precedence
  rules between global, project, and editor configuration layers, the
  survey of how other language servers handle the same question, and
  the reference implementations we copied each piece of behaviour
  from.

### First-principles runtime contracts (v2 / "if starting over")

Forward-looking semantic contracts — the models a from-scratch Tcl
runtime + AOT compiler should commit to *before* writing commands.
Distilled from the trickiest scars in the WASM runtime history
(frame aliasing, the parser/interpreter seam, the numeric tower):

- [runtime-variable-frame-model.md](contracts/runtime-variable-frame-model.md)
  — the cell/frame/namespace resolution algorithm behind `upvar`,
  `global`, `variable`, arrays, and traces; why locals are not slots.
- [parser-and-aot-interpret-boundary.md](contracts/parser-and-aot-interpret-boundary.md)
  — the one canonical grammar, and the AOT-compile vs. runtime-interpret
  boundary that `eval`/`uplevel`/`source`/`apply`/`{*}` straddle.
- [numeric-tower-and-expr-semantics.md](contracts/numeric-tower-and-expr-semantics.md)
  — the small-int→wide→bignum→double tower and `expr` as a separate
  language with overridable `mathfunc` dispatch.
- [compiled-scope-and-name-lowering.md](contracts/compiled-scope-and-name-lowering.md)
  — scope class (local/qualified/global) as an explicit lowering output,
  the "emits-nothing" trap, token-faithful eval fallback, and why
  introspection must read live state (`foreach ::v` ran zero times; stale
  `info exists` after `unset`).
- [variable-trace-dispatch-and-introspection.md](contracts/variable-trace-dispatch-and-introspection.md)
  — variable traces as re-entrant interrupts: firing order, the
  read/write error reshape (`can't read/set "NAME": …`), unset-error
  ignore, mutation independent of trace outcome, and live `info`/`trace`
  queries.
- [command-binding-and-aliasing.md](contracts/command-binding-and-aliasing.md)
  — the one resolution model behind `rename`, `interp alias`, `namespace
  import`/`export`/`forget`/`path`, ensembles, and `::tcl::mathop` /
  `::tcl::mathfunc`; the binding lattice that gates compile-time
  resolution (the command-layer parallel of the variable-frame model).

## Knowledge base coverage

- [kcs-completeness-plan.md](kcs-completeness-plan.md) — audit of the
  current KCS coverage (features and top-level notes) and the plan to
  close the gaps toward completeness.

## Templates

Templates for new design docs live at
[templates/](templates/):

- [template-contract.md](templates/template-contract.md) — ownership,
  contracts, and integration boundaries.
- [template-reference.md](templates/template-reference.md) — compact
  reference or decision pages.
- [template-matrix.md](templates/template-matrix.md) — producer/consumer
  ownership matrices.
