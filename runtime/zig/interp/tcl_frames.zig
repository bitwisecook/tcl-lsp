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
// Per-frame hash table capacity.  Procs with more than this many distinct
// local names are rare but possible; insertion is bounded and traps on
// overflow rather than looping.  If this limit is hit in practice, bump
// here or switch to a growable per-frame hash table (mirroring globals).
// Per-frame hash table capacity.  Must be a power of 2.  Was 64;
// bumped to 256 because tcltest's ``test`` proc (which is uplevel'd
// by ``RunTest``) accumulates over 100 distinct locals once every
// tcltest option variable is read — 64 and 128 both overflowed the
// open-addressing probe chain and traced to ``frame_insert``
// raising "frame local table full".  256 × 16 B = 4 KB per frame
// × 64 max depth = 256 KB total frame memory — acceptable for the
// web container's bump allocator.  When growable per-frame tables
// land this cap can move back down; the trap is load-bearing so
// callers notice rather than silently looping.
const FRAME_BUCKET_COUNT: u32 = 256;
const FRAME_SIZE: u32 = FRAME_BUCKET_COUNT * FRAME_BUCKET_SIZE; // 4096 bytes
const OFF_VALUE: u32 = ht.HEADER_SIZE; // 12 — value follows header

const FrameTable = ht.Table(FRAME_BUCKET_SIZE);

/// Construct a transient view of the per-frame buffer as a fixed-
/// capacity hash table.  Frames are pre-allocated in ``frame_push``
/// (not ``Table.init``-allocated), so the view's ``count`` field is
/// not authoritative — we never call ``grow`` / ``needs_grow`` on
/// these tables.
inline fn frame_table(base: u32) FrameTable {
    return .{ .buf = base, .cap = FRAME_BUCKET_COUNT, .count = 0 };
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
pub var frame_depth: u32 = 0;

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
// Frames whose overall capacity exceeds 32 chunks (4 KB) would need
// a wider mask; today FRAME_SIZE = 4096 bytes = 32 chunks, so a
// single u32 covers the entire frame.
const FRAME_CHUNK_BYTES: u32 = 128;
const FRAME_CHUNKS: u32 = FRAME_SIZE / FRAME_CHUNK_BYTES; // 32 with current sizing
var frame_dirty: [MAX_DEPTH]u32 = [_]u32{0} ** MAX_DEPTH;

/// Mark the chunk containing ``bucket_offset`` (a byte offset within
/// the frame buffer, NOT an absolute address) as dirty so the next
/// ``frame_push`` for this slot clears it.
pub inline fn mark_dirty(idx: u32, bucket_offset: u32) void {
    const chunk = bucket_offset / FRAME_CHUNK_BYTES;
    if (chunk < FRAME_CHUNKS) {
        frame_dirty[idx] |= @as(u32, 1) << @intCast(chunk);
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
        // Allocate frame on first use
        frame_stack[idx] = alloc(FRAME_SIZE);
        first_use = true;
    }
    const base = frame_stack[idx];
    if (first_use) {
        // Brand-new buffer from the bump allocator — Zig zero-fills
        // pages on grow but a recycled OBJ_SIZE slab from the size-
        // class free-list won't be (and our frame size doesn't
        // currently come from a free-list, but be defensive).
        const slice: [*]u8 = @ptrFromInt(base);
        @memset(slice[0..FRAME_SIZE], 0);
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
                const slice: [*]u8 = @ptrFromInt(base + off);
                @memset(slice[0..FRAME_CHUNK_BYTES], 0);
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

/// Pop the current call frame. Caller should not access frame locals after this.
pub export fn frame_pop() void {
    if (frame_depth > 0) {
        frame_depth -= 1;
        // MM-B.5: release the argv reference we retained in
        // frame_set_argv before clearing the slot.
        const old = frame_argv[frame_depth];
        frame_argv[frame_depth] = 0;
        if (old != 0) obj.tcl_obj_release(old);
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

/// Save the current frame depth and decrement it by *relative_up*
/// (clamped to 0).  Returns the saved depth so the caller can
/// restore it via ``frame_depth_restore``.  Used by ``uplevel`` to
/// temporarily act as if we're running *relative_up* frames above
/// the current one.
pub export fn frame_depth_stash(relative_up: i32) i32 {
    const saved: i32 = @intCast(frame_depth);
    var up = relative_up;
    if (up < 0) up = 0;
    const u: u32 = @intCast(up);
    if (u >= frame_depth) {
        frame_depth = 0;
    } else {
        frame_depth -= u;
    }
    return saved;
}

/// Restore frame_depth to the value returned by an earlier
/// ``frame_depth_stash``.
pub export fn frame_depth_restore(saved: i32) void {
    if (saved < 0) {
        frame_depth = 0;
    } else {
        frame_depth = @intCast(saved);
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
    var t = frame_table(base);
    if (t.try_insert_header(name_ptr, name_len, hash)) |bucket| {
        write_i32(bucket + OFF_VALUE, value);
        // Mark the chunk this bucket lives in as dirty so the next
        // ``frame_push`` for this slot clears it.  Overwrites to the
        // same bucket via ``frame_find`` + ``write_i32`` later don't
        // need a separate ``mark_dirty`` call — once a chunk's bit
        // is set, subsequent writes within the same chunk are
        // already covered.
        if (frame_depth > 0) {
            const idx = frame_depth - 1;
            if (frame_stack[idx] == base) {
                mark_dirty(idx, bucket - base);
            }
        }
        return;
    }
    // Frame full — emit a clear diagnostic and trap.  Previously this
    // looped forever; now we fail loudly so the limit is discoverable.
    const errmsg = "frame local table full";
    rt_fd_write_stderr(errmsg);
    @trap();
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
            return value;
        }
        // Fresh insert — retain the new value (no old to release).
        if (value != 0) obj.tcl_obj_retain(value);
        frame_insert(base, sn.ptr, sn.len, hash, value);
        return value;
    }
    // No frame active — set global (var_set_scalar handles
    // refcount on its end via MM-B.2).
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
            return v;
        }
    }
    return 0;
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
    const sn = obj_ensure_string(name);
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
            if (obj.obj_get_int(globals.global_exists(qname)) != 0) {
                return globals.global_get(qname);
            }
        }
    }
    // Fall through to root global
    return globals.global_get(name);
}

/// Set a variable: sets in current frame if one is active, otherwise global.
/// If the local is an alias to a global, the write propagates to globals.
pub export fn var_set(name: i32, value: i32) i32 {
    // ``::``-qualified names are always global, regardless of frame
    // depth — matches Tcl's namespace resolution where an absolute
    // name bypasses all local scopes.
    const sn = obj_ensure_string(name);
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
