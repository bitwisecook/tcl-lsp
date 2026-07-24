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
- [family-b-routing.md](family-b-routing.md) — companion to the above (§4
  Family B): the Family-B runtime contract as implemented on both runtimes,
  which command families were lifted to shared cores, the bugs that surfaced,
  and the boundaries where a command cannot be a shared body.
- [example-script-walkthroughs.md](example-script-walkthroughs.md) — full
  pipeline traces for progressively complex Tcl scripts.
- [code-importing-examples.md](code-importing-examples.md) — reference
  patterns for Tcl code importing (package require, sourcing).
- [incremental-analysis-worker.md](incremental-analysis-worker.md) — the
  persistent per-document analysis worker model: bounded pool + per-uri
  single-writer lock for incremental edits, process pool + serialized
  warm-start seed for the cold build.
- [tcloo-mro-lattice.md](tcloo-mro-lattice.md) — the (measured-negative)
  `TclOO` object→class dispatch lattice experiment: why intraprocedural class
  resolution collapses to ⊤ on real corpora, and what the shipping
  MRO/CHA + provenance model does instead.
- [tcloo-object-typing.md](tcloo-object-typing.md) — the shipping `TclOO`
  object-handle typing model: how `set v [Class new]` provenance is harvested
  so `$v method …` dispatch resolves to the object's class.
- [tk-widget-instance-typing.md](tk-widget-instance-typing.md) — the sibling
  model for Tk/ttk widgets: how a widget-creating command's instance path
  (`.t`, `$w`) resolves back to the widget class, so `.t instate …` / `$w tag
  configure …` reach subcommand-aware highlighting, hover, completion, and
  diagnostics.
- [dialect-profile-model.md](dialect-profile-model.md) — the compositional
  `DialectProfile` model: one profile per dialect owning both command/feature
  availability and runtime/behaviour semantics (octal, expr/lexer grammar,
  versioned libraries keyed by base/BIG-IP/tool version), replacing
  per-consumer `DialectSet` arithmetic across the whole stack. Carries the
  complete consumer inventory and a milestone/stage delivery plan.
- [eda-library-packages.md](eda-library-packages.md) — the migration from the
  5 EDA vendor-bit dialects (`XILINX`/`SYNOPSYS`/`CADENCE`/`QUARTUS`/`MENTOR`)
  to a base-Tcl-version dialect plus `required_package`-gated per-tool command
  libraries (a shared `sdc` pack + per-tool vendor packages). Carries the
  21-package taxonomy, the `is_available` package-loaded gate, detection
  hardening, base-version reconciliation, and a phased, differential-guarded,
  behaviour-preserving execution plan.

> Past project-tracking documents (perf reports, phase trackers,
> migration plans) are kept in [`../archive/`](../archive/) and are
> not part of the current design surface.

## Name resolution

The workspace-scoped, C-Tcl-faithful command / variable / class
name-resolution effort (issue #923): the audits behind it, the
version-sensitive semantics, and the staged fix plan being executed on this
surface.

- [name-resolution-fix-plan.md](name-resolution-fix-plan.md) — the master
  execution plan: the staged milestones (M1–M16) for correct, workspace-scoped,
  dialect-aware resolution, with per-stage status.
- [resolution-soundness-945.md](resolution-soundness-945.md) — the issue #945
  follow-up contract: flow-sensitive constant-dispatch value provenance,
  one-to-many source views, the typed TclOO method table + C-faithful dispatch
  chains, the interpreter-domain model (safe visibility, temporal identity),
  and probe command references.
- [name-resolution-centralization.md](name-resolution-centralization.md) — the
  audit + proposal to consolidate the ad-hoc target-selection sites onto one
  C-Tcl command-resolution routine so every LSP provider agrees.
- [cross-file-command-resolution-lattice.md](cross-file-command-resolution-lattice.md)
  — the proposal for the cross-file resolution lattice: settling a call to its
  defining proc/class across the workspace index, sound by abstention.
- [name-resolution-tcl-version-and-c-source.md](name-resolution-tcl-version-and-c-source.md)
  — the version-sensitive resolution semantics (8.4→9.1), each fact pinned to a
  stable C-Tcl source permalink (`tclNamesp.c` / `tclVar.c`).
- [tricky-name-resolution-surfaces.md](tricky-name-resolution-surfaces.md) — the
  navigation-link audit of the hard cases: aliases, renames, imports, forwards,
  ensembles, per-object methods, and command-names-held-as-data.
- [colon-names-and-addressability.md](colon-names-and-addressability.md) — the
  written-name colon-run rule vs the constructed-key discipline (issue #934):
  `proc :`, `proc {}`, `namespace eval :`, which definitions have no absolute
  spelling, and the W314 diagnostic that flags them.
- [issue-923-differential-audit/STATUS.md](issue-923-differential-audit/STATUS.md)
  — status and handoff for the issue #923 differential-audit campaign: mined
  tricky patterns from tcllib/tk/georgtree/nico-robert corpora, verified
  against real `tclsh` oracles, fixed and tested. Tracks what's fixed, what's
  triaged-but-open, the shared resolution mechanisms added, and everything
  needed to resume (raw findings data + the exact orchestration scripts, in
  the same directory).

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
  `scripts/dev/gen_query_builtins_doc.py`
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
- [tclpkg-security.md](tclpkg-security.md) — sandboxing (the `tcl-sandbox`
  crate), operator hooks, and the layered, admin-lockable policy for the Rust
  package manager, with the supply-chain threat model that drives it.
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
- [contracts/explorer-view-audit.md](contracts/explorer-view-audit.md) —
  audit of the compiler-explorer views to represent the Rust compiler,
  not the Python one.

## Compiler internals

See [compiler/README.md](compiler/README.md) for the compiler design-doc
index — pipeline stages, analyses, codegen, optimisation passes, and
ownership matrices.

## Runtime internals

- [runtime/namespace-tree.md](runtime/namespace-tree.md) — design for
  the Rust runtime's namespace tree (root, child links, per-ns
  command/variable/path tables) modelled on Tcl 9's `Namespace`
  struct, with the migration plan from FQN-string fallbacks to a real
  namespace tree in `runtime/rust/src/namespace.rs` (proc lookup lives
  in `cmd_proc.rs` / `interp.rs`).
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
- [runtime/tcl-test-tiers.md](runtime/tcl-test-tiers.md) — the capability
  ladder (parsing → interpretation → fundamentals → control flow → I/O →
  platform features) ordering the work toward C tcltest parity.
- [runtime/backend-constraints.md](runtime/backend-constraints.md) — the
  ``tcl_platform`` backend-introspection schema and the loadable overlay that
  skips upstream tests a wasm / WASI / eBPF build cannot run.
- [runtime/tclvm-opcode-status.md](runtime/tclvm-opcode-status.md) —
  C Tcl 9.0 bytecode instruction coverage for the TCLVM.

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
- [rust/workspace-deep-review-2026-06-22.md](rust/workspace-deep-review-2026-06-22.md)
  — full subsystem deep review (every crate: architecture, layout,
  algorithms, code quality), with the recursion, regex, and optimiser-
  miscompile themes and five CLI-reproduced defects.
- [rust/lsp-server-deep-review-2026-06-22.md](rust/lsp-server-deep-review-2026-06-22.md)
  — companion deep review of the native LSP server stack
  (``tcl-lsp-server`` / ``-core`` / ``-db`` / ``-py``), 18 findings.
- [rust/python-rust-parity-audit-2026-06-22.md](rust/python-rust-parity-audit-2026-06-22.md)
  — Python→Rust parity audit (registry, diagnostics, optimisations,
  passes/features): one missing command (``ledit``), four Rust-only
  optimiser miscompiles, an unwired inliner, and the deleted parity-check
  tooling.
- [rust/coherence-and-coverage-2026-06-23.md](rust/coherence-and-coverage-2026-06-23.md)
  — closing review pass: a coverage map proving every goal aspect is documented
  across the six reviews, plus the remaining axes — **type-system coherence**
  (bimodal: the value/registry half is the template, the editor half fractures
  along the UTF-16 seam — raw offsets, 3 `Severity` enums, 2 `Diagnostic`
  structs, stringly-typed IR), **naming coherence + glossary currency**, the
  **explorer trio (CLI/TUI/GUI)** (the model of one-core reuse), and the
  **"information" subsystem** (Info→Hint severity collapse; `info`-command
  `VM ⊂ WASM` parity gap). Reconciled against the just-landed `origin/rust`
  API-PYO3 / xtask / TEST-MIGRATE work.
- [rust/srv-incremental-review-2026-06-23.md](rust/srv-incremental-review-2026-06-23.md)
  — deep review of the SRV-INCREMENTAL work (#692): per-edit incremental salsa
  pipeline (incremental `LineIndex`, per-function check memo, interprocedural
  taint summary memo) + opt-in cross-file W123/arity diagnostics. Verdict: lands
  clean (no correctness regression; off-by-default airtight; corpus differentials
  + 38 gates green), with three actionable findings — a `project_command_arities`
  firewall perf-leak, an open→open push-staleness gap, and god-code growth — plus
  the doc's own missing random-edit checks fuzzer.
- [rust/compiler-pipeline-parity.md](rust/compiler-pipeline-parity.md) —
  deep parity audit of the Rust rewrite's lexer, CST, IR/lowering, CFG/SSA,
  analyses, optimiser, and bytecode codegen against the Python source of
  truth, with a per-code coverage table and a prioritised gap register.
- [runtime/runtime-execution-gaps.md](runtime/runtime-execution-gaps.md) — the
  consolidated index for the runtime & execution port scope (RT-WASM / RT-VM /
  `runtime/rust`) and the tiered VM/runtime delivery plan; the single entry
  point separate from the tooling/LSP/compiler/analysis gaps.
- [runtime/rust-vm-tier-parity.md](runtime/rust-vm-tier-parity.md) — the
  Rust bytecode VM's Tier 1/2/3 tcltest parity scoreboard versus C Tcl 9.
- [runtime/rust-regex-port.md](runtime/rust-regex-port.md) — the
  `tcl-regex` crate: a pure-Rust port of Tcl 9's Henry-Spencer ARE engine,
  its architecture, and the `reg.test` corpus that validates it.
- [rust/incremental-analysis.md](rust/incremental-analysis.md) —
  per-item walk with cascade invalidation: the incremental analysis design.
- [rust/incremental-analysis-experiments.md](rust/incremental-analysis-experiments.md)
  — experiments, discoveries, and the reasoning behind the incremental plan.
- [rust/lsp-performance.md](rust/lsp-performance.md) — native LSP
  performance: results, optimisations, and how to measure.
- [rust/s110-byte-array-corruption-port.md](rust/s110-byte-array-corruption-port.md)
  — FE-TYPESHIM port spec for the S110 byte-array-corruption shimmer
  (Python #656): algorithm, Rust integration points, and verification contract.
- [rust/python-rust-port-gaps.md](rust/python-rust-port-gaps.md) — consolidated
  audit of every feature not yet completely ported from Python to Rust
  (front-end / analyser / server / API / tooling scope).
- [rust/scripts-retirement-triage.md](rust/scripts-retirement-triage.md) —
  per-`scripts/` triage for Python retirement: retire-with-Python vs survivor
  vs runtime-scope, and the `scripts`→`xtask` decisions.
- [rust/fp-rust-port-plan.md](rust/fp-rust-port-plan.md) — plan for porting the
  false-positive / ground-truth precision catalogue to Rust.
- [rust/fp-rust-port-status.md](rust/fp-rust-port-status.md) — live worklist of
  the remaining FP-precision Rust-vs-Python divergences.
- [rust/analyser-verification-2026-06-30.md](rust/analyser-verification-2026-06-30.md)
  — analyser verification snapshot (2026-06-30).
- [rust/python-parity-scrub.md](rust/python-parity-scrub.md) — Python-parity
  scrub pass findings.
- [rust/cleanup-status.md](rust/cleanup-status.md) — workspace cleanup status.

## Optional WASM extensions

- [compiler/wasm-extensions.md](compiler/wasm-extensions.md) —
  contract for shipping optional runtime features the user's
  program requests via ``package require``. Variant runtimes today;
  deferred Stage 2 plan for separately-merged extension WASMs.
  Includes the file layout for the in-tree tcltest port (every ~107
  upstream tcltest command registered, PORTABLE/PARTIAL ones
  implemented and NOT-PORTABLE ones stubbed with explicit error
  messages).

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
- [compiler/byte-array-corruption.md](compiler/byte-array-corruption.md)
  — the `S110` byte-array corruption diagnostic: binary data forced
  through character-string semantics.

## Contracts and interfaces

See [contracts/](contracts/) for focused notes on module contracts and
cross-module interfaces. Each file answers "who owns this surface, what
are its rules, and what are the failure modes". One contract per file.

- [command-registry-event-model.md](contracts/command-registry-event-model.md)
  — command and event registry ownership rules.
- [special-variable-registry.md](special-variable-registry.md) — the
  dialect-versioned registry of interpreter-provided special variables
  (`auto_path`, `env`, `tcl_platform`, iRules `static::`): its data model,
  dialect resolution, and the analyser / taint / side-effect / hover consumers.
- [registry-contract-tests.md](contracts/registry-contract-tests.md) —
  the language-agnostic registry shape contract, golden fixtures, and the
  front-end-driven tests that validate them (including the `rust` branch).
- [shared-utility-contracts-rust.md](contracts/shared-utility-contracts-rust.md)
  — the Rust workspace's shared-utility owners (`tcl-syntax` grammars and
  switch-body tokeniser, `tcl-lexer` backslash decode, `tcl-cmd-core`
  namespace byte-ops / prefix matcher + `OptionTable` wrapper / error
  catalogue, `tcl-compiler` text similarity, `tcl-core-types`
  vocabulary), the no-re-derivation rule, and the documented exceptions.
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
- [command-resolution.md](contracts/command-resolution.md) — the one
  C-Tcl command-name resolution algorithm, its consumers, and the
  tclsh-pinned conformance vector gates.
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
