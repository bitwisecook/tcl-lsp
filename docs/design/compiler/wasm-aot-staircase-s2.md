# Stage S2 — Per-proc frame elision with refcount discipline

> Index: [`wasm-aot-staircase.md`](wasm-aot-staircase.md). Foundation in
> [`wasm-aot-staircase-s0.md`](wasm-aot-staircase-s0.md). Floor in
> [`wasm-aot-staircase-s1.md`](wasm-aot-staircase-s1.md).

S2 is the keystone stage. A previous session attempted it in one shot
(branch `claude/tcl-wasm-refcount-discipline-001`, reverted) and the
failure modes mapped directly to "did everything at once with no
partial-progress measurement". The fix is the same principle as the
runtime side's MM-B work: catalogue every store site, classify the
ownership at that store, then change one site at a time with the leak
counter pinned at zero between commits.

## Pre-conditions

- **S0 complete** (leak counter, repro, ratcheting CI gate).
- **S1 complete** (the framed path is the safety floor; rollback for
  any S2 sub-plan is "set `frame_elision=False` for the affected
  procs").

## Sub-plans

S2 splits into seven sub-plans. Each is sized for 1–3 commits and
each one's sweep + leakcheck must be net-neutral or net-positive
before moving on.

### S2.1 — Ownership enum on `_emit_value`

**Goal**: Every value pushed onto the WASM operand stack carries a
compile-time tag of `OWNED` (caller owns +1, can transfer) or
`BORROWED` (caller does not own; if a slot wants it, retain). The
keystone — every later sub-plan reads this tag.

**Why it matters**: The failed attempt treated all writes uniformly —
always retain new and release prior. This over-counted on `OWNED`
sources (which already brought a +1), producing a +1 leak per
literal/runtime-call write, and broke runtime fast paths that check
`rc == 1`. A correct discipline must distinguish the two cases.

**Tasks**:

- [ ] Define `class Ownership(Enum): OWNED, BORROWED` in
  `core/compiler/codegen/wasm/_ir.py` (or a dedicated
  `_ownership.py`).
- [ ] Change `_WasmEmitterValuesMixin._emit_value(value, *, was_braced)`
  return type from `None` to `Ownership`. Update the type stub in every
  sibling mixin's `if TYPE_CHECKING` block.
- [ ] Classify each `_emit_value` branch:
  - `_emit_var_read_obj(name)` → `BORROWED`
  - `_emit_obj_literal(value)` → `OWNED`
  - `_emit_command_subst_value(text)` → `OWNED` (runtime returns +1)
  - `_emit_interpolated_value(value)` → `OWNED` (concat allocs fresh)
  - `_emit_eval_fallback(...)` → `OWNED` (`tcl_eval` returns +1)
- [ ] Same for `_emit_expr_obj`: most expr ops are `OWNED` (allocate
  a fresh boxed result); the special-case `local.get` of an int slot
  in a tail position is `BORROWED`.
- [ ] Threading: every caller of `_emit_value` either propagates the
  return up or makes an explicit decision. ~160 call sites; a
  mechanical refactor with type-checker assistance.
- [ ] Add a debug-only assertion in `_emit_owned_local_write` (lands
  in S2.2) that the `Ownership` parameter is passed (i.e. nobody
  defaulted away the tag).

**Files**:

- New: `core/compiler/codegen/wasm/_ownership.py` (the enum + small
  helpers like `Ownership.OWNED.is_transferable`)
- Modify: `core/compiler/codegen/wasm/_emitter/_values.py`
  (return-type changes)
- Modify: every other emitter mixin under
  `core/compiler/codegen/wasm/_emitter/` to thread the return up
- Modify: `core/compiler/codegen/wasm/_emitter/cmds/*.py` (per-command
  hooks all call `_emit_value`)

**Test plan**:

- The refactor itself is type-driven: ty / mypy will catch every site
  that drops the new return value.
- Sweep: must be net-zero (no behaviour change yet — we are only
  threading a tag, not reading it).
- Leakcheck: must be net-zero.
- Add a unit test that compiles `set x 1; set y $x` and asserts the
  emitted IR records `OWNED` for the literal `1` and `BORROWED` for
  the `$x` read.

**Rollback**: Revert the commit. The enum is purely additive; nothing
acts on it yet.

**Acceptance gate**: ty / mypy clean, sweep neutral, leakcheck
neutral, the unit test passes.

**Estimated size**: 2–3 commits. The first lands the enum + signature
change with a default value (`OWNED` to be safe); subsequent commits
remove the default and force every caller to be explicit.

---

### S2.2 — Owned-slot primitives

**Goal**: Add `_emit_local_set_owned(idx, source: Ownership)` and
`_emit_local_tee_owned(idx, source)` that route through one of two
emission paths based on `source`:

- `OWNED`: store directly. The slot now owns the +1 the value brought
  with it. Release the prior slot value.
- `BORROWED`: retain the new value (slot needs its own +1), then
  release the prior slot value.

**Why it matters**: The failed attempt always retained — that worked
for `BORROWED` but doubled-up for `OWNED`, leaking +1 per write. With
`source` available (from S2.1), the primitive picks the right path
and ownership is exactly +1 in the slot at all times.

**Tasks**:

- [ ] In `core/compiler/codegen/wasm/_emitter/_core.py` add:
  ```python
  def _emit_local_set_owned(self, idx: int, source: Ownership) -> None:
      if not self._owned_local_wrap_active(idx):
          self._emit_local_set_raw(idx)
          return
      if source is Ownership.OWNED:
          # Stack: [v]; v has +1 we transfer to the slot.
          self._emit_local_get(idx)             # push prior
          self._emit_call(self._release_idx)    # release prior
          self._emit_local_set_raw(idx)         # transfer +1 to slot
      else:                                     # BORROWED
          # Slot must claim its own +1.
          tmp = self._rc_set_scratch_lazy()
          self._emit_local_tee_raw(tmp)         # tmp := v, leave on stack
          self._emit_call(self._retain_idx)     # retain
          self._emit_local_get(idx)             # push prior
          self._emit_call(self._release_idx)    # release prior
          self._emit_local_get(tmp)
          self._emit_local_set_raw(idx)
  ```
- [ ] Same shape for `_emit_local_tee_owned` — leaves the value on
  stack at the end.
- [ ] Keep the raw helpers (`_emit_local_set_raw`,
  `_emit_local_tee_raw`) for scratch slots and for the wrap's
  internal use (so the wrap does not recurse).
- [ ] Make `_owned_local_wrap_active(idx)` (already drafted in the
  failed attempt) the single gate: returns True only when
  `is_proc and not wants_frame and idx in _owned_locals_set and
  retain/release imports loaded`.
- [ ] Cache one `_rc_set_scratch` per emitter — do not allocate a
  fresh scratch local per write.

**Files**:

- Modify: `core/compiler/codegen/wasm/_emitter/_core.py`
- Modify: `core/compiler/codegen/wasm/_emitter/_variables.py` (export
  the helpers via the mixin's TYPE_CHECKING block)

**Test plan**:

- Unit test: compile `set x 1` (`OWNED` source) inside a frame-elided
  proc and assert the emitted WASM has zero `tcl_obj_retain` calls
  for the `set x 1` site (only the prior-release; nothing extra).
- Unit test: compile `set y $x` (`BORROWED` source) and assert one
  `tcl_obj_retain` plus one `tcl_obj_release`.
- Sweep: still net-zero — nothing yet calls the new helpers.
- Leakcheck: still net-zero.

**Rollback**: Helpers are unused at this stage; revert is risk-free.

**Acceptance gate**: Helpers exist, are documented, and the unit
tests above pass. Sweep + leakcheck both neutral.

**Estimated size**: 1 commit.

---

### S2.3 — Migrate every owned-slot write site, one at a time

**Goal**: Replace every direct `_emit_local_set` / `_emit_local_tee`
that targets a Tcl-variable slot with the `_owned` variant from
S2.2, threading the right `Ownership` tag for the value source.
Each migration is one commit with a sweep + leakcheck delta proving
no regression and (where relevant) a leak reduction.

**Why it matters**: The failed attempt put the wrap inside
`_emit_local_set` itself and silently affected every caller. That
was too wide a net: scratch-local writes got wrapped (no-op due to
the `_owned_locals_set` filter, but adds noise), some loop-internal
writes got wrapped at the wrong time, and the wrap fired during
prologue/epilogue helpers that needed raw semantics. Per-site
migration keeps the blast radius small and lets the leak counter
identify exactly which migration introduced any regression.

**Tasks** (each bullet is its own commit):

- [ ] **`_emit_var_write_obj_impl`** (in-proc plain WASM-local
  branch) — the main `set var x` site. Pass the `Ownership`
  through from the `_emit_value` call that produced the stack
  value.
- [ ] **`IRIncr` codegen** (`_emit_stmt` `case IRIncr`) — the
  `_emit_box_int()` after `i64.add` returns `OWNED` (fresh int
  obj). Use `_emit_local_set_owned(idx, OWNED)`.
- [ ] **`foreach` loop variable** (`_emit_foreach`,
  `_control_flow.py:248`) — `tcl_list_index` returns `OWNED`.
- [ ] **`switch` subject local** (line 285, 345) — `OWNED` if the
  subject came from `_emit_value`, otherwise scratch (use the raw
  helper).
- [ ] **`switch` rv local** (line 445) — match-case result. `OWNED`.
- [ ] **`for` loop init** — typically lowered into individual
  `IRAssignValue` statements via the IR; the existing
  `_emit_var_write_obj` covers it. Verify and add a unit test.
- [ ] **`while` loop body** — same; covered by the variable path.
- [ ] **default-substitution prologue** (`_core.py:828-833`) — the
  literal stored is `OWNED`. Use the owned variant.
- [ ] **lappend in-proc fast path** (`cmds/lappend_.py:60-68`) —
  `tcl_cmd_lappend` returns `OWNED`. The `keep_last` branch uses
  tee.
- [ ] **`dict` mutators** (`cmds/dict_.py`) — every site that
  stores back into the named variable.
- [ ] **`array set` initialiser** if it stores via local.set (audit
  needed).
- [ ] **`regexp` capture variable assignment** — captures land in
  named slots; the runtime helper that produces them returns
  `OWNED` (`obj_new_string_copy`).

For each migration:

1. Identify the source's `Ownership` (most are `OWNED` because they
   come from a runtime call or a fresh literal; only direct `$var`
   reads are `BORROWED`).
2. Replace `_emit_local_set(idx)` with
   `_emit_local_set_owned(idx, source)`.
3. Run `make sweep` and `make leakcheck`. Diff vs prior commit.
4. If the sweep regresses on any file, identify whether the source
   tag is wrong (most common) or the runtime path has a bug exposed
   by the new discipline (rare; file an issue and pin the failing
   site to the framed path via `frame_elision=False` on that proc).

**Files**: many — every emitter file under
`core/compiler/codegen/wasm/_emitter/`, plus the per-command hooks
under `cmds/*.py`. Roughly one file per migration commit.

**Test plan**:

- Per-commit: sweep delta is non-negative; leak baseline shrinks or
  stays equal.
- Aggregate: by the last migration, leak baseline for previously
  frame-elided procs is much smaller than today.
- Add per-pattern unit tests under
  `tests/test_wasm_owned_slots.py` covering: literal store,
  var-to-var copy, runtime call result store, lappend in loop,
  foreach loop variable, default-sub fall-through.

**Rollback**: Per-commit revert if a single migration regresses.
The other migrations are independent.

**Acceptance gate**: Every direct `_emit_local_set(idx)` /
`_emit_local_tee(idx)` where `idx in _owned_locals_set` is gone.
A grep ratchet (`scripts/check_owned_local_writes.py`) blocks new
direct writes from sneaking in.

**Estimated size**: ~12 commits (one per migration site). Two CI
runs per commit (sweep + leakcheck).

---

### S2.4 — Prologue: claim ownership of params

**Goal**: At function entry, each param slot holds the caller's
handle (rc reflects the caller's hold). The slot must claim its own
+1 so subsequent stores to the slot can release the prior value
without dropping the caller's hold.

**Why it matters**: Without this, the first overwrite of a param
inside the body releases what the caller still holds → use-after-free
in the caller. The failed attempt did this but in the wrong order
relative to default-substitution.

**Tasks**:

- [ ] In `_core.py::generate`, before the default-substitution
  block, emit for each non-empty param `i`:
  ```python
  self._emit_local_get(i)
  self._emit_call(self._retain_idx)
  ```
  Gated on `is_proc and not wants_frame and retain import loaded`.
- [ ] `tcl_obj_retain` is null-safe (S0 prerequisite — null guard
  added to the runtime function), so a slot left at 0 (caller
  passed nothing, no default) is a silent no-op.
- [ ] After the param-retain pass, run the default-substitution
  pass. The default literal is `OWNED`; the wrapped store from
  S2.3 releases the prior caller-retained value (rc--, still ≥1
  because caller still owns) and transfers the literal's +1 to
  the slot.

**Files**: `core/compiler/codegen/wasm/_emitter/_core.py`.

**Test plan**:

- Unit test: a one-arg proc `proc f {x} { return $x }`, called
  with a fresh literal, returns the same handle (rc unchanged
  net-net through the call).
- Unit test: a two-arg proc with a default for the second arg,
  called with one arg, returns the default literal — and the
  caller-held first arg is unchanged at the call boundary.
- Sweep: net-zero delta.
- Leakcheck: marginal increase possible (default literals leak
  +1 each call) — quantify the regression and ratchet baseline.

**Rollback**: Single-commit revert.

**Acceptance gate**: Unit tests pass; sweep neutral; leak delta
explained.

**Estimated size**: 1 commit.

---

### S2.5 — Epilogue: release every owned slot before return

**Goal**: At every `RETURN` and the natural fall-through `END`,
release each owned slot's value. Wrap around the return value so
the upcoming releases cannot queue the return value for free.

**Why it matters**: Without epilogue release, every owned slot
leaks its current value (rc=1 forever). The failed attempt did
this but the wrap-around-return-value got the stack ordering
slightly wrong on one of the fall-through paths.

**Tasks**:

- [ ] In `_core.py::generate`, build `cleanup_instrs` for the
  frame-elided case:
  ```
  local.tee save_local      # save return value
  call retain               # protect return value from upcoming releases
  for slot in self._owned_locals:
      local.get slot
      call release
  local.get save_local      # restore return value to stack
  ```
- [ ] `save_local` is allocated lazily via `_add_extra_local`
  ONCE per emitter (re-used at every RETURN site).
- [ ] Use the existing cleanup walker (already injects
  `frame_pop` / `ns_restore` before each RETURN) — append the
  release sequence ahead of those.
- [ ] On the implicit fall-through path (where the codegen
  emits `i32.const 0; END`), the cleanup goes between the
  `0` and the `END`. The `0` retain/release is null-safe.

**Files**: `core/compiler/codegen/wasm/_emitter/_core.py`.

**Test plan**:

- Unit test: a proc that allocates many literals in a loop and
  returns the last one. Leak count under `make leakcheck` is
  bounded (no unbounded growth across iterations).
- Unit test: a proc with multiple RETURN sites — each path
  releases all owned slots correctly. Verify by inspecting the
  WASM disassembly.
- Sweep: net-zero or net-positive (this is the missing release
  side; combined with S2.4 the leak baseline drops materially).
- Leakcheck: leak baseline ratchets DOWN — this is the first
  sub-plan that should produce a measurable improvement.

**Rollback**: Single-commit revert.

**Acceptance gate**: 

- Sweep neutral or positive.
- Leakcheck strictly improves (fewer leaked TclObjs per
  test file).
- The canonical-bug repro from S0.3 starts passing for
  frame-elided procs (the original use case).

**Estimated size**: 1 commit, possibly 2 if the cleanup walker
needs refactoring to support multiple injection groups.

---

(remaining sub-plans S2.6 + S2.7 + stage exit criteria appended in
follow-up commits.)
