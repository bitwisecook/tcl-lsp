# v2.1.24

**2.x alpha — pre-release channel.**

This prerelease makes the Spec Studio and Compiler Explorer reference surfaces
easier to navigate, keeps their embedded language tooling aligned with the
compiler registry, and strengthens the release path. It remains opt-in through
the VS Code Marketplace **pre-release** channel, the JetBrains Marketplace
**eap** channel, or the assets on this GitHub release. The stable **1.x** line
remains the default.

## Spec Studio and Compiler Explorer

- Compiler traits now have one typed, closed registry definition for their
  names, groups, author-facing documentation, and annotated examples. New or
  unknown trait groups fail at compile time, while Spec Studio and Compiler
  Explorer receive the same generated reference automatically.
- Spec Studio replaces the flat trait-label bucket with grouped, searchable
  toggles. Every help and reference entry includes a small Tcl example with
  callouts showing where the trait, taint rule, or other behaviour applies.
- Standalone web interfaces use Monaco exclusively, with the SpecTcl/Tcl 9
  language configuration, semantic highlighting, and hover available from the
  initial render. Editor extensions keep their native file tabs and place the
  Studio or Explorer beside them.

## Release delivery

- Every `v*` release tag now deploys the unified Pages site from the exact
  tagged commit, while branch deployments retain their path filtering. A drift
  gate protects the trigger and concurrency contract.
- Post-release installer verification now affirmatively installs and validates
  MCP and Claude skills in an isolated temporary home, including an offline
  checksum, protocol, version, skills, and cleanup regression test.

# v2.1.23

**2.x alpha — pre-release channel.**

This prerelease hardens the Rust-native compiler, runtimes, language server,
SpecTcl packs, and editor integrations. It remains opt-in through the VS Code
Marketplace **pre-release** channel, the JetBrains Marketplace **eap** channel,
or the assets on this GitHub release. The stable **1.x** line remains the
default.

## Tcl, compiler, and runtime correctness

- Tcl 8.x and Tcl 9.x `${...}` parsing now follows one release-aware rule
  across lexing, expressions, substitution, analysis, refactors, optimisation,
  code generation, and both execution engines. Nested Tcl 9 names no longer
  truncate or inherit an 8.x closing-brace rule.
- Namespace, ensemble, TclOO, variable, trace, list, and dictionary behaviour
  is aligned more closely with C Tcl. This includes prefix resolution,
  namespace teardown, export/import options, element references, trace order,
  last-value-wins dictionaries, and command-binding-aware constant folding.
- Compiler and analyser fixes cover quoted whole-command evaluation, braced
  loop lists, lambda and untyped-body fall-through, computed metaclasses,
  versioned command arguments, and deeply nested source walks without stack
  overflow.

## Language server, SpecTcl, and editors

- Diagnostic publishing now uses a persistent, latest-wins per-URI mailbox and
  performs client I/O outside document-ordering locks. A stalled publish can no
  longer freeze edits and unrelated requests across the language-server
  session; retained telemetry makes any residual scheduler stall
  self-diagnosing.
- Cross-file analysis no longer strands callers on W123 while a definition is
  opening, or repeatedly re-wakes a deleted-on-disk buffer after facts move.
  Extension-host liveness verdicts now distinguish load, document-pipeline
  stalls, and whole-server wedges with process and thread evidence.
- SpecTcl packs gain versioned arity, per-argument lifecycle rows, ambient
  package versions, robust malformed-pack notices, deterministic collision
  handling, and live pack-declared file-extension registration. VS Code keeps
  user associations intact while adding and retiring pack associations.
- Editor language IDs, filename patterns, and dialect associations now come
  from the shared catalogue, closing drift across VS Code, JetBrains, Sublime,
  and the command-line tools.

## WebAssembly and delivery

- The browser language server uses a JavaScript-backed clock instead of native
  time primitives that trap in WebAssembly, and the import-free VM build stays
  import-free.
- Pull requests now build and execute the browser language-server WebAssembly
  target, including its complete local Cargo dependency closure, so Pages-only
  failures are caught before merge.
- The workspace is clean on Rust 1.98, runtime formatting is part of the Rust
  gate, and generated and satellite WebAssembly surfaces have stronger drift
  and real-link coverage.

# v2.1.22

**2.x alpha — pre-release channel.**

This maintenance prerelease carries forward v2.1.21 with refreshed release
performance measurements and the portable source-data build tooling.

## Engineering and delivery

- Release verification remains portable across GNU and BSD host utilities,
  including the PEM source-data generation path.

For the complete feature list, see the [v2.1.21 release notes](https://github.com/bitwisecook/tcl-lsp/releases/tag/v2.1.21).

# v2.1.21

**2.x alpha — pre-release channel.**

This maintenance prerelease carries forward the v2.1.20 Rust-native compiler,
analyser, language server, runtime, and editor tooling while fixing the release
packaging path for the embedded Spec Studio.

## Engineering and delivery

- VS Code and JetBrains release jobs now install the pinned
  `wasm-bindgen-cli` and stable Rust toolchain required to build the embedded
  language-server worker, so both editor artefacts are produced reliably.

For the complete feature list, see the [v2.1.20 release notes](https://github.com/bitwisecook/tcl-lsp/releases/tag/v2.1.20).

# v2.1.20

**2.x alpha — pre-release channel.**

This release advances the Rust-native compiler, analyser, language server,
runtime, and extension tooling. It remains opt-in through the VS Code
Marketplace **pre-release** channel, the JetBrains Marketplace **eap** channel,
or the assets on this GitHub release. The stable **1.x** line remains the
default.

## Compiler and runtime correctness

- Optimisation now respects dynamic variable observation, trace state,
  executable control-flow edges, computed names, frame reach, and NaN-sensitive
  comparisons. This closes a batch of silent-miscompile cases while retaining
  guarded fast paths when their proofs hold.
- Tcl release selection now drives lexing, escape decoding, expression
  operators and numerals, formatting, command availability, compilation, and
  host execution end to end. Tcl 8.4 through 9.1 programs no longer inherit
  syntax or built-ins from the host's default release.
- Shared syntax owners now handle substitution literals, parameter lists,
  word closers, case-list layouts, expression numeral boundaries, comments,
  and iRules event regions. Compiler, analyser, formatter, diagram, and LSP
  consumers use those facts instead of maintaining divergent parsers.
- The virtual machine preserves command-surface and profile identity through
  aliases, hide/expose, rename, imports, coroutines, child interpreters, traces,
  and cached bytecode. Switch, try/return, expression, and host-fallback paths
  now retain their exact Tcl completion and dialect behaviour.

## Language server, iRules, and extensibility

- iRules event discovery, symbols, semantic tokens, comments, selection,
  diagnostics, and compilation now share top-level, registry-resolved event
  boundaries. Nested or unreachable declarations are excluded, exact command
  identity survives aliases, and conflicting event priorities are diagnosed.
- Dialect strings are resolved once at CLI and LSP ingress and carried as typed
  profiles. Editor dialect choices and lexical grammars are generated from the
  same registry and syntax owners for VS Code, JetBrains, and Sublime.
- Registry lifecycle facts model Tcl startup globals for W210 without hiding
  genuine undefined locals or values invalidated by `unset`. Other LSP
  features now consume shared case, frame, reference, and embedded-language
  contracts, including canonical Windows diagnostic URIs.
- Project SpecTcl packs apply consistently across the CLI and VS Code, with
  version ranges and release history enforced throughout their lifecycle. Spec
  Studio gains native editor hosts, import/export provenance, formatting,
  highlighting, and a Monaco-backed Pack DSL client whose LSP readiness is
  explicitly verified.

## BIG-IP and TLS assurance

- A new safe, non-executing SslicTcl engine models certificates, keys, chains,
  trust programmes, protocols, ciphers, HSTS, endpoint evidence, nginx,
  OpenSSL, and testssl.sh input. Its pinned Chromium and Trust Stores
  Observatory data is embedded with deterministic provenance and offline
  verification.
- BIG-IP reports now project effective client- and server-SSL configuration,
  correlate certificate and key material, validate object references, and
  surface TLS assurance and chain findings across native, compatibility, and
  WebAssembly outputs. Unknown evidence remains explicitly unknown rather than
  receiving an optimistic grade.

## Engineering and delivery

- The native installer is stricter and more portable, including archive
  verification, platform selection, upgrade behaviour, and a broader harness.
  CI cache-warming and VS Code test downloads are corrected and faster.
- New owner-resolution, editor-generator, documentation-link, diagnostic-tag,
  iRule-test-data, and tooling drift gates keep generated assets and semantic
  contracts aligned with their Rust sources of truth.
- Differential-fuzz campaigns now pin one Tcl release across both engines and
  replay it from durable, race-safe finding records, preventing deliberate
  cross-release differences or incomplete record pairs from being reported as
  findings.

## Performance across the 2.1 pre-releases

These graphs cover every measured `2.1.x` pre-release from `v2.1.0` through
`v2.1.24`, run by `scripts/perf/` against a pinned 113-file corpus (scope
`small`, revision `1`). The corpus, scope, and revision are fixed across the
whole series, so the lines are comparable with each other.

There is no `v2.1.2` point: that version was never released.

**`2.1.24` is this release**: it is the bright-blue line in the memory and CPU
graphs and the rightmost bar in each wall-time group. Earlier releases are
drawn in grey and fade with age.

The series spans more than one measurement host — Apple M1 Max (darwin-arm64);
AMD EPYC 9V74 80-Core Processor (linux-x86_64); AMD EPYC 7763 64-Core
Processor (linux-x86_64); Apple M5 Max (darwin-arm64). Wall time and CPU are
properties of the machine as much as of the build, so compare within a host
era rather than reading the boundary as a product change. Resident memory is
far less host-sensitive.

### Resident memory

![Resident memory across the 2.1 pre-releases](https://github.com/bitwisecook/tcl-lsp/releases/download/v2.1.24/perf-memory.svg)

### CPU utilisation

![CPU utilisation across the 2.1 pre-releases](https://github.com/bitwisecook/tcl-lsp/releases/download/v2.1.24/perf-cpu.svg)

### Per-check wall time

![Per-check wall time across the 2.1 pre-releases](https://github.com/bitwisecook/tcl-lsp/releases/download/v2.1.24/perf-walltime.svg)

[Benchmark table and method
notes](https://github.com/bitwisecook/tcl-lsp/releases/download/v2.1.24/perf-summary.md)
— the raw result JSON is attached to this release as `perf-2.1.24.json`.
