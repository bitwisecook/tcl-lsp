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

(remaining sub-plans S2.2 … S2.7 + stage exit criteria appended in
follow-up commits — see git log of this file.)
