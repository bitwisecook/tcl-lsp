# v2.1.19

**2.x alpha — pre-release channel.**

This release advances the Rust-native compiler, analyser, language server,
runtime, and extension tooling. It remains opt-in through the VS Code
Marketplace **pre-release** channel, the JetBrains Marketplace **eap** channel,
or the assets on this GitHub release. The stable **1.x** line remains the
default.

## Compiler and runtime

- The compiler now has one registry-owned semantic invocation contract for
  effects, state transitions, dispatch dependencies, completion behaviour,
  executable intermediate representation, and WebAssembly backend selection.
  Optimisation and analysis consumers use typed registry facts instead of
  command-name branches.
- A semantic ahead-of-time optimisation foundation adds proof-carrying,
  dialect-aware native paths with guarded fallback to the exact evaluated
  argument vector. The new optimisation switches remain off by default while
  the proof surface expands.
- Tcl's release-dependent numeral grammar is now handled by one shared facility
  across syntax, analysis, optimisation, code generation, the virtual machine,
  and the WebAssembly runtime. This fixes nine defects, including Tcl 8 octal
  handling, `string is` options, frame-level parsing, and inconsistent constant
  folding.
- The virtual machine now covers all 191 Tcl 9.0.4 opcodes, including
  coroutine, TclOO, exception-range, variable, array, and introspection
  families.

## Language server and extensibility

- Computed `source` paths now resolve through `file normalize`, chained local
  constants, namespace variables, and cross-file constants. Document links
  anchor on the file-name token instead of painting across substitutions.
- W308 now recognises generated `oo::configurable` accessors and template
  methods supplied by known subclasses, while retaining warnings for genuine
  unknown methods.
- Registry semantic proofs now drive option, callback-arity, formal-parameter,
  lifecycle, dispatch, and optimisation decisions. The compiler explorer shows
  the durable world-state and proof evidence behind those decisions.
- SpecTcl packs can provide live, sandboxed hooks and folder-scoped overlays.
  The bundled EDA command libraries have moved to loadable `.tclspec` packs,
  and the Spec Studio can edit, validate, and render the same source format.

## Engineering

- The local and continuous-integration test surface is now split into fast
  smoke, deep, and explicitly manual exhaustive tiers. Consolidated integration
  binaries and smaller build artefacts reduce test startup time and disk use.
- Rust, TypeScript, Kotlin, editor, continuous-integration action, Wasmtime,
  Binaryen, and wasi-sdk dependencies have been refreshed. The TypeScript
  updates also clear the affected transitive dependency advisories.
- Design and user documentation has been rewritten around the current native
  Rust architecture, with obsolete Python-era APIs and port narratives removed.

## Performance across the 2.1 pre-releases

These graphs include every published `2.1.x` pre-release from `v2.1.0` through
`v2.1.19`. There is no `v2.1.2` point because that version was never released.
The benchmark corpus, scope, and revision are fixed across the series.

Runs through `v2.1.16` were recorded on the maintainer's Apple M1 Max; later
runs use four-core GitHub Linux runners. The host change is visible in the
series, so compare CPU and wall time within a host era rather than treating the
boundary as a product-only delta. The raw result and generated summary are
attached to this release.

### Resident memory

![Resident memory across all 2.1 pre-releases](https://github.com/bitwisecook/tcl-lsp/releases/download/v2.1.19/perf-memory.svg)

### CPU utilisation

![CPU utilisation across all 2.1 pre-releases](https://github.com/bitwisecook/tcl-lsp/releases/download/v2.1.19/perf-cpu.svg)

### Per-check wall time

![Per-check wall time across all 2.1 pre-releases](https://github.com/bitwisecook/tcl-lsp/releases/download/v2.1.19/perf-walltime.svg)

[Benchmark table and method notes](https://github.com/bitwisecook/tcl-lsp/releases/download/v2.1.19/perf-summary.md)
