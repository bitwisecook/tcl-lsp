# Stage S3 — Interprocedural escape-analysis tightening

> Index: [`wasm-aot-staircase.md`](wasm-aot-staircase.md). Foundation in
> [`wasm-aot-staircase-s0.md`](wasm-aot-staircase-s0.md). Floor in
> [`wasm-aot-staircase-s1.md`](wasm-aot-staircase-s1.md). Per-proc
> discipline in [`wasm-aot-staircase-s2.md`](wasm-aot-staircase-s2.md).

S2 made frame elision **correct**; S3 makes it **available to more
procs**. Today's escape analysis (`core/compiler/var_escape/`) has
already done the heavy lifting (per-proc tags + interprocedural
fixpoint via `_interprocedural.py`). S3 audits the conservative
backstops and tightens them where the cost is concrete.

The deliverable is a higher count of frame-elided procs at the end
of compilation, measured by a new
`scripts/profile_frame_elision.py` that runs the in-scope tcltest
bundles and reports `(file, total_procs, elided_procs)`.

## Pre-conditions

- **S2 complete**. Without correct per-proc discipline, more
  elision means more bugs; tightening is a hazard rather than a
  win.
- **S0.4 leakcheck baseline**. Each tightening must not regress
  the baseline.

## Sub-plans

### S3.1 — Tighten `info level` propagation

**Goal**: Today's check `_body_references_info_level()` looks for
the literal token `info level` anywhere in the proc body. That
catches false positives — a proc that does `if {…} { … } else {
  set info level "" }` has no real `info level` call but the
substring scanner trips. Replace the substring scan with a
proper IR walk that only counts genuine command invocations of
`info level`.

**Why it matters**: `info level` use is rare; false positives
disable elision for ~5–10 % of procs that could otherwise elide.

**Tasks**:

- [ ] Replace `_body_references_info_level()` with a recursive
  IR walker that visits every `IRCall(command="info", args=…)`
  and `IRBarrier` whose first arg is `level`.
- [ ] Same treatment for `info frame`, `info args`, `info body`
  if any of them currently force frame retention.
- [ ] Add a unit test: a proc body that contains the literal
  string `info level` inside a `set var "..."` call but never
  invokes the command — must elide.
- [ ] Document in `docs/design/compiler/var-escape-analysis.md`.

**Files**:

- Modify: `core/compiler/codegen/wasm/_emitter/_core.py`
  (`_body_references_info_level`)
- New utility under `core/compiler/var_escape/_info_subcommands.py`
  (already exists — extend rather than duplicate).

**Test plan**: Per-test unit; sweep neutral or positive;
elision-profile shows more procs elided.

**Rollback**: Single revert.

**Acceptance gate**: Elision profile ≥ today; sweep neutral.

**Estimated size**: 1 commit.

---

### S3.2 — Tighten `has_fallback`

**Goal**: `summary.has_fallback = True` today fires whenever the
intraprocedural pass sees an `IRCall` it cannot statically resolve.
Many of those are commands with real runtime imports (`puts`,
`incr`, `string length`, …) that the codegen dispatches via the
import table without ever calling `tcl_eval`. A real
"falls back to the interpreter" set is much smaller.

**Why it matters**: `has_fallback` is one of the gates on frame
elision (`_core.py:529`). A tighter notion increases elision
coverage materially.

**Tasks**:

- [ ] Define "real fallback" precisely: an `IRCall` is a real
  fallback iff (a) the command has no `wasm_runtime_import` in
  any of its registered specs AND (b) the command is not in the
  built-in NOP list (`command_emits_nothing`).
- [ ] Update the var-escape pass that sets `has_call_fallback`
  to use the precise definition.
- [ ] Keep the conservative version available as
  `has_any_unresolved_call` for callers that need it
  (diagnostics, static analysis); the elision gate switches to
  the precise version.
- [ ] Audit every consumer of `has_fallback` for assumptions
  that might break under the tighter definition.

**Files**:

- Modify: `core/compiler/var_escape/_propagation.py`,
  `_interprocedural.py`, `_types.py`
- Modify: `core/compiler/codegen/wasm/_emitter/_core.py`

**Test plan**:

- Unit test: a proc body containing only `puts "hi"` should now
  elide. Today it does not (because `puts` is sometimes routed
  through the eval fallback for `-nonewline` etc.).
- Sweep: neutral or positive.
- Leakcheck: neutral.

**Rollback**: Per-commit revert; the conservative
`has_any_unresolved_call` remains as a safety fallback.

**Acceptance gate**: Elision profile up; sweep neutral.

**Estimated size**: 2 commits (one for the precise definition,
one for the consumer migration).

---

### S3.3 — Cross-proc upvar-source aggregation audit

**Goal**: The fixpoint already aggregates `upvar_source_names`
across statically resolvable callees. Audit the corner cases —
mutual recursion, dynamic dispatch through a const string,
`upvar` with a computed name — and add explicit tests.

**Why it matters**: Confidence. The fixpoint is correct in the
common cases but the corner cases are the ones that bite during
S2's per-proc migration.

**Tasks**:

- [ ] Build a fixture set under
  `tests/test_var_escape_corners.py` covering:
  - Direct recursion `proc f {} { f }`.
  - Mutual recursion `proc a {} { b }` / `proc b {} { a }`.
  - Const-dispatch `set name foo; $name a` where `foo` is
    statically known.
  - `upvar 1 [const-string] target` where the source name is
    const-folded.
  - Dynamic source name `upvar 1 $name target`.
- [ ] For each, snapshot the expected escape summary.
- [ ] Compare against the current fixpoint output.
- [ ] Fix any divergence in the analysis (not the test).

**Files**:

- New: `tests/test_var_escape_corners.py`
- Modify (as needed): `core/compiler/var_escape/_interprocedural.py`

**Test plan**: Pure unit tests; no sweep impact expected unless
a bug is found.

**Rollback**: Per-fix revert.

**Acceptance gate**: All corner-case fixtures pass; any analysis
fix has its own commit.

**Estimated size**: 1–3 commits depending on bugs found.

---

### S3.4 — Tag pure leaf procs

**Goal**: Add a `pure_leaf: bool` field to `ProcEscapeSummary`,
true when:

- no `upvar` / `uplevel` / `info` / `tailcall` use (already
  required for elision today),
- no global mutation (no `set ::x`, no `global` decl with a
  write),
- no I/O (no `puts`, `chan`, `socket`, …),
- no command spec that has `side_effect = True` in the
  registry,
- all callees are themselves `pure_leaf`.

**Why it matters**: S4 (inlining) needs a precise predicate for
"safe to inline". `pure_leaf` is that predicate. Computing it
here, in the escape pass, keeps the fixpoint logic in one place.

**Tasks**:

- [ ] Extend `ProcEscapeSummary` with `pure_leaf: bool = False`.
- [ ] Compute the local part (no upvar/uplevel/info/tailcall, no
  global mutation, no I/O) in `_propagation.py`.
- [ ] Compute the transitive part (all callees pure_leaf) in
  `_interprocedural.py` by including `pure_leaf` in the fixpoint.
- [ ] Add a `CommandSpec.has_side_effect` accessor and ensure
  every spec in `core/commands/registry/tcl/` has it set
  correctly.
- [ ] Document in
  `docs/design/compiler/var-escape-analysis.md`.

**Files**:

- Modify: `core/compiler/var_escape/_types.py`,
  `_propagation.py`, `_interprocedural.py`
- Modify: `core/commands/registry/models.py` (add accessor)
- Modify: every spec under `core/commands/registry/tcl/` that
  currently lacks `has_side_effect` (audit pass).
- Modify: `docs/design/compiler/var-escape-analysis.md`

**Test plan**:

- Unit test: a proc with only arithmetic and local set/get is
  pure_leaf.
- Unit test: a proc that calls `puts` is not.
- Unit test: a proc that calls another pure_leaf proc is
  pure_leaf.
- Unit test: a proc that calls a non-pure proc is not.
- Sweep: neutral (this is metadata; nothing acts on it yet).
- Leakcheck: neutral.

**Rollback**: Single revert.

**Acceptance gate**: Tests pass; sweep neutral; the count of
`pure_leaf` procs across the in-scope tcltest bundles is
non-zero (recorded in the elision profile output).

**Estimated size**: 2 commits (the field + propagation; the
spec audit).

---

## Stage exit criteria

S3 is "done" when:

- `info level` and `has_fallback` are precise (not conservative).
- The fixpoint passes the corner-case fixtures.
- `pure_leaf` is computed and documented.
- Elision profile shows materially more elided procs than today
  across the in-scope tcltest bundles.
- Sweep + leakcheck baselines unchanged or improved.

After S3, S4's inlining work has a precise "safe to inline"
predicate (`pure_leaf`) and a larger surface of frame-elided
callsites to operate on.
