# Tcl-WASM runtime: memory-management design

## Problem statement

The Zig WASM runtime allocates TclObjs and their string buffers on
the wasm linear memory.  Until this commit the allocator was a
"bump pointer + size-class free-lists" scheme that worked fine for
bounded workloads but had three structural defects:

1. **Heap incoherence with wasi-libc.**  The vendored Spencer
   regex engine (`runtime/zig/vendor/tcl-regex/`) calls `MALLOC`/
   `FREE` which bind to wasi-libc's dlmalloc.  dlmalloc's heap
   starts at `__heap_base` and grows upward.  Our bump pointer
   started at a fixed offset (originally 64 KB, then 17 MB after
   the Phase 1.3 data-segment fix) and *also* grew upward.  On
   heavy regex workloads — `regexp.test` compiles thousands of
   regex_t — wasi-libc's heap eventually crossed our bump base
   and the two consumers stomped on each other's free-list
   metadata.  Symptom: ``out of bounds memory access at <text-
   bytes-as-pointer>`` deep in `c.malloc.malloc`.

2. **Refcount infrastructure exists but is barely used.**  Each
   TclObj has an `OBJ_REFCOUNT` field plus
   `tcl_obj_retain` / `tcl_obj_release` / `tcl_obj_drain_pending`
   exports, but only two call sites in the entire runtime
   actually invoke them (both added incidentally as part of the
   Phase 1.3 / 4.5 fixes).  The frame, proc, namespace, and
   dispatch layers all hold TclObj references without retaining
   them.  Consequence: every freshly-allocated TclObj is
   effectively immortal — `release` is never called, so the
   refcount never drops to zero, so the buffer is never freed.

3. **Borrow vs own ambiguity.**  Some TclObjs own their buffer
   (`OBJ_STR_CAP > 0`) and the buffer must be freed when the
   TclObj dies.  Other TclObjs borrow their buffer from a parent
   script source or from a sibling TclObj (`OBJ_STR_CAP == 0`).
   The ownership flag is set correctly at construction but the
   *lifetime relationship* between borrower and lender is not
   tracked.  Phase 1.3 surfaced this when parser-borrowed words
   stored in proc bodies went stale because their lender (the
   outer script) was released first.

This document fixes #1 immediately, then plans a staged solution
for #2 and #3.

## Phase MM-A (this commit) — coherent allocator

`runtime/zig/valtypes/tcl_obj.zig::alloc` now routes every
allocation through wasi-libc `malloc`.  `free_obj` and
`free_sized` route through `free`.  The size-class free-lists are
kept as a no-op layer for ABI compat (callers that read them
still see a valid empty list) but are never populated.

Effect:
- One coherent heap shared with the regex engine.  No more
  cross-allocator corruption.
- `regfree_safe` (the wrapper around the engine's `TclReFree`)
  goes back to calling the real engine cleanup on success-path
  callers, since the call_indirect now lands on a sane heap.
- The bump pointer is gone.  `tcl_test_heap_ptr` returns 0 (kept
  as a stub for the existing test export).
- `@wasmMemoryGrow` calls inside our allocator are gone — wasi-
  libc handles linear-memory growth via its own sbrk path.

Trade-off: per-allocation cost rises from ~5 ns (bump add) to
~100 ns (libc malloc).  For tcltest workloads this is invisible
— the parser, dispatch, and command bodies dominate by orders of
magnitude.  For per-op microbenchmarks (`set`, `incr`, etc.) the
cost shows up in the alloc-bound ones; we accept that trade for
correctness.

## Phase MM-B — finish refcounting

Goal: every TclObj reference holder explicitly retains and
releases.  Once that's true, refcount-driven `free` is the
canonical lifetime mechanism, leaks become impossible, and the
borrow-vs-own ambiguity gets a clear answer ("if you hold a
reference, you took a retain").

### B.1 — Audit retain/release call sites

For each subsystem, list every place that:
- *creates* a TclObj (refcount = 1 at birth, no retain needed)
- *stores* an existing TclObj into a field/slot it didn't allocate
  (must retain)
- *evicts* an existing TclObj from a field/slot (must release)
- *transfers ownership* across an API boundary (caller releases,
  callee retains, or vice versa)

Subsystems to audit:
- `interp/tcl_frames.zig` — frame locals (the frame_table entries
  hold TclObj pointers; bind/unbind of locals must retain/release)
- `interp/tcl_procs.zig` — proc table (`OFF_PARAMS_OBJ` /
  `OFF_BODY_OBJ` slots; proc registration must retain, proc
  unregistration must release)
- `interp/tcl_ns.zig` — namespace var table (`var_set_scalar`
  stores into a slot — must release the prior occupant, retain
  the new value)
- `interp/tcl_interp.zig` — eval loop (parser-produced word
  TclObjs need refcount management across dispatch, especially
  for words that get stored elsewhere via `set` / `proc` /
  `lappend` / etc.)
- `dispatch/tcl_dispatch.zig` — host-bridge call paths (caller
  passes a TclObj; callee may store; clear ownership rule
  needed)
- `cmds/*.zig` — every command handler that returns or stores a
  TclObj
- `valtypes/tcl_*.zig` — list, dict, string-buffer ops that hold
  internal refs to element TclObjs

Each site gets a one-line comment stating its retain/release
contract.

### B.2 — Make `var_set_scalar` retain/release

The simplest high-leverage fix.  Currently:

```zig
pub fn var_set_scalar(v_addr: u32, obj_handle: u32) void {
    ...
    v.value = obj_handle;     // overwrite; old value leaks
}
```

After:

```zig
pub fn var_set_scalar(v_addr: u32, obj_handle: u32) void {
    ...
    const old = v.value;
    v.value = obj_handle;
    obj.tcl_obj_retain(obj_handle);
    if (old != 0) obj.tcl_obj_release(old);
}
```

Same shape applies to `frame.local_set`, `dict.dict_set`,
`list.list_set_at`, etc.

### B.3 — Make proc parameter binding retain

`eval_proc_call_bucket` at line ~1160:

```zig
_ = frames.local_set(param_name, words[arg_idx]);
```

The frame entry now holds a reference to `words[arg_idx]`.  The
caller of `eval_proc_call_bucket` may release `words[]` after
dispatch; the frame's hold must keep the value alive.  After the
B.2 fix to `local_set`, this becomes correct automatically (the
local_set retains).

### B.4 — Make eval_command release words[] after dispatch

After `eval_command` finishes a command, the parser-produced
`words[]` array goes out of scope.  Each entry must be released
once.  Exception: if the result is one of the words (e.g. `set x
$y` returns `$y`), the *result* keeps a retain via the return
value — release the others, leave the result alone.

**Status:** staged in `execute_parsed_command` (both fast and
slow paths) but currently DISABLED.  See B.6 for the blocker.

### B.5 — Per-store-site retain/release

Done in this commit:
- B.5a: `array_set` / `ar_insert` value slot
- B.5d: `frame_set_argv` + `frame_pop` argv release
- `eval_return` / `eval_proc_call_bucket` `return_val` slot

Not done (and not needed — list/dict store as strings, not
TclObj refs):
- list element setters (`tcl_cmd_list_insert` /
  `tcl_cmd_list_replace` / `tcl_cmd_list_set`) — rebuild the
  list as a fresh string TclObj
- `dict_set` — rebuilds the dict as a fresh string TclObj

### B.6 — Pointer-borrow audit (BLOCKER for B.4)

cmdIL.test still traps with `site=N <binary garbage>` when
B.4 is enabled.  The trap signature: a TclObj's owned buffer
got freed mid-dispatch and the slab was re-issued before the
dispatch finished reading the command name from it.

Concrete scenario observed: bracket-subst ``[namespace
current]`` inside ``namespace which -command [namespace
current] ::CleanupTest``.  The inner eval_script("namespace
current") parses two words, dispatches "namespace", which
returns "::tcltest::test".  Then the outer dispatch continues
with the substituted result — but reading words[0] for the
outer dispatch returns garbage because something released the
outer's already-parsed word.

Audit candidates (each holds a borrowed (ptr, len) into a
TclObj's buffer):
- `proc_table[].name_ptr` / `.name_len` — currently a heap-
  copied buffer in `alloc_command`, not a TclObj ref, so
  appears safe.  Verify.
- `tcl_ns_export_patterns` — each pattern stored as raw
  `(ptr, len)`.  If the pattern came from a TclObj that was
  released, the pattern bytes go stale.
- `tcl_alias_*` descriptor structs — alias source / target
  command names.  Same shape.
- `parse_cache` entries — token offsets into a body whose
  buffer might be freed.  Cache invalidation needed.
- `tcl_diag.current_eval_ptr` — recorded for trap context.
  Set per `eval_script` call but might point to a script
  whose owner is releasing.
- `frame_argv[]` for `info level 0` — fixed in B.5d but
  similar shape for `info level -N` if implemented.

For each, the fix is the same: either retain the source TclObj
for the duration the (ptr, len) is held, or copy the bytes
into a fresh owned buffer at storage time.

Once B.6 is in, B.4 should re-enable cleanly and reg* OOM
disappears.

### B.7 — Track "shared buffer" relationships explicitly (future)

Today an `OBJ_STR_CAP == 0` TclObj points into someone else's
buffer with no link to the lender.  Add a hidden `OBJ_LENDER`
field that stores the lender's TclObj address (or 0 for a
data-segment / static-string borrow).  Borrowing TclObj's
construction takes a retain on the lender; the borrowing
TclObj's `release_now` releases the lender.

Enables things like `obj_new_string_borrow(parent, off, len)` —
parser word creation — to set up the lender link automatically.
Avoids the Phase 1.3 corruption because the lender chain is
explicit and ref-counted.

Optional: this whole story can be skipped if B.6 fully copies
borrowed bytes at every store site, but that's wasteful for
the dispatch fast path.  B.7 is the perf-friendly version.

## Phase MM-C — debug-only leak detection

Compile-time flag (`-Dleak-check=true` or similar).  When set:
- `obj_alloc` increments a global counter.
- `release_now` decrements.
- Reactor entry `tcl_test_finalize` (called by tests after the
  last work item) asserts the counter is zero.
- A non-zero count on exit prints the type-tag distribution of
  the leaked objs ("12 STRING, 3 LIST, 1 DICT") so the offending
  subsystem is obvious.

ReleaseFast builds skip this entirely — zero runtime cost on the
hot path.

## Phase MM-D — performance recovery

After MM-A's libc-malloc switch, per-call alloc cost is ~100 ns.
For workloads where this matters:

- **Reuse via the existing free-lists.**  The size-class arrays
  in tcl_obj.zig are still defined.  Re-enable free-list pushes
  in `free_obj` / `free_sized` for the four most-common size
  classes (32, 48, 64, 96 — the TclObj header + small-string
  cases).  Allocations from those classes pop from the list
  before falling through to malloc.  Each push/pop is two stores
  + one load — much cheaper than malloc.  Bound the lists at 256
  entries each so a regex stress-test doesn't pin too much
  memory in the recycler.

- **Inline-string optimisation.**  TclObjs whose string fits in
  ≤ 23 bytes can store the bytes inline (in the
  `OBJ_INT_CACHE` + `OBJ_STR_PTR` fields) instead of allocating
  a separate buffer.  Halves alloc count for a typical Tcl
  workload (many short identifiers, command names, integer-
  string-conversion results).

- **Per-arena allocators for short-lived buffers.**  Parser
  scratch (token arrays, sub_buf for `subst`), regex
  intermediate state (UTF-8 decode buffers, `pmatch_buf`) — all
  live for one command's worth of time.  An arena that resets
  on `eval_command` boundaries reclaims them in O(1).

These are independent and can land separately; each should come
with a microbench delta in its commit message.

## Phase MM-E — test infrastructure

- **`make leakcheck`** — runs the wasm test suite under MM-C's
  leak-detection mode; CI gate fails on any regression.
- **`scripts/probe_alloc_pattern.py`** — instruments a single
  test bundle to log every alloc/free with type tag + caller
  PC; produces a histogram of leak hotspots when MM-C reports a
  non-zero residual.  Useful for narrowing audits without
  recompiling the runtime.

## Acceptance gates per phase

| Phase | Gate |
|---|---|
| MM-A | 395 wasm tests pass; `regexp.test` no longer hits cross-allocator corruption (may still trap on other reasons but not the dlmalloc-vs-bump signature) |
| MM-B | leak counter (under MM-C) reads 0 after each in-scope tcltest run |
| MM-C | `make leakcheck` is a CI gate |
| MM-D | per-op microbench rows for `set` / `incr` / `expr` recover within 10 % of the pre-MM-A bump-allocator numbers |
| MM-E | leak hotspots show up as actionable per-subsystem entries in CI output |

## Sequencing

```
MM-A (this commit) ────┬─→ MM-B (audit + retain/release fixes)
                       │     │
                       │     ↓
                       └─→ MM-C (leak detection) ←──┘
                                  │
                                  ↓
                              MM-D (perf recovery)
                                  │
                                  ↓
                              MM-E (test infra)
```

MM-A is unblocking — the crashes it eliminates were blocking
reg* and any future regex-heavy tcltest run.  MM-B and MM-C
are correctness-only; they're necessary before we can claim
"no leaks" but the runtime works without them today (everything
just leaks, bounded by tcltest workload size).  MM-D and MM-E
are quality-of-life on top.

## Out of scope

- Tracing GC.  Refcounting is sufficient for Tcl's value model
  (no mutable shared structures with cycles in normal workloads;
  the few cyclic cases — recursive list-of-lists — already
  use explicit `unset` to break cycles).  GC would be a
  much larger architectural change.
- Generation-aware allocator.  TclObjs typically have very
  short lives (one statement) or very long lives (a proc body
  for the lifetime of the proc registration).  A bump arena per
  command boundary (Phase MM-D's third bullet) captures the
  short-lived case without needing generations.

## Risks

- **MM-B audit miss.**  If we forget to retain at one site, the
  refcount drops to zero prematurely and we free a still-
  referenced obj.  Surfaces as use-after-free on the next read
  of the freed obj's bytes.  Mitigation: MM-C's leak detector
  paired with a "double-free counter" that increments when
  `release_now` is called on an already-zero refcount.
- **Performance regression.**  MM-A makes every alloc go through
  libc.  Microbenches will show 2-5× per-op cost.  MM-D
  (free-list reuse + inline strings + arena scratch) should
  recover most of it; if it doesn't, we may revisit the
  allocator routing — perhaps a "regex calls malloc, everything
  else uses bump" hybrid with explicit reservation.
