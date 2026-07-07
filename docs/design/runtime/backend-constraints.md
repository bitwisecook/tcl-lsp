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
| `runtime`        | interpreter implementation                                 | `bytecode`, `treewalk` |
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

## Overlay — `tests/external/backend_constraints.tcl`

A tcltest overlay, sourced **after** `package require tcltest` and **before**
the `.test` file. It:

1. reads the introspection facts (defaulting to `native` when absent, so it is
   safe under C tclsh or an older build);
2. registers backend constraints — `native`, `wasm`/`notWasm`,
   `wasi`/`notWasi`, `ebpf`/`notEbpf`, the runtime-impl constraints
   (`bytecodeRuntime`, `treewalkRuntime`), and spec-version gates (`wasm2`,
   `wasiPreview1`, `wasiPreview2`);
3. feeds `tcltest::configure -skip` the test-id globs the current backend
   cannot run, from a data-driven exclusion table ordered by capability area.

### Running with the overlay

Both harnesses source the overlay when `TCL_BACKEND_CONSTRAINTS` points at it:

```sh
# tcl-vm bytecode VM, evaluated as the WASI backend
TCL_BACKEND_CONSTRAINTS=tests/external/backend_constraints.tcl \
TCL_WASI_SPEC=preview1 \
  cargo run -p tcl-vm --release --example run_test -- tmp/tcl9.0.3/tests/socket.test

# runtime/rust tree-walk runtime, evaluated as eBPF
TCL_BACKEND_CONSTRAINTS=tests/external/backend_constraints.tcl \
TCL_EBPF_SPEC=1.4 \
  cargo run --release --example run_script -- --init tmp/tcl9.0.3/tests/clock.test
```

A native run (no `TCL_*_SPEC`) skips nothing and runs the full suite.

### Adding an exclusion

Edit the `Exclusions` table in the overlay. Each row is
`{globs {backends...} reason}`: a test whose name matches any glob is skipped
when the current backend is in `{backends...}`. Keep rows ordered by capability
area and cite *why* — the table is meant to grow as backend gaps are
characterised, and the reason is the audit trail for why a test is not run.

## Relationship to the tier ladder

Backend constraints bite hardest at the top of the
[test-tier ladder](tcl-test-tiers.md): Tier 1–4 (parsing, interpretation,
fundamentals, control flow) are pure computation and run on every backend;
Tier 5 (I/O) and Tier 6 (platform features) are where capabilities diverge and
the overlay does its skipping.
