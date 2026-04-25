# Tcl-WASM vs Tcl 9.0 Performance & Correctness Report

**Date:** 2026-04-25
**Branch:** `claude/tcl-wasm-performance-profile-QP0yH`

This report compares the in-tree WASM runtime
(`runtime/zig/`, ReleaseFast, 2.5 MB) against a freshly-built C
`tclsh` 9.0.3 (`tmp/tcl9.0.3/unix/tclsh`) on three workloads:

1. The 14 sample scripts in `samples/tcl/` — small, real-world
   Tcl snippets.
2. Stress / micro benchmarks of common primitives (`set`, `incr`,
   `expr`, `proc`, `foreach`, `dict`, `append`).
3. **97 fundamental tcltest files** from `tmp/tcl9.0.3/tests/`
   (the same `_IN_SCOPE` list `tests.external.run_tcl9_tests`
   uses) — `parse`, `list`, `dict`, `string`, `expr`, `control`,
   `var`, `namespace`, `proc`, `oo`, `eval`, `cmd-dispatch`, …
   Skipped: I/O, sockets, threads, fs, encoding, platform-
   specific, `clock`.

## How to read these numbers

- **wasm runs through Python wasmtime bindings** — every "wasm
  median" includes ≈ 9 ms of wasmtime store + linker setup per
  call. That cost is amortised away in the stress / microbench
  tables by subtracting a no-op baseline.
- **tclsh runs as a fresh subprocess** — every "tclsh median"
  includes ≈ 72 ms of process spawn + libc + interp init. The
  microbench table subtracts that baseline.
- **All wasm numbers are ReleaseFast.** Debug mode is ≈ 6×
  slower (see [`04-debug-vs-release.md`](04-debug-vs-release.md))
  and is only useful for development.

## Sub-reports

| # | File | Topic |
|---|---|---|
| 1 | [`01-end-to-end.md`](01-end-to-end.md) | Per-sample single-shot runs |
| 2 | [`02-stress.md`](02-stress.md) | Amplified-iteration runs |
| 3 | [`03-microbench.md`](03-microbench.md) | Per-Tcl-primitive cost |
| 4 | [`04-debug-vs-release.md`](04-debug-vs-release.md) | Zig `Debug` vs `ReleaseFast` |
| 5 | [`05-correctness.md`](05-correctness.md) | Bugs and divergences from tclsh |
| 6 | [`06-hotspots.md`](06-hotspots.md) | Source-level hot-spot analysis |
| 7 | [`07-recommendations.md`](07-recommendations.md) | Prioritised list of changes |
| 8 | [`08-tcltest-suites.md`](08-tcltest-suites.md) | full Tcl 9 in-scope tcltest sweep |
| 9 | [`09-after-action.md`](09-after-action.md) | **NEW** — deltas from running the master plan's phases 0–6 |

## Headline numbers

### tcltest sweep — baseline → after master-plan phases 0–6

| | files | tests passed / total | pass % | run time |
|---|---:|---:|---:|---:|
| **WASM (baseline)** | 97 | 355 / 35,921 | 1.00 % | 13.2 s |
| **WASM (after)** | 97 | **384 / 35,921** | **1.07 %** | (varies — see 09) |
| **tclsh** | 97 | 32,695 / 35,921 | 91.0 % | 54.9 s |

After-pass details: [`09-after-action.md`](09-after-action.md).
Headline run-trap count went from 49 → **45** (4 fewer files).

- **1 file passes 100 %** on WASM (`concat.test`, 9/9).
- **47 files run partially** — usable signal, mixed pass rates.
- **49 files trap mid-run** — the runtime aborts before
  `tcltest::cleanupTests` prints the summary.
- WASM wall time is lower only because traps cut runs short;
  individual successful runs are mostly faster than tclsh by 0.6
  – 0.9× — see [`08-tcltest-suites.md`](08-tcltest-suites.md).

### Per-subsystem trap-vs-pass map (top-level read)

| subsystem | files | wasm pass | wasm partial | wasm trap | wasm test pass% |
|---|---:|---:|---:|---:|---:|
| cmd-dispatch | 3 | 0 | 0 | 3 | 0.0% |
| control | 9 | 0 | 6 | 3 | 3.5% |
| coroutine | 3 | 0 | 3 | 0 | 0.0% |
| dict | 1 | 0 | 0 | 1 | 0.0% |
| eval | 4 | 0 | 1 | 3 | 2.4% |
| expr | 5 | 0 | 1 | 4 | 0.6% |
| interp | 5 | 0 | 1 | 3 | 0.0% |
| list | 17 | 0 | 9 | 8 | 0.9% |
| misc | 15 | **1** | 9 | 4 | 6.9% |
| object | 4 | 0 | 2 | 2 | 0.0% |
| parsing | 5 | 0 | 2 | 3 | 6.4% |
| proc | 7 | 0 | 4 | 2 | 5.4% |
| string | 10 | 0 | 5 | 5 | 1.8% |
| variable | 9 | 0 | 4 | 5 | 4.8% |

### Per-primitive cost (Tcl 9.0.3 baseline subtracted)

| Op | wasm (ns/op) | tclsh (ns/op) | wasm/tclsh |
|---|---:|---:|---|
| `incr x` (loop var) | **120** | 445 | **3.7× faster** |
| `if/else` branch | **222** | 423 | **1.9× faster** |
| `set` + read | **232** | 380 | **1.6× faster** |
| `proc` call (3 args, expr body) | **252** | 320 | **1.3× faster** |
| `proc` call (no args) | 153 | **48** | **3.2× slower** |
| `expr` arithmetic in tight loop | TRAP @ 100k | 380 | bump-allocator OOM |
| `lappend` + `lindex` | TRAP @ 20k | — | bump-allocator OOM |
| `dict set` / `dict get` | TRAP @ 5k | — | bump-allocator OOM |
| `append s x` (string-grow) | 2,568 | < baseline noise | **O(N²)** |

Full table and methodology in [`03-microbench.md`](03-microbench.md).

## Sample scripts (tldr)

11 / 14 samples run cleanly on WASM with output matching tclsh
(samples 1–3, 8, 11–13 + `compiler_explorer_demo`). Samples
4 (`clock format`), 5 (parser permissiveness), 7
(intentional errors), 9 (TclOO), 10 (`format %2$s`) hit known
gaps; sample 6 self-recurses via `source $argv0` and is
skipped on both backends. Detail in
[`01-end-to-end.md`](01-end-to-end.md) and
[`05-correctness.md`](05-correctness.md).

## Where to spend optimisation effort

The **tcltest sweep changes the priority ranking** from the
sample-only analysis. Updated top three:

1. **Bump allocator gives back recycled / overlapping memory** —
   not just an OOM at scale. The
   `unknown command: <garbage bytes>` traps in 9 tcltest files
   prove that the dispatcher receives `(ptr, len)` pairs that
   point into stale storage. This is a correctness blocker, not
   only a perf issue. Fix: refcount-driven free-lists per
   size class + page growth on heap-pointer overflow.
   [`06-hotspots.md`](06-hotspots.md), [`07-recommendations.md`](07-recommendations.md) §R1.

2. **`tcl_cmd_append` is O(N) per call** —
   `valtypes/tcl_string.zig:22`. Every `append` loop is
   quadratic. Affects `append.test`, `appendComp.test`, and
   any tcltest that builds output incrementally.
   [`07-recommendations.md`](07-recommendations.md) §R2.

3. **`frame_push` zeros 4 KB per proc call AND the table is
   fixed-capacity** — `interp/tcl_frames.zig`. Fixed cost is
   ~100 ns of the 153 ns no-arg proc call (perf), and the
   256-bucket table overflows on `set.test`, `incr.test`,
   `execute.test` (correctness). Fix: shrink default frame +
   dirty-bitmap clearing + grow on demand.
   [`07-recommendations.md`](07-recommendations.md) §R3.

Also surfaced by the tcltest sweep:

4. **`tcltest::cleanupTests` traps in 5 files** — runtime
   crashes during the post-test cleanup walk over the command
   table. Likely a missing `info commands` filter combination.
   Cheap fix once isolated; would convert several `run-trap`
   files to `partial`.
5. **`source filename`** is unsupported — three test files
   need it for helper data; easy unblock once WASI fd
   resolution lands.
6. **Constraint initialiser arity surface** — `parseExpr`,
   `dict` test bundles trap with
   `ConstraintInitializer must be complete script`; suggests
   `tcltest::testConstraint` initialiser dispatch isn't
   matching ours.

## Summary in one sentence

When the WASM runtime succeeds, it's already at parity or
ahead of tclsh on per-op cost (1.3 – 3.7× faster on warm
loops, 0.6× wall time on the one tcltest file that passes
end-to-end), but the in-scope tcltest sweep shows it can
only drive **355 of 35,921** real Tcl tests to a pass
result today — the gating issue is the allocator / heap
hygiene, with `append` and per-call frame cost as the
runner-up perf items.
