// Tcl array operations — dedicated per-array hash table storage.
//
// Tcl arrays are associative maps distinct from scalars and dicts.  An
// array named "a" is addressed as ``a(key)``: ``set a(1) foo``,
// ``$a(1)``, ``info exists a(1)``, ``array exists a``, ``array names
// a``, ``array size a``, ``array unset a``.  Arrays and scalars of the
// same name cannot coexist (``set a 1; set a(1) 2`` is a runtime
// error in real Tcl; we diverge here and silently shadow to keep the
// compiled sandbox small).
//
// Storage layout:
//   Top-level "array directory": a growing open-addressing hash table
//   keyed by array name → pointer to the array's own hash table.  Each
//   per-array table is again open-addressing keyed by string key →
//   TclObj value.  Both tables share layout with tcl_globals so we
//   only need one hasher (FNV-1a).
//
// Memory: the bump allocator (tcl_obj.alloc) owns everything.  We
// never free — small enough for batch-style scripts.

const obj = @import("tcl_obj.zig");
const alloc = obj.alloc;
const memcpy = obj.memcpy;
const read_i32 = obj.read_i32;
const write_i32 = obj.write_i32;
const obj_ensure_string = obj.obj_ensure_string;
const obj_new_int = obj.obj_new_int;
const obj_new_string = obj.obj_new_string;
const obj_get_int = obj.obj_get_int;

/// Phase-1 unification: every array — global, namespace-qualified, or
/// proc-local — lives in the single :data:`dir_buf` directory.  Local
/// arrays get a synthetic FQ name of the form
/// ``::__local::<frame_depth>::<arr>`` so they share storage and
/// iteration paths with global arrays.  ``frame_pop`` calls
/// :func:`drop_local_arrays_for_depth` to evict the entries when the
/// frame unwinds.
///
/// Naming convention rationale: ``::`` starts a fully-qualified name
/// so :func:`normalize_ns_name` won't re-prefix it.  ``__local`` is
/// reserved (double-underscore prefix per Tcl convention for runtime-
/// internal names).  ``<frame_depth>`` keeps unique per-call entries
/// even when a slot is reused.  All four parts are joined with the
/// standard ``::`` separator so debug dumps look like a normal FQ
/// name.
const LOCAL_PREFIX: []const u8 = "::__local::";

/// Build the FQ local-array name for a user-facing array name at the
/// given frame depth.  Allocated bytes live in the bump allocator;
/// the returned TclObj is the caller's responsibility to release.
pub fn make_local_array_obj(arr: i32, frame_depth: u32) i32 {
    const arr_s = obj_ensure_string(arr);
    if (arr_s.ptr == 0) return 0;
    // Rough upper bound: depth fits in 10 decimal digits for a u32.
    const total: u32 = @as(u32, @intCast(LOCAL_PREFIX.len)) + 10 + 2 + arr_s.len;
    const buf = alloc(total);
    if (buf == 0) return 0;
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (LOCAL_PREFIX) |b| {
        dst[off] = b;
        off += 1;
    }
    // Render frame_depth as decimal.
    var d_tmp: [10]u8 = undefined;
    var d_len: u32 = 0;
    var d_val: u32 = frame_depth;
    if (d_val == 0) {
        d_tmp[0] = '0';
        d_len = 1;
    } else {
        while (d_val > 0) {
            d_tmp[d_len] = @intCast('0' + (d_val % 10));
            d_val /= 10;
            d_len += 1;
        }
    }
    var di: u32 = d_len;
    while (di > 0) {
        di -= 1;
        dst[off] = d_tmp[di];
        off += 1;
    }
    dst[off] = ':';
    off += 1;
    dst[off] = ':';
    off += 1;
    if (arr_s.len > 0) {
        const ap: [*]const u8 = @ptrFromInt(arr_s.ptr);
        for (0..arr_s.len) |k| {
            dst[off] = ap[k];
            off += 1;
        }
    }
    return obj.obj_new_string_take(buf, off, total);
}

/// True iff *name_ptr / name_len* names a Phase-1 internal
/// proc-local array entry (``::__local::*``).  Used by directory-
/// iteration helpers (``array_dir_names_matching``, etc.) to filter
/// these out of user-visible listings like ``info vars``.
fn name_ptr_is_local_internal(name_ptr: u32, name_len: u32) bool {
    if (name_len < LOCAL_PREFIX.len) return false;
    const sp: [*]const u8 = @ptrFromInt(name_ptr);
    for (LOCAL_PREFIX, 0..) |b, k| if (sp[k] != b) return false;
    return true;
}

/// Walk the array directory and tombstone every entry whose name
/// starts with ``::__local::<depth>::``.  Called by ``frame_pop`` so
/// arrays a returning proc created don't leak into the next call at
/// the same depth.  Also called on ``frame_push`` defensively in case
/// a previous push at this slot didn't clean up (interpreter trap, …).
pub fn drop_local_arrays_for_depth(frame_depth: u32) void {
    if (dir_buf == 0) return;
    // Build the depth-specific prefix once.
    var prefix_buf: [LOCAL_PREFIX.len + 12]u8 = undefined;
    var pi: u32 = 0;
    for (LOCAL_PREFIX) |b| {
        prefix_buf[pi] = b;
        pi += 1;
    }
    var d_tmp: [10]u8 = undefined;
    var d_len: u32 = 0;
    var d_val: u32 = frame_depth;
    if (d_val == 0) {
        d_tmp[0] = '0';
        d_len = 1;
    } else {
        while (d_val > 0) {
            d_tmp[d_len] = @intCast('0' + (d_val % 10));
            d_val /= 10;
            d_len += 1;
        }
    }
    var di: u32 = d_len;
    while (di > 0) {
        di -= 1;
        prefix_buf[pi] = d_tmp[di];
        pi += 1;
    }
    prefix_buf[pi] = ':';
    pi += 1;
    prefix_buf[pi] = ':';
    pi += 1;

    var i: u32 = 0;
    while (i < dir_cap) : (i += 1) {
        const bucket = dir_buf + i * DIR_BUCKET_SIZE;
        const ep: u32 = @bitCast(read_i32(bucket));
        if (ep == 0) continue; // empty
        if (ep == @as(u32, 0xFFFFFFFF)) continue; // tombstone
        const el: u32 = @bitCast(read_i32(bucket + 4));
        if (el < pi) continue;
        const sp: [*]const u8 = @ptrFromInt(ep);
        var match = true;
        for (0..pi) |k| {
            if (sp[k] != prefix_buf[k]) {
                match = false;
                break;
            }
        }
        if (!match) continue;
        // Release the array's element table values, then tombstone.
        const t: u32 = @bitCast(read_i32(bucket + 12));
        if (t != 0) {
            const cap = ar_cap(t);
            var j: u32 = 0;
            while (j < cap) : (j += 1) {
                const eb = t + AR_HEADER_SIZE + j * AR_BUCKET_SIZE;
                const raw = read_i32(eb);
                if (raw == 0 or raw == AR_TOMBSTONE) continue;
                const old: i32 = read_i32(eb + 12);
                if (old != 0) obj.tcl_obj_release(old);
                write_i32(eb, AR_TOMBSTONE);
            }
        }
        write_i32(bucket, @bitCast(@as(u32, 0xFFFFFFFF))); // tombstone
    }
}

const ht = @import("hash_table.zig");
const fnv1a = ht.fnv1a;

/// Phase 6: variable-trace dispatcher.  Imported lazily to avoid a
/// hard dep cycle (var_trace → interp → array).  ``fire_*`` are no-
/// ops when no trace is installed for the given name.
const var_trace = @import("../interp/tcl_var_trace.zig");

/// Forward-declared probe into the namespace var subsystem so we can
/// detect scalar/array name conflicts without creating a circular
/// import (tcl_ns already imports this module).  Defined in
/// ``tcl_ns.zig`` as an exported helper and called only from
/// ``find_or_create`` below.  Returns 1 iff a scalar of the given
/// name already exists with a non-null value; 0 otherwise (including
/// "name has been unset" and "name is currently the array's key").
extern fn ns_scalar_exists(name_ptr: u32, name_len: u32) i32;
const stubs = @import("../stubs/tcl_stubs.zig");

// --- Array directory (name → per-array table) --------------------------

// Bucket: [name_ptr:4 | name_len:4 | hash:4 | table_ptr:4] = 16 bytes
const DIR_BUCKET_SIZE: u32 = 16;
const DIR_INITIAL_CAP: u32 = 16;

var dir_buf: u32 = 0;
var dir_cap: u32 = 0;
var dir_count: u32 = 0;

fn dir_init() void {
    if (dir_cap != 0) return;
    dir_cap = DIR_INITIAL_CAP;
    dir_buf = alloc(dir_cap * DIR_BUCKET_SIZE);
    var i: u32 = 0;
    while (i < dir_cap) : (i += 1) {
        write_i32(dir_buf + i * DIR_BUCKET_SIZE, 0);
    }
}

fn dir_find(name_ptr: u32, name_len: u32, hash: u32) ?u32 {
    if (dir_buf == 0) return null;
    const mask = dir_cap - 1;
    var idx = hash & mask;
    var probes: u32 = 0;
    while (probes < dir_cap) : (probes += 1) {
        const bucket = dir_buf + idx * DIR_BUCKET_SIZE;
        const ep: u32 = @bitCast(read_i32(bucket));
        if (ep == 0) return null;
        // Phase 1: ``drop_local_arrays_for_depth`` tombstones entries
        // it removes (``ep == 0xFFFFFFFF``).  Skip past them — the
        // probe chain may have grown over them while they were live,
        // so we cannot stop here without breaking lookups for entries
        // hashed earlier.
        if (ep == @as(u32, 0xFFFFFFFF)) {
            idx = (idx + 1) & mask;
            continue;
        }
        const el: u32 = @bitCast(read_i32(bucket + 4));
        const eh: u32 = @bitCast(read_i32(bucket + 8));
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

fn dir_insert(name_ptr: u32, name_len: u32, hash: u32, table_ptr: u32) void {
    dir_init();
    if (dir_count * 4 >= dir_cap * 3) dir_grow();
    const mask = dir_cap - 1;
    var idx = hash & mask;
    while (true) {
        const bucket = dir_buf + idx * DIR_BUCKET_SIZE;
        const ep: u32 = @bitCast(read_i32(bucket));
        // Empty (ep == 0) and tombstoned (ep == 0xFFFFFFFF) buckets
        // are both reusable for inserts.  The find path skips
        // tombstones so claiming one here doesn't shadow live entries
        // probed via the same chain.
        if (ep == 0 or ep == @as(u32, 0xFFFFFFFF)) {
            const nbuf = alloc(name_len);
            memcpy(nbuf, name_ptr, name_len);
            write_i32(bucket, @bitCast(nbuf));
            write_i32(bucket + 4, @bitCast(name_len));
            write_i32(bucket + 8, @bitCast(hash));
            write_i32(bucket + 12, @bitCast(table_ptr));
            dir_count += 1;
            return;
        }
        idx = (idx + 1) & mask;
    }
}

fn dir_grow() void {
    const old_buf = dir_buf;
    const old_cap = dir_cap;
    dir_cap *= 2;
    dir_buf = alloc(dir_cap * DIR_BUCKET_SIZE);
    var i: u32 = 0;
    while (i < dir_cap) : (i += 1) {
        write_i32(dir_buf + i * DIR_BUCKET_SIZE, 0);
    }
    dir_count = 0;
    var j: u32 = 0;
    while (j < old_cap) : (j += 1) {
        const bucket = old_buf + j * DIR_BUCKET_SIZE;
        const ep: u32 = @bitCast(read_i32(bucket));
        if (ep == 0 or ep == @as(u32, 0xFFFFFFFF)) continue; // empty / tombstone
        const el: u32 = @bitCast(read_i32(bucket + 4));
        const eh: u32 = @bitCast(read_i32(bucket + 8));
        const tp: u32 = @bitCast(read_i32(bucket + 12));
        dir_insert(ep, el, eh, tp);
    }
}

// --- Per-array table (key → value) -------------------------------------
// Layout: [cap:4 | count:4 | buckets[cap]...] where each bucket is
// [key_ptr:4 | key_len:4 | hash:4 | value:4] = 16 bytes.

const AR_BUCKET_SIZE: u32 = 16;
const AR_INITIAL_CAP: u32 = 8;
const AR_HEADER_SIZE: u32 = 8; // cap:4 | count:4

// Deletion sentinel for the ``name_ptr`` field of a bucket.  A regular
// bucket has ``name_ptr = 0`` when empty and a valid heap pointer when
// occupied — all real pointers are word-aligned below 2^32, so
// ``0xFFFFFFFF`` cannot collide with one.  ``ar_find`` must skip past
// tombstones (rather than treating them as empty terminators) to keep
// probe chains intact for keys that collided onto the deleted slot.
const AR_TOMBSTONE: i32 = @bitCast(@as(u32, 0xFFFF_FFFF));

fn ar_new() u32 {
    const cap: u32 = AR_INITIAL_CAP;
    const t = alloc(AR_HEADER_SIZE + cap * AR_BUCKET_SIZE);
    write_i32(t, @bitCast(cap));
    write_i32(t + 4, 0);
    var i: u32 = 0;
    while (i < cap) : (i += 1) {
        write_i32(t + AR_HEADER_SIZE + i * AR_BUCKET_SIZE, 0);
    }
    return t;
}

fn ar_cap(table: u32) u32 {
    return @bitCast(read_i32(table));
}

fn ar_count(table: u32) u32 {
    return @bitCast(read_i32(table + 4));
}

fn ar_set_count(table: u32, count: u32) void {
    write_i32(table + 4, @bitCast(count));
}

fn ar_find(table: u32, key_ptr: u32, key_len: u32, hash: u32) ?u32 {
    const cap = ar_cap(table);
    const mask = cap - 1;
    var idx = hash & mask;
    var probes: u32 = 0;
    while (probes < cap) : (probes += 1) {
        const bucket = table + AR_HEADER_SIZE + idx * AR_BUCKET_SIZE;
        const raw = read_i32(bucket);
        if (raw == 0) return null; // empty slot — key cannot be further on
        if (raw == AR_TOMBSTONE) {
            // Deleted slot; continue probing past it.
            idx = (idx + 1) & mask;
            continue;
        }
        const ep: u32 = @bitCast(raw);
        const el: u32 = @bitCast(read_i32(bucket + 4));
        const eh: u32 = @bitCast(read_i32(bucket + 8));
        if (eh == hash and el == key_len) {
            const sp: [*]const u8 = @ptrFromInt(ep);
            const np: [*]const u8 = @ptrFromInt(key_ptr);
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

/// MM-B.5 helper: store *value* into the value slot at ``bucket+12``,
/// retaining the new value and releasing the prior occupant.  The
/// array bucket owns one reference; without this helper, parser-side
/// release (MM-B.4) would free values still held only by the bucket.
fn bucket_set_value(bucket: u32, value: i32) void {
    const old: i32 = read_i32(bucket + 12);
    if (value != 0) obj.tcl_obj_retain(value);
    write_i32(bucket + 12, value);
    if (old != 0 and old != value) obj.tcl_obj_release(old);
}

fn ar_insert(table: u32, key_ptr: u32, key_len: u32, hash: u32, value: i32) u32 {
    var t = table;
    if (ar_count(t) * 4 >= ar_cap(t) * 3) {
        t = ar_grow(t);
    }
    const cap = ar_cap(t);
    const mask = cap - 1;
    var idx = hash & mask;
    // Track the first tombstone slot we see; we can reuse it only once
    // we've confirmed the key isn't present further along the probe chain.
    var first_tomb: ?u32 = null;
    var probes: u32 = 0;
    while (probes < cap) : (probes += 1) {
        const bucket = t + AR_HEADER_SIZE + idx * AR_BUCKET_SIZE;
        const raw = read_i32(bucket);
        if (raw == 0) {
            // Empty terminator: insert here, or fill the earlier tombstone if any.
            const target = first_tomb orelse bucket;
            const kbuf = alloc(key_len);
            memcpy(kbuf, key_ptr, key_len);
            write_i32(target, @bitCast(kbuf));
            write_i32(target + 4, @bitCast(key_len));
            write_i32(target + 8, @bitCast(hash));
            // Fresh slot — no old value to release.  Just retain.
            if (value != 0) obj.tcl_obj_retain(value);
            write_i32(target + 12, value);
            ar_set_count(t, ar_count(t) + 1);
            return t;
        }
        if (raw == AR_TOMBSTONE) {
            if (first_tomb == null) first_tomb = bucket;
            idx = (idx + 1) & mask;
            continue;
        }
        // Occupied slot: if it matches, overwrite in place.
        const el: u32 = @bitCast(read_i32(bucket + 4));
        const eh: u32 = @bitCast(read_i32(bucket + 8));
        if (eh == hash and el == key_len) {
            const ep: u32 = @bitCast(raw);
            const sp: [*]const u8 = @ptrFromInt(ep);
            const np: [*]const u8 = @ptrFromInt(key_ptr);
            var match = true;
            for (0..el) |k| {
                if (sp[k] != np[k]) {
                    match = false;
                    break;
                }
            }
            if (match) {
                bucket_set_value(bucket, value);
                return t;
            }
        }
        idx = (idx + 1) & mask;
    }
    // Table was full of tombstones — fall back to the tombstone slot.
    if (first_tomb) |target| {
        const kbuf = alloc(key_len);
        memcpy(kbuf, key_ptr, key_len);
        write_i32(target, @bitCast(kbuf));
        write_i32(target + 4, @bitCast(key_len));
        write_i32(target + 8, @bitCast(hash));
        if (value != 0) obj.tcl_obj_retain(value);
        write_i32(target + 12, value);
        ar_set_count(t, ar_count(t) + 1);
    }
    return t;
}

fn ar_grow(old_table: u32) u32 {
    const old_cap = ar_cap(old_table);
    const new_cap = old_cap * 2;
    const t = alloc(AR_HEADER_SIZE + new_cap * AR_BUCKET_SIZE);
    write_i32(t, @bitCast(new_cap));
    write_i32(t + 4, 0);
    var i: u32 = 0;
    while (i < new_cap) : (i += 1) {
        write_i32(t + AR_HEADER_SIZE + i * AR_BUCKET_SIZE, 0);
    }
    var j: u32 = 0;
    while (j < old_cap) : (j += 1) {
        const bucket = old_table + AR_HEADER_SIZE + j * AR_BUCKET_SIZE;
        const raw = read_i32(bucket);
        // Skip empty slots and tombstones — growing is an opportunity
        // to compact the probe chains.
        if (raw == 0 or raw == AR_TOMBSTONE) continue;
        const ep: u32 = @bitCast(raw);
        const el: u32 = @bitCast(read_i32(bucket + 4));
        const eh: u32 = @bitCast(read_i32(bucket + 8));
        const v: i32 = read_i32(bucket + 12);
        _ = ar_insert(t, ep, el, eh, v);
    }
    // Rewrite the directory entries that pointed at old_table.  Since
    // we don't know which directory entry owned it cheaply, walk the
    // entire directory — arrays are rare enough that O(n) is fine.
    if (dir_buf != 0) {
        var di: u32 = 0;
        while (di < dir_cap) : (di += 1) {
            const db = dir_buf + di * DIR_BUCKET_SIZE;
            const v: u32 = @bitCast(read_i32(db + 12));
            if (v == old_table) {
                write_i32(db + 12, @bitCast(t));
            }
        }
    }
    return t;
}

/// Canonicalise an array name to the directory key used by
/// :data:`dir_buf`:
///
/// * ``::foo`` (root global, no further ``::``)  → ``foo``  (strip)
/// * ``::ns::foo`` / ``::__local::N::foo``       → unchanged
/// * ``ns::foo`` (internal ``::``, no leading)   → ``::ns::foo``
/// * ``foo`` (bare, no ``::`` anywhere)           → unchanged
///
/// The strip direction matches the rest of the runtime's convention
/// for root-global keys: :func:`tcl_ns.strip_global_prefix` and
/// :func:`array_exists_raw` both treat the bare form as canonical.
/// Without it, ``set foo(a) 1`` and ``set ::foo(a) 1`` would key
/// distinct directory buckets — and ``set ::foo bar``'s scalar/array
/// conflict probe (which strips ``::`` before calling
/// :func:`array_exists_raw`) would miss the array stored at the
/// ``::foo`` spelling.
///
/// Returned ``i32`` is *either* the input ``name`` (no change) or a
/// fresh TclObj that the caller must :func:`tcl_obj_release` once it
/// is done with the lookup.  All in-tree callers follow the
/// ``defer if (n != name) tcl_obj_release(n)`` pattern.
fn normalize_ns_name(name: i32) i32 {
    const sn = obj_ensure_string(name);
    if (sn.len == 0) return name;
    const sp: [*]const u8 = @ptrFromInt(sn.ptr);
    if (sn.len >= 2 and sp[0] == ':' and sp[1] == ':') {
        // ``::``-prefixed.  Probe for another ``::`` after the
        // leading pair: if present (``::ns::foo``,
        // ``::__local::N::foo``), the name is namespace-qualified or
        // an internal proc-local key — keep as-is.
        var i: u32 = 2;
        while (i + 1 < sn.len) : (i += 1) {
            if (sp[i] == ':' and sp[i + 1] == ':') return name;
        }
        // Bare root global ``::foo`` — strip the prefix.  Empty body
        // (``::``) falls back to no-op so we never produce a
        // zero-length directory key.
        const stripped_len = sn.len - 2;
        if (stripped_len == 0) return name;
        const buf = alloc(stripped_len);
        // OOM — fall back to the qualified form rather than a
        // null-pointer trap.  The directory probe will miss and the
        // caller's higher-level error machinery handles the rest.
        if (buf == 0) return name;
        memcpy(buf, sn.ptr + 2, stripped_len);
        return obj.obj_new_string_take(buf, stripped_len, stripped_len);
    }
    // Internal ``::`` (``ns::foo``) → ``::ns::foo``.  Keeps
    // namespace arrays addressable by both spellings.
    var i: u32 = 0;
    while (i + 1 < sn.len) : (i += 1) {
        if (sp[i] == ':' and sp[i + 1] == ':') {
            const total: u32 = 2 + sn.len;
            const buf = alloc(total);
            if (buf == 0) return name;
            const d: [*]u8 = @ptrFromInt(buf);
            d[0] = ':';
            d[1] = ':';
            memcpy(buf + 2, sn.ptr, sn.len);
            return obj.obj_new_string_take(buf, total, total);
        }
    }
    // Bare unqualified name — already canonical.
    return name;
}

fn find_or_create(name: i32) u32 {
    const n = normalize_ns_name(name);
    defer if (n != name) obj.tcl_obj_release(n);
    const sn = obj_ensure_string(n);
    const hash = fnv1a(sn.ptr, sn.len);
    if (dir_find(sn.ptr, sn.len, hash)) |bucket| {
        const existing: u32 = @bitCast(read_i32(bucket + 12));
        if (existing != 0) return existing;
        // Bucket was nulled by array_unset — re-create a fresh table.
        const fresh = ar_new();
        write_i32(bucket + 12, @bitCast(fresh));
        return fresh;
    }
    // Namespace-aware fallback for unqualified names.  When *name* is
    // unqualified and a non-root namespace is active, also probe the
    // directory under ``<ns_full>::<name>`` and — when neither key
    // exists — *create* under the qualified form.  Mirrors the read
    // side in :func:`find_table` and the compile-time qualification
    // performed by ``_emit_array_name_obj`` for script-level writes
    // inside ``namespace eval``.  Required at runtime too because
    // proc bodies emit unqualified array names (the proc's
    // resolution namespace is only known once it's running), and a
    // tcltest proc that does ``set errorInfo(body) ...`` inside
    // ``::tcltest`` would otherwise collide with the global scalar
    // ``::errorInfo`` that ``stamp_error_globals`` writes after
    // every ``error`` — both keyed in root as ``errorInfo``, even
    // though Tcl semantics keep the namespace's array variable
    // disjoint from the global scalar.
    const sp: [*]const u8 = @ptrFromInt(sn.ptr);
    const is_qualified = sn.len >= 2 and sp[0] == ':' and sp[1] == ':';
    if (!is_qualified) {
        const ns_ptr = current_ns_full_ptr();
        const ns_len = current_ns_full_len();
        if (ns_len > 2) {
            const total: u32 = ns_len + 2 + sn.len;
            const qbuf = obj.alloc(total);
            if (qbuf != 0) {
                // ``qbuf`` is a *temporary* lookup key — ``dir_insert``
                // copies the bytes into its own bucket-owned buffer
                // (see :func:`dir_insert` above), and the find / scalar
                // probe paths only read through it.  Free on every
                // exit so a long-running script that keeps writing
                // namespace arrays doesn't leak one allocation per
                // ``set`` (Codex review on PR #297).  Mirrors the same
                // ``defer obj.free_sized`` pattern in :func:`find_table`.
                defer obj.free_sized(qbuf, total);
                const dst: [*]u8 = @ptrFromInt(qbuf);
                const ns_p: [*]const u8 = @ptrFromInt(ns_ptr);
                for (0..ns_len) |i| dst[i] = ns_p[i];
                dst[ns_len] = ':';
                dst[ns_len + 1] = ':';
                for (0..sn.len) |i| dst[ns_len + 2 + i] = sp[i];
                const qhash = fnv1a(qbuf, total);
                if (dir_find(qbuf, total, qhash)) |bucket| {
                    const existing: u32 = @bitCast(read_i32(bucket + 12));
                    if (existing != 0) return existing;
                    const fresh = ar_new();
                    write_i32(bucket + 12, @bitCast(fresh));
                    return fresh;
                }
                // Conflict check & insertion both keyed by the
                // qualified form so a same-namespace scalar (e.g.
                // ``::ns::errorInfo``) still raises while the bare
                // root scalar ``::errorInfo`` does not.
                if (ns_scalar_exists(qbuf, total) != 0) {
                    stubs.raise("can't set: variable isn't array");
                    return 0;
                }
                const t = ar_new();
                dir_insert(qbuf, total, qhash, t);
                return t;
            }
        }
    }
    // No array with this name yet — but a *scalar* might exist.  Real
    // Tcl raises ``can't set "<name>(...)": variable isn't array`` in
    // that case.  Detect via ``ns_scalar_exists`` (probe into the var
    // subsystem); if so, raise AND return 0 so callers know not to
    // proceed.  ``stubs.raise`` only sets the catch-side error flag —
    // it does NOT abort execution — so without the 0 sentinel we'd
    // both report an error and silently mutate the directory by
    // creating the array anyway, which is exactly what real Tcl
    // doesn't do.
    if (ns_scalar_exists(sn.ptr, sn.len) != 0) {
        stubs.raise("can't set: variable isn't array");
        return 0;
    }
    const t = ar_new();
    dir_insert(sn.ptr, sn.len, hash, t);
    return t;
}

fn find_table(name: i32) u32 {
    const n = normalize_ns_name(name);
    defer if (n != name) obj.tcl_obj_release(n);
    const sn = obj_ensure_string(n);
    // Null-TclObj guard: obj_ensure_string(0) returns (ptr=0, len=0).
    // @ptrFromInt(0) panics in WASM safety mode before any dereference,
    // so bail early when the name string has no backing memory.
    // Note: sn.len == 0 alone is NOT guarded here — empty-string array
    // names ("" / "") are valid Tcl, and a non-zero ptr with len=0 is
    // safe throughout the rest of the function (issue #327, var.test).
    if (sn.ptr == 0) return 0;
    if (dir_buf == 0) return 0;
    const hash = fnv1a(sn.ptr, sn.len);
    if (dir_find(sn.ptr, sn.len, hash)) |bucket| {
        return @bitCast(read_i32(bucket + 12));
    }
    // Namespace-aware fallback: when *name* is unqualified and a
    // non-root namespace is active, retry as ``<ns_full>::<name>``.
    // The compiled writer side (``_emit_array_name_obj`` in
    // _variables.py) already qualifies array names inside a
    // ``namespace eval`` block, so the read side has to mirror it
    // — otherwise an interpreted ``array exists path`` from inside
    // an upleveled body misses the qualified array the compiled
    // ``set path(test1) …`` deposited.  Fully-qualified names that
    // already start with ``::`` skip this branch.
    const sp: [*]const u8 = @ptrFromInt(sn.ptr);
    if (sn.len >= 2 and sp[0] == ':' and sp[1] == ':') return 0;
    const ns_ptr = current_ns_full_ptr();
    const ns_len = current_ns_full_len();
    if (ns_len <= 2) return 0; // root or unset — no extra prefix to try
    const total: u32 = ns_len + 2 + sn.len;
    const buf = obj.alloc(total);
    if (buf == 0) return 0;
    defer obj.free_sized(buf, total);
    const dst: [*]u8 = @ptrFromInt(buf);
    const ns_p: [*]const u8 = @ptrFromInt(ns_ptr);
    for (0..ns_len) |i| dst[i] = ns_p[i];
    dst[ns_len] = ':';
    dst[ns_len + 1] = ':';
    for (0..sn.len) |i| dst[ns_len + 2 + i] = sp[i];
    const qhash = fnv1a(buf, total);
    if (dir_find(buf, total, qhash)) |bucket| {
        return @bitCast(read_i32(bucket + 12));
    }
    return 0;
}

extern fn current_ns_full_ptr() u32;
extern fn current_ns_full_len() u32;

// --- Public exports ----------------------------------------------------

/// array_set arrName key value — stores value under key in the array
/// named arrName.  Creates the array on first write.  Returns value
/// on success, or 0 (null TclObj) when ``find_or_create`` reported a
/// scalar/array name conflict via ``stubs.raise`` — in that case no
/// element is stored.
pub export fn array_set(arr: i32, key: i32, value: i32) i32 {
    const t = find_or_create(arr);
    if (t == 0) return 0;
    const sk = obj_ensure_string(key);
    const hash = fnv1a(sk.ptr, sk.len);
    if (ar_find(t, sk.ptr, sk.len, hash)) |bucket| {
        // Overwrite: leaves the keyset intact, so any in-flight
        // ``array startsearch`` cursor is still valid (set-old-9.7).
        bucket_set_value(bucket, value);
    } else {
        // Adding a new key shifts the keyset under any open cursor;
        // Tcl 9 auto-stops all searches on this array
        // (set-old-9.6).
        array_invalidate_searches_for(arr);
        _ = ar_insert(t, sk.ptr, sk.len, hash, value);
    }
    // Phase 6: fire WRITE trace AFTER the update so the trace
    // observes the new value if it reads back.
    fire_var_trace_write(arr, sk.ptr, sk.len);
    return value;
}

/// Compute the canonical FQ name for ``arr`` (after ns normalisation)
/// and call var_trace.fire(WRITE).  Hot-path optimised: skip the
/// expensive name-build when no trace is installed.
fn fire_var_trace_write(arr: i32, key_ptr: u32, key_len: u32) void {
    fire_var_trace(arr, key_ptr, key_len, var_trace.OP_WRITE, 'w');
}

fn fire_var_trace_read(arr: i32, key_ptr: u32, key_len: u32) void {
    fire_var_trace(arr, key_ptr, key_len, var_trace.OP_READ, 'r');
}

fn fire_var_trace_unset(arr: i32, key_ptr: u32, key_len: u32) void {
    fire_var_trace(arr, key_ptr, key_len, var_trace.OP_UNSET, 'u');
}

/// Common ``has_trace`` probe + ``fire`` dispatch for the three
/// ``array_*`` hook points.  Probes both whole-array (``arr``) and
/// element-specific (``arr(key)``) registry entries; fires the
/// trace callback when either matches.  Tcl arrays allow
/// empty-string keys (``set arr() X``) so the element-form probe
/// runs even when ``key_len == 0`` — the key bytes are simply
/// omitted from the formatted ``arr()`` lookup name.
///
/// Returns early on null array name (defensive — ``@ptrFromInt(0)``
/// panics in ReleaseSafe even without a deref) and on the per-
/// element scratch-buffer OOM path so a failed allocation doesn't
/// trap the runtime.
fn fire_var_trace(arr: i32, key_ptr: u32, key_len: u32, op: u32, op_char: u8) void {
    const an = normalize_ns_name(arr);
    defer if (an != arr) obj.tcl_obj_release(an);
    const sn = obj_ensure_string(an);
    if (sn.ptr == 0 or sn.len == 0) return;
    if (!var_trace.has_trace(sn.ptr, sn.len, op)) {
        // Probe the element-specific ``arr(key)`` form before
        // bailing.  Build the form even for an empty key so
        // ``trace add variable arr() OPS CMD`` is honoured.
        const total: u32 = sn.len + 2 + key_len;
        const buf = alloc(total);
        if (buf == 0) return;
        const dst: [*]u8 = @ptrFromInt(buf);
        const ap: [*]const u8 = @ptrFromInt(sn.ptr);
        for (0..sn.len) |i| dst[i] = ap[i];
        dst[sn.len] = '(';
        if (key_len > 0 and key_ptr != 0) {
            const kp: [*]const u8 = @ptrFromInt(key_ptr);
            for (0..key_len) |i| dst[sn.len + 1 + i] = kp[i];
        }
        dst[total - 1] = ')';
        const has_elem = var_trace.has_trace(buf, total, op);
        obj.free_sized(buf, total);
        if (!has_elem) return;
    }
    var_trace.fire(sn.ptr, sn.len, key_ptr, key_len, op, op_char);
}

/// ``array set arrName {k v k v …}`` — the *list-of-pairs* form.
/// Splits the list payload via ``list_element_at`` and stores each
/// ``(key, value)`` pair under *arr*.  Returns empty string.
///
/// This exists as a runtime helper because the compile-time form
/// in ``_emit_array_set_list`` only fires when the value argument
/// is a brace-literal; command-substitution payloads (``[list
/// Total 0 …]``) or variable payloads (``$pairs``) reach here at
/// runtime and need the same "iterate pairs" semantics.  Without
/// it, the compiler's eval-fallback would take the interpreter's
/// 1-pair ``array set`` path and (mis-)store the whole list as a
/// single key — which silently broke ``ArrayDefault numTests``
/// initialisation in tcltest, since ``incr numTests(Total)`` then
/// ran on an uninitialised element.
pub export fn array_set_list(arr: i32, pairs: i32) i32 {
    const sp = obj_ensure_string(pairs);
    // Validate the list parses cleanly *first* — invalid list syntax
    // (e.g. unmatched brace) must raise before any side effects, and
    // a list with an odd number of elements raises ``list must have
    // an even number of elements`` (set-old-8.58).  ``list_count_elements``
    // counts the parser's view of elements; for catch-all syntax
    // errors we use the dedicated validator.
    const list_parse = @import("tcl_list_parse.zig");
    if (list_parse.check_list_syntax(sp.ptr, sp.len) != 0) {
        // The validator already raised the canonical
        // ``list element in ...`` / ``unmatched open brace`` /
        // similar error.
        return obj_new_string(0, 0);
    }
    const n = obj.list_count_elements(sp.ptr, sp.len);
    if (@rem(n, 2) != 0) {
        raise_odd_list();
        return obj_new_string(0, 0);
    }
    if (n == 0) {
        // Empty payload — ``array set X {}`` on a scalar raises
        // ``can't array set "X": variable isn't array`` (a different
        // shape from the non-empty-list path; see set-old-8.38.1 /
        // 8.38.3).  Pass key_ptr = 0 / key_len = 0 so the helper
        // selects the empty-list message.
        if (!array_check_or_create(arr, 0, 0)) return obj_new_string(0, 0);
        return obj_new_string(0, 0);
    }
    // Up-front scalar-conflict probe: ``array set arr ...`` on a
    // variable that already exists as a scalar raises ``can't set
    // "arr(<first_key>)": variable isn't array`` (set-old-8.35).
    // Extract the first key so the error message matches Tcl.
    const first_k = obj.list_element_at(sp.ptr, sp.len, 0);
    if (!array_check_or_create(arr, sp.ptr + first_k.start, first_k.len)) {
        return obj_new_string(0, 0);
    }
    var i: i64 = 0;
    while (i + 1 < n) : (i += 2) {
        const k_info = obj.list_element_at(sp.ptr, sp.len, i);
        const v_info = obj.list_element_at(sp.ptr, sp.len, i + 1);
        const k_obj = obj.obj_new_string_copy(sp.ptr + k_info.start, k_info.len);
        const v_obj = obj.obj_new_string_copy(sp.ptr + v_info.start, v_info.len);
        if (array_set(arr, k_obj, v_obj) == 0) {
            obj.tcl_obj_release(k_obj);
            obj.tcl_obj_release(v_obj);
            break;
        }
        obj.tcl_obj_release(k_obj);
        obj.tcl_obj_release(v_obj);
    }
    return obj_new_string(0, 0);
}

/// Detect the ``array set name ...`` scalar-vs-array conflict.
/// Returns false when *arr* already exists as a scalar (Tcl 9
/// raises ``can't set "<arr>(<key>)": variable isn't array`` for
/// non-empty lists or ``can't array set "<arr>": variable isn't
/// array`` for empty lists); in that case the canonical error has
/// been queued.  Returns true when *arr* is absent or already an
/// array.
///
/// *first_key_ptr* / *first_key_len* describe the first key of
/// the supplied list (only used to format the error message).
/// Pass 0 / 0 when the list was empty.
fn array_check_or_create(arr: i32, first_key_ptr: u32, first_key_len: u32) bool {
    // ``array_exists`` reads the directory; if the name is already
    // an array we're fine.
    const exists_obj = array_exists(arr);
    if (obj.obj_get_int(exists_obj) != 0) return true;
    // Probe the global flat-key store for a scalar of the same name.
    const tcl_ns = @import("../interp/tcl_ns.zig");
    const exists_scalar = obj.obj_get_int(tcl_ns.global_exists(arr)) != 0;
    if (!exists_scalar) return true;
    // Scalar conflict — raise the shape that matches the caller's
    // payload.
    if (first_key_len == 0) {
        raise_array_set_scalar_conflict_empty(arr);
    } else {
        raise_array_set_scalar_conflict(arr, first_key_ptr, first_key_len);
    }
    return false;
}

fn raise_odd_list() void {
    const catch_mod = @import("../interp/tcl_catch.zig");
    const msg: []const u8 = "list must have an even number of elements";
    const buf = obj.alloc(@intCast(msg.len));
    if (buf == 0) {
        catch_mod.tcl_cmd_error(obj_new_string(0, 0));
        return;
    }
    const dst: [*]u8 = @ptrFromInt(buf);
    for (msg, 0..) |c, i| dst[i] = c;
    const e = obj.obj_new_string_take(buf, @intCast(msg.len), @intCast(msg.len));
    catch_mod.tcl_cmd_error(e);
}

/// Format ``can't set "<arr>(<first_key>)": variable isn't array``
/// for the non-empty ``array set X {k v …}`` path (set-old-8.35).
fn raise_array_set_scalar_conflict(arr: i32, key_ptr: u32, key_len: u32) void {
    const catch_mod = @import("../interp/tcl_catch.zig");
    const sn = obj_ensure_string(arr);
    const prefix: []const u8 = "can't set \"";
    const mid: []const u8 = "(";
    const suffix: []const u8 = ")\": variable isn't array";
    const total: u32 = @intCast(prefix.len + sn.len + mid.len + key_len + suffix.len);
    const buf = obj.alloc(total);
    if (buf == 0) {
        catch_mod.tcl_cmd_error(obj_new_string(0, 0));
        return;
    }
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    const np: [*]const u8 = @ptrFromInt(sn.ptr);
    for (0..sn.len) |k| {
        dst[off] = np[k];
        off += 1;
    }
    for (mid) |c| {
        dst[off] = c;
        off += 1;
    }
    if (key_len > 0) {
        const kp: [*]const u8 = @ptrFromInt(key_ptr);
        for (0..key_len) |k| {
            dst[off] = kp[k];
            off += 1;
        }
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const e = obj.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(e);
}

/// Format ``can't array set "<arr>": variable isn't array`` for
/// the empty ``array set X {}`` path (set-old-8.38.1 / 8.38.3).
fn raise_array_set_scalar_conflict_empty(arr: i32) void {
    const catch_mod = @import("../interp/tcl_catch.zig");
    const sn = obj_ensure_string(arr);
    const prefix: []const u8 = "can't array set \"";
    const suffix: []const u8 = "\": variable isn't array";
    const total: u32 = @intCast(prefix.len + sn.len + suffix.len);
    const buf = obj.alloc(total);
    if (buf == 0) {
        catch_mod.tcl_cmd_error(obj_new_string(0, 0));
        return;
    }
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    const np: [*]const u8 = @ptrFromInt(sn.ptr);
    for (0..sn.len) |k| {
        dst[off] = np[k];
        off += 1;
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const e = obj.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(e);
}

/// array_get arrName key — returns the stored value, or 0 (null
/// TclObj) if the array/key is missing.  Callers that need to
/// distinguish missing-vs-present should use ``array_element_exists``
/// first or ``info exists``.
pub export fn array_get(arr: i32, key: i32) i32 {
    const sk = obj_ensure_string(key);
    // Phase 6: fire READ trace BEFORE the lookup.  The callback may
    // ``set`` the variable to provide a lazy value (e.g. tcltest
    // testConstraints), and we want the just-set value to surface to
    // the caller.  Re-find after firing.
    fire_var_trace_read(arr, sk.ptr, sk.len);
    const t = find_table(arr);
    if (t == 0) return 0;
    const hash = fnv1a(sk.ptr, sk.len);
    if (ar_find(t, sk.ptr, sk.len, hash)) |bucket| {
        return read_i32(bucket + 12);
    }
    return 0;
}

/// array_get_all arrName ?pattern? — returns the array contents as a
/// flat ``{k v k v …}`` Tcl list.  Optional glob *pattern* filters
/// keys (empty/0 pattern => all keys).  Each key and value is
/// quoted via ``list_elem_quote_nth`` so embedded spaces, braces,
/// or backslashes survive the round-trip through ``foreach {k v}
/// [array get arr] {…}``.  This is the runtime backing for
/// ``array get arrName ?pattern?`` — the previous wiring used
/// the single-element ``array_get`` lookup path which is the wrong
/// semantics (``array get`` returns *all* matching pairs as a list).
pub export fn array_get_all(arr: i32, pattern: i32) i32 {
    const str_mod = @import("tcl_string.zig");
    const lq = @import("tcl_list_quote.zig");
    var pat_ptr: u32 = 0;
    var pat_len: u32 = 0;
    const use_filter = blk: {
        if (pattern == 0) break :blk false;
        const ps = obj_ensure_string(pattern);
        pat_ptr = ps.ptr;
        pat_len = ps.len;
        break :blk ps.len > 0;
    };
    const t = find_table(arr);
    if (t == 0) return obj_new_string(0, 0);
    const cap = ar_cap(t);
    // Two-pass build: pass 1 sums upper-bound bytes (worst case
    // 2*len + 2 per element via ``list_elem_quote_nth`` plus one
    // separator space per element after the first).  Pass 2 emits
    // bytes via the quoter.
    var total: u32 = 0;
    var nonempty: u32 = 0;
    var i: u32 = 0;
    while (i < cap) : (i += 1) {
        const bucket = t + AR_HEADER_SIZE + i * AR_BUCKET_SIZE;
        const raw = read_i32(bucket);
        if (raw == 0 or raw == AR_TOMBSTONE) continue;
        const ep: u32 = @bitCast(raw);
        const el: u32 = @bitCast(read_i32(bucket + 4));
        if (use_filter) {
            // Match the raw key bytes via ``glob_match`` directly
            // rather than allocating a TclObj per probe — the
            // earlier ``string_match`` path leaked one key obj per
            // entry across both passes.
            if (!str_mod.glob_match(pat_ptr, pat_len, ep, el)) continue;
        }
        const v = read_i32(bucket + 12);
        const vs = obj_ensure_string(v);
        // worst-case quoted length per element: 2*len + 2 (braces
        // or backslashes) ; plus inter-element spaces (2 per pair —
        // between key+value and between pairs) — over-allocate.
        total += (el * 2 + 2) + (vs.len * 2 + 2) + 2;
        nonempty += 1;
    }
    if (nonempty == 0) return obj_new_string(0, 0);
    const buf = alloc(total);
    if (buf == 0) return obj_new_string(0, 0);
    var off: u32 = 0;
    var written: u32 = 0;
    i = 0;
    while (i < cap) : (i += 1) {
        const bucket = t + AR_HEADER_SIZE + i * AR_BUCKET_SIZE;
        const raw = read_i32(bucket);
        if (raw == 0 or raw == AR_TOMBSTONE) continue;
        const ep: u32 = @bitCast(raw);
        const el: u32 = @bitCast(read_i32(bucket + 4));
        if (use_filter) {
            // Match the raw key bytes via ``glob_match`` directly
            // rather than allocating a TclObj per probe — the
            // earlier ``string_match`` path leaked one key obj per
            // entry across both passes.
            if (!str_mod.glob_match(pat_ptr, pat_len, ep, el)) continue;
        }
        const v = read_i32(bucket + 12);
        const vs = obj_ensure_string(v);
        if (written > 0) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        off = lq.list_elem_quote_nth(buf, off, ep, el);
        const d2: [*]u8 = @ptrFromInt(buf + off);
        d2[0] = ' ';
        off += 1;
        off = lq.list_elem_quote_nth(buf, off, vs.ptr, vs.len);
        written += 1;
    }
    // ``obj_new_string_take`` so the returned TclObj owns the
    // join buffer.  We over-allocated ``total`` bytes against the
    // worst-case quote bound; ``off`` is the actual emitted size.
    return obj.obj_new_string_take(buf, off, total);
}

/// array_exists arrName — 1 if the array *variable* has ever been
/// created, regardless of current element count.  This matches Tcl:
/// ``set a(x) 1; unset a(x); array exists a`` returns 1 because the
/// array variable itself still exists as an (empty) array.  We treat
/// "directory entry present" as equivalent to "array variable exists".
pub export fn array_exists(arr: i32) i32 {
    const t = find_table(arr);
    if (t == 0) return obj_new_int(0);
    return obj_new_int(1);
}

/// Bare-int variant of :func:`array_exists` for runtime-side
/// callers that don't want a TclObj round-trip.  Returns 1 iff the
/// array directory has an entry for the given (raw byte, length)
/// name; 0 otherwise.  Used by ``tcl_ns.global_set`` so it can
/// query the array directory using the same stripped-``::``
/// canonical form ``ns_var_find`` uses for the matching scalar
/// lookup, *without* allocating a TclObj that the caller would
/// have to release.
pub fn array_exists_raw(name_ptr: u32, name_len: u32) bool {
    if (dir_buf == 0 or name_len == 0) return false;
    // The directory keys things by the *post-normalisation* name
    // (``::ns::a`` for any qualified write, bare ``a`` otherwise),
    // and ``find_or_create`` does normalisation on insert.  We
    // mirror the same key here: callers pass the stripped key
    // (``a`` or ``ns::a``); for ``ns::a`` we add the ``::`` prefix
    // so the lookup matches the inserted form.
    const sp: [*]const u8 = @ptrFromInt(name_ptr);
    var probe_ptr = name_ptr;
    var probe_len = name_len;
    var prefix_buf: [256]u8 = undefined;
    if (name_len >= 2 and sp[0] != ':' and sp[1] != ':') {
        // Look for an internal ``::`` to mirror normalize_ns_name's
        // behaviour.  If we find one, prepend ``::`` for the
        // directory lookup.
        var i: u32 = 0;
        while (i + 1 < name_len) : (i += 1) {
            if (sp[i] == ':' and sp[i + 1] == ':') {
                if (name_len + 2 > prefix_buf.len) return false;
                prefix_buf[0] = ':';
                prefix_buf[1] = ':';
                for (0..name_len) |k| prefix_buf[2 + k] = sp[k];
                probe_ptr = @intFromPtr(&prefix_buf[0]);
                probe_len = name_len + 2;
                break;
            }
        }
    }
    const hash = fnv1a(probe_ptr, probe_len);
    if (dir_find(probe_ptr, probe_len, hash)) |bucket| {
        return @as(u32, @bitCast(read_i32(bucket + 12))) != 0;
    }
    return false;
}

/// array_element_exists arrName key — 1 if arr(key) is set, 0 otherwise.
/// Semantically equivalent to ``info exists arr(key)`` for a named
/// array element.
pub export fn array_element_exists(arr: i32, key: i32) i32 {
    const t = find_table(arr);
    if (t == 0) return obj_new_int(0);
    const sk = obj_ensure_string(key);
    const hash = fnv1a(sk.ptr, sk.len);
    if (ar_find(t, sk.ptr, sk.len, hash) != null) return obj_new_int(1);
    return obj_new_int(0);
}

/// array_size arrName — element count (0 if missing).
pub export fn array_size(arr: i32) i32 {
    const t = find_table(arr);
    if (t == 0) return obj_new_int(0);
    return obj_new_int(@intCast(ar_count(t)));
}

/// array_unset arrName — remove the entire array (all elements).
pub export fn array_unset(arr: i32) i32 {
    array_invalidate_searches_for(arr);
    const n = normalize_ns_name(arr);
    defer if (n != arr) obj.tcl_obj_release(n);
    const sn = obj_ensure_string(n);
    if (dir_buf == 0) return obj_new_int(0);
    const hash = fnv1a(sn.ptr, sn.len);
    if (dir_find(sn.ptr, sn.len, hash)) |bucket| {
        // Null out the table pointer so array_exists / find_table
        // treat this array as non-existent.  The directory entry itself
        // stays so the open-addressing chain isn't broken; find_or_create
        // will re-allocate a fresh table when the array is re-created.
        write_i32(bucket + 12, 0);
    }
    return obj_new_int(0);
}

/// array_unset_element arrName key — remove a single element.
///
/// Writes ``AR_TOMBSTONE`` into the bucket's ``name_ptr`` slot so
/// ``ar_find`` skips past it instead of treating it as the end of the
/// probe chain — that's what would otherwise make collision-mates
/// unreachable.  Tombstones are cleared wholesale on grow() and are
/// reusable for future insertions.
pub export fn array_unset_element(arr: i32, key: i32) i32 {
    const t = find_table(arr);
    if (t == 0) return obj_new_int(0);
    const sk = obj_ensure_string(key);
    const hash = fnv1a(sk.ptr, sk.len);
    if (ar_find(t, sk.ptr, sk.len, hash)) |bucket| {
        // Real deletion — auto-stops every search on this array
        // (set-old-9.8).  Skipped for the "key already absent"
        // branch (set-old-9.9 — ``catch {unset a(c)}`` when ``c``
        // isn't there does NOT auto-stop).
        array_invalidate_searches_for(arr);
        // MM-B.5: release the value slot's reference before tombstoning.
        const old: i32 = read_i32(bucket + 12);
        write_i32(bucket, AR_TOMBSTONE);
        write_i32(bucket + 4, 0);
        write_i32(bucket + 8, 0);
        write_i32(bucket + 12, 0);
        ar_set_count(t, ar_count(t) - 1);
        if (old != 0) obj.tcl_obj_release(old);
    }
    // Phase 6: fire UNSET trace AFTER the unset so the callback sees
    // the variable in its final (gone) state.
    fire_var_trace_unset(arr, sk.ptr, sk.len);
    return obj_new_int(0);
}

// --- Array search state ------------------------------------------------
//
// Tcl 9 ``array startsearch`` / ``array nextelement`` / ``array
// donesearch`` / ``array anymore`` maintain per-array cursor state
// across multiple concurrent searches.  The set-old-9.x tests cover
// the contract:
//
//   * Multiple concurrent searches on the same array each get a
//     unique monotonically-allocated ID (``s-1-arr``, ``s-2-arr``,
//     …).  IDs reset to 1 once *all* searches on the array have
//     been retired.
//   * ``nextelement`` returns one element at a time and the empty
//     string once exhausted.  Different concurrent searches each
//     keep their own cursor.
//   * Mutating the array (insert, unset element, unset whole array,
//     add a read trace) auto-stops every live search on it —
//     subsequent ``nextelement`` / ``anymore`` / ``donesearch`` on
//     the retired SID error with ``couldn't find search "s-N-arr"``.
//   * Overwriting an existing element does NOT auto-stop a search
//     (count unchanged).
//
// Storage: a single intrusively-linked list anchored at
// :data:`searches_head`.  Each record is 24 bytes; the array name
// is duplicated into a bump-allocated copy so the record can outlive
// the caller's TclObj.
const SEARCH_REC_SIZE: u32 = 24;
const SEARCH_OFF_NEXT: u32 = 0;
const SEARCH_OFF_NAME_PTR: u32 = 4;
const SEARCH_OFF_NAME_LEN: u32 = 8;
const SEARCH_OFF_ID: u32 = 12;
const SEARCH_OFF_ITER: u32 = 16;
const SEARCH_OFF_TABLE: u32 = 20;

var searches_head: u32 = 0;

fn search_name_matches(rec: u32, name_ptr: u32, name_len: u32) bool {
    const rl: u32 = @bitCast(read_i32(rec + SEARCH_OFF_NAME_LEN));
    if (rl != name_len) return false;
    const rp: u32 = @bitCast(read_i32(rec + SEARCH_OFF_NAME_PTR));
    if (rp == name_ptr) return true;
    if (rp == 0 or name_ptr == 0) return false;
    const a: [*]const u8 = @ptrFromInt(rp);
    const b: [*]const u8 = @ptrFromInt(name_ptr);
    for (0..name_len) |k| if (a[k] != b[k]) return false;
    return true;
}

fn next_search_id_for(name_ptr: u32, name_len: u32) u32 {
    var max_id: u32 = 0;
    var cur = searches_head;
    while (cur != 0) {
        if (search_name_matches(cur, name_ptr, name_len)) {
            const id: u32 = @bitCast(read_i32(cur + SEARCH_OFF_ID));
            if (id > max_id) max_id = id;
        }
        cur = @bitCast(read_i32(cur + SEARCH_OFF_NEXT));
    }
    return max_id + 1;
}

fn find_search_rec(name_ptr: u32, name_len: u32, id: u32) u32 {
    var cur = searches_head;
    while (cur != 0) {
        const rid: u32 = @bitCast(read_i32(cur + SEARCH_OFF_ID));
        if (rid == id and search_name_matches(cur, name_ptr, name_len)) return cur;
        cur = @bitCast(read_i32(cur + SEARCH_OFF_NEXT));
    }
    return 0;
}

fn unlink_search_rec(rec: u32) void {
    if (rec == 0) return;
    if (searches_head == rec) {
        searches_head = @bitCast(read_i32(rec + SEARCH_OFF_NEXT));
        return;
    }
    var prev = searches_head;
    while (prev != 0) {
        const nxt: u32 = @bitCast(read_i32(prev + SEARCH_OFF_NEXT));
        if (nxt == rec) {
            const after: i32 = read_i32(rec + SEARCH_OFF_NEXT);
            write_i32(prev + SEARCH_OFF_NEXT, after);
            return;
        }
        prev = nxt;
    }
}

/// Remove every active search on *arr*.  Called from
/// ``array_invalidate_searches_for`` whenever the array's element
/// count changes (insert / unset) — Tcl 9's
/// :func:`DeleteSearches` semantics.
fn drop_searches_for(name_ptr: u32, name_len: u32) void {
    var cur = searches_head;
    var prev: u32 = 0;
    while (cur != 0) {
        const nxt: u32 = @bitCast(read_i32(cur + SEARCH_OFF_NEXT));
        if (search_name_matches(cur, name_ptr, name_len)) {
            if (prev == 0) {
                searches_head = nxt;
            } else {
                write_i32(prev + SEARCH_OFF_NEXT, @bitCast(nxt));
            }
            // Don't advance prev — cur was removed.
        } else {
            prev = cur;
        }
        cur = nxt;
    }
}

/// Called from every mutation path (set new key, unset element,
/// array_unset, traces add).  ``arr`` is the user-visible name —
/// we normalise it here so the search lookup keys match the form
/// stored on the record.
pub fn array_invalidate_searches_for_obj(arr: i32) void {
    array_invalidate_searches_for(arr);
}

fn array_invalidate_searches_for(arr: i32) void {
    if (searches_head == 0) return;
    const n = normalize_ns_name(arr);
    defer if (n != arr) obj.tcl_obj_release(n);
    const sn = obj_ensure_string(n);
    if (sn.ptr == 0 or sn.len == 0) return;
    drop_searches_for(sn.ptr, sn.len);
}

/// Allocate a fresh search record on the array bump heap.  The
/// array name is copied so the record's name pointer survives the
/// caller's TclObj going out of scope (parser-borrowed bytes).
fn alloc_search_rec(name_ptr: u32, name_len: u32, id: u32, table_ptr: u32) u32 {
    const rec = alloc(SEARCH_REC_SIZE);
    if (rec == 0) return 0;
    const nbuf = if (name_len == 0) 0 else alloc(name_len);
    if (name_len > 0 and nbuf == 0) return 0;
    if (name_len > 0) memcpy(nbuf, name_ptr, name_len);
    write_i32(rec + SEARCH_OFF_NEXT, 0);
    write_i32(rec + SEARCH_OFF_NAME_PTR, @bitCast(nbuf));
    write_i32(rec + SEARCH_OFF_NAME_LEN, @bitCast(name_len));
    write_i32(rec + SEARCH_OFF_ID, @bitCast(id));
    write_i32(rec + SEARCH_OFF_ITER, 0);
    write_i32(rec + SEARCH_OFF_TABLE, @bitCast(table_ptr));
    return rec;
}

/// ``array startsearch arrName`` — registers a fresh search on
/// *storage* and returns its TclObj search id (``s-N-NAME``).
/// *display* is the user-typed name used in the returned SID;
/// *storage* is the post-:func:`frame_resolve_array_name` form so
/// the search's table lookup honours proc-local arrays and
/// :func:`upvar` aliases (which key off ``::__local::<depth>::``
/// or the alias target rather than the user-visible name).
/// Returns 0 when the array doesn't exist (caller raises "X isn't
/// an array" first; this entry point assumes existence).
pub fn array_search_start(storage: i32, display: i32) i32 {
    const n = normalize_ns_name(storage);
    defer if (n != storage) obj.tcl_obj_release(n);
    const sn = obj_ensure_string(n);
    if (sn.ptr == 0 or sn.len == 0) return 0;
    const table = find_table(storage);
    if (table == 0) return 0;
    const id = next_search_id_for(sn.ptr, sn.len);
    const rec = alloc_search_rec(sn.ptr, sn.len, id, table);
    if (rec == 0) return 0;
    write_i32(rec + SEARCH_OFF_NEXT, @bitCast(searches_head));
    searches_head = rec;
    return build_search_id_obj(display, id);
}

/// Build the canonical ``s-N-<user-visible-name>`` TclObj.  Uses
/// the *caller-supplied* form of the array name (not the
/// normalised storage key) so the round-trip matches Tcl's
/// behaviour: ``array startsearch ::foo`` returns ``s-1-::foo``,
/// not ``s-1-foo``.
fn build_search_id_obj(arr: i32, id: u32) i32 {
    const arr_s = obj_ensure_string(arr);
    // Render the decimal id.
    var d_buf: [10]u8 = undefined;
    var d_len: u32 = 0;
    var d_val = id;
    if (d_val == 0) {
        d_buf[0] = '0';
        d_len = 1;
    } else {
        while (d_val > 0) {
            d_buf[d_len] = @intCast('0' + (d_val % 10));
            d_val /= 10;
            d_len += 1;
        }
    }
    const prefix_len: u32 = 2; // "s-"
    const sep_len: u32 = 1; // "-"
    const total: u32 = prefix_len + d_len + sep_len + arr_s.len;
    const buf = alloc(total);
    if (buf == 0) return obj_new_string(0, 0);
    const dst: [*]u8 = @ptrFromInt(buf);
    dst[0] = 's';
    dst[1] = '-';
    var off: u32 = 2;
    var di: u32 = d_len;
    while (di > 0) {
        di -= 1;
        dst[off] = d_buf[di];
        off += 1;
    }
    dst[off] = '-';
    off += 1;
    if (arr_s.len > 0) {
        const ap: [*]const u8 = @ptrFromInt(arr_s.ptr);
        for (0..arr_s.len) |k| dst[off + k] = ap[k];
    }
    return obj.obj_new_string_take(buf, total, total);
}

/// Look up a search by *(storage_name, id)*.  Returns the record
/// address, or 0 if the search isn't registered (was auto-stopped
/// or already done).  Search records are keyed off the *normalised
/// storage* form of the array name (mirrors what
/// :func:`array_search_start` writes), so callers MUST pass the
/// post-:func:`frame_resolve_array_name` form for proc-local
/// arrays and :func:`upvar` aliases to resolve correctly.
pub fn array_search_lookup(storage: i32, id: u32) u32 {
    if (searches_head == 0) return 0;
    const n = normalize_ns_name(storage);
    defer if (n != storage) obj.tcl_obj_release(n);
    const sn = obj_ensure_string(n);
    if (sn.ptr == 0 or sn.len == 0) return 0;
    return find_search_rec(sn.ptr, sn.len, id);
}

/// ``array nextelement arrName SID`` body once the SID has been
/// validated and matched to a live search record.  Returns the
/// next key TclObj or an empty string when the search has been
/// exhausted; advances the record's iter cursor as a side
/// effect.  Does NOT raise on its own — the caller (array.zig)
/// owns the wrong-array / not-found / illegal error paths.
pub fn array_search_next(arr: i32, rec: u32) i32 {
    _ = arr;
    if (rec == 0) return obj_new_string(0, 0);
    const table: u32 = @bitCast(read_i32(rec + SEARCH_OFF_TABLE));
    if (table == 0) return obj_new_string(0, 0);
    const cap = ar_cap(table);
    var pos: u32 = @bitCast(read_i32(rec + SEARCH_OFF_ITER));
    while (pos < cap) : (pos += 1) {
        const bucket = table + AR_HEADER_SIZE + pos * AR_BUCKET_SIZE;
        const raw = read_i32(bucket);
        if (raw == 0 or raw == AR_TOMBSTONE) continue;
        // Found a live key — package and advance.
        const kp: u32 = @bitCast(raw);
        const kl: u32 = @bitCast(read_i32(bucket + 4));
        write_i32(rec + SEARCH_OFF_ITER, @bitCast(pos + 1));
        return obj_new_string(@bitCast(kp), @bitCast(kl));
    }
    // Exhausted.
    write_i32(rec + SEARCH_OFF_ITER, @bitCast(cap));
    return obj_new_string(0, 0);
}

/// True when *rec* still has at least one key the cursor hasn't
/// returned yet.  Pure peek — does NOT advance the cursor.
pub fn array_search_anymore(rec: u32) bool {
    if (rec == 0) return false;
    const table: u32 = @bitCast(read_i32(rec + SEARCH_OFF_TABLE));
    if (table == 0) return false;
    const cap = ar_cap(table);
    var pos: u32 = @bitCast(read_i32(rec + SEARCH_OFF_ITER));
    while (pos < cap) : (pos += 1) {
        const bucket = table + AR_HEADER_SIZE + pos * AR_BUCKET_SIZE;
        const raw = read_i32(bucket);
        if (raw == 0 or raw == AR_TOMBSTONE) continue;
        return true;
    }
    return false;
}

/// Retire *rec*: detach from the global list.  Caller has the
/// record's address; we don't bother clearing the record fields
/// (bump-allocated memory; never reused).
pub fn array_search_done(rec: u32) void {
    unlink_search_rec(rec);
}

/// array_names arrName ?pattern? — returns a space-separated list
/// of keys.  When *pattern* is a non-empty TclObj, only keys that
/// match the glob *pattern* (same semantics as ``string match``)
/// are included — the filter tcltest's ``MatchingOption`` relies
/// on to turn ``array names Option $option*`` into a
/// prefix-scoped lookup.  Order is hash-table order (Tcl makes no
/// ordering promise without an explicit sort).
///
/// Pattern of ``0`` or an empty string disables the filter and
/// returns every key.
pub export fn array_names(arr: i32, pattern: i32) i32 {
    const str_mod = @import("tcl_string.zig");
    const lq = @import("tcl_list_quote.zig");
    var pat_ptr: u32 = 0;
    var pat_len: u32 = 0;
    const use_filter = blk: {
        if (pattern == 0) break :blk false;
        const ps = obj_ensure_string(pattern);
        pat_ptr = ps.ptr;
        pat_len = ps.len;
        break :blk ps.len > 0;
    };
    const t = find_table(arr);
    if (t == 0) return obj_new_string(0, 0);
    const cap = ar_cap(t);

    // Two-pass build: size, then assemble.  Each key is emitted via
    // ``list_elem_quote_nth`` so keys containing whitespace, braces,
    // backslashes or other list metacharacters round-trip through
    // ``lsort`` / ``foreach`` as a single element.  Without quoting,
    // a key like ``"testexprparser && !ieeeFloatingPoint"`` (added
    // by ``::tcltest::AddToSkippedBecause`` for compound constraints)
    // collapses into 4 list elements when callers pass the result
    // through any list-aware command — and ``cleanupTests`` then
    // traps with ``can't read "skippedBecause(!ieeeFloatingPoint)"``.
    var total: u32 = 0;
    var nonempty: u32 = 0;
    var i: u32 = 0;
    while (i < cap) : (i += 1) {
        const bucket = t + AR_HEADER_SIZE + i * AR_BUCKET_SIZE;
        const raw = read_i32(bucket);
        if (raw == 0 or raw == AR_TOMBSTONE) continue;
        const ep: u32 = @bitCast(raw);
        const el: u32 = @bitCast(read_i32(bucket + 4));
        if (use_filter) {
            // Match the raw key bytes via ``glob_match`` directly
            // rather than allocating a TclObj per probe — the
            // earlier ``string_match`` path leaked one key obj per
            // entry across both passes.
            if (!str_mod.glob_match(pat_ptr, pat_len, ep, el)) continue;
        }
        // Worst-case quoted length: ``2 * len + 2`` (matches
        // ``list_elem_quote_nth``); plus one separator space per
        // element after the first.
        total += el * 2 + 2;
        if (nonempty > 0) total += 1;
        nonempty += 1;
    }
    if (nonempty == 0) return obj_new_string(0, 0);
    const buf = alloc(total);
    if (buf == 0) return obj_new_string(0, 0);
    var off: u32 = 0;
    var written: u32 = 0;
    i = 0;
    while (i < cap) : (i += 1) {
        const bucket = t + AR_HEADER_SIZE + i * AR_BUCKET_SIZE;
        const raw = read_i32(bucket);
        if (raw == 0 or raw == AR_TOMBSTONE) continue;
        const ep: u32 = @bitCast(raw);
        const el: u32 = @bitCast(read_i32(bucket + 4));
        if (use_filter) {
            if (!str_mod.glob_match(pat_ptr, pat_len, ep, el)) continue;
        }
        if (written > 0) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
            off = lq.list_elem_quote_nth(buf, off, ep, el);
        } else {
            // First element keeps the leading-``#`` quoting rule so a
            // key starting with ``#`` round-trips through ``eval`` /
            // canonical-list contexts without the rest of the list
            // being parsed as a comment.  Subsequent elements pass
            // ``FLAG_DONT_QUOTE_HASH`` (via ``list_elem_quote_nth``)
            // because they can never sit at the script start.  Mirrors
            // ``UpdateStringOfList`` in upstream Tcl.
            off = lq.list_elem_quote(buf, off, ep, el);
        }
        written += 1;
    }
    return obj.obj_new_string_take(buf, off, total);
}

/// Scan the array directory for names matching the glob pattern
/// ``(pat_ptr, pat_len)`` and return a space-separated list of
/// matching array names as a TclObj.  Used by ``tcl_cmd_info.info_vars``
/// to include array variables in ``info vars`` results.
/// ``pat_len == 0`` means "no pattern — return all array names".
/// True iff *(name_ptr, name_len)* contains a ``::`` namespace
/// separator anywhere — used by :func:`array_top_level_names_matching`
/// to exclude qualified entries.  ``::``-prefixed bare-root names
/// are already stripped to their bare form by ``normalize_ns_name``
/// before they reach the directory, so a hit here is always a real
/// namespace qualifier (``::ns::arr``).
fn name_has_ns_separator(name_ptr: u32, name_len: u32) bool {
    if (name_len < 2) return false;
    const sp: [*]const u8 = @ptrFromInt(name_ptr);
    var i: u32 = 0;
    while (i + 1 < name_len) : (i += 1) {
        if (sp[i] == ':' and sp[i + 1] == ':') return true;
    }
    return false;
}

/// Like :func:`array_dir_names_matching` but skips qualified entries
/// (anything containing ``::``) so the result is exactly the set of
/// arrays visible at the top-level / global scope.  Used by
/// ``info vars`` (no proc frame) to surface ``array set arr {...}``
/// entries — these live in the array directory rather than the root
/// namespace's scalar ``var_table``, so the var-table-only scan
/// otherwise drops them silently.
pub fn array_top_level_names_matching(pat_ptr: u32, pat_len: u32) i32 {
    if (dir_buf == 0) return obj_new_string(0, 0);
    const str_mod = @import("tcl_string.zig");
    const use_filter = pat_len > 0;

    var total: u32 = 0;
    var count: u32 = 0;
    var i: u32 = 0;
    while (i < dir_cap) : (i += 1) {
        const bucket = dir_buf + i * DIR_BUCKET_SIZE;
        const name_ptr: u32 = @bitCast(read_i32(bucket));
        if (name_ptr == 0 or name_ptr == @as(u32, 0xFFFFFFFF)) continue;
        const name_len: u32 = @bitCast(read_i32(bucket + 4));
        if (name_ptr_is_local_internal(name_ptr, name_len)) continue;
        if (name_has_ns_separator(name_ptr, name_len)) continue;
        const table_ptr: u32 = @bitCast(read_i32(bucket + 12));
        if (table_ptr == 0) continue;
        if (use_filter and !str_mod.glob_match(pat_ptr, pat_len, name_ptr, name_len)) continue;
        if (count > 0) total += 1;
        total += name_len;
        count += 1;
    }
    if (total == 0) return obj_new_string(0, 0);

    const buf = alloc(total);
    var off: u32 = 0;
    var written: u32 = 0;
    i = 0;
    while (i < dir_cap) : (i += 1) {
        const bucket = dir_buf + i * DIR_BUCKET_SIZE;
        const name_ptr: u32 = @bitCast(read_i32(bucket));
        if (name_ptr == 0 or name_ptr == @as(u32, 0xFFFFFFFF)) continue;
        const name_len: u32 = @bitCast(read_i32(bucket + 4));
        if (name_ptr_is_local_internal(name_ptr, name_len)) continue;
        if (name_has_ns_separator(name_ptr, name_len)) continue;
        const table_ptr: u32 = @bitCast(read_i32(bucket + 12));
        if (table_ptr == 0) continue;
        if (use_filter and !str_mod.glob_match(pat_ptr, pat_len, name_ptr, name_len)) continue;
        if (written > 0) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        memcpy(buf + off, name_ptr, name_len);
        off += name_len;
        written += 1;
    }
    return obj_new_string(@bitCast(buf), @bitCast(off));
}

pub fn array_dir_names_matching(pat_ptr: u32, pat_len: u32) i32 {
    if (dir_buf == 0) return obj_new_string(0, 0);
    const str_mod = @import("tcl_string.zig");
    const use_filter = pat_len > 0;

    var total: u32 = 0;
    var count: u32 = 0;
    var i: u32 = 0;
    while (i < dir_cap) : (i += 1) {
        const bucket = dir_buf + i * DIR_BUCKET_SIZE;
        const name_ptr: u32 = @bitCast(read_i32(bucket));
        if (name_ptr == 0 or name_ptr == @as(u32, 0xFFFFFFFF)) continue;
        // Skip Phase-1 internal names (``::__local::*``) so
        // ``info vars`` etc. don't surface them.
        if (name_ptr_is_local_internal(name_ptr, @bitCast(read_i32(bucket + 4)))) continue;
        const name_len: u32 = @bitCast(read_i32(bucket + 4));
        const table_ptr: u32 = @bitCast(read_i32(bucket + 12));
        if (table_ptr == 0) continue; // array_unset'd
        if (use_filter and !str_mod.glob_match(pat_ptr, pat_len, name_ptr, name_len)) continue;
        if (count > 0) total += 1;
        total += name_len;
        count += 1;
    }
    if (total == 0) return obj_new_string(0, 0);

    const buf = alloc(total);
    var off: u32 = 0;
    var written: u32 = 0;
    i = 0;
    while (i < dir_cap) : (i += 1) {
        const bucket = dir_buf + i * DIR_BUCKET_SIZE;
        const name_ptr: u32 = @bitCast(read_i32(bucket));
        if (name_ptr == 0 or name_ptr == @as(u32, 0xFFFFFFFF)) continue;
        // Skip Phase-1 internal names (``::__local::*``) so
        // ``info vars`` etc. don't surface them.
        if (name_ptr_is_local_internal(name_ptr, @bitCast(read_i32(bucket + 4)))) continue;
        const name_len: u32 = @bitCast(read_i32(bucket + 4));
        const table_ptr: u32 = @bitCast(read_i32(bucket + 12));
        if (table_ptr == 0) continue;
        if (use_filter and !str_mod.glob_match(pat_ptr, pat_len, name_ptr, name_len)) continue;
        if (written > 0) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        memcpy(buf + off, name_ptr, name_len);
        off += name_len;
        written += 1;
    }
    return obj_new_string(@bitCast(buf), @bitCast(off));
}
