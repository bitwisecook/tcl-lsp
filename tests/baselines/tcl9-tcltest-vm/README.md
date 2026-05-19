# Tcl 9 core test slice — Python VM baseline & hand-off

This directory captures the result of running the upstream **C Tcl 9.0.3**
core test slice through the **Python VM** (`vm.interp`) using the upstream
`init.tcl` and `tcltest.tcl` **unmodified**.  It is the durable hand-off
for whoever picks up the per-bucket fix work the harness identified.

## Scope — read this first

**This baseline gates the Python VM only.  It is _not_ a WASM ship gate.**

The production deliverable is the Zig WASM runtime under `runtime/zig/`.
Fixes landed in `runtime/zig/` will *not* move this baseline — the
harness never invokes the compiled WASM module.  Crashes catalogued
here (e.g. Python `ValueError` leaking from a builtin) are
Python-VM-specific failure modes that do not exist in the Zig runtime
by construction.

What this work *is* good for:

- Triaging shared compiler / parser bugs in `compiler/parsing/` and
  `compiler/`, where fixes propagate to both backends.
- Tightening the Python-side command implementations under
  `compiler/` and `tooling/tooling/vm/commands/` whose specs the WASM parity
  gate (`make check-wasm-parity`) then enforces against
  `runtime/zig/`.
- Keeping the Python VM honest as the cheaper iteration loop while
  the Zig runtime catches up on framework features.

What this work is *not*:

- A signal that the WASM ship target is X% correct against upstream
  Tcl 9.  The WASM-equivalent harness is the priority next step;
  the existing entry point for compiling-and-running `.test` files
  through wasmtime is `tests/external/run_tcl9_tests.py`.
- A gate for any production change.  Treat the pass/fail counts here
  as internal dev signal, not a release metric.

## Hard rules — read before touching anything

These rules are not negotiable.  They exist so that the test-suite
contract can never silently drift away from upstream.

1. **Never edit `tmp/tcl9.0.3/library/tcltest/tcltest.tcl`.**
   The whole point of this exercise is that our Python VM must accept
   the real upstream framework.  Any change to `tcltest.tcl`
   invalidates
   the experiment.

2. **Never edit any `.test` file in `tmp/tcl9.0.3/tests/`.**
   The `.test` files are the contract.  When a test fails we either
   fix the VM, or — if the test is incompatible-by-design — we
   classify it as `B9-internal` / `B9-cosmetic` in the per-stem TOML.
   We do not patch the test.

3. **Never edit `tmp/tcl9.0.3/library/init.tcl`** for the same reason.
   If the boot path needs help, the help goes into `tooling/tooling/vm/`, not the
   library.

4. **No new monkey-patches in `tooling/tooling/vm/commands/tcltest_cmds.py` or
   anywhere else.**  An existing `auto_load` / `parray` shim in
   `_setup_real_tcltest` (`tooling/tooling/vm/commands/tcltest_cmds.py:583`) is a
   debug breadcrumb left from when tcltest first booted; it is
   technical debt that **must be removed** before this slice ships,
   not a pattern to extend.  The fix is to make our VM's auto-load
   path through `init.tcl`'s `tclIndex` work end-to-end so the stub
   becomes unnecessary.

5. **Don't bypass `catch`.**  Every crash in the dossier is a
   case where a Python exception escaped past a Tcl-level `catch`.
   The fix is always to convert the host exception (typically
   Python `ValueError`) to `TclError` at the boundary, never to
   widen the harness's exception filter.

6. **Don't lift the baseline floor without a real win.**
   `summary.json` records `passed_min` / `failed_max` for every
   stem.  The regression gate is a pass-only ratchet — fixes can
   only raise it.  Don't edit the JSON to "match what the run
   says now"; the run must demonstrate an honest improvement.

## Files in this directory

| Path | Purpose | Lifecycle |
|---|---|---|
| `summary.json` | Per-stem pass/fail floor.  Single source of truth for the regression gate. | Committed.  Refresh with `--refresh-baseline`. |
| `categories/<stem>.toml` | Per-stem classification.  Mirrors the WASM-side schema (`good_to_have`, `just_to_match_ctcl`, `skip`, `[baseline]`, `[failing]`).  Supports manual triage of individual test IDs into B9-internal / B9-cosmetic buckets. | Committed.  Auto-generated for new stems; never overwritten if already present. |
| `README.md` | This file. | Committed; static — does not get rewritten by the harness. |

The matching ephemeral artefacts (regenerated each run, **not committed**) are:

| Path | Purpose |
|---|---|
| `tmp/tcl9-vm-core-report.json` | Full machine-readable report (per-stem rows, crash details, durations). |
| `tmp/tcl9-vm-core-categories.md` | Ranked human-readable dossier — current crash list, fix-order, leverage roll-up. |

## Running the harness

```bash
# Regression gate — fails if any stem regresses against summary.json
# (~3.5 min wall-clock, 4 workers, 60 s/stem timeout).  Does NOT
# refresh the committed baseline — it runs the harness with
# --no-baseline and just compares against what's checked in.
make test-tcl9-vm-core
#   ↳ writes tmp/tcl9-vm-core-{report.json,categories.md} for triage
#   ↳ exit code is non-zero on any stem regression

# Refresh the committed baseline (use *only* after a confirmed VM fix
# whose improvements you want to ratchet into the floor).
make refresh-tcl9-vm-core-baseline
#   ↳ overwrites tests/baselines/tcl9-tcltest-vm/summary.json
#   ↳ overwrites/recreates tests/baselines/tcl9-tcltest-vm/categories/*.toml

# Subset only (no gate, no baseline write):
python scripts/dev/run_tcl9_vm_core.py --stems parse basic info string set --no-baseline

# Reproduce one stem in isolation, in a real CLI process:
python -m vm --enable-test-support tmp/tcl9.0.3/tests/<stem>.test

# Run the gate explicitly (same as `make test-tcl9-vm-core`):
RUN_VM_TCL9_CORE=1 uv run pytest tests/test_vm_tcl9_core_baseline.py -q
```

## How the harness avoids state contamination

* **Compile-once amortisation**: parent process loads `init.tcl` +
  `tcltest.tcl` exactly once.  Children fork from it (Linux fork start
  method) and inherit the loaded interp via copy-on-write.
* **Fork-on-demand isolation**: each test file runs in its own forked
  child.  No state leaks between files.
* **Per-stem sandbox**: each child runs in a fresh
  `tempfile.TemporaryDirectory`; `::tcltest::temporaryDirectory` and
  `::tcltest::workingDirectory` point at the sandbox so misbehaving
  tests cannot dump artefacts at the repo root.
  `::tcltest::testsDirectory` still points at the upstream tests
  folder so sibling-source patterns resolve.
* **Per-stem timeout**: 60 s wall-clock, enforced by SIGKILL from the
  parent.  Stems that hit it are recorded as `B0-timeout`.

## Bucket dictionary

Buckets surface the *root cause family* a failure belongs to.  The
stem-level bucket on each row is the dominant bucket for the file;
per-test-ID classification (`good_to_have` vs `just_to_match_ctcl`
vs `skip`) goes in the per-stem TOML.

| id | meaning | leverage |
|---|---|---|
| `B0-bootstrap` | Stem crashes before tcltest emits a summary line.  Generic catch-all when the more specific B0-* labels don't apply. | very high |
| `B0-host-exception` | Python `ValueError` (or similar) escaped from a builtin instead of being converted to `TclError`.  Truncates the rest of the file. | very high |
| `B0-tcl-error` | Genuine `TclError` escaped past a user-level `catch`. | very high |
| `B0-timeout` | Wall-clock exceeded — almost certainly an infinite loop / pathological compile-time blowup. | very high |
| `B0-child-died` | Forked child segfaulted / crashed without sending a result. | very high |
| `B1-parser` | Token / range / line-number drift in `parse.test`, `parseExpr.test`, `parseOld.test`, `subst.test`, `word.test`. | high |
| `B2-expr` | Expr / mathop / number formatting, integer width, NaN/Inf/bignum. | medium |
| `B3-control-flow` | `if/while/for/foreach/switch/error/break/continue/return`, `apply` — return-code propagation, `errorInfo`/`errorCode`. | medium |
| `B4-list-string` | `list*`, `string`, `lset*`, `split`/`join`/`concat`, `format`, `scan`, `append`. | medium |
| `B5-var-scope` | `set*`, `var`, `upvar`, `uplevel`, `incr*`, `proc*`, `namespace*`, `rename`, `cmdInfo`. | medium |
| `B6-introspection` | `info` subcommands, `cmdAH/IL/MZ`, `apply`, `rename`. | medium |
| `B7-framework` | `tcltest` itself / its glue commands (`fconfigure`, `interp create -safe`, `makeFile`, `viewFile`, `outputChannel`). | very high |
| `B7-no-tests` | Stem completes without a tcltest summary (all skipped or never invoked the `test` command). | medium |
| `B8-missing-command` | Whole file dies on `invalid command name "X"`. | very high |
| `B9-cosmetic` | Cosmetic error wording / list-quoting / frame counts that round-trip identically but differ byte-for-byte.  **Do not fix.** | none |
| `B9-internal` | Bytecode / object-internal / `info frame` / `info cmdcount` / `tcl::unsupported::disassemble` / `representation` / refcount / shimmer.  **Incompatible by design.** | none |
| `B10-environment` | C-test commands (`testparser`, `testevalex`, …) deliberately unregistered; tests should skip. | none |

## How to use the per-stem TOML

Each `categories/<stem>.toml` carries:

```toml
# <stem>.test — auto-generated baseline.
# bucket = '<headline bucket>'
# reason = '<short reason>'
trap_allowed = true | false
good_to_have       = ["<test-id>", ...]   # tests we *should* pass
just_to_match_ctcl = ["<test-id>", ...]   # cosmetic / B9-internal
skip               = ["<test-id>", ...]   # never-fixable

[baseline]
good_to_have_failing       = N
just_to_match_ctcl_failing = N
skip_failing               = N
observed_total   = N
observed_passed  = N
observed_failed  = N
observed_skipped = N

[failing]
ids = ["<test-id>", ...]   # currently failing — visibility only
```

When you classify a test ID into `just_to_match_ctcl` or `skip`,
add a `# rationale: …` comment on a nearby line.  The rationale is
how a future reviewer can tell whether to revisit the classification.

## How to land a fix without regressing

1. **Pick a target.** Open the regenerated dossier
   (`make test-tcl9-vm-core`, then read `tmp/tcl9-vm-core-categories.md`).
   Start with Tier 1 (B0-host-exception) — they unblock the most tests
   per fix.
2. **Reproduce in isolation.**
   `python -m vm --enable-test-support tmp/tcl9.0.3/tests/<stem>.test`
   should reproduce the crash with the same `crash_type` /
   `crash_msg`.
3. **Read the upstream contract.** The C source under
   `tmp/tcl9.0.3/generic/` (e.g. `tclParse.c`, `tclBasic.c`,
   `tclCmdAH.c`) is the spec.  Match its error wording and return-code
   path exactly.
4. **Fix in `tooling/tooling/vm/`, `compiler/parsing/`, `compiler/`, or `compiler/`.**
   Never in `tcltest.tcl`, never in `.test`, never in `init.tcl`,
   and never by adding a new monkey-patch.  Note: this gate does *not*
   see `runtime/zig/`; if the same bug exists on the WASM side, it
   needs a separate fix and a separate (future) WASM-side gate.
5. **Mirror the contract in pytest.** Add a focused test under
   `tests/test_vm_<area>_test.py` that pins both the success path and
   (where applicable) the host-exception → `TclError` conversion.
6. **Re-run the gate**: `make test-tcl9-vm-core`.  This runs the
   harness with `--no-baseline` and asserts no regression against the
   currently-committed baseline.  It will *fail* the moment any stem
   passes fewer tests than the floor — which is the point.  Inspect
   `tmp/tcl9-vm-core-categories.md` for the live failure list.
7. **Ratchet the baseline**: once the gate passes (or once a *better*
   floor is achievable), run
   `make refresh-tcl9-vm-core-baseline`.  Inspect the diff on
   `summary.json` — it should be a clean improvement.
8. **Commit** the source fix, the new pytest, the refreshed
   `summary.json`, and any per-stem TOML edits in a single commit.
9. **Run** `make test-tcl9-vm-core` one last time before opening a
   PR.

## Snapshot of current state

The numbers below describe the floor at the time this README was
committed.  The committed `summary.json` is the source of truth; this
section is just orientation.

* **5913** test IDs reach the harness across 68 stems.
* **3359 passing**, **1368 failing**, **1178 skipped**, **17 crashed
  stems**.
* The 17 crashes split as:
  - 7 × `B0-host-exception` (Python `ValueError` from int-parsing in
    a builtin):  `error`, `proc`, `proc-old`, `format`, `scan`,
    `info`, `cmdMZ`.
  - 6 × `B0-tcl-error` (escaped `TclError`):  `namespace-old`,
    `lreplace`, `lrange`, `regexpComp`, `mathop`, `cmdAH`.
  - 3 × `B0-timeout` (suspected hang):  `trace`, `string`, `expr`.
  - 1 × `error` (generic):  `util`.

For the live ranked fix-order, look at the regenerated
`tmp/tcl9-vm-core-categories.md`.  This README never claims to be
current beyond what the committed `summary.json` says.
