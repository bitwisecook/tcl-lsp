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

const globals = @import("tcl_globals.zig");
const ht = @import("hash_table.zig");
const fnv1a = ht.fnv1a;

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
                write_i32(bucket + 12, value);
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

fn find_or_create(name: i32) u32 {
    const sn = obj_ensure_string(name);
    const hash = fnv1a(sn.ptr, sn.len);
    if (dir_find(sn.ptr, sn.len, hash)) |bucket| {
        return @bitCast(read_i32(bucket + 12));
    }
    const t = ar_new();
    dir_insert(sn.ptr, sn.len, hash, t);
    return t;
}

fn find_table(name: i32) u32 {
    const sn = obj_ensure_string(name);
    if (dir_buf == 0) return 0;
    const hash = fnv1a(sn.ptr, sn.len);
    if (dir_find(sn.ptr, sn.len, hash)) |bucket| {
        return @bitCast(read_i32(bucket + 12));
    }
    return 0;
}

// --- Public exports ----------------------------------------------------

/// array_set arrName key value — stores value under key in the array
/// named arrName.  Creates the array on first write.  Returns value.
pub export fn array_set(arr: i32, key: i32, value: i32) i32 {
    const t = find_or_create(arr);
    const sk = obj_ensure_string(key);
    const hash = fnv1a(sk.ptr, sk.len);
    if (ar_find(t, sk.ptr, sk.len, hash)) |bucket| {
        write_i32(bucket + 12, value);
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
        const k_obj = obj_new_string(@bitCast(sp.ptr + k_info.start), @bitCast(k_info.len));
        const v_obj = obj_new_string(@bitCast(sp.ptr + v_info.start), @bitCast(v_info.len));
        _ = array_set(arr, k_obj, v_obj);
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
    const sn = obj_ensure_string(arr);
    if (dir_buf == 0) return obj_new_int(0);
    const hash = fnv1a(sn.ptr, sn.len);
    if (dir_find(sn.ptr, sn.len, hash)) |bucket| {
        // Replace the array's table with a fresh empty one rather
        // than trying to free — the bump allocator can't free.
        const fresh = ar_new();
        write_i32(bucket + 12, @bitCast(fresh));
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
        write_i32(bucket, AR_TOMBSTONE);
        write_i32(bucket + 4, 0);
        write_i32(bucket + 8, 0);
        write_i32(bucket + 12, 0);
        ar_set_count(t, ar_count(t) - 1);
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
    const t = find_table(arr);
    if (t == 0) return obj_new_string(0, 0);
    const cap = ar_cap(t);

    // Resolve the pattern filter up-front.  Empty pattern → match
    // everything.  Import ``string_match`` lazily to avoid a
    // circular ``tcl_string`` import at module init.
    const str_mod = @import("tcl_string.zig");
    const use_filter = pattern != 0 and blk: {
        const ps = obj_ensure_string(pattern);
        break :blk ps.len > 0;
    };

    // Inline matcher wrapper that keeps the hash-table walk tidy.
    const matches = struct {
        fn go(use: bool, pat: i32, key_ptr: u32, key_len: u32) bool {
            if (!use) return true;
            const k = obj_new_string(@bitCast(key_ptr), @bitCast(key_len));
            // ``string_match`` returns a TclObj wrapping 1 or 0.
            const r = str_mod.string_match(pat, k);
            return obj_get_int(r) != 0;
        }
    }.go;

    // First pass: compute required buffer size.  Skip empty and
    // tombstoned slots and slots whose key doesn't match the
    // pattern.
    var total: u32 = 0;
    var nonempty: u32 = 0;
    var i: u32 = 0;
    while (i < cap) : (i += 1) {
        const bucket = t + AR_HEADER_SIZE + i * AR_BUCKET_SIZE;
        const raw = read_i32(bucket);
        if (raw == 0 or raw == AR_TOMBSTONE) continue;
        const ep: u32 = @bitCast(raw);
        const el: u32 = @bitCast(read_i32(bucket + 4));
        if (!matches(use_filter, pattern, ep, el)) continue;
        total += el;
        if (nonempty > 0) total += 1; // separator
        nonempty += 1;
    }
    if (nonempty == 0) return obj_new_string(0, 0);
    const buf = alloc(total);
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
    return obj_new_string(@bitCast(buf), @bitCast(off));
}
