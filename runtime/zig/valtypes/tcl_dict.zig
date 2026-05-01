// Dict operations: dict_create, dict_get, dict_set, dict_exists,
// dict_keys, dict_values, dict_size.

const obj = @import("tcl_obj.zig");
const ht = @import("hash_table.zig");
const alloc = obj.alloc;
const memcpy = obj.memcpy;
const obj_ensure_string = obj.obj_ensure_string;
const obj_new_string = obj.obj_new_string;
const obj_new_int = obj.obj_new_int;
const obj_new_string_copy = obj.obj_new_string_copy;
const str_cmp = obj.str_cmp;
const list_count_elements = obj.list_count_elements;
const list_element_at = obj.list_element_at;
const list_elem_quote = obj.list_elem_quote;
const list_elem_quote_nth = obj.list_elem_quote_nth;

// -------------------------------------------------------------------
// Hash side-cache for dict objects.
//
// A TclObj whose string rep is a list-encoded dict can carry an
// optional hash side-cache for O(1) ``dict get`` / ``dict set``
// lookup.  The list rep stays canonical; the hash is built lazily on
// first lookup and invalidated whenever the list rep is rewritten.
//
// The cache is a single heap allocation pointed to by
// ``OBJ_DICT_EXT`` (offset 28 in the obj header, previously
// padding).  Layout::
//
//    offset 0  hash_buf   (u32) — bucket array (cap * BUCKET_SIZE bytes)
//    offset 4  hash_cap   (u32) — bucket count, power of 2
//    offset 8  hash_count (u32) — live entries
//
// Each bucket is 16 bytes::
//
//    offset 0  key_ptr    (u32) — heap copy of the key bytes; 0 = empty,
//                                 1 = tombstone
//    offset 4  key_len    (u32)
//    offset 8  hash       (u32) — FNV-1a of the key bytes
//    offset 12 value      (i32) — TclObj handle of the value (retained)
//
// On cache invalidation we walk the buckets, ``free_sized`` each
// key copy, ``tcl_obj_release`` each value, and ``free_sized`` the
// bucket buffer + the DictExt header.
//
// **Refcount discipline**: when a value is inserted or replaced, the
// hash retains the new handle and releases the old.  On destroy
// (cache invalidation OR obj release), every bucket releases its
// stored value.  This balances: external callers that
// ``tcl_obj_release`` the dict obj will eventually drop the dict's
// rc to 0; ``release_now`` (in tcl_obj.zig) calls
// ``dict_destroy_ext`` which walks the buckets and releases each
// value.

pub const DICT_EXT_SIZE: u32 = 12;
pub const DICT_EXT_BUF: u32 = 0;
pub const DICT_EXT_CAP: u32 = 4;
pub const DICT_EXT_COUNT: u32 = 8;

pub const DICT_BUCKET_SIZE: u32 = 16;
pub const DICT_BUCKET_KEY_PTR: u32 = 0;
pub const DICT_BUCKET_KEY_LEN: u32 = 4;
pub const DICT_BUCKET_HASH: u32 = 8;
pub const DICT_BUCKET_VALUE: u32 = 12;
pub const DICT_INITIAL_CAP: u32 = 8;
pub const DICT_TOMBSTONE: u32 = 1;

/// Free a hash side-cache.  Releases every bucket's value handle,
/// frees each key buffer, frees the bucket array, and frees the ext
/// struct itself.
pub fn dict_destroy_ext(ext_addr: u32) void {
    if (ext_addr == 0) return;
    const buf: u32 = @bitCast(obj.read_i32(ext_addr + DICT_EXT_BUF));
    const cap: u32 = @bitCast(obj.read_i32(ext_addr + DICT_EXT_CAP));
    if (buf != 0) {
        var i: u32 = 0;
        while (i < cap) : (i += 1) {
            const base = buf + i * DICT_BUCKET_SIZE;
            const kp: u32 = @bitCast(obj.read_i32(base + DICT_BUCKET_KEY_PTR));
            if (kp != 0 and kp != DICT_TOMBSTONE) {
                const kl: u32 = @bitCast(obj.read_i32(base + DICT_BUCKET_KEY_LEN));
                obj.free_sized(kp, kl);
                const v: i32 = obj.read_i32(base + DICT_BUCKET_VALUE);
                obj.tcl_obj_release(v);
            }
        }
        obj.free_sized(buf, cap * DICT_BUCKET_SIZE);
    }
    obj.free_sized(ext_addr, DICT_EXT_SIZE);
}

/// Detach (and free) any hash side-cache attached to *dict_obj*.
/// Called whenever the list rep is rewritten — the cached
/// (key → value) bindings reference the old layout and must not
/// outlive it.
pub fn dict_invalidate_cache(dict_obj: i32) void {
    if (dict_obj == 0) return;
    if (obj.is_immediate(dict_obj)) return;
    const addr: u32 = @intCast(dict_obj);
    const ext: u32 = @bitCast(obj.read_i32(addr + obj.OBJ_DICT_EXT));
    if (ext == 0) return;
    obj.write_i32(addr + obj.OBJ_DICT_EXT, 0);
    dict_destroy_ext(ext);
}

fn dict_alloc_buckets(cap: u32) u32 {
    // PR #237 review: ``alloc`` returns 0 on OOM.  Without a check
    // the loop below would dereference null and silently corrupt
    // the bucket store; the guard converts OOM into a propagated
    // 0 sentinel that callers reject.
    const buf = alloc(cap * DICT_BUCKET_SIZE);
    if (buf == 0) return 0;
    var i: u32 = 0;
    while (i < cap) : (i += 1) {
        const base = buf + i * DICT_BUCKET_SIZE;
        obj.write_i32(base + DICT_BUCKET_KEY_PTR, 0);
        obj.write_i32(base + DICT_BUCKET_KEY_LEN, 0);
        obj.write_i32(base + DICT_BUCKET_HASH, 0);
        obj.write_i32(base + DICT_BUCKET_VALUE, 0);
    }
    return buf;
}

fn dict_ext_alloc() u32 {
    const ext = alloc(DICT_EXT_SIZE);
    if (ext == 0) return 0;
    const buckets = dict_alloc_buckets(DICT_INITIAL_CAP);
    if (buckets == 0) {
        obj.free_sized(ext, DICT_EXT_SIZE);
        return 0;
    }
    obj.write_i32(ext + DICT_EXT_BUF, @bitCast(buckets));
    obj.write_i32(ext + DICT_EXT_CAP, @bitCast(DICT_INITIAL_CAP));
    obj.write_i32(ext + DICT_EXT_COUNT, 0);
    return ext;
}

/// Clone an existing hash side-cache.  Allocates a fresh ext + bucket
/// buffer of the same capacity and copies every live entry, retaining
/// each value handle so the source cache and clone are independent
/// owners of the values.  Used when ``dict_set`` produces a new dict
/// from a shared (rc>1) source — without cloning, the cache transfer
/// would strip the source dict of its cache, leaving any other holder
/// of the source TclObj with cache-cold reads.
fn dict_clone_ext(src_ext: u32) u32 {
    const src_buf: u32 = @bitCast(obj.read_i32(src_ext + DICT_EXT_BUF));
    const src_cap: u32 = @bitCast(obj.read_i32(src_ext + DICT_EXT_CAP));
    const src_count: u32 = @bitCast(obj.read_i32(src_ext + DICT_EXT_COUNT));
    const dst = alloc(DICT_EXT_SIZE);
    if (dst == 0) return 0;
    const new_buckets = dict_alloc_buckets(src_cap);
    if (new_buckets == 0) {
        obj.free_sized(dst, DICT_EXT_SIZE);
        return 0;
    }
    obj.write_i32(dst + DICT_EXT_BUF, @bitCast(new_buckets));
    obj.write_i32(dst + DICT_EXT_CAP, @bitCast(src_cap));
    obj.write_i32(dst + DICT_EXT_COUNT, @bitCast(src_count));
    const dst_buf: u32 = @bitCast(obj.read_i32(dst + DICT_EXT_BUF));
    var i: u32 = 0;
    while (i < src_cap) : (i += 1) {
        const sb = src_buf + i * DICT_BUCKET_SIZE;
        const skp: u32 = @bitCast(obj.read_i32(sb + DICT_BUCKET_KEY_PTR));
        if (skp == 0 or skp == DICT_TOMBSTONE) continue;
        const skl: u32 = @bitCast(obj.read_i32(sb + DICT_BUCKET_KEY_LEN));
        const skh: u32 = @bitCast(obj.read_i32(sb + DICT_BUCKET_HASH));
        const sv: i32 = obj.read_i32(sb + DICT_BUCKET_VALUE);
        const mask = src_cap - 1;
        var idx = skh & mask;
        while (true) {
            const db = dst_buf + idx * DICT_BUCKET_SIZE;
            if (obj.read_i32(db + DICT_BUCKET_KEY_PTR) == 0) {
                const nbuf = alloc(skl);
                if (nbuf == 0) {
                    // OOM mid-clone: tear down what we built.
                    dict_destroy_ext(dst);
                    return 0;
                }
                memcpy(nbuf, skp, skl);
                obj.write_i32(db + DICT_BUCKET_KEY_PTR, @bitCast(nbuf));
                obj.write_i32(db + DICT_BUCKET_KEY_LEN, @bitCast(skl));
                obj.write_i32(db + DICT_BUCKET_HASH, @bitCast(skh));
                obj.write_i32(db + DICT_BUCKET_VALUE, sv);
                obj.tcl_obj_retain(sv);
                break;
            }
            idx = (idx + 1) & mask;
        }
    }
    return dst;
}

/// Look up *key* in a hash side-cache.  Returns the bucket address
/// (so callers can read the value without re-hashing) or null.
fn dict_hash_find(buf: u32, cap: u32, kp: u32, kl: u32, h: u32) ?u32 {
    if (buf == 0 or cap == 0) return null;
    const mask = cap - 1;
    var idx = h & mask;
    var probes: u32 = 0;
    while (probes < cap) : (probes += 1) {
        const base = buf + idx * DICT_BUCKET_SIZE;
        const ekp: u32 = @bitCast(obj.read_i32(base + DICT_BUCKET_KEY_PTR));
        if (ekp == 0) return null;
        if (ekp != DICT_TOMBSTONE) {
            const ekl: u32 = @bitCast(obj.read_i32(base + DICT_BUCKET_KEY_LEN));
            const ekh: u32 = @bitCast(obj.read_i32(base + DICT_BUCKET_HASH));
            if (ekh == h and ekl == kl) {
                const sp: [*]const u8 = @ptrFromInt(ekp);
                const np: [*]const u8 = @ptrFromInt(kp);
                var match = true;
                var k: u32 = 0;
                while (k < kl) : (k += 1) {
                    if (sp[k] != np[k]) {
                        match = false;
                        break;
                    }
                }
                if (match) return base;
            }
        }
        idx = (idx + 1) & mask;
    }
    return null;
}

/// Insert (or replace) a key→value mapping in the side-cache.  The
/// bucket retains the value; if a previous entry existed for this
/// key, its value is released and its key buffer is reused.  Callers
/// that have already verified absence-via-find can skip the
/// replace-detection probe.
fn dict_hash_insert(ext_addr: u32, kp: u32, kl: u32, h: u32, value: i32) void {
    var buf: u32 = @bitCast(obj.read_i32(ext_addr + DICT_EXT_BUF));
    var cap: u32 = @bitCast(obj.read_i32(ext_addr + DICT_EXT_CAP));
    var count: u32 = @bitCast(obj.read_i32(ext_addr + DICT_EXT_COUNT));
    if (count * 4 >= cap * 3) {
        // Grow at 75% load factor.  Walk old buckets and re-hash.
        const new_cap = cap * 2;
        const new_buf = dict_alloc_buckets(new_cap);
        if (new_buf == 0) {
            // OOM during grow: leave the table at the current
            // capacity.  Side-cache mutators fail soft — the next
            // ``dict_get`` misses the key and falls back to the
            // list-rep walk; correctness is preserved.
            return;
        }
        const old_mask = cap - 1;
        _ = old_mask;
        var i: u32 = 0;
        while (i < cap) : (i += 1) {
            const old_base = buf + i * DICT_BUCKET_SIZE;
            const ekp: u32 = @bitCast(obj.read_i32(old_base + DICT_BUCKET_KEY_PTR));
            if (ekp == 0 or ekp == DICT_TOMBSTONE) continue;
            const ekl: u32 = @bitCast(obj.read_i32(old_base + DICT_BUCKET_KEY_LEN));
            const ekh: u32 = @bitCast(obj.read_i32(old_base + DICT_BUCKET_HASH));
            const ev: i32 = obj.read_i32(old_base + DICT_BUCKET_VALUE);
            const new_mask = new_cap - 1;
            var idx = ekh & new_mask;
            while (true) {
                const nb = new_buf + idx * DICT_BUCKET_SIZE;
                if (obj.read_i32(nb + DICT_BUCKET_KEY_PTR) == 0) {
                    obj.write_i32(nb + DICT_BUCKET_KEY_PTR, @bitCast(ekp));
                    obj.write_i32(nb + DICT_BUCKET_KEY_LEN, @bitCast(ekl));
                    obj.write_i32(nb + DICT_BUCKET_HASH, @bitCast(ekh));
                    obj.write_i32(nb + DICT_BUCKET_VALUE, ev);
                    break;
                }
                idx = (idx + 1) & new_mask;
            }
        }
        obj.free_sized(buf, cap * DICT_BUCKET_SIZE);
        buf = new_buf;
        cap = new_cap;
        obj.write_i32(ext_addr + DICT_EXT_BUF, @bitCast(buf));
        obj.write_i32(ext_addr + DICT_EXT_CAP, @bitCast(cap));
    }

    const mask = cap - 1;
    var idx = h & mask;
    var first_tombstone: u32 = 0;
    var probes: u32 = 0;
    while (probes < cap) : (probes += 1) {
        const base = buf + idx * DICT_BUCKET_SIZE;
        const ekp: u32 = @bitCast(obj.read_i32(base + DICT_BUCKET_KEY_PTR));
        if (ekp == 0) {
            // End of probe chain — insert here (or in the first
            // tombstone seen along the way).
            const target = if (first_tombstone != 0) first_tombstone else base;
            const nbuf = alloc(kl);
            if (nbuf == 0) {
                // OOM on key copy: leave the bucket empty so
                // probes don't see a half-populated entry (a
                // ``key_ptr=0`` would alias the empty-slot sentinel
                // and silently drop later probes).
                return;
            }
            memcpy(nbuf, kp, kl);
            obj.write_i32(target + DICT_BUCKET_KEY_PTR, @bitCast(nbuf));
            obj.write_i32(target + DICT_BUCKET_KEY_LEN, @bitCast(kl));
            obj.write_i32(target + DICT_BUCKET_HASH, @bitCast(h));
            obj.write_i32(target + DICT_BUCKET_VALUE, value);
            obj.tcl_obj_retain(value);
            count += 1;
            obj.write_i32(ext_addr + DICT_EXT_COUNT, @bitCast(count));
            return;
        }
        if (ekp == DICT_TOMBSTONE) {
            if (first_tombstone == 0) first_tombstone = base;
        } else {
            const ekl: u32 = @bitCast(obj.read_i32(base + DICT_BUCKET_KEY_LEN));
            const ekh: u32 = @bitCast(obj.read_i32(base + DICT_BUCKET_HASH));
            if (ekh == h and ekl == kl) {
                const sp: [*]const u8 = @ptrFromInt(ekp);
                const np: [*]const u8 = @ptrFromInt(kp);
                var match = true;
                var k: u32 = 0;
                while (k < kl) : (k += 1) {
                    if (sp[k] != np[k]) {
                        match = false;
                        break;
                    }
                }
                if (match) {
                    // Replace: release the old value, retain the new.
                    const old_v: i32 = obj.read_i32(base + DICT_BUCKET_VALUE);
                    obj.write_i32(base + DICT_BUCKET_VALUE, value);
                    obj.tcl_obj_retain(value);
                    obj.tcl_obj_release(old_v);
                    return;
                }
            }
        }
        idx = (idx + 1) & mask;
    }
    // Probe table fully occupied (load factor 1.0 — should be impossible
    // given the 75% grow guard above).  Fall through silently — the
    // missing entry will be picked up the next time the cache is
    // rebuilt.
}

/// Build (or rebuild) the hash side-cache from the canonical list rep.
/// Skipped when the cache is already attached.  Returns the ext addr
/// (or 0 if dict is immediate / 0).
fn dict_ensure_cache(dict_obj: i32) u32 {
    if (dict_obj == 0) return 0;
    if (obj.is_immediate(dict_obj)) return 0;
    const addr: u32 = @intCast(dict_obj);
    var ext: u32 = @bitCast(obj.read_i32(addr + obj.OBJ_DICT_EXT));
    if (ext != 0) return ext;
    ext = dict_ext_alloc();
    if (ext == 0) {
        // OOM during cache build: skip the side-cache and let the
        // caller fall back to a list-rep walk.  Don't write a
        // sentinel into ``OBJ_DICT_EXT`` (it's already 0) so the
        // next ``dict_ensure_cache`` retries cleanly.
        return 0;
    }
    obj.write_i32(addr + obj.OBJ_DICT_EXT, @bitCast(ext));
    const sd = obj_ensure_string(dict_obj);
    if (sd.len == 0) return ext;
    const n = list_count_elements(sd.ptr, sd.len);
    var idx: i64 = 0;
    while (idx + 1 < n) : (idx += 2) {
        const k = list_element_at(sd.ptr, sd.len, idx);
        const v = list_element_at(sd.ptr, sd.len, idx + 1);
        // Materialise the value as a TclObj.  ``obj_new_string_copy``
        // dups the bytes so it survives a later list-rep rewrite.
        const v_obj = obj_new_string_copy(sd.ptr + v.start, v.len);
        const h = ht.fnv1a(sd.ptr + k.start, k.len);
        // The value handle starts at rc=1 from ``obj_new_string_copy``;
        // ``dict_hash_insert`` will retain (rc=2), then we release the
        // initial +1 so the bucket is the sole owner (rc=1).
        dict_hash_insert(ext, sd.ptr + k.start, k.len, h, v_obj);
        obj.tcl_obj_release(v_obj);
    }
    return ext;
}

// Exported: dict create — create an empty dict.
pub export fn dict_create() i32 {
    return obj_new_string(0, 0);
}

// Exported: dict get — look up a key in a dict, return its value.
pub export fn dict_get(dict: i32, key: i32) i32 {
    const sk = obj_ensure_string(key);
    // Hash side-cache fast path.  Build the cache lazily on the
    // first lookup against this dict; subsequent lookups are O(1)
    // average.  Cache is invalidated whenever the list rep is
    // rewritten by a mutator (``dict_set`` replace branch /
    // ``dict_unset``).
    const ext = dict_ensure_cache(dict);
    if (ext != 0) {
        const buf: u32 = @bitCast(obj.read_i32(ext + DICT_EXT_BUF));
        const cap: u32 = @bitCast(obj.read_i32(ext + DICT_EXT_CAP));
        const h = ht.fnv1a(sk.ptr, sk.len);
        if (dict_hash_find(buf, cap, sk.ptr, sk.len, h)) |bucket| {
            const v: i32 = obj.read_i32(bucket + DICT_BUCKET_VALUE);
            // The bucket owns +1; return a fresh +1 so callers can
            // store / release as usual without disturbing the cache.
            obj.tcl_obj_retain(v);
            return v;
        }
        return obj_new_string(0, 0);
    }
    // Fallback (immediate handle, etc.) — walk the list rep directly.
    const sd = obj_ensure_string(dict);
    const n = list_count_elements(sd.ptr, sd.len);
    var idx: i64 = 0;
    while (idx < n - 1) : (idx += 2) {
        const k = list_element_at(sd.ptr, sd.len, idx);
        if (str_cmp(sd.ptr + k.start, k.len, sk.ptr, sk.len) == 0) {
            const v = list_element_at(sd.ptr, sd.len, idx + 1);
            return obj_new_string_copy(sd.ptr + v.start, v.len);
        }
    }
    return obj_new_string(0, 0);
}

// Exported: dict set — set a key in a dict, return the updated dict.
//
// Fast path mirrors ``tcl_cmd_lappend``: when the dict is non-empty,
// we own the byte buffer (refcount == 1 with a recorded capacity),
// and the key is new, append ``" key value"`` in place with
// geometric capacity growth.  This converts a ``for {} {<N>} {dict
// set d k$i $i}`` builder loop from O(N) per call (full buffer copy
// on every append) to O(1) amortised.  The slow path retains the
// previous full-rebuild behaviour for shared dicts and key replace.
//
// Hash side-cache: when one is attached, the in-place fast path also
// inserts the new key/value into the hash so subsequent ``dict get``
// stays O(1).  The replace / shared-rebuild paths invalidate the
// cache wholesale because they construct a new TclObj and the old
// cache's bucket pointers reference the now-stale list rep.
pub export fn dict_set(dict: i32, key: i32, value: i32) i32 {
    const sd = obj_ensure_string(dict);
    const sk = obj_ensure_string(key);
    const sv = obj_ensure_string(value);
    // Eagerly build / reuse the hash side-cache so the existence
    // check is O(1) instead of an O(N) list-rep walk.  Without this
    // the build phase of a ``for {} {<N>} {dict set d k$i $i}`` loop
    // is O(N^2) — every iteration scans the entire growing list to
    // confirm the key is new.
    const h = ht.fnv1a(sk.ptr, sk.len);
    const ext = dict_ensure_cache(dict);
    if (ext != 0) {
        const buf: u32 = @bitCast(obj.read_i32(ext + DICT_EXT_BUF));
        const cap: u32 = @bitCast(obj.read_i32(ext + DICT_EXT_CAP));
        if (dict_hash_find(buf, cap, sk.ptr, sk.len, h) != null) {
            // Existing key — full rebuild for canonical list rep.
            // Cache invalidated because bucket key pointers will
            // dangle into the freed buffer.
            const new = dict_rebuild_with_value_full_scan(sd.ptr, sd.len, sk.ptr, sk.len, sv.ptr, sv.len);
            dict_invalidate_cache(dict);
            return new;
        }
        // Cache present and confirms key absence — skip the linear
        // scan and go straight to the append path.
    } else {
        // Cache unavailable (immediate handle or alloc failure) —
        // fall back to a linear list-rep scan to detect existing key.
        const n = list_count_elements(sd.ptr, sd.len);
        var idx: i64 = 0;
        while (idx < n - 1) : (idx += 2) {
            const k = list_element_at(sd.ptr, sd.len, idx);
            if (str_cmp(sd.ptr + k.start, k.len, sk.ptr, sk.len) == 0) {
                const new = dict_rebuild_with_value(sd.ptr, sd.len, n, idx, sv.ptr, sv.len);
                return new;
            }
        }
    }
    // Key absent — try the in-place geometric append fast path.
    // ``is_immediate`` guard (PR #237 review): with S6.4, every small
    // integer is a tagged immediate.  ``@intCast(dict)`` on a tagged
    // handle yields a low integer (e.g. ``11`` for the immediate form
    // of ``5``); reading from ``addr + OBJ_REFCOUNT`` then deref's a
    // wasm data-segment / null-padding address, either trapping or
    // returning junk that the cap/rc check would silently believe.
    // The earlier ``obj_ensure_string`` / ``dict_ensure_cache`` calls
    // already cope with immediates; the in-place path needs the same
    // guard.
    if (sd.len > 0 and dict != 0 and !obj.is_immediate(dict)) {
        const addr: u32 = @intCast(dict);
        const rc = obj.read_i32(addr + obj.OBJ_REFCOUNT);
        const tag = obj.read_i32(addr + obj.OBJ_TYPE_TAG);
        const cap: u32 = @bitCast(obj.read_i32(addr + obj.OBJ_STR_CAP));
        if (rc == 1 and tag == obj.TYPE_STRING and cap > 0) {
            const tail_max: u32 = 1 + sk.len * 2 + 1 + sv.len * 2 + 8;
            const want: u32 = sd.len + tail_max;
            var buf: u32 = sd.ptr;
            var new_cap: u32 = cap;
            if (cap < want) {
                new_cap = if (cap == 0) 16 else cap * 2;
                while (new_cap < want) new_cap *= 2;
                const nbuf = obj.alloc(new_cap);
                if (nbuf == 0) return dict;
                memcpy(nbuf, sd.ptr, sd.len);
                obj.free_sized(sd.ptr, cap);
                buf = nbuf;
                obj.write_i32(addr + obj.OBJ_STR_PTR, @bitCast(nbuf));
                obj.write_i32(addr + obj.OBJ_STR_CAP, @bitCast(new_cap));
            }
            const d: [*]u8 = @ptrFromInt(buf + sd.len);
            d[0] = ' ';
            var off = list_elem_quote_nth(buf, sd.len + 1, sk.ptr, sk.len);
            const d2: [*]u8 = @ptrFromInt(buf + off);
            d2[0] = ' ';
            off = list_elem_quote_nth(buf, off + 1, sv.ptr, sv.len);
            obj.write_i32(addr + obj.OBJ_STR_LEN, @bitCast(off));
            // In-place mutation preserves the cache's bucket key
            // pointers (they index into the old buffer which we just
            // grew or reused — fast path memcpys old contents).  But
            // when we GREW (allocated a new buffer), bucket key
            // pointers are now dangling.  Invalidate to be safe.
            // Bucket keys / values are independent heap copies (see
            // ``dict_hash_insert``), so they survive a list-rep buffer
            // grow.  Insert into the cache unconditionally — no need
            // to invalidate just because ``buf != sd.ptr``.
            if (ext != 0) {
                const v_obj = obj_new_string_copy(sv.ptr, sv.len);
                dict_hash_insert(ext, sk.ptr, sk.len, h, v_obj);
                obj.tcl_obj_release(v_obj);
            }
            return dict;
        }
    }
    const new = dict_append_pair(sd.ptr, sd.len, sk.ptr, sk.len, sv.ptr, sv.len);
    // Propagate the hash side-cache to the new dict.  Strategy
    // depends on whether the source is uniquely owned:
    //
    // * **rc == 1 (sole owner)** — *transfer* the cache.  No other
    //   holder will read the source obj after this call returns, so
    //   we save the O(N) clone cost and detach + reattach in O(1).
    //
    // * **rc > 1 (shared)** — *clone* the cache.  Other holders of
    //   the source dict still expect cache-warm reads of their copy.
    //   Transferring would cache-cold-strip them.  Cloning is O(N)
    //   per shared mutation but preserves COW symmetry.
    //
    // Either way the new dict ends up with a cache containing the
    // just-appended (k, v) pair, so the next mutation on the new
    // dict starts cache-warm.
    if (new != 0 and !obj.is_immediate(new) and ext != 0 and dict != 0 and !obj.is_immediate(dict)) {
        const dict_addr: u32 = @intCast(dict);
        const new_addr: u32 = @intCast(new);
        const dict_rc = obj.read_i32(dict_addr + obj.OBJ_REFCOUNT);
        var target_ext: u32 = 0;
        if (dict_rc <= 1) {
            // Transfer.
            obj.write_i32(dict_addr + obj.OBJ_DICT_EXT, 0);
            target_ext = ext;
        } else {
            // Clone — the source keeps its own cache.
            target_ext = dict_clone_ext(ext);
        }
        obj.write_i32(new_addr + obj.OBJ_DICT_EXT, @bitCast(target_ext));
        const v_obj = obj_new_string_copy(sv.ptr, sv.len);
        dict_hash_insert(target_ext, sk.ptr, sk.len, h, v_obj);
        obj.tcl_obj_release(v_obj);
    } else {
        dict_invalidate_cache(dict);
    }
    return new;
}

/// Variant of :func:`dict_rebuild_with_value` that rescans the list
/// rep to locate the key — used by the cache-hit path of
/// :func:`dict_set` where the cache lookup confirmed key presence
/// but the rebuild routine wants a list-rep index.
fn dict_rebuild_with_value_full_scan(sd_ptr: u32, sd_len: u32, kp: u32, kl: u32, vp: u32, vl: u32) i32 {
    const n = list_count_elements(sd_ptr, sd_len);
    var idx: i64 = 0;
    while (idx + 1 < n) : (idx += 2) {
        const k = list_element_at(sd_ptr, sd_len, idx);
        if (str_cmp(sd_ptr + k.start, k.len, kp, kl) == 0) {
            return dict_rebuild_with_value(sd_ptr, sd_len, n, idx, vp, vl);
        }
    }
    // Cache claimed key present but list rep doesn't have it — should
    // be impossible.  Fall through to append.
    return dict_append_pair(sd_ptr, sd_len, kp, kl, vp, vl);
}

/// ``dict unset DICT KEY`` — drop the matching key/value pair.
/// If the key is absent the dict is returned unchanged.  Used by
/// ``dict unset`` and by ``dict update`` when the bound local var
/// is unset inside the body.
pub export fn dict_unset(dict: i32, key: i32) i32 {
    const sd = obj_ensure_string(dict);
    const sk = obj_ensure_string(key);
    if (sd.len == 0) return dict;
    const n = list_count_elements(sd.ptr, sd.len);
    var idx: i64 = 0;
    while (idx + 1 < n) : (idx += 2) {
        const k = list_element_at(sd.ptr, sd.len, idx);
        if (str_cmp(sd.ptr + k.start, k.len, sk.ptr, sk.len) == 0) {
            const new = dict_rebuild_without_pair(sd.ptr, sd.len, n, idx);
            dict_invalidate_cache(dict);
            return new;
        }
    }
    return dict;
}

fn dict_rebuild_without_pair(sd_ptr: u32, sd_len: u32, n: i64, target_idx: i64) i32 {
    const cap: u32 = sd_len * 2 + 16;
    const buf = alloc(cap);
    // OOM: under the libc-malloc routing alloc() returns 0 on
    // failure.  Without this guard the @ptrFromInt(buf + off)
    // writes below would deref address 0 and trap; instead, return
    // an empty TclObj string so the caller's ``dict unset`` becomes
    // a benign no-op rather than an unrecoverable trap.
    if (buf == 0) return obj_new_string(0, 0);
    var off: u32 = 0;
    var idx: i64 = 0;
    while (idx < n) : (idx += 1) {
        if (idx == target_idx or idx == target_idx + 1) continue;
        if (off > 0) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        const elem = list_element_at(sd_ptr, sd_len, idx);
        if (elem.braced) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = '{';
            off += 1;
            memcpy(buf + off, sd_ptr + elem.start, elem.len);
            off += elem.len;
            const d2: [*]u8 = @ptrFromInt(buf + off);
            d2[0] = '}';
            off += 1;
        } else {
            memcpy(buf + off, sd_ptr + elem.start, elem.len);
            off += elem.len;
        }
    }
    // Claim ownership of ``buf`` via OBJ_STR_CAP so eventual
    // ``tcl_obj_release`` reclaims the bytes through ``free_sized``.
    // Without this the rebuilt-dict buffer would leak on every
    // ``dict unset`` (or any caller that releases the returned
    // dict TclObj).
    const out = obj_new_string(@bitCast(buf), @bitCast(off));
    if (out == 0) {
        obj.free_sized(buf, cap);
        return 0;
    }
    obj.write_i32(@as(u32, @bitCast(out)) + obj.OBJ_STR_CAP, @bitCast(cap));
    return out;
}

fn dict_rebuild_with_value(sd_ptr: u32, sd_len: u32, n: i64, target_idx: i64, vp: u32, vl: u32) i32 {
    // Worst-case: each existing byte could double (backslash-escape) plus
    // braces, plus the new value doubled, plus separator spaces.  Round
    // up to a known size class so OBJ_STR_CAP records the real
    // allocation size — without that, ``release_now`` skips the
    // ``free_sized`` call (gated on ``cap > 0``) and the rebuilt
    // buffer leaks on every ``dict set $d existing-key new-val`` (PR
    // #237 second-pass review).
    const max_buf: u32 = sd_len * 2 + vl * 2 + 16;
    const cap = obj.round_up_to_class((max_buf + 7) & ~@as(u32, 7));
    const buf = alloc(cap);
    if (buf == 0) return obj_new_string(0, 0);
    var off: u32 = 0;
    var idx: i64 = 0;
    while (idx < n) : (idx += 1) {
        if (off > 0) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        if (idx == target_idx + 1) {
            // A dict's value slots live at odd indices (1, 3, 5, …)
            // — they are never element 0 — so ``DONT_QUOTE_HASH`` is
            // always the right choice here.
            off = list_elem_quote_nth(buf, off, vp, vl);
        } else {
            const elem = list_element_at(sd_ptr, sd_len, idx);
            if (elem.braced) {
                const d: [*]u8 = @ptrFromInt(buf + off);
                d[0] = '{';
                off += 1;
                memcpy(buf + off, sd_ptr + elem.start, elem.len);
                off += elem.len;
                const d2: [*]u8 = @ptrFromInt(buf + off);
                d2[0] = '}';
                off += 1;
            } else {
                memcpy(buf + off, sd_ptr + elem.start, elem.len);
                off += elem.len;
            }
        }
    }
    const out = obj_new_string(@bitCast(buf), @bitCast(off));
    if (out == 0) {
        obj.free_sized(buf, cap);
        return 0;
    }
    obj.write_i32(@as(u32, @bitCast(out)) + obj.OBJ_STR_CAP, @bitCast(cap));
    return out;
}

fn dict_append_pair(sd_ptr: u32, sd_len: u32, kp: u32, kl: u32, vp: u32, vl: u32) i32 {
    // Round the allocation up to a known size class so the returned
    // dict can take the in-place fast path on the *next* ``dict_set``
    // — without recording ``OBJ_STR_CAP``, the rc==1 fast-path check
    // sees cap=0 and falls back to this rebuild every time.
    const max_buf: u32 = sd_len + kl * 2 + vl * 2 + 16;
    const cap = obj.round_up_to_class((max_buf + 7) & ~@as(u32, 7));
    const buf = alloc(cap);
    if (buf == 0) return obj_new_string(0, 0);
    var off: u32 = 0;
    if (sd_len > 0) {
        memcpy(buf, sd_ptr, sd_len);
        off = sd_len;
        const d: [*]u8 = @ptrFromInt(buf + off);
        d[0] = ' ';
        off += 1;
        // Key / value follow at least one prior element → hash-safe.
        off = list_elem_quote_nth(buf, off, kp, kl);
    } else {
        off = list_elem_quote(buf, off, kp, kl);
    }
    const d: [*]u8 = @ptrFromInt(buf + off);
    d[0] = ' ';
    off += 1;
    off = list_elem_quote_nth(buf, off, vp, vl);
    const new_obj = obj_new_string(@bitCast(buf), @bitCast(off));
    if (new_obj != 0) {
        obj.write_i32(@as(u32, @bitCast(new_obj)) + obj.OBJ_STR_CAP, @bitCast(cap));
    }
    return new_obj;
}

// Exported: dict merge — merge *source* into *target*; for duplicate
// keys the source value wins.  Variadic ``dict merge d1 d2 d3`` is
// implemented at the compiler level by chaining pair-merges.
pub export fn dict_merge_pair(target: i32, source: i32) i32 {
    const ss = obj_ensure_string(source);
    if (ss.len == 0) return target;
    const n = list_count_elements(ss.ptr, ss.len);
    if (n <= 0) return target;
    var current = target;
    var idx: i64 = 0;
    while (idx + 1 < n) : (idx += 2) {
        const k = list_element_at(ss.ptr, ss.len, idx);
        const v = list_element_at(ss.ptr, ss.len, idx + 1);
        current = dict_set_pair(current, ss.ptr + k.start, k.len, ss.ptr + v.start, v.len);
    }
    return current;
}

fn dict_set_pair(d: i32, kp: u32, kl: u32, vp: u32, vl: u32) i32 {
    const sd = obj_ensure_string(d);
    const n = list_count_elements(sd.ptr, sd.len);
    var idx: i64 = 0;
    while (idx + 1 < n) : (idx += 2) {
        const k = list_element_at(sd.ptr, sd.len, idx);
        if (str_cmp(sd.ptr + k.start, k.len, kp, kl) == 0) {
            return dict_rebuild_with_value(sd.ptr, sd.len, n, idx, vp, vl);
        }
    }
    return dict_append_pair(sd.ptr, sd.len, kp, kl, vp, vl);
}

// Exported: dict exists — check if key exists in dict, return 1 or 0.
pub export fn dict_exists(dict: i32, key: i32) i32 {
    const sd = obj_ensure_string(dict);
    const sk = obj_ensure_string(key);
    const n = list_count_elements(sd.ptr, sd.len);
    var idx: i64 = 0;
    while (idx < n - 1) : (idx += 2) {
        const k = list_element_at(sd.ptr, sd.len, idx);
        if (str_cmp(sd.ptr + k.start, k.len, sk.ptr, sk.len) == 0) {
            return obj_new_int(1);
        }
    }
    return obj_new_int(0);
}

// Exported: dict keys — return a list of all keys in the dict.
pub export fn dict_keys(dict: i32) i32 {
    const sd = obj_ensure_string(dict);
    const n = list_count_elements(sd.ptr, sd.len);
    if (n == 0) return obj_new_string(0, 0);
    const buf = alloc(sd.len);
    var off: u32 = 0;
    var idx: i64 = 0;
    while (idx < n) : (idx += 2) {
        if (off > 0) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        const elem = list_element_at(sd.ptr, sd.len, idx);
        if (elem.braced) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = '{';
            off += 1;
            memcpy(buf + off, sd.ptr + elem.start, elem.len);
            off += elem.len;
            const d2: [*]u8 = @ptrFromInt(buf + off);
            d2[0] = '}';
            off += 1;
        } else {
            memcpy(buf + off, sd.ptr + elem.start, elem.len);
            off += elem.len;
        }
    }
    return obj_new_string(@bitCast(buf), @bitCast(off));
}

// Exported: dict values — return a list of all values in the dict.
pub export fn dict_values(dict: i32) i32 {
    const sd = obj_ensure_string(dict);
    const n = list_count_elements(sd.ptr, sd.len);
    if (n == 0) return obj_new_string(0, 0);
    const buf = alloc(sd.len);
    var off: u32 = 0;
    var idx: i64 = 1;
    while (idx < n) : (idx += 2) {
        if (off > 0) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        const elem = list_element_at(sd.ptr, sd.len, idx);
        if (elem.braced) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = '{';
            off += 1;
            memcpy(buf + off, sd.ptr + elem.start, elem.len);
            off += elem.len;
            const d2: [*]u8 = @ptrFromInt(buf + off);
            d2[0] = '}';
            off += 1;
        } else {
            memcpy(buf + off, sd.ptr + elem.start, elem.len);
            off += elem.len;
        }
    }
    return obj_new_string(@bitCast(buf), @bitCast(off));
}

// Exported: dict size — number of key-value pairs.
pub export fn dict_size(dict: i32) i32 {
    const sd = obj_ensure_string(dict);
    const n = list_count_elements(sd.ptr, sd.len);
    return obj_new_int(@divTrunc(n, 2));
}
