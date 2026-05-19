# Stage S5 — SSA-driven codegen optimisations

> Index: [`wasm-aot-staircase.md`](wasm-aot-staircase.md). Prior stages
> [S2](wasm-aot-staircase-s2.md), [S3](wasm-aot-staircase-s3.md),
> [S4](wasm-aot-staircase-s4.md).

S2's correctness work introduced overhead — every owned-slot write
emits at minimum a release of the prior. S3 + S4 grew the surface
where that overhead applies. S5 trims it back: when SSA already
proves the wrap is unnecessary, skip it.

The compile pipeline already runs SSA + SCCP + the rest of the
optimiser stack (`compiler/optimiser/`). S5 is mostly about
plugging the existing SSA facts into the WASM codegen so it can
make smarter emission choices.

The deliverable is a strict reduction in `tcl_obj_retain` /
`tcl_obj_release` call counts, measured by the leakcheck binary's
counter export. Sweep + leakcheck must remain neutral.

## Pre-conditions

- **S2 complete** (the wrap exists; otherwise there is nothing to
  optimise).
- **S0.2 leak counter** (the metric we read to confirm fewer calls
  produce no behaviour change).

## Sub-plans

### S5.1 — Skip wrap when SSA proves prior is null

**Goal**: At the first write to a body-local within a basic block,
the slot is provably 0 (WASM locals start at 0, no earlier write
has happened on this control-flow edge). The release of the prior
is a guaranteed no-op (`tcl_obj_release(0)` is null-safe). Skip
emitting that release.

**Why it matters**: Many proc bodies start with `set x …` for
several scratch locals. Each such write today emits a
`tcl_obj_release(0)` call that is a guaranteed runtime no-op.

**Tasks**:

- [ ] In `compiler/codegen/wasm/_emitter/_core.py::_emit_owned_local_write`:
  query the SSA fact "is this slot's value at this program point
  provably 0?".
- [ ] Plumb the SSA fact in: the existing
  `compiler/ssa.py` already produces `(var, version)` keys.
  Add a "version 0 is the initial undef-or-0 value" convention
  if it is not already there.
- [ ] When the answer is yes, emit only the retain (or for OWNED
  source, just the raw store) and skip the local.get + release.
- [ ] Add a counter increment to a debug-only "elided releases"
  meter so the leak-check build can report how many wraps were
  skipped.

**Files**:

- Modify: `compiler/codegen/wasm/_emitter/_core.py`
- Modify: `compiler/ssa.py` if the convention needs
  formalising.
- Modify: `runtime/zig/valtypes/tcl_obj.zig` if the leak-check
  build needs a new counter.

**Test plan**:

- Unit test: a proc with `set x 1; set y 2; return $x` emits
  zero `tcl_obj_release` calls (both writes are first-writes on
  null slots).
- Sweep: net-positive on wall time (one fewer call per first
  write); semantics neutral.
- Leakcheck: neutral on residual leaks; the elided-release
  counter is non-zero.

**Rollback**: Single revert. Re-emits the unconditional release
on every write.

**Acceptance gate**: Tests pass; sweep neutral on correctness,
positive on wall time; the elided-release counter shows a
meaningful percentage of writes hit the fast path.

**Estimated size**: 1–2 commits.

---

### S5.2 — Skip wrap when new and prior alias

**Goal**: When SSA proves that the new value about to be stored
is the same TclObj handle as the slot's current value
(`set x $x` after a chain of var-to-var copies, or
`incr x 0` no-op), the retain + release pair cancels out. Skip
both.

**Why it matters**: The wrap on no-op writes is the most
embarrassing overhead — three runtime calls accomplishing
nothing.

**Tasks**:

- [ ] Define the alias predicate: at the program point of
  `_emit_owned_local_write(idx, source)`, can we prove the
  value on the stack came from `local.get idx` on the same
  control-flow edge with no intervening write to `idx`?
- [ ] Implement via an SSA-version comparison: the value's SSA
  version equals the slot's current SSA version.
- [ ] When yes AND `source is BORROWED`, skip the wrap entirely
  — the retain + release cancel and the store is a no-op (the
  slot already holds the value).
- [ ] When yes AND `source is OWNED`, still skip — but this
  case is rare (a fresh literal is never the same handle as
  the slot's current value).
- [ ] Add a counter for "wrap fully elided" hits.

**Files**:

- Modify: `compiler/codegen/wasm/_emitter/_core.py`
- Modify: `compiler/ssa.py` if version comparison helpers
  need a new method.

**Test plan**:

- Unit test: `set x $x` emits no instructions (or just a no-op
  marker for diag).
- Sweep: neutral.
- Leakcheck: neutral.

**Rollback**: Single revert.

**Acceptance gate**: Tests pass; counter shows non-zero hits on
typical bundles.

**Estimated size**: 1 commit.

---

### S5.3 — Hoist invariant retain/release out of loops

**Goal**: In a loop body where a slot's "store value source" is
loop-invariant (e.g. `for {set i 0} {…} {…} { set guard $g }`
where `g` is loop-invariant), the retain + release on the
slot can move out of the loop.

**Why it matters**: A loop body that runs N iterations performs
N retain/release pairs today, even if the value never changes.
Hoisting drops that to one pair total.

**Tasks**:

- [ ] LICM (loop-invariant code motion) pass for the wrap. The
  optimiser already has facts for SSA versions; add a "store is
  loop-invariant" check.
- [ ] If the slot is written only by loop-invariant stores AND
  is not read by any inner loop's escape path, hoist the wrap
  out: emit a single retain + release pair before the loop, and
  in the loop body emit a raw `local.set` (the slot's hold is
  already accounted for).
- [ ] Be careful with break / continue paths — must still
  release before the loop's "natural exit" point.

**Files**:

- Modify: `compiler/codegen/wasm/_emitter/_control_flow.py`
- Modify: `compiler/optimiser/_licm.py` if it exists; new
  module under `compiler/optimiser/` if not.

**Test plan**:

- Unit test: `for {set i 0} {$i < 10} {incr i} { set guard $g }`
  emits one retain/release pair around the loop, not ten.
- Sweep: neutral.
- Microbench: a tight assignment loop measurably faster.

**Rollback**: Per-pattern. The hoist is a separate optimiser pass;
revert reverts the pass without affecting the rest of S5.

**Acceptance gate**: Tests pass; microbench shows the expected
N→1 reduction.

**Estimated size**: 2 commits (the pass + integration).

---

### S5.4 — Plug existing optimiser passes into codegen

**Goal**: The compiler already runs SCCP, DCE, GVN, and several
other passes (`compiler/optimiser/`). The WASM codegen
reads few of their facts. Audit and plug each into the codegen
where it can reduce emission.

**Why it matters**: Quality-of-life. The optimiser does the
analysis; the codegen should benefit from it.

**Tasks**:

- [ ] Audit each optimiser pass for facts the WASM codegen
  ignores (look at `compiler/optimiser/_manager.py` for
  the pass list).
- [ ] For each, write a small adapter that exposes the facts to
  `_WasmEmitterBase`.
- [ ] Concrete candidates:
  - SCCP: integer constants known at compile time should
    inline as `i32.const` / `i64.const` directly, not via
    `obj_new_int`.
  - DCE: dead `IRAssignValue` whose result is never read can
    be elided entirely.
  - GVN: redundant `_emit_value` for the same expression can
    reuse a scratch local.
- [ ] Each adapter is one commit; sweep + leakcheck must be
  neutral.

**Files**:

- Modify: `compiler/codegen/wasm/_emitter/_optimisation.py`
  (already exists; extend)
- Modify: `compiler/optimiser/_manager.py` if any pass
  needs to expose more facts.

**Test plan**:

- Unit tests per adapter.
- Sweep: neutral.
- Microbench: incremental wall-time reduction per adapter.

**Rollback**: Per-adapter revert.

**Acceptance gate**: Each adapter ships with a measurable
microbench delta; sweep neutral.

**Estimated size**: 3–6 commits depending on how many adapters
land.

---

## Stage exit criteria

S5 is "done" when:

- The retain/release call count, measured under leakcheck, is
  materially lower than at the end of S2.
- Sweep is neutral or positive throughout.
- Microbenchmarks show measurable wall-time wins per
  optimisation.
- The elided-wrap counters are exposed in CI output so future
  PRs can see when an optimisation regresses.

After S5, the compile-side has both correctness and reasonable
performance. The remaining wins are at the runtime/allocator
layer (S6).
