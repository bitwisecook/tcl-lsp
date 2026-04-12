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

const obj = @import("tcl_obj.zig");
const alloc = obj.alloc;
const memcpy = obj.memcpy;
const read_i32 = obj.read_i32;
const write_i32 = obj.write_i32;
const obj_ensure_string = obj.obj_ensure_string;
const obj_new_int = obj.obj_new_int;

const globals = @import("tcl_globals.zig");
const fnv1a = globals.fnv1a;

// -- Frame layout --
// Each frame is a contiguous block in linear memory:
//   [16 buckets * 16 bytes] = 256 bytes per frame
// Bucket: [name_ptr:4 | name_len:4 | hash:4 | value:4] = 16 bytes
// Same layout as globals for code reuse.

const FRAME_BUCKET_SIZE: u32 = 16;
// Per-frame hash table capacity.  Procs with more than this many distinct
// local names are rare but possible; insertion is bounded and traps on
// overflow rather than looping.  If this limit is hit in practice, bump
// here or switch to a growable per-frame hash table (mirroring globals).
const FRAME_BUCKET_COUNT: u32 = 64; // per frame, power of 2
const FRAME_SIZE: u32 = FRAME_BUCKET_COUNT * FRAME_BUCKET_SIZE; // 1024 bytes

/// Sentinel stored in the value field to mark a bucket as a global alias
/// rather than a literal TclObj pointer.  A valid TclObj pointer is always
/// in the high-memory region (heap_ptr >= 65536) and is word-aligned, so
/// -1 (0xFFFFFFFF) cannot collide with a real pointer.  When a var lookup
/// finds this marker, reads/writes pass through to the globals table.
const ALIAS_GLOBAL: i32 = -1;

const MAX_DEPTH: u32 = 64;

// Frame stack — array of frame buffer pointers
var frame_stack: [MAX_DEPTH]u32 = [_]u32{0} ** MAX_DEPTH;
var frame_depth: u32 = 0;

// -- Frame operations --

/// Push a new call frame. Returns the frame index.
pub export fn frame_push() i32 {
    if (frame_depth >= MAX_DEPTH) return -1; // stack overflow
    const idx = frame_depth;
    if (frame_stack[idx] == 0) {
        // Allocate frame on first use
        frame_stack[idx] = alloc(FRAME_SIZE);
    }
    // Zero out all buckets
    const base = frame_stack[idx];
    var i: u32 = 0;
    while (i < FRAME_BUCKET_COUNT) : (i += 1) {
        write_i32(base + i * FRAME_BUCKET_SIZE, 0); // name_ptr = 0 means empty
    }
    frame_depth += 1;
    return @intCast(idx);
}

/// Pop the current call frame. Caller should not access frame locals after this.
pub export fn frame_pop() void {
    if (frame_depth > 0) frame_depth -= 1;
}

/// Get current frame depth (0 = global scope, no frames pushed).
pub export fn frame_get_depth() i32 {
    return @intCast(frame_depth);
}

// -- Local variable operations on current frame --

fn current_frame() ?u32 {
    if (frame_depth == 0) return null;
    return frame_stack[frame_depth - 1];
}

fn frame_find(base: u32, name_ptr: u32, name_len: u32, hash: u32) ?u32 {
    const mask = FRAME_BUCKET_COUNT - 1;
    var idx = hash & mask;
    var probes: u32 = 0;
    while (probes < FRAME_BUCKET_COUNT) : (probes += 1) {
        const bucket = base + idx * FRAME_BUCKET_SIZE;
        const ep: u32 = @intCast(read_i32(bucket));
        if (ep == 0) return null; // empty slot
        const el: u32 = @intCast(read_i32(bucket + 4));
        const eh: u32 = @intCast(read_i32(bucket + 8));
        if (eh == hash and el == name_len) {
            const sp: [*]const u8 = @ptrFromInt(ep);
            const np: [*]const u8 = @ptrFromInt(name_ptr);
            var match = true;
            for (0..el) |k| {
                if (sp[k] != np[k]) {
                    match = false;
                    break;
                }
            }
            if (match) return bucket;
        }
        idx = (idx + 1) & mask;
    }
    return null;
}

fn frame_insert(base: u32, name_ptr: u32, name_len: u32, hash: u32, value: i32) void {
    const mask = FRAME_BUCKET_COUNT - 1;
    var idx = hash & mask;
    var probes: u32 = 0;
    while (probes < FRAME_BUCKET_COUNT) : (probes += 1) {
        const bucket = base + idx * FRAME_BUCKET_SIZE;
        const ep: u32 = @intCast(read_i32(bucket));
        if (ep == 0) {
            // Copy name to heap (frame outlives the source script potentially)
            const nbuf = alloc(name_len);
            memcpy(nbuf, name_ptr, name_len);
            write_i32(bucket, @intCast(nbuf));
            write_i32(bucket + 4, @intCast(name_len));
            write_i32(bucket + 8, @intCast(hash));
            write_i32(bucket + 12, value);
            return;
        }
        idx = (idx + 1) & mask;
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
    const io = @import("tcl_io.zig");
    io.fd_write_all(2, msg.ptr, @intCast(msg.len));
    io.fd_write_all(2, "\n", 1);
}

/// Register *name* in the current frame as an alias to the global scope.
/// Subsequent var_set/var_resolve/var_exists calls for this name pass
/// through to the globals table.  This is how the Tcl ``global`` command
/// makes proc-local writes actually land in global storage.
pub export fn frame_alias_global(name: i32) void {
    const sn = obj_ensure_string(name);
    if (current_frame()) |base| {
        const hash = fnv1a(sn.ptr, sn.len);
        if (frame_find(base, sn.ptr, sn.len, hash)) |bucket| {
            write_i32(bucket + 12, ALIAS_GLOBAL);
            return;
        }
        frame_insert(base, sn.ptr, sn.len, hash, ALIAS_GLOBAL);
    }
    // No active frame — global is already the scope, nothing to alias.
}

/// Set a local variable in the current frame.
/// If no frame is active, falls through to global_set.
/// If the name is an alias to a global, writes to the global table.
pub export fn local_set(name: i32, value: i32) i32 {
    const sn = obj_ensure_string(name);
    if (current_frame()) |base| {
        const hash = fnv1a(sn.ptr, sn.len);
        if (frame_find(base, sn.ptr, sn.len, hash)) |bucket| {
            if (read_i32(bucket + 12) == ALIAS_GLOBAL) {
                return globals.global_set(name, value);
            }
            write_i32(bucket + 12, value);
            return value;
        }
        frame_insert(base, sn.ptr, sn.len, hash, value);
        return value;
    }
    // No frame active — set global
    return globals.global_set(name, value);
}

/// Get a local variable from the current frame.
/// Returns 0 if not found in current frame (does NOT fall through to globals).
/// If the name is an alias to a global, reads from the global table.
pub export fn local_get(name: i32) i32 {
    const sn = obj_ensure_string(name);
    if (current_frame()) |base| {
        const hash = fnv1a(sn.ptr, sn.len);
        if (frame_find(base, sn.ptr, sn.len, hash)) |bucket| {
            const v = read_i32(bucket + 12);
            if (v == ALIAS_GLOBAL) return globals.global_get(name);
            return v;
        }
    }
    return 0;
}

/// Check if a local variable exists in the current frame.
/// An alias-to-global bucket counts as a local for this check only when
/// the underlying global is actually set.
pub export fn local_exists(name: i32) i32 {
    const sn = obj_ensure_string(name);
    if (current_frame()) |base| {
        const hash = fnv1a(sn.ptr, sn.len);
        if (frame_find(base, sn.ptr, sn.len, hash)) |bucket| {
            if (read_i32(bucket + 12) == ALIAS_GLOBAL) {
                return globals.global_exists(name);
            }
            return obj_new_int(1);
        }
    }
    return obj_new_int(0);
}

/// Resolve a variable: check current frame first, then globals.
/// This is the standard Tcl lookup order for the interpreter.
pub export fn var_resolve(name: i32) i32 {
    const sn = obj_ensure_string(name);
    if (current_frame()) |base| {
        const hash = fnv1a(sn.ptr, sn.len);
        if (frame_find(base, sn.ptr, sn.len, hash)) |bucket| {
            const v = read_i32(bucket + 12);
            if (v == ALIAS_GLOBAL) return globals.global_get(name);
            return v;
        }
    }
    // Fall through to global
    return globals.global_get(name);
}

/// Set a variable: sets in current frame if one is active, otherwise global.
/// If the local is an alias to a global, the write propagates to globals.
pub export fn var_set(name: i32, value: i32) i32 {
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
            if (read_i32(bucket + 12) == ALIAS_GLOBAL) {
                return globals.global_exists(name);
            }
            return obj_new_int(1);
        }
    }
    // Check global
    return globals.global_exists(name);
}
