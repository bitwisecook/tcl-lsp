# KCS: How do I run the C tcltest suite through the bytecode VM?

> **Audience:** Contributor, Maintainer
> **Type:** How-To

## Applies to

tcl-lsp CLI (bytecode VM)

## Question

How do I run the bundled Tcl 9.0.4 test files (`tmp/tcl9.0.4/tests/*.test`)
through the bytecode VM, compare the result to reference C `tclsh`, and read
the parity scoreboard?

## Before you start

- The Tcl 9.0.4 source tree must be present at `tmp/tcl9.0.4/`. On the web
  harness this is fetched automatically by the SessionStart hook; locally run
  the `fetch-tcl-source` skill (`bash
  .claude/skills/fetch-tcl-source/fetch_tcl_source.sh 9.0`).
- The reference `tclsh9.0` must be built at `tmp/tcl9-install/bin/tclsh9.0`
  (the same skill builds it) — it is the C oracle the VM is scored against.
- `timeout` (coreutils) must be on `PATH`; the sweep runs each file under it so
  a hang or native-stack overflow can't stall the whole run.

> The retired Python sweep (`scripts/tcltest_sweep/`, `tests/external/
> run_tcl9_tests.py`, `aggregate.py`, `measure_perf.py`) and its
> `uv run pytest` workflow have been removed — this is the Rust-native
> replacement.

## Answer

### Run one `.test` file through the VM

The `run_test` example sources the real `tcltest.tcl`, then the test file, and
lets tcltest print its `Total N Passed X Skipped Y Failed Z` summary:

```
TCL_LIBRARY=tmp/tcl9.0.4/library \
  cargo run -p tcl-vm --example run_test -- tmp/tcl9.0.4/tests/<stem>.test
```

`<stem>` is the file stem (e.g. `llength`, `interp`, `coroutine`).
`TCL_TEST_VERBOSE=1` makes tcltest announce each test as it starts (to pinpoint
a hang); `TCL_BACKEND_CONSTRAINTS=<overlay.tcl>` sources a skip overlay before
the file so tests the backend cannot support are skipped. The tree-walk runtime
analogue is `runtime/rust`'s `run_script --init`.

Run the same file through the C oracle to compare:

```
TCL_LIBRARY=tmp/tcl9.0.4/library tmp/tcl9-install/bin/tclsh9.0 \
  tmp/tcl9.0.4/tests/<stem>.test
```

### Run the whole sweep + regenerate the scoreboard

```
make tcltest-sweep            # == cargo xtask tcltest-sweep --backend both
```

This builds the `run_test` example `--release`, runs every stem in the
capability ladder through both the VM and reference `tclsh` (each under a
per-file timeout), caches the stable C results in
`tests/baselines/tcl9-tcltest/c-tclsh.ndjson`, and regenerates the scoreboard
`docs/design/runtime/rust-vm-tier-parity.md`. Useful flags (`cargo xtask
tcltest-sweep …`):

- `--stem <name>` — sweep one stem and print its result (does not rewrite the
  committed scoreboard); handy for a before/after on a single file.
- `--backend vm` — re-run only the VM, reading the C column from the cached
  baseline (faster; skips `tclsh`).
- `--timeout <secs>` — per-file budget (default 120).
- `--check` — verify the committed scoreboard is in sync instead of rewriting
  it; exits non-zero on drift (the nightly `make tcltest-sweep-check` gate — not
  a per-commit check, the sweep is minutes long).

### Outcome categories

Each stem's VM result is classified against its C reference:

- **`MATCH`** — the VM's `(passed, skipped, failed)` equals C's. The ship target.
- **`gap`** — the file ran to completion but the counts differ (`Failed > 0`, or
  a passed/skipped mismatch). A real per-test divergence — run the file with
  `run_test` and diff against `tclsh` to find the first failing case.
- **`CRASH`** — an uncaught error / no `Total …` summary aborted the file. The
  highest-leverage bucket: one fix unlocks the whole file (a low-tier `CRASH`
  often zeroes thousands of higher-tier tests, e.g. a file gated behind
  `interp create`).
- **`TIMEOUT`** — the file didn't finish inside the per-file budget.

## How to tell it worked

`cargo xtask tcltest-sweep --backend both --stem join` prints:

```
join: C 10/0/0 | VM 10/0/0 | MATCH
```

A full run rewrites `docs/design/runtime/rust-vm-tier-parity.md` with a
per-stem `C P/S/F | VM P/S/F | status` table grouped by tier and a
`Tally: N MATCH · N gap · N crash` header. Commit the scoreboard (and the
`c-tclsh.ndjson` baseline) when the numbers change.

## How to triage a gap

Use the **scoreboard for "where"** and the **[capability
ladder](../design/runtime/tcl-test-tiers.md) for "why"**: fix bottom-up (a Tier
1 parser or Tier 3 trace/encoding bug shows up as scattered failures across many
higher-tier stems), and prefer a `CRASH` on a low tier — it is the single
highest-leverage fix.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [`rust-vm-tier-parity.md`](../design/runtime/rust-vm-tier-parity.md) — the live
  per-stem scoreboard (regenerate with `make tcltest-sweep`).
- [`tcl-test-tiers.md`](../design/runtime/tcl-test-tiers.md) — the capability
  ladder (what each tier means, which files belong to it, why the order matters).
- `rust/tcl-vm/examples/run_test.rs` — the single-file VM driver.
- `rust/xtask/src/tcltest_sweep.rs` — the sweep + scoreboard generator.
