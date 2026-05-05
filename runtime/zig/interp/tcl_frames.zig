// Call frame stack — local variable scoping for both compiled and interpreted Tcl.
//
// Provides push/pop frame semantics with per-frame local variable hash tables.
// Used by:
//   - The interpreter (always): push frame on proc call, pop on return
//   - Compiled WASM procs (when needed): push frame if proc uses info exists,
//     upvar, uplevel, or is callable from interpreted code
//   - AOT-only mode: compiled procs that don't need introspection skip frames
//     entirely and use WASM locals directly (fast path)
//
// Design: fixed-depth stack of frames. Each frame has a small open-addressing
// hash table for local variables. Frames are recycled (not freed) to avoid
// allocation churn.
//
// Variable aliasing
// -----------------
// Each frame bucket stores one of:
//   value >= 0          : TclObj pointer (0 = unset/null)
//   value == -1         : ALIAS_GLOBAL — same-name global alias (``global x``)
//   value < -1          : ALIAS_EXT   — heap-allocated 12-byte descriptor at
//                         address (-value).  Descriptor layout:
//                           [0..3]  kind  (i32): 0=global_named, 1=frame_var
//                           [4..7]  param (i32): frame_var → abs frame depth
//                           [8..11] target_name (i32): TclObj* for target name
//                         heap_ptr starts at 65536 so (-heap_addr) <= -65536 < -1,
//                         never colliding with ALIAS_GLOBAL (-1).

const obj = @import("../valtypes/tcl_obj.zig");
const alloc = obj.alloc;
const memcpy = obj.memcpy;
const read_i32 = obj.read_i32;
const write_i32 = obj.write_i32;
const obj_ensure_string = obj.obj_ensure_string;
const obj_new_int = obj.obj_new_int;
const obj_get_int = obj.obj_get_int;

const tcl_ns = @import("tcl_ns.zig");
// ``globals`` here is a name-only alias for tcl_ns — the four
// globals exports moved into tcl_ns in P3.4.  Aliasing keeps the
// existing call sites readable (``globals.global_set`` reads as
// "go to global storage", which is more obvious than
// ``tcl_ns.global_set`` would be).
const globals = tcl_ns;
const ht = @import("../valtypes/hash_table.zig");
const fnv1a = ht.fnv1a;

// -- Frame layout --
// Each frame is a contiguous block in linear memory used as a fixed-
// capacity ``hash_table.Table(16)`` — 12-byte header + 4-byte i32
// value (TclObj handle, ALIAS_GLOBAL sentinel, or ALIAS_EXT desc).
//   [64 buckets * 16 bytes] = 1024 bytes per frame
// We don't grow on overflow; we trap, so the per-frame view is
// constructed read-only over the pre-allocated buffer (no init / no
// grow path), and inserts go through ``try_insert_header``.

const FRAME_BUCKET_SIZE: u32 = 16;
// Per-frame hash table capacity.  Phase 2.1: tables are growable,
// starting at FRAME_INITIAL_BUCKETS (256 B per frame — fits a typical
// proc with ≤ 8 locals) and doubling on probe-chain exhaustion via
// :func:`frame_grow_at_base`, capped at FRAME_MAX_BUCKETS.
//
// Empirical lower bound: tcltest's ``test`` proc + every option
// variable read accumulates > 256 distinct names — bumping the
// ceiling to 1024 buckets (16 KB per frame) keeps that path
// inside the grow path rather than the trap path.  The dirty
// bitmap below uses u64 to track 64 chunks at 256 bytes each
// (16 KB total), so the cap and the bitmap stay in sync.  Most
// procs never grow past the initial 16.
const FRAME_INITIAL_BUCKETS: u32 = 16;
const FRAME_MAX_BUCKETS: u32 = 1024;
const FRAME_BUFFER_MAX: u32 = FRAME_MAX_BUCKETS * FRAME_BUCKET_SIZE; // 16384 bytes
const OFF_VALUE: u32 = ht.HEADER_SIZE; // 12 — value follows header

const FrameTable = ht.Table(FRAME_BUCKET_SIZE);

/// Construct a transient view of the per-frame buffer as a fixed-
/// capacity hash table.  Frames are pre-allocated in ``frame_push``
/// (not ``Table.init``-allocated), so the view's ``count`` field is
/// not authoritative — we never call ``grow`` / ``needs_grow`` on
/// these tables; growth is driven by :func:`frame_grow_at_base`
/// instead and the per-frame capacity lives in ``frame_capacity``.
inline fn frame_table(base: u32) FrameTable {
    return .{ .buf = base, .cap = capacity_for_base(base), .count = 0 };
}

/// Look up a frame's current bucket capacity by base address.
/// O(MAX_DEPTH) linear scan over ``frame_stack`` — cheap (≤64
/// entries) and avoids threading an extra ``cap`` parameter through
/// every internal helper.  Falls back to ``FRAME_INITIAL_BUCKETS``
/// if the base isn't found in the live stack (defensive — keeps
/// the table view valid for transitional callers).
inline fn capacity_for_base(base: u32) u32 {
    var i: u32 = 0;
    while (i < MAX_DEPTH) : (i += 1) {
        if (frame_stack[i] == base) return frame_capacity[i];
    }
    return FRAME_INITIAL_BUCKETS;
}

/// Sentinel: alias to a global variable with the same local name.
const ALIAS_GLOBAL: i32 = -1;
/// Descriptor-based alias: value = -(heap_ptr).  See module comment.
/// Any value < -1 is an ALIAS_EXT (heap_ptr >= 65536 => value <= -65536).
const ALIAS_EXT_THRESHOLD: i32 = -2;

inline fn is_alias_ext(v: i32) bool {
    return v < ALIAS_EXT_THRESHOLD or v == ALIAS_EXT_THRESHOLD;
}

/// Recover the descriptor heap address from an ALIAS_EXT bucket value.
inline fn alias_desc_ptr(v: i32) u32 {
    return @bitCast(-v);
}

// -- Frame-alias descriptor kinds --
const KIND_GLOBAL_NAMED: i32 = 0; // target_name is global var name
const KIND_FRAME_VAR: i32 = 1; // param = abs frame depth, target_name = var
const KIND_NS_VAR_PTR: i32 = 2; // descriptor.target = absolute *Var address (P3.3)

const MAX_DEPTH: u32 = 64;

// Frame stack — array of frame buffer pointers
var frame_stack: [MAX_DEPTH]u32 = [_]u32{0} ** MAX_DEPTH;
// Per-frame bucket capacity — Phase 2.1 of the master plan.  Slots
// start at ``FRAME_INITIAL_BUCKETS`` and double on probe-chain
// exhaustion.  Slots that have never been allocated stay 0;
// ``frame_push`` initialises this to ``FRAME_INITIAL_BUCKETS`` on
// first use and ``frame_grow_at_base`` updates it on growth.
var frame_capacity: [MAX_DEPTH]u32 = [_]u32{0} ** MAX_DEPTH;
pub var frame_depth: u32 = 0;

// Per-frame namespace context.  Set by the proc-call dispatcher
// (see :file:`tcl_interp.zig` immediately after each ``frame_push``)
// so ``uplevel`` can restore the caller's namespace alongside the
// frame depth.  Without this slot ``uplevel 1 $body`` runs the body
// at the right frame depth but with the *callee's* namespace still
// active — so unqualified array lookups (``$path(test1)`` inside a
// tcltest body uplevel'd from ``::tcltest::RunTest``) miss the
// outer-namespace array and silently return empty.
pub var frame_ns: [MAX_DEPTH]u32 = [_]u32{0} ** MAX_DEPTH;

// Per-frame "dirty" bitmap.  Each bit covers a 128-byte chunk (8
// buckets × 16 bytes) of the frame buffer.  ``frame_push`` only
// clears chunks whose bit is set, then resets the bitmap.  Writers
// (``frame_insert`` / ``frame_overwrite``) OR in the bit covering
// the bucket they wrote.
//
// A no-arg / few-locals proc dirties one or two chunks per call,
// so push cost drops from a 4 KB ``@memset`` to 128–256 bytes —
// the dominant fix for the no-arg-proc-call cost.
//
// Phase 2.1: frames are growable up to ``FRAME_MAX_BUCKETS``
// (16 KB = 64 chunks at 256 bytes each), so a single u64 covers
// the entire max-sized buffer.  Growth past that ceiling would
// need a wider mask — most procs never get close.
const FRAME_CHUNK_BYTES: u32 = 256;
const FRAME_CHUNKS_MAX: u32 = FRAME_BUFFER_MAX / FRAME_CHUNK_BYTES; // 64 with 16 KB buffer
var frame_dirty: [MAX_DEPTH]u64 = [_]u64{0} ** MAX_DEPTH;

/// Mark the chunk containing ``bucket_offset`` (a byte offset within
/// the frame buffer, NOT an absolute address) as dirty so the next
/// ``frame_push`` for this slot clears it.
pub inline fn mark_dirty(idx: u32, bucket_offset: u32) void {
    const chunk = bucket_offset / FRAME_CHUNK_BYTES;
    if (chunk < FRAME_CHUNKS_MAX) {
        frame_dirty[idx] |= @as(u64, 1) << @intCast(chunk);
    }
}

// Per-frame invocation argv — the list of words (command name +
// arguments) that entered each frame.  Set by callers immediately
// after ``frame_push`` — both the interpreter's
// ``eval_proc_call_bucket`` (for interpreted procs) and the
// compiled-proc prologue (via ``frame_set_argv`` emitted by the
// WASM codegen) — so ``info level 0`` / ``info level N`` can
// return the real invocation list instead of a placeholder.
//
// Each slot holds an i32 TclObj handle (the list) or 0 when the
// caller didn't populate it (legacy compiled procs without the
// prologue instrumentation, top-level frame, etc.).  The array is
// not zeroed on ``frame_pop`` — next ``frame_push`` reuses the
// slot and ``frame_set_argv`` overwrites it.  Readers check
// against 0 and fall back to an empty list.
var frame_argv: [MAX_DEPTH]i32 = [_]i32{0} ** MAX_DEPTH;

// Phase 8: per-frame call-site metadata for ``info frame``.
//
// Real Tcl's ``info frame`` returns a dict of (type, line, cmd,
// proc, source, level) describing the frame's invocation site.
// Phase 3 reserved a ``cmd_source`` slot for this; phase 8 wires
// the rest of the fields.
//
// ``type`` is one of ``proc`` / ``source`` / ``eval`` / ``uplevel``
// / ``alias`` / ``unknown``; encoded as a u8 with the table below.
// Most frames are ``proc`` (a Tcl proc body); ``source`` marks a
// top-level script eval (``source FILE``); ``eval`` marks generic
// nested eval contexts; ``uplevel`` / ``alias`` mark inter-frame
// trampolines.
//
// Memory: each FrameInfo is 24 bytes (1 type byte + 3 padding +
// 4×i32 + 1×u32) × MAX_DEPTH = 6 KB; cheap.  Slots are zeroed on
// ``frame_pop`` so a stale stamp from a previous tenant doesn't
// leak into a frame that didn't populate the slot.
pub const FrameType = enum(u8) {
    UNKNOWN = 0,
    PROC = 1,
    SOURCE = 2,
    EVAL = 3,
    UPLEVEL = 4,
    ALIAS = 5,
};

pub const FrameInfo = struct {
    /// Type of invocation that pushed this frame.
    type: FrameType = .UNKNOWN,
    /// TclObj handle of the script being evaluated (the body for
    /// type=proc; the file/script for type=source/eval).  Retained
    /// for the frame's lifetime; released on ``frame_pop``.
    script_obj: i32 = 0,
    /// 1-based line number within ``script_obj`` where the
    /// invocation appears.  0 = unknown.
    line: u32 = 0,
    /// TclObj handle for the source slice of the call site.
    /// Retained; released on ``frame_pop``.
    cmd_text: i32 = 0,
    /// TclObj handle of the proc's fully-qualified name (only
    /// meaningful for type=proc).  Retained; released on
    /// ``frame_pop``.
    proc_name: i32 = 0,
    /// Phase 8 follow-up: 1 = the codegen owns the ``line`` field
    /// for this frame (i.e. the compiled-proc prologue is going to
    /// stamp ``frame_set_line`` per-statement).  ``eval_script``
    /// suppresses its own line stamping when this flag is set so
    /// nested command-substitution / ``[…]`` evaluations don't
    /// clobber the outer compiled stamp.  0 = the interpreter's
    /// :func:`eval_script` is the line authority for the frame.
    line_owned_by_codegen: u8 = 0,
};

var frame_info: [MAX_DEPTH]FrameInfo = [_]FrameInfo{.{}} ** MAX_DEPTH;

// Phase 6 follow-up: per-frame variable-trace lists.  Each slot holds
// the head pointer of a singly-linked chain of frame-local trace
// records (records are allocated in :file:`tcl_var_trace.zig`).  A
// frame with no traces installed has ``frame_trace_heads[idx] == 0``
// — the read / write hooks short-circuit on that case so untraced
// procs pay nothing per-access.  ``frame_pop`` calls ``drain_list``
// to fire UNSET callbacks for every record, release the chain, and
// reset the slot to 0 before the frame is reused.
pub var frame_trace_heads: [MAX_DEPTH]u32 = [_]u32{0} ** MAX_DEPTH;

// Pending ``argv0`` for the next compiled-proc entry.  Set by a
// compiled caller via ``frame_set_pending_argv0`` immediately
// before it transfers control to a compiled callee, and consumed
// (cleared) by the callee's prologue via
// ``frame_take_pending_argv0``.  Holds the *exact word the caller
// wrote* for the invocation — imported names, renamed entry points,
// and qualified forms (``::foo::bar`` / ``foo::bar``) all show up
// here intact, so ``info level 0`` can report the caller's invoked
// word rather than the proc's registered tail.
//
// Entries reached via the host bridge (``tcl_eval`` →
// ``eval_proc_call_bucket`` → ``call_compiled_proc``) don't touch
// this slot; those callers build the full argv list directly with
// ``build_invocation_list`` + ``frame_set_argv``, so the prologue's
// ``frame_take_pending_argv0`` returns 0 and the prologue falls
// back to the qname tail baked in at compile time.  That fallback
// is also why every entry point — ``::top``, methods, procs
// invoked from C — keeps working without the call-site ABI hook.
var pending_argv0: i32 = 0;

/// Record the invoked word a compiled caller is about to use for
/// a compiled callee.  Must be immediately followed by the callee
/// call — no other compiled-proc invocation may happen in between
/// or the pending slot will be consumed by the wrong callee.
pub export fn frame_set_pending_argv0(argv0: i32) void {
    pending_argv0 = argv0;
}

/// Consume and clear the pending argv0.  Returns 0 when no caller
/// recorded a word (e.g. host-bridge dispatch, ``::top`` entry, or
/// a compiled call emitted by an older codegen without the ABI
/// hook); the prologue falls back to its qname-derived tail in
/// that case.
pub export fn frame_take_pending_argv0() i32 {
    const v = pending_argv0;
    pending_argv0 = 0;
    return v;
}


// -- Frame operations --

/// Push a new call frame. Returns the frame index.
pub export fn frame_push() i32 {
    if (frame_depth >= MAX_DEPTH) return -1; // stack overflow
    const idx = frame_depth;
    var first_use = false;
    if (frame_stack[idx] == 0) {
        // Allocate frame on first use at the initial capacity.
        // Growth (Phase 2.1) is driven by ``frame_grow_at_base``
        // when an insert exhausts the probe chain.
        frame_capacity[idx] = FRAME_INITIAL_BUCKETS;
        frame_stack[idx] = alloc(FRAME_INITIAL_BUCKETS * FRAME_BUCKET_SIZE);
        first_use = true;
    }
    const base = frame_stack[idx];
    const cap = frame_capacity[idx];
    const buffer_bytes = cap * FRAME_BUCKET_SIZE;
    if (first_use) {
        // Brand-new buffer from the bump allocator — Zig zero-fills
        // pages on grow but a recycled OBJ_SIZE slab from the size-
        // class free-list won't be (and our frame size doesn't
        // currently come from a free-list, but be defensive).
        const slice: [*]u8 = @ptrFromInt(base);
        @memset(slice[0..buffer_bytes], 0);
    } else {
        // Selective clear via the dirty bitmap.  The bitmap is from
        // the previous push that recycled this slot, so each set bit
        // marks a 128-byte chunk that needs zeroing.  Chunks that
        // were never written stay zero from the previous clear (or
        // from the first-use ``@memset`` above), so we don't have
        // to touch them.
        var dirty = frame_dirty[idx];
        var chunk: u32 = 0;
        while (dirty != 0) : (chunk += 1) {
            if ((dirty & 1) != 0) {
                const off = chunk * FRAME_CHUNK_BYTES;
                if (off < buffer_bytes) {
                    const remaining = buffer_bytes - off;
                    const span = if (remaining < FRAME_CHUNK_BYTES) remaining else FRAME_CHUNK_BYTES;
                    const slice: [*]u8 = @ptrFromInt(base + off);
                    @memset(slice[0..span], 0);
                }
            }
            dirty >>= 1;
        }
    }
    frame_dirty[idx] = 0;
    // Clear any stale argv from the previous occupant of this
    // slot so ``info level 0`` after a push without a subsequent
    // ``frame_set_argv`` reads 0, not an outdated list.
    frame_argv[idx] = 0;
    frame_depth += 1;
    return @intCast(idx);
}

/// Grow the frame at *base* to double its current bucket capacity
/// (capped at ``FRAME_MAX_BUCKETS``).  Allocates a new buffer,
/// rehashes every populated bucket from the old buffer into the
/// new one, swaps the pointer in ``frame_stack``, and frees the
/// old buffer.  Returns the new base address, or 0 if the frame
/// is already at the max capacity (caller traps).
///
/// Pointer invariant: callers reach the frame via ``frame_stack``
/// indices (or via a same-frame ``current_frame`` lookup), not by
/// caching raw bucket addresses across calls.  After the swap,
/// ``frame_stack[idx]`` is the new base; any previously cached
/// bucket pointer into the old buffer is stale and must be re-
/// resolved through ``frame_find``.
fn frame_grow_at_base(base: u32) u32 {
    var idx: u32 = 0;
    while (idx < MAX_DEPTH) : (idx += 1) {
        if (frame_stack[idx] == base) break;
    }
    if (idx == MAX_DEPTH) return 0;

    const old_cap = frame_capacity[idx];
    if (old_cap >= FRAME_MAX_BUCKETS) return 0;
    const new_cap = old_cap * 2;
    const new_buffer_bytes = new_cap * FRAME_BUCKET_SIZE;
    const new_base = alloc(new_buffer_bytes);
    if (new_base == 0) return 0;
    const new_slice: [*]u8 = @ptrFromInt(new_base);
    @memset(new_slice[0..new_buffer_bytes], 0);

    // Rehash every populated bucket from the old buffer into the
    // new (larger) buffer.  Use the new table's ``try_insert_header``
    // — its probe chain is bounded by the new capacity, which is
    // strictly larger than the old, so the only way insertion can
    // fail is an OOM on the per-key heap copy ``try_insert_header``
    // makes.  Silently skipping a null return would drop a live
    // frame-local during growth (state corruption); trap loudly so
    // the partially-populated new buffer never replaces the old.
    var new_table: FrameTable = .{ .buf = new_base, .cap = new_cap, .count = 0 };
    // Bucket header layout (see hash_table.zig:63):
    //   [0..3]   name_ptr  (0 or TOMBSTONE means skip)
    //   [4..7]   name_len
    //   [8..11]  hash
    //   [12..]   value
    var i: u32 = 0;
    while (i < old_cap) : (i += 1) {
        const old_bucket = base + i * FRAME_BUCKET_SIZE;
        const name_ptr: u32 = @bitCast(read_i32(old_bucket));
        if (name_ptr == 0 or name_ptr == ht.TOMBSTONE) continue;
        const name_len: u32 = @bitCast(read_i32(old_bucket + 4));
        if (name_len == 0) continue;
        const hash: u32 = @bitCast(read_i32(old_bucket + 8));
        const value = read_i32(old_bucket + OFF_VALUE);
        if (new_table.try_insert_header(name_ptr, name_len, hash)) |new_bucket| {
            write_i32(new_bucket + OFF_VALUE, value);
        } else {
            const errmsg = "frame_grow: OOM during rehash";
            rt_fd_write_stderr(errmsg);
            @trap();
        }
    }

    // Swap.  The dirty bitmap from the old buffer is now stale
    // (different chunk → bucket mapping); reset it — every
    // populated bucket lives in the new buffer's chunk(s) which
    // were just zeroed, so the next ``frame_push`` for this slot
    // doesn't need to clear anything until ``frame_insert``
    // re-marks chunks.
    obj.free_sized(base, old_cap * FRAME_BUCKET_SIZE);
    frame_stack[idx] = new_base;
    frame_capacity[idx] = new_cap;
    frame_dirty[idx] = 0;
    // Re-mark every populated bucket as dirty so the NEXT push of
    // this slot clears them.  Without this, recycled-slot reads
    // could see stale entries from a previous occupant.
    var j: u32 = 0;
    while (j < new_cap) : (j += 1) {
        const new_bucket = new_base + j * FRAME_BUCKET_SIZE;
        const np: u32 = @bitCast(read_i32(new_bucket));
        if (np != 0) mark_dirty(idx, j * FRAME_BUCKET_SIZE);
    }
    return new_base;
}

/// Pop the current call frame. Caller should not access frame locals after this.
pub export fn frame_pop() void {
    if (frame_depth > 0) {
        // Phase 6 follow-up: fire UNSET callbacks for every
        // frame-local trace, release the chain, and clear the head
        // slot.  Drains BEFORE the array directory cleanup so a
        // trace body's read-back of ``$arr(key)`` still sees the
        // pre-pop state.
        const tcl_var_trace = @import("tcl_var_trace.zig");
        tcl_var_trace.drain_list(&frame_trace_heads[frame_depth - 1]);
        // Phase 1: drop every ``::__local::<depth>::*`` array that
        // belonged to this frame from the global directory.  Without
        // this the next call at the same depth would inherit the
        // returning frame's array elements and ``info exists arr``
        // / ``array names arr`` would lie.
        const tcl_array = @import("../valtypes/tcl_array.zig");
        tcl_array.drop_local_arrays_for_depth(frame_depth);
        // Phase 7: release every compile-time-indexed local slot
        // for this frame.  The hash-keyed store has its own
        // reset-on-reuse via ``mark_dirty``; the indexed array
        // is a parallel store with its own ownership tracking.
        frame_locals_array_drop_current();
        frame_depth -= 1;
        // MM-B.5: release the argv reference we retained in
        // frame_set_argv before clearing the slot.
        const old = frame_argv[frame_depth];
        frame_argv[frame_depth] = 0;
        if (old != 0) obj.tcl_obj_release(old);
        // Phase 8: release the FrameInfo's retained TclObj handles
        // so the frame's call-site metadata doesn't leak.  Reset
        // the slot to default so the next tenant starts clean.
        const fi = &frame_info[frame_depth];
        if (fi.script_obj != 0) obj.tcl_obj_release(fi.script_obj);
        if (fi.cmd_text != 0) obj.tcl_obj_release(fi.cmd_text);
        if (fi.proc_name != 0) obj.tcl_obj_release(fi.proc_name);
        fi.* = .{};
        // Reset the per-frame namespace slot so the next
        // ``frame_push`` for this index sees a fresh (zero) value
        // and ``frame_set_ns_if_unset`` from ``ns_set`` records
        // the caller-ns of the new frame, not the previous
        // tenant's.
        frame_ns[frame_depth] = 0;
        // Note: we do NOT walk the local_table to release each
        // bucket's value here.  Doing so leaks-by-design rather
        // than risking a use-after-free where the bucket contains
        // a tombstone or alias-encoded value our scan
        // misclassifies.  Frame buckets are reused on the next
        // ``frame_push`` (selective-zero via the dirty bitmap),
        // so the leaked values stay reachable from the (zero-on-
        // reuse) bucket until the next ``local_set`` for that
        // bucket releases them via the in-place overwrite path
        // in ``local_set``.  Net effect: locals leak across the
        // pop boundary but are reclaimed when their slot is
        // reused, bounded by the proc's local-count.
    }
}

/// Get current frame depth (0 = global scope, no frames pushed).
pub export fn frame_get_depth() i32 {
    return @intCast(frame_depth);
}

/// Record the invocation argv for the current (topmost) frame.
/// Callers should invoke this immediately after ``frame_push``
/// with a list TclObj whose elements are the proc name followed
/// by the call arguments.  Zero is the "no argv recorded"
/// sentinel — equivalent to never calling this function.
pub export fn frame_set_argv(argv: i32) void {
    if (frame_depth == 0) return;
    // MM-B.5: the frame_argv slot owns a reference for the lifetime
    // of the frame.  Retain new, release old.  Without this the
    // argv list (built per-call from words[]) gets freed by the
    // parser-side release at end-of-statement (MM-B.4) before the
    // proc body's ``info level 0`` can read it.
    const old = frame_argv[frame_depth - 1];
    if (argv != 0) obj.tcl_obj_retain(argv);
    frame_argv[frame_depth - 1] = argv;
    if (old != 0 and old != argv) obj.tcl_obj_release(old);
}

// --- Phase 8: FrameInfo setters / getter -------------------------------
//
// Callers set fields immediately after ``frame_push``, like
// ``frame_set_argv``.  Each setter retains the new TclObj and
// releases the previous slot (which may have a stale value if the
// same dispatch site populates the field twice; in practice the
// dispatcher only sets each field once per push).

/// Stamp the FrameType for the current (topmost) frame.  Callers:
///   * ``eval_proc_call_bucket`` / ``eval_apply`` → ``.PROC``
///   * ``eval_uplevel`` → ``.UPLEVEL``
///   * ``dispatch_alias`` → ``.ALIAS``
///   * ``source FILE`` → ``.SOURCE``
///   * generic eval / namespace eval → ``.EVAL``
pub fn frame_set_type(t: FrameType) void {
    if (frame_depth == 0) return;
    frame_info[frame_depth - 1].type = t;
}

/// WASM-ABI shim for compiled-proc prologues — the codegen passes
/// the type as an i32 (1=PROC, 2=SOURCE, 3=EVAL, 4=UPLEVEL,
/// 5=ALIAS, anything else=UNKNOWN).  Internal Zig callers use the
/// typed :func:`frame_set_type` directly.
pub export fn frame_set_type_i32(t_int: i32) void {
    const t: FrameType = switch (t_int) {
        1 => .PROC,
        2 => .SOURCE,
        3 => .EVAL,
        4 => .UPLEVEL,
        5 => .ALIAS,
        else => .UNKNOWN,
    };
    frame_set_type(t);
}

/// Record the script TclObj being evaluated by the current frame
/// (for ``info frame N`` to surface as the ``-script`` field).
pub export fn frame_set_script(script_obj: i32) void {
    if (frame_depth == 0) return;
    const fi = &frame_info[frame_depth - 1];
    const old = fi.script_obj;
    if (script_obj != 0) obj.tcl_obj_retain(script_obj);
    fi.script_obj = script_obj;
    if (old != 0 and old != script_obj) obj.tcl_obj_release(old);
}

/// Record the 1-based line within ``script_obj`` where the
/// invocation appears.  0 = unknown.
/// Read the line number recorded for the frame *offset* steps below
/// the top (0 = current).  Returns 0 when the offset is out of range
/// or no line was stamped.  Phase 8 follow-up — used by
/// ``eval_script`` to save/restore the caller's line across an inner
/// command-subst body's execution.
pub export fn frame_get_line(offset: i32) u32 {
    if (offset < 0) return 0;
    const u: u32 = @intCast(offset);
    if (u >= frame_depth) return 0;
    return frame_info[frame_depth - 1 - u].line;
}

/// Phase 8 follow-up: claim line-stamping ownership for the current
/// frame on behalf of compiled-proc codegen.  After this call,
/// :func:`eval_script` suppresses its own per-command line updates
/// for any nested invocation under this frame — the prologue's
/// ``frame_set_line`` emissions are the single source of truth for
/// ``info frame -line`` inside compiled procs.  Idempotent.
pub export fn frame_claim_line_codegen() void {
    if (frame_depth == 0) return;
    frame_info[frame_depth - 1].line_owned_by_codegen = 1;
}

/// True when the current frame's line is owned by the compiled-proc
/// prologue.  Read by :func:`eval_script` to decide whether to
/// stamp.  Returns false for top-level / interpreted-proc frames.
pub fn frame_line_owned_by_codegen() bool {
    if (frame_depth == 0) return false;
    return frame_info[frame_depth - 1].line_owned_by_codegen != 0;
}

pub export fn frame_set_line(line: u32) void {
    if (frame_depth == 0) return;
    frame_info[frame_depth - 1].line = line;
}

/// Record the source-slice TclObj for the call site (the literal
/// command text, e.g. ``my-proc arg1 arg2``).  ``info frame N``
/// surfaces this as the ``-cmd`` field.
pub export fn frame_set_cmd_text(cmd_text: i32) void {
    if (frame_depth == 0) return;
    const fi = &frame_info[frame_depth - 1];
    const old = fi.cmd_text;
    if (cmd_text != 0) obj.tcl_obj_retain(cmd_text);
    fi.cmd_text = cmd_text;
    if (old != 0 and old != cmd_text) obj.tcl_obj_release(old);
}

/// Record the proc's fully-qualified name TclObj for type=PROC
/// frames.  ``info frame N`` surfaces this as the ``-proc`` field.
pub export fn frame_set_proc_name(proc_name: i32) void {
    if (frame_depth == 0) return;
    const fi = &frame_info[frame_depth - 1];
    const old = fi.proc_name;
    if (proc_name != 0) obj.tcl_obj_retain(proc_name);
    fi.proc_name = proc_name;
    if (old != 0 and old != proc_name) obj.tcl_obj_release(old);
}

/// Read the FrameInfo for an absolute frame depth.  Returns null
/// when the depth is out of range (0 or > current frame_depth).
/// ``info frame N`` from Tcl uses 1-based depth, so the caller
/// translates: ``info frame 1`` = the outermost frame's info, which
/// is ``frame_get_info(1)``; ``info frame N`` = innermost is
/// ``frame_get_info(frame_depth)``.
pub fn frame_get_info(abs_depth: u32) ?*const FrameInfo {
    if (abs_depth == 0 or abs_depth > frame_depth) return null;
    return &frame_info[abs_depth - 1];
}

// --- Frame contexts (used by the coroutine driver) ---------------------
//
// :file:`sched/tcl_coro.zig` save-transfers each coro's frame stack
// into ``Coro.ctx`` on yield and restores it on the next resume.
// The transfer-ownership variants (:func:`frame_context_save_transfer`
// / :func:`frame_context_restore_transfer`) move the live retains
// into the snapshot and zero the live slots, so a paired
// save/restore stays refcount-balanced without any explicit
// retain/release: at most one of "snapshot" or "live" owns a given
// TclObj at any moment.
//
// via ``wasm-opt --asyncify`` and doesn't use ``FrameContext`` —
// asyncify's stack-save covers WASM locals; the runtime side-state
// stays shared with the caller.  Coroutines under that driver
// observe the caller's frame stack rather than an isolated one.
//
// Shape: a ``FrameContext`` is an opaque snapshot of the
// per-frame slot arrays — ``frame_stack`` / ``frame_capacity`` /
// ``frame_ns`` / ``frame_argv`` / ``frame_info`` /
// ``frame_depth``.  Storage cost: ~14 KB at MAX_DEPTH=64 — modest
// given coroutines are short-lived and few in number.  An
// alternative "rebase-pointer" design (each context owns a
// SLICE of a shared backing buffer) would shrink the per-context
// cost but add complexity around the dirty bitmap / capacity
// tracking; the flat copy is the right trade-off until benchmarks
// say otherwise.
pub const FrameContext = struct {
    depth: u32,
    stack: [MAX_DEPTH]u32,
    capacity: [MAX_DEPTH]u32,
    ns: [MAX_DEPTH]u32,
    argv: [MAX_DEPTH]i32,
    info: [MAX_DEPTH]FrameInfo,
};

/// Snapshot the current frame state into a fresh ``FrameContext``.
/// Caller owns the snapshot; the live state is unchanged.
pub fn frame_context_save() FrameContext {
    return .{
        .depth = frame_depth,
        .stack = frame_stack,
        .capacity = frame_capacity,
        .ns = frame_ns,
        .argv = frame_argv,
        .info = frame_info,
    };
}

/// Phase 10: transfer-ownership variant of :func:`frame_context_save`.
/// The snapshot inherits the per-frame TclObj retains the live slots
/// were holding (``frame_argv`` plus ``frame_info``'s ``script_obj`` /
/// ``cmd_text`` / ``proc_name``); the live slots are zeroed so a
/// subsequent ``frame_pop`` or restore-into-different-state can't
/// double-release.  Used by the coroutine driver: a yielding coro
/// snapshots its frame stack with this primitive, then restores the
/// caller's previously-snapshotted context — both sides stay
/// refcount-balanced because the snapshots are the only place a
/// given retain lives at a time.
pub fn frame_context_save_transfer() FrameContext {
    const ctx: FrameContext = .{
        .depth = frame_depth,
        .stack = frame_stack,
        .capacity = frame_capacity,
        .ns = frame_ns,
        .argv = frame_argv,
        .info = frame_info,
    };
    // Live slots: the snapshot owns the retains now.  Zero so the
    // next ``frame_push`` for these slots starts clean and the
    // dirty bitmap (already zeroed in the snapshot) gets re-armed
    // when needed.
    var i: u32 = 0;
    while (i < MAX_DEPTH) : (i += 1) {
        frame_argv[i] = 0;
        frame_info[i] = .{};
        frame_ns[i] = 0;
        frame_stack[i] = 0;
        frame_capacity[i] = 0;
        frame_dirty[i] = 0;
        frame_trace_heads[i] = 0;
    }
    frame_depth = 0;
    return ctx;
}

/// Phase 10: transfer-ownership variant of
/// :func:`frame_context_restore`.  Symmetric to
/// :func:`frame_context_save_transfer` — caller is responsible for
/// having drained the live state (typically via a paired
/// ``save_transfer`` call) before invoking this.  No retain/release
/// happens here; the snapshot's holds become the live state's holds.
pub fn frame_context_restore_transfer(ctx: FrameContext) void {
    frame_depth = ctx.depth;
    frame_stack = ctx.stack;
    frame_capacity = ctx.capacity;
    frame_ns = ctx.ns;
    frame_argv = ctx.argv;
    frame_info = ctx.info;
    // ``frame_dirty`` and ``frame_trace_heads`` are not part of
    // ``FrameContext`` (they're cheap to rebuild — the dirty mask
    // is a cache, the trace heads are zero in any newly-pushed
    // frame).  Reset them so a stale entry from the previous
    // tenant doesn't leak.
    var i: u32 = 0;
    while (i < MAX_DEPTH) : (i += 1) {
        frame_dirty[i] = 0;
        frame_trace_heads[i] = 0;
    }
}

/// Restore the live frame state from a previously-captured
/// ``FrameContext``.  Mirror of :func:`frame_context_save`.
///
/// IMPORTANT: this does NOT release any TclObj handles in the
/// argv / info slots that the live state currently holds.  The
/// caller is responsible for ownership accounting — typically
/// the save/restore is paired (save on yield, restore on
/// resume) so the same handles round-trip without a release.
/// For non-paired transfers the caller must explicitly drain
/// the live state before calling restore.
pub fn frame_context_restore(ctx: FrameContext) void {
    frame_depth = ctx.depth;
    frame_stack = ctx.stack;
    frame_capacity = ctx.capacity;
    frame_ns = ctx.ns;
    frame_argv = ctx.argv;
    frame_info = ctx.info;
}

/// Reset the live frame state to "no frames pushed".  Used at the
/// entry to an isolated eval (e.g. a fresh coroutine resume) where
/// the wrapper wants to start with a clean stack independent of
/// the caller's depth.  After the inner eval, the caller restores
/// its previously-saved ``FrameContext`` to undo this reset.
pub fn frame_context_reset() void {
    frame_depth = 0;
    // The slot arrays don't need zeroing — ``frame_push`` reuses
    // them via the dirty-bitmap mechanism (per-bucket reset on
    // first reuse) and the argv / info slots have their own
    // reset path on push.
}

// --- Indexed locals (compile-time slot resolution) ---------------------
//
// The Python-side ``var_escape/_slot_resolution.py`` pass walks each
// proc body, decides which scalar literal locals can safely live in
// the indexed array (no upvar / global / variable / vwait / regexp
// capture-binding / info introspection / trace / nested eval / etc.
// — see the pass for the full eligibility list), and stamps the
// resulting ``{name: slot}`` map on :class:`ProcEscapeSummary.
// local_slot_indices`.  The WASM emitter consults that map in
// :func:`_emit_var_write_obj_impl` / :func:`_emit_var_read_obj_lenient`
// to swap ``tcl_local_set`` / ``tcl_local_get_or_error`` for
// ``frame_local_set_at(idx)`` / ``frame_local_at(idx)``.
//
// Shape: a per-frame ``locals_array`` of fixed capacity.  We
// pre-allocate ``LOCALS_ARRAY_CAP`` slots per frame; codegen-
// resolved locals use indices ``0..LOCALS_ARRAY_CAP-1``.  Procs
// with more locals than that capacity continue to use the
// hash-keyed store transparently — ``frame_local_at`` only
// handles indexed slots, never the dynamic ones.  ``frame_pop``
// drains the indexed slots so their refcount accounting can't
// drift even if a stray future caller writes through the indexed
// accessor in an unexpected way.
const LOCALS_ARRAY_CAP: u32 = 16;
var frame_locals_array: [MAX_DEPTH][LOCALS_ARRAY_CAP]i32 = undefined;

/// Read a compiled-proc local by its compile-time-resolved slot
/// index.  Returns 0 (the "unset" sentinel) when the slot has
/// never been written or when the index is out of range.
/// Compiled callers MUST stay within ``[0, LOCALS_ARRAY_CAP)``;
/// the runtime trusts the codegen to emit valid indices.
pub export fn frame_local_at(idx: u32) i32 {
    if (frame_depth == 0) return 0;
    if (idx >= LOCALS_ARRAY_CAP) return 0;
    return frame_locals_array[frame_depth - 1][idx];
}

/// Write a compiled-proc local by slot index.  Caller-side
/// retention contract matches ``local_set``: the slot's prior
/// occupant (if any) is released; the new value is retained for
/// the slot's lifetime.  Returns the stored value for chaining.
pub export fn frame_local_set_at(idx: u32, value: i32) i32 {
    if (frame_depth == 0) return value;
    if (idx >= LOCALS_ARRAY_CAP) return value;
    const slot = &frame_locals_array[frame_depth - 1][idx];
    const old = slot.*;
    if (value != 0) obj.tcl_obj_retain(value);
    slot.* = value;
    if (old != 0 and old != value) obj.tcl_obj_release(old);
    return value;
}

/// Clear every compile-time slot for the current frame.  Called
/// from ``frame_pop`` to release the slot references before the
/// frame is reused.  Codegen doesn't need to call this directly.
fn frame_locals_array_drop_current() void {
    if (frame_depth == 0) return;
    var i: u32 = 0;
    const slots = &frame_locals_array[frame_depth - 1];
    while (i < LOCALS_ARRAY_CAP) : (i += 1) {
        const v = slots[i];
        if (v != 0) {
            slots[i] = 0;
            obj.tcl_obj_release(v);
        }
    }
}

/// Record the namespace context for the *current* frame.  Called
/// by the proc-call dispatcher right after it switches
/// ``current_ns`` to the proc's namespace, so a later ``uplevel``
/// from inside the body can read the caller's saved namespace via
/// :func:`frame_get_ns` and re-enter it.  No retain/release
/// bookkeeping — the namespace is a long-lived ``*Namespace`` heap
/// address, not a refcounted TclObj.
pub export fn frame_set_ns(ns: u32) void {
    if (frame_depth == 0) return;
    frame_ns[frame_depth - 1] = ns;
}

/// Like :func:`frame_set_ns` but only stamps when the slot is
/// still empty (zero).  Used by ``ns_set`` so a compiled proc's
/// prologue records its caller-ns once on frame entry, and any
/// secondary ``ns_set`` calls (eval-fallback regions push the
/// proc's own namespace for the duration of the fallback) don't
/// clobber the original caller-ns slot.
pub export fn frame_set_ns_if_unset(ns: u32) void {
    if (frame_depth == 0) return;
    if (frame_ns[frame_depth - 1] == 0) {
        frame_ns[frame_depth - 1] = ns;
    }
}

/// Return the namespace recorded for the frame *offset* steps down
/// from the top (0 = current, 1 = caller, …).  Returns 0 when no
/// ns was recorded (frame predates :func:`frame_set_ns`); callers
/// treat 0 as "leave ``current_ns`` as-is".
pub export fn frame_get_ns(offset: i32) u32 {
    if (offset < 0) return 0;
    const u: u32 = @intCast(offset);
    if (u >= frame_depth) return 0;
    return frame_ns[frame_depth - 1 - u];
}

/// Return the invocation argv stored for the frame *offset*
/// steps down from the top (0 = current, 1 = caller, …).
/// Returns 0 if the offset is out of range or no argv was
/// recorded for that frame.
pub export fn frame_get_argv(offset: i32) i32 {
    if (offset < 0) return 0;
    const u: u32 = @intCast(offset);
    if (u >= frame_depth) return 0;
    return frame_argv[frame_depth - 1 - u];
}

// Side stack for namespace context saved by ``frame_depth_stash``
// alongside the frame depth.  Same MAX_DEPTH cap as the frame stack
// itself — every nested ``uplevel`` records one slot.  The two
// stacks share an index (``ns_save_top``) so a stash/restore pair
// always touches one slot per nesting level.
var ns_save_stack: [MAX_DEPTH]u32 = [_]u32{0} ** MAX_DEPTH;
var ns_save_top: u32 = 0;

// Side stack for "parked" frames — a frame's full state (buffer
// pointer + capacity + dirty bitmap + ns + argv) is moved here
// while ``frame_depth_stash`` shifts the active depth lower for
// an ``uplevel``.  Without this parking step, a compiled-proc
// call dispatched *inside* the upleveled body would push its
// new frame at the index the stashed frame still logically
// owned — overwriting the caller's locals (selective-clear via
// the dirty bitmap zeros every populated bucket) and leaving
// the caller's frame data destroyed once ``uplevel`` returns.
//
// One slot per stashed level per active stash, so a deeply
// nested ``uplevel`` can't run out of room before the frame
// stack itself does.  The per-stash count lives in
// ``parked_count_stack`` so each ``frame_depth_restore`` knows
// exactly how many frames its corresponding stash parked.
var parked_stack:    [MAX_DEPTH]u32 = [_]u32{0} ** MAX_DEPTH;
var parked_capacity: [MAX_DEPTH]u32 = [_]u32{0} ** MAX_DEPTH;
var parked_dirty:    [MAX_DEPTH]u64 = [_]u64{0} ** MAX_DEPTH;
var parked_ns:       [MAX_DEPTH]u32 = [_]u32{0} ** MAX_DEPTH;
var parked_argv:     [MAX_DEPTH]i32 = [_]i32{0} ** MAX_DEPTH;
var parked_top: u32 = 0;
// Per-stash count of parked frames.  Indexed by ``ns_save_top``
// at stash time so restore can recover the matching count.
var parked_count_stack: [MAX_DEPTH]u32 = [_]u32{0} ** MAX_DEPTH;

/// Save the current frame depth and decrement it by *relative_up*
/// (clamped to 0).  Returns the saved depth so the caller can
/// restore it via ``frame_depth_restore``.  Used by ``uplevel`` to
/// temporarily act as if we're running *relative_up* frames above
/// the current one.
///
/// Also stashes ``current_ns`` and re-enters the target frame's
/// recorded namespace so unqualified variable lookups inside the
/// upleveled body resolve against the caller's namespace, not the
/// callee's.  Without this, ``uplevel 1 $body`` from a tcltest
/// dispatcher in ``::tcltest`` would leave ``current_ns`` pointing
/// at ``::tcltest`` and the body's array refs would miss the
/// caller's namespace's array — see :func:`frame_set_ns`.
///
/// Parks the top *relative_up* frames in a side stack (clearing
/// the original slots) so a compiled-proc call inside the
/// upleveled body gets a fresh slot at ``frame_depth`` instead of
/// reusing the parked frame's buffer.  ``frame_depth_restore``
/// puts them back unchanged.  Required because our frame buffers
/// are reused across pushes via the dirty-bitmap selective
/// clear — without parking, ``uplevel 1 [list CompiledCall]``
/// would zero every populated bucket of the caller's frame on
/// the inner push, losing all of its locals.
pub export fn frame_depth_stash(relative_up: i32) i32 {
    const saved: i32 = @intCast(frame_depth);
    var up = relative_up;
    if (up < 0) up = 0;
    var u: u32 = @intCast(up);
    if (u > frame_depth) u = frame_depth;

    // Capture caller-ns BEFORE we pop, then push and switch.
    var stash_idx: u32 = 0;
    if (ns_save_top < MAX_DEPTH) {
        stash_idx = ns_save_top;
        ns_save_stack[ns_save_top] = tcl_ns.current_ns;
        ns_save_top += 1;
    }
    var target_ns: u32 = 0;
    if (u > 0 and u <= frame_depth) {
        // ``u-1`` levels down from the top is the frame whose
        // recorded caller-ns matches the level we're shifting to.
        target_ns = if (u - 1 < frame_depth) frame_ns[frame_depth - 1 - (u - 1)] else 0;
    }
    if (target_ns != 0) {
        tcl_ns.current_ns = target_ns;
    } else if (u >= frame_depth) {
        tcl_ns.current_ns = tcl_ns.ns_root();
    }

    // Park the top ``u`` frames so the slots can be reused by
    // any compiled-proc dispatched within the upleveled body
    // without trampling the stashed locals.  The slots are
    // cleared (frame_stack=0) so the next ``frame_push`` for
    // them allocates a brand-new buffer.
    var parked_here: u32 = 0;
    if (parked_top + u <= MAX_DEPTH) {
        var i: u32 = 0;
        while (i < u) : (i += 1) {
            const slot = frame_depth - 1 - i;
            parked_stack[parked_top]    = frame_stack[slot];
            parked_capacity[parked_top] = frame_capacity[slot];
            parked_dirty[parked_top]    = frame_dirty[slot];
            parked_ns[parked_top]       = frame_ns[slot];
            parked_argv[parked_top]     = frame_argv[slot];
            // Clear the slot.  ``frame_argv`` is NOT released
            // here — the parked entry retains the caller's
            // refcount; restore puts the same handle back.
            frame_stack[slot]    = 0;
            frame_capacity[slot] = 0;
            frame_dirty[slot]    = 0;
            frame_ns[slot]       = 0;
            frame_argv[slot]     = 0;
            parked_top += 1;
            parked_here += 1;
        }
    }
    if (stash_idx < MAX_DEPTH) {
        parked_count_stack[stash_idx] = parked_here;
    }

    if (u >= frame_depth) {
        frame_depth = 0;
    } else {
        frame_depth -= u;
    }
    return saved;
}

/// Restore frame_depth to the value returned by an earlier
/// ``frame_depth_stash``, and pop the matching namespace save so
/// ``current_ns`` returns to the callee's namespace.
pub export fn frame_depth_restore(saved: i32) void {
    if (saved < 0) {
        frame_depth = 0;
    } else {
        frame_depth = @intCast(saved);
    }
    if (ns_save_top > 0) {
        ns_save_top -= 1;
        tcl_ns.current_ns = ns_save_stack[ns_save_top];
        // Unpark the matching frames in reverse insertion order
        // — the topmost park lands back in the caller's slot,
        // restoring the buffer pointer + capacity + dirty
        // bitmap so a subsequent ``local_get`` finds the
        // pre-uplevel value untouched.
        const u = parked_count_stack[ns_save_top];
        var i: u32 = u;
        while (i > 0) : (i -= 1) {
            if (parked_top == 0) break;
            parked_top -= 1;
            const slot = frame_depth - i;
            // If a compiled-proc dispatched inside the upleveled
            // body re-allocated this slot, ``frame_stack[slot]``
            // now points at a fresh buffer.  ``free_sized`` it
            // before overwriting with the parked pointer — without
            // this, repeated ``uplevel`` calls that invoke
            // compiled procs at the parked-slot depth grow linear
            // memory by one full frame buffer per call (Copilot
            // review on PR #325).  The capacity-driven size
            // matches what ``frame_push`` would have allocated;
            // the bump allocator's free-list reclaims the slab.
            const orphan_base = frame_stack[slot];
            const orphan_cap = frame_capacity[slot];
            if (orphan_base != 0
                and orphan_base != parked_stack[parked_top]
                and orphan_cap > 0)
            {
                obj.free_sized(orphan_base, orphan_cap * FRAME_BUCKET_SIZE);
            }
            frame_stack[slot] = parked_stack[parked_top];
            frame_capacity[slot] = parked_capacity[parked_top];
            frame_dirty[slot] = parked_dirty[parked_top];
            frame_ns[slot] = parked_ns[parked_top];
            frame_argv[slot] = parked_argv[parked_top];
        }
    }
}

// -- Internal helpers --

fn current_frame() ?u32 {
    if (frame_depth == 0) return null;
    return frame_stack[frame_depth - 1];
}

/// Return the frame base for an absolute 1-indexed depth, or null if out of range.
/// depth 1 = oldest frame (frame_stack[0]), depth frame_depth = current frame.
fn frame_at_depth(abs_depth: u32) ?u32 {
    if (abs_depth == 0 or abs_depth > frame_depth) return null;
    return frame_stack[abs_depth - 1];
}

fn frame_find(base: u32, name_ptr: u32, name_len: u32, hash: u32) ?u32 {
    const t = frame_table(base);
    return t.find(name_ptr, name_len, hash);
}

fn frame_insert(base: u32, name_ptr: u32, name_len: u32, hash: u32, value: i32) void {
    var current_base = base;
    var attempts: u32 = 0;
    // Phase 2.1: retry-with-grow loop.  Each grow doubles the
    // bucket count (up to FRAME_MAX_BUCKETS); a single growth may
    // not be enough on a heavily-clustered hash chain, so loop
    // until we either succeed, run out of grow budget, or the
    // grow itself fails to make progress.
    while (attempts < 8) : (attempts += 1) {
        var t = frame_table(current_base);
        if (t.try_insert_header(name_ptr, name_len, hash)) |bucket| {
            write_i32(bucket + OFF_VALUE, value);
            // Mark the chunk this bucket lives in as dirty so the
            // next ``frame_push`` for this slot clears it.
            // Overwrites to the same bucket via ``frame_find`` +
            // ``write_i32`` later don't need a separate
            // ``mark_dirty`` call — once a chunk's bit is set,
            // subsequent writes within the same chunk are already
            // covered.
            mark_dirty_for_base(current_base, bucket - current_base);
            return;
        }
        // Probe chain exhausted — grow and retry.
        const new_base = frame_grow_at_base(current_base);
        if (new_base == 0 or new_base == current_base) break;
        current_base = new_base;
    }
    // At the cap or grow alloc failed — fall through to the
    // load-bearing trap so callers notice rather than looping.
    const errmsg = "frame local table full";
    rt_fd_write_stderr(errmsg);
    @trap();
}

/// ``mark_dirty`` keyed by base address — looks up the matching
/// frame index in ``frame_stack`` and forwards the bucket offset.
/// Replaces the older "current frame only" check; needed because
/// frame_insert can run against any frame whose base is passed in
/// (uplevel paths reach into other frames).
inline fn mark_dirty_for_base(base: u32, bucket_offset: u32) void {
    var i: u32 = 0;
    while (i < MAX_DEPTH) : (i += 1) {
        if (frame_stack[i] == base) {
            mark_dirty(i, bucket_offset);
            return;
        }
    }
}

/// Write *msg* directly to stderr (fd=2) via WASI.  Used only in the
/// frame-overflow trap path, so we accept a tiny amount of duplication
/// rather than introduce a circular import with tcl_catch.
fn rt_fd_write_stderr(msg: []const u8) void {
    const io = @import("../io/tcl_io.zig");
    io.fd_write_all(2, msg.ptr, @intCast(msg.len));
    io.fd_write_all(2, "\n", 1);
}

// -- ALIAS_EXT resolution helpers --

/// Read the value that an ALIAS_EXT bucket points to.
fn resolve_ext_get(desc: u32, local_name: i32) i32 {
    const kind = read_i32(desc);
    const tgt = read_i32(desc + 8);
    if (kind == KIND_GLOBAL_NAMED) return globals.global_get(tgt);
    if (kind == KIND_NS_VAR_PTR) {
        // tgt is an absolute *Var address.  ``var_get_scalar``
        // follows VAR_LINK chains to the terminal storage.
        return @bitCast(tcl_ns.var_get_scalar(@bitCast(tgt)));
    }
    // KIND_FRAME_VAR
    const abs: u32 = @bitCast(read_i32(desc + 4));
    return frame_get_at_depth(abs, tgt, local_name);
}

/// Write a value through an ALIAS_EXT bucket.
fn resolve_ext_set(desc: u32, local_name: i32, value: i32) i32 {
    const kind = read_i32(desc);
    const tgt = read_i32(desc + 8);
    if (kind == KIND_GLOBAL_NAMED) return globals.global_set(tgt, value);
    if (kind == KIND_NS_VAR_PTR) {
        tcl_ns.var_set_scalar(@bitCast(tgt), @bitCast(value));
        return value;
    }
    const abs: u32 = @bitCast(read_i32(desc + 4));
    frame_set_at_depth(abs, tgt, local_name, value);
    return value;
}

/// Check existence through an ALIAS_EXT bucket.
fn resolve_ext_exists(desc: u32, local_name: i32) i32 {
    const kind = read_i32(desc);
    const tgt = read_i32(desc + 8);
    if (kind == KIND_GLOBAL_NAMED) return globals.global_exists(tgt);
    if (kind == KIND_NS_VAR_PTR) {
        // The Var entry exists once we've created it; existence
        // checks return 1 even if the value was never written.
        // Matches the long-standing ``global_exists`` semantics.
        return if (tgt != 0) obj_new_int(1) else obj_new_int(0);
    }
    const abs: u32 = @bitCast(read_i32(desc + 4));
    return if (frame_exists_at_depth(abs, tgt, local_name)) obj_new_int(1) else obj_new_int(0);
}

// -- Frame-at-depth read/write/exists --

/// Read variable *name* from the frame at *abs_depth* (1-indexed).
/// Follows aliases within that frame.  *fallback_name* is the local name
/// used to look up same-name ALIAS_GLOBAL entries.
fn frame_get_at_depth(abs_depth: u32, name: i32, fallback_name: i32) i32 {
    if (abs_depth == 0) return globals.global_get(name);
    if (frame_at_depth(abs_depth)) |base| {
        const sn = obj_ensure_string(name);
        const hash = fnv1a(sn.ptr, sn.len);
        if (frame_find(base, sn.ptr, sn.len, hash)) |bucket| {
            const v = read_i32(bucket + OFF_VALUE);
            if (v == ALIAS_GLOBAL) return globals.global_get(name);
            if (is_alias_ext(v)) return resolve_ext_get(alias_desc_ptr(v), fallback_name);
            return v;
        }
    }
    return 0;
}

/// Write *value* to variable *name* in the frame at *abs_depth* (1-indexed).
fn frame_set_at_depth(abs_depth: u32, name: i32, fallback_name: i32, value: i32) void {
    if (abs_depth == 0) { _ = globals.global_set(name, value); return; }
    if (frame_at_depth(abs_depth)) |base| {
        const sn = obj_ensure_string(name);
        const hash = fnv1a(sn.ptr, sn.len);
        if (frame_find(base, sn.ptr, sn.len, hash)) |bucket| {
            const v = read_i32(bucket + OFF_VALUE);
            if (v == ALIAS_GLOBAL) { _ = globals.global_set(name, value); return; }
            if (is_alias_ext(v)) { _ = resolve_ext_set(alias_desc_ptr(v), fallback_name, value); return; }
            write_i32(bucket + OFF_VALUE, value);
            return;
        }
        frame_insert(base, sn.ptr, sn.len, hash, value);
    } else {
        _ = globals.global_set(name, value);
    }
}

/// Check whether variable *name* exists in the frame at *abs_depth*.
fn frame_exists_at_depth(abs_depth: u32, name: i32, fallback_name: i32) bool {
    // ``global_exists`` and ``resolve_ext_exists`` return a freshly
    // allocated TclObj with integer value 0 or 1 — never a handle
    // that round-trips.  Comparing those handles against a new
    // ``obj_new_int(0)`` is always *unequal* (different addresses),
    // so we must unwrap them with ``obj_get_int`` instead.
    if (abs_depth == 0) return obj_get_int(globals.global_exists(name)) != 0;
    if (frame_at_depth(abs_depth)) |base| {
        const sn = obj_ensure_string(name);
        const hash = fnv1a(sn.ptr, sn.len);
        if (frame_find(base, sn.ptr, sn.len, hash)) |bucket| {
            const v = read_i32(bucket + OFF_VALUE);
            if (v == ALIAS_GLOBAL) return obj_get_int(globals.global_exists(name)) != 0;
            if (is_alias_ext(v)) return obj_get_int(resolve_ext_exists(alias_desc_ptr(v), fallback_name)) != 0;
            return true;
        }
    }
    return false;
}

// -- Alias registration --

/// Register *name* in the current frame as an alias to the global scope.
/// Subsequent var_set/var_resolve/var_exists calls for this name pass
/// through to the globals table.  This is how the Tcl ``global`` command
/// makes proc-local writes actually land in global storage.
pub export fn frame_alias_global(name: i32) void {
    const sn = obj_ensure_string(name);
    if (current_frame()) |base| {
        const hash = fnv1a(sn.ptr, sn.len);
        if (frame_find(base, sn.ptr, sn.len, hash)) |bucket| {
            write_i32(bucket + OFF_VALUE, ALIAS_GLOBAL);
            return;
        }
        frame_insert(base, sn.ptr, sn.len, hash, ALIAS_GLOBAL);
    }
    // No active frame — global is already the scope, nothing to alias.
}

/// Register *local_name* in the current frame as an alias to global
/// variable *target_name* (which may differ from *local_name*).
/// Used by the interpreter's ``upvar #0 other local`` handling.
pub export fn frame_alias_named(local_name: i32, target_name: i32) void {
    const desc = alloc(12);
    write_i32(desc, KIND_GLOBAL_NAMED);
    write_i32(desc + 4, 0); // param unused for global aliases
    write_i32(desc + 8, target_name);
    const encoded: i32 = -@as(i32, @intCast(desc));
    if (current_frame()) |base| {
        const sn = obj_ensure_string(local_name);
        const hash = fnv1a(sn.ptr, sn.len);
        if (frame_find(base, sn.ptr, sn.len, hash)) |bucket| {
            write_i32(bucket + OFF_VALUE, encoded);
        } else {
            frame_insert(base, sn.ptr, sn.len, hash, encoded);
        }
    }
}

/// Register *local_name* in the current frame as an alias to variable
/// *target_name* in the frame at absolute depth *abs_depth* (1-indexed).
/// Used by the interpreter's ``upvar N other local`` handling.
pub export fn frame_alias_frame_var(local_name: i32, abs_depth: i32, target_name: i32) void {
    const desc = alloc(12);
    write_i32(desc, KIND_FRAME_VAR);
    write_i32(desc + 4, abs_depth);
    write_i32(desc + 8, target_name);
    const encoded: i32 = -@as(i32, @intCast(desc));
    if (current_frame()) |base| {
        const sn = obj_ensure_string(local_name);
        const hash = fnv1a(sn.ptr, sn.len);
        if (frame_find(base, sn.ptr, sn.len, hash)) |bucket| {
            write_i32(bucket + OFF_VALUE, encoded);
        } else {
            frame_insert(base, sn.ptr, sn.len, hash, encoded);
        }
    }
}

/// Register *local_name* in the current frame as a VAR_LINK-style
/// alias to a namespace variable identified by its absolute
/// ``*Var`` address (P3.3 onwards).  Used by the interpreter's
/// ``variable`` and ``global`` handlers.
///
/// The descriptor encoding follows the existing ALIAS_EXT shape:
/// ``[kind | unused | *Var]`` packed into 12 bytes; the bucket
/// value is the negated descriptor address.  Reads / writes go
/// through ``var_get_scalar`` / ``var_set_scalar`` which transparently
/// chase ``VAR_LINK`` chains on the target side.
pub fn frame_alias_ns_var(local_name: i32, var_ptr: u32) void {
    const cur = current_frame() orelse return;
    const desc = alloc(12);
    write_i32(desc, KIND_NS_VAR_PTR);
    write_i32(desc + 4, 0);
    write_i32(desc + 8, @bitCast(var_ptr));
    const encoded: i32 = -@as(i32, @intCast(desc));
    const sn = obj_ensure_string(local_name);
    const hash = fnv1a(sn.ptr, sn.len);
    if (frame_find(cur, sn.ptr, sn.len, hash)) |bucket| {
        write_i32(bucket + OFF_VALUE, encoded);
    } else {
        frame_insert(cur, sn.ptr, sn.len, hash, encoded);
    }
}

/// Resolve a Tcl ``upvar`` *level* token to an absolute frame depth.
/// *cur_depth* is the runtime's current frame depth (passed in to
/// avoid duplicate ``frame_get_depth`` reads on the codegen side).
///
/// Tcl semantics:
///   * ``#N`` -> absolute depth ``N`` (``#0`` is global scope).
///   * ``-?N`` (a bare integer, optionally signed) -> relative,
///     ``cur_depth - |N|``; reference Tcl rejects negative values
///     with an error but our codegen never asks for one.
///   * Anything else (a non-numeric token) means the level was
///     absent and the caller should have passed ``"1"`` instead;
///     we still return ``cur_depth - 1`` defensively rather than
///     trapping so a mis-detected dynamic value degrades gracefully.
pub export fn upvar_resolve_depth(level_obj: i32, cur_depth: i32) i32 {
    const sn = obj_ensure_string(level_obj);
    if (sn.len == 0) return cur_depth - 1;
    const sp: [*]const u8 = @ptrFromInt(sn.ptr);
    if (sp[0] == '#') {
        // Absolute level.
        var n: i32 = 0;
        var i: u32 = 1;
        while (i < sn.len) : (i += 1) {
            const c = sp[i];
            if (c < '0' or c > '9') return cur_depth - 1;
            n = n * 10 + @as(i32, @intCast(c - '0'));
        }
        return n;
    }
    // Relative integer (possibly signed).
    var i: u32 = 0;
    var sign: i32 = 1;
    if (sp[0] == '-') {
        sign = -1;
        i = 1;
    } else if (sp[0] == '+') {
        i = 1;
    }
    if (i >= sn.len) return cur_depth - 1;
    var n: i32 = 0;
    while (i < sn.len) : (i += 1) {
        const c = sp[i];
        if (c < '0' or c > '9') return cur_depth - 1;
        n = n * 10 + @as(i32, @intCast(c - '0'));
    }
    const rel = if (sign < 0) -n else n;
    if (rel < 0) {
        // Reference Tcl rejects negative ``upvar`` levels with
        // ``bad level "-N"`` rather than silently aliasing the
        // wrong frame (Copilot review on PR #325).  Surface the
        // same error so a script that computes ``upvar [expr
        // {-$n}] ...`` fails fast.
        const tcl_catch = @import("tcl_catch.zig");
        const prefix: []const u8 = "bad level \"";
        const suffix: []const u8 = "\"";
        const total: u32 = @as(u32, @intCast(prefix.len)) + sn.len +
            @as(u32, @intCast(suffix.len));
        const buf = obj.alloc(total);
        const d: [*]u8 = @ptrFromInt(buf);
        var off: u32 = 0;
        for (prefix) |b| {
            d[off] = b;
            off += 1;
        }
        if (sn.len > 0) {
            const src: [*]const u8 = @ptrFromInt(sn.ptr);
            for (0..sn.len) |k| {
                d[off] = src[k];
                off += 1;
            }
        }
        for (suffix) |b| {
            d[off] = b;
            off += 1;
        }
        const msg = obj.obj_new_string_take(buf, total, total);
        tcl_catch.tcl_cmd_error(msg);
        return cur_depth - 1;
    }
    return cur_depth - rel;
}

// -- Local variable operations on current frame --

/// Set a local variable in the current frame.
/// If no frame is active, falls through to global_set.
/// Follows ALIAS_GLOBAL and ALIAS_EXT on write.
pub export fn local_set(name: i32, value: i32) i32 {
    const sn = obj_ensure_string(name);
    if (current_frame()) |base| {
        const hash = fnv1a(sn.ptr, sn.len);
        if (frame_find(base, sn.ptr, sn.len, hash)) |bucket| {
            const v = read_i32(bucket + OFF_VALUE);
            if (v == ALIAS_GLOBAL) return globals.global_set(name, value);
            if (is_alias_ext(v)) return resolve_ext_set(alias_desc_ptr(v), name, value);
            // MM-B.3 refcount discipline: the frame slot owns a
            // reference to its value.  Retain the new value, release
            // the old one.  Without this the slot's hold is "free"
            // and the parser-side release at end-of-statement
            // (MM-B.4) would free param values out from under the
            // running proc body.
            if (value != 0) obj.tcl_obj_retain(value);
            write_i32(bucket + OFF_VALUE, value);
            if (v != 0 and v != value) obj.tcl_obj_release(v);
            fire_local_trace(sn.ptr, sn.len, value);
            return value;
        }
        // Fresh insert — retain the new value (no old to release).
        if (value != 0) obj.tcl_obj_retain(value);
        frame_insert(base, sn.ptr, sn.len, hash, value);
        fire_local_trace(sn.ptr, sn.len, value);
        return value;
    }
    // No frame active — set global (var_set_scalar handles
    // refcount on its end via MM-B.2).
    return globals.global_set(name, value);
}

/// Phase 6 follow-up: fire any matching frame-local WRITE / UNSET
/// trace.  ``value == 0`` is the in-runtime "unset" signal (matches
/// the ``var_set(name, 0)`` route taken by ``unset NAME`` in
/// :file:`cmds/var.zig`); for non-zero values we fire WRITE.  No-op
/// when the frame's trace head is empty — the hot path for untraced
/// procs.
fn fire_local_trace(name_ptr: u32, name_len: u32, value: i32) void {
    if (frame_depth == 0) return;
    const head = frame_trace_heads[frame_depth - 1];
    if (head == 0) return;
    const tcl_var_trace = @import("tcl_var_trace.zig");
    if (value == 0) {
        tcl_var_trace.fire_in_list(head, name_ptr, name_len, tcl_var_trace.OP_UNSET, 'u');
    } else {
        tcl_var_trace.fire_in_list(head, name_ptr, name_len, tcl_var_trace.OP_WRITE, 'w');
    }
}

/// Frame-sync write — store ``value`` in the local slot for ``name``
/// without firing variable traces.  Used by the codegen's
/// ``_emit_frame_sync`` path on its way into a ``tcl_eval`` /
/// runtime-import boundary: the call is a state transfer (mirror
/// the WASM local into the frame so the interpreter sees the
/// up-to-date value), not a user-visible assignment, so a write
/// trace must not observe it.  Same refcount discipline as
/// :func:`local_set`.
pub export fn local_set_silent(name: i32, value: i32) i32 {
    const sn = obj_ensure_string(name);
    if (current_frame()) |base| {
        const hash = fnv1a(sn.ptr, sn.len);
        if (frame_find(base, sn.ptr, sn.len, hash)) |bucket| {
            const v = read_i32(bucket + OFF_VALUE);
            if (v == ALIAS_GLOBAL) return globals.global_set(name, value);
            if (is_alias_ext(v)) return resolve_ext_set(alias_desc_ptr(v), name, value);
            if (value != 0) obj.tcl_obj_retain(value);
            write_i32(bucket + OFF_VALUE, value);
            if (v != 0 and v != value) obj.tcl_obj_release(v);
            return value;
        }
        if (value != 0) obj.tcl_obj_retain(value);
        frame_insert(base, sn.ptr, sn.len, hash, value);
        return value;
    }
    return globals.global_set(name, value);
}

/// Get a local variable from the current frame.
/// Returns 0 if not found in current frame (does NOT fall through to globals).
/// Follows ALIAS_GLOBAL and ALIAS_EXT on read.
pub export fn local_get(name: i32) i32 {
    const sn = obj_ensure_string(name);
    if (current_frame()) |base| {
        const hash = fnv1a(sn.ptr, sn.len);
        if (frame_find(base, sn.ptr, sn.len, hash)) |bucket| {
            const v = read_i32(bucket + OFF_VALUE);
            if (v == ALIAS_GLOBAL) return globals.global_get(name);
            if (is_alias_ext(v)) return resolve_ext_get(alias_desc_ptr(v), name);
            // Phase 6 follow-up: fire READ trace before handing the
            // value back.  Trace bodies can mutate the slot (write
            // trace + assign), so re-read after firing.
            if (frame_depth != 0 and frame_trace_heads[frame_depth - 1] != 0) {
                const tcl_var_trace = @import("tcl_var_trace.zig");
                if (tcl_var_trace.has_trace_in_list(frame_trace_heads[frame_depth - 1], sn.ptr, sn.len, tcl_var_trace.OP_READ)) {
                    tcl_var_trace.fire_in_list(frame_trace_heads[frame_depth - 1], sn.ptr, sn.len, tcl_var_trace.OP_READ, 'r');
                    return read_i32(bucket + OFF_VALUE);
                }
            }
            return v;
        }
    }
    return 0;
}

/// Frame-readback read — counterpart of :func:`local_set_silent`.
/// Reads the local slot without firing READ traces; used by the
/// codegen ``_emit_frame_readback`` path that pulls interpreter-side
/// updates back into WASM locals after a ``tcl_eval`` boundary.
pub export fn local_get_silent(name: i32) i32 {
    const sn = obj_ensure_string(name);
    if (current_frame()) |base| {
        const hash = fnv1a(sn.ptr, sn.len);
        if (frame_find(base, sn.ptr, sn.len, hash)) |bucket| {
            const v = read_i32(bucket + OFF_VALUE);
            if (v == ALIAS_GLOBAL) return globals.global_get(name);
            if (is_alias_ext(v)) return resolve_ext_get(alias_desc_ptr(v), name);
            return v;
        }
    }
    return 0;
}

/// Strict variant of :func:`local_get` for codegen-emitted ``$x``
/// substitutions / ``set x`` reads / ``expr {$x}`` operands.  When the
/// frame slot resolves to 0 (variable never set in this scope) it
/// raises ``can't read "<name>": no such variable`` through
/// :func:`tcl_catch.var_unset_error` so the WASM backend matches the
/// Python VM and reference Tcl.  ALIAS_GLOBAL / ALIAS_EXT slots that
/// resolve to a 0-valued target trigger the same error path.  Lookups
/// from ``info exists`` / ``unset -nocomplain`` / frame readback after
/// an eval-fallback continue to use the lenient :func:`local_get`.
///
/// Lazy-imports ``tcl_catch`` to side-step the older "frame-overflow
/// trap path can't reach the catch module" comment near
/// :func:`rt_fd_write_stderr` — that constraint applied to the trap
/// path's stderr write, not the var-read error which goes through the
/// normal :func:`tcl_catch.tcl_cmd_error` route.
pub export fn local_get_or_error(name: i32) i32 {
    const v = local_get(name);
    if (v == 0) {
        const tcl_catch = @import("tcl_catch.zig");
        tcl_catch.var_unset_error(name);
    }
    return v;
}

/// Check if a local variable exists in the current frame.
/// Follows aliases to their final target for the existence check.
pub export fn local_exists(name: i32) i32 {
    const sn = obj_ensure_string(name);
    if (current_frame()) |base| {
        const hash = fnv1a(sn.ptr, sn.len);
        if (frame_find(base, sn.ptr, sn.len, hash)) |bucket| {
            const v = read_i32(bucket + OFF_VALUE);
            if (v == ALIAS_GLOBAL) return globals.global_exists(name);
            if (is_alias_ext(v)) return resolve_ext_exists(alias_desc_ptr(v), name);
            return obj_new_int(1);
        }
    }
    return obj_new_int(0);
}

/// Resolve a variable: check current frame first, then globals.
/// This is the standard Tcl lookup order for the interpreter.
pub export fn var_resolve(name: i32) i32 {
    // Phase 9: cross-interp variable link probe.  When a name in
    // the current interp is linked to a target in another interp,
    // route the read through the target interp's resolution.  The
    // probe is gated by the link-registry's re-entry guard so a
    // cycle (A → B → A) doesn't loop.
    const xlinks = @import("tcl_xlinks.zig");
    const interp_reg = @import("tcl_interp_registry.zig");
    const lk = xlinks.lookup(interp_reg.interp_current(), name);
    if (lk.found) {
        defer xlinks.lookup_done();
        if (lk.target_interp == 0 or lk.target_name == 0) {
            return 0;
        }
        const save = interp_reg.enter(lk.target_interp);
        const v = var_resolve(lk.target_name);
        interp_reg.leave(save);
        return v;
    }
    const sn = obj_ensure_string(name);
    // Array-element form ``arr(key)`` — split into ``arr`` + ``key``
    // and route through the array directory.  The previous code
    // handed the entire ``arr(key)`` string to ``ns_var_find`` /
    // ``frame_find``, which look for SCALAR slots and miss every
    // array element.  ``info exists arr(key)`` already does this
    // split (see ``var_exists`` below); without the matching read
    // path, ``set $::ns::arr(key)`` reading an existing element
    // returned empty even when ``info exists`` confirmed the
    // element was set — root cause of opt-10.5..10 / 11.1 (the
    // tcltest test bodies use ``$::tcl::OptDesc(...)`` reads
    // through the eval-fallback path).
    if (sn.len >= 3) {
        const sp: [*]const u8 = @ptrFromInt(sn.ptr);
        var paren: u32 = 0;
        var found = false;
        var k: u32 = 0;
        while (k < sn.len) : (k += 1) {
            if (sp[k] == '(') {
                paren = k;
                found = true;
                break;
            }
        }
        if (found and paren > 0 and sp[sn.len - 1] == ')') {
            const tcl_array = @import("../valtypes/tcl_array.zig");
            // Phase 1: a single lookup path.  The Variable lives in
            // the array directory; ``frame_resolve_array_name`` picks
            // global / namespace / proc-local based on context.
            const arr_name = obj.obj_new_string(@bitCast(sn.ptr), @bitCast(paren));
            const resolved_arr = frame_resolve_array_name(arr_name);
            const key_len = sn.len - paren - 2;
            const key = obj.obj_new_string(@bitCast(sn.ptr + paren + 1), @bitCast(key_len));
            const v = tcl_array.array_get(resolved_arr, key);
            obj.tcl_obj_release(arr_name);
            if (resolved_arr != arr_name) obj.tcl_obj_release(resolved_arr);
            obj.tcl_obj_release(key);
            return v;
        }
    }
    // ``::``-qualified names always go to globals.
    if (sn.len >= 2) {
        const sp: [*]const u8 = @ptrFromInt(sn.ptr);
        if (sp[0] == ':' and sp[1] == ':') {
            return globals.global_get(name);
        }
    }
    if (current_frame()) |base| {
        const hash = fnv1a(sn.ptr, sn.len);
        if (frame_find(base, sn.ptr, sn.len, hash)) |bucket| {
            const v = read_i32(bucket + OFF_VALUE);
            if (v == ALIAS_GLOBAL) return globals.global_get(name);
            if (is_alias_ext(v)) return resolve_ext_get(alias_desc_ptr(v), name);
            return v;
        }
    }
    // No frame match.  Tcl 9: an unqualified name at script level
    // resolves against the current namespace's variable table
    // first, then falls through to the root global.  Inside
    // ``namespace eval ::ns { … }`` the compiled-side writer mirrors
    // ``set v X`` to the global table under ``::ns::v`` (see
    // ``_emit_var_write_obj_impl`` in the codegen), so the
    // interpreter must look for that qualified form before giving
    // up.  Without this branch ``$varName`` inside an eval-fallback
    // returned empty even after a successful compiled write.
    if (tcl_ns.current_ns != 0) {
        const ns_full = tcl_ns.ns_full_name(tcl_ns.current_ns);
        if (ns_full.len > 2) {
            // Build ``<ns_full>::<name>`` in the bump allocator and
            // look it up.  Skip when ns is the root (length 2 = "::").
            const total: u32 = ns_full.len + 2 + sn.len;
            const buf = obj.alloc(total);
            const dst: [*]u8 = @ptrFromInt(buf);
            const ns_p: [*]const u8 = @ptrFromInt(ns_full.ptr);
            for (0..ns_full.len) |i| dst[i] = ns_p[i];
            dst[ns_full.len] = ':';
            dst[ns_full.len + 1] = ':';
            const name_p: [*]const u8 = @ptrFromInt(sn.ptr);
            for (0..sn.len) |i| dst[ns_full.len + 2 + i] = name_p[i];
            const qname = obj.obj_new_string(@bitCast(buf), @bitCast(total));
            // global_exists returns a TclObj wrapping 0 or 1 — its
            // *handle* is always non-zero (a fresh integer obj), so a
            // raw ``!= 0`` test always passed and we always returned
            // ``global_get(qname)`` even for non-existent names.  Check
            // the wrapped int.
            const exists = obj.obj_get_int(globals.global_exists(qname)) != 0;
            if (exists) {
                const v = globals.global_get(qname);
                // Reclaim the qname temp + its backing buffer.  Without
                // this every ``$var`` read inside a namespace
                // accumulated a small leak that pushed io.test past the
                // 2 GiB linear-memory ceiling and tripped a u32→i32
                // ``@intCast`` panic in ``obj_new_string``.
                obj.tcl_obj_release(qname);
                obj.free_sized(buf, total);
                return v;
            }
            obj.tcl_obj_release(qname);
            obj.free_sized(buf, total);
        }
    }
    // Fall through to root global
    return globals.global_get(name);
}

/// Set a variable: sets in current frame if one is active, otherwise global.
/// If the local is an alias to a global, the write propagates to globals.
pub export fn var_set(name: i32, value: i32) i32 {
    // Phase 9: cross-interp variable link probe — same shape as
    // :func:`var_resolve`'s probe.  When the current interp's
    // variable is linked to another interp's, write through to
    // the target.  ``transfer_result`` doesn't run here because
    // the value lives in the shared linear memory; both interps
    // see the same handle and the destination's hash bucket
    // retains it on store.
    const xlinks = @import("tcl_xlinks.zig");
    const interp_reg = @import("tcl_interp_registry.zig");
    const lk = xlinks.lookup(interp_reg.interp_current(), name);
    if (lk.found) {
        defer xlinks.lookup_done();
        if (lk.target_interp == 0 or lk.target_name == 0) {
            return value;
        }
        const save = interp_reg.enter(lk.target_interp);
        const v = var_set(lk.target_name, value);
        interp_reg.leave(save);
        return v;
    }
    const sn = obj_ensure_string(name);
    // Array-element form ``arr(key)`` — split off the key and route
    // through ``array_set``.  Mirrors the read path in
    // ``var_resolve``; without this, ``set ::ns::arr(key) value``
    // would be stored as a SCALAR ns var literally named
    // ``arr(key)`` and the matching read would need to use the
    // same literal-string key path.  The split keeps array reads
    // and writes in the same array directory.
    if (sn.len >= 3) {
        const sp: [*]const u8 = @ptrFromInt(sn.ptr);
        var paren: u32 = 0;
        var found = false;
        var k: u32 = 0;
        while (k < sn.len) : (k += 1) {
            if (sp[k] == '(') {
                paren = k;
                found = true;
                break;
            }
        }
        if (found and paren > 0 and sp[sn.len - 1] == ')') {
            const tcl_array = @import("../valtypes/tcl_array.zig");
            // Phase 1 unification: every array — global, namespace,
            // proc-local — lives in the same ``tcl_array`` directory.
            // ``frame_resolve_array_name`` picks the right key:
            //   * already-FQ ``::ns::arr``  → unchanged.
            //   * upvar / global alias       → follow alias to target.
            //   * unaliased proc-local       → ``::__local::<depth>::arr``.
            //   * top-level                  → unchanged (global).
            const arr_name = obj.obj_new_string(@bitCast(sn.ptr), @bitCast(paren));
            const resolved_arr = frame_resolve_array_name(arr_name);
            const key_len = sn.len - paren - 2;
            const key = obj.obj_new_string(@bitCast(sn.ptr + paren + 1), @bitCast(key_len));
            _ = tcl_array.array_set(resolved_arr, key, value);
            obj.tcl_obj_release(arr_name);
            if (resolved_arr != arr_name) obj.tcl_obj_release(resolved_arr);
            obj.tcl_obj_release(key);
            return value;
        }
    }
    // ``::``-qualified names are always global, regardless of frame
    // depth — matches Tcl's namespace resolution where an absolute
    // name bypasses all local scopes.
    if (sn.len >= 2) {
        const sp: [*]const u8 = @ptrFromInt(sn.ptr);
        if (sp[0] == ':' and sp[1] == ':') {
            return globals.global_set(name, value);
        }
    }
    if (current_frame() != null) {
        return local_set(name, value);
    }
    return globals.global_set(name, value);
}

/// Check if a variable exists in local frame OR globals.
pub export fn var_exists(name: i32) i32 {
    const sn = obj_ensure_string(name);
    // Array-element form: ``arr(key)``.  Split on the first ``(`` and
    // probe the array storage for the named element.  ``var_exists``
    // is the implementation of ``info exists`` (see
    // ``cmds/tcl_cmd_info.zig::info_exists``), which Tcl 9 documents
    // as supporting both whole-variable and element forms.  Without
    // the split, ``info exists arr(key)`` looked up the literal
    // string ``arr(key)`` as a single var name, missed every time,
    // and any tcltest constraint check (``info exists
    // testConstraints($c)``) returned 0 — which caused the
    // ``Skipped`` proc to fall through to "do not skip" for every
    // simple constraint, so ``testevalex``-gated tests ran instead
    // of being skipped.
    if (sn.len >= 3) {
        const sp: [*]const u8 = @ptrFromInt(sn.ptr);
        var paren: u32 = 0;
        var found = false;
        var k: u32 = 0;
        while (k < sn.len) : (k += 1) {
            if (sp[k] == '(') { paren = k; found = true; break; }
        }
        if (found and paren > 0 and sp[sn.len - 1] == ')') {
            const tcl_array = @import("../valtypes/tcl_array.zig");
            const arr_name = obj.obj_new_string(@bitCast(sn.ptr), @bitCast(paren));
            // Phase 1: route through ``frame_resolve_array_name`` so a
            // proc-local ``info exists arr(key)`` finds the
            // ``::__local::<depth>::arr`` directory entry.
            const resolved_arr = frame_resolve_array_name(arr_name);
            const key_len = sn.len - paren - 2;
            const key = obj.obj_new_string(@bitCast(sn.ptr + paren + 1), @bitCast(key_len));
            const exists = tcl_array.array_element_exists(resolved_arr, key);
            obj.tcl_obj_release(arr_name);
            if (resolved_arr != arr_name) obj.tcl_obj_release(resolved_arr);
            obj.tcl_obj_release(key);
            return exists;
        }
    }
    if (current_frame()) |base| {
        const hash = fnv1a(sn.ptr, sn.len);
        if (frame_find(base, sn.ptr, sn.len, hash)) |bucket| {
            const v = read_i32(bucket + OFF_VALUE);
            if (v == ALIAS_GLOBAL) return globals.global_exists(name);
            if (is_alias_ext(v)) return resolve_ext_exists(alias_desc_ptr(v), name);
            return obj_new_int(1);
        }
    }
    // Check global
    return globals.global_exists(name);
}

/// Resolve an alias to get the underlying array name for ``array names``,
/// ``array exists``, ``array size``, etc.  When the current frame has an
/// ALIAS_EXT entry for *local_name* (created by ``upvar N otherVar
/// localName``), return the target's name TclObj so array operations can
/// find the array in the global directory.  For KIND_GLOBAL_NAMED and
/// KIND_FRAME_VAR descriptors the stored ``tgt`` field IS the name; for
/// KIND_NS_VAR_PTR the name isn't available, so fall back to *local_name*.
/// When there is no alias, return *local_name* unchanged.
pub export fn frame_resolve_array_name(local_name: i32) i32 {
    const sn = obj_ensure_string(local_name);
    // Already-FQ names (``::nsa::name``) bypass scope resolution —
    // they point at a specific global / namespace array directly.
    if (sn.len >= 2) {
        const sp: [*]const u8 = @ptrFromInt(sn.ptr);
        if (sp[0] == ':' and sp[1] == ':') return local_name;
    }
    if (current_frame()) |base| {
        const hash = fnv1a(sn.ptr, sn.len);
        if (frame_find(base, sn.ptr, sn.len, hash)) |bucket| {
            const v = read_i32(bucket + OFF_VALUE);
            if (is_alias_ext(v)) {
                const desc = alias_desc_ptr(v);
                const kind = read_i32(desc);
                const tgt = read_i32(desc + 8);
                if (kind == KIND_GLOBAL_NAMED or kind == KIND_FRAME_VAR) {
                    return tgt;
                }
            }
            if (v == ALIAS_GLOBAL) return local_name; // global alias keeps the same name
        }
        // Phase 1 unification: an unqualified, unaliased array inside
        // a proc frame lives in the global directory under a synthetic
        // ``::__local::<depth>::<name>`` key so :func:`array_names`,
        // ``array exists``, etc. all reach the same storage as
        // ``set arr(key)`` / ``$arr(key)``.  ``frame_pop`` evicts these
        // entries via ``drop_local_arrays_for_depth``.
        const tcl_array = @import("../valtypes/tcl_array.zig");
        return tcl_array.make_local_array_obj(local_name, frame_depth);
    }
    return local_name;
}
