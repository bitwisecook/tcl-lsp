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
- [family-b-routing.md](family-b-routing.md) — the Family-B runtime contract
  as implemented on both runtimes,
  which command families were lifted to shared cores, the bugs that surfaced,
  and the boundaries where a command cannot be a shared body.
- [example-script-walkthroughs.md](example-script-walkthroughs.md) — full
  pipeline traces for progressively complex Tcl scripts.
- [code-importing-examples.md](code-importing-examples.md) — reference
  patterns for Tcl code importing (package require, sourcing).
- [tcloo-object-typing.md](tcloo-object-typing.md) — the shipping `TclOO`
  object-handle typing model: how `set v [Class new]` provenance is harvested
  so `$v method …` dispatch resolves to the object's class.
- [tk-widget-instance-typing.md](tk-widget-instance-typing.md) — the sibling
  model for Tk/ttk widgets: how a widget-creating command's instance path
  (`.t`, `$w`) resolves back to the widget class, so `.t instate …` / `$w tag
  configure …` reach subcommand-aware highlighting, hover, completion, and
  diagnostics.
- [tk-static-ui-model.md](tk-static-ui-model.md) — the conservative,
  registry-driven widget-tree and geometry model shared by editor and MCP
  previews, including its abstention, size-bound, and stale-document rules.
- [dialect-profile-model.md](dialect-profile-model.md) — the compositional
  `DialectProfile` model: one profile per dialect owning both command/feature
  availability and runtime/behaviour semantics (octal, expr/lexer grammar,
  versioned libraries keyed by base/BIG-IP/tool version), replacing
  per-consumer `DialectSet` arithmetic across the whole stack.
  **Superseded in part by #1631 and still shipping**: the string-boundary
  resolvers it describes are deleted and gated at zero references, but the
  interned catalogue itself survives as retirement-ledger row C1. Read it
  for how the shipping catalogue behaves; read the registry redesign below
  for the model it answers to.
- [eda-library-packages.md](eda-library-packages.md) — the migration from the
  5 EDA vendor-bit dialects (`XILINX`/`SYNOPSYS`/`CADENCE`/`QUARTUS`/`MENTOR`)
  to a base-Tcl-version dialect plus `required_package`-gated per-tool command
  libraries (a shared `sdc` pack + per-tool vendor packages). Carries the
  21-package taxonomy, the `is_available` package-loaded gate, detection
  hardening, and base-version reconciliation.
The seven documents below are one cluster (issue #1631 — the
dialect/package/environment redesign). Read them in this order: the
**redesign** for the model and, in its §11, the only list of what is still
open; the **centralisation** companion for the seam-by-seam audit and the
retirement ledger; the **two reviews** for why the model has the shape it
has; the **measurements** for the live F5 evidence that rewrote its F5
half; and the **two SpecTcl documents** for the authoring surface the
model's declarations are written in.

- [dialect-and-package-registry-redesign.md](dialect-and-package-registry-redesign.md)
  — **the #1631 model, revision 2, implemented through P6 (2026-08-27) —
  start here.** The four-layer model — core profiles (family × release ×
  build), packages as providers of SpecTcl surface declarations, dynamic
  environments (definitions + overlays with a fixed editor-identity set),
  and realm-scoped binding knowledge; the axis-typed `VersionSet` algebra
  replacing `DialectSet`, version-range targeting, SpecTcl 2.0 with
  fail-closed semantic vocabulary and trust-aware provenance, and the
  migration plan with **every phase carrying its final status** (§8).
  Revision 2 accepts all thirteen blocking findings of the adversarial
  review (§0.1) and all eight of the F5 evidence review (§0.2). §9 records
  what became of each research defect, §10 what became of each owner
  question, and **§11 is the single open-questions ledger for the whole
  programme** — owner decisions never ratified, deferred model items,
  evidence gaps, and the doc-versus-code divergences the closing sweep
  found. If you want to know what is still outstanding anywhere in this
  cluster of documents, read §11 and nothing else.
- [dialect-and-package-registry-centralisation.md](dialect-and-package-registry-centralisation.md)
  — **the audit of record and the retirement ledger, companion** to the
  registry redesign; every ledger row carries its final state (done /
  partial-with-reason / open-gated-on-X) and every open row is repeated in
  the redesign's §11. Read it for *why* a mechanism was retired and what
  gate proves it. The end-to-end audit of
  every registration and resolution seam (front end, compiler, analyser,
  backends, runtimes/VMs, tooling) against the revision-2 model — the single
  registration pipeline and five-question resolution stack, the complete
  retirement ledger (no shims; old systems deleted), the gap rulings, the
  proving gates, the `tcl spec upgrade` 1.x→2.0 specification that
  discharges the sole backwards-compatibility exception, and the
  name-resolution oracle programme grounding namespaces, variables,
  procs/commands, and packages in the C Tcl test suites, the stdlib's
  executable specifications, tcllib, Tk, and the corpus — with the
  consumer conformance lattice as the completion checklist.
- [dialect-and-package-registry-redesign-adversarial-review.md](dialect-and-package-registry-redesign-adversarial-review.md)
  — **request-changes review, all thirteen findings accepted and built**
  (disposition banner at its head; table in the redesign's §0.1). Kept
  verbatim as the record of why the model has four layers rather than one.
  Grounded in immutable Tcl,
  Tk, JimTcl, picol, tcllib, ticklecharts, pave, and SpiceGenTcl sources plus
  reproducible interpreter/build experiments. It identifies the blocking
  separation between provider catalogues and per-interpreter live bindings,
  then specifies corrected version-set, build-profile, trust, lifetime,
  editor-identity, and behavioural-parity contracts.
- [spectcl-syntax-alternatives.md](spectcl-syntax-alternatives.md) —
  **decided (owner, 2026-08-26): design E.** Retained as the comparison
  record; the deep dive supersedes its recommendation section as the basis
  of the decision. Six authoring-surface designs answering the "SpecTcl is
  not very Tcl-like" complaint — synopsis-first, proc-mirror,
  namespace-native, pure-dict, executable registration, and annotated
  stubs — each on one identical worked example over the same internal
  model, with a rubric, comparison matrix, and hybrid recommendation for
  the owner to weigh.
- [spectcl-design-e-deep-dive.md](spectcl-design-e-deep-dive.md) —
  **adopted** (owner, 2026-08-26) and **implemented** (2026-08-27): design
  E — executable registration — is the SpecTcl 2.0 authoring surface,
  together with this document's §1 execution model and rulings E-R1–E-R9.
  The evaluation loader, `tcl spec export`, `spectcl_expand`, the studio on
  the eval loader, canonical-2.0 rendering and StudioOverride patch-pack
  editing all shipped; §14's table now carries a per-ruling status column
  and §15.4 the tick list. E-R11–E-R13 were implemented as proposed and
  await formal ratification (redesign §11, O3). Stress-tests E
  against the widest real surfaces — the
  pinned execution model (frozen snapshots, determinism,
  target-independence, provenance), literal-driven typing via
  `format`/`scan`/`binary`, iRules against the profile and event graph,
  TclOO/Tk, tcllib, the EDA shells, tcl-bpf, SpecTcl self-hosting, and
  corpus-chosen oddities — collecting numbered `E-R` ruling candidates
  and the feedback each walk sends into the Rust model.
- [bigip-irule-parser-measurements.md](bigip-irule-parser-measurements.md) —
  **measured evidence** (owner, live appliance), and the document that
  rewrote the F5 half of the model: it proved the three BIG-IP contexts are
  **one parser**, forcing the `f5-tcl` trunk with `f5-irules` as a dialect
  offshoot, and falsified the iApps/tmsh 8.5 rows. Its §12 names exactly
  what a next appliance run must answer. The E3 transcript the
  BIG-IP evidence review recorded as pending — 378 probes against
  BIG-IP 21.1.0.1 with same-host stock-Tcl controls. Answers F3's
  six-row matrix (the `}{` separator is generic and lexical, gated on
  the word starting with `{` or `"`; `{*}` must not be implemented),
  discovers a second independent divergence (brace-line continuation,
  the N-rules), measures the 31-disabled command surface, 16
  discriminating 8.4-vs-8.5 features, four Tcl contexts on one
  appliance, event-context compile-time validity, and rule priority
  order. Probe corpus in `scripts/dev/bigip-probes/`; the model consumes
  it through the evidence layer in `rust/tcl-registry/src/f5/`
  (`BigIpExecutionContext`, `EmbeddedRuntimeEvidence`, and 205 hermetic
  conformance vectors).
- [dialect-and-package-registry-redesign-bigip-evidence-review.md](dialect-and-package-registry-redesign-bigip-evidence-review.md)
  — **F5 evidence review, all eight findings accepted; its E3 transcript
  has since run and falsified the iApps/tmsh 8.5 hypothesis outright**
  (both report 8.4.6 and carry the fork grammar). Read it with the
  measurements document beside it. Of the fixed iRules, tmsh, and iApps
  Tcl-version
  assumptions left in #1631 revision 2, separating TMM iRules, tmsh CLI
  scripts, iApp implementation Tcl, presentation APL and its embedded Tcl
  callbacks, and host Tcl. It combines upstream parser evidence, official F5
  interfaces, stock-Tcl controls, and an isolated live-appliance probe contract,
  with required provenance and conformance gates.

## Name resolution

How the stack answers "which command / variable / class / expr function does
this name denote?" — the model, and the C ground truth it is held to. The
rule itself, its single Rust home, and its conformance gates live in
[contracts/command-resolution.md](contracts/command-resolution.md).

- [name-resolution.md](name-resolution.md) — the model: the one-resolver
  invariant and its drift gate, written-name colon runs and W314
  addressability, the document / workspace / autoload tiers, source-site
  namespace seeding, the import-alias-rename link graph, command names held
  as data (flow-sensitive constant provenance, dispatch tables, probe roles),
  the variable `VAR_LINK` model, `TclOO` one-hop class resolution and
  C-faithful dispatch chains, interpreter domains, expr functions, and the
  catalogue of deliberate abstentions.
- [name-resolution-c-conformance.md](name-resolution-c-conformance.md) — the
  ground truth: the algorithm for all four name kinds as extracted from the C
  sources, and the 8.4 → 9.1 matrix, each fact pinned to a stable C-Tcl
  permalink (`tclNamesp.c` / `tclVar.c` / `tclOOCall.c` / `tclCompExpr.c`).
- [import-order-source-graph.md](import-order-source-graph.md) — the load
  order derived from the `source` **and `package require`** graphs
  (`tcl_lsp_core::source_graph::RunOrder`): the relation both wildcard-import
  tiers rank cross-document lifecycle events with, which abstentions it lifted,
  where it deliberately still abstains, and — §7 — the one-sided edge a
  `package require` contributes, its three abstentions, and its measured reach
  on tcllib.

## F5 BIG-IP CLI

- [contracts/sslictcl-source-data.md](contracts/sslictcl-source-data.md) —
  the embedded SslicTcl source-data layout, provenance/hash schema, offline
  drift gate, explicit refresh command, and release freshness contract.
- [f5-cli-architecture.md](f5-cli-architecture.md) — verb registry,
  reference graph, IP-redaction model, tmsh emitter, file layout, and
  the recipe for adding a new verb.
- [f5-query-engine-internals.md](f5-query-engine-internals.md) —
  internals of the `f5 query` engine: module layout, pipeline,
  invariants, edit-plan apply order, builtin registration,
  extension points.  User-facing reference (grammar, every
  builtin, sample configs, F5 KB cross-references) lives at
  [`docs/references/f5_query/`](../references/f5_query/), whose
  alphabetical builtin catalogue is hand-maintained against the
  registry in `rust/tcl-bigip-query/src/builtins/`.
- [bigip-registry-architecture.md](bigip-registry-architecture.md) —
  registry contract for object kinds, value specs (parse / project
  / render / references), source-range fidelity, and the pilot
  migration table that opts properties into the typed dispatch.
- [f5-query-renderer-contract.md](f5-query-renderer-contract.md) — the
  compile-time renderer, builtin, and input-format catalogues behind
  `f5 q --render NAME`: the `RendererSpec` / `BuiltinSpec` /
  `InputFormatSpec` contracts, error mapping, and how to add one.

## tclpkg package manager

- [tclpkg-architecture.md](tclpkg-architecture.md) — architecture overview,
  contracts, file-path anchors, test anchors.
- [contracts/tclpkg-contracts.md](contracts/tclpkg-contracts.md) — the
  manifest, lockfile, resolver, cache, and venv contracts for `rust/tcl-pkg`,
  plus the stated gap where the LSP integration does not exist.
- [tclpkg-security.md](tclpkg-security.md) — sandboxing (the `tcl-sandbox`
  crate), operator hooks, and the layered, admin-lockable policy for the Rust
  package manager, with the supply-chain threat model that drives it.
- [contracts/explorer-compiler-coverage.md](contracts/explorer-compiler-coverage.md)
  — coverage contract for durable Rust compiler artefacts in Explorer.

## Compiler internals

See [compiler/README.md](compiler/README.md) for the compiler design-doc
index — pipeline stages, analyses, codegen, optimisation passes, and
ownership matrices.

The target-independent implementation contract is
[common-semantic-compiler.md](compiler/common-semantic-compiler.md). It defines
the common semantic IR and analyses consumed by the LSP, TclVM, WASM, eBPF, and
future native or accelerator target families.

Guarded native specialisation follows the separate default-off
[semantic AOT optimisation contract](compiler/semantic-aot-optimisation.md),
including live runtime identity, trace, materialisation, numeric, TclOO,
namespace, interpreter, and dialect obligations.

The proof that completes its world/effect half is
[compiler/dispatch-stability-proof.md](compiler/dispatch-stability-proof.md) —
the world-state contents/absence lattice, the typed per-site dispatch-stability
proof, and the entry contract that together gate stable-call CSE (`O105`).

## Runtime internals

- [runtime/namespace-tree.md](runtime/namespace-tree.md) — design for
  the Rust runtime's namespace tree (root, child links, per-ns
  command/variable/path tables) modelled on Tcl 9's `Namespace`
  struct, as built in `runtime/rust/src/namespace.rs` (proc lookup lives
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

## The Rust workspace

How the native workspace is laid out, the rules a change to it is measured
against, and how it stays fast under an editor's keystroke load.

- [rust/engineering-guide.md](rust/engineering-guide.md) — the rules every
  change under `rust/` is held to: the two non-negotiable principles (C Tcl
  9.0.4 as the reference standard; time-to-first-tokens), the ratified
  library and data-structure choices, crate layering, and what good code
  looks like here.
- [rust/current-architecture.md](rust/current-architecture.md) — the crate
  graph as it stands: who owns what, the dependency direction that must not
  be violated, and the runtime shape of the native LSP server.
- [rust/target-architecture.md](rust/target-architecture.md) — the target
  design (zero-copy, single-parse, incremental-reuse, MVCC) and how the
  competing goals are reconciled.
- [runtime/rust-vm-tier-parity.md](runtime/rust-vm-tier-parity.md) — the
  Rust bytecode VM's Tier 1/2/3 tcltest parity scoreboard versus C Tcl 9.
- [runtime/rust-regex-port.md](runtime/rust-regex-port.md) — the
  `tcl-regex` crate: a pure-Rust port of Tcl 9's Henry-Spencer ARE engine,
  its architecture, and the `reg.test` corpus that validates it.
- [rust/incremental-analysis.md](rust/incremental-analysis.md) —
  per-item walk with cascade invalidation: the incremental analysis design.
- [rust/incremental-analysis-experiments.md](rust/incremental-analysis-experiments.md)
  — experiments, discoveries, and the reasoning behind the incremental plan.
- [rust/salsa-interned-gc.md](rust/salsa-interned-gc.md) — the salsa
  interned garbage collector that keeps the per-keystroke interned keys in
  `tcl-lsp-db` bounded: how it works, the two ways to disable it by accident
  (a `Durability` bump on an input, interning outside a tracked query), and
  the guardrails that pin it.
- [rust/lsp-performance.md](rust/lsp-performance.md) — native LSP
  performance: results, optimisations, and how to measure.
- [notes/tokio-task-resumption-wedge-repro.md](notes/tokio-task-resumption-wedge-repro.md)
  — the #1657 whole-server wedge: the instrumented evidence chain (a task
  woken twice and never polled again), what is excluded and by what, measured
  repro rates per tokio version, and a self-contained distillation sample
  with its honestly-reported results.

## Optional WASM extensions

- [compiler/wasm-extensions.md](compiler/wasm-extensions.md) —
  current `wasm_stdlib` embedding boundary and the explicitly dated,
  not-yet-implemented package-driven extension design.
- [compiler/wasm-target-surfaces.md](compiler/wasm-target-surfaces.md) —
  WASI vs in-browser WASM: per-command-family capability matrix, the
  browser-target build/wiring gaps, the proposed host-import surface, and
  measured module sizes (raw, `wasm-opt`, gzip).
- [compiler/aot-command-priority.md](compiler/aot-command-priority.md) —
  real-corpus census (issue #1181) ranking which Tcl commands the AOT
  WASM compiler should emit directly next, with a breadth-weighted
  tiering and what is already covered versus what cannot be direct.

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
- [formatter-engine.md](contracts/formatter-engine.md) — formatter
  idempotency and rewrite contracts.
- [project-layout.md](contracts/project-layout.md) — repository layout
  and dependency direction.
- [release-and-publish.md](contracts/release-and-publish.md) —
  the four-layer build/CI/publish model, the no-marketplace-tokens-in-CI
  invariant, and the release flow.
- [lsp-feature-providers.md](contracts/lsp-feature-providers.md) —
  non-diagnostics LSP provider contracts and failure modes.
- [lsp-transport-liveness.md](contracts/lsp-transport-liveness.md) — stdin,
  handler-admission, and stdout liveness boundaries for the LSP transport.
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
  shape produced by `wasm_to_explorer_json` and consumed by
  the compiler explorer disassembly panel.
- [explorer-compiler-coverage.md](contracts/explorer-compiler-coverage.md) —
  durable compiler artefacts that every Explorer front-end must expose.
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
- [command-spec-studio.md](contracts/command-spec-studio.md) — the spec
  studio's schema / draft / renderer layering, the invariants that keep it
  in step with `CommandSpec`, the rules its rendered `.rs` must satisfy,
  and the multi-snapshot version-range importer behind `tcl spec import`.
- [callback-surface-inventory.md](contracts/callback-surface-inventory.md) —
  registry-derived executable/callback coverage, the audited external/dynamic
  catalogue, and the generated JSON/Markdown drift gate.
- [spec-packs.md](spec-packs.md) — SpecTcl, the Tcl-DSL command database
  for private libraries: the runtime loader, discovery, and hook
  execution that have landed; the version-ranges lifecycle model at every
  gateable level; and the vocabulary-tolerance policy that avoids
  per-release rebuilds.
- [spec-dsl-examples/tricky-surfaces.md](spec-dsl-examples/tricky-surfaces.md)
  — the DSL's acceptance rubric: the tricky Tcl surfaces (operator
  aliasing, TclOO corners, real-world options, paired tails, hooks)
  each ticked against a ported example during review.
- [spec-dsl-examples/external/README.md](spec-dsl-examples/external/README.md)
  — the external-library census (ticklecharts, apave, SpiceGenTcl,
  uncovered tcllib): per-library command-shape findings and the
  frequency-ranked DSL requirements with their gap catalogue.
- [spec-dsl-examples/external/corpus-expansion.md](spec-dsl-examples/external/corpus-expansion.md)
  — the wider corpus hunt behind the #1181 additions, plus sixteen
  hook-pattern exemplars quoted from cloned sources.
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
- [cross-file-diagnostics.md](contracts/cross-file-diagnostics.md) — the
  single cross-document command lookup diagnostics and navigation share,
  the cross-file arity envelope, the two directions of the `source` graph,
  and the complete list of what makes the server abstain.
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

## Differential audits

- [kcs-codes-drift-audit-2026-08-12.md](kcs-codes-drift-audit-2026-08-12.md)
  — source-verified audit of the per-code diagnostic pages: missing
  pages, behaviour contradictions, and the now-closed `safe_on_uninit`
  wiring gap it surfaced.
- [issue-923-differential-audit/README.md](issue-923-differential-audit/README.md)
  — the three-way differential method (mine a corpus, reduce to a minimal
  repro, compare a real `tclsh` oracle against the LSP), the oracle-environment
  recipe, and the eight-corpus inventory, alongside the raw findings data and
  the orchestration scripts that produced them.

## Templates

Templates for new design docs live at
[templates/](templates/README.md):

- [template-contract.md](templates/template-contract.md) — ownership,
  contracts, and integration boundaries.
- [template-reference.md](templates/template-reference.md) — compact
  reference or decision pages.
- [template-matrix.md](templates/template-matrix.md) — producer/consumer
  ownership matrices.
