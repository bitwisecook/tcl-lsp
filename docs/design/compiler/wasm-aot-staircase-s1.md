# Stage S1 — "Frames everywhere" correctness baseline

> Index: [`wasm-aot-staircase.md`](wasm-aot-staircase.md). Foundation in
> [`wasm-aot-staircase-s0.md`](wasm-aot-staircase-s0.md).

The premise: a Tcl proc with a runtime frame is **trivially correct** for
every refcount question because the runtime's `tcl_local_set` already
retains the new value and releases the prior (`MM-B.3`, commit `fe68d410`).
Frame elision is an optimisation; the un-elided path is the safety floor.

Today's codegen elides the frame whenever the escape analysis (`var_escape`)
says the proc has no FRAME-tagged var, no fallback, and no `info level`
reference (`core/compiler/codegen/wasm/_emitter/_core.py:503-532`).
Elision is the source of every refcount bug we have seen on the compile
side, because the WASM-local mirror is then the slot's authoritative
storage and nothing on the compile side currently retains / releases it.

S1 makes the un-elided baseline an **explicit and testable floor**.

## Pre-conditions

- S0.2 (leak counter) and S0.4 (`make leakcheck`) — without them we cannot
  prove "frames everywhere" actually leaks zero TclObjs at the seams.

## Sub-plans

### S1.1 — Add `--no-frame-elision` codegen flag

**Goal**: a single source-of-truth boolean that can force every proc to
push a runtime frame, regardless of what the escape analysis says.

**Why it matters**: We need a way to A/B the elided vs un-elided codegen
on the same source. Today's codegen has no kill-switch; toggling elision
requires editing the conditional inline.

**Tasks**:

- [ ] Add a parameter `frame_elision: bool = True` to the public
  `wasm_codegen_module(ir_module, ..., frame_elision=...)` entry in
  `core/compiler/codegen/wasm/__init__.py`.
- [ ] Thread the flag into every emitter constructor (`_WasmEmitterBase`
  + the various mixins).
- [ ] Inside `_core.py::generate`, gate the existing elision condition on
  the flag:
  ```python
  if frame_elision and wants_frame and summary is not None and ...:
      wants_frame = False
  ```
- [ ] Surface the flag at the test-harness layer (`_compile_tcl(..., 
  frame_elision=False)`) and at the sweep harness 
  (`scripts/dev/run_tcl9_tcltest_sweep.py --no-frame-elision`).
- [ ] CLI: add `tcl-lsp wasm-codegen --no-frame-elision` (if a CLI exists 
  for the compiler) or a Make target `make sweep-no-elision`.

**Files**:

- Modify: `core/compiler/codegen/wasm/__init__.py`
- Modify: `core/compiler/codegen/wasm/_emitter/_core.py`
- Modify: `tests/test_wasm_real_tcl.py` (`_compile_tcl` signature)
- Modify: `scripts/dev/run_tcl9_tcltest_sweep.py` (CLI flag)
- Modify: `Makefile` (new `sweep-no-elision` target)
- Reference: existing `var_escape/_types.py::ProcEscapeSummary`

**Test plan**: 

- Unit test: compile a proc that today gets `wants_frame=False` 
  (e.g. `proc f {x} { return $x }`) with `frame_elision=False` and 
  assert the WASM module imports `tcl_frame_push` / `tcl_frame_pop`.
- Sweep: `make sweep-no-elision` runs the full in-scope tcltest 
  suite. Result must be at least as good as `make sweep` 
  (no regressions; possibly fewer regressions because the un-elided 
  path is correct today and the elided path may have lurking bugs).

**Rollback**: Flag defaults to `True` (elision on). Removing the flag 
returns to today's behaviour. Zero risk.

**Acceptance gate**: `make sweep-no-elision` produces a JSON 
comparable to today's baseline. The diff is non-negative.

**Estimated size**: 1 commit.

---

### S1.2 — Verify "frames everywhere" matches or beats the baseline

**Goal**: prove that running with `--no-frame-elision` is correctness-
equivalent to today's mixed mode (some procs framed, some elided), and 
that the leak-check baseline shows the framed path is leak-free.

**Why it matters**: If the un-elided path leaks or traps in cases the 
elided path does not, our "safety floor" is a myth and S2 has nowhere 
solid to stand on.

**Tasks**:

- [ ] Run `make sweep` (mixed mode, today's behaviour) → snapshot the 
  per-file pass counts.
- [ ] Run `make sweep-no-elision` → snapshot the per-file pass counts.
- [ ] Diff the two. Expect:
  - Equal or higher pass count under `--no-frame-elision`.
  - Any file that *regresses* under `--no-frame-elision` is a runtime 
    bug exposed by always taking the framed path; file an issue per 
    regression and fix in S1.3.
- [ ] Same comparison under `make leakcheck` and 
  `make leakcheck NO_FRAME_ELISION=1`.
- [ ] Document the deltas in 
  `docs/design/compiler/wasm-aot-staircase-s1.md` (this file) under 
  a new "S1 measurement" section.

**Files**:

- Modify: `tests/baselines/tcl9_tcltest_baseline.json` (probably 
  unchanged; if `--no-frame-elision` finds latent bugs, the baseline 
  may shift)
- Modify: `tests/baselines/wasm_leak_baseline.json` (the framed path 
  should leak less or the same as the mixed path)
- Modify: this doc with measurement results

**Test plan**: Empirical. The sweep + leakcheck IS the test.

**Rollback**: No code change required at this step; this is 
measurement.

**Acceptance gate**: 

- `--no-frame-elision` pass count ≥ today's pass count.
- `--no-frame-elision` leak count ≤ today's leak count.
- Any deltas in either direction are explained in the measurement 
  section.

**Estimated size**: 0 commits (measurement) + however many fix 
commits S1.3 needs.

---

### S1.3 — Audit + fix any runtime gaps S1.2 surfaces

**Goal**: When S1.2 finds a file that regresses under 
`--no-frame-elision`, the cause is a runtime-side bug in the framed 
path (because the elided path must be a strict subset of the framed 
path). Fix those bugs before S2 starts.

**Why it matters**: S2 builds on the framed path's correctness 
guarantee. If the framed path has bugs, S2's "elide when proven safe" 
becomes "elide when guaranteed broken".

**Tasks**:

- [ ] For each regression file from S1.2:
  - Capture the specific failing test (`tcltest` produces named 
    test output).
  - Reduce to a minimal repro.
  - Trace through the runtime: does it involve `tcl_local_set`, 
    `frame_pop`, `var_resolve`, `tcl_eval` boundaries?
  - Fix in the runtime, not the compile side. The compile side at 
    this stage emits straightforward `tcl_frame_push` / 
    `tcl_local_set` / `tcl_frame_pop`; runtime must handle them.
- [ ] For each fix, add a unit test under 
  `tests/test_runtime_frame_discipline.py` that exercises the 
  exact sequence in isolation.
- [ ] Re-run S1.2's measurement after each fix. Stop when 
  `--no-frame-elision` is clearly ≥ today's mixed mode.

**Files** (per fix; concrete files depend on what surfaces):

- Likely: `runtime/zig/interp/tcl_frames.zig`, 
  `runtime/zig/interp/tcl_ns.zig`, `runtime/zig/interp/tcl_interp.zig`
- New: `tests/test_runtime_frame_discipline.py`

**Test plan**: Each fix has a targeted unit test; the sweep is the 
integration check.

**Rollback**: Each fix is independently revertable. If a fix causes 
unexpected regression elsewhere, revert and re-investigate.

**Acceptance gate**:

- `--no-frame-elision` produces a sweep with **zero regressions** 
  vs today.
- `tests/test_runtime_frame_discipline.py` covers every 
  surfaced bug.

**Estimated size**: Variable. Likely 0–3 commits depending on 
runtime bugs S1.2 turns up.

---

## Stage exit criteria

S1 is "done" when **all** of:

- `make sweep-no-elision` matches or beats today's baseline (pass 
  count and leak count).
- Every gap S1.2 found is fixed in the runtime, with a unit test.
- The "frames everywhere" mode is documented as the safety floor 
  the rest of the staircase rolls back to.

After S1, S2's promise becomes concrete: "if we can prove a proc 
does not need a frame AND maintain the slot ownership invariant, 
elide. Otherwise, fall back to S1's framed path."

## Notes on the design choice

S1 deliberately does **not** introduce any new optimisation. The flag 
exists purely as a kill-switch and a measurement axis. This keeps the 
"safety floor" at exactly the work the runtime team has already done 
under MM-B.

If S2 ever runs into a bug it cannot resolve, the fall-back is "set 
`frame_elision=False` for that proc" — equivalent to the framed 
path. This makes per-proc rollback trivial and gives the AOT 
compiler a graceful degradation path: when a proof fails, the proc 
stays framed and runs slower but correctly.
