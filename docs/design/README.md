# Design documentation

This folder is the home for technical documentation about how tcl-lsp is
built: architecture, contracts, interfaces, data-structure references,
pipeline internals, and pass/fact ownership. Technical jargon and
specialist terms are allowed here — design docs describe how the system
is structured and why, and assume the reader can read the code.

If you are writing a user-facing answer, a how-to, a Q&A, or a feature
description, it belongs in [`docs/kcs/`](../kcs/README.md) instead. The
rules for the KCS/documentation split live in
[`docs/kcs/STYLE.md`](../kcs/STYLE.md).

Documents are filed by area, one folder per area.

## analysis/

How the analyser decides what a name denotes and what a value holds.

- [name-resolution.md](analysis/name-resolution.md) — the resolution model
  for commands, variables, classes, and expr functions: the one-resolver
  invariant, the document / workspace / autoload tiers, command names held
  as data, the `VAR_LINK` model, TclOO dispatch chains, and the deliberate
  abstentions.
- [name-resolution-c-conformance.md](analysis/name-resolution-c-conformance.md)
  — the same four name kinds as extracted from the C sources, with the
  8.4 → 9.1 matrix pinned to C-Tcl permalinks.
- [import-order-source-graph.md](analysis/import-order-source-graph.md) — the
  load order derived from the `source` and `package require` graphs, what it
  lets the wildcard-import tiers rank, and where it abstains.
- [code-importing-examples.md](analysis/code-importing-examples.md) — worked
  `package require` and `source` importing patterns to check the model
  against.
- [tcloo-object-typing.md](analysis/tcloo-object-typing.md) — how
  `set v [Class new]` provenance is harvested so `$v method …` resolves to
  the object's class.
- [tk-widget-instance-typing.md](analysis/tk-widget-instance-typing.md) — the
  same for Tk/ttk: how a widget instance path resolves back to its widget
  class, feeding highlighting, hover, completion, and diagnostics.
- [tk-static-ui-model.md](analysis/tk-static-ui-model.md) — the
  registry-driven widget-tree and geometry model behind the editor and MCP
  previews, with its abstention, size-bound, and stale-document rules.

## compiler/

The multi-pass compiler: pipeline stages, analyses, codegen, optimisation
passes, and ownership matrices.

- [compiler/README.md](compiler/README.md) — the compiler design-doc index;
  every stage, analysis, and pass page is reached from it.
- [architecture.md](compiler/architecture.md) — the high-level map of the
  pipeline, with diagrams and cross-links to the stage documents.
- [example-walkthroughs.md](compiler/example-walkthroughs.md) — full pipeline
  traces for progressively complex Tcl scripts.
- [value-transfers.md](compiler/value-transfers.md) — **proposal** for the
  registry's dataflow axis (issue #1943): how a command invocation transforms
  the constant lattice, declared once per command, authorable from a
  `.tclspec` pack, and consumed generically by every pass and diagnostic.
- [registry-consumer-contracts.md](compiler/registry-consumer-contracts.md)
  — **proposal** companion: the description, identity, and implementation
  contracts under which the registry can drive the analyser, codegen, and the
  runtimes, with the dialect, package, and C-extension consequences.
- [value-transfers-review.md](compiler/value-transfers-review.md) — review
  of the value-transfer and consumer-contract proposals: registry-owned
  specialisation, shared expression/regexp evaluation, correctness findings,
  analysis/diagnostic separation, adversarial attacks, and delivery order.

## contracts/

One contract per file: who owns a surface, what its rules are, and how it
fails.

- [callback-surface-inventory.md](contracts/callback-surface-inventory.md) —
  registry-derived executable/callback coverage, the audited external and
  dynamic catalogue, and the generated JSON/Markdown drift gate.
- [command-alias-resolution.md](contracts/command-alias-resolution.md) —
  `interp alias` resolution and argument-role inheritance.
- [command-binding-and-aliasing.md](contracts/command-binding-and-aliasing.md)
  — the one resolution model behind `rename`, `interp alias`, `namespace
  import`/`export`/`forget`/`path`, ensembles, and `::tcl::mathop` /
  `::tcl::mathfunc`, plus the binding lattice that gates compile-time
  resolution.
- [command-registry-event-model.md](contracts/command-registry-event-model.md)
  — command and event registry ownership rules.
- [command-resolution.md](contracts/command-resolution.md) — the one C-Tcl
  command-name resolution algorithm, its consumers, and the tclsh-pinned
  conformance vector gates.
- [command-spec-studio.md](contracts/command-spec-studio.md) — the spec
  studio's schema / draft / renderer layering, the rules its rendered `.rs`
  must satisfy, and the version-range importer behind `tcl spec import`.
- [compiled-scope-and-name-lowering.md](contracts/compiled-scope-and-name-lowering.md)
  — scope class as an explicit lowering output, the "emits-nothing" trap,
  token-faithful eval fallback, and why introspection must read live state.
- [config-precedence.md](contracts/config-precedence.md) — precedence between
  global, project, and editor configuration layers, and the reference
  implementations each behaviour is copied from.
- [cross-file-diagnostics.md](contracts/cross-file-diagnostics.md) — the
  cross-document command lookup share, the cross-file arity envelope, the two
  directions of the `source` graph, and what makes the server abstain.
- [development-environment.md](contracts/development-environment.md) —
  toolchain prerequisites, what the remote-session hook pre-installs, the
  owner of every pinned version, and the build entry points.
- [dialect-detection.md](contracts/dialect-detection.md) — the dialect
  detection priority chain.
- [dialect-stubs.md](contracts/dialect-stubs.md) — dialect command stubs and
  inline stub blocks.
- [differential-fuzzing.md](contracts/differential-fuzzing.md) — the
  differential fuzzing oracle and coverage-guided mutation contracts.
- [docstring-handling.md](contracts/docstring-handling.md) — proc docstring
  extraction, parsing, and formatting.
- [explorer-compiler-coverage.md](contracts/explorer-compiler-coverage.md) —
  the durable compiler artefacts every Explorer front-end must expose.
- [formatter-engine.md](contracts/formatter-engine.md) — formatter
  idempotency and rewrite contracts.
- [irule-test-framework.md](contracts/irule-test-framework.md) — the iRule
  event orchestrator and TMM simulation.
- [irule4005-racy-static-cross-event.md](contracts/irule4005-racy-static-cross-event.md)
  — the IRULE4005 racy `static::` cross-event contract.
- [lexing.md](contracts/lexing.md) — token and range fidelity rules.
- [lsp-diagnostics-publication.md](contracts/lsp-diagnostics-publication.md) —
  the LSP diagnostics publication and suppression model.
- [lsp-feature-providers.md](contracts/lsp-feature-providers.md) — the
  non-diagnostics LSP provider contracts and their failure modes.
- [lsp-source-store.md](contracts/lsp-source-store.md) — the `SourceStore`
  seam every closed file reaches the server through, why `NativeStore` stays
  a literal `std::fs` delegation, the virtual `.tclspec` mount a browser host
  upserts packs under, and the host message contract.
- [lsp-transport-liveness.md](contracts/lsp-transport-liveness.md) — stdin,
  handler-admission, and stdout liveness boundaries for the LSP transport.
- [namespace-model.md](contracts/namespace-model.md) — the unified namespace
  model across dialects.
- [numeric-tower-and-expr-semantics.md](contracts/numeric-tower-and-expr-semantics.md)
  — the small-int → wide → bignum → double tower, and `expr` as a separate
  language with overridable `mathfunc` dispatch.
- [package-loading.md](contracts/package-loading.md) — stdlib, tcllib, Tk,
  and iRules cross-file package loading.
- [parser-and-aot-interpret-boundary.md](contracts/parser-and-aot-interpret-boundary.md)
  — the one canonical grammar, and the AOT-compile versus runtime-interpret
  boundary that `eval` / `uplevel` / `source` / `apply` / `{*}` straddle.
- [parsing.md](contracts/parsing.md) — segmentation and recovery contracts.
- [pipeline-lsp-first.md](contracts/pipeline-lsp-first.md) — pipeline layering
  for LSP use.
- [proc-arg-traits.md](contracts/proc-arg-traits.md) — proc argument trait
  inference.
- [project-layout.md](contracts/project-layout.md) — repository layout and
  dependency direction.
- [registry-contract-tests.md](contracts/registry-contract-tests.md) — the
  language-agnostic registry shape contract, its golden fixtures, and the
  front-end-driven tests that validate them.
- [release-and-publish.md](contracts/release-and-publish.md) — the four-layer
  build/CI/publish model, the marketplace-tokens-only-as-approval-gated-
  Environment-secrets invariant, and the release flow.
- [runtime-variable-frame-model.md](contracts/runtime-variable-frame-model.md)
  — the cell/frame/namespace resolution algorithm behind `upvar`, `global`,
  `variable`, arrays, and traces, and why locals are not slots.
- [shared-utility-contracts-rust.md](contracts/shared-utility-contracts-rust.md)
  — the workspace's shared-utility owners, the no-re-derivation rule, and the
  documented exceptions.
- [shimmer-reference-behaviour.md](contracts/shimmer-reference-behaviour.md) —
  shimmer expectations and validation strategy.
- [sslictcl-source-data.md](contracts/sslictcl-source-data.md) — the embedded
  SslicTcl source-data layout, provenance/hash schema, offline drift gate,
  and release freshness contract.
- [tank-persistent-cargo-target.md](contracts/tank-persistent-cargo-target.md)
  — safe per-registration Cargo target reuse on the trusted Tank runner:
  lock-aware cleanup, disk guard, telemetry, and hosted overflow boundaries.
- [tcloo-implementation.md](contracts/tcloo-implementation.md) — the TclOO
  class hierarchy, VM runtime, and MRO.
- [tclpkg-contracts.md](contracts/tclpkg-contracts.md) — the manifest,
  lockfile, resolver, cache, and venv contracts for `rust/tcl-pkg`, and the
  gap where the LSP integration does not exist.
- [test-tiers-and-ci-gates.md](contracts/test-tiers-and-ci-gates.md) — the
  smoke / deep / exhaustive tiers, the local gates before a push, the
  `#[ignore]` and xfail policies, and the CI redundancy contract.
- [variable-case-mismatch-suggestions.md](contracts/variable-case-mismatch-suggestions.md)
  — case-mismatch suggestion diagnostics.
- [variable-trace-dispatch-and-introspection.md](contracts/variable-trace-dispatch-and-introspection.md)
  — variable traces as re-entrant interrupts: firing order, the read/write
  error reshape, unset-error ignore, mutation independent of trace outcome,
  and live `info` / `trace` queries.
- [vm-bytecode-test-boundary.md](contracts/vm-bytecode-test-boundary.md) — VM
  and bytecode identity and fixture boundaries.
- [vm-compiled-artifact-provenance.md](contracts/vm-compiled-artifact-provenance.md)
  — the TclVM owner for assembly plus profile, command, and compile-service
  generations, and the lazy-recompile versus fail-closed invalidation rules.
- [vscode-extension.md](contracts/vscode-extension.md) — VS Code extension
  integration contracts.
- [wasm-explorer-view.md](contracts/wasm-explorer-view.md) — the JSON shape
  produced by `wasm_to_explorer_json` and consumed by the Explorer
  disassembly panel.
- [workspace-indexing.md](contracts/workspace-indexing.md) — workspace cache,
  index, and scanner contracts.
- [xdg-config.md](contracts/xdg-config.md) — XDG configuration file format
  reference.

## f5/

The BIG-IP side of the toolchain: the appliance evidence, the registry it
feeds, and the `f5` CLI.

- [bigip-irule-parser-measurements.md](f5/bigip-irule-parser-measurements.md)
  — live-appliance measurements behind the F5 model: the three BIG-IP
  contexts as one parser, the `}{` separator and brace-continuation rules,
  the disabled-command surface, the 8.4-versus-8.5 discriminators, and rule
  priority order.
- [bigip-registry-architecture.md](f5/bigip-registry-architecture.md) — the
  registry contract for BIG-IP object kinds and value specs (parse / project
  / render / references), and source-range fidelity.
- [f5-cli-architecture.md](f5/f5-cli-architecture.md) — verb registry,
  reference graph, IP-redaction model, tmsh emitter, file layout, and the
  recipe for adding a verb.
- [f5-query-engine-internals.md](f5/f5-query-engine-internals.md) — module
  layout, pipeline, invariants, edit-plan apply order, builtin registration,
  and extension points of the `f5 query` engine. The user-facing grammar and
  builtin reference is [`docs/references/f5_query/`](../references/f5_query/).
- [f5-query-renderer-contract.md](f5/f5-query-renderer-contract.md) — the
  `RendererSpec` / `BuiltinSpec` / `InputFormatSpec` catalogues behind
  `f5 q --render NAME`, their error mapping, and how to add one.
- [iruleslx-remote-methods.md](f5/iruleslx-remote-methods.md) — how an
  `ILX::call` / `ILX::notify` method word reaches the `ILXServer.addMethod`
  registration that implements it, the workspace-directory association rule,
  and the JavaScript forms that abstain.
- [sslictcl-vocabulary.md](f5/sslictcl-vocabulary.md) — the `.sslictcl`
  vocabulary-1 reference: every declaration and member, the value domains,
  the open/closed rule, the never-evaluated guarantee, the
  `(check_id, endpoint)` finding identity, and the `SSLIC1xxx` diagnostics.

## registry/

The dialect, package, and environment model, and the authoring surface its
declarations are written in.

- [dialect-and-package-registry-redesign.md](registry/dialect-and-package-registry-redesign.md)
  — the model as built: the four layers, the `VersionSet` algebra, range
  targeting, the SpecTcl 2.0 vocabulary and trust, the F5 evidence layer and
  probe contract, and the invariants.
- [dialect-and-package-registry-centralisation.md](registry/dialect-and-package-registry-centralisation.md)
  — the registration and resolution contract every consumer is held to, the
  gap rulings, the `tcl spec upgrade` specification, and the name-resolution
  oracle programme.
- [dialect-profile-model.md](registry/dialect-profile-model.md) — the
  interned `DialectProfile` catalogue the lexer is keyed on.
- [eda-library-packages.md](registry/eda-library-packages.md) — the EDA
  shells as base-Tcl profiles plus `required_package`-gated tool packages,
  shipped as bundled `.tclspec` packs with `environment` blocks.
- [spec-packs.md](registry/spec-packs.md) — SpecTcl: the loader, discovery
  tiers, hook host, compatibility policy, version ranges, and the 2.0
  authoring rules and `environment` / `include from` vocabulary.
- [special-variable-registry.md](registry/special-variable-registry.md) — the
  dialect-versioned registry of interpreter-provided special variables
  (`auto_path`, `env`, `tcl_platform`, iRules `static::`) and its analyser,
  taint, side-effect, and hover consumers.

## runtime/

The Tcl runtime: object model, namespaces, commands, the C surface it
exports, and the ladder toward C tcltest parity.

- [backend-constraints.md](runtime/backend-constraints.md) — the
  `tcl_platform` backend-introspection schema and the loadable overlay that
  skips upstream tests a wasm / WASI / eBPF build cannot run.
- [c-api-ownership-contract.md](runtime/c-api-ownership-contract.md) — the C
  Tcl API the runtime ships (`tcl.h` / `tclOO.h` / `tclTomMath.h`) and the
  refcount-ownership and error-return contract every exported entry point
  honours.
- [c-extension-abi.md](runtime/c-extension-abi.md) — the C-Tcl-extension →
  WASM ABI for compiling and linking an unmodified C extension against the
  runtime. A contract; nothing in the tree implements it yet.
- [c-extension-shim.md](runtime/c-extension-shim.md) — the `tcl-cshim` shim:
  the trust model for shimmed native code, `Tcl_Obj` to structured-value
  marshalling, the `_Init` registration story, and the C API subset it
  covers.
- [child-interp.md](runtime/child-interp.md) — child-interpreter primitives,
  the `Interp` handle and per-interp hidden table, `with_child` as the
  re-entrancy guard for nested eval, and the compiler's conservative
  proc-index flush.
- [command-introspection.md](runtime/command-introspection.md) — the
  interpreter-wide hidden-commands table, `interp hide` / `expose` /
  `invokehidden` semantics and C's observable check order, and the
  `info commands` / `info procs` / `namespace which -command` walkers.
- [family-b-routing.md](runtime/family-b-routing.md) — the Family-B contract
  across both runtimes: which command families are lifted to shared cores,
  and the boundaries where a command cannot be a shared body.
- [memory-management.md](runtime/memory-management.md) — TclObj refcount
  discipline, `OBJ_STR_CAP` ownership, the deferred-free queue, parse-cache
  invalidation, and the bump-allocator → libc-malloc routing rationale.
- [namespace-tree.md](runtime/namespace-tree.md) — the namespace tree (root,
  child links, per-namespace command / variable / path tables) modelled on
  Tcl 9's `Namespace` struct.
- [proc-call-and-stack-traces.md](runtime/proc-call-and-stack-traces.md) —
  the proc call protocol: argument binding, exception propagation, and
  `-errorinfo` / `-errorcode` stack-trace assembly across the call stack.
- [refcount-contract.md](runtime/refcount-contract.md) — ownership categories
  for every WASM-exported runtime function (callee-takes / caller-keeps /
  borrow), the linter that enforces them, and the decision rules for new
  exports.
- [rename-alias.md](runtime/rename-alias.md) — layout and flow for `rename`
  and single-interp `interp alias`: the `Command` enum, the dispatch
  trampoline, the `TclPreventAliasLoop` gate, the occupied-destination
  refusal, and proc re-homing across a cross-namespace rename.
- [rust-regex-port.md](runtime/rust-regex-port.md) — the `tcl-regex` crate: a
  pure-Rust port of Tcl 9's Henry-Spencer ARE engine, and the `reg.test`
  corpus that validates it.
- [rust-vm-tier-parity.md](runtime/rust-vm-tier-parity.md) — the Rust
  bytecode VM's tcltest parity scoreboard against C Tcl 9.
- [tcl-conformance-harness.md](runtime/tcl-conformance-harness.md) — the
  shared C Tcl oracle and source-discovery contract, and the focused-versus-
  full tcltest execution model.
- [tcl-test-tiers.md](runtime/tcl-test-tiers.md) — the capability ladder
  (parsing → interpretation → fundamentals → control flow → I/O → platform
  features) that orders the work toward C tcltest parity.
- [tclvm-opcode-status.md](runtime/tclvm-opcode-status.md) — C Tcl 9.0
  bytecode instruction coverage for the TclVM.
- [trace-implementation.md](runtime/trace-implementation.md) — the
  `trace add variable` no-op gap and the per-`Var` TraceList design that
  closes it.

## rust/

How the native workspace is laid out, the rules a change to it is measured
against, and how it stays fast under an editor's keystroke load.

- [current-architecture.md](rust/current-architecture.md) — the crate graph
  as it stands: who owns what, the dependency direction that must not be
  violated, and the runtime shape of the native LSP server.
- [engineering-guide.md](rust/engineering-guide.md) — the rules every change
  under `rust/` is held to: C Tcl 9.0.4 as the reference standard,
  time-to-first-tokens, the ratified library and data-structure choices, and
  crate layering.
- [incremental-analysis.md](rust/incremental-analysis.md) — the per-item walk
  with cascade invalidation behind incremental analysis.
- [lsp-performance.md](rust/lsp-performance.md) — native LSP performance:
  results, optimisations, and how to measure.
- [lsp-runtime-and-transports.md](rust/lsp-runtime-and-transports.md) — the
  one protocol core and its three drivers: the `rt.rs` seam's native /
  browser / wasi arms, what each host owes them, the WASI stdin driver, and
  how the WASI module ships.
- [salsa-interned-gc.md](rust/salsa-interned-gc.md) — the interned garbage
  collector that keeps the per-keystroke interned keys in `tcl-lsp-db`
  bounded, the two ways to disable it by accident, and the guardrails that
  pin it.
- [target-architecture.md](rust/target-architecture.md) — the target design
  (zero-copy, single-parse, incremental-reuse, MVCC) and how the competing
  goals are reconciled.

## tclpkg/

The Rust package manager for Tcl.

- [architecture.md](tclpkg/architecture.md) — architecture overview,
  contracts, file-path anchors, and test anchors.
- [security.md](tclpkg/security.md) — sandboxing (the `tcl-sandbox` crate),
  operator hooks, and the layered, admin-lockable policy, with the
  supply-chain threat model that drives it.

## templates/

Starting shapes for a new design document.

- [templates/README.md](templates/README.md) — the templates (contract,
  reference, ownership matrix), what a design doc looks like here, and the
  checklist to run before merging one.

## lanes/

Tracking documents for work handed to a background agent.

- [lanes/README.md](lanes/README.md) — the protocol: tracking document,
  checkpoint commits, explicit-path staging, orchestrator pushes. A file in
  that folder means the work is in flight or was interrupted.

## spec-dsl-examples/

Worked `.tclspec` examples that exercise the SpecTcl DSL against real command
surfaces.

- [spec-dsl-examples/README.md](spec-dsl-examples/README.md) — the example
  index, and what each ported surface taught about the DSL.
- [tricky-surfaces.md](spec-dsl-examples/tricky-surfaces.md) — the acceptance
  rubric: the tricky Tcl surfaces (operator aliasing, TclOO corners,
  real-world options, paired tails, hooks) each example is ticked against.
