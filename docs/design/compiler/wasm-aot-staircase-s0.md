# Stage S0 — Foundation (observability + contract + repro)

> Index: [`wasm-aot-staircase.md`](wasm-aot-staircase.md). Other stages live
> alongside as `wasm-aot-staircase-sN.md`.

Without S0 every later stage is blind. Once it lands the team can:

- know when refcount is wrong (leak counter)
- reproduce the canonical "`set var $other` → drained between iterations"
  bug deterministically
- cite the exact ownership contract for any runtime call

## Pre-conditions

None. S0 builds the testbed; no other stage depends on prior work.

## Sub-plans

### S0.1 — Document the runtime refcount contract

**Goal**: produce a contract table in `docs/design/runtime/refcount-contract.md`
that names, for every WASM-exported runtime function, the +1 ownership rules:
who owns the input args, who owns the result, what is stored long-term in
the callee.

**Why it matters**: The failed S2 attempt over-released because some runtime
fast paths assume `rc == 1` to mutate in place. Without a written contract
we can't tell a fast-path safety violation from a real refcount bug.

**Tasks**:

- [ ] Walk every `pub export fn` in `runtime/zig/` and classify each by:
  - **Args**: `borrowed` (caller still owns), `consumed` (callee took the
    +1), `passthrough_returned` (returned verbatim).
  - **Return**: `owned` (caller gets +1), `borrowed` (caller does not own),
    `null_or_owned` (0 means no value, otherwise +1).
  - **Internal storage**: which slots / tables retain the arg long-term.
- [ ] Produce a markdown table grouped by subsystem (frames, namespace,
  list, dict, string, expr, regex, IO, …).
- [ ] Identify every fast-path `if (rc == 1) { /* mutate */ }` site and
  flag it: needs a "sole owner" predicate that does not depend on raw rc
  once compile-side discipline lands.
- [ ] Cross-reference each `tcl_obj_retain` and `tcl_obj_release` site in
  `runtime/zig/` to the contract entry that justifies it.
- [ ] Add a CI lint script `scripts/check_refcount_contract.py` that
  warns when a new `pub export fn` is added without a contract row.

**Files**:

- New: `docs/design/runtime/refcount-contract.md`
- New: `scripts/check_refcount_contract.py`
- Reference (read-only): `runtime/zig/**/*.zig`

**Test plan**: This is documentation; the test is the lint script
asserting every export has a contract row. CI runs it on every PR
touching `runtime/zig/`.

**Rollback**: Documentation is risk-free; no runtime change. Lint script
is warning-only initially, escalates to error after every existing
export has a row.

**Acceptance gate**: Every existing `pub export fn` in `runtime/zig/`
has exactly one contract row. CI lint passes.

**Estimated size**: 1 commit (large, mechanical). Maybe 2 if the lint
script is split out.

---

### S0.2 — Debug-mode leak counter (MM-C)

**Goal**: A `-Dleak-check=true` zig build flag that wraps every
`obj_alloc` / `release_now` with a global counter. Reactor exit asserts
the counter is zero. Non-zero count prints the type-tag distribution
of the leaked objs.

**Why it matters**: Per `docs/design/runtime/memory-management.md` MM-C
(documented but not implemented). Without this the only signal we have
for refcount bugs is "test failed mysteriously hours later".

**Tasks**:

- [ ] Add `runtime/zig/build.zig` flag: `b.option(bool, "leak-check", ...)`
  threaded into a `build_options` import.
- [ ] Inside `runtime/zig/valtypes/tcl_obj.zig`:
  - When flag is on: `obj_alloc` increments `g_alloc_count[type_tag]`;
    `release_now` decrements.
  - When flag is on: add `tcl_test_finalize` export that asserts all
    counts are zero, prints distribution to stderr, and traps on
    non-zero.
- [ ] Add a "double-free" counter that fires when `release_now` is
  called with `OBJ_REFCOUNT == 0` (already-queued obj). This catches
  the case the failed S2 hit.
- [ ] Test harness change: `_run_wasm` calls `tcl_test_finalize` on
  clean exit when the leak-check binary is loaded.
- [ ] New build profile: `make build-leakcheck` that produces
  `runtime/zig/zig-out/bin/tcl_runtime_leakcheck.wasm` alongside the
  normal build (tests opt in via env var).

**Files**:

- Modify: `runtime/zig/build.zig`, `runtime/zig/valtypes/tcl_obj.zig`
- Modify: `tests/test_wasm_real_tcl.py` (`_get_rt_module` switches
  based on env var)
- Modify: `Makefile` (new `build-leakcheck` target)
- Optional: `.claude/hooks/session-start.sh` (build the leakcheck
  variant on startup so CI/web sessions have it ready)

**Test plan**:

- Run the existing in-scope tcltest sweep against the leakcheck
  binary. Today most tests leak (runtime has not completed MM-B
  yet); the gate at this stage is "every test runs to completion
  and reports a leak count" — no spurious traps from the assert.
- Add a fixture under `tests/test_runtime_leakcheck.py` that runs a
  known-clean snippet (`set x 1; puts $x`) and asserts zero leak.

**Rollback**: Flag defaults to off. Zero impact on production builds.
Revert the build flag and tests pass as before.

**Acceptance gate**:

- `make build-leakcheck` produces a wasm binary.
- The known-clean snippet test reports zero leaked objs.
- The full sweep runs to completion (leaks are reported but do not
  crash).
- Double-free counter reads zero on a clean run.

**Estimated size**: 2 commits (build flag + counter wiring; test
harness integration).

---

### S0.3 — Canonical bug deterministic repro

**Goal**: Land a test that reproduces the "`set var $other` → drained
between iterations" bug deterministically when refcount discipline is
missing, and passes when discipline is correct. This is the test S2
measures against.

**Why it matters**: The failed S2 attempt could not validate "did my
fix actually fix the bug?" because there was no repro. The trap-cluster
prompt said "re-create `test_iso2.tcl` if missing" — but recreations
did not trip the bug. Without a repro every fix is a guess.

**Tasks**:

- [ ] Bisect the prior session's git history (PR #225 branch and earlier
  WIPs) for any test that exercised the canonical pattern.
- [ ] If none found, build one from the runtime side: a minimal Tcl
  fragment that
  - stores a `[string range $line 0 N]` result in a WASM local,
  - aliases it via `set var $other`,
  - forces a tcl_eval boundary that drains the queue (e.g. `eval {}`),
  - reads `$var` after the drain and asserts the bytes are intact.
- [ ] Verify the repro fails on a runtime where `tcl_obj_release` is
  forcibly invoked at the end of `eval_command` (simulating the
  canonical bug). This may need a temporary `-Drelease-aggressively=true`
  build option.
- [ ] Place the repro at `tests/test_wasm_refcount_canonical.py` with
  parametrised variants: through a proc, at top level, inside a loop,
  with frame elided vs not, with `string range` vs `lindex` vs other
  tail-allocating sources.
- [ ] Document in the test file the exact sequence of operations that
  triggers it, so a future engineer reading the test understands the
  mechanism.

**Files**:

- New: `tests/test_wasm_refcount_canonical.py`
- Optional: small Zig test-instrument flag (`release-aggressively`)
  that forces "release after every command dispatch" — only needed
  if natural execution does not trip it.

**Test plan**:

- Today (no compile-side discipline): the test must fail (or `xfail`
  if natural execution does not yet trigger the bug).
- After S2 (compile-side discipline): the test passes.

**Rollback**: Test marked `xfail` initially. If we cannot get a repro,
document why and continue with synthetic tests built from the leak
counter.

**Acceptance gate**: At least one fixture in
`test_wasm_refcount_canonical.py` is `xfail` today and will be unmarked
by S2.

**Estimated size**: 1 commit if the repro is naturally reachable; 2–3
if we need a runtime test-instrument flag.

---

### S0.4 — `make leakcheck` CI gate

**Goal**: A Make target that runs the in-scope tcltest sweep against
the leakcheck binary and ratchets the per-file leak counts into a
baseline file. CI fails on any unfavourable delta.

**Why it matters**: Without ratcheting, fixes hide regressions and
the leak counter becomes ignored. With ratcheting, every PR sees a
delta against the baseline and a new leak shows up in the PR's CI
surface.

**Tasks**:

- [ ] New script `scripts/leak_sweep.py` that runs every in-scope test
  through the leakcheck binary and dumps `(stem, alloc_count,
  free_count, type_tag_residuals)` to JSON.
- [ ] Baseline file `tests/baselines/wasm_leak_baseline.json`.
- [ ] Diff script `scripts/diff_leak_sweep.py` (mirrors
  `scripts/diff_tcl9_tcltest.py`) that compares a fresh sweep
  against baseline and fails CI on regression.
- [ ] Snapshot target `make snapshot-leak-baseline` for intentional
  changes.
- [ ] Document in `AGENTS.md` and the runtime memory-management doc.

**Files**:

- New: `scripts/leak_sweep.py`
- New: `scripts/diff_leak_sweep.py`
- New: `tests/baselines/wasm_leak_baseline.json` (initially mostly
  non-zero; ratcheted down as MM-B completes)
- Modify: `Makefile` (add `leakcheck` and `snapshot-leak-baseline`
  targets)
- Modify: `docs/design/runtime/memory-management.md` (link the gate)
- Modify: `AGENTS.md` (mention the new check)

**Test plan**: First run produces the initial baseline. Subsequent
runs must match. Adding a deliberate leak (test) must fail the gate.

**Rollback**: The gate is opt-in via `make leakcheck`; nothing else
depends on it. Revert by removing the target.

**Acceptance gate**:

- `make leakcheck` runs in < 5 minutes locally.
- Baseline is committed.
- A test PR with an artificially injected leak fails the gate.

**Estimated size**: 2 commits (sweep + diff scripts; baseline +
Makefile plumbing).

---

## Stage exit criteria

S0 is "done" when **all** of the following hold:

- The runtime refcount contract is published and CI lints new exports
  against it.
- A leakcheck wasm binary exists and the test harness can run any
  fixture against it.
- A canonical-bug repro is committed (xfail or pass depending on
  whether the underlying bug is currently triggerable).
- `make leakcheck` baseline is captured and a regression PR fails it.

Only then does S1 become tractable. S0's deliverables are the lens
through which every later stage's correctness is measured.
