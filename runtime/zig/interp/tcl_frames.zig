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
//   value == 0          : unset/null
//   value == -1         : ALIAS_GLOBAL — same-name global alias (``global x``)
//   (value & 1) != 0    : ALIAS_EXT   — heap-allocated 12-byte descriptor at
//                         (value & ~1).  Descriptor layout:
//                           [0..3]  kind  (i32): 0=global_named, 1=frame_var
//                           [4..7]  param (i32): frame_var → abs frame depth
//                           [8..11] target_name (i32): TclObj* for target name
//                         Heap allocations (TclObj headers, descriptors,
//                         hash-table buffers) all come from a size-class /
//                         libc allocator that returns at least 8-byte-
//                         aligned addresses, so a real pointer always has
//                         the low 3 bits clear.  The encoding sets bit 0
//                         to mark a value as an alias-descriptor address;
//                         decoding masks it off.  Stays correct for the
//                         full 32-bit address range — the previous
//                         "value < -1" sign-bit scheme collapsed once a
//                         TclObj header landed at addr ≥ 0x80000000
//                         (i32 reinterpretation makes the handle look
//                         like an alias and ``frame_find`` chases a
//                         garbage descriptor address out of the obj
//                         header bytes — observed as ``$body``/``$match``/
//                         ``$result`` in ``tcltest::test`` reading
//                         "-preservecore" once the cumulative trace.test
//                         run grew the heap past the 2 GiB mark).
//   else                : TclObj pointer (8-byte aligned, low bit clear)

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

/// Bucket VALUE field encoding (Phase 2 hardening — Stream 1's
/// trace.test cumulative-state run grows the heap past the 2 GiB
/// mark, after which TclObj handles land in [-0x80000000, -1] when
/// reinterpreted as i32).  The previous "value < -1 means ALIAS_EXT"
/// scheme used the SIGN BIT to distinguish aliases from regular obj
/// handles.  That scheme collapses once a TclObj header is allocated
/// at ``addr >= 0x80000000``: the handle bit-casts to a negative i32
/// indistinguishable from an ALIAS_EXT encoding, so ``frame_find`` /
/// ``local_get`` for an unaliased proc-local mistakes the body /
/// match / result TclObj handle for an alias descriptor and chases
/// it through ``resolve_ext_get`` into garbage memory — observed as
/// every scalar local in ``tcltest::test`` reading "-preservecore"
/// (the bytes the bogus alias path picks up) on trace-2.7.
///
/// New scheme: an ALIAS_EXT bucket value has BOTH bit 31 (sign) AND
/// bit 0 set — i.e. ``(v & 0x80000001) == 0x80000001 && v != -1``.
/// Heap allocations are ≥8-byte aligned (size-class allocator and
/// libc malloc both honour this), so a real obj handle / descriptor
/// address has bit 0 = 0; tagged-immediate encodings have bit 0 = 1
/// but are restricted to non-negative values (bit 31 = 0); and
/// ``ALIAS_GLOBAL = -1`` is excluded explicitly.  None of the four
/// other domains can therefore collide with the ALIAS_EXT pattern.
///
/// To keep the descriptor address bits intact across the encoding,
/// the address — which has its low 3 bits clear — is shifted right
/// once before being merged with the marker bits.  Decoding masks
/// off the markers and shifts back.  This costs one bit of address
/// space (the shift loses a high bit), but a 30-bit address range
/// is still 8 GiB after factoring in the 8-byte alignment, more
/// than enough for our heap ceiling.
inline fn is_alias_ext(v: i32) bool {
    if (v == ALIAS_GLOBAL) return false;
    const u: u32 = @bitCast(v);
    return (u & 0x80000001) == 0x80000001;
}

inline fn alias_desc_ptr(v: i32) u32 {
    const u: u32 = @bitCast(v);
    // Strip both marker bits, then shift back to recover the address.
    return (u & 0x7FFFFFFE) << 1;
}

inline fn encode_alias_ext(desc: u32) i32 {
    // 8-byte alignment guarantees ``desc & 0x7 == 0`` and in
    // particular ``desc & 1 == 0``, so ``desc >> 1`` is lossless.
    // The OR then sets bit 0 and bit 31 to mark the value as an
    // alias-descriptor encoding.
    return @bitCast((desc >> 1) | 0x80000001);
}

// -- Frame-alias descriptor kinds --
const KIND_GLOBAL_NAMED: i32 = 0; // target_name is global var name
const KIND_FRAME_VAR: i32 = 1; // param = abs frame depth, target_name = var
const KIND_NS_VAR_PTR: i32 = 2; // descriptor.target = absolute *Var address (P3.3)

// Recursion depth cap.  Reference Tcl 9 defaults to a soft stack
// limit of 1000 frames (``Tcl_RecursionLimit``); deep recursive
// algorithms like the 12days IOCCC stress test push tens of
// thousands of calls during their lifetime (peak depth a few
// hundred).  64 was enough for tcltest's own machinery but not for
// user procs that recurse meaningfully — at depth >64 the
// ``frame_push`` returned -1 and the codegen DROPped the result,
// so subsequent ``local_set`` calls silently wrote into the
// previous frame.  Bumping to 1024 covers reference Tcl's default
// while keeping the per-slot ``frame_stack`` / ``frame_capacity``
// / ``frame_dirty`` arrays bounded.
const MAX_DEPTH: u32 = 1024;

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

// Per-frame ``const`` tracker (Tcl 9 ``const VARNAME VAL`` /
// ``info constant`` / ``info consts``).  Frame buckets are 16 bytes
// (12 header + 4 value) with no room for a per-slot ``VAR_CONSTANT``
// flag, so const-ness for proc-local names rides in this side list.
// Each slot is a small array of (name_ptr, name_len, hash) records
// that get matched on every ``const`` re-declaration / ``info
// constant NAME`` probe.  Capacity is bounded — runaway use is
// pathological for a single proc — and ``frame_pop`` clears the
// count so a recycled depth never inherits stale entries.
const FRAME_CONST_CAP: u32 = 64;
const FrameConstName = extern struct {
    name_ptr: u32,
    name_len: u32,
    hash: u32,
};
// Slot contents are read only when guarded by ``frame_const_count`` —
// the zeroed count covers stale data so leaving the records
// ``undefined`` here is safe and avoids paying for a ~65 KiB static
// init on module startup.
var frame_const_names: [MAX_DEPTH][FRAME_CONST_CAP]FrameConstName = undefined;
var frame_const_count: [MAX_DEPTH]u32 = [_]u32{0} ** MAX_DEPTH;

/// Mark a name as constant in the current frame.  Idempotent — a
/// repeat ``const X V`` after the first one is silently no-op in the
/// caller (Tcl 9 semantics: re-const of an already-constant name
/// succeeds without changing the value).  Linear scan over the
/// frame's records; capacity overflow drops the marker silently so
/// the runtime can't trap on a pathological proc.
pub fn frame_const_mark(name_ptr: u32, name_len: u32, hash: u32) void {
    if (frame_depth == 0) return;
    const idx = frame_depth - 1;
    const n = frame_const_count[idx];
    var i: u32 = 0;
    while (i < n) : (i += 1) {
        const r = frame_const_names[idx][i];
        if (r.hash == hash and r.name_len == name_len) {
            const a: [*]const u8 = @ptrFromInt(r.name_ptr);
            const b: [*]const u8 = @ptrFromInt(name_ptr);
            var matches = true;
            var k: u32 = 0;
            while (k < name_len) : (k += 1) {
                if (a[k] != b[k]) {
                    matches = false;
                    break;
                }
            }
            if (matches) return;
        }
    }
    if (n >= FRAME_CONST_CAP) return;
    // Copy the name bytes into an allocator-owned buffer.  The caller
    // typically passes ``obj_ensure_string(name).ptr``, which points
    // into a TclObj's borrowed string buffer; that obj is released by
    // the parser at end-of-statement (MM-B.4), invalidating the
    // pointer.  Heap-copying keeps every entry's lifetime bounded by
    // the frame itself.
    const nbuf = obj.alloc(name_len);
    if (nbuf == 0) return;
    if (name_len > 0) {
        const dst: [*]u8 = @ptrFromInt(nbuf);
        const src: [*]const u8 = @ptrFromInt(name_ptr);
        var k: u32 = 0;
        while (k < name_len) : (k += 1) dst[k] = src[k];
    }
    frame_const_names[idx][n] = .{ .name_ptr = nbuf, .name_len = name_len, .hash = hash };
    frame_const_count[idx] = n + 1;
}

/// Check whether ``name`` is flagged constant in the current frame.
pub fn frame_const_check(name_ptr: u32, name_len: u32, hash: u32) bool {
    if (frame_depth == 0) return false;
    const idx = frame_depth - 1;
    const n = frame_const_count[idx];
    var i: u32 = 0;
    while (i < n) : (i += 1) {
        const r = frame_const_names[idx][i];
        if (r.hash != hash or r.name_len != name_len) continue;
        const a: [*]const u8 = @ptrFromInt(r.name_ptr);
        const b: [*]const u8 = @ptrFromInt(name_ptr);
        var matches = true;
        var k: u32 = 0;
        while (k < name_len) : (k += 1) {
            if (a[k] != b[k]) {
                matches = false;
                break;
            }
        }
        if (matches) return true;
    }
    return false;
}

/// Per-frame visitor over the const-name list.  Used by
/// ``info consts ?pattern?`` to enumerate constants of the current
/// proc frame.  ``ctx`` is opaque; the callback receives each
/// const's (name_ptr, name_len) span.
pub fn frame_const_visit_current(
    ctx: u32,
    visit: *const fn (ctx: u32, name_ptr: u32, name_len: u32) callconv(.c) void,
) void {
    if (frame_depth == 0) return;
    const idx = frame_depth - 1;
    const n = frame_const_count[idx];
    var i: u32 = 0;
    while (i < n) : (i += 1) {
        const r = frame_const_names[idx][i];
        if (r.name_ptr == 0 or r.name_len == 0) continue;
        visit(ctx, r.name_ptr, r.name_len);
    }
}

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
        // Phase B (const): drop the frame's const-name list so the
        // next tenant of this depth doesn't inherit ``const X`` flags.
        // Each entry owns a heap-copied name buffer (see
        // ``frame_const_mark`` for the rationale); free those before
        // resetting the count so the allocator can recycle the
        // slabs.
        {
            const cur_depth = frame_depth - 1;
            const cn = frame_const_count[cur_depth];
            var ci: u32 = 0;
            while (ci < cn) : (ci += 1) {
                const r = frame_const_names[cur_depth][ci];
                if (r.name_ptr != 0 and r.name_len > 0) {
                    obj.free_sized(r.name_ptr, r.name_len);
                }
            }
            frame_const_count[cur_depth] = 0;
        }
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
    /// Phase 7 indexed-local slots.  Each slot is a TclObj handle
    /// (or 0 = unset), and the live array's slots own a +1 retain.
    /// Including the slots in the snapshot is essential for Phase 10
    /// coroutine save/restore: yielding the coro must transfer those
    /// retains into ``ctx`` so the live array can be zeroed without
    /// leaking and the resumed coro picks up its prior values.
    locals: [MAX_DEPTH][LOCALS_ARRAY_CAP]i32,
    /// Phase 6 follow-up frame-local trace chains.  Each head is a
    /// heap-allocated linked list of ``(name, ops, cmd_prefix)``
    /// records that own retains on the cmd-prefix TclObj.  Save-
    /// transfer ownership semantics apply: the snapshot inherits
    /// the chain pointers and the live slots are nulled, so a
    /// later ``frame_pop`` / ``drain_list`` on the live state
    /// can't double-release the shared chain.
    trace_heads: [MAX_DEPTH]u32,
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
        .locals = frame_locals_array,
        .trace_heads = frame_trace_heads,
    };
}

/// Phase 10: transfer-ownership variant of :func:`frame_context_save`.
/// The snapshot inherits every refcount-bearing slot the live state
/// was holding:
///
///   * ``frame_argv`` — invocation argv list (one TclObj per frame).
///   * ``frame_info`` — ``script_obj`` / ``cmd_text`` / ``proc_name``
///     TclObj handles per frame.
///   * ``frame_locals_array`` — Phase 7 indexed-slot TclObj handles.
///   * ``frame_trace_heads`` — Phase 6 follow-up per-frame variable-
///     trace chains (each carries retained ``cmd_prefix`` TclObj
///     handles).
///
/// All five live arrays are then zeroed so a subsequent ``frame_pop``
/// / ``drain_list`` / restore-into-different-state can't double-
/// release.  Used by the coroutine driver: a yielding coro snapshots
/// its frame stack with this primitive, then restores the caller's
/// previously-snapshotted context.  Both sides stay refcount-
/// balanced because the snapshots are the only place a given retain
/// lives at a time.
pub fn frame_context_save_transfer() FrameContext {
    const ctx: FrameContext = .{
        .depth = frame_depth,
        .stack = frame_stack,
        .capacity = frame_capacity,
        .ns = frame_ns,
        .argv = frame_argv,
        .info = frame_info,
        .locals = frame_locals_array,
        .trace_heads = frame_trace_heads,
    };
    // Live slots: the snapshot owns the retains now.  Zero every
    // refcount-bearing slot plus the auxiliary state (dirty mask,
    // bucket pointers) so the next ``frame_push`` for these slots
    // starts clean.
    var i: u32 = 0;
    while (i < MAX_DEPTH) : (i += 1) {
        frame_argv[i] = 0;
        frame_info[i] = .{};
        frame_ns[i] = 0;
        frame_stack[i] = 0;
        frame_capacity[i] = 0;
        frame_dirty[i] = 0;
        frame_trace_heads[i] = 0;
        var j: u32 = 0;
        while (j < LOCALS_ARRAY_CAP) : (j += 1) {
            frame_locals_array[i][j] = 0;
        }
    }
    frame_depth = 0;
    return ctx;
}

/// Phase 10: drain every live frame, releasing the retains they
/// hold on argv / FrameInfo / indexed locals / variable-trace
/// records.  Used by the coroutine driver on terminal completion
/// (RETURN / ERROR / body-end): the coro's live frames are about
/// to be discarded by the caller-context restore, so their retains
/// must be released here to avoid leaking when the snapshot in
/// ``Coro.ctx`` becomes unreachable.  Equivalent to calling
/// :func:`frame_pop` until ``frame_depth == 0``.
pub fn frame_drain_all_live() void {
    while (frame_depth > 0) {
        frame_pop();
    }
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
    frame_locals_array = ctx.locals;
    frame_trace_heads = ctx.trace_heads;
    // ``frame_dirty`` is a cache of populated buckets; reset it so
    // the next ``frame_push`` for this slot re-arms via the bitmap
    // (a stale chunk-dirty bit from the previous tenant would skip
    // a clear that's actually needed).
    var i: u32 = 0;
    while (i < MAX_DEPTH) : (i += 1) {
        frame_dirty[i] = 0;
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
var parked_stack: [MAX_DEPTH]u32 = [_]u32{0} ** MAX_DEPTH;
var parked_capacity: [MAX_DEPTH]u32 = [_]u32{0} ** MAX_DEPTH;
var parked_dirty: [MAX_DEPTH]u64 = [_]u64{0} ** MAX_DEPTH;
var parked_ns: [MAX_DEPTH]u32 = [_]u32{0} ** MAX_DEPTH;
var parked_argv: [MAX_DEPTH]i32 = [_]i32{0} ** MAX_DEPTH;
pub var parked_top: u32 = 0;
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
/// Absolute-level variant of :func:`frame_depth_stash`.  ``abs_level``
/// is the Tcl-style absolute level number (``#N``):
///   * ``0`` → global scope (clamps shift to ``frame_depth``).
///   * ``1`` → outermost proc frame (a in a→b→c).
///   * ``frame_depth`` → current frame (no shift).
/// Computes ``shift = frame_depth - abs_level`` and routes through
/// the relative-form stash so callers don't need to know the
/// runtime call depth at compile time.  ``uplevel #N body`` from
/// the WASM codegen lands here.
pub export fn frame_depth_stash_abs(abs_level: i32) i32 {
    const lvl: u32 = if (abs_level < 0) 0 else @intCast(abs_level);
    // ``lvl == frame_depth`` is the current frame: ``shift = 0``.
    // ``lvl > frame_depth`` is over-deep; treat as the current
    // frame too rather than aliasing to the global scope.
    // ``lvl == 0`` (``#0`` global) clamps to ``shift = frame_depth``
    // through the explicit subtract below.
    if (lvl >= frame_depth) {
        return frame_depth_stash(0);
    }
    return frame_depth_stash(@intCast(frame_depth - lvl));
}

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
            parked_stack[parked_top] = frame_stack[slot];
            parked_capacity[parked_top] = frame_capacity[slot];
            parked_dirty[parked_top] = frame_dirty[slot];
            parked_ns[parked_top] = frame_ns[slot];
            parked_argv[parked_top] = frame_argv[slot];
            // Clear the slot.  ``frame_argv`` is NOT released
            // here — the parked entry retains the caller's
            // refcount; restore puts the same handle back.
            frame_stack[slot] = 0;
            frame_capacity[slot] = 0;
            frame_dirty[slot] = 0;
            frame_ns[slot] = 0;
            frame_argv[slot] = 0;
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
            if (orphan_base != 0 and orphan_base != parked_stack[parked_top] and orphan_cap > 0) {
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

/// Splits an ``arr(key)`` target name into its array-name and key
/// pieces.  Returns ``null`` when the name has no balanced ``(...)``
/// suffix.  Used by the ``upvar`` array-element alias paths to route
/// reads / writes / exists / unset through the array directory rather
/// than treating ``a(0)`` as a flat scalar name.
const ArrayElemSplit = struct {
    arr_ptr: u32,
    arr_len: u32,
    key_ptr: u32,
    key_len: u32,
};

fn split_array_elem_name(name_ptr: u32, name_len: u32) ?ArrayElemSplit {
    if (name_len < 3) return null;
    // Defensive: callers occasionally hand us a TclObj whose
    // ``obj_ensure_string`` returned ``ptr=0`` with a non-zero ``len``
    // (e.g. an alias descriptor whose target was released and left
    // partially-initialised state behind), or a wildly out-of-bounds
    // address (memory-fault trap reading past the linear-memory
    // limit).  Both panic under ReleaseSafe; treat such names as
    // plain scalars so the caller falls through to the by-name slow
    // path and surfaces a regular ``no such variable`` error instead
    // of an opaque wasm trap.
    if (name_ptr == 0) return null;
    // Compute the memory ceiling in u64 so the multiplication doesn't
    // wrap when the wasm32 instance has the maximum 65536 pages
    // (4 GiB).  ``mem_pages *% PAGE_SIZE`` in u32 wraps to 0 at that
    // limit, which would make the subsequent bounds check treat
    // every pointer as out-of-range and route legitimate
    // ``arr(key)`` parses through the no-op fallback.
    const mem_pages: u32 = @intCast(@wasmMemorySize(0));
    const mem_bytes: u64 = @as(u64, mem_pages) * @as(u64, obj.PAGE_SIZE);
    if (name_ptr >= mem_bytes or @as(u64, name_len) > mem_bytes - @as(u64, name_ptr)) return null;
    const sp: [*]const u8 = @ptrFromInt(name_ptr);
    if (sp[name_len - 1] != ')') return null;
    var paren: u32 = 0;
    var found = false;
    var k: u32 = 0;
    while (k < name_len) : (k += 1) {
        if (sp[k] == '(') {
            paren = k;
            found = true;
            break;
        }
    }
    if (!found or paren == 0) return null;
    return .{
        .arr_ptr = name_ptr,
        .arr_len = paren,
        .key_ptr = name_ptr + paren + 1,
        .key_len = name_len - paren - 2,
    };
}

/// Resolve an ``upvar #0 arr(key) x`` style alias on the read side.
/// *tgt* is the target name TclObj (e.g. ``arr(key)``); the array's
/// directory entry is found via the global-scope path.  Returns 0
/// when the element doesn't exist — the caller must distinguish that
/// from "exists but empty" via the ``exists`` probe.
fn resolve_global_array_elem_get(tgt: i32, split: ArrayElemSplit) i32 {
    const tcl_array = @import("../valtypes/tcl_array.zig");
    const arr_name = obj.obj_new_string(@bitCast(split.arr_ptr), @bitCast(split.arr_len));
    defer obj.tcl_obj_release(arr_name);
    const key_obj = obj.obj_new_string(@bitCast(split.key_ptr), @bitCast(split.key_len));
    defer obj.tcl_obj_release(key_obj);
    _ = tgt; // unused — split provides the spans we need.
    return tcl_array.array_get(arr_name, key_obj);
}

fn resolve_global_array_elem_set(_: i32, split: ArrayElemSplit, value: i32) i32 {
    const tcl_array = @import("../valtypes/tcl_array.zig");
    const arr_name = obj.obj_new_string(@bitCast(split.arr_ptr), @bitCast(split.arr_len));
    defer obj.tcl_obj_release(arr_name);
    const key_obj = obj.obj_new_string(@bitCast(split.key_ptr), @bitCast(split.key_len));
    defer obj.tcl_obj_release(key_obj);
    if (value == 0) {
        _ = tcl_array.array_unset_element(arr_name, key_obj);
        return 0;
    }
    _ = tcl_array.array_set(arr_name, key_obj, value);
    return value;
}

fn resolve_global_array_elem_exists(_: i32, split: ArrayElemSplit) i32 {
    const tcl_array = @import("../valtypes/tcl_array.zig");
    const arr_name = obj.obj_new_string(@bitCast(split.arr_ptr), @bitCast(split.arr_len));
    defer obj.tcl_obj_release(arr_name);
    const key_obj = obj.obj_new_string(@bitCast(split.key_ptr), @bitCast(split.key_len));
    defer obj.tcl_obj_release(key_obj);
    return tcl_array.array_element_exists(arr_name, key_obj);
}

/// Resolve a frame-relative array element: ``upvar N arr(key) x``.  The
/// array name is resolved in the target frame's scope by mimicking
/// :func:`frame_resolve_array_name` at ``abs_depth`` — proc-local
/// arrays land under ``::__local::<abs_depth>::<arr>``.
fn resolve_frame_array_elem_qualified(arr_ptr: u32, arr_len: u32, abs_depth: u32) i32 {
    const tcl_array = @import("../valtypes/tcl_array.zig");
    const arr_name = obj.obj_new_string(@bitCast(arr_ptr), @bitCast(arr_len));
    defer obj.tcl_obj_release(arr_name);
    return tcl_array.make_local_array_obj(arr_name, abs_depth);
}

fn resolve_frame_array_elem_get(split: ArrayElemSplit, abs_depth: u32) i32 {
    const tcl_array = @import("../valtypes/tcl_array.zig");
    const qual = resolve_frame_array_elem_qualified(split.arr_ptr, split.arr_len, abs_depth);
    defer obj.tcl_obj_release(qual);
    const key_obj = obj.obj_new_string(@bitCast(split.key_ptr), @bitCast(split.key_len));
    defer obj.tcl_obj_release(key_obj);
    return tcl_array.array_get(qual, key_obj);
}

fn resolve_frame_array_elem_set(split: ArrayElemSplit, abs_depth: u32, value: i32) i32 {
    const tcl_array = @import("../valtypes/tcl_array.zig");
    const qual = resolve_frame_array_elem_qualified(split.arr_ptr, split.arr_len, abs_depth);
    defer obj.tcl_obj_release(qual);
    const key_obj = obj.obj_new_string(@bitCast(split.key_ptr), @bitCast(split.key_len));
    defer obj.tcl_obj_release(key_obj);
    if (value == 0) {
        _ = tcl_array.array_unset_element(qual, key_obj);
        return 0;
    }
    _ = tcl_array.array_set(qual, key_obj, value);
    return value;
}

fn resolve_frame_array_elem_exists(split: ArrayElemSplit, abs_depth: u32) i32 {
    const tcl_array = @import("../valtypes/tcl_array.zig");
    const qual = resolve_frame_array_elem_qualified(split.arr_ptr, split.arr_len, abs_depth);
    defer obj.tcl_obj_release(qual);
    const key_obj = obj.obj_new_string(@bitCast(split.key_ptr), @bitCast(split.key_len));
    defer obj.tcl_obj_release(key_obj);
    return tcl_array.array_element_exists(qual, key_obj);
}

/// Read the value that an ALIAS_EXT bucket points to.
fn resolve_ext_get(desc: u32, local_name: i32) i32 {
    const kind = read_i32(desc);
    const tgt = read_i32(desc + 8);
    if (kind == KIND_GLOBAL_NAMED) {
        // ``upvar #0 arr(key) local`` — the target name is an
        // array-element form; route through the array directory so
        // reads see the same storage that ``set arr(key) ...`` wrote.
        const sn = obj_ensure_string(tgt);
        if (split_array_elem_name(sn.ptr, sn.len)) |split| {
            return resolve_global_array_elem_get(tgt, split);
        }
        return globals.global_get(tgt);
    }
    if (kind == KIND_NS_VAR_PTR) {
        // tgt is an absolute *Var address.  ``var_get_scalar``
        // follows VAR_LINK chains to the terminal storage.
        return @bitCast(tcl_ns.var_get_scalar(@bitCast(tgt)));
    }
    // KIND_FRAME_VAR
    const abs: u32 = @bitCast(read_i32(desc + 4));
    const sn = obj_ensure_string(tgt);
    if (split_array_elem_name(sn.ptr, sn.len)) |split| {
        return resolve_frame_array_elem_get(split, abs);
    }
    return frame_get_at_depth(abs, tgt, local_name);
}

/// Write a value through an ALIAS_EXT bucket.
fn resolve_ext_set(desc: u32, local_name: i32, value: i32) i32 {
    // Defensive: an out-of-bounds ``desc`` would trap on the
    // ``read_i32`` below.  Bail with the input value untouched so
    // the caller observes a no-op rather than an opaque memory
    // fault.  Same idea for ``tgt`` further down.
    //
    // Compute the memory ceiling in u64 so the multiplication
    // doesn't wrap to 0 when the wasm32 instance has the maximum
    // 65536 pages (4 GiB).  The previous u32 ``*%`` form wrapped
    // there and made the guard reject every descriptor.
    const mem_pages: u32 = @intCast(@wasmMemorySize(0));
    const mem_bytes: u64 = @as(u64, mem_pages) * @as(u64, obj.PAGE_SIZE);
    if (desc == 0 or @as(u64, desc) + 12 > mem_bytes) return value;
    const kind = read_i32(desc);
    const tgt = read_i32(desc + 8);
    if (kind == KIND_GLOBAL_NAMED) {
        const sn = obj_ensure_string(tgt);
        if (split_array_elem_name(sn.ptr, sn.len)) |split| {
            return resolve_global_array_elem_set(tgt, split, value);
        }
        return globals.global_set(tgt, value);
    }
    if (kind == KIND_NS_VAR_PTR) {
        tcl_ns.var_set_scalar(@bitCast(tgt), @bitCast(value));
        return value;
    }
    if (kind != KIND_FRAME_VAR) {
        // Corrupt / freed descriptor — every kind except the three
        // above is invalid by construction.  Skip the write rather
        // than fall through to ``frame_set_at_depth`` with a
        // garbage ``abs`` / ``tgt``.
        return value;
    }
    const abs: u32 = @bitCast(read_i32(desc + 4));
    const sn = obj_ensure_string(tgt);
    if (split_array_elem_name(sn.ptr, sn.len)) |split| {
        return resolve_frame_array_elem_set(split, abs, value);
    }
    frame_set_at_depth(abs, tgt, local_name, value);
    return value;
}

/// Check existence through an ALIAS_EXT bucket.
fn resolve_ext_exists(desc: u32, local_name: i32) i32 {
    const kind = read_i32(desc);
    const tgt = read_i32(desc + 8);
    if (kind == KIND_GLOBAL_NAMED) {
        const sn = obj_ensure_string(tgt);
        if (split_array_elem_name(sn.ptr, sn.len)) |split| {
            return resolve_global_array_elem_exists(tgt, split);
        }
        return globals.global_exists(tgt);
    }
    if (kind == KIND_NS_VAR_PTR) {
        // Reference Tcl's ``info exists`` returns 1 only when the
        // variable has been *assigned* a value — declaring a
        // namespace var via ``variable name`` (no initialiser) is
        // not enough.  Without this, ``proc Default {n v} {
        // variable $n; if {![info exists $n]} { variable $n $v } }``
        // (tcltest's per-namespace lazy initialiser) skipped its
        // initialiser branch — the bare ``variable $n`` had already
        // fabricated a Var entry and the previous "tgt non-zero ⇒
        // exists" rule reported it as initialised.
        //
        // Two shapes to distinguish:
        //   * scalar var — ``var_get_scalar`` returns the stored
        //     TclObj handle (or 0 for unset).
        //   * array-shaped var (``VAR_ARRAY`` flag set) — the
        //     storage lives in the array directory keyed by the
        //     FQ name synthesised from the ``target_ns`` /
        //     ``(simple_ptr, simple_len)`` triple we stashed at
        //     ``desc+12 .. desc+24`` when ``frame_alias_ns_var``
        //     ran.  ``var_get_scalar`` deliberately returns 0 for
        //     arrays, so without the array-directory probe
        //     ``info exists arr`` after ``array set arr {...}``
        //     would falsely report 0 (Codex review on PR #343).
        if (tgt == 0) return obj_new_int(0);
        const v: *const tcl_ns.Var = @ptrFromInt(@as(u32, @bitCast(tgt)));
        if ((v.flags & tcl_ns.VAR_ARRAY) != 0) {
            const tcl_array = @import("../valtypes/tcl_array.zig");
            // Reuse ``frame_resolve_array_name`` to synthesise the
            // FQ-name TclObj — same logic, shared cleanup pattern.
            const arr_name = frame_resolve_array_name(local_name);
            const present = tcl_array.array_exists(arr_name);
            if (arr_name != local_name) obj.tcl_obj_release(arr_name);
            return present;
        }
        const val = tcl_ns.var_get_scalar(@bitCast(tgt));
        return if (val != 0) obj_new_int(1) else obj_new_int(0);
    }
    const abs: u32 = @bitCast(read_i32(desc + 4));
    const sn2 = obj_ensure_string(tgt);
    if (split_array_elem_name(sn2.ptr, sn2.len)) |split| {
        return resolve_frame_array_elem_exists(split, abs);
    }
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
    if (abs_depth == 0) {
        _ = globals.global_set(name, value);
        return;
    }
    if (frame_at_depth(abs_depth)) |base| {
        const sn = obj_ensure_string(name);
        const hash = fnv1a(sn.ptr, sn.len);
        if (frame_find(base, sn.ptr, sn.len, hash)) |bucket| {
            const v = read_i32(bucket + OFF_VALUE);
            if (v == ALIAS_GLOBAL) {
                _ = globals.global_set(name, value);
                return;
            }
            if (is_alias_ext(v)) {
                _ = resolve_ext_set(alias_desc_ptr(v), fallback_name, value);
                return;
            }
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

/// Byte length of an ALIAS_EXT descriptor by kind.  Used by the
/// re-alias path to decide whether the existing slot can be reused
/// in place, or whether the old slab must be returned to the
/// allocator's free-list before a differently-sized one is allocated.
inline fn desc_size_for_kind(kind: i32) u32 {
    return if (kind == KIND_NS_VAR_PTR) 24 else 12;
}

/// If *bucket* currently holds an ALIAS_EXT descriptor whose kind
/// differs from *new_kind* (or is held at a different size class),
/// return its slab to the allocator's free-list so the next ``alloc``
/// can recycle it.  No-op for non-ALIAS_EXT bucket values.  Without
/// this, every ``variable`` / ``upvar`` re-binding leaks a 12- or
/// 24-byte descriptor, which under workloads that re-alias inside a
/// hot loop (tcltest's option-fetch helpers, dict iteration with
/// per-iteration ``upvar``) accumulates into multi-megabyte heap
/// growth and a corresponding scan-time slowdown.
inline fn release_alias_desc_if_extant(old_v: i32) void {
    if (!is_alias_ext(old_v)) return;
    const old_desc = alias_desc_ptr(old_v);
    const old_kind = read_i32(old_desc);
    obj.free_sized(old_desc, desc_size_for_kind(old_kind));
}

/// Register *name* in the current frame as an alias to the global scope.
/// Subsequent var_set/var_resolve/var_exists calls for this name pass
/// through to the globals table.  This is how the Tcl ``global`` command
/// makes proc-local writes actually land in global storage.
pub export fn frame_alias_global(name: i32) void {
    const sn = obj_ensure_string(name);
    if (current_frame()) |base| {
        // ``global ::a::b::c`` makes the *simple* tail (``c``) the
        // local alias name and points it at the qualified
        // ``::a::b::c`` global (Tcl 9 ``Tcl_GlobalObjCmd``).  For
        // unqualified names the local == target.
        var simple_ptr = sn.ptr;
        var simple_len = sn.len;
        var has_qualifiers = false;
        if (sn.len > 0 and sn.ptr != 0) {
            const sp: [*]const u8 = @ptrFromInt(sn.ptr);
            var last_sep: i32 = -1;
            var k: u32 = 0;
            while (k + 1 < sn.len) : (k += 1) {
                if (sp[k] == ':' and sp[k + 1] == ':') {
                    last_sep = @intCast(k);
                    k += 1;
                }
            }
            if (last_sep >= 0) {
                has_qualifiers = true;
                const start: u32 = @as(u32, @intCast(last_sep)) + 2;
                simple_ptr = sn.ptr + start;
                simple_len = sn.len - start;
            }
        }
        const local_hash = fnv1a(simple_ptr, simple_len);
        if (has_qualifiers) {
            // Qualified target: route through KIND_GLOBAL_NAMED so the
            // alias reads / writes the named global var directly.
            const desc = alloc(12);
            write_i32(desc, KIND_GLOBAL_NAMED);
            write_i32(desc + 4, 0);
            write_i32(desc + 8, name);
            if (frame_find(base, simple_ptr, simple_len, local_hash)) |bucket| {
                const old = read_i32(bucket + OFF_VALUE);
                release_alias_desc_if_extant(old);
                write_i32(bucket + OFF_VALUE, encode_alias_ext(desc));
                return;
            }
            frame_insert(base, simple_ptr, simple_len, local_hash, encode_alias_ext(desc));
            return;
        }
        // Same-name alias: use ALIAS_GLOBAL sentinel so var_resolve /
        // var_set route through the root global table.
        if (frame_find(base, sn.ptr, sn.len, local_hash)) |bucket| {
            const old = read_i32(bucket + OFF_VALUE);
            release_alias_desc_if_extant(old);
            write_i32(bucket + OFF_VALUE, ALIAS_GLOBAL);
            return;
        }
        frame_insert(base, sn.ptr, sn.len, local_hash, ALIAS_GLOBAL);
    }
    // No active frame — global is already the scope, nothing to alias.
}

/// Register *local_name* in the current frame as an alias to global
/// variable *target_name* (which may differ from *local_name*).
/// Used by the interpreter's ``upvar #0 other local`` handling.
pub export fn frame_alias_named(local_name: i32, target_name: i32) void {
    if (current_frame()) |base| {
        const sn = obj_ensure_string(local_name);
        const hash = fnv1a(sn.ptr, sn.len);
        // Qualify the target with the current namespace when:
        //   * the target is unqualified (no leading ``::`` and no
        //     ``::`` separator),
        //   * we're inside a non-root namespace (the caller's
        //     namespace context is what ``upvar #0`` / ``upvar 1`` to
        //     the global frame is supposed to resolve against), and
        //   * the resulting qualified name actually exists at the
        //     namespace scope.
        // Otherwise leave ``target_name`` as-is so root-global aliases
        // still route through the flat-key store.
        const qualified_target = qualify_alias_target(target_name);
        const old_value = blk: {
            if (frame_find(base, sn.ptr, sn.len, hash)) |bucket| {
                break :blk bucket;
            }
            break :blk 0;
        };
        if (old_value != 0) {
            // Re-alias fast path: reuse the existing 12-byte slab.
            const old = read_i32(old_value + OFF_VALUE);
            if (is_alias_ext(old)) {
                const old_desc = alias_desc_ptr(old);
                const old_kind = read_i32(old_desc);
                if (old_kind != KIND_NS_VAR_PTR) {
                    write_i32(old_desc, KIND_GLOBAL_NAMED);
                    write_i32(old_desc + 4, 0);
                    write_i32(old_desc + 8, qualified_target);
                    return;
                }
                obj.free_sized(old_desc, 24);
            }
            const desc = alloc(12);
            write_i32(desc, KIND_GLOBAL_NAMED);
            write_i32(desc + 4, 0);
            write_i32(desc + 8, qualified_target);
            write_i32(old_value + OFF_VALUE, encode_alias_ext(desc));
            return;
        }
        const desc = alloc(12);
        write_i32(desc, KIND_GLOBAL_NAMED);
        write_i32(desc + 4, 0);
        write_i32(desc + 8, qualified_target);
        frame_insert(base, sn.ptr, sn.len, hash, encode_alias_ext(desc));
    }
}

/// Return *target* unchanged when already qualified, when we're at
/// root scope, or when the namespace-qualified form doesn't currently
/// exist in the global store.  Otherwise return a fresh TclObj wrapping
/// ``<current_ns>::<target>`` so the alias resolves to the namespace
/// variable instead of the root global.
fn qualify_alias_target(target: i32) i32 {
    const sn = obj_ensure_string(target);
    if (sn.len == 0 or sn.ptr == 0) return target;
    const sp: [*]const u8 = @ptrFromInt(sn.ptr);
    // Already qualified — leave alone.
    if (sn.len >= 2 and sp[0] == ':' and sp[1] == ':') return target;
    var k: u32 = 0;
    while (k + 1 < sn.len) : (k += 1) {
        if (sp[k] == ':' and sp[k + 1] == ':') return target;
    }
    // Bare name + non-root current_ns — synthesise the qualified form
    // and check the global store for an existing namespace var.
    if (tcl_ns.current_ns == 0 or tcl_ns.current_ns == tcl_ns.ns_root()) return target;
    const ns_full = tcl_ns.ns_full_name(tcl_ns.current_ns);
    if (ns_full.len <= 2) return target;
    const total: u32 = ns_full.len + 2 + sn.len;
    const buf = obj.alloc(total);
    if (buf == 0) return target;
    const dst: [*]u8 = @ptrFromInt(buf);
    const ns_p: [*]const u8 = @ptrFromInt(ns_full.ptr);
    for (0..ns_full.len) |i| dst[i] = ns_p[i];
    dst[ns_full.len] = ':';
    dst[ns_full.len + 1] = ':';
    const name_p: [*]const u8 = @ptrFromInt(sn.ptr);
    for (0..sn.len) |i| dst[ns_full.len + 2 + i] = name_p[i];
    // Respect the docstring: only redirect when the qualified
    // namespace var actually exists in root's flat-key store.
    // Otherwise leave the alias targeted at the bare name so a
    // later ``set`` lands in the root global (or the caller's
    // proc-local) rather than synthesising a never-set
    // namespace var.  The flat keys are stored without the
    // leading ``::`` (matches :func:`tcl_ns.strip_global_prefix`).
    const flat_ptr: u32 = buf + 2;
    const flat_len: u32 = total - 2;
    if (tcl_ns.ns_var_find(tcl_ns.ns_root(), flat_ptr, flat_len) == 0) {
        obj.free_sized(buf, total);
        return target;
    }
    // Take ownership of the buffer via ``obj_new_string_take`` so
    // the slab returns to the free list when the alias-target
    // TclObj is eventually released (the previous shape borrowed
    // via ``obj_new_string`` and leaked the buffer).
    return obj.obj_new_string_take(buf, total, total);
}

/// Register *local_name* in the current frame as an alias to variable
/// *target_name* in the frame at absolute depth *abs_depth* (1-indexed).
/// Used by the interpreter's ``upvar N other local`` handling.
pub export fn frame_alias_frame_var(local_name: i32, abs_depth: i32, target_name: i32) void {
    if (current_frame()) |base| {
        const sn = obj_ensure_string(local_name);
        const hash = fnv1a(sn.ptr, sn.len);
        if (frame_find(base, sn.ptr, sn.len, hash)) |bucket| {
            const old = read_i32(bucket + OFF_VALUE);
            if (is_alias_ext(old)) {
                const old_desc = alias_desc_ptr(old);
                const old_kind = read_i32(old_desc);
                if (old_kind != KIND_NS_VAR_PTR) {
                    write_i32(old_desc, KIND_FRAME_VAR);
                    write_i32(old_desc + 4, abs_depth);
                    write_i32(old_desc + 8, target_name);
                    return;
                }
                obj.free_sized(old_desc, 24);
            }
            const desc = alloc(12);
            write_i32(desc, KIND_FRAME_VAR);
            write_i32(desc + 4, abs_depth);
            write_i32(desc + 8, target_name);
            write_i32(bucket + OFF_VALUE, encode_alias_ext(desc));
            return;
        }
        const desc = alloc(12);
        write_i32(desc, KIND_FRAME_VAR);
        write_i32(desc + 4, abs_depth);
        write_i32(desc + 8, target_name);
        frame_insert(base, sn.ptr, sn.len, hash, encode_alias_ext(desc));
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
pub fn frame_alias_ns_var(local_name: i32, var_ptr: u32, target_ns: u32, simple_ptr: u32, simple_len: u32) void {
    const cur = current_frame() orelse return;
    // Descriptor layout (24 bytes — extended from the 12-byte form
    // for KIND_GLOBAL_NAMED / KIND_FRAME_VAR):
    //   desc[0..4]   = kind
    //   desc[4..8]   = unused (mirrors GLOBAL_NAMED/FRAME_VAR ``param``)
    //   desc[8..12]  = ``*Var`` for the namespace var (scalar fast path)
    //   desc[12..16] = target ``*Namespace`` address — used to look
    //                  up the cached FQ name on demand via
    //                  :func:`tcl_ns.ns_full_name`.
    //   desc[16..20] = simple-name byte pointer (interned in the
    //                  namespace's own ``Var`` table key — stable for
    //                  the namespace's lifetime).
    //   desc[20..24] = simple-name byte length.
    //
    // Storing ``target_ns`` + ``(simple_ptr, simple_len)`` separately
    // (rather than a TclObj handle to the FQ name) lets
    // :func:`frame_resolve_array_name` synthesise a fresh, owned
    // ``::ns::name`` TclObj on each call — the caller already
    // releases the result when it differs from the input, so there's
    // no descriptor-side TclObj that frame teardown would need to
    // walk and release.  Without it, traces installed against the
    // namespace array (e.g. tcltest's ``SafeFetch`` on
    // ``::tcltest::testConstraints``) never fire from inside an
    // interpreted proc body because the resolver landed on a
    // different array — string.test's ``[testConstraint valgrind]``
    // returned ``""`` and PR #341's strict-boolean ``!`` then
    // trapped.  (Codex review on PR #343.)
    const sn = obj_ensure_string(local_name);
    const hash = fnv1a(sn.ptr, sn.len);
    if (frame_find(cur, sn.ptr, sn.len, hash)) |bucket| {
        // Re-alias fast path: if the existing slot is already a
        // 24-byte KIND_NS_VAR_PTR descriptor, overwrite in place so a
        // proc body that calls ``variable foo`` repeatedly (or one
        // that re-binds via ``upvar`` after a ``variable``) doesn't
        // leak a 24-byte slab on every call.
        const old = read_i32(bucket + OFF_VALUE);
        if (is_alias_ext(old)) {
            const old_desc = alias_desc_ptr(old);
            const old_kind = read_i32(old_desc);
            if (old_kind == KIND_NS_VAR_PTR) {
                write_i32(old_desc + 4, 0);
                write_i32(old_desc + 8, @bitCast(var_ptr));
                write_i32(old_desc + 12, @bitCast(target_ns));
                write_i32(old_desc + 16, @bitCast(simple_ptr));
                write_i32(old_desc + 20, @bitCast(simple_len));
                return;
            }
            // Old descriptor was 12 bytes (KIND_GLOBAL_NAMED /
            // KIND_FRAME_VAR) — return its slab to the free-list
            // before allocating a fresh 24-byte one.
            obj.free_sized(old_desc, 12);
        }
        const desc = alloc(24);
        write_i32(desc, KIND_NS_VAR_PTR);
        write_i32(desc + 4, 0);
        write_i32(desc + 8, @bitCast(var_ptr));
        write_i32(desc + 12, @bitCast(target_ns));
        write_i32(desc + 16, @bitCast(simple_ptr));
        write_i32(desc + 20, @bitCast(simple_len));
        write_i32(bucket + OFF_VALUE, encode_alias_ext(desc));
        return;
    }
    const desc = alloc(24);
    write_i32(desc, KIND_NS_VAR_PTR);
    write_i32(desc + 4, 0);
    write_i32(desc + 8, @bitCast(var_ptr));
    write_i32(desc + 12, @bitCast(target_ns));
    write_i32(desc + 16, @bitCast(simple_ptr));
    write_i32(desc + 20, @bitCast(simple_len));
    frame_insert(cur, sn.ptr, sn.len, hash, encode_alias_ext(desc));
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
        return 0;
    }
    // No frame (frame-elided compiled proc reaching an
    // eval-fallback boundary, or a top-level script context).
    // ``local_set_silent`` falls back to ``global_set`` in the
    // same condition, so the symmetric read must consult globals
    // too — otherwise the sync/readback pair drops the value on
    // the floor and a subsequent ``$var`` read sees nothing.
    return globals.global_get(name);
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
            // No FQN match — fall through to the root global below.
            // (The strict ``namespace-17.7`` discipline of refusing
            // to fall back was correct in spirit but broke the
            // common eval-fallback path where a proc-local read
            // probes the frame, misses, falls through here, and
            // expects to keep hunting up the chain.  We re-evaluate
            // namespace-17.7 separately via a different mechanism.)
        }
    }
    // Fall through to root global (current_ns is root or not set)
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
    // At script level: an unqualified name resolves to the *current
    // namespace's* variable table (per Tcl 9 — ``set X 99`` inside
    // ``namespace eval ::A`` creates ``::A::X``).  Synthesise the
    // qualified name and route through ``global_set`` so the flat-
    // key store is used consistently with the read path's current_ns
    // probe (var_resolve / var_exists).
    if (tcl_ns.current_ns != 0 and tcl_ns.current_ns != tcl_ns.ns_root()) {
        const ns_full = tcl_ns.ns_full_name(tcl_ns.current_ns);
        if (ns_full.len > 2) {
            const total: u32 = ns_full.len + 2 + sn.len;
            const buf = obj.alloc(total);
            if (buf != 0) {
                const dst: [*]u8 = @ptrFromInt(buf);
                const ns_p: [*]const u8 = @ptrFromInt(ns_full.ptr);
                for (0..ns_full.len) |i| dst[i] = ns_p[i];
                dst[ns_full.len] = ':';
                dst[ns_full.len + 1] = ':';
                const name_p: [*]const u8 = @ptrFromInt(sn.ptr);
                for (0..sn.len) |i| dst[ns_full.len + 2 + i] = name_p[i];
                const qname = obj.obj_new_string(@bitCast(buf), @bitCast(total));
                const v = globals.global_set(qname, value);
                obj.tcl_obj_release(qname);
                obj.free_sized(buf, total);
                return v;
            }
            // OOM — fall through to the bare-name ``global_set`` so
            // we don't trap on a deref of the null buffer.  The
            // write lands in the root global slot which is the
            // least-surprising fallback if memory pressure is
            // already this tight.
        }
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
            if (sp[k] == '(') {
                paren = k;
                found = true;
                break;
            }
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
    // Tcl 9: an unqualified name at script level resolves against
    // the current namespace's variable table first, then falls
    // through to the root global (mirrors var_resolve's discipline —
    // without this, ``info exists X`` inside ``namespace eval ::A {
    // set X 99 ; info exists X }`` returns 0 even though ``$X`` reads
    // 99).  Only the bare-name case needs this — qualified names
    // already route through globals via the ``::``-prefix check below.
    if (sn.len > 0 and sn.ptr != 0) {
        const sp_probe: [*]const u8 = @ptrFromInt(sn.ptr);
        const is_qualified = sn.len >= 2 and sp_probe[0] == ':' and sp_probe[1] == ':';
        if (!is_qualified and tcl_ns.current_ns != 0) {
            const ns_full = tcl_ns.ns_full_name(tcl_ns.current_ns);
            if (ns_full.len > 2) {
                const total: u32 = ns_full.len + 2 + sn.len;
                const buf = obj.alloc(total);
                if (buf != 0) {
                    const dst: [*]u8 = @ptrFromInt(buf);
                    const ns_p: [*]const u8 = @ptrFromInt(ns_full.ptr);
                    for (0..ns_full.len) |i| dst[i] = ns_p[i];
                    dst[ns_full.len] = ':';
                    dst[ns_full.len + 1] = ':';
                    const name_p: [*]const u8 = @ptrFromInt(sn.ptr);
                    for (0..sn.len) |i| dst[ns_full.len + 2 + i] = name_p[i];
                    const qname = obj.obj_new_string(@bitCast(buf), @bitCast(total));
                    const exists = globals.global_exists(qname);
                    obj.tcl_obj_release(qname);
                    obj.free_sized(buf, total);
                    if (obj.obj_get_int(exists) != 0) return exists;
                }
                // OOM — skip the namespace probe and fall through
                // to the global existence check below.
            }
        }
    }
    // Check global (current_ns is root or not set)
    return globals.global_exists(name);
}

/// Resolve an alias to get the underlying array name for ``array names``,
/// ``array exists``, ``array size``, etc.  When the current frame has an
/// ALIAS_EXT entry for *local_name* (created by ``upvar N otherVar
/// localName``), return the target's name TclObj so array operations can
/// find the array in the global directory.  For KIND_GLOBAL_NAMED and
/// KIND_FRAME_VAR descriptors the stored ``tgt`` field IS the name; for
/// KIND_NS_VAR_PTR the name isn't available, so fall back to *local_name*.
/// When there is no alias, return *local_name* unchanged — root-global
/// canonicalisation of ``foo`` ↔ ``::foo`` happens in
/// :func:`tcl_array.normalize_ns_name` at the directory layer (strip
/// direction, matching :func:`tcl_ns.strip_global_prefix`).
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
                if (kind == KIND_GLOBAL_NAMED) {
                    // Same-name global alias — let array ops use the
                    // global directory under the bare target name.
                    return tgt;
                }
                if (kind == KIND_FRAME_VAR) {
                    // ``upvar N a x`` aliases ``x`` to the array ``a``
                    // in the frame at absolute depth ``N``.  Local
                    // arrays in that frame live under
                    // ``::__local::N::a`` in the array directory, so
                    // surface the qualified key here — without it,
                    // ``array names x`` / ``array exists x`` look up
                    // the bare ``a`` (which only matches a top-level
                    // global) and miss the proc-local storage.  See
                    // upvar-6.1 (``foreach i [array names x] { ... }``).
                    const abs: u32 = @bitCast(read_i32(desc + 4));
                    const tcl_array = @import("../valtypes/tcl_array.zig");
                    return tcl_array.make_local_array_obj(tgt, abs);
                }
                if (kind == KIND_NS_VAR_PTR) {
                    // Descriptor layout is 24 bytes for this kind:
                    // desc+12 holds the target ``*Namespace`` and
                    // desc+16/20 hold the simple-name byte span.
                    // Synthesise a fresh ``<ns_full>::<simple>`` TclObj
                    // here (caller already releases the result when it
                    // differs from the input) so array writes / reads
                    // land in the namespace's array directory instead
                    // of the proc-local synthetic key.
                    const target_ns: u32 = @bitCast(read_i32(desc + 12));
                    const simple_ptr: u32 = @bitCast(read_i32(desc + 16));
                    const simple_len: u32 = @bitCast(read_i32(desc + 20));
                    if (target_ns != 0 and simple_ptr != 0 and simple_len != 0) {
                        const ns_full = tcl_ns.ns_full_name(target_ns);
                        // Root namespace ``::`` collapses to ``::name``
                        // (no double colon).
                        const ns_emit_len: u32 = if (ns_full.len == 2) 0 else ns_full.len;
                        const total: u32 = 2 + ns_emit_len + simple_len;
                        const buf = obj.alloc(total);
                        if (buf != 0) {
                            const dst: [*]u8 = @ptrFromInt(buf);
                            var off: u32 = 0;
                            if (ns_emit_len == 0) {
                                dst[0] = ':';
                                dst[1] = ':';
                                off = 2;
                            } else {
                                const np: [*]const u8 = @ptrFromInt(ns_full.ptr);
                                for (0..ns_full.len) |k| dst[k] = np[k];
                                dst[ns_full.len] = ':';
                                dst[ns_full.len + 1] = ':';
                                off = ns_full.len + 2;
                            }
                            const sp: [*]const u8 = @ptrFromInt(simple_ptr);
                            for (0..simple_len) |k| dst[off + k] = sp[k];
                            return obj.obj_new_string_take(buf, total, total);
                        }
                    }
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

/// Bucket header offsets — matches the ``HEADER_SIZE = 12`` layout in
/// :file:`valtypes/hash_table.zig`: ``[name_ptr, name_len, hash]``
/// followed by the per-table value at ``OFF_VALUE``.
const BUCKET_NAME_PTR: u32 = 0;
const BUCKET_NAME_LEN: u32 = 4;

/// Visitor callback for :func:`frame_locals_visit_current`.  Receives
/// the bucket's name span and current value.  The implementation
/// guarantees the bucket is live (non-zero name, non-tombstone) before
/// the call; the visitor decides whether the entry counts as
/// "existing" — e.g. ``info vars`` skips ``value == 0`` entries to
/// match Tcl's ``unset``-leaves-the-slot-empty semantics.
pub const FrameLocalVisitor = fn (ctx: u32, name_ptr: u32, name_len: u32, value: i32) callconv(.c) void;

/// Walk the current frame's hash table, invoking *visit* for every
/// occupied bucket.  No-op when called outside a proc frame.  Used by
/// ``info vars`` / ``info locals`` to enumerate proc-local names; the
/// visitor receives the raw bucket ``value`` (TclObj handle, 0 for
/// unset, ALIAS_GLOBAL or ALIAS_EXT for upvar/global aliases) so the
/// caller can decide whether to count ``unset``-cleared slots.
pub fn frame_locals_visit_current(ctx: u32, visit: *const FrameLocalVisitor) void {
    const base = current_frame() orelse return;
    const cap = capacity_for_base(base);
    var i: u32 = 0;
    while (i < cap) : (i += 1) {
        const bucket = base + i * FRAME_BUCKET_SIZE;
        const name_ptr: u32 = @bitCast(read_i32(bucket + BUCKET_NAME_PTR));
        if (name_ptr == 0 or name_ptr == ht.TOMBSTONE) continue;
        const name_len: u32 = @bitCast(read_i32(bucket + BUCKET_NAME_LEN));
        if (name_len == 0) continue;
        const value = read_i32(bucket + OFF_VALUE);
        visit(ctx, name_ptr, name_len, value);
    }
}

/// Probe whether an alias bucket's ALIAS_GLOBAL / ALIAS_EXT target
/// currently has a stored value.  Returns true when the alias chain
/// terminates in a live (non-zero) variable; ``info vars`` uses this
/// to surface upvar locals that point at an existing target while
/// hiding aliases whose targets have been ``unset``.
pub fn frame_alias_target_exists(value: i32, name_ptr: u32, name_len: u32) bool {
    if (value == ALIAS_GLOBAL) {
        // Same-name global alias: probe the global table directly.
        const name_obj = obj.obj_new_string(@bitCast(name_ptr), @bitCast(name_len));
        defer obj.tcl_obj_release(name_obj);
        return obj_get_int(globals.global_exists(name_obj)) != 0;
    }
    if (!is_alias_ext(value)) return false;
    const name_obj = obj.obj_new_string(@bitCast(name_ptr), @bitCast(name_len));
    defer obj.tcl_obj_release(name_obj);
    return obj_get_int(resolve_ext_exists(alias_desc_ptr(value), name_obj)) != 0;
}
