# 08 — Tcl 9 tcltest suites: in-scope test files

## What this report covers

97 test files from `tmp/tcl9.0.3/tests/` that exercise the
fundamental Tcl semantics — the same list the project's existing
`tests/external/run_tcl9_tests.py::_IN_SCOPE` table picks. Skipped
by design: `chan*`, `socket`, `http*`, `fileSystem*`, `unix*`,
`win*`, `clock`, `event`, `notify`, `thread`, `mutex`, `pid`, etc.
— anything that needs sockets, real files, threads, or
platform-specific syscalls that WASM wasi-libc can't service yet.

For every in-scope file the harness:

1. Builds a tcltest bundle (`_bundle()` from
   `tests.external.run_tcl9_tests`) — concatenates Tcl 9's
   `library/tcltest/tcltest.tcl` + a small preamble + the test
   file, so the WASM runtime sees one self-contained script
   without needing `package require`.
2. Compiles the bundle to WASM and times it.
3. Runs the WASM and parses the standard tcltest summary line
   (`Total N Passed N Skipped N Failed N`).
4. Runs the same `.test` file directly under `tclsh` and parses
   the same summary line.

Per-file 60-second timeout to bound the worst case.

## Per-subsystem totals

Headline: **WASM passes 1.0% of all in-scope tests; tclsh passes
91.0%**. The gap is mostly traps that abort the run before the
summary line is printed, not high failure rates within runs.

| subsystem | files | wasm pass | wasm partial | wasm trap | wasm tests pass/total | tcl tests pass/total | wasm pass% | tcl pass% | wasm run (ms) | tcl run (ms) | wasm/tcl |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| cmd-dispatch | 3 | 0 | 0 | 3 | 0/0 | 17,079/17,266 | 0.0% | 98.9% | 360 | 6,868 | 0.05× |
| control | 9 | 0 | 6 | 3 | 25/202 | 666/720 | 3.5% | 92.5% | 881 | 966 | 0.91× |
| coroutine | 3 | 0 | 3 | 0 | 0/142 | 108/142 | 0.0% | 76.1% | 373 | 449 | 0.83× |
| dict | 1 | 0 | 0 | 1 | 0/0 | 367/373 | 0.0% | 98.4% | 72 | 361 | 0.20× |
| eval | 4 | 0 | 1 | 3 | 8/12 | 229/340 | 2.4% | 67.4% | 1,094 | 793 | 1.38× |
| expr | 5 | 0 | 1 | 4 | 19/82 | 3,215/3,280 | 0.6% | 98.0% | 435 | 1,401 | 0.31× |
| interp | 5 | 0 | 1 | 3 | 0/23 | 521/543 | 0.0% | 95.9% | 688 | 4,308 | 0.16× |
| list | 17 | 0 | 9 | 8 | 58/599 | 6,001/6,501 | 0.9% | 92.3% | 3,064 | 2,816 | 1.09× |
| misc | 15 | **1** | 9 | 4 | 51/743 | 230/735 | 6.9% | 31.3% | 1,567 | 2,321 | 0.68× |
| object | 4 | 0 | 2 | 2 | 0/88 | 517/538 | 0.0% | 96.1% | 411 | 556 | 0.74× |
| parsing | 5 | 0 | 2 | 3 | 53/213 | 432/833 | 6.4% | 51.9% | 1,141 | 877 | 1.30× |
| proc | 7 | 0 | 4 | 2 | 26/131 | 441/479 | 5.4% | 92.1% | 820 | 936 | 0.88× |
| string | 10 | 0 | 5 | 5 | 53/473 | 1,646/2,868 | 1.8% | 57.4% | 1,113 | **31,005** | 0.04× |
| variable | 9 | 0 | 4 | 5 | 62/290 | 1,243/1,303 | 4.8% | 95.4% | 1,131 | 1,234 | 0.92× |
| **TOTAL** | **97** | **1** | **47** | **49** | **355/2,998** | **32,695/35,921** | **1.0%** | **91.0%** | **13,150** | **54,890** | **0.24×** |

`misc.tcl pass%` is high relative to `tcl pass%` only because most
misc tests are marked `nonRoot` / `notTip457` etc. and tclsh
skips them via constraints; those constraints don't fire in our
runtime so we attempt them and most fail.

`string` total tcl runtime is dominated by `regexpComp.test`
(23 s) and `get.test` (6.6 s) — both heavy regex / parser
exerciser suites where our WASM runtime traps in the first few
hundred ms instead of running the full ~6,000 cases.

## The one fully-passing file

| file | wasm | tcl |
|---|---|---|
| `concat.test` | **9 / 9** passed | 9 / 9 passed |

That's the entire green list. Everything else is `partial` or
`trap`.

## Top wasm pass rates among partial-pass files

These are the files where the WASM runtime makes the most
forward progress before failing or trapping:

| file | wasm passed | of total | failed | skipped | notes |
|---|---:|---:|---:|---:|---|
| `concat` | 9 | 9 | 0 | 0 | full pass |
| `join` | 7 | 10 | 3 | 0 | |
| `eval` | 8 | 12 | 4 | 0 | |
| `linsert` | 18 | 28 | 10 | 0 | |
| `split` | 10 | 18 | 8 | 0 | |
| `for-old` | 5 | 9 | 4 | 0 | |
| `appendComp` | 18 | 48 | 30 | 0 | |
| `append` | 19 | 52 | 33 | 0 | |
| `parseOld` | 53 | 158 | 105 | 0 | |
| `uplevel` | 19 | 57 | 38 | 0 | |
| `proc-old` | 23 | 74 | 51 | 0 | |

The "old" suite variants (parseOld, proc-old, if-old, …) tend to
fare slightly better because they predate features like
`{*}` expansion, ensembles, or `{expand}` syntax that our
parser doesn't yet handle.

## Trap signature buckets

The WASM runtime aborts mid-run on 49 of 97 files. The trap
messages cluster:

| trap pattern | files | example files |
|---|---:|---|
| **(empty stderr / silent exit)** | 10 | `lreplace`, `expr`, `expr-old`, `compExpr-old`, `mathop`, `error`, `lseq`, `lrepeat`, … |
| `frame local table full` | 3 | `set`, `execute`, `incr` |
| `unknown command: <garbage bytes>` | 9 | `listObj`, `listRep`, `lrange`, `format`, `var`, `namespace`, `trace`, `info`, `rename` |
| `preserveCore` (during `cleanupTests`) | 5 | `parse`, `subst`, `for`, `foreach`, `parseExpr` |
| `unsupported command: source` | 3 | `regexp`, `get`, `cmdIL` |
| `regexp: unsupported or unknown option` | 3 | `lseq`, `lrepeat`, `reg` |
| `unsupported command: switch` | 1 | `switch` |
| `wrong # args: should be "test name desc ?options?"` | 1 | `list` |
| `can't rename "list": command doesn't exist` | 1 | `rename` |
| `tcl::build-info` unknown | 1 | `format` |
| `ConstraintInitializer must be complete script` | 2 | `parseExpr`, `dict` |

### What the buckets actually mean

- **Silent traps (10 files)** — the runtime exited with no
  Tcl-level message; usually a wasm `unreachable` or out-of-bounds
  inside a runtime export. Same root cause as the bump-allocator
  OOM in [`05-correctness.md`](05-correctness.md). All ten files
  are computation-heavy (expr, math, regex compile) and the
  bundle compounds the allocation pressure.

- **`frame local table full` (3 files)** — the fixed 256-bucket
  per-frame local table can't hold the local-var count of these
  test files' helpers. Bug **and** perf issue: today every
  frame is 4 KB regardless of need, but the table is
  fixed-capacity so deep procs spill.

- **`unknown command: <garbage>` (9 files)** — the dispatcher
  reads a command name from a `(ptr, len)` that points into
  recycled / overlapping heap memory. Garbled names like
  `2971669`, `tConstraintsHookam`, `\nds`, `gleFile` are
  unmistakable signs of pointer aliasing through the bump
  allocator. Until the allocator gives stable addresses, these
  cannot be debugged at the dispatcher level — the dispatcher
  is doing its job; the input is corrupt.

- **`preserveCore` traps (5 files)** — these all happen during
  `tcltest::cleanupTests`, after every test in the file has
  already been run. The ::tcltest cleanup walks the master
  command table to make sure no test polluted the global
  command set; our runtime trips somewhere in that walk. Likely
  a missing introspection accessor (`info procs` / `info
  commands` filter) rather than a real bug, but it currently
  prevents a clean summary from any of these files even when
  individual tests pass.

- **`unsupported command: source`** — three test files load
  helper data via `source helpers.tcl`. `source` against a
  preopen'd path is still on the runtime's todo list. Easy
  unblock once the WASI fd-resolution piece lands.

## How long do failures take?

Because the runtime tends to trap early, **WASM total runtime is
4× faster than tclsh for the in-scope sweep** (13.2 s vs 54.9 s)
— but that's not a real performance signal, it's a coverage
gap. The two cleanest comparisons:

| file | wasm | tcl | wasm/tcl |
|---|---:|---:|---:|
| `concat` (both pass) | 76 ms | 131 ms | **0.58×** — wasm wins |
| `join` (wasm 7/10) | 92 ms | 103 ms | 0.89× — wasm wins |

So when the runtime *does* run a suite to completion, it's
faster than tclsh; the headline 0.24× ratio is misleading.

## Per-file table

A full per-file breakdown (bundle KB, wasm KB, compile ms,
run ms on each backend, P/F/S of T on each backend, status
classification) is in
[`tcltest_per_file.md`](tcltest_per_file.md), generated by
`aggregate_tcltest.py files`.

## Reproduction

```bash
# Build the harness (one-shot)
$ ls docs/perf/wasm-tcl9-parity/run_tcltest.py
$ ls docs/perf/wasm-tcl9-parity/aggregate_tcltest.py

# Run all 97 in-scope files (≈ 5 minutes wall time)
$ uv run python docs/perf/wasm-tcl9-parity/run_tcltest.py

# Aggregate
$ uv run python docs/perf/wasm-tcl9-parity/aggregate_tcltest.py subsystem
$ uv run python docs/perf/wasm-tcl9-parity/aggregate_tcltest.py traps
$ uv run python docs/perf/wasm-tcl9-parity/aggregate_tcltest.py files \
    > docs/perf/wasm-tcl9-parity/tcltest_per_file.md
```

Raw data: `tcltest_results.json`. Raw run log: `tcltest.log`.
