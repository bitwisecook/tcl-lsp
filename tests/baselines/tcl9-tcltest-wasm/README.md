# Tcl 9 core test slice — Zig WASM runtime baseline & hand-off

This directory captures the result of running the upstream **C Tcl 9.0.3**
core test slice through the **Zig WASM runtime** (`runtime/zig/` driven
by `core/compiler/codegen/wasm/`).  Each stem's `.test` file is
bundled with the unmodified upstream `tcltest.tcl`, compiled to WASM,
executed under `wasmtime`, and the resulting tcltest summary is
recorded as a regression floor.  This is the durable hand-off for
whoever picks up the per-bucket fix work the harness identified.

## Scope — read this first

**This baseline is the production WASM ship gate.**

Unlike the Python-VM-side sibling
([`tcl9-tcltest-vm/`](../tcl9-tcltest-vm/README.md)), which is internal
dev signal, every regression here represents a real WASM-runtime
gap that blocks production correctness against upstream Tcl 9.  The
production deliverable is the Zig WASM runtime under `runtime/zig/`
plus the WASM codegen under `core/compiler/codegen/wasm/`; this
harness exercises both end-to-end against the upstream framework.

What this work *is* good for:

- Pinning the actual WASM-runtime correctness against upstream Tcl 9
  on a stem-by-stem basis.
- Surfacing fix candidates ranked by leverage — every `W0-*` row is
  one failed compile / trap / timeout that truncates dozens to
  hundreds of test IDs.
- Catching regressions in `runtime/zig/` and `core/compiler/codegen/
  wasm/` that the per-test pytest gates would not reach (the slice
  is much wider than `tests/test_wasm_*.py` covers).

What this work is *not*:

- A duplicate of the existing per-stem WASM gates in
  [`tcl9-tcltest/`](../tcl9-tcltest/README.md).  That directory holds
  the curated, multi-bucket triage TOMLs consumed by
  `tests/external/run_tcl9_tests.py`.  This `-wasm` slice is the
  64-stem **core semantics** subset wired into a single regression
  floor; the wider `tcl9-tcltest/` slice keeps doing its own job.
- A signal about the Python VM.  Fixes landed in `vm/` will not
  move this baseline.

## Hard rules — read before touching anything

These rules are not negotiable.  They exist so that the test-suite
contract can never silently drift away from upstream.

1. **Never edit `tmp/tcl9.0.3/library/tcltest/tcltest.tcl`.**
   The whole point of this exercise is that our WASM runtime must
   accept the real upstream framework.  Any change to `tcltest.tcl`
   invalidates the experiment.

2. **Never edit any `.test` file in `tmp/tcl9.0.3/tests/`.**
   The `.test` files are the contract.  When a test fails we either
   fix the runtime, or — if the test is incompatible-by-design — we
   classify it as `W9-internal` / `W9-cosmetic` / `W10-environment`
   in the per-stem TOML.  We do not patch the test.

3. **Never edit `tmp/tcl9.0.3/library/init.tcl`** for the same reason.
   If the boot path needs help, the help goes into `runtime/zig/` or
   `core/compiler/codegen/wasm/`, not the library.

4. **No new monkey-patches.**  The bundle's pre-tcltest preamble
   (`tests/external/run_tcl9_tests.py:_PRE_TCLTEST`) and the
   `_patch_tcltest_source` rewrites are existing technical debt
   carried over from when tcltest first booted under WASM.  They are
   **not** a pattern to extend.  If `tcltest.tcl` needs framework
   support that isn't there today, fix the runtime, not the
   framework.

5. **Fix root causes.**  Crashes show up as one of:

   | bucket | fix site |
   |---|---|
   | `W0-codegen-bug` | `core/compiler/codegen/wasm/` (lowering / IR / codegen) |
   | `W0-stub-trap` | `runtime/zig/dispatch/tcl_stub_fallback.zig` + a real handler in `runtime/zig/cmds/` |
   | `W0-runtime-error` | the named handler in `runtime/zig/cmds/*.zig` |
   | `W0-arity` | `runtime/zig/dispatch/tcl_cmd_registry.zig` arity bounds vs. the Python `CommandSpec` |
   | `W8-missing-command` | `core/commands/registry/tcl/` + a Zig handler / stub |
   | `W0-timeout` | the runtime path the bundle is spinning in (bisect by test ID) |

   Mirror every contract you fix with a focused pytest under
   `tests/test_wasm_*.py`.  The pytest pins the contract; this
   baseline catches the slice-wide regression.

6. **Don't lift the baseline floor without a real win.**
   `summary.json` records `passed_min` / `failed_max` for every
   stem.  The regression gate is a pass-only ratchet — fixes can
   only raise it.  Don't edit the JSON to "match what the run
   says now"; the run must demonstrate an honest improvement.
   Refresh through the harness, not by hand:

   ```bash
   make refresh-tcl9-wasm-core-baseline
   ```

7. **Don't widen this baseline to silently absorb a parity-gate
   regression.**  The WASM command-parity gate
   (`tests/baselines/wasm_command_parity.json`, enforced by
   `make check-wasm-parity`) is the source of truth for which
   commands exist, with what arity, and which sub-commands.  When
   a missing command shows up in `W8-missing-command` here, the
   fix is to add the registry / handler pair and refresh both
   baselines together — never to lift this one alone.

## Files in this directory

| Path | Purpose | Lifecycle |
|---|---|---|
| `summary.json` | Per-stem pass/fail floor.  Single source of truth for the regression gate. | Committed.  Refresh with `--refresh-baseline`. |
| `categories/<stem>.toml` | Per-stem classification.  Mirrors the existing WASM-side schema (`good_to_have`, `just_to_match_ctcl`, `skip`, `[baseline]`, `[failing]`).  Supports manual triage of individual test IDs into `W9-internal` / `W9-cosmetic` / `W10-environment` buckets. | Committed.  Auto-generated for new stems; never overwritten if already present. |
| `README.md` | This file. | Committed; static — does not get rewritten by the harness. |

The matching ephemeral artefacts (regenerated each run, **not committed**):

| Path | Purpose |
|---|---|
| `tmp/tcl9-wasm-core-report.json` | Full machine-readable report (per-stem rows, crash details, durations). |
| `tmp/tcl9-wasm-core-categories.md` | Ranked human-readable dossier — current crash list, fix-order, leverage roll-up, top traps by command. |

## Bucket dictionary (WASM-shaped)

The buckets here are deliberately distinct from the Python-VM-side
`B*` codes.  WASM has different failure modes — there is no Python
`ValueError` leaking from a builtin, but there is a wasmtime trap with
`unsupported command: X` on stderr, a codegen-side IR validation
failure, and a wasmtime watchdog interrupt.  Map by symptom, not by
position in the alphabet.

| id | symptom | leverage |
|---|---|---|
| `W0-bootstrap` | Bundle harness / setup error before WASM ever ran (test file missing, bundle pre-flight failure). | very high |
| `W0-codegen-bug` | `core/compiler/codegen/wasm/` raised during lowering / IR build / codegen. | very high |
| `W0-stub-trap` | wasmtime trap with `unsupported command: X` on stderr — `runtime/zig/dispatch/tcl_stub_fallback.zig` was hit. | very high |
| `W0-runtime-error` | wasmtime trap inside an implemented Zig handler — bug in `runtime/zig/cmds/`. | very high |
| `W0-arity` | Arity mismatch: the Python `CommandSpec` registry says N..M, the Zig runtime accepts a different range.  Caught by `make check-wasm-parity` first; surfaces here as a runtime trap inside a registered command. | very high |
| `W0-timeout` | wasmtime watchdog or parent wall-clock killed the run — likely infinite loop / pathological input. | very high |
| `W0-child-died` | Worker process died without sending a row (SIGSEGV in wasmtime, OOM, etc.). | very high |
| `W7-no-tests` | Bundle ran without trapping but tcltest never emitted a `Total / Passed / Skipped / Failed` summary line. | high |
| `W8-missing-command` | Trap with `invalid command name "X"` for X **not currently in the registry** — Python-side spec gap, not just a Zig stub. | very high |
| `W9-internal` | Per-test-ID classification: bytecode disassembly, object-rep probes, `info frame` line tables, `info cmdcount`, `tcl::test` C-extension hooks.  **Incompatible by design.** | none |
| `W9-cosmetic` | Per-test-ID classification: error-message wording / list-quoting / frame counts that round-trip identically but differ byte-for-byte. **Do not fix.** | none |
| `W10-environment` | C-test commands (`testparser`, `testevalex`, …) deliberately unregistered; tests should skip via `tcltest::configure -skip` or upstream constraints. | none |
| `W-mixed-fail` | Stem completed; some tests passed and some failed.  Per-test triage in the matching `categories/<stem>.toml`. | medium |
| `clean` | All tests passed or skipped — no failures. | (no fix needed) |

## Running the harness

```bash
# Regression gate — fails if any stem regresses against summary.json.
# Default ~3.5 min wall-clock with 4 workers + 240 s/stem timeout.
make test-tcl9-wasm-core

# Refresh the baseline after a confirmed runtime/codegen fix.  Never
# do this to "match what the run says now" — only after a focused
# pytest under tests/test_wasm_*.py demonstrates the underlying
# contract is fixed.
make refresh-tcl9-wasm-core-baseline

# Drive the harness directly for a subset of stems while triaging:
uv run --extra dev python scripts/run_tcl9_wasm_core.py --stems list lseq mathop --workers 1 --no-baseline
```

The harness is `scripts/run_tcl9_wasm_core.py` and the regression
pytest is `tests/test_wasm_tcl9_core_baseline.py` (env-gated behind
`RUN_WASM_TCL9_CORE`).  Read the source for the exact CLI flags and
the per-stem timeout / parallelism knobs.

## Fix-and-ratchet workflow

When a `W0-*` crash or `W-mixed-fail` row needs work:

1. **Find the leverage**.  `tmp/tcl9-wasm-core-categories.md` ranks
   crashes by tier — `CompileFail` > `Trap` > `Timeout` >
   `NoSummary` > largest mixed-fail.  Fix highest-leverage first.

2. **Reproduce in isolation**.  Single-stem mode points at exactly
   the bundle:

   ```bash
   uv run --extra dev python scripts/run_tcl9_wasm_core.py \
       --stems <stem> --workers 1 --no-baseline
   ```

3. **Pin the contract in pytest**.  Add or extend a focused test
   under `tests/test_wasm_*.py` that fails against the current
   runtime and passes after the fix.  This is the small, durable
   gate; the slice baseline is the wide net.

4. **Fix in the right module** — see the table in *Hard rules* §5.
   Do not bypass `tcl_stub_fallback.zig` by adding case-by-case
   exceptions in callers; the dispatch layer is the only place a
   command becomes available.

5. **Refresh the baseline**:

   ```bash
   make refresh-tcl9-wasm-core-baseline
   ```

   Inspect the diff in `summary.json` — `passed_min` should rise
   and / or `failed_max` should drop.  No regression elsewhere.

6. **Commit the runtime fix, the new pytest, and the refreshed
   baseline together.**  Reviewers should see the three pieces in
   one diff: contract, gate, ratchet.

## Hand-off invariants

- Tests classified `W9-internal` / `W9-cosmetic` / `W10-environment`
  in a per-stem TOML are **incompatible by design**; never reclassify
  as bugs.  Examples: bytecode disassembly assertions, refcount
  probes, `info frame` line tables tied to the bcc, `tcl::test` /
  `tcl::dict::*` private namespaces, C-test commands like
  `testparser`.
- Existing parity-gate failures (in
  `tests/baselines/wasm_command_parity.json`) are **not** new
  regressions here.  Interrogate the parity baseline first when a
  command-name failure surfaces.
- The bundle's pre-tcltest preamble (`_PRE_TCLTEST`) and
  `_patch_tcltest_source` rewrites are tracked technical debt.  Each
  one is a candidate for elimination as the runtime grows the
  underlying capability — `glob -directory`, true namespace-eval
  call frames, real `array(key)` `upvar 0`, real auto-loading
  through `tclIndex`.  When the runtime catches up, **delete the
  matching patch**, do not leave it in.
