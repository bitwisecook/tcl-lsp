# Stage S4 — Inlining of small leaf procs

> Index: [`wasm-aot-staircase.md`](wasm-aot-staircase.md). Prior stages
> [S2](wasm-aot-staircase-s2.md), [S3](wasm-aot-staircase-s3.md).

S2 made elision correct; S3 made it cover more procs. S4 goes one
step further: for procs that are small AND `pure_leaf` (S3.4), splice
the callee's IR into the caller's IRBlock so the call vanishes
entirely. Frame elision then has a strictly larger surface to work on
(the inlined body's locals join the caller's escape set), and the
runtime never even sees the proc-call boundary.

The deliverable is a measurable wall-time reduction on a workload
that is dominated by short proc calls. The benchmark target is
`scripts/dev/perf_microbench.py` — a `proc add {a b} { return [expr {$a +
$b}] }` invoked in a loop should run within 2× of an inline `expr {$a
+ $b}` after S4.

## Pre-conditions

- **S2 + S3 complete.** Inlining without correct refcount
  discipline (S2) explodes leaks; without `pure_leaf` (S3.4) the
  inliner has no safe predicate.
- **S0.4 leakcheck** to spot any inliner-introduced leaks.

## Sub-plans

### S4.1 — Catalogue inlining-eligible procs

**Goal**: Tag every IR proc that is **safe** and **profitable** to
inline. Safety is `pure_leaf` from S3.4. Profit is a heuristic on
size (≤ N statements) and call-site characteristics (single-call vs
many-call).

**Why it matters**: Without an explicit catalogue, the inliner
either inlines too aggressively (code bloat) or too rarely (no
benefit). The catalogue separates the policy from the mechanism.

**Tasks**:

- [ ] Add a `InlineDecision` enum to
  `core/compiler/ir.py`: `NEVER` | `ALWAYS` | `IF_SINGLE_CALL` |
  `IF_HOT`.
- [ ] Compute `inline_decision` per proc on `IRProcedure`:
  - `NEVER` if `not pure_leaf` or body size > threshold or has
    upvar / uplevel / info / tailcall (already excluded by
    pure_leaf, but explicit guard).
  - `ALWAYS` if pure_leaf AND body size ≤ small threshold (e.g. 5
    statements) AND every callee body fits the same rule (already
    inlined transitively).
  - `IF_SINGLE_CALL` if pure_leaf AND only one static call site
    in the module.
  - `IF_HOT` reserved for S4.3.
- [ ] Compute `static_call_count` per proc — how many times the
  module statically references it.
- [ ] Surface the decision in
  `scripts/profile_frame_elision.py` so the per-bundle counts are
  visible.

**Files**:

- Modify: `core/compiler/ir.py` (add the enum + fields to
  `IRProcedure`)
- New: `core/compiler/inlining/decision.py` for the policy logic.

**Test plan**:

- Unit tests for each `InlineDecision` branch.
- Sweep: neutral (catalogue only).
- Profile output shows non-zero counts of `ALWAYS` and
  `IF_SINGLE_CALL`.

**Rollback**: Single revert; nothing acts on the tag yet.

**Acceptance gate**: Catalogue produces deterministic decisions on
the in-scope test bundles; counts logged in CI.

**Estimated size**: 1 commit.

---

### S4.2 — IR-level inlining

**Goal**: Substitute the callee's IR into the caller's IRBlock
when `inline_decision in (ALWAYS, IF_SINGLE_CALL)`. After the
inline, the caller re-runs escape analysis and re-emits, which
naturally folds the inlined locals into the caller's elision
decision.

**Why it matters**: This is where the wall-time win lives. A 5-statement
`pure_leaf` proc inlined at every call site removes the entire
call/return overhead and unlocks SCCP / DCE / further escape
proofs in the caller's body.

**Tasks**:

- [ ] New module `core/compiler/inlining/inline_pass.py` that
  walks the IR module post-lowering, pre-codegen.
- [ ] For each `IRCall(command=…)` where the resolved
  `IRProcedure.inline_decision` is `ALWAYS` or `IF_SINGLE_CALL`:
  - Substitute the callee's body IR for the call.
  - α-rename the callee's body locals to fresh names so they do
    not collide with the caller's locals.
  - Replace `IRReturn` in the inlined body with assignment to
    the caller's `defs[0]` slot (if any) and a structured exit
    out of the inlined block — emit a labelled IRBlock around
    the inlined body so internal `IRReturn` translates to
    `IRBreak` against that label.
- [ ] Re-run var-escape on the modified IR module so the inlined
  locals participate in the caller's escape decision.
- [ ] After inlining, the original proc may be unreferenced —
  drop unreferenced `IRProcedure` entries to shrink the module.
- [ ] Be careful with `proc f {x} { … return $x } / f $literal`
  — the inliner must respect the value-vs-reference semantics of
  Tcl args (always by value).

**Files**:

- New: `core/compiler/inlining/inline_pass.py`
- Modify: `core/compiler/lowering.py` or
  `core/compiler/codegen/wasm/__init__.py` to invoke the pass at
  the right point in the pipeline.
- Modify: `core/compiler/var_escape/__init__.py` so the pass can
  re-run after inlining.

**Test plan**:

- Unit test: `proc inc {x} { return [expr {$x + 1}] }` called
  three times, the resulting WASM has zero `proc_register_compiled`
  for `::inc` and the call sites are inline arithmetic.
- Sweep: neutral or positive (the codegen output gets smaller and
  fewer indirect calls; both effects are wins).
- Leakcheck: neutral.
- Microbench: inline `inc` matches inline `expr {$x + 1}` within
  20 %.

**Rollback**: Single revert. The pass is opt-in via a codegen
flag (`--inline=disabled` falls back to no inlining); revert
defaults the flag to `disabled` until the bake-in is confirmed.

**Acceptance gate**:

- Sweep neutral or positive.
- Microbench delta meets the 2× target stated at the top of S4.
- Codegen output is strictly smaller for bundles with many small
  procs.

**Estimated size**: 4–6 commits — IR substitution, α-rename,
return-as-break translation, re-run pass, dead-proc pruning,
flag plumbing. Each is testable independently.

---

### S4.3 — Hot-proc WASM-level inlining (deferred)

**Goal**: For procs that S4.2 cannot inline at the IR level
(too large, multiple call sites, dynamic dispatch), allow a
profile-guided pass to inline at the WASM level: copy the
callee's compiled function body into the caller's call site.

**Why it matters**: A handful of "hot" procs (tcltest's `test`
wrapper, `lassign`, the regex compiler) account for a
disproportionate share of runtime. Inlining them even though they
are not pure_leaf is a measurable win, but only if we have a
profile to identify which ones.

**Tasks** (deferred — no concrete plan until S4.2 ships and we
measure the residual call overhead):

- [ ] Add a profile-collection mode that counts call frequency
  per qualified proc name during a sweep run.
- [ ] Surface the top-N procs as candidates.
- [ ] Decide: WASM-level inlining (copy compiled function bytes)
  vs IR-level inlining with a relaxed safety predicate ("hot but
  not provably safe — accept the risk").
- [ ] Implement and measure.

**Estimated size**: deferred — at least 5 commits when the time
comes.

---

## Stage exit criteria

S4 is "done" when:

- `inline_decision` is computed and surfaced in the elision
  profile.
- `S4.2` lands net-positive on sweep + microbench.
- `S4.3` is documented but not implemented (acceptable; ship the
  S4.2 wins first).

After S4, the AOT compiler is producing materially smaller and
faster WASM for proc-heavy workloads. The remaining wins are
codegen-internal (S5) and allocator-level (S6).
