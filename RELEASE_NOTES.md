# v2.1.19

**2.x alpha — pre-release channel.**

This release advances the Rust-native compiler, analyser, language server,
runtime, and extension tooling. It remains opt-in through the VS Code
Marketplace **pre-release** channel, the JetBrains Marketplace **eap** channel,
or the assets on this GitHub release. The stable **1.x** line remains the
default.

The `v2.1.19` tag dereferences to commit
`54229c4f6cfd38bcb8efece7f797cbd8aca332bc`.

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

These graphs cover every measured `2.1.x` pre-release from `v2.1.0` through
`v2.1.19`, run by `scripts/perf/` against a pinned 113-file corpus (scope
`small`, revision `1`). The corpus, scope, and revision are fixed across the
whole series, so the lines are comparable with each other.

There is no `v2.1.2` point: that version was never released.

**`2.1.19` is this release**: it is the bright blue line — and the rightmost
bar in each group — in every graph below. Earlier releases are drawn in grey
and fade with age.

The series spans more than one measurement host — Apple M1 Max (darwin-arm64);
AMD EPYC 9V74 80-Core Processor (linux-x86_64); AMD EPYC 7763 64-Core
Processor (linux-x86_64). Wall time and CPU are properties of the machine as
much as of the build, so compare within a host era rather than reading the
boundary as a product change. Resident memory is far less host-sensitive.

### Resident memory

![Resident memory across the 2.1 pre-releases](https://github.com/bitwisecook/tcl-lsp/releases/download/v2.1.19/perf-memory.svg)

### CPU utilisation

![CPU utilisation across the 2.1 pre-releases](https://github.com/bitwisecook/tcl-lsp/releases/download/v2.1.19/perf-cpu.svg)

### Per-check wall time

![Per-check wall time across the 2.1 pre-releases](https://github.com/bitwisecook/tcl-lsp/releases/download/v2.1.19/perf-walltime.svg)

[Benchmark table and method
notes](https://github.com/bitwisecook/tcl-lsp/releases/download/v2.1.19/perf-summary.md)
— the raw result JSON is attached to this release as `perf-2.1.19.json`.
