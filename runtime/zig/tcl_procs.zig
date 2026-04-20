// Proc registry — maps proc names to either compiled WASM functions or
// interpreted bodies.
//
// Design:
//   - Open-addressing hash table from ``hash_table.zig`` (shared with
//     the global var table and per-frame local table).  Each bucket
//     is 32 bytes: the 12-byte header (name_ptr / name_len / hash)
//     plus a 20-byte value payload:
//
//       [12..15]  params_obj : i32 (TclObj for interpreted procs; 0 for compiled)
//       [16..19]  body_obj   : i32 (TclObj for interpreted procs; 0 for compiled)
//       [20..23]  n_params   : i32
//       [24..27]  func_idx   : i32 (>0 means AOT-compiled to a WASM fn)
//       [28..31]  args_tail  : i32 (1 if last param is "args"; compiled procs only)
//
//   - func_idx > 0 means the proc was AOT-compiled to a WASM function
//     and dispatch should call it directly (fast path).
//   - func_idx == 0 means the proc body needs interpretation.
//
// Used by:
//   - tcl_interp.zig: proc definition + dispatch
//   - Compiled WASM: registers compiled procs at _start so the interpreter
//     can call them; also used for info commands introspection

const obj = @import("tcl_obj.zig");
const read_i32 = obj.read_i32;
const write_i32 = obj.write_i32;
const obj_ensure_string = obj.obj_ensure_string;
const obj_new_int = obj.obj_new_int;

const ht = @import("hash_table.zig");
const fnv1a = ht.fnv1a;

const tcl_ns = @import("tcl_ns.zig");

// -- Bucket layout --
const PROC_BUCKET_SIZE: u32 = 32;
const PROC_INITIAL_CAP: u32 = 16;

// Value-payload field offsets within the bucket (header is 12 bytes).
const OFF_PARAMS_OBJ: u32 = 12;
const OFF_BODY_OBJ: u32 = 16;
const OFF_N_PARAMS: u32 = 20;
const OFF_FUNC_IDX: u32 = 24;
const OFF_ARGS_TAIL: u32 = 28;

const ProcTable = ht.Table(PROC_BUCKET_SIZE);
var proc_table: ProcTable = .{};

/// Cheap check used by ``eval_command``'s proc-first fast path to
/// skip the lookup machinery entirely when the registry is empty
/// (e.g. a bundle with no procs defined yet).
pub fn proc_buf_nonzero() bool {
    return proc_table.nonzero();
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
// * ``(hash, len, first_byte)`` match → fall through to ``proc_table.find``
//   to confirm under the rare hash+len+byte collision.  Collisions
//   are rare; the miss cost is a second find call which is what we'd
//   do without the cache anyway.
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

/// Promote a (hash, len, first_byte, bucket) tuple to MRU slot 0,
/// shifting older entries down by one and dropping the oldest off
/// the end.  Shared by every successful ``proc_lookup`` exit so
/// the next lookup can fast-path through the cache.
fn lru_insert(hash: u32, len: u32, first_byte: u8, base: u32) void {
    var j: u32 = LRU_SIZE - 1;
    while (j > 0) : (j -= 1) {
        lru_hash[j] = lru_hash[j - 1];
        lru_len[j] = lru_len[j - 1];
        lru_first_byte[j] = lru_first_byte[j - 1];
        lru_bucket[j] = lru_bucket[j - 1];
    }
    lru_hash[0] = hash;
    lru_len[0] = len;
    lru_first_byte[0] = first_byte;
    lru_bucket[0] = base;
}

/// Grow with LRU invalidation: bucket addresses move on grow, so the
/// LRU's cached bucket pointers would become dangling without this.
fn maybe_grow() void {
    if (proc_table.needs_grow()) {
        lru_invalidate();
        proc_table.grow();
    }
}

/// P2.1 dual-write: also publish this proc into the current
/// namespace's ``cmd_table`` so the new namespace-tree resolution
/// path (P2.2+) can find it.  The cmd_table value is the flat
/// proc_table bucket address — letting reads pivot to the existing
/// ``proc_get_*`` accessors without needing a real ``Command``
/// struct yet (P2.4 finishes the migration).
///
/// ``full_name`` is the FQN as registered (e.g. ``::tcltest::test``
/// or just ``foo``).  We resolve it to (target_ns, simple_name)
/// — find-or-creating any missing intermediates — then insert into
/// that target's ``cmd_table`` keyed by the simple name.  Names
/// without any ``::`` separator land directly in ``ns_current()``.
fn ns_dual_write(name_ptr: u32, name_len: u32, bucket_base: u32) void {
    const cxt = tcl_ns.ns_current();
    const r = tcl_ns.ns_resolve_qualified_creating(cxt, name_ptr, name_len);
    if (r.target_ns == 0 or r.simple_len == 0) return;
    _ = tcl_ns.ns_cmd_put(r.target_ns, r.simple_ptr, r.simple_len, bucket_base);
}

/// Register an interpreted proc (body is Tcl source, func_idx = 0).
/// params_obj and body_obj are TclObj handles (string representations).
pub export fn proc_register(name: i32, params_obj: i32, body_obj: i32) i32 {
    const sn = obj_ensure_string(name);
    proc_table.init(PROC_INITIAL_CAP);
    // Any registration — whether a fresh insert, an update, or a
    // trigger for a grow — could move or redefine a bucket the LRU
    // points at.  Invalidate unconditionally; the cache is rebuilt
    // on the next ``proc_lookup``.
    lru_invalidate();
    const hash = fnv1a(sn.ptr, sn.len);

    // Count parameters from the params list
    const sp = obj_ensure_string(params_obj);
    const n_params = obj.list_count_elements(sp.ptr, sp.len);

    // Check if already registered (update in place — preserves the
    // existing name allocation so other code holding bucket pointers
    // sees the same name bytes).
    if (proc_table.find(sn.ptr, sn.len, hash)) |base| {
        write_i32(base + OFF_PARAMS_OBJ, params_obj);
        write_i32(base + OFF_BODY_OBJ, body_obj);
        write_i32(base + OFF_N_PARAMS, @intCast(n_params));
        write_i32(base + OFF_FUNC_IDX, 0); // interpreted
        write_i32(base + OFF_ARGS_TAIL, 0);
        ns_dual_write(sn.ptr, sn.len, base);
        return obj_new_int(0);
    }

    maybe_grow();

    const base = proc_table.insert_header(sn.ptr, sn.len, hash);
    write_i32(base + OFF_PARAMS_OBJ, params_obj);
    write_i32(base + OFF_BODY_OBJ, body_obj);
    write_i32(base + OFF_N_PARAMS, @intCast(n_params));
    // func_idx + args_tail are zero from insert_header's payload clear.
    ns_dual_write(sn.ptr, sn.len, base);
    return obj_new_int(0);
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
    proc_table.init(PROC_INITIAL_CAP);
    lru_invalidate();
    const hash = fnv1a(sn.ptr, sn.len);

    if (proc_table.find(sn.ptr, sn.len, hash)) |base| {
        write_i32(base + OFF_PARAMS_OBJ, 0); // no params_obj for compiled
        write_i32(base + OFF_BODY_OBJ, 0); // no body_obj for compiled
        write_i32(base + OFF_N_PARAMS, n_params);
        write_i32(base + OFF_FUNC_IDX, func_idx);
        write_i32(base + OFF_ARGS_TAIL, args_tail);
        ns_dual_write(sn.ptr, sn.len, base);
        return obj_new_int(0);
    }

    maybe_grow();

    const base = proc_table.insert_header(sn.ptr, sn.len, hash);
    write_i32(base + OFF_N_PARAMS, n_params);
    write_i32(base + OFF_FUNC_IDX, func_idx);
    write_i32(base + OFF_ARGS_TAIL, args_tail);
    ns_dual_write(sn.ptr, sn.len, base);
    return obj_new_int(0);
}

/// Lookup a proc by name. Returns a struct with all info, or null marker.
/// Return encoding (packed into linear memory scratch):
///   If found: returns pointer to the bucket (caller reads fields)
///   If not found: returns 0
///
/// Resolution order:
///   1. LRU cache (4-slot MRU on the flat ``proc_table``).
///   2. Namespace-tree walk via ``ns_find_command`` — the new
///      primary path (P2.2).  Handles qualified names through
///      ``TclGetNamespaceForQualName``-style descent and
///      unqualified names through context-ns + root cmd_table
///      probes (P5 will add ``commandPathArray`` between).
///   3. Flat exact match — belt-and-suspenders fallback for any
///      bucket that somehow isn't dual-written (e.g. an early-
///      bootstrap proc registered before ``ns_root()`` was
///      reachable).  Removed in P2.4.
///   4. Global ``::name`` prefix probe — same fallback rationale.
///   5. Linear ``*::name`` suffix scan — the legacy hack.  Removed
///      in P2.3 once we've confirmed the tree walk catches every
///      previously-suffix-resolved call.
pub export fn proc_lookup(name: i32) i32 {
    const sn = obj_ensure_string(name);
    if (proc_table.buf == 0) return 0;
    const hash = fnv1a(sn.ptr, sn.len);

    // LRU fast path.  Match on (hash, len, first_byte) to cheaply
    // reject most misses before the memcmp; on a tentative hit do
    // a ``find`` against the same bucket to confirm under the rare
    // hash+len+byte collision.
    const first_byte: u8 = if (sn.len > 0) blk: {
        const src: [*]const u8 = @ptrFromInt(sn.ptr);
        break :blk src[0];
    } else 0;
    var slot: u32 = 0;
    while (slot < LRU_SIZE) : (slot += 1) {
        if (lru_bucket[slot] == 0) continue;
        if (lru_hash[slot] != hash) continue;
        if (lru_len[slot] != sn.len) continue;
        if (lru_first_byte[slot] != first_byte) continue;
        if (proc_table.find(sn.ptr, sn.len, hash)) |base| {
            if (base == lru_bucket[slot]) {
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
        break;
    }

    // 2. Namespace-tree walk.  Returns the same flat-bucket address
    //    the dual-write inserted, so LRU caching works unchanged.
    {
        const ns_hit = tcl_ns.ns_find_command(tcl_ns.ns_current(), sn.ptr, sn.len);
        if (ns_hit != 0) {
            lru_insert(hash, sn.len, first_byte, ns_hit);
            return @bitCast(ns_hit);
        }
    }

    // 3. Flat-table exact match — fallback for the rare case where
    //    a proc bucket exists but somehow isn't reflected in the
    //    tree (early bootstrap, unusual entry path).  Removed in
    //    P2.4 once dual-write has been audited as exhaustive.
    if (proc_table.find(sn.ptr, sn.len, hash)) |base| {
        lru_insert(hash, sn.len, first_byte, base);
        return @bitCast(base);
    }
    // Only fall through to namespace search for UNQUALIFIED names.
    if (sn.len >= 2) {
        const first_two: [*]const u8 = @ptrFromInt(sn.ptr);
        if (first_two[0] == ':' and first_two[1] == ':') return 0;
    }
    // 2. Try ``::name``.
    {
        const alen: u32 = sn.len + 2;
        const buf = obj.alloc(alen);
        const b: [*]u8 = @ptrFromInt(buf);
        b[0] = ':';
        b[1] = ':';
        const src: [*]const u8 = @ptrFromInt(sn.ptr);
        for (0..sn.len) |i| b[2 + i] = src[i];
        const h2 = fnv1a(buf, alen);
        if (proc_table.find(buf, alen, h2)) |base| {
            return @bitCast(base);
        }
    }
    // 3. Linear scan for ``*::name`` suffix.  Inlined here (rather
    // than using ``proc_table.each``) so the early-out on a match
    // can return immediately.
    const sp: [*]const u8 = @ptrFromInt(sn.ptr);
    var idx: u32 = 0;
    while (idx < proc_table.cap) : (idx += 1) {
        const base = proc_table.buf + idx * PROC_BUCKET_SIZE;
        const np: u32 = @bitCast(read_i32(base));
        if (np == 0) continue;
        const nl: u32 = @bitCast(read_i32(base + 4));
        if (nl <= sn.len + 2) continue; // need room for "::name"
        const npp: [*]const u8 = @ptrFromInt(np);
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
    return read_i32(base + OFF_FUNC_IDX);
}

/// Get the n_params field from a proc bucket pointer.
pub export fn proc_get_n_params(bucket: i32) i32 {
    if (bucket == 0) return 0;
    const base: u32 = @bitCast(bucket);
    return read_i32(base + OFF_N_PARAMS);
}

/// Non-zero when the last declared parameter of the proc is ``args`` —
/// the variadic-tail marker Tcl treats specially by binding all extra
/// call arguments as a list.  Used by the host-bridge dispatcher to
/// pack excess arguments into a single list TclObj before handing
/// them to the compiled function's fixed-arity signature.
pub export fn proc_get_args_tail(bucket: i32) i32 {
    if (bucket == 0) return 0;
    const base: u32 = @bitCast(bucket);
    return read_i32(base + OFF_ARGS_TAIL);
}

/// Get the params_obj field from a proc bucket pointer.
pub export fn proc_get_params(bucket: i32) i32 {
    if (bucket == 0) return 0;
    const base: u32 = @bitCast(bucket);
    return read_i32(base + OFF_PARAMS_OBJ);
}

/// Get the body_obj field from a proc bucket pointer.
pub export fn proc_get_body(bucket: i32) i32 {
    if (bucket == 0) return 0;
    const base: u32 = @bitCast(bucket);
    return read_i32(base + OFF_BODY_OBJ);
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
    if (proc_table.buf == 0) return obj_new_int(0);
    const hash = fnv1a(sn.ptr, sn.len);
    if (proc_table.find(sn.ptr, sn.len, hash) != null) {
        return obj_new_int(1);
    }
    return obj_new_int(0);
}
