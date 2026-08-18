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
`v2.1.21`, run by `scripts/perf/` against a pinned 113-file corpus (scope
`small`, revision `1`). The corpus, scope, and revision are fixed across the
whole series, so the lines are comparable with each other.

There is no `v2.1.2` point: that version was never released.

**`2.1.21` is this release**: it is the bright-blue line in the memory and CPU
graphs and the rightmost bar in each wall-time group. Earlier releases are
drawn in grey and fade with age.

The series spans more than one measurement host — Apple M1 Max (darwin-arm64);
AMD EPYC 9V74 80-Core Processor (linux-x86_64); AMD EPYC 7763 64-Core
Processor (linux-x86_64); Apple M5 Max (darwin-arm64). Wall time and CPU are
properties of the machine as much as of the build, so compare within a host
era rather than reading the boundary as a product change. Resident memory is
far less host-sensitive.

### Resident memory

![Resident memory across the 2.1 pre-releases](https://github.com/bitwisecook/tcl-lsp/releases/download/v2.1.21/perf-memory.svg)

### CPU utilisation

![CPU utilisation across the 2.1 pre-releases](https://github.com/bitwisecook/tcl-lsp/releases/download/v2.1.21/perf-cpu.svg)

### Per-check wall time

![Per-check wall time across the 2.1 pre-releases](https://github.com/bitwisecook/tcl-lsp/releases/download/v2.1.21/perf-walltime.svg)

[Benchmark table and method
notes](https://github.com/bitwisecook/tcl-lsp/releases/download/v2.1.21/perf-summary.md)
— the raw result JSON is attached to this release as `perf-2.1.21.json`.
