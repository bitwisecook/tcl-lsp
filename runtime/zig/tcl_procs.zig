// Proc registry — maps proc names to either compiled WASM functions or
// interpreted bodies.
//
// Design:
//   - Open-addressing hash table (same pattern as globals/frames)
//   - Each entry stores: name, param list, body text, param count, func_idx
//   - func_idx > 0 means the proc was AOT-compiled to a WASM function
//     and dispatch should call it directly (fast path)
//   - func_idx == 0 means the proc body needs interpretation
//
// Used by:
//   - tcl_interp.zig: proc definition + dispatch
//   - Compiled WASM: registers compiled procs at _start so the interpreter
//     can call them; also used for info commands introspection

const obj = @import("tcl_obj.zig");
const alloc = obj.alloc;
const memcpy = obj.memcpy;
const read_i32 = obj.read_i32;
const write_i32 = obj.write_i32;
const obj_ensure_string = obj.obj_ensure_string;
const obj_new_int = obj.obj_new_int;

const globals = @import("tcl_globals.zig");
const fnv1a = globals.fnv1a;

// -- Registry layout --
// Bucket: [name_ptr:4 | name_len:4 | hash:4 | params_obj:4 |
//          body_obj:4 | n_params:4 | func_idx:4 | _pad:4] = 32 bytes
const PROC_BUCKET_SIZE: u32 = 32;
const PROC_INITIAL_CAP: u32 = 16;

var proc_buf: u32 = 0;
var proc_cap: u32 = 0;
var proc_count: u32 = 0;

/// Cheap check used by ``eval_command``'s proc-first fast path to
/// skip the lookup machinery entirely when the registry is empty
/// (e.g. a bundle with no procs defined yet).  Inlined ``i32`` load;
/// the branch predictor keeps the "has procs" path free in the
/// common case.
pub fn proc_buf_nonzero() bool {
    return proc_buf != 0;
}

// -- Proc lookup LRU cache --
// Small MRU cache keyed by (hash, len, first_byte).  ``proc_lookup``
// dominates dispatch-heavy bundles — tcltest bundles resolve the
// same handful of procs (``test``, ``set``, ``if``, ``expr``,
// ``lassign``, etc.) hundreds of times per file.  Caching skips the
// hash-table probe (and the ``::name`` + linear-suffix namespace
// fallback for unqualified names) when we've just seen the same
// name.
//
// Correctness:
// * ``(hash, len, first_byte)`` match → fall through to ``proc_find``
//   with the cached bucket's saved key content for a final memcmp
//   (the hash probe already does this, but with a bucket-pointer
//   fast path we avoid the hash probe's ``read_i32`` per field).
//   Collisions are rare; the miss cost is a second ``proc_find``
//   call which is what we'd do without the cache anyway.
// * Any registration / grow invalidates the cache — those paths
//   zero the array.
const LRU_SIZE: u32 = 4;
var lru_hash: [LRU_SIZE]u32 = [_]u32{0} ** LRU_SIZE;
var lru_len: [LRU_SIZE]u32 = [_]u32{0} ** LRU_SIZE;
var lru_first_byte: [LRU_SIZE]u8 = [_]u8{0} ** LRU_SIZE;
var lru_bucket: [LRU_SIZE]u32 = [_]u32{0} ** LRU_SIZE;

/// Clear the LRU cache.  Called from every registration / grow path
/// so the cache never returns a stale pointer after the hash table
/// moves or a proc is redefined.
fn lru_invalidate() void {
    var i: u32 = 0;
    while (i < LRU_SIZE) : (i += 1) {
        lru_hash[i] = 0;
        lru_len[i] = 0;
        lru_first_byte[i] = 0;
        lru_bucket[i] = 0;
    }
}

fn proc_init() void {
    if (proc_buf != 0) return;
    proc_cap = PROC_INITIAL_CAP;
    proc_buf = alloc(proc_cap * PROC_BUCKET_SIZE);
    var i: u32 = 0;
    while (i < proc_cap) : (i += 1) {
        write_i32(proc_buf + i * PROC_BUCKET_SIZE, 0); // empty marker
    }
}

fn proc_find(name_ptr: u32, name_len: u32, hash: u32) ?u32 {
    if (proc_buf == 0) return null;
    const mask = proc_cap - 1;
    var idx = hash & mask;
    var probes: u32 = 0;
    while (probes < proc_cap) : (probes += 1) {
        const base = proc_buf + idx * PROC_BUCKET_SIZE;
        const ep: u32 = @bitCast(read_i32(base));
        if (ep == 0) return null;
        const el: u32 = @bitCast(read_i32(base + 4));
        const eh: u32 = @bitCast(read_i32(base + 8));
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
            if (match) return base;
        }
        idx = (idx + 1) & mask;
    }
    return null;
}

fn proc_grow() void {
    // Every bucket's address changes under the growth — anything
    // in the LRU pointing at ``old_buf`` would become invalid.
    lru_invalidate();
    const old_buf = proc_buf;
    const old_cap = proc_cap;
    proc_cap = old_cap * 2;
    proc_buf = alloc(proc_cap * PROC_BUCKET_SIZE);
    proc_count = 0;
    var i: u32 = 0;
    while (i < proc_cap) : (i += 1) {
        write_i32(proc_buf + i * PROC_BUCKET_SIZE, 0);
    }
    i = 0;
    while (i < old_cap) : (i += 1) {
        const base = old_buf + i * PROC_BUCKET_SIZE;
        const ep: u32 = @bitCast(read_i32(base));
        if (ep != 0) {
            const el: u32 = @bitCast(read_i32(base + 4));
            const eh: u32 = @bitCast(read_i32(base + 8));
            const mask = proc_cap - 1;
            var idx = eh & mask;
            while (true) {
                const nb = proc_buf + idx * PROC_BUCKET_SIZE;
                if (@as(u32, @bitCast(read_i32(nb))) == 0) {
                    // Copy all 32 bytes
                    var k: u32 = 0;
                    while (k < PROC_BUCKET_SIZE) : (k += 4) {
                        write_i32(nb + k, read_i32(base + k));
                    }
                    proc_count += 1;
                    break;
                }
                idx = (idx + 1) & mask;
            }
            _ = el;
        }
    }
}

/// Register an interpreted proc (body is Tcl source, func_idx = 0).
/// params_obj and body_obj are TclObj handles (string representations).
pub export fn proc_register(name: i32, params_obj: i32, body_obj: i32) i32 {
    const sn = obj_ensure_string(name);
    proc_init();
    // Any registration — whether a fresh insert, an update, or a
    // trigger for ``proc_grow`` — could move or redefine a bucket
    // the LRU points at.  Invalidate unconditionally; the cache is
    // rebuilt on the next ``proc_lookup``.
    lru_invalidate();
    const hash = fnv1a(sn.ptr, sn.len);

    // Count parameters from the params list
    const sp = obj_ensure_string(params_obj);
    const n_params = obj.list_count_elements(sp.ptr, sp.len);

    // Check if already registered (update in place)
    if (proc_find(sn.ptr, sn.len, hash)) |base| {
        write_i32(base + 12, params_obj);
        write_i32(base + 16, body_obj);
        write_i32(base + 20, @intCast(n_params));
        write_i32(base + 24, 0); // func_idx = 0 (interpreted)
        return obj_new_int(0);
    }

    // Grow if needed
    if (proc_count * 4 >= proc_cap * 3) {
        proc_grow();
    }

    // Insert new entry
    const mask = proc_cap - 1;
    var idx = hash & mask;
    while (true) {
        const base = proc_buf + idx * PROC_BUCKET_SIZE;
        if (@as(u32, @bitCast(read_i32(base))) == 0) {
            const nbuf = alloc(sn.len);
            memcpy(nbuf, sn.ptr, sn.len);
            write_i32(base, @bitCast(nbuf));
            write_i32(base + 4, @bitCast(sn.len));
            write_i32(base + 8, @bitCast(hash));
            write_i32(base + 12, params_obj);
            write_i32(base + 16, body_obj);
            write_i32(base + 20, @intCast(n_params));
            write_i32(base + 24, 0); // func_idx = 0
            write_i32(base + 28, 0); // padding
            proc_count += 1;
            return obj_new_int(0);
        }
        idx = (idx + 1) & mask;
    }
}

/// Register a compiled proc (AOT). func_idx is the WASM function table index.
/// ``args_tail`` is 1 when the last declared parameter is named ``args``
/// (Tcl's variadic-tail marker) and 0 otherwise — the host-bridge
/// dispatcher consults it to pack excess call arguments into a list
/// TclObj so the compiled function's fixed-arity signature still sees
/// the right number of parameters.
pub export fn proc_register_compiled(
    name: i32,
    n_params: i32,
    func_idx: i32,
    args_tail: i32,
) i32 {
    const sn = obj_ensure_string(name);
    proc_init();
    lru_invalidate();
    const hash = fnv1a(sn.ptr, sn.len);

    // Update if exists
    if (proc_find(sn.ptr, sn.len, hash)) |base| {
        write_i32(base + 12, 0); // no params_obj for compiled
        write_i32(base + 16, 0); // no body_obj for compiled
        write_i32(base + 20, n_params);
        write_i32(base + 24, func_idx);
        write_i32(base + 28, args_tail);
        return obj_new_int(0);
    }

    if (proc_count * 4 >= proc_cap * 3) {
        proc_grow();
    }

    const mask = proc_cap - 1;
    var idx = hash & mask;
    while (true) {
        const base = proc_buf + idx * PROC_BUCKET_SIZE;
        if (@as(u32, @bitCast(read_i32(base))) == 0) {
            const nbuf = alloc(sn.len);
            memcpy(nbuf, sn.ptr, sn.len);
            write_i32(base, @bitCast(nbuf));
            write_i32(base + 4, @bitCast(sn.len));
            write_i32(base + 8, @bitCast(hash));
            write_i32(base + 12, 0); // no params
            write_i32(base + 16, 0); // no body
            write_i32(base + 20, n_params);
            write_i32(base + 24, func_idx);
            write_i32(base + 28, args_tail);
            proc_count += 1;
            return obj_new_int(0);
        }
        idx = (idx + 1) & mask;
    }
}

/// Lookup a proc by name. Returns a struct with all info, or null marker.
/// Return encoding (packed into linear memory scratch):
///   If found: returns pointer to the bucket (caller reads fields)
///   If not found: returns 0
///
/// Resolution order for an unqualified name:
///   1. Exact match (``name``).
///   2. Global namespace (``::name``).
///   3. Linear scan for an entry ending in ``::name`` — a best-
///      effort stand-in for Tcl's namespace-path semantics when
///      a dynamically-registered proc (``proc $varName …``) calls
///      a helper that lives in the surrounding namespace.
///      Non-ambiguous matches win; multiple matches take the
///      first seen (deterministic because buckets are append-only
///      until a grow).
pub export fn proc_lookup(name: i32) i32 {
    const sn = obj_ensure_string(name);
    if (proc_buf == 0) return 0;
    // 1. Exact match.
    const hash = fnv1a(sn.ptr, sn.len);

    // LRU fast path.  Match on (hash, len, first_byte) to cheaply
    // reject most misses before the memcmp; on a tentative hit do
    // a ``proc_find`` against the same bucket to confirm under the
    // rare hash+len+byte collision.  The LRU is entirely elided on
    // a ``proc_buf == 0`` early-return above.
    const first_byte: u8 = if (sn.len > 0) blk: {
        const src: [*]const u8 = @ptrFromInt(sn.ptr);
        break :blk src[0];
    } else 0;
    var slot: u32 = 0;
    while (slot < LRU_SIZE) : (slot += 1) {
        if (lru_bucket[slot] == 0) continue; // empty slot
        if (lru_hash[slot] != hash) continue;
        if (lru_len[slot] != sn.len) continue;
        if (lru_first_byte[slot] != first_byte) continue;
        // Tentative hit — verify the stored bucket still has a
        // matching name.  ``proc_find`` walks the probe chain from
        // the hash bucket; if the cache is stale (invalidated on
        // any register/grow) we'll get a miss and fall through to
        // the slow path.
        if (proc_find(sn.ptr, sn.len, hash)) |base| {
            if (base == lru_bucket[slot]) {
                // Promote to slot 0.
                if (slot != 0) {
                    const h = lru_hash[slot];
                    const l = lru_len[slot];
                    const f = lru_first_byte[slot];
                    const b = lru_bucket[slot];
                    var j: u32 = slot;
                    while (j > 0) : (j -= 1) {
                        lru_hash[j] = lru_hash[j - 1];
                        lru_len[j] = lru_len[j - 1];
                        lru_first_byte[j] = lru_first_byte[j - 1];
                        lru_bucket[j] = lru_bucket[j - 1];
                    }
                    lru_hash[0] = h;
                    lru_len[0] = l;
                    lru_first_byte[0] = f;
                    lru_bucket[0] = b;
                }
                return @bitCast(base);
            }
        }
        // Stale / collision — fall through to the slow path below.
        break;
    }

    if (proc_find(sn.ptr, sn.len, hash)) |base| {
        // Insert at slot 0, shift others down.
        var j: u32 = LRU_SIZE - 1;
        while (j > 0) : (j -= 1) {
            lru_hash[j] = lru_hash[j - 1];
            lru_len[j] = lru_len[j - 1];
            lru_first_byte[j] = lru_first_byte[j - 1];
            lru_bucket[j] = lru_bucket[j - 1];
        }
        lru_hash[0] = hash;
        lru_len[0] = sn.len;
        lru_first_byte[0] = first_byte;
        lru_bucket[0] = base;
        return @bitCast(base);
    }
    // Only fall through to namespace search for UNQUALIFIED names.
    // A ``::-``prefixed name is already explicitly global and a
    // miss is a real miss.
    if (sn.len >= 2) {
        const first_two: [*]const u8 = @ptrFromInt(sn.ptr);
        if (first_two[0] == ':' and first_two[1] == ':') return 0;
    }
    // 2. Try ``::name``.
    {
        const alen: u32 = sn.len + 2;
        const buf = alloc(alen);
        const b: [*]u8 = @ptrFromInt(buf);
        b[0] = ':';
        b[1] = ':';
        const src: [*]const u8 = @ptrFromInt(sn.ptr);
        for (0..sn.len) |i| b[2 + i] = src[i];
        const h2 = fnv1a(buf, alen);
        if (proc_find(buf, alen, h2)) |base| {
            return @bitCast(base);
        }
    }
    // 3. Linear scan for ``*::name`` suffix.
    const sp: [*]const u8 = @ptrFromInt(sn.ptr);
    var idx: u32 = 0;
    while (idx < proc_cap) : (idx += 1) {
        const base = proc_buf + idx * PROC_BUCKET_SIZE;
        const np: u32 = @bitCast(read_i32(base));
        if (np == 0) continue;
        const nl: u32 = @bitCast(read_i32(base + 4));
        if (nl <= sn.len + 2) continue; // need room for "::name"
        const npp: [*]const u8 = @ptrFromInt(np);
        // Match suffix "::<name>"
        const tail_start: u32 = nl - sn.len;
        if (tail_start < 2) continue;
        if (npp[tail_start - 2] != ':' or npp[tail_start - 1] != ':') continue;
        var matches = true;
        for (0..sn.len) |i| {
            if (npp[tail_start + i] != sp[i]) {
                matches = false;
                break;
            }
        }
        if (matches) return @bitCast(base);
    }
    return 0;
}

/// Get the func_idx field from a proc bucket pointer.
pub export fn proc_get_func_idx(bucket: i32) i32 {
    if (bucket == 0) return 0;
    const base: u32 = @bitCast(bucket);
    return read_i32(base + 24);
}

/// Get the n_params field from a proc bucket pointer.
pub export fn proc_get_n_params(bucket: i32) i32 {
    if (bucket == 0) return 0;
    const base: u32 = @bitCast(bucket);
    return read_i32(base + 20);
}

/// Non-zero when the last declared parameter of the proc is ``args`` —
/// the variadic-tail marker Tcl treats specially by binding all extra
/// call arguments as a list.  Used by the host-bridge dispatcher to
/// pack excess arguments into a single list TclObj before handing
/// them to the compiled function's fixed-arity signature.
pub export fn proc_get_args_tail(bucket: i32) i32 {
    if (bucket == 0) return 0;
    const base: u32 = @bitCast(bucket);
    return read_i32(base + 28);
}

/// Get the params_obj field from a proc bucket pointer.
pub export fn proc_get_params(bucket: i32) i32 {
    if (bucket == 0) return 0;
    const base: u32 = @bitCast(bucket);
    return read_i32(base + 12);
}

/// Get the body_obj field from a proc bucket pointer.
pub export fn proc_get_body(bucket: i32) i32 {
    if (bucket == 0) return 0;
    const base: u32 = @bitCast(bucket);
    return read_i32(base + 16);
}

/// Get the stored name pointer for a proc bucket — the
/// fully-qualified name the proc was registered under.  Distinct
/// from ``words[0]`` at the caller (which may be the unqualified
/// form resolved via namespace-path search).  Used by the host-
/// bridge dispatcher so the embedder can look up the compiled
/// WASM export by its real qualified name.
pub export fn proc_get_name_ptr(bucket: i32) i32 {
    if (bucket == 0) return 0;
    const base: u32 = @bitCast(bucket);
    return read_i32(base);
}

pub export fn proc_get_name_len(bucket: i32) i32 {
    if (bucket == 0) return 0;
    const base: u32 = @bitCast(bucket);
    return read_i32(base + 4);
}

/// Check if a proc exists by name. Returns 1 or 0.
pub export fn proc_exists(name: i32) i32 {
    const sn = obj_ensure_string(name);
    if (proc_buf == 0) return obj_new_int(0);
    const hash = fnv1a(sn.ptr, sn.len);
    if (proc_find(sn.ptr, sn.len, hash) != null) {
        return obj_new_int(1);
    }
    return obj_new_int(0);
}
