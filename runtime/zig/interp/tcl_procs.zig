// Proc registry — maps proc names to either compiled WASM functions or
// interpreted bodies.  Storage now lives entirely inside the namespace
// tree: each ``proc`` keyword allocates a 40-byte ``Command`` struct
// and inserts a pointer to it into the target namespace's ``cmd_table``
// (a ``hash_table.Table(16)`` keyed by simple name).  The flat
// ``proc_table`` that lived here through P2.3 has been retired —
// dual-write was an audit step; the cmd_tables are now the single
// source of truth (P2.4).
//
// The ``Command`` layout grew from 32 → 40 bytes in P4.3 to carry
// the ``import_ref_head`` back-list for ``namespace forget``
// invalidation, and offset 8 was repurposed from ``hash`` (never
// read after construction) to ``flags`` in P4.2.  The ``proc_get_*``
// accessors still read at the historical offsets:
//
//       [ 0..3]  name_ptr        : i32 (heap-copied FQN bytes; ``info procs``)
//       [ 4..7]  name_len        : i32
//       [ 8..11] flags           : u32 (CMD_IMPORTED etc., was ``hash``)
//       [12..15] params_obj      : i32 (TclObj for interpreted procs;
//                                        ``*ImportedCmdData`` for imports;
//                                        0 for compiled procs)
//       [16..19] body_obj        : i32 (TclObj for interpreted procs; 0 otherwise)
//       [20..23] n_params        : i32
//       [24..27] func_idx        : i32 (>0 means AOT-compiled to a WASM fn)
//       [28..31] args_tail       : i32 (1 if last param is "args")
//       [32..35] import_ref_head : u32 (``ImportRef`` list head; P4.3)
//       [36..39] export_name_bucket : u32 (non-zero on compiled procs — points
//                                           at a ``{ptr: u32, len: u32}`` pair
//                                           holding the registration-time
//                                           WASM export name so ``rename``
//                                           can overwrite the Command's
//                                           live name slot without breaking
//                                           host-bridge dispatch; zero on
//                                           interpreted procs, aliases, and
//                                           imports)
//
// See the detailed field-by-field layout block above the ``OFF_*``
// constants further down for the canonical in-module description.
//
// Used by:
//   - tcl_interp.zig: proc definition + dispatch
//   - Compiled WASM: registers compiled procs at _start so the
//     interpreter can call them; also used for ``info commands``
//     introspection

const obj = @import("../valtypes/tcl_obj.zig");
const alloc = obj.alloc;
const memcpy = obj.memcpy;
const read_i32 = obj.read_i32;
const write_i32 = obj.write_i32;
const obj_ensure_string = obj.obj_ensure_string;
const obj_new_int = obj.obj_new_int;

const ht = @import("../valtypes/hash_table.zig");
const fnv1a = ht.fnv1a;

const tcl_ns = @import("tcl_ns.zig");
const parse_cache = @import("../valtypes/parse_cache.zig");

// -- Command struct layout --
//
// Sizes are given as comptime constants so the accessors can keep
// using the historical ``OFF_*`` names without changing call sites.
//
// Layout (44 bytes after P4.4):
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
//     [36..39] export_name_bucket : u32 (non-zero on compiled procs;
//                                         points at an 8-byte
//                                         ``{ptr: u32, len: u32}`` record
//                                         holding the registration-time
//                                         WASM export name.  ``tcl_dispatch``
//                                         reads it first so ``rename`` can
//                                         overwrite the Command's live
//                                         name slot — interpreted procs,
//                                         aliases, and imports leave it
//                                         zero and the dispatcher falls
//                                         back to the live name slot)
//     [40..43] n_required       : i32  (minimum supplied args needed for a
//                                         compiled proc; 0 for interpreted
//                                         procs.  Set by proc_register_compiled
//                                         so dispatch() can raise "wrong # args"
//                                         before calling the compiled WASM fn.)
pub const COMMAND_SIZE: u32 = 44;
pub const OFF_FLAGS: u32 = 8;
pub const OFF_PARAMS_OBJ: u32 = 12;
const OFF_BODY_OBJ: u32 = 16;
const OFF_N_PARAMS: u32 = 20;
pub const OFF_FUNC_IDX: u32 = 24;
const OFF_ARGS_TAIL: u32 = 28;
pub const OFF_IMPORT_REF_HEAD: u32 = 32;
pub const OFF_EXPORT_NAME_BUCKET: u32 = 36;
const OFF_N_REQUIRED: u32 = 40;

/// Set on imported (redirect) commands.  ``params_obj`` holds an
/// ``*ImportedCmdData`` pointing at the source ``*Command`` and
/// back at this redirect Command itself.  The dispatcher follows
/// the chain on every lookup so callers always see the source's
/// payload.
pub const CMD_IMPORTED: u32 = 0x80;

/// Set on ``interp alias`` redirect commands.  ``params_obj`` holds
/// an ``*AliasRec`` (see ``tcl_alias.zig``) that names the target
/// command and carries its frozen argv prefix.  Unlike
/// ``CMD_IMPORTED``, the dispatcher does NOT unwrap aliases at
/// lookup time — the redirect's identity is preserved so queries
/// like ``interp alias {} foo`` can introspect it.  The proc
/// dispatch fast path (``eval_proc_call_bucket``) checks this bit
/// before treating the Command as a plain interpreted proc.
pub const CMD_ALIAS: u32 = 0x100;

/// Set on the ``Command`` registered under the child's simple name
/// in the parent's ``cmd_table`` when ``interp create name`` runs.
/// ``params_obj`` stashes the child ``Interp*``; the dispatcher
/// consults this slot to route ``name eval script`` (and future
/// subcommands like ``name alias ...``) into the child interp.
/// Same ``params_obj``-carries-the-target shape ``CMD_ALIAS`` uses
/// so the ``proc_lookup`` fast path just needs a flag check.
pub const CMD_INTERP_CHILD: u32 = 0x200;

/// Set on a ``Command`` registered by ``coroutine NAME body``.  The
/// ``params_obj`` slot holds a ``*Coro`` (see ``sched/tcl_coro.zig``).
/// The proc-dispatch fast path checks this bit before treating the
/// Command as a plain interpreted proc, and routes through
/// :func:`tcl_coro.resume_one` so subsequent ``[NAME]`` calls
/// resume the coroutine.
pub const CMD_COROUTINE: u32 = 0x400;

// ``tcl_ns.zig`` keeps a shadow copy of the Command layout constants
// above because it can't ``@import`` this module without a circular
// dependency.  Pin the shadow to the canonical values here so any
// drift is a compile error rather than a silent runtime corruption.
comptime {
    const shadow = tcl_ns.tcl_procs_constants;
    if (shadow.COMMAND_SIZE != COMMAND_SIZE) @compileError("tcl_ns.tcl_procs_constants.COMMAND_SIZE out of sync with tcl_procs.COMMAND_SIZE");
    if (shadow.OFF_FLAGS != OFF_FLAGS) @compileError("tcl_ns.tcl_procs_constants.OFF_FLAGS out of sync with tcl_procs.OFF_FLAGS");
    if (shadow.OFF_PARAMS_OBJ != OFF_PARAMS_OBJ) @compileError("tcl_ns.tcl_procs_constants.OFF_PARAMS_OBJ out of sync with tcl_procs.OFF_PARAMS_OBJ");
    if (shadow.OFF_IMPORT_REF_HEAD != OFF_IMPORT_REF_HEAD) @compileError("tcl_ns.tcl_procs_constants.OFF_IMPORT_REF_HEAD out of sync with tcl_procs.OFF_IMPORT_REF_HEAD");
    if (shadow.CMD_IMPORTED != CMD_IMPORTED) @compileError("tcl_ns.tcl_procs_constants.CMD_IMPORTED out of sync with tcl_procs.CMD_IMPORTED");
}

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
///
/// Lifetime fix (Phase 1.3): the caller's TclObjs may BORROW their
/// bytes from a parent script buffer (parser-produced TclObjs use
/// ``obj_new_string`` with cap=0).  When that parent script is
/// later released, the borrowed bytes become stale and any read of
/// the proc body — including the next proc invocation's parser
/// pass — sees garbage.  The visible symptom in tcltest.tcl was
/// ``info complete $script`` returning 0 on a script with the
/// correct length but corrupted bytes (e.g. ``expr {[testCons    pc]}``
/// instead of ``expr {[testConstraint unix] || [testConstraint pc]}``).
///
/// We therefore promote any borrowing input to an owning copy
/// before storing it, so the proc table holds a stable buffer
/// independent of the original script's lifetime.
pub export fn proc_register(name: i32, params_obj: i32, body_obj: i32) i32 {
    const sn = obj_ensure_string(name);
    lru_invalidate();
    const hash = fnv1a(sn.ptr, sn.len);

    // Promote borrowing TclObjs to owning copies, then RETAIN the
    // owning TclObj so its lifetime is anchored by the proc-table
    // entry rather than by the caller's transient hold.
    //
    // Why both:
    //  - ``ensure_owned`` makes sure the BUFFER bytes (cap > 0) are
    //    owned by *some* TclObj that's distinct from the caller's
    //    parser-borrowed one.  Without this, the buffer is shared
    //    with the source script and dies when the source releases.
    //  - The retain anchors the OWNING TclObj's refcount > 1 so
    //    even after the caller drops its reference (release at
    //    drain time), the proc-table slot still holds a valid
    //    pointer to live bytes.  Without this, under the libc-
    //    coherent allocator the owning TclObj's slab gets freed
    //    and recycled — observed as ``unknown command: <body>``
    //    when invoking the proc later, because the slab now
    //    holds some other obj's data.
    const owned_params = ensure_owned(params_obj);
    const owned_body = ensure_owned(body_obj);
    obj.tcl_obj_retain(owned_params);
    obj.tcl_obj_retain(owned_body);
    // (debug removed — see commit history)

    const sp = obj_ensure_string(owned_params);
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
    write_i32(cmd + OFF_PARAMS_OBJ, owned_params);
    write_i32(cmd + OFF_BODY_OBJ, owned_body);
    write_i32(cmd + OFF_N_PARAMS, @intCast(n_params));
    write_i32(cmd + OFF_FUNC_IDX, 0);
    write_i32(cmd + OFF_ARGS_TAIL, 0);
    // P9.3: pre-parse the interpreted body into the parse cache
    // so the first ``eval_script`` call on this body hits the
    // warm path.  The cache is keyed on the 8-byte ``(body_ptr,
    // body_len)`` tuple, so re-registrations that produce a
    // fresh TclObj string (different ``body_ptr``) get a new
    // cache entry automatically even when the content is
    // identical.  ``build_for_body`` no-ops on an already-cached
    // ``(body_ptr, body_len)`` tuple so re-registering the same
    // TclObj doesn't re-parse.
    const body_s = obj_ensure_string(owned_body);
    parse_cache.build_for_body(body_s.ptr, body_s.len);
    return obj_new_int(0);
}

/// Promote a possibly-borrowing TclObj to an owning copy.  When the
/// input already owns its buffer (``OBJ_STR_CAP > 0``) it is returned
/// unchanged.  When it borrows (``cap == 0``) — typical for parser-
/// produced word TclObjs that point into the parent script — we
/// allocate a fresh owned buffer and copy the bytes.  Used by
/// proc_register so the proc table holds bytes independent of the
/// outer script's lifetime; without this, releasing the outer
/// script frees the borrowed-from buffer and the proc body's
/// pointer becomes stale (Phase 1.3 fix).
fn ensure_owned(o: i32) i32 {
    if (o == 0) return 0;
    const addr: u32 = @bitCast(o);
    const cap: u32 = @bitCast(read_i32(addr + obj.OBJ_STR_CAP));
    if (cap > 0) return o;
    const sptr: u32 = @bitCast(read_i32(addr + obj.OBJ_STR_PTR));
    const slen: u32 = @bitCast(read_i32(addr + obj.OBJ_STR_LEN));
    if (slen == 0) return o;
    return obj.obj_new_string_copy(sptr, slen);
}

/// Register a compiled proc (AOT). func_idx is the WASM function
/// table index.  ``args_tail`` is 1 when the last declared parameter
/// is named ``args`` (Tcl's variadic-tail marker) and 0 otherwise.
pub export fn proc_register_compiled(
    name: i32,
    n_params: i32,
    func_idx: i32,
    args_tail: i32,
    n_required: i32,
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
    write_i32(cmd + OFF_N_REQUIRED, n_required);

    // Stash the registration-time WASM export name in the sidecar
    // record at ``OFF_EXPORT_NAME_BUCKET``.  ``tcl_dispatch`` reads
    // it first on every call so a later ``rename`` can overwrite
    // the Command's live name slot without breaking host-bridge
    // lookup.  Only compiled procs need this — interpreted procs
    // leave the slot zero and the dispatcher never hits the
    // compiled-path branch for them.
    //
    // On re-registration of an existing bucket we leave the
    // previously-stamped sidecar in place: the export name never
    // changes for a given WASM module instance, so re-allocating
    // would leak without observable benefit.
    const existing_sidecar: u32 = @bitCast(read_i32(cmd + OFF_EXPORT_NAME_BUCKET));
    if (existing_sidecar == 0) {
        const sidecar = alloc(8);
        const nbuf = alloc(sn.len);
        if (sn.len > 0) memcpy(nbuf, sn.ptr, sn.len);
        write_i32(sidecar, @bitCast(nbuf));
        write_i32(sidecar + 4, @bitCast(sn.len));
        write_i32(cmd + OFF_EXPORT_NAME_BUCKET, @bitCast(sidecar));
    }
    return obj_new_int(0);
}

/// Stash the source-text body string for a compiled proc so
/// ``info body`` returns the original ``proc`` body rather than the
/// empty string the AOT path otherwise leaves behind.  Called by the
/// WASM codegen prologue right after ``proc_register_compiled`` for
/// every proc whose body source is statically known (the common
/// case — the inliner only has to drop body_source for synthetic
/// procs like ``when`` that have no Tcl-source equivalent).
///
/// Lifetime contract: ``body_obj`` is the result of an
/// ``obj_new_string`` call against the compiled module's data segment
/// — the bytes live in immutable WASM memory that outlives every
/// runtime allocation, so we don't need ``ensure_owned``'s
/// promote-to-heap-copy step (which would multiply RAM by O(total
/// proc-body bytes) for large bundles like ``tcltest.test``).  We
/// just retain the TclObj header so refcounting can't reclaim the
/// slab while the proc table still references it.
pub export fn proc_set_body_source(name: i32, body_obj: i32) i32 {
    if (body_obj == 0) return obj_new_int(0);
    const bucket = proc_lookup(name);
    if (bucket == 0) return obj_new_int(0);
    const cmd: u32 = @bitCast(bucket);
    // Redefining the same proc (``proc foo …`` inside a loop, or the
    // tcltest re-define dance) re-enters this hook each time.  Without
    // dropping the prior body's retain, every redefinition leaks one
    // TclObj header and the slab it owns.  Retain the new body first
    // (so a self-set with the same handle stays alive), then release
    // the old slot — the order makes a same-obj rebind a net no-op
    // refcount change rather than a transient drop-to-zero.
    const prev: i32 = read_i32(cmd + OFF_BODY_OBJ);
    obj.tcl_obj_retain(body_obj);
    write_i32(cmd + OFF_BODY_OBJ, body_obj);
    if (prev != 0) obj.tcl_obj_release(prev);
    return obj_new_int(0);
}

/// Return the sidecar export-name pointer stashed at
/// ``OFF_EXPORT_NAME_BUCKET``, or 0 if the Command wasn't registered
/// as a compiled proc.  Read by ``tcl_dispatch.dispatch`` so the
/// host-bridge lookup stays tied to the registration-time WASM
/// export name even after ``rename`` has rewritten the Command's
/// live name slot.
pub fn proc_get_export_name(bucket: u32) struct { ptr: u32, len: u32 } {
    const sidecar: u32 = @bitCast(read_i32(bucket + OFF_EXPORT_NAME_BUCKET));
    if (sidecar == 0) return .{ .ptr = 0, .len = 0 };
    const p: u32 = @bitCast(read_i32(sidecar));
    const l: u32 = @bitCast(read_i32(sidecar + 4));
    return .{ .ptr = p, .len = l };
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
/// Returns 0 for a dead redirect — i.e. one whose
/// ``ImportedCmdData.real_cmd`` was cleared to 0 by
/// ``namespace forget`` (P4.4).  The cmd_table bucket stays
/// populated (we don't tombstone) but the lookup result becomes
/// "command not found", matching the C behaviour.
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
        if (desc == 0) return 0;
        const real: u32 = @bitCast(read_i32(desc));
        if (real == 0) return 0; // P4.4: forgotten redirect
        cur = real;
    }
    return cur;
}

/// Public LRU clear for callers outside this module that need to
/// invalidate the dispatch cache after they shadow / hide an
/// existing command.  ``namespace forget`` (P4.4) calls this so a
/// previously-cached redirect lookup doesn't keep serving the now-
/// dead source.
pub fn lru_invalidate_all() void {
    lru_invalidate();
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

/// Get the n_required field for a compiled proc — minimum number of
/// call arguments that must be supplied before the compiled fn is called.
/// Returns 0 for interpreted procs (they do their own arity check via
/// the params_obj string).
pub export fn proc_get_n_required(bucket: i32) i32 {
    if (bucket == 0) return 0;
    const base: u32 = @bitCast(bucket);
    return read_i32(base + OFF_N_REQUIRED);
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
