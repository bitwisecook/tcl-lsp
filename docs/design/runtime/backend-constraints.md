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

**The hook is wired in both harnesses; no overlay script is checked in.**
Setting `TCL_BACKEND_CONSTRAINTS` to a path sources that file at the right
moment, but the repository ships no such file, and no backend constraint names
are registered anywhere in the tree. Until an overlay exists, every backend
runs the full suite and reports capability gaps as failures rather than skips.

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
TCL_BACKEND_CONSTRAINTS=<overlay.tcl> \
TCL_WASI_SPEC=preview1 \
  cargo run -p tcl-vm --release --example run_test -- tmp/tcl9.0.3/tests/socket.test

# runtime/rust tree-walk runtime, evaluated as eBPF
TCL_BACKEND_CONSTRAINTS=<overlay.tcl> \
TCL_EBPF_SPEC=1.4 \
  cargo run --release --example run_script -- --init tmp/tcl9.0.3/tests/clock.test
```

A native run (no `TCL_*_SPEC`) has nothing to skip and runs the full suite.

### What an overlay has to do

1. Read the introspection facts, defaulting to `native` when absent, so it is
   safe under C tclsh or an older build.
2. Register the backend constraints the exclusion rows name.
3. Feed `tcltest::configure -skip` from a data-driven exclusion table ordered
   by capability area, each row carrying the *reason* — the audit trail for why
   a test is not run.

## Relationship to the tier ladder

Backend constraints bite hardest at the top of the
[test-tier ladder](tcl-test-tiers.md): Tier 1–4 (parsing, interpretation,
fundamentals, control flow) are pure computation and run on every backend;
Tier 5 (I/O) and Tier 6 (platform features) are where capabilities diverge and
the overlay does its skipping.
