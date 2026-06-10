# 06 — Hot-spot analysis

Three areas appear repeatedly in the perf and trap data — both
in the small samples and in the new
[in-scope tcltest sweep](08-tcltest-suites.md). Each is traced
back to a specific source location and a fix is sketched.

The tcltest sweep promoted **#1** from "slow at scale" to
"correctness blocker that produces garbled command names" and
added a new sibling under #3 (`frame local table full`).

## Hot-spot #1 — Bump allocator without growth or recycling

**Where:** `runtime/zig/valtypes/tcl_obj.zig:44`

```zig
var heap_ptr: u32 = 65536;
var free_list: u32 = 0;

pub fn alloc(size: u32) callconv(.c) u32 {
    const aligned = (size + 7) & ~@as(u32, 7);
    if (aligned == OBJ_SIZE and free_list != 0) {
        const ptr = free_list;
        free_list = @intCast(read_i32(ptr));
        return ptr;
    }
    const ptr = heap_ptr;
    heap_ptr += aligned;       // <-- never bounded, never `memory.grow`
    return ptr;
}
```

**Symptoms:**

- `out of bounds memory access` once `heap_ptr` crosses the
  current WASM linear-memory size (16 MB by default).
- The `dict set` bench traps at heap_ptr ≈ 0x376b2033 (≈ 928 MB)
  because nothing checks the page count.
- Only 24-byte `OBJ_SIZE` objects are ever recycled — the larger
  string buffers, list backing stores, and dict pair tables leak
  forever.
- **NEW from the tcltest sweep:** the dispatcher receives
  garbage `(name_ptr, name_len)` pairs in 9 in-scope test
  files (`listObj`, `listRep`, `lrange`, `format`, `var`,
  `namespace`, `trace`, `info`, `rename`). Sample garbage:
  `unknown command: 2971669`, `unknown command:
  tConstraintsHookam`, `unknown command: gleFile`. The bytes
  are not random — they are the contents of memory that another
  allocation has since reused. This is not just a perf issue
  any more; it's a **correctness blocker** — the bug surface
  every long bundle (tcltest is hundreds of KB) hits.

**Per-call cost:** small while it succeeds, but this allocator
is on the inner loop of every Tcl op.

**Fix sketch:**

1. Track current page count; call `@wasmMemoryGrow(N)` when
   `heap_ptr + size` would cross it. Trap with a Tcl-friendly
   "out of memory" message if grow returns -1 instead of letting
   the WASM trap surface raw.
2. Add size-class free-lists (e.g. 24, 32, 48, 64, 128, 256,
   then power-of-two up to 1 KB). Most TclObj-shaped allocations
   fall in the first three.
3. Wire `obj_release` (refcount → 0) to push the slab back onto
   its size-class free-list. Today that only fires for the 24-byte
   path.

Expected impact: turns three traps in the microbench into
successful runs and removes the ceiling on real workloads;
allocator throughput stays at bump-allocator speed for the
hot path because the size-class lookup is a switch on a small
constant.

## Hot-spot #2 — `tcl_cmd_append` is O(N) per call

**Where:** `runtime/zig/valtypes/tcl_string.zig:22`

```zig
pub export fn tcl_cmd_append(current: i32, addition: i32) i32 {
    const a = obj_ensure_string(current);
    const b = obj_ensure_string(addition);
    const total = a.len + b.len;
    if (total == 0) return obj_new_string(0, 0);
    const buf = alloc(total);
    if (a.len > 0) memcpy(buf, a.ptr, a.len);
    if (b.len > 0) memcpy(buf + a.len, b.ptr, b.len);
    return obj_new_string(@intCast(buf), @intCast(total));
}
```

**Symptoms:**

- Microbench shows 2,568 ns/op for `append s x; string length $s`
  at N=5 000. tclsh runs the same workload below the noise floor
  of the spawn baseline (≈ 0 ns/op).
- The total cost is quadratic in the string length: 5 000
  iterations grow `s` to 5 000 bytes, copying ≈ 12 MB through
  the bump allocator.
- Combined with hot-spot #1 this is also a steady leak — every
  intermediate string buffer leaks the previous one.

**Fix sketch:**

1. Add a `MutableString` variant (or just a capacity field on the
   existing string layout) — Tcl 9 stores `length`, `bytes`, and
   `available` for exactly this case.
2. When `obj_release` sees an only-reference string getting
   appended to, append in place and double the capacity if
   needed (geometric growth). Same algorithm as
   `Tcl_AppendObjToObj` in `tcl9.0.3/generic/tclStringObj.c`.
3. Falls back to copy-on-write for the shared case — call
   `obj_ensure_string` only when the source object's refcount > 1.

Expected impact: turns the O(N²) loop into O(N) and lifts the
ceiling on text-building workloads (template expansion, `subst`
output, JSON / XML emission).

## Hot-spot #3 — `frame_push` zeros 4 KB per proc call

**Where:** `runtime/zig/interp/tcl_frames.zig:171`

```zig
pub export fn frame_push() i32 {
    if (frame_depth >= MAX_DEPTH) return -1;
    const idx = frame_depth;
    if (frame_stack[idx] == 0) {
        frame_stack[idx] = alloc(FRAME_SIZE);   // 4 096 bytes
    }
    const base = frame_stack[idx];
    const slice: [*]u8 = @ptrFromInt(base);
    @memset(slice[0..FRAME_SIZE], 0);           // <-- 4 KB zero per call
    frame_argv[idx] = 0;
    frame_depth += 1;
    return @intCast(idx);
}
```

`FRAME_BUCKET_SIZE = 16` × `FRAME_BUCKET_COUNT = 256` =
4 096 bytes per frame. Every single proc call clears the whole
table even when the proc has no locals at all.

**Symptoms:**

- Microbench: no-arg proc call costs 153 ns on wasm vs 48 ns on
  tclsh — only place we lose to tclsh on per-op cost.
- Stress workloads with proc-heavy bodies (sample 03,
  compiler_explorer_demo, sample 02 with its inner `for` loop)
  spend a significant fraction of their cycles in `@memset`.
- **NEW from the tcltest sweep:** `set.test`, `incr.test`, and
  `execute.test` trap with `frame local table full` because the
  256-bucket table is fixed-capacity. So we both pay for a 4 KB
  table on every push *and* still overflow it on real workloads.

**Fix sketch:**

1. Drop the default frame size to a single bucket (256 bytes,
   16 entries). Procs above that grow on demand. The microbench
   N-arg cases use ≤ 4 locals; 16 covers most real procs.
2. Add a per-frame `dirty_bitmap` (one bit per bucket) so
   `frame_push` only re-clears buckets that were written by the
   previous occupant of this slot. The bookkeeping is one extra
   write per `local_set`.
3. When the callee is escape-analysed as needing no FRAME vars
   (the codegen already tracks this — see
   `_core.py:438-467`), elide the entire push/pop sequence —
   today we still push when interp-fallback path is reachable
   even though the body never reads its locals.

Expected impact: brings no-arg proc call to ≈ 50 ns on wasm —
parity with tclsh — and shaves real time off proc-heavy
workloads where the relative speed-up will be 1.5 – 2× given
proc-call is the dominant op in that profile.

## Honourable mentions

- **`subst_flagged` re-tokenises every word fragment** —
  `parse/tcl_subst.zig`. Cached parse trees would help but only
  on warm-loop bodies; the precompiled wasm path bypasses subst
  entirely for the common case.
- **Hash-table linear-scan in `proc_lookup`** —
  `interp/tcl_procs.zig`. Acceptable today because user proc
  count stays small (< 64 in every sample), but tcltest bundles
  register hundreds of helper procs and the tcltest sweep
  numbers (97 files at 300–800 ms compile + 60–660 ms run)
  suggest this is starting to bite.
- **Diag map lookups** — `dispatch/tcl_diag.zig`. Currently a
  linear walk per trap; only matters under error — but the
  tcltest sweep traps a LOT.

## New hot-spots surfaced by the tcltest sweep

### `tcltest::cleanupTests` walks the command table and trips

5 files (`parse`, `subst`, `for`, `foreach`, `parseExpr`) run
their tests fine and then trap inside `preserveCore` during
the standard cleanup post-amble. The cleanup helper iterates
the master command table to detect any test that registered a
stray command. The trap means an `info commands` /
`info procs` filter combination it relies on returns
something we don't expect (or crashes inside `info`).

This isn't a hot **spot** for cycles, but it is a hot spot for
"how many `run-trap` files will become `partial` if it gets
fixed" — five suites in one go.

### Bundled `tcltest` constraint dispatch

2 files (`parseExpr`, `dict`) trap with
`ConstraintInitializer must be complete script` while loading
the bundled tcltest. Likely an `eval` arity mismatch when
dispatching a constraint script body — same area as the
sample 5 unbraced-expression bug, but in a different code
path.

### `regexp` option parser

3 files (`lseq`, `lrepeat`, `reg`) trap with
`unsupported or unknown option` from `regexp`. The Spencer
engine itself is vendored from Tcl 9, but the option parser
in `cmds/regexp.zig` doesn't recognise the full set of
`-line`, `-indices`, `-command`, etc. flags Tcl 9 ships with.

Source pointers above are line-accurate as of the runtime
checked in on `claude/tcl-wasm-performance-profile-QP0yH`.
