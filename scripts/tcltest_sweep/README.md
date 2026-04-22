# Tcl 9 tcltest sweep harness

A per-file timeout-bounded driver for running every bundle in
`_IN_SCOPE` of `tests/external/run_tcl9_tests.py` and producing a
triage roll-up at `tests/external/baseline-tcl9.json`.

## Why a shell harness rather than a pytest flag?

`pytest-timeout` isn't in our dev dependencies. More importantly, a
few bundles hang in tight eval-fallback loops (the compiled
`foreach` over thousands of elements is the worst offender), and a
single hang would otherwise kill the whole session — the per-file
`timeout` wrapper skips the hung file and lets the rest of the sweep
finish.

## Usage

```
bash scripts/tcltest_sweep/run_all.sh
uv run python scripts/tcltest_sweep/aggregate.py /tmp/tcltest-sweep.ndjson
```

The aggregator writes `tests/external/baseline-tcl9.json` and
prints the roll-up to stdout.

Each record captures:

- `name`, `classname` (hyphens → underscores to match the
  dynamically-built pytest class).
- `outcome`: `pass`, `fail`, `trap`, `compile_fail`, `no_summary`,
  `timeout`, or `unknown`.
- `rc`: the pytest exit code for that invocation.
- `trap`: the first `tcl trap:` line found in pytest's output, if
  any.
- `first_failing`: the first `==== <name> FAILED` marker when
  tcltest reports failing tests.

## Adding a new bundle

1. Add the stem to `_IN_SCOPE` in
   `tests/external/run_tcl9_tests.py`.
2. Append the stem to `names.txt` here.
3. Re-run the sweep and aggregator.
4. Commit the updated `baseline-tcl9.json` alongside the new
   bundle.

See `docs/kcs/kcs-how-to-run-tcltest-bundles.md` for the full
workflow.
