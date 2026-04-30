# KCS: How do the `TCL_LSP_RUST_*` env vars work?

> **Audience:** Contributor
> **Type:** Q&A

## Applies to

all-editors, tcl-lsp-cli

## Question

What do the `TCL_LSP_RUST_*` environment variables do, and when
should I set them?

## Answer

The Python-to-Rust rewrite (see
[`docs/rust-rewrite.md`](../../docs/rust-rewrite.md)) is landing in
chunks. As each chunk's Rust port becomes feature-complete, the
Python side gains a small dispatch shim that routes work to the
Rust binding. Default-on shims always prefer the Rust binding when
it is importable; default-off shims still require an explicit opt-in
via the named env var. In every case the dispatcher falls back to
the Python implementation if (a) the binding isn't installed or
(b) the Rust path raises an exception.

The current default for each shim is given in the tables below.
Default-on shims (`TCL_LSP_RUST_SIGNATURE_SCAN`,
`TCL_LSP_RUST_ANALYSER`) are **not** opt-in experiments — they ship
as the canonical path, and the env var is the opt-out knob you set
to `0` when bisecting a regression or running the Python side as
the differential oracle.

Each env var is recognised in the same vocabulary: truthy values
(`1`, `true`, `yes`, `on`, `y`, `t` — case-insensitive) opt the
subsystem into the Rust pipeline; falsy values (`0`, `false`, `no`,
`off`, `n`, `f`) opt out; an unset / empty / unrecognised value
falls through to the chunk's current default. Each subsystem's
default flips to **on** after its parity has baked in default-off
mode for a release cycle; the env var stays in place as the opt-out
knob until the Python implementation retires entirely.

> Folding (and any future LSP feature provider in `tcl-lsp-core`) is
> a separate case: the Python dispatcher imports the Rust function
> unconditionally and uses it whenever the wheel is installed, with
> no env-var gate. Those paths appear in
> [`docs/design/rust/current-architecture.md`](../design/rust/current-architecture.md)
> under "Authoritative Rust paths" rather than in the tables below.

### Default-on (Rust by default; opt out via `=0`)

| Env var                          | Subsystem                 | Module wired                                   | Flipped in |
|----------------------------------|---------------------------|------------------------------------------------|------------|
| `TCL_LSP_RUST_SIGNATURE_SCAN`    | Background signature scan | `core/analysis/signature_scan.py`              | C40-default-on |
| `TCL_LSP_RUST_ANALYSER`          | Single-pass Tcl analyser  | `core/analysis/_analyser/__init__.py`          | C41-default-on |

### Default-off (opt in via `=1`)

| Env var                          | Subsystem                 | Module wired                                   |
|----------------------------------|---------------------------|------------------------------------------------|
| `TCL_LSP_RUST_OPTIMISER`         | Optimiser pass manager    | `core/compiler/optimiser/_manager.py`          |
| `TCL_LSP_RUST_INTERPROC`         | Interprocedural analysis  | `core/compiler/interprocedural.py`             |
| `TCL_LSP_RUST_GVN`               | GVN redundancy detection  | `core/compiler/gvn.py`                         |

The Rust path is gated on (a) the binding being importable and
(b) the env var resolving to "use Rust"; any exception from the
Rust path is logged at DEBUG and the Python path runs as a safety
net.

You should set one of these vars when:

- **Differential testing.** The `tests/test_rust_*_differential.py`
  harnesses run with the env var explicitly set (truthy for
  default-off shims, falsy for default-on shims) so each path is
  exercised under realistic dispatch.
- **Local benchmarking.** Profile-comparing Python vs Rust on a
  workload before promoting a chunk to default-on, or after a
  default-on flip when investigating a perf regression.
- **Reproducing a Rust-side bug.** Set the var, replay the
  workload, observe the difference. For default-on shims, set
  `=0` to confirm the bug disappears under the Python fallback.

You should **not** set the **default-off** vars in production / CI
runs that aren't exercising the Rust ports — those subsystems are
still Python-authoritative until each chunk's default-on flip
lands. Default-on shims, on the other hand, are the canonical
shipping path and need no opt-in.

When a chunk's default-on flip lands, the env var inverts (becomes
an opt-out under the same name with value `0`); after a release
cycle the var is removed entirely along with the Python
implementation. The chunk-log table in
[`docs/rust-rewrite.md`](../../docs/rust-rewrite.md) tracks the
status (`landed (default-off env-var gate)` vs `landed (default-on)`
vs Python-retired).

## Related

- [Rust rewrite plan](../../docs/rust-rewrite.md)
- [Current Rust architecture](../design/rust/current-architecture.md)
- [Test-port audit](../../docs/rust-rewrite-test-audit.md)
- [KCS index](README.md)
