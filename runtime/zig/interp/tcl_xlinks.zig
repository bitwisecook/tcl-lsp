// Phase 9 of the var/frame/ns/exception architecture refactor:
// cross-interpreter variable links.
//
// A ``Variable`` in interp A can be transparently aliased to a
// variable in interp B via the link registry below.  When
// ``var_resolve`` / ``var_set`` in interp A see a name with a
// matching link, the lookup forwards to interp B's table for the
// link's target name.  ``transfer_result`` (in tcl_interp_registry)
// handles the refcount-bookkeeping required to ferry a TclObj across
// the boundary; this module is the dispatcher's pre-flight check.
//
// Storage: a single hash table keyed by ``(local_interp_addr,
// canonical_name)``.  Most variables are not linked, so we don't
// pay a per-bucket field — we keep the link table on the side and
// only consult it for the pre-flight in :file:`tcl_frames.zig`.
//
// The doc records two design alternatives for Phase 9: (a) the
// per-name ``Variable``-record refactor that the design's Phase 1
// ideal calls for, and (b) a contained side-registry.  We pick (b):
// the side registry adds one lookup path on the variable read/write
// hot path, but it doesn't churn every existing storage site that
// the synthetic ``::__local::N::*`` namespace already covers.
//
// Re-entrancy: a link target whose dispatch resolves back to the
// caller interp would loop.  We guard via a small "currently
// resolving" stack — any second probe of the same (interp, name) in
// the same call stack drops to "no link" and lets the normal
// resolution path run.

const obj = @import("../valtypes/tcl_obj.zig");
const ht = @import("../valtypes/hash_table.zig");
const fnv1a = ht.fnv1a;
const interp_reg = @import("tcl_interp_registry.zig");

const alloc = obj.alloc;
const memcpy = obj.memcpy;
const read_i32 = obj.read_i32;
const write_i32 = obj.write_i32;
const obj_ensure_string = obj.obj_ensure_string;
const obj_new_string_copy = obj.obj_new_string_copy;
const tcl_obj_retain = obj.tcl_obj_retain;
const tcl_obj_release = obj.tcl_obj_release;

/// Bucket layout (16 bytes):
///   [0..4]   local_name_ptr: u32  (heap-owned name copy, 0 = empty bucket)
///   [4..8]   local_name_len: u32
///   [8..12]  local_interp:   u32  (Interp* address; the link's owning interp)
///   [12..16] target_handle:  u32  (offset into ``targets`` for the link record)
const BUCKET_SIZE: u32 = 16;
const OFF_NAME_PTR: u32 = 0;
const OFF_NAME_LEN: u32 = 4;
const OFF_LOCAL_INTERP: u32 = 8;
const OFF_TARGET_HANDLE: u32 = 12;

const INITIAL_CAP: u32 = 16;

/// Target record stored in the parallel ``targets`` slab so each
/// bucket holds a fixed-size offset rather than a variable-length
/// payload.
///
///   [0..4]   target_interp: u32  (Interp* of the destination)
///   [4..8]   target_name:   i32  (TclObj handle, retained)
const TARGET_REC_SIZE: u32 = 8;
const T_OFF_INTERP: u32 = 0;
const T_OFF_NAME: u32 = 4;

var dir_buf: u32 = 0;
var dir_cap: u32 = 0;
var dir_count: u32 = 0;

// Re-entrancy guard.  Tcl's ``interp share-variable`` accepts cycles
// (A → B → A) at install time but a runtime probe must not infinite-
// loop.  Walks of length up to MAX_REENTRY are silently broken at
// the next probe, falling through to local resolution.
const MAX_REENTRY: u32 = 8;
var reentry_stack: [MAX_REENTRY]u64 = [_]u64{0} ** MAX_REENTRY;
var reentry_top: u32 = 0;

fn pack_key(local_interp: u32, name_hash: u32) u64 {
    return (@as(u64, local_interp) << 32) | @as(u64, name_hash);
}

fn reentry_push(local_interp: u32, name_hash: u32) bool {
    const k = pack_key(local_interp, name_hash);
    var i: u32 = 0;
    while (i < reentry_top) : (i += 1) {
        if (reentry_stack[i] == k) return false; // cycle — refuse the probe
    }
    if (reentry_top >= MAX_REENTRY) return false;
    reentry_stack[reentry_top] = k;
    reentry_top += 1;
    return true;
}

fn reentry_pop() void {
    if (reentry_top > 0) reentry_top -= 1;
}

fn dir_init() void {
    if (dir_cap != 0) return;
    const buf = alloc(INITIAL_CAP * BUCKET_SIZE);
    if (buf == 0) return;
    dir_buf = buf;
    dir_cap = INITIAL_CAP;
    var i: u32 = 0;
    while (i < dir_cap) : (i += 1) {
        write_i32(dir_buf + i * BUCKET_SIZE, 0);
    }
}

fn dir_find(local_interp: u32, name_ptr: u32, name_len: u32, hash: u32) ?u32 {
    if (dir_buf == 0) return null;
    const mask = dir_cap - 1;
    var idx = hash & mask;
    var probes: u32 = 0;
    while (probes < dir_cap) : (probes += 1) {
        const bucket = dir_buf + idx * BUCKET_SIZE;
        const np: u32 = @bitCast(read_i32(bucket + OFF_NAME_PTR));
        if (np == 0) return null;
        const nl: u32 = @bitCast(read_i32(bucket + OFF_NAME_LEN));
        const li: u32 = @bitCast(read_i32(bucket + OFF_LOCAL_INTERP));
        if (li == local_interp and nl == name_len) {
            const sp: [*]const u8 = @ptrFromInt(np);
            const np2: [*]const u8 = @ptrFromInt(name_ptr);
            var match = true;
            for (0..nl) |k| {
                if (sp[k] != np2[k]) {
                    match = false;
                    break;
                }
            }
            if (match) return bucket;
        }
        idx = (idx + 1) & mask;
        probes += 1;
    }
    return null;
}

fn dir_grow() void {
    const old_buf = dir_buf;
    const old_cap = dir_cap;
    const new_cap = dir_cap * 2;
    const new_buf = alloc(new_cap * BUCKET_SIZE);
    if (new_buf == 0) return;
    dir_cap = new_cap;
    dir_buf = new_buf;
    var i: u32 = 0;
    while (i < dir_cap) : (i += 1) {
        write_i32(dir_buf + i * BUCKET_SIZE, 0);
    }
    dir_count = 0;
    var j: u32 = 0;
    while (j < old_cap) : (j += 1) {
        const bucket = old_buf + j * BUCKET_SIZE;
        const np: u32 = @bitCast(read_i32(bucket + OFF_NAME_PTR));
        if (np == 0) continue;
        const nl: u32 = @bitCast(read_i32(bucket + OFF_NAME_LEN));
        const li: u32 = @bitCast(read_i32(bucket + OFF_LOCAL_INTERP));
        const th: u32 = @bitCast(read_i32(bucket + OFF_TARGET_HANDLE));
        const hash = fnv1a(np, nl);
        dir_insert_known(li, np, nl, hash, th);
    }
}

fn dir_insert_known(local_interp: u32, name_ptr: u32, name_len: u32, hash: u32, target_handle: u32) void {
    dir_init();
    if (dir_count * 4 >= dir_cap * 3) dir_grow();
    if (dir_buf == 0) return;
    const mask = dir_cap - 1;
    var idx = hash & mask;
    while (true) {
        const bucket = dir_buf + idx * BUCKET_SIZE;
        const np: u32 = @bitCast(read_i32(bucket + OFF_NAME_PTR));
        if (np == 0) {
            write_i32(bucket + OFF_NAME_PTR, @bitCast(name_ptr));
            write_i32(bucket + OFF_NAME_LEN, @bitCast(name_len));
            write_i32(bucket + OFF_LOCAL_INTERP, @bitCast(local_interp));
            write_i32(bucket + OFF_TARGET_HANDLE, @bitCast(target_handle));
            dir_count += 1;
            return;
        }
        idx = (idx + 1) & mask;
    }
}

/// Install a link from ``(local_interp, local_name)`` to
/// ``(target_interp, target_name)``.  Idempotent: re-installing the
/// same key updates the target.  Returns ``true`` on success;
/// ``false`` on OOM (caller surfaces the diagnostic).
pub fn install(
    local_interp: u32,
    local_name: i32,
    target_interp: u32,
    target_name: i32,
) bool {
    const sn = obj_ensure_string(local_name);
    if (sn.len == 0 or sn.ptr == 0) return false;
    const hash = fnv1a(sn.ptr, sn.len);
    if (dir_find(local_interp, sn.ptr, sn.len, hash)) |bucket| {
        // Update existing entry — release old target name, retain
        // the new one.
        const th: u32 = @bitCast(read_i32(bucket + OFF_TARGET_HANDLE));
        const old_name = read_i32(th + T_OFF_NAME);
        write_i32(th + T_OFF_INTERP, @bitCast(target_interp));
        if (target_name != 0) tcl_obj_retain(target_name);
        write_i32(th + T_OFF_NAME, target_name);
        if (old_name != 0 and old_name != target_name) tcl_obj_release(old_name);
        return true;
    }
    // Allocate a fresh target record.
    const th = alloc(TARGET_REC_SIZE);
    if (th == 0) return false;
    write_i32(th + T_OFF_INTERP, @bitCast(target_interp));
    if (target_name != 0) tcl_obj_retain(target_name);
    write_i32(th + T_OFF_NAME, target_name);
    // Copy the local name into a stable buffer.
    const nbuf = alloc(sn.len);
    if (nbuf == 0) {
        if (target_name != 0) tcl_obj_release(target_name);
        obj.free_sized(th, TARGET_REC_SIZE);
        return false;
    }
    memcpy(nbuf, sn.ptr, sn.len);
    dir_insert_known(local_interp, nbuf, sn.len, hash, th);
    return true;
}

/// Remove the link for ``(local_interp, local_name)`` if present.
/// Returns ``true`` on hit, ``false`` if no link existed.
pub fn remove(local_interp: u32, local_name: i32) bool {
    const sn = obj_ensure_string(local_name);
    if (sn.len == 0 or sn.ptr == 0) return false;
    const hash = fnv1a(sn.ptr, sn.len);
    const bucket = dir_find(local_interp, sn.ptr, sn.len, hash) orelse return false;
    const np: u32 = @bitCast(read_i32(bucket + OFF_NAME_PTR));
    const nl: u32 = @bitCast(read_i32(bucket + OFF_NAME_LEN));
    const th: u32 = @bitCast(read_i32(bucket + OFF_TARGET_HANDLE));
    const tn = read_i32(th + T_OFF_NAME);
    if (tn != 0) tcl_obj_release(tn);
    obj.free_sized(th, TARGET_REC_SIZE);
    if (np != 0 and nl != 0) obj.free_sized(np, nl);
    write_i32(bucket + OFF_NAME_PTR, 0);
    write_i32(bucket + OFF_NAME_LEN, 0);
    write_i32(bucket + OFF_LOCAL_INTERP, 0);
    write_i32(bucket + OFF_TARGET_HANDLE, 0);
    if (dir_count > 0) dir_count -= 1;
    return true;
}

/// Lookup result: ``found = false`` when no link exists or the
/// re-entry guard refused the probe.
pub const Lookup = struct {
    found: bool,
    target_interp: u32,
    target_name: i32,
};

/// Probe for a link on ``(local_interp, local_name)``.  When found,
/// returns the target interp address and the target name TclObj
/// (BORROWED — caller must not release).  When the re-entry guard
/// fires (cycle detected) the lookup returns ``found=false`` so the
/// caller resolves the name locally.
pub fn lookup(local_interp: u32, local_name: i32) Lookup {
    const sn = obj_ensure_string(local_name);
    if (sn.len == 0 or sn.ptr == 0) return .{ .found = false, .target_interp = 0, .target_name = 0 };
    const hash = fnv1a(sn.ptr, sn.len);
    const bucket = dir_find(local_interp, sn.ptr, sn.len, hash) orelse {
        return .{ .found = false, .target_interp = 0, .target_name = 0 };
    };
    if (!reentry_push(local_interp, hash)) {
        return .{ .found = false, .target_interp = 0, .target_name = 0 };
    }
    const th: u32 = @bitCast(read_i32(bucket + OFF_TARGET_HANDLE));
    const ti: u32 = @bitCast(read_i32(th + T_OFF_INTERP));
    const tn = read_i32(th + T_OFF_NAME);
    return .{ .found = true, .target_interp = ti, .target_name = tn };
}

/// Counterpart of :func:`lookup` — pops the re-entry guard frame.
/// Callers MUST invoke this after they finish dispatching to the
/// target interp's resolution.  Pairs with the ``found = true``
/// case only; the ``found = false`` case left no frame to pop.
pub fn lookup_done() void {
    reentry_pop();
}

/// Drop every link whose ``local_interp`` or ``target_interp``
/// equals ``deleted_interp``.  Called by ``interp delete`` so a
/// dangling link doesn't leak past its endpoint's lifetime.  Walks
/// the whole table — O(N) in the link count, run once per delete
/// so the cost is acceptable.
pub fn drop_for_interp(deleted_interp: u32) void {
    if (dir_buf == 0) return;
    var idx: u32 = 0;
    while (idx < dir_cap) : (idx += 1) {
        const bucket = dir_buf + idx * BUCKET_SIZE;
        const np: u32 = @bitCast(read_i32(bucket + OFF_NAME_PTR));
        if (np == 0) continue;
        const li: u32 = @bitCast(read_i32(bucket + OFF_LOCAL_INTERP));
        const th: u32 = @bitCast(read_i32(bucket + OFF_TARGET_HANDLE));
        const ti: u32 = @bitCast(read_i32(th + T_OFF_INTERP));
        if (li == deleted_interp or ti == deleted_interp) {
            const nl: u32 = @bitCast(read_i32(bucket + OFF_NAME_LEN));
            const tn = read_i32(th + T_OFF_NAME);
            if (tn != 0) tcl_obj_release(tn);
            obj.free_sized(th, TARGET_REC_SIZE);
            if (np != 0 and nl != 0) obj.free_sized(np, nl);
            write_i32(bucket + OFF_NAME_PTR, 0);
            write_i32(bucket + OFF_NAME_LEN, 0);
            write_i32(bucket + OFF_LOCAL_INTERP, 0);
            write_i32(bucket + OFF_TARGET_HANDLE, 0);
            if (dir_count > 0) dir_count -= 1;
        }
    }
}

// Suppress unused-import warnings for tcl_obj_release / interp_reg
// when the file is read in isolation (no consumer in test builds).
test "compile-only" {
    _ = &tcl_obj_release;
    _ = &interp_reg;
    _ = &obj_new_string_copy;
}
