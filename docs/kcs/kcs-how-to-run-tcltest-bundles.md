# KCS: How do I run a Tcl 9 tcltest bundle through the WASM runtime?

> **Audience:** Contributor, Maintainer
> **Type:** How-To

## Applies to

tcl-lsp CLI

## Question

How do I run the bundled Tcl 9.0.3 test files (`tmp/tcl9.0.3/tests/*.test`)
through our compile-to-WASM path and interpret the outcome?

## Before you start

- The Tcl 9.0.3 source tree must be present at `tmp/tcl9.0.3/`. On the
  web harness this is fetched automatically by the SessionStart hook;
  locally run `bash .claude/skills/fetch-tcl-source/fetch_tcl_source.sh 9.0`.
- The Zig runtime must be built (`cd runtime/zig && zig build
  -Doptimize=ReleaseFast`). Many of the fixes that let tcltest's
  preamble complete live in `tcl_cmd_info.zig`, `tcl_frames.zig`,
  `tcl_interp.zig`, and the per-command modules under `runtime/zig/cmds/`
  — a stale runtime will trap earlier than the bundle expects.
- `uv sync --extra dev` has been run so the `pytest` / `wasmtime`
  Python dependencies are in the venv.

## Answer

### Run one bundle

```
uv run pytest 'tests/external/run_tcl9_tests.py::TestTcl9_<name>' -v
```

`<name>` is the test-file stem listed in `_IN_SCOPE` inside
`tests/external/run_tcl9_tests.py` (e.g. `llength`, `interp`,
`concat`). Each class has two tests: `test_compiles` (bundle compiles
to WASM) and `test_runs` (bundle executes and reports `Failed == 0`).

### Run the whole sweep

The full sweep through pytest's normal runner can hang on a single
bundle for minutes. Run each bundle with a per-file 30 s timeout
instead:

```
bash scripts/tcltest_sweep/run_all.sh
uv run python scripts/tcltest_sweep/aggregate.py /tmp/tcltest-sweep.ndjson
```

The runner writes one JSON record per bundle to
`/tmp/tcltest-sweep.ndjson`; the aggregator rolls the records up
into `tests/external/baseline-tcl9.json`. See
[`scripts/tcltest_sweep/README.md`](../../scripts/tcltest_sweep/README.md)
for the outcome classification and per-record schema.

### Bundle shape

`tests/external/run_tcl9_tests.py::_bundle` concatenates three
sources in order:

1. `tmp/tcl9.0.3/library/tcltest/tcltest.tcl` — the real Tcl 9
   tcltest preamble (procs, namespace exports, configure defaults).
2. A short preamble (`_PREAMBLE`) that imports every tcltest command
   into the global namespace and silences per-test verbose output.
3. The test file itself (e.g. `tmp/tcl9.0.3/tests/llength.test`).

The whole concatenation is handed to `lower_to_ir` → `build_cfg` →
`wasm_codegen_module` as a single translation unit, compiled with
the same pipeline as any user program.

### Outcome categories

The triage-report JSON (see `baseline-tcl9.json`) buckets every
bundle into one of:

- **`pass`** — the bundle ran to completion and tcltest reported
  `Failed == 0`. This is the ship target.
- **`fail`** — the bundle ran to completion but tcltest reported
  `Failed > 0`. A real test-level failure — open the `stdout_tail`
  field to see the first failing test name.
- **`trap`** — the WASM module raised a trap (unknown command,
  unreachable, unsupported primitive). The `trap_site` field
  resolves to a `(file, line, col, command, args)` tuple via the
  compiled-in diag sidecar.
- **`no_summary`** — the bundle ran without trapping but never
  printed the `Total N Passed X Skipped Y Failed Z` summary line.
  Usually means tcltest's `cleanupTests` was skipped or the bundle
  exited early.
- **`timeout`** — the bundle didn't finish inside the 30 s budget.
  Mostly hit by tests that drive large `foreach` loops through
  eval-fallback paths.

### Adding a new file to `_IN_SCOPE`

1. Confirm the file is purely Tcl semantics (no threads, sockets,
   real filesystem writes outside `tmp/`, or C extensions gated by
   `testConstraint`). Files that need deferred primitives should
   stay out of `_IN_SCOPE`.
2. Append a `(stem, subsystem)` tuple to `_IN_SCOPE` in
   `tests/external/run_tcl9_tests.py`. The subsystem is the
   inventory label (`list`, `proc`, `interp`, …).
3. Re-run `make prep-pr` to register the two dynamically-generated
   test methods with pytest.
4. Run the bundle once to see whether it passes, fails with test
   errors, or traps. If it traps, note the trap category in the
   commit message so the maintainer can decide whether to fix the
   gap or skip the file.

## How to tell it worked

The bundle reports a clean `Total N Passed N Skipped 0 Failed 0`
summary and the pytest test class prints two `PASSED` lines:

```
TestTcl9_concat::test_compiles PASSED
TestTcl9_concat::test_runs PASSED
```

For the sweep, `/tmp/sweep/aggregate.py` prints a roll-up like:

```
{ "total": 97, "pass_count": 20, ... }
```

and writes `tests/external/baseline-tcl9.json` with per-file
detail. Check that file into source control when the pass count
changes.

## Known gaps tripping bundles in flight

These are the recurring trap categories the triage sweep surfaces
today. None are preamble-level (tcltest.tcl itself initialises
cleanly); they all fire once individual test cases start running.

- **Top-level vars not visible to nested eval-fallbacks** — fixed
  for `foreach` iter vars in the `wasm/_emitter/` package; analogous patterns with
  `lassign`, `try` bindings, or deeply-nested `eval [list set]`
  still slip through.
- **`interp create` collision after test-local cleanup fails** —
  a test creates `interp a`; the file-level `foreach i [interp
  children] { interp delete $i }` cleanup ran before `a` existed, so
  the next test's `interp create a` fails. Needs tcltest's
  `-setup`/`-cleanup` harness to run reliably through our `test`
  proc, which requires `return -code` and `error` plumbing we
  haven't finished.
- **Large bundles compile but time out at exec** — often the
  bundle spins inside a `foreach` over thousands of elements with
  each iteration falling back to `tcl_eval`. Either the iter-var
  sync needs to be cheaper or a Tcl-level optimisation is
  missing.

### Measuring bundle performance

```
uv run python scripts/tcltest_sweep/measure_perf.py
```

Runs a compile / exec timing pass over three representative
bundles (`llength`, `concat`, `interp`) and writes the result to
`tests/external/perf-baseline-tcl9.json`. Each bundle is compiled
and executed three times; the min and median of both numbers are
reported.

Targets (from the tcltest wave plan):

- Small bundle (`llength`, ~30 tests, ~107 KB after preamble):
  compile + exec under 10 s end-to-end.
- Large bundle (`interp`, ~200 tests, ~212 KB after preamble):
  compile + exec under 60 s end-to-end.

Current headroom on the Claude-Code-on-the-web container is an
order of magnitude below both targets, so the sweep isn't
bottlenecked on compiler speed; triage work should focus on
correctness gaps before chasing perf.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- `tests/external/run_tcl9_tests.py` — the bundle driver.
- `tests/external/baseline-tcl9.json` — the last recorded sweep
  roll-up. Regenerate with
  `scripts/tcltest_sweep/aggregate.py`.
- `tests/external/perf-baseline-tcl9.json` — compile + exec
  timings for three representative bundles.
- `docs/design/runtime/child-interp.md` — the conservative-flush
  rationale that underpins every fix in the
  `claude/fix-tcltest-tcl9-*` wave.
