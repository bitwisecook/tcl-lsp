// Proc registry — maps proc names to either compiled WASM functions or
// interpreted bodies.  Storage now lives entirely inside the namespace
// tree: each ``proc`` keyword allocates a 32-byte ``Command`` struct
// and inserts a pointer to it into the target namespace's ``cmd_table``
// (a ``hash_table.Table(16)`` keyed by simple name).  The flat
// ``proc_table`` that lived here through P2.3 has been retired —
// dual-write was an audit step; the cmd_tables are now the single
// source of truth (P2.4).
//
// The 32-byte ``Command`` layout is unchanged from the prior bucket
// layout so the ``proc_get_*`` accessors keep reading at the same
// offsets — the "bucket" handle they take is now a ``Command*``
// rather than a flat-table bucket address, but field positions match.
//
//       [ 0..3]  name_ptr   : i32 (heap-copied FQN bytes; ``info procs``)
//       [ 4..7]  name_len   : i32
//       [ 8..11] hash       : i32 (FNV-1a of the FQN, kept for parity)
//       [12..15] params_obj : i32 (TclObj for interpreted procs; 0 for compiled)
//       [16..19] body_obj   : i32 (TclObj for interpreted procs; 0 for compiled)
//       [20..23] n_params   : i32
//       [24..27] func_idx   : i32 (>0 means AOT-compiled to a WASM fn)
//       [28..31] args_tail  : i32 (1 if last param is "args"; compiled procs only)
//
// Used by:
//   - tcl_interp.zig: proc definition + dispatch
//   - Compiled WASM: registers compiled procs at _start so the
//     interpreter can call them; also used for ``info commands``
//     introspection

const obj = @import("tcl_obj.zig");
const alloc = obj.alloc;
const memcpy = obj.memcpy;
const read_i32 = obj.read_i32;
const write_i32 = obj.write_i32;
const obj_ensure_string = obj.obj_ensure_string;
const obj_new_int = obj.obj_new_int;

const ht = @import("hash_table.zig");
const fnv1a = ht.fnv1a;

const tcl_ns = @import("tcl_ns.zig");

// -- Command struct layout --
//
// Sizes are given as comptime constants so the accessors can keep
// using the historical ``OFF_*`` names without changing call sites.
//
// Layout (40 bytes after P4.3):
//
//     [ 0.. 3] name_ptr        : i32  (heap-copied FQN bytes; ``info procs``)
//     [ 4.. 7] name_len        : i32
//     [ 8..11] flags           : u32  (CMD_IMPORTED; was ``hash`` — never
//                                       read after construction, repurposed
//                                       in P4.2 with no observable change)
//     [12..15] params_obj      : i32  (TclObj for interpreted procs;
//                                       ``*ImportedCmdData`` for imports;
//                                       0 for compiled procs)
//     [16..19] body_obj        : i32  (TclObj for interpreted procs;
//                                       0 otherwise)
//     [20..23] n_params        : i32
//     [24..27] func_idx        : i32  (>0 means AOT-compiled to a WASM fn)
//     [28..31] args_tail       : i32  (1 if last param is "args")
//     [32..35] import_ref_head : u32  (head of singly-linked ``ImportRef``
//                                       list — every redirect that
//                                       imports this Command, used by
//                                       ``namespace forget`` to splice
//                                       redirects out cleanly; P4.3)
//     [36..39] reserved        : u32  (zero — kept so the struct is
//                                       8-byte aligned for any future
//                                       u64 field)
pub const COMMAND_SIZE: u32 = 40;
pub const OFF_FLAGS: u32 = 8;
pub const OFF_PARAMS_OBJ: u32 = 12;
const OFF_BODY_OBJ: u32 = 16;
const OFF_N_PARAMS: u32 = 20;
const OFF_FUNC_IDX: u32 = 24;
const OFF_ARGS_TAIL: u32 = 28;
pub const OFF_IMPORT_REF_HEAD: u32 = 32;

/// Set on imported (redirect) commands.  ``params_obj`` holds an
/// ``*ImportedCmdData`` pointing at the source ``*Command`` and
/// back at this redirect Command itself.  The dispatcher follows
/// the chain on every lookup so callers always see the source's
/// payload.
pub const CMD_IMPORTED: u32 = 0x80;

/// Total number of registered procs.  Bumped on every fresh insert
/// (not on update-in-place).  Used by ``proc_buf_nonzero`` as the
/// fast-path "any procs at all?" check on ``eval_command``'s
/// proc-first dispatch path.
var proc_count: u32 = 0;

/// Cheap check used by ``eval_command``'s proc-first fast path to
/// skip the lookup machinery entirely when the registry is empty
/// (e.g. a bundle with no procs defined yet).
pub fn proc_buf_nonzero() bool {
    return proc_count != 0;
}

/// Counterpart for non-``proc_register`` paths that publish a
/// command into a namespace's ``cmd_table`` — currently only
/// ``tcl_ns.ns_import``.  Bumping this lets ``proc_lookup``'s
/// "any procs at all?" early-out fire correctly when the only
/// commands present are imports.
pub fn proc_count_bump() void {
    proc_count += 1;
    lru_invalidate();
}

// -- Proc lookup LRU cache --
//
// Small MRU cache keyed by (current_ns, hash, len, first_byte) —
// ``proc_lookup`` dominates dispatch-heavy bundles so caching skips
// the namespace-tree walk when we've just seen the same name in the
// same context.
//
// The ``current_ns`` slot is essential post-P2.4: the same lookup
// name (``test``) can resolve to different commands in different
// namespaces, so the cache key has to disambiguate by context.  An
// alternative would be invalidating on every ``ns_set`` /
// ``ns_restore`` but that would defeat the cache inside any
// dispatch-heavy ns-switching loop (tcltest's test driver, for
// instance).
//
// Correctness: with all four key parts matching, the chance of a
// genuine miss being served as a false hit is ~1 / (2^32 * 256) per
// slot per query — small enough to skip the second confirmatory
// lookup the pre-P2.4 code did.
const LRU_SIZE: u32 = 4;
var lru_ns: [LRU_SIZE]u32 = [_]u32{0} ** LRU_SIZE;
var lru_hash: [LRU_SIZE]u32 = [_]u32{0} ** LRU_SIZE;
var lru_len: [LRU_SIZE]u32 = [_]u32{0} ** LRU_SIZE;
var lru_first_byte: [LRU_SIZE]u8 = [_]u8{0} ** LRU_SIZE;
var lru_cmd: [LRU_SIZE]u32 = [_]u32{0} ** LRU_SIZE;

/// Clear the LRU cache.  Called from every registration path so the
/// cache never returns a freshly-shadowed Command.
fn lru_invalidate() void {
    var i: u32 = 0;
    while (i < LRU_SIZE) : (i += 1) {
        lru_ns[i] = 0;
        lru_hash[i] = 0;
        lru_len[i] = 0;
        lru_first_byte[i] = 0;
        lru_cmd[i] = 0;
    }
}

/// Promote a (ns, hash, len, first_byte, cmd) tuple to MRU slot 0.
fn lru_insert(ns: u32, hash: u32, len: u32, first_byte: u8, cmd: u32) void {
    var j: u32 = LRU_SIZE - 1;
    while (j > 0) : (j -= 1) {
        lru_ns[j] = lru_ns[j - 1];
        lru_hash[j] = lru_hash[j - 1];
        lru_len[j] = lru_len[j - 1];
        lru_first_byte[j] = lru_first_byte[j - 1];
        lru_cmd[j] = lru_cmd[j - 1];
    }
    lru_ns[0] = ns;
    lru_hash[0] = hash;
    lru_len[0] = len;
    lru_first_byte[0] = first_byte;
    lru_cmd[0] = cmd;
}

/// Allocate a fresh ``Command`` struct, copy ``name`` bytes onto
/// the heap so the struct outlives the source TclObj string slab,
/// and zero the value payload.  ``hash`` is unused now (offset 8
/// holds ``flags`` post-P4.2) — kept in the signature for source
/// continuity but ignored.
fn alloc_command(name_ptr: u32, name_len: u32, hash: u32) u32 {
    _ = hash;
    const addr = alloc(COMMAND_SIZE);
    const slice: [*]u8 = @ptrFromInt(addr);
    @memset(slice[0..COMMAND_SIZE], 0);
    const nbuf = alloc(name_len);
    if (name_len > 0) memcpy(nbuf, name_ptr, name_len);
    write_i32(addr, @bitCast(nbuf));
    write_i32(addr + 4, @bitCast(name_len));
    // flags slot at offset 8 stays zero — set later for imports.
    return addr;
}

/// Resolve the registered FQN to ``(target_ns, simple_name)`` and
/// return the existing ``*Command`` from that ns's ``cmd_table`` if
/// any, plus the resolution result so the insert path can use it
/// without re-walking.  ``existing == 0`` for a fresh registration.
fn resolve_for_register(name_ptr: u32, name_len: u32) struct {
    r: tcl_ns.QualifiedResult,
    existing: u32,
} {
    const cxt = tcl_ns.ns_current();
    const r = tcl_ns.ns_resolve_qualified_creating(cxt, name_ptr, name_len);
    if (r.target_ns == 0 or r.simple_len == 0) return .{ .r = r, .existing = 0 };
    const existing = tcl_ns.ns_cmd_find(r.target_ns, r.simple_ptr, r.simple_len);
    return .{ .r = r, .existing = existing };
}

/// Register an interpreted proc (body is Tcl source, func_idx = 0).
/// params_obj and body_obj are TclObj handles (string representations).
pub export fn proc_register(name: i32, params_obj: i32, body_obj: i32) i32 {
    const sn = obj_ensure_string(name);
    lru_invalidate();
    const hash = fnv1a(sn.ptr, sn.len);

    const sp = obj_ensure_string(params_obj);
    const n_params = obj.list_count_elements(sp.ptr, sp.len);

    const ctx = resolve_for_register(sn.ptr, sn.len);
    if (ctx.r.target_ns == 0 or ctx.r.simple_len == 0) return obj_new_int(0);

    var cmd: u32 = ctx.existing;
    if (cmd == 0) {
        cmd = alloc_command(sn.ptr, sn.len, hash);
        _ = tcl_ns.ns_cmd_put(ctx.r.target_ns, ctx.r.simple_ptr, ctx.r.simple_len, cmd);
        proc_count += 1;
    }

    // Clear any CMD_IMPORTED bit — defining a proc with the same
    // simple name shadows an import in this ns (matches C Tcl).
    write_i32(cmd + OFF_FLAGS, 0);
    write_i32(cmd + OFF_PARAMS_OBJ, params_obj);
    write_i32(cmd + OFF_BODY_OBJ, body_obj);
    write_i32(cmd + OFF_N_PARAMS, @intCast(n_params));
    write_i32(cmd + OFF_FUNC_IDX, 0);
    write_i32(cmd + OFF_ARGS_TAIL, 0);
    return obj_new_int(0);
}

/// Register a compiled proc (AOT). func_idx is the WASM function
/// table index.  ``args_tail`` is 1 when the last declared parameter
/// is named ``args`` (Tcl's variadic-tail marker) and 0 otherwise.
pub export fn proc_register_compiled(
    name: i32,
    n_params: i32,
    func_idx: i32,
    args_tail: i32,
) i32 {
    const sn = obj_ensure_string(name);
    lru_invalidate();
    const hash = fnv1a(sn.ptr, sn.len);

    const ctx = resolve_for_register(sn.ptr, sn.len);
    if (ctx.r.target_ns == 0 or ctx.r.simple_len == 0) return obj_new_int(0);

    var cmd: u32 = ctx.existing;
    if (cmd == 0) {
        cmd = alloc_command(sn.ptr, sn.len, hash);
        _ = tcl_ns.ns_cmd_put(ctx.r.target_ns, ctx.r.simple_ptr, ctx.r.simple_len, cmd);
        proc_count += 1;
    }

    // Clear any CMD_IMPORTED bit (see ``proc_register`` above).
    write_i32(cmd + OFF_FLAGS, 0);
    write_i32(cmd + OFF_PARAMS_OBJ, 0);
    write_i32(cmd + OFF_BODY_OBJ, 0);
    write_i32(cmd + OFF_N_PARAMS, n_params);
    write_i32(cmd + OFF_FUNC_IDX, func_idx);
    write_i32(cmd + OFF_ARGS_TAIL, args_tail);
    return obj_new_int(0);
}

/// Lookup a proc by name.  Returns the ``Command*`` (read by
/// ``proc_get_*`` accessors) or 0 if not found.
///
/// Resolution:
///   1. LRU cache keyed on (current_ns, hash, len, first_byte).
///   2. ``ns_find_command`` — walks context-ns then root.cmd_table
///      for unqualified names; for qualified names uses
///      ``TclGetNamespaceForQualName``-style descent + alt path.
///      P5 splices ``commandPathArray`` between the two unqualified
///      probes.
pub export fn proc_lookup(name: i32) i32 {
    if (proc_count == 0) return 0;
    const sn = obj_ensure_string(name);
    const hash = fnv1a(sn.ptr, sn.len);
    const ns = tcl_ns.ns_current();

    const first_byte: u8 = if (sn.len > 0) blk: {
        const src: [*]const u8 = @ptrFromInt(sn.ptr);
        break :blk src[0];
    } else 0;

    // 1. LRU.
    var slot: u32 = 0;
    while (slot < LRU_SIZE) : (slot += 1) {
        if (lru_cmd[slot] == 0) continue;
        if (lru_ns[slot] != ns) continue;
        if (lru_hash[slot] != hash) continue;
        if (lru_len[slot] != sn.len) continue;
        if (lru_first_byte[slot] != first_byte) continue;
        const cached = lru_cmd[slot];
        if (slot != 0) {
            // Promote to MRU slot 0.
            const n = lru_ns[slot];
            const h = lru_hash[slot];
            const l = lru_len[slot];
            const f = lru_first_byte[slot];
            const c = lru_cmd[slot];
            var j: u32 = slot;
            while (j > 0) : (j -= 1) {
                lru_ns[j] = lru_ns[j - 1];
                lru_hash[j] = lru_hash[j - 1];
                lru_len[j] = lru_len[j - 1];
                lru_first_byte[j] = lru_first_byte[j - 1];
                lru_cmd[j] = lru_cmd[j - 1];
            }
            lru_ns[0] = n;
            lru_hash[0] = h;
            lru_len[0] = l;
            lru_first_byte[0] = f;
            lru_cmd[0] = c;
        }
        return @bitCast(cached);
    }

    // 2. Namespace tree walk.
    const cmd = tcl_ns.ns_find_command(ns, sn.ptr, sn.len);
    if (cmd != 0) {
        const real = unwrap_imports(cmd);
        // Cache the *unwrapped* address so subsequent LRU hits
        // skip the redirect chain too.
        lru_insert(ns, hash, sn.len, first_byte, real);
        return @bitCast(real);
    }
    return 0;
}

/// Follow ``CMD_IMPORTED`` redirects to the source ``Command``.
/// In C Tcl ``CMD_IMPORTED`` Commands store an ``ImportedCmdData``
/// (``{real_cmd, self_cmd}``) in the ``client_data`` slot — we use
/// the same shape, holding the descriptor at ``OFF_PARAMS_OBJ``
/// (which is unused for redirect commands since they have no body).
///
/// Capped at 64 hops as a defence against pathological cycles —
/// the only way to create a cycle is `namespace import` aliasing
/// in a loop, which Tcl itself rejects, but the cap is cheap and
/// makes the fast path bounded-time.
fn unwrap_imports(cmd_in: u32) u32 {
    var cur: u32 = cmd_in;
    var hops: u32 = 0;
    while (hops < 64) : (hops += 1) {
        if (cur == 0) return 0;
        const flags: u32 = @bitCast(read_i32(cur + OFF_FLAGS));
        if ((flags & CMD_IMPORTED) == 0) return cur;
        const desc: u32 = @bitCast(read_i32(cur + OFF_PARAMS_OBJ));
        if (desc == 0) return cur;
        // ImportedCmdData layout: [real_cmd: u32 | self_cmd: u32]
        const real: u32 = @bitCast(read_i32(desc));
        if (real == 0) return cur;
        cur = real;
    }
    return cur;
}

/// Get the func_idx field from a proc Command pointer.
pub export fn proc_get_func_idx(bucket: i32) i32 {
    if (bucket == 0) return 0;
    const base: u32 = @bitCast(bucket);
    return read_i32(base + OFF_FUNC_IDX);
}

/// Get the n_params field from a proc Command pointer.
pub export fn proc_get_n_params(bucket: i32) i32 {
    if (bucket == 0) return 0;
    const base: u32 = @bitCast(bucket);
    return read_i32(base + OFF_N_PARAMS);
}

/// Non-zero when the last declared parameter of the proc is ``args`` —
/// the variadic-tail marker Tcl treats specially by binding all
/// extra call arguments as a list.  Used by the host-bridge
/// dispatcher to pack excess arguments into a single list TclObj
/// before handing them to the compiled function's fixed-arity
/// signature.
pub export fn proc_get_args_tail(bucket: i32) i32 {
    if (bucket == 0) return 0;
    const base: u32 = @bitCast(bucket);
    return read_i32(base + OFF_ARGS_TAIL);
}

/// Get the params_obj field from a proc Command pointer.
pub export fn proc_get_params(bucket: i32) i32 {
    if (bucket == 0) return 0;
    const base: u32 = @bitCast(bucket);
    return read_i32(base + OFF_PARAMS_OBJ);
}

/// Get the body_obj field from a proc Command pointer.
pub export fn proc_get_body(bucket: i32) i32 {
    if (bucket == 0) return 0;
    const base: u32 = @bitCast(bucket);
    return read_i32(base + OFF_BODY_OBJ);
}

/// Get the stored name pointer for a proc Command — the
/// fully-qualified name the proc was registered under.  Used by the
/// host-bridge dispatcher so the embedder can look up the compiled
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

/// Check if a proc exists by name.  Returns a TclObj 1 or 0.
pub export fn proc_exists(name: i32) i32 {
    if (proc_count == 0) return obj_new_int(0);
    const sn = obj_ensure_string(name);
    const cmd = tcl_ns.ns_find_command(tcl_ns.ns_current(), sn.ptr, sn.len);
    return obj_new_int(if (cmd != 0) 1 else 0);
}
