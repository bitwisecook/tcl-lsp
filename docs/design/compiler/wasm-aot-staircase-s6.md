# Stage S6 — Allocation + small-value representation

> Index: [`wasm-aot-staircase.md`](wasm-aot-staircase.md). Closely
> related to the runtime memory-management plan in
> [`docs/design/runtime/memory-management.md`](../runtime/memory-management.md)
> (Phase MM-D).

S6 collects the allocation-side and value-representation
optimisations that the runtime team had already scoped under MM-D
plus a few extras the compile side benefits from. These are
**independent of S2–S5** — they speed up the runtime regardless of
what the compile side emits — so S6 can land in parallel with the
other stages.

The deliverable is recovery of the per-op cost that MM-A's switch
to wasi-libc malloc gave up. Microbench targets are restated from
the runtime doc:

- `set` / `incr` / `expr` micro-bench within 10 % of pre-MM-A
  bump-allocator numbers.

## Pre-conditions

- **MM-A complete** (libc-coherent allocator) — landed.
- **MM-B complete** (refcount discipline on the runtime side) —
  largely landed; final passes track in
  `docs/design/runtime/memory-management.md`.
- **S0.2 leak counter** to confirm representation changes do not
  introduce leaks.

## Sub-plans

### S6.1 — Re-enable size-class free-lists

**Goal**: Restore the per-size-class free-list reuse that MM-A
disabled (kept the data structure as a no-op layer for ABI
compat). Pop / push to the free-list before falling through to
`malloc` / `free` for the four most-common size classes (32, 48,
64, 96 — the TclObj header + small-string cases).

**Why it matters**: `malloc` round-trip is ~100 ns; a free-list
push/pop is ~5 ns. For workloads that allocate many short-lived
TclObjs (every tcltest), this is half the per-op cost.

**Tasks**:

- [ ] In `runtime/zig/valtypes/tcl_obj.zig::alloc`: when the
  aligned size matches one of the four common classes AND the
  class's free-list is non-empty, pop and return.
- [ ] In `free_obj` / `free_sized`: when size matches AND the
  class's free-list is below cap (256 entries), push.
- [ ] Bound the lists at 256 entries each so a regex stress-test
  does not pin too much memory in the recycler.
- [ ] Microbench: `set i 0` in a tight loop should run within
  10 % of pre-MM-A.

**Files**:

- Modify: `runtime/zig/valtypes/tcl_obj.zig`

**Test plan**:

- Microbench: per-op set/incr times.
- Sweep: neutral.
- Leakcheck: still neutral (free-list reuse must not change the
  alloc/free balance).

**Rollback**: Single revert; the lists become inert again.

**Acceptance gate**: Microbench within 10 % of pre-MM-A; sweep
neutral.

**Estimated size**: 1 commit.

---

### S6.2 — Inline-string optimisation

**Goal**: TclObjs whose string data fits in the obj header
store the bytes inline instead of allocating a separate buffer.

**Implementation note (PR #237 review)**: the original design
proposed using `OBJ_INT_CACHE + OBJ_STR_PTR + OBJ_STR_LEN` to
cover ≤ 23-byte strings.  The landed implementation stores
**only ≤ 8 bytes** inline (in the `OBJ_INT_CACHE` 8-byte slot,
gated by a new `TYPE_INLINE_STRING` tag), leaving `OBJ_STR_PTR`
free to point at the inline payload (so readers don't need a
special-case branch).  Expanding to 23 bytes would mean
spreading a single payload across three header fields, which
complicates every reader and the str_cap-driven free path —
the simpler 8-byte cap covers integer-string conversion
results (the common case for tagged-int → string fall-through)
and short identifiers (`set x …`, `puts foo`).

The 8-byte cap is recorded as `MAX_INLINE_STR = 8` in
`runtime/zig/valtypes/tcl_obj.zig`.  Re-opening the 23-byte
target is a future S6.x sub-plan; the 8-byte landing is the
shipping baseline.

**Why it matters**: A typical Tcl workload allocates many short
strings (identifiers, command names, integer-string conversion
results). Inline storage halves the alloc count for that
population.

**Tasks** (landed):

- [x] New type tag `TYPE_INLINE_STRING = 5` for the inline case.
- [x] `obj_new_string_copy(src, len)` with `len ≤
  MAX_INLINE_STR` copies bytes into the obj's
  `OBJ_INT_CACHE`-aligned slot and sets `OBJ_STR_PTR` to point
  at that inline buffer.
- [x] Every reader (`obj_str_ptr`, `obj_str_len`,
  `obj_ensure_string`) sees the inline payload through
  `OBJ_STR_PTR` without a special-case branch.
- [x] `release_now` skips `free_sized` for inline-string
  TclObjs (no separate buffer to free; `OBJ_STR_CAP` stays 0).
- [x] In-place mutation fast paths (`tcl_cmd_append`,
  `tcl_cmd_lappend`) detect the inline → owned-buffer
  transition when the inline cap is exceeded.

**Hazard / unenforced invariant**: the comment on
`MAX_INLINE_STR` warns "readers must not hold the pointer past
the obj's lifetime".  A `(ptr, len)` pair captured by a long-
lived structure (interp result, parse cache entry) and the obj
subsequently released would alias a recycled obj header with
silent corruption.  The deferred-free queue makes the common
adjacent-buffer case safe but doesn't protect the obj header
itself.  See review thread on `tcl_obj.zig:41`.

**Files**:

- Modify: `runtime/zig/valtypes/tcl_obj.zig` (most of the work)
- Modify: every consumer of the str_ptr / str_len fields

**Test plan**:

- Unit test (Zig): a 5-byte string is stored inline.
- Unit test: a 100-byte string is stored in a separate buffer.
- Unit test: appending 50 bytes to a 5-byte inline string
  promotes to a buffer.
- Sweep: neutral.
- Leakcheck: leak count drops materially (no buffer alloc for
  short strings).

**Rollback**: Single revert. Sweep + leakcheck to verify.

**Acceptance gate**: Tests pass; sweep neutral; leak count
drops measurably.

**Estimated size**: 3–4 commits (representation, readers, fast
paths, tests).

---

### S6.3 — Per-statement arena for parser scratch

**Goal**: Parser scratch (token arrays, `subst` sub_buf, regex
intermediates) live for one command's worth of time. An arena
that resets on `eval_command` boundaries reclaims them in O(1)
without `free` calls.

**Why it matters**: Parser allocation is currently the
single largest source of short-lived allocs. Arena allocation
is a single pointer bump; reset is a single pointer reset.

**Tasks**:

- [ ] Add `runtime/zig/valtypes/tcl_arena.zig`: a fixed-size
  arena (e.g. 64 KB) with `arena_alloc(size)`, `arena_reset()`,
  and overflow-to-libc fallback.
- [ ] Identify call sites whose scratch lifetime ends at
  `eval_command` boundaries (parser tokens, subst buffers,
  regex match buffers).
- [ ] Route those allocations through the arena; reset between
  commands.
- [ ] Be careful: anything that escapes the command (e.g. a
  TclObj that survives because it was stored in a slot)
  must NOT come from the arena. Arena is for true scratch
  only.

**Files**:

- New: `runtime/zig/valtypes/tcl_arena.zig`
- Modify: `runtime/zig/parse/tcl_parse.zig`,
  `runtime/zig/parse/tcl_subst.zig`,
  `runtime/zig/valtypes/tcl_regex.zig`

**Test plan**:

- Sweep: neutral.
- Leakcheck: arena allocations show as neither alloc nor free
  in the counter (they are reset, not freed). No regression.
- Microbench: parser-heavy workloads (regex tests) measurably
  faster.

**Rollback**: Per-call-site revert. Each routing change is
independently reversible.

**Acceptance gate**: Parser microbench faster; sweep neutral.

**Estimated size**: 2–4 commits.

---

### S6.4 — Tagged-immediate small ints

**Goal**: TclObj handles whose value is a small integer (e.g. fits
in 31 bits) store the int directly in the i32 handle (with the
high bit set as a tag), bypassing both allocation and the obj
header.

**Why it matters**: `set i 0`, `incr i`, every loop counter — the
hot allocation path. Tagged immediates eliminate that allocation
entirely for the common small-int case.

**Tasks**:

- [ ] Reserve the high bit of i32 handles: `0` means pointer,
  `1` means tagged immediate.
- [ ] In `obj_new_int(value)`: when value fits in 31 bits, return
  `0x80000000 | (value & 0x7FFFFFFF)` instead of allocating.
- [ ] Update every reader (`obj_get_int`, `obj_ensure_string`,
  `obj_str_ptr`, `obj_str_len`, `obj_type`, `tcl_obj_retain`,
  `tcl_obj_release`) to detect the tag bit and short-circuit.
  Tagged immediates have no refcount (immortal) and no buffer.
- [ ] Update `tcl_obj_retain` / `tcl_obj_release` to be no-ops
  on tagged immediates.
- [ ] Update arithmetic fast paths (`tcl_arith_add`, etc.) to
  use the tag-bit path directly without alloc.
- [ ] Update the WASM codegen so int literals ≤ 31 bits emit the
  tagged immediate directly instead of `obj_new_int`.

**Files**:

- Modify: most of `runtime/zig/valtypes/`
- Modify: `core/compiler/codegen/wasm/_emitter/_values.py`
  (`_emit_obj_literal` int branch)

**Test plan**:

- Unit test (Zig): `obj_new_int(42)` returns a tagged
  immediate; `obj_new_int(2**40)` allocates.
- Unit test: tagged immediate retain/release are no-ops.
- Sweep: neutral.
- Leakcheck: massive leak-count drop — int objs in tight loops
  no longer allocate.
- Microbench: `incr` loop measurably faster.

**Rollback**: Single revert. The tag-bit convention is
all-or-nothing — partial rollback is unsafe.

**Acceptance gate**: Tests pass; sweep neutral; leak count
drops materially; microbench `incr` faster.

**Estimated size**: 3–5 commits.

---

## Stage exit criteria

S6 is "done" when:

- Free-list reuse is on for the common size classes.
- Inline strings work and reduce alloc count.
- Per-statement arena handles parser scratch.
- Tagged immediates handle small ints.
- Microbench targets met:
  - `set` within 10 % of pre-MM-A.
  - `incr` within 10 % of pre-MM-A.
  - `expr` within 10 % of pre-MM-A.
- Sweep + leakcheck baselines materially better than today.

## Independence

S6 has no compile-side prerequisites beyond what the runtime
already has. It can land in parallel with S1–S5; in particular,
S6.4 (tagged immediates) is the largest single
performance recovery and should be prioritised once MM-B finishes
on the runtime side.

## Ordering vs the runtime memory-management doc

The runtime doc (`docs/design/runtime/memory-management.md`) lists
MM-D as a future phase covering S6.1 (free-list reuse), S6.2
(inline strings), and S6.3 (arena). S6.4 (tagged immediates) was
explicitly out of scope there. This doc treats all four as one
stage because from the staircase's perspective they are
representation choices that affect every other stage's
performance equally.

If MM-D progresses on the runtime side independently, S6 here
should track those changes by linking commit hashes rather than
duplicating the work.
