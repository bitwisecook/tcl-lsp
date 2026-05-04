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

const ht = @import("hash_table.zig");
const fnv1a = ht.fnv1a;

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
        if (read_i32(bucket) == 0) {
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
        if (ep == 0) continue;
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

/// Normalise a variable name that contains ``::`` but does not start
/// with ``::`` (e.g. ``ns::var``) by prepending ``::`` to produce a
/// fully-qualified name (``::ns::var``).  Names that are already
/// qualified or that contain no ``::`` (local arrays) are returned
/// unchanged.  Keeps the array directory consistent with Tcl's view
/// that unqualified namespace paths in global scope are equivalent
/// to their ``::``-prefixed forms, so ``info vars ::ns::T-*`` can
/// find arrays created via ``upvar #0 ns::T-$tag local``.
fn normalize_ns_name(name: i32) i32 {
    const sn = obj_ensure_string(name);
    if (sn.len < 2) return name;
    const sp: [*]const u8 = @ptrFromInt(sn.ptr);
    if (sp[0] == ':' and sp[1] == ':') return name; // already qualified
    var i: u32 = 0;
    while (i + 1 < sn.len) : (i += 1) {
        if (sp[i] == ':' and sp[i + 1] == ':') {
            const buf = alloc(2 + sn.len);
            // OOM — fall back to the unqualified form rather than a
            // null-pointer trap.  Caller's directory probe will miss
            // and either signal "no such array" cleanly or trip a
            // Tcl-level error from a higher allocator.
            if (buf == 0) return name;
            const d: [*]u8 = @ptrFromInt(buf);
            d[0] = ':';
            d[1] = ':';
            memcpy(buf + 2, sn.ptr, sn.len);
            return obj_new_string(@bitCast(buf), @bitCast(2 + sn.len));
        }
    }
    return name; // no '::' in name — local array, no normalization
}

fn find_or_create(name: i32) u32 {
    const n = normalize_ns_name(name);
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
        bucket_set_value(bucket, value);
        return value;
    }
    _ = ar_insert(t, sk.ptr, sk.len, hash, value);
    return value;
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
    const n = obj.list_count_elements(sp.ptr, sp.len);
    if (n == 0) return obj_new_string(0, 0);
    // Tcl silently tolerates odd-count lists here in practice
    // (stores ``n/2`` pairs) — but real Tcl raises
    // ``list must have an even number of elements``.  Err on the
    // side of tolerance so partial initialisation lists don't
    // trap an otherwise-progressing bundle.
    var i: i64 = 0;
    while (i + 1 < n) : (i += 2) {
        // ``list_element_at`` returns ``start`` as an *offset*
        // from the list payload pointer, not an absolute memory
        // address — add ``sp.ptr`` before wrapping in an obj.
        // Without this, ``fnv1a`` panics on the sub-2GB offset
        // when it calls ``@ptrFromInt(offset)`` with a value
        // the runtime's heap never mapped.
        const k_info = obj.list_element_at(sp.ptr, sp.len, i);
        const v_info = obj.list_element_at(sp.ptr, sp.len, i + 1);
        // Issue #317: ``obj_new_string_copy`` so the per-pair
        // TclObjs own their bytes.  Borrowing into ``sp`` (the
        // source pairs list) is unsafe because ``array_set``
        // stores ``v_obj`` in the array slot; once the source
        // list is released the borrowed bytes go stale and the
        // stored value reads as binary garbage on the next
        // ``array get``.  Copying also lets ``release_now``
        // reclaim the per-pair bufs cleanly.
        const k_obj = obj.obj_new_string_copy(sp.ptr + k_info.start, k_info.len);
        const v_obj = obj.obj_new_string_copy(sp.ptr + v_info.start, v_info.len);
        // ``array_set`` returns 0 when ``find_or_create`` flagged
        // a scalar/array name-conflict.  Stop iterating in that
        // case — every further call would re-raise the same
        // error and do no useful work.
        if (array_set(arr, k_obj, v_obj) == 0) {
            obj.tcl_obj_release(k_obj);
            obj.tcl_obj_release(v_obj);
            break;
        }
        // ``array_set`` retains ``v_obj`` for the slot via
        // ``bucket_set_value``; ``k_obj``'s bytes are copied by
        // the hash table.  Drop our creator-side refs so the
        // array's retain is the only live owner of ``v_obj`` and
        // ``k_obj`` can be freed at end of statement.
        obj.tcl_obj_release(k_obj);
        obj.tcl_obj_release(v_obj);
    }
    return obj_new_string(0, 0);
}

/// array_get arrName key — returns the stored value, or 0 (null
/// TclObj) if the array/key is missing.  Callers that need to
/// distinguish missing-vs-present should use ``array_element_exists``
/// first or ``info exists``.
pub export fn array_get(arr: i32, key: i32) i32 {
    const t = find_table(arr);
    if (t == 0) return 0;
    const sk = obj_ensure_string(key);
    const hash = fnv1a(sk.ptr, sk.len);
    if (ar_find(t, sk.ptr, sk.len, hash)) |bucket| {
        return read_i32(bucket + 12);
    }
    return 0;
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
    const n = normalize_ns_name(arr);
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
        // MM-B.5: release the value slot's reference before tombstoning.
        const old: i32 = read_i32(bucket + 12);
        write_i32(bucket, AR_TOMBSTONE);
        write_i32(bucket + 4, 0);
        write_i32(bucket + 8, 0);
        write_i32(bucket + 12, 0);
        ar_set_count(t, ar_count(t) - 1);
        if (old != 0) obj.tcl_obj_release(old);
    }
    return obj_new_int(0);
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
    const use_filter = pattern != 0 and blk: {
        const ps = obj_ensure_string(pattern);
        break :blk ps.len > 0;
    };
    const matches = struct {
        fn go(use: bool, pat: i32, key_ptr: u32, key_len: u32) bool {
            if (!use) return true;
            const k = obj_new_string(@bitCast(key_ptr), @bitCast(key_len));
            const r = str_mod.string_match(pat, k);
            return obj_get_int(r) != 0;
        }
    }.go;

    const t = find_table(arr);
    const cap = if (t == 0) 0 else ar_cap(t);

    // Two-pass accumulator: count + assemble.  First pass gathers
    // both the array directory entries (for global / namespace-
    // qualified arrays) and any frame-local "arr(key)" scalars
    // (set-1.26 — proc-local arrays live in the frame, not in the
    // directory).
    var total: u32 = 0;
    var nonempty: u32 = 0;

    // Sizing pass: directory entries.  Match Tcl ``lsort``-style raw
    // byte concat — array_names emits elements without re-quoting
    // (set-1.26 expects raw-byte output, including unescaped ``"``
    // mid-element).
    var i: u32 = 0;
    while (i < cap) : (i += 1) {
        const bucket = t + AR_HEADER_SIZE + i * AR_BUCKET_SIZE;
        const raw = read_i32(bucket);
        if (raw == 0 or raw == AR_TOMBSTONE) continue;
        const ep: u32 = @bitCast(raw);
        const el: u32 = @bitCast(read_i32(bucket + 4));
        if (!matches(use_filter, pattern, ep, el)) continue;
        total += el;
        if (nonempty > 0) total += 1;
        nonempty += 1;
    }

    // Sizing pass: frame-local ``<arr>(<elem>)`` scalars.
    const frames = @import("../interp/tcl_frames.zig");
    const arr_s = obj_ensure_string(arr);
    const SizeCtx = struct {
        total: u32,
        nonempty: u32,
        use: bool,
        pat: i32,
        match_fn: *const fn (use: bool, pat: i32, key_ptr: u32, key_len: u32) bool,
    };
    var size_ctx: SizeCtx = .{
        .total = total,
        .nonempty = nonempty,
        .use = use_filter,
        .pat = pattern,
        .match_fn = &matches,
    };
    const size_cb = struct {
        fn go(ctx: *anyopaque, elem_ptr: u32, elem_len: u32) void {
            const c: *SizeCtx = @ptrCast(@alignCast(ctx));
            if (!c.match_fn(c.use, c.pat, elem_ptr, elem_len)) return;
            c.total += elem_len;
            if (c.nonempty > 0) c.total += 1;
            c.nonempty += 1;
        }
    }.go;
    if (arr_s.len > 0) {
        frames.frame_iter_local_array(arr_s.ptr, arr_s.len, &size_ctx, size_cb);
    }
    total = size_ctx.total;
    nonempty = size_ctx.nonempty;

    if (nonempty == 0) return obj_new_string(0, 0);
    const buf = alloc(total);

    // Emit pass: directory entries.  Use ``list_elem_quote`` so
    // elements containing spaces / special chars get braced.
    var off: u32 = 0;
    var written: u32 = 0;
    i = 0;
    while (i < cap) : (i += 1) {
        const bucket = t + AR_HEADER_SIZE + i * AR_BUCKET_SIZE;
        const raw = read_i32(bucket);
        if (raw == 0 or raw == AR_TOMBSTONE) continue;
        const ep: u32 = @bitCast(raw);
        const el: u32 = @bitCast(read_i32(bucket + 4));
        if (!matches(use_filter, pattern, ep, el)) continue;
        if (written > 0) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        memcpy(buf + off, ep, el);
        off += el;
        written += 1;
    }

    // Emit pass: frame-local entries.
    const EmitCtx = struct {
        buf: u32,
        off: u32,
        written: u32,
        use: bool,
        pat: i32,
        match_fn: *const fn (use: bool, pat: i32, key_ptr: u32, key_len: u32) bool,
    };
    var emit_ctx: EmitCtx = .{
        .buf = buf,
        .off = off,
        .written = written,
        .use = use_filter,
        .pat = pattern,
        .match_fn = &matches,
    };
    const emit_cb = struct {
        fn go(ctx: *anyopaque, elem_ptr: u32, elem_len: u32) void {
            const c: *EmitCtx = @ptrCast(@alignCast(ctx));
            if (!c.match_fn(c.use, c.pat, elem_ptr, elem_len)) return;
            if (c.written > 0) {
                const d: [*]u8 = @ptrFromInt(c.buf + c.off);
                d[0] = ' ';
                c.off += 1;
            }
            memcpy(c.buf + c.off, elem_ptr, elem_len);
            c.off += elem_len;
            c.written += 1;
        }
    }.go;
    if (arr_s.len > 0) {
        frames.frame_iter_local_array(arr_s.ptr, arr_s.len, &emit_ctx, emit_cb);
    }
    off = emit_ctx.off;

    return obj_new_string(@bitCast(buf), @bitCast(off));
}

/// Scan the array directory for names matching the glob pattern
/// ``(pat_ptr, pat_len)`` and return a space-separated list of
/// matching array names as a TclObj.  Used by ``tcl_cmd_info.info_vars``
/// to include array variables in ``info vars`` results.
/// ``pat_len == 0`` means "no pattern — return all array names".
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
        if (name_ptr == 0) continue;
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
        if (name_ptr == 0) continue;
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
