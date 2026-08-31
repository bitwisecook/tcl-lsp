# KCS: How do I run the C tcltest suite through the bytecode VM?

> **Audience:** Contributor
> **Type:** How-To

## Applies to

tcl-lsp CLI

## Question

How do I run Tcl 9.0 test files through the bytecode VM, compare the result to
reference C `tclsh`, and read the parity scoreboard?

## Before you start

- For a full scoreboard run, the pinned Tcl 9.0.4 source tree must be present
  at `tmp/tcl9.0.4/`. On the web harness the SessionStart hook fetches it;
  locally run `bash
  .claude/skills/fetch-tcl-source/fetch_tcl_source.sh 9.0`.
- A focused run may instead use another Tcl 9.0 patchlevel with `--tcl-root`
  or `TCL_LSP_TCL_ROOT90`. The matching source and interpreter patchlevels
  must be identical.
- A versioned `tclsh9.0` must be on `PATH`, or set `TCL_LSP_TCLSH90` to the
  executable. `make ensure-test-deps` installs the pinned reference build.
- `timeout` (coreutils) must be on `PATH`; the sweep runs each file under it so
  a hang or native-stack overflow can't stall the whole run.

## Answer

### Run one `.test` file through the VM

The `run_test` example sources the real `tcltest.tcl`, then the test file, and
lets tcltest print its `Total N Passed X Skipped Y Failed Z` summary:

```
TCL_LIBRARY=~/src/tcl9.0.3/library \
  cargo run -p tcl-vm --example run_test -- \
  ~/src/tcl9.0.3/tests/<stem>.test --match '<test-id-glob>'
```

`<stem>` is the file stem (e.g. `llength`, `interp`, `coroutine`). Omit
`--match` to run the whole file.
`TCL_TEST_VERBOSE=1` makes tcltest announce each test as it starts (to pinpoint
a hang); `TCL_BACKEND_CONSTRAINTS=<overlay.tcl>` sources a skip overlay before
the file so tests the backend cannot support are skipped. The tree-walk runtime
analogue is `runtime/rust`'s `run_script --init`.

Run the same file through the C oracle to compare:

```
TCL_LIBRARY=~/src/tcl9.0.3/library tclsh9.0 \
  ~/src/tcl9.0.3/tests/<stem>.test -match '<test-id-glob>'
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

- `--stem <name>` — sweep one stem and print its result (repeatable; focused
  runs do not rewrite the committed scoreboard).
- `--match <glob-list>` — restrict each selected stem to Tcltest IDs matching
  the Tcl glob/list. It requires at least one `--stem`.
- `--tcl-root <path>` — use an explicit source tree. Without it, discovery
  checks `TCL_LSP_TCL_ROOT90`, the pinned repository tree, matching sibling
  checkouts, and `$HOME/src/tcl9.0*`.
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

`cargo xtask tcltest-sweep --backend both --tcl-root ~/src/tcl9.0.3 --stem
join` prints:

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
- [`tcl-conformance-harness.md`](../design/runtime/tcl-conformance-harness.md) —
  source/oracle discovery, exact-patch rules, and ownership.
- `rust/tcl-vm/examples/run_test.rs` — the single-file VM driver.
- `rust/xtask/src/tcltest_sweep.rs` — the sweep + scoreboard generator.
