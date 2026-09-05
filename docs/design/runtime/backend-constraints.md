# Backend test constraints — running the tcltest suite per target

The Rust interpreters target several backends with different capabilities:

| backend    | crate            | filesystem | sockets | exec | threads | clock |
|------------|------------------|:----------:|:-------:|:----:|:-------:|:-----:|
| native     | `tcl-vm`, `runtime/rust` | ✓ | ✓ | ✓ | ✓ | ✓ |
| wasm/WASI  | `runtime/rust` (wasm32-wasip1) | ✓ | ✗ | ✗ | ✗ | ✓ |
| eBPF       | `bpf-tcl` (DSL subset → eBPF)   | ✗ | ✗ | ✗ | ✗ | ✗ |

The upstream Tcl 9 tcltest suite is a fixed contract — we never edit
`tcltest.tcl`, `init.tcl`, or any `.test` file. A test that needs a capability a
backend lacks must therefore be **skipped**, not failed, so the backend's
pass / fail / skip line matches what that backend can honestly run. C tclsh
does the same thing with its `win` / `unix` / `nonPortable` constraints.

Two pieces make this work: runtime **introspection** (what am I?) and a
loadable **overlay** (skip what I can't run).

## Introspection — `tcl_platform` keys

Each interpreter publishes these keys in the standard `tcl_platform` array,
after the usual fields. The schema and the compiled-in (cfg-detected) values
live in `tcl-platform::backend`; both `tcl-vm` and `runtime/rust` publish
them from their startup-globals path.

| key              | meaning                                                    | example          |
|------------------|------------------------------------------------------------|------------------|
| `runtime`        | interpreter implementation                                 | `bytecode`, `treewalk`, `ebpf` |
| `runtimeVersion` | that implementation's host (crate) version                 | `0.1.0`          |
| `wasm`           | wasm spec version on a wasm build, else empty              | `2.0`            |
| `wasi`           | WASI spec version on a WASI build, else empty              | `preview1`, `0.2`|
| `wasiVersion`    | WASI host / preview identifier, else empty                 | `wasip1`         |
| `ebpf`           | eBPF target version, else empty                            | `1.4`            |

An empty string means "not this target". A native build reports
`runtime`/`runtimeVersion` and leaves `wasm`/`wasi`/`ebpf` empty.

### Environment overrides

Each fact may be overridden from the environment before it is published:

| key           | override variable    |
|---------------|----------------------|
| `wasm`        | `TCL_WASM_SPEC`      |
| `wasi`        | `TCL_WASI_SPEC`      |
| `wasiVersion` | `TCL_WASI_VERSION`   |
| `ebpf`        | `TCL_EBPF_SPEC`      |

This lets a **native** binary evaluate another backend's skip lists — the only
way to reason about the **eBPF** target, which is a DSL-subset compiler
(`bpf-tcl`) and cannot host a full interpreter at all. For example,
`TCL_EBPF_SPEC=1.4 run_test foo.test` runs `foo.test` natively but skips
everything the eBPF backend could not.

## Overlay — the `TCL_BACKEND_CONSTRAINTS` hook

The second half is a tcltest overlay: a Tcl script that reads the
introspection facts, registers backend constraints, and feeds
`tcltest::configure -skip` the test-id globs the current backend cannot run.

**The overlay is checked in at `tests/external/backend_constraints.tcl`**, and
`TCL_BACKEND_CONSTRAINTS` points a harness at it (or at a replacement). It
registers seven constraint names — `rustBackend`, `bytecodeRuntime`,
`treewalkRuntime`, `wasmBackend`, `notWasm`, `wasiPreview1`, `ebpfBackend` —
and is safe to source under C tclsh, because every fact it reads defaults when
the key is absent.

The tcltest sweep passes it by default: `rust/xtask/src/tcltest_sweep.rs`
names it in the `BACKEND_CONSTRAINTS` const, sets the env var for every VM
run, and *refuses to sweep at all* without it
(`missing default backend constraint overlay …`). The two example harnesses do
not default it — set `TCL_BACKEND_CONSTRAINTS` yourself for a manual
`run_test` or `run_script --init`.

Where each harness sources it:

- `rust/tcl-vm/examples/run_test.rs` builds the driver script
  `source <tcltest>` → `namespace import -force ::tcltest::*` → *overlay* →
  `source <testfile>`, so the overlay lands after tcltest is available and
  before the test file's own body.
- `runtime/rust/examples/run_script.rs` pre-loads tcltest itself
  (`package require tcltest`, then the import) and sources the overlay before
  the script, but **only when `--init` is passed**; an overlay error aborts the
  run with a diagnostic rather than continuing unconstrained.

```sh
# tcl-vm bytecode VM, evaluated as the WASI backend
TCL_BACKEND_CONSTRAINTS=tests/external/backend_constraints.tcl \
TCL_WASI_SPEC=preview1 \
  cargo run -p tcl-vm --release --example run_test -- tmp/tcl9.0.4/tests/socket.test

# runtime/rust tree-walk runtime, evaluated as eBPF
TCL_BACKEND_CONSTRAINTS=tests/external/backend_constraints.tcl \
TCL_EBPF_SPEC=1.4 \
  cargo run --release --example run_script -- --init tmp/tcl9.0.4/tests/clock.test
```

A native run (no `TCL_*_SPEC`) has nothing to skip and runs the full suite.

### What the overlay does

1. Reads `::tcl_platform(runtime)` (defaulting to `c`) and `(wasm)` / `(wasi)`
   / `(ebpf)` (defaulting to empty), so the file is safe under C tclsh.
   `runtime/rust` publishes `runtime = "treewalk"` and `tcl-vm` publishes
   `"bytecode"`; `tcl-platform` owns the schema.
2. Registers the seven constraint names:

   | Name | True when |
   |---|---|
   | `rustBackend` | `runtime` is `bytecode`, `treewalk`, or `ebpf` |
   | `bytecodeRuntime` | `runtime eq "bytecode"` |
   | `treewalkRuntime` | `runtime eq "treewalk"` |
   | `wasmBackend` / `notWasm` | `wasm ne ""` / `wasm eq ""` |
   | `wasiPreview1` | `wasi` is `preview1` or `0.1` |
   | `ebpfBackend` | `ebpf ne ""` |

3. Appends to any existing `::tcltest::configure -skip`, one row per backend
   boundary: `platform-1.1` (the Rust runtimes add introspection keys, so C's
   exact-key assertion is inapplicable); `socket-*` when there is no `socket`
   command; `exec-*` when there is no `exec`, or under WASI or eBPF;
   `thread-* async-*` when `tcl_platform(threaded)` is absent or false;
   `fCmd-* fileSystem-*` under eBPF.

The authoring rule is in the file's own header and is **gated**: an overlay may
exclude only platform identity, unavailable host capabilities, and C-only
internal-representation probes. Semantic stems (`set-*`, `expr-*`, `proc-*`,
`namespace-*`, `dict-*`) are forbidden.
`tcltest_sweep.rs::tests::overlay_only_skips_explicit_backend_cases` pins every
`lappend exclusions` pattern against `ALLOWED_BACKEND_SKIP_PATTERNS` and fails
on a semantic stem, so an overlay edit must update that list in the same
change.

## Relationship to the tier ladder

Backend constraints bite hardest at the top of the
[test-tier ladder](tcl-test-tiers.md): Tier 1–4 (parsing, interpretation,
fundamentals, control flow) are pure computation and run on every backend;
Tier 5 (I/O) and Tier 6 (platform features) are where capabilities diverge and
the overlay does its skipping.
