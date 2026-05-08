// Phase 6 of the var/frame/ns/exception architecture refactor:
// variable traces.
//
// Implements ``trace add variable NAME OPS CMD`` and friends.  When a
// traced variable is read / written / unset, every matching callback
// fires before / after the operation per reference Tcl semantics
// (tclTrace.c::TclCallVarTraces).
//
// Storage: a single hash table keyed by the canonical (fully-qualified)
// variable name.  Each entry holds a singly-linked list of Trace
// records; each record is ``(next, ops, cmd_prefix, active)`` packed
// into 16 bytes of WASM linear memory.  Traces are NOT stored in the
// variable's own bucket — most variables are not traced and paying a
// per-bucket field would inflate the array directory.
//
// Re-entrancy: when a callback fires we set ``active=1`` on that
// record; nested fires for the same record skip if already active.
// This matches reference Tcl's ``TCL_TRACE_ACTIVE`` flag.
//
// Scope: this module supports traces on global scalars, global arrays
// (whole-array traces), array elements (``arr(key)``), and namespace
// variables (their canonical FQ form).  Proc-local variable traces
// use a parallel per-frame storage rooted in ``tcl_frames.zig``'s
// ``frame_trace_heads`` array.  Records there carry the local name
// inline so the same fire / drain helpers work without a separate
// directory; ``frame_pop`` walks the head and fires UNSET callbacks
// before releasing the chain.

const obj = @import("../valtypes/tcl_obj.zig");
const ht = @import("../valtypes/hash_table.zig");
const fnv1a = ht.fnv1a;

const alloc = obj.alloc;
const memcpy = obj.memcpy;
const read_i32 = obj.read_i32;
const write_i32 = obj.write_i32;
const obj_ensure_string = obj.obj_ensure_string;
const obj_new_string = obj.obj_new_string;
const obj_new_string_copy = obj.obj_new_string_copy;
const tcl_obj_retain = obj.tcl_obj_retain;
const tcl_obj_release = obj.tcl_obj_release;

/// Operation bitmask.  Matches the ``TCL_TRACE_*`` constants used by
/// reference Tcl's tclTrace.c.
pub const OP_READ: u32 = 1;
pub const OP_WRITE: u32 = 2;
pub const OP_UNSET: u32 = 4;
pub const OP_ARRAY: u32 = 8;

/// Trace record layout in linear memory:
///   [0..4]  next: u32   (wasm address of the next Trace, 0 = end)
///   [4..8]  ops: u32    (bitmask of OP_READ / OP_WRITE / OP_UNSET / OP_ARRAY)
///   [8..12] cmd_prefix: i32  (TclObj handle, retained while record lives)
///   [12..16] active: u32 (re-entrancy guard; 0 = inactive, 1 = firing)
const TRACE_REC_SIZE: u32 = 16;
const OFF_NEXT: u32 = 0;
const OFF_OPS: u32 = 4;
const OFF_CMD: u32 = 8;
const OFF_ACTIVE: u32 = 12;

// --- Trace registry (canonical-name → head pointer) --------------------

// Bucket: [name_ptr:4 | name_len:4 | hash:4 | head_ptr:4] = 16 bytes.
// ``head_ptr`` is the wasm address of the first Trace record, or 0
// when the entry has no traces (which means the entry is reusable).
const DIR_BUCKET_SIZE: u32 = 16;
const DIR_INITIAL_CAP: u32 = 16;

var dir_buf: u32 = 0;
var dir_cap: u32 = 0;
var dir_count: u32 = 0;

fn dir_init() void {
    if (dir_cap != 0) return;
    const buf = alloc(DIR_INITIAL_CAP * DIR_BUCKET_SIZE);
    if (buf == 0) {
        // OOM — leave the registry uninitialised so subsequent
        // ``dir_find`` calls bail via the ``dir_buf == 0`` guard
        // and ``add`` / ``dir_get_or_create`` callers re-attempt
        // on the next install rather than writing through a null
        // pointer.  Codex/Copilot review.
        return;
    }
    dir_buf = buf;
    dir_cap = DIR_INITIAL_CAP;
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

fn dir_grow() void {
    const old_buf = dir_buf;
    const old_cap = dir_cap;
    const new_cap = dir_cap * 2;
    const new_buf = alloc(new_cap * DIR_BUCKET_SIZE);
    if (new_buf == 0) {
        // OOM — leave the existing buffer in place.  The trace
        // install that triggered this growth will fail (callers
        // detect via the post-grow ``dir_find``); existing traces
        // keep working off the smaller buffer.  Codex/Copilot
        // review.
        return;
    }
    dir_cap = new_cap;
    dir_buf = new_buf;
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
        const hp: u32 = @bitCast(read_i32(bucket + 12));
        dir_insert_known(ep, el, eh, hp);
    }
}

fn dir_insert_known(name_ptr: u32, name_len: u32, hash: u32, head_ptr: u32) void {
    dir_init();
    if (dir_count * 4 >= dir_cap * 3) dir_grow();
    const mask = dir_cap - 1;
    var idx = hash & mask;
    while (true) {
        const bucket = dir_buf + idx * DIR_BUCKET_SIZE;
        const ep: u32 = @bitCast(read_i32(bucket));
        if (ep == 0) {
            write_i32(bucket, @bitCast(name_ptr));
            write_i32(bucket + 4, @bitCast(name_len));
            write_i32(bucket + 8, @bitCast(hash));
            write_i32(bucket + 12, @bitCast(head_ptr));
            dir_count += 1;
            return;
        }
        idx = (idx + 1) & mask;
    }
}

/// Look up ``name_ptr``/``name_len``'s directory bucket, allocating
/// it if missing.  Returns 0 on OOM so the caller can drop the
/// install attempt cleanly (rather than dereferencing a null
/// pointer when ``dir_init`` / ``dir_grow`` / the name copy fails).
fn dir_get_or_create(name_ptr: u32, name_len: u32) u32 {
    const hash = fnv1a(name_ptr, name_len);
    if (dir_find(name_ptr, name_len, hash)) |bucket| return bucket;
    dir_init();
    if (dir_buf == 0) return 0;
    if (dir_count * 4 >= dir_cap * 3) dir_grow();
    // Copy name bytes into a stable allocation (caller's buffer may be
    // a transient subst payload).
    const nbuf = alloc(name_len);
    if (nbuf == 0) return 0;
    memcpy(nbuf, name_ptr, name_len);
    dir_insert_known(nbuf, name_len, hash, 0);
    // dir_insert_known incremented dir_count and possibly triggered a
    // grow; re-find to return the post-insert bucket address.
    return dir_find(nbuf, name_len, hash) orelse 0;
}

// --- Trace record ops --------------------------------------------------

fn trace_new(ops: u32, cmd_prefix: i32) u32 {
    const rec = alloc(TRACE_REC_SIZE);
    if (rec == 0) {
        // OOM — return 0 so :func:`add` can drop the install
        // attempt cleanly.  Without this guard the caller's
        // ``write_i32(rec + ...)`` writes through address 0 and
        // traps in ReleaseSafe.  Codex/Copilot review.
        return 0;
    }
    write_i32(rec + OFF_NEXT, 0);
    write_i32(rec + OFF_OPS, @bitCast(ops));
    write_i32(rec + OFF_CMD, cmd_prefix);
    write_i32(rec + OFF_ACTIVE, 0);
    if (cmd_prefix != 0) tcl_obj_retain(cmd_prefix);
    return rec;
}

fn trace_eq(rec: u32, ops: u32, cmd_prefix: i32) bool {
    const r_ops: u32 = @bitCast(read_i32(rec + OFF_OPS));
    if (r_ops != ops) return false;
    const r_cmd = read_i32(rec + OFF_CMD);
    if (r_cmd == cmd_prefix) return true;
    if (r_cmd == 0 or cmd_prefix == 0) return false;
    const a = obj_ensure_string(r_cmd);
    const b = obj_ensure_string(cmd_prefix);
    if (a.len != b.len) return false;
    const ap: [*]const u8 = @ptrFromInt(a.ptr);
    const bp: [*]const u8 = @ptrFromInt(b.ptr);
    for (0..a.len) |i| if (ap[i] != bp[i]) return false;
    return true;
}

/// Install a trace on the canonical FQ name ``name``.  ``ops`` is the
/// bitmask of OP_*; ``cmd_prefix`` is the TclObj script prefix to
/// invoke.  Caller doesn't need to retain ``cmd_prefix``; this module
/// does its own retain.
pub fn add(name: i32, ops: u32, cmd_prefix: i32) void {
    if (ops == 0 or cmd_prefix == 0) return;
    const sn = obj_ensure_string(name);
    if (sn.ptr == 0 or sn.len == 0) return;
    const bucket = dir_get_or_create(sn.ptr, sn.len);
    if (bucket == 0) return; // OOM in dir_init / dir_grow / name copy
    const rec = trace_new(ops, cmd_prefix);
    if (rec == 0) return; // OOM allocating the Trace record
    const head: u32 = @bitCast(read_i32(bucket + 12));
    write_i32(rec + OFF_NEXT, @bitCast(head));
    write_i32(bucket + 12, @bitCast(rec));
}

/// Remove the first trace matching ``(ops, cmd_prefix)``.  Returns
/// true on hit, false if no matching trace was found.
pub fn remove(name: i32, ops: u32, cmd_prefix: i32) bool {
    const sn = obj_ensure_string(name);
    if (sn.ptr == 0 or sn.len == 0) return false;
    const hash = fnv1a(sn.ptr, sn.len);
    const bucket = dir_find(sn.ptr, sn.len, hash) orelse return false;
    var prev: u32 = 0;
    var cur: u32 = @bitCast(read_i32(bucket + 12));
    while (cur != 0) {
        if (trace_eq(cur, ops, cmd_prefix)) {
            const next: u32 = @bitCast(read_i32(cur + OFF_NEXT));
            if (prev == 0) {
                write_i32(bucket + 12, @bitCast(next));
            } else {
                write_i32(prev + OFF_NEXT, @bitCast(next));
            }
            const cmd = read_i32(cur + OFF_CMD);
            if (cmd != 0) tcl_obj_release(cmd);
            obj.free_sized(cur, TRACE_REC_SIZE);
            return true;
        }
        prev = cur;
        cur = @bitCast(read_i32(cur + OFF_NEXT));
    }
    return false;
}

/// Return a Tcl list of ``{ops cmd}`` pairs for every trace on
/// ``name``, or empty list if none.
pub fn info(name: i32) i32 {
    const list_mod = @import("../valtypes/tcl_list.zig");
    const sn = obj_ensure_string(name);
    if (sn.ptr == 0 or sn.len == 0) return obj_new_string(0, 0);
    const hash = fnv1a(sn.ptr, sn.len);
    const bucket = dir_find(sn.ptr, sn.len, hash) orelse return obj_new_string(0, 0);
    var result = obj_new_string(0, 0);
    var cur: u32 = @bitCast(read_i32(bucket + 12));
    while (cur != 0) : (cur = @bitCast(read_i32(cur + OFF_NEXT))) {
        const r_ops: u32 = @bitCast(read_i32(cur + OFF_OPS));
        const r_cmd = read_i32(cur + OFF_CMD);
        const ops_str = ops_to_string(r_ops);
        // Build {ops cmd} pair as a 2-element list, then append to result.
        var pair = list_mod.tcl_list(0, ops_str);
        const pair2 = list_mod.tcl_list(pair, r_cmd);
        tcl_obj_release(pair);
        pair = pair2;
        tcl_obj_release(ops_str);
        const new_result = list_mod.tcl_list(result, pair);
        tcl_obj_release(result);
        tcl_obj_release(pair);
        result = new_result;
    }
    return result;
}

/// Convert an ops bitmask back to the textual form ``array read
/// write unset`` for ``trace info`` output.  Allocates on the heap so
/// the returned TclObj's bytes outlive this stack frame.
fn ops_to_string(ops: u32) i32 {
    const max_len: u32 = 40;
    const buf = alloc(max_len);
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    if ((ops & OP_ARRAY) != 0) {
        const s = "array";
        for (s) |c| {
            dst[off] = c;
            off += 1;
        }
    }
    if ((ops & OP_READ) != 0) {
        if (off > 0) {
            dst[off] = ' ';
            off += 1;
        }
        const s = "read";
        for (s) |c| {
            dst[off] = c;
            off += 1;
        }
    }
    if ((ops & OP_WRITE) != 0) {
        if (off > 0) {
            dst[off] = ' ';
            off += 1;
        }
        const s = "write";
        for (s) |c| {
            dst[off] = c;
            off += 1;
        }
    }
    if ((ops & OP_UNSET) != 0) {
        if (off > 0) {
            dst[off] = ' ';
            off += 1;
        }
        const s = "unset";
        for (s) |c| {
            dst[off] = c;
            off += 1;
        }
    }
    const result = obj.obj_new_string_take(buf, off, max_len);
    return result;
}

// --- Firing -----------------------------------------------------------

/// Internal: find the head Trace pointer for ``name``, or 0.
fn head_for(name_ptr: u32, name_len: u32) u32 {
    if (dir_buf == 0) return 0;
    const hash = fnv1a(name_ptr, name_len);
    const bucket = dir_find(name_ptr, name_len, hash) orelse return 0;
    return @bitCast(read_i32(bucket + 12));
}

/// Test for any trace matching ``op`` on the given canonical name.
/// O(n) over the trace list; the common case (no traces) is O(1)
/// because ``head == 0``.  Defensive: an empty / null name has no
/// traces by definition.
pub fn has_trace(name_ptr: u32, name_len: u32, op: u32) bool {
    if (name_ptr == 0 or name_len == 0) return false;
    var cur = head_for(name_ptr, name_len);
    while (cur != 0) : (cur = @bitCast(read_i32(cur + OFF_NEXT))) {
        const r_ops: u32 = @bitCast(read_i32(cur + OFF_OPS));
        if ((r_ops & op) != 0) return true;
    }
    return false;
}

/// Fire every callback whose ops bitmask intersects ``op`` on the
/// canonical name ``name_ptr``/``name_len``.  Each callback is
/// invoked as ``CMD name1 name2 op`` per reference Tcl semantics:
/// for a scalar, name1 = the var, name2 = "".  For an array element,
/// name1 = arr name, name2 = element key.  ``op_char`` is the
/// internal single-char op code (``'r'`` / ``'w'`` / ``'u'`` /
/// ``'a'``); :func:`invoke_cb` expands it to the full word
/// (``read`` / ``write`` / ``unset`` / ``array``) reference Tcl
/// passes to the callback.
///
/// Re-entrancy: each Trace record carries an ``active`` flag; when
/// firing we set it, and skip already-active records on nested fires
/// (e.g. the callback's body sets the same variable, triggering a
/// recursive fire).
pub fn fire(
    name1_ptr: u32,
    name1_len: u32,
    name2_ptr: u32,
    name2_len: u32,
    op: u32,
    op_char: u8,
) void {
    // Defensive: a null/empty array name has no traces.  Required
    // because ``@ptrFromInt(0)`` panics in ReleaseSafe even without
    // a deref, and tcltest's setup paths sometimes ``array set`` an
    // empty TclObj while initialising default state.
    if (name1_ptr == 0 or name1_len == 0) return;
    // Use the array name (name1) as the lookup key — whole-array
    // traces store under the array's name.  Element-specific traces
    // store under ``arr(key)``; we probe both.  Tcl arrays allow
    // empty-string keys (``set a() 1`` is valid), and a trace
    // installed against ``a()`` should fire on that operation, so
    // build the ``arr(key)`` form even when ``name2_len == 0`` —
    // skip only when ``name2_ptr`` is genuinely null (i.e., the
    // caller is signalling "no element context", not "element with
    // empty-string key").  Codex/Copilot review.
    fire_at_key(name1_ptr, name1_len, name1_ptr, name1_len, name2_ptr, name2_len, op, op_char);
    // Element-form probe: fire whenever the caller passed a key,
    // which includes the empty-string case (``set arr() 1``).
    // ``name2_ptr == 0 and name2_len == 0`` is the sentinel for
    // "no element context" — array-wide ops (e.g. ``array unset
    // arr``) take that path and don't probe the element form.
    if (name2_ptr != 0 or name2_len > 0) {
        // Build ``arr(key)`` and probe element-level traces.
        const total: u32 = name1_len + 2 + name2_len;
        const buf = alloc(total);
        if (buf == 0) return;
        const dst: [*]u8 = @ptrFromInt(buf);
        const ap: [*]const u8 = @ptrFromInt(name1_ptr);
        for (0..name1_len) |i| dst[i] = ap[i];
        dst[name1_len] = '(';
        if (name2_len > 0) {
            const kp: [*]const u8 = @ptrFromInt(name2_ptr);
            for (0..name2_len) |i| dst[name1_len + 1 + i] = kp[i];
        }
        dst[total - 1] = ')';
        fire_at_key(buf, total, name1_ptr, name1_len, name2_ptr, name2_len, op, op_char);
        obj.free_sized(buf, total);
    }
}

fn fire_at_key(
    key_ptr: u32,
    key_len: u32,
    name1_ptr: u32,
    name1_len: u32,
    name2_ptr: u32,
    name2_len: u32,
    op: u32,
    op_char: u8,
) void {
    if (key_ptr == 0 or key_len == 0) return;
    var cur = head_for(key_ptr, key_len);
    while (cur != 0) : (cur = @bitCast(read_i32(cur + OFF_NEXT))) {
        const r_ops: u32 = @bitCast(read_i32(cur + OFF_OPS));
        if ((r_ops & op) == 0) continue;
        const active: u32 = @bitCast(read_i32(cur + OFF_ACTIVE));
        if (active != 0) continue;
        write_i32(cur + OFF_ACTIVE, 1);
        const cmd = read_i32(cur + OFF_CMD);
        invoke_cb(cmd, name1_ptr, name1_len, name2_ptr, name2_len, op_char);
        write_i32(cur + OFF_ACTIVE, 0);
    }
}

/// Build ``CMD name1 name2 op`` and feed it to the interpreter via
/// :func:`eval_script`.  Variable names and array keys can contain
/// arbitrary bytes (an unbalanced ``}`` in a key is valid Tcl), so
/// each appended element is routed through ``list_elem_quote`` /
/// ``list_elem_quote_nth`` — the canonical quoter — to keep the
/// resulting script syntactically well-formed.  Codex/Copilot
/// review: the previous bare ``{...}`` wrap broke trace firing on
/// keys containing ``{`` / ``}``.
fn invoke_cb(
    cmd_prefix: i32,
    name1_ptr: u32,
    name1_len: u32,
    name2_ptr: u32,
    name2_len: u32,
    op_char: u8,
) void {
    const interp = @import("tcl_interp.zig");
    const cs = obj_ensure_string(cmd_prefix);
    // Reference Tcl 9 invokes variable-trace callbacks with the full
    // op word (``read`` / ``write`` / ``unset`` / ``array``), not the
    // single-character internal op code we carry in ``op_char``.
    // Without this expansion, the test ``trace add variable x read
    // traceScalar; set x`` reports ``op = r`` to the callback and
    // every reference-Tcl-shaped assertion fails (cf. trace.test
    // 1.1–1.14, 2.1–2.5 in the Tcl 9 core slice).
    const op_word: []const u8 = switch (op_char) {
        'r' => "read",
        'w' => "write",
        'u' => "unset",
        'a' => "array",
        else => &[_]u8{op_char},
    };
    // Worst-case expansion per element: ``2 * len + 2`` (full
    // backslash-escape mode), plus one separator byte per gap and
    // the trailing ``op`` word.
    const total: u32 = cs.len + 1 + (name1_len * 2 + 2) + 1 +
        (name2_len * 2 + 2) + 1 + @as(u32, @intCast(op_word.len)) + 8;
    const buf = alloc(total);
    if (buf == 0) return;
    const dst: [*]u8 = @ptrFromInt(buf);
    const cp: [*]const u8 = @ptrFromInt(cs.ptr);
    var off: u32 = 0;
    // The command prefix is already a syntactically-valid script
    // fragment (caller passes ``trace add variable NAME OPS CMD``'s
    // ``CMD`` argument verbatim) — copy it as-is.
    for (0..cs.len) |i| {
        dst[off] = cp[i];
        off += 1;
    }
    dst[off] = ' ';
    off += 1;
    // name1 — quote-as-list-element so binary-safe names round-trip.
    off = obj.list_elem_quote_nth(buf, off, name1_ptr, name1_len);
    dst[off] = ' ';
    off += 1;
    // name2 — same.  When ``name2_len == 0`` (whole-array trace
    // fire) ``list_elem_quote_nth`` emits ``{}`` correctly.
    off = obj.list_elem_quote_nth(buf, off, name2_ptr, name2_len);
    dst[off] = ' ';
    off += 1;
    for (op_word) |b| {
        dst[off] = b;
        off += 1;
    }
    const r = interp.eval_script(buf, off);
    // Intentional leak of ``buf``: the parser's ``subst_flagged``
    // returns non-copying ``obj_new_string`` TclObjs for word tokens
    // that contain no substitution (the common case for the four
    // op-word tokens we just emitted: ``read`` / ``write`` /
    // ``unset`` / ``array``), so any TclObj the callback assigned
    // to a long-lived variable would dangle into freed memory if we
    // ``free_sized``-ed *buf* here.  The leak is bounded — one
    // small allocation per trace fire — and matches the same
    // discipline the rest of the runtime applies to script buffers
    // whose word TclObjs may outlive the call.
    if (r != 0) tcl_obj_release(r);
}

// --- Phase 6 follow-up: per-frame trace lists --------------------------
//
// Frame-local traces live on a per-frame singly-linked list whose head
// pointer is stored in :var:`tcl_frames.frame_trace_heads`.  Records
// here carry the local variable's name inline (so the same list can
// hold traces on several different locals) and are otherwise the same
// shape as the directory-keyed Traces above.
//
// Layout per record:
//   [0..4]   next: u32           (wasm address of the next record)
//   [4..8]   ops: u32            (bitmask of OP_*)
//   [8..12]  cmd_prefix: i32     (TclObj handle, retained)
//   [12..16] active: u32         (re-entrancy guard)
//   [16..20] name_ptr: u32       (heap-allocated name copy, 0 if empty)
//   [20..24] name_len: u32       (byte length of the name copy)
//
// The name buffer is a per-record allocation owned by the trace; the
// chain releases it in :func:`drain_list` along with the cmd prefix.

const FRAME_TRACE_REC_SIZE: u32 = 24;
const F_OFF_NEXT: u32 = 0;
const F_OFF_OPS: u32 = 4;
const F_OFF_CMD: u32 = 8;
const F_OFF_ACTIVE: u32 = 12;
const F_OFF_NAME_PTR: u32 = 16;
const F_OFF_NAME_LEN: u32 = 20;

inline fn name_bytes_match(rec: u32, name_ptr: u32, name_len: u32) bool {
    const rl: u32 = @bitCast(read_i32(rec + F_OFF_NAME_LEN));
    if (rl != name_len) return false;
    const rp: u32 = @bitCast(read_i32(rec + F_OFF_NAME_PTR));
    if (rp == 0 and name_ptr == 0) return true;
    if (rp == 0 or name_ptr == 0) return false;
    const a: [*]const u8 = @ptrFromInt(rp);
    const b: [*]const u8 = @ptrFromInt(name_ptr);
    for (0..name_len) |k| if (a[k] != b[k]) return false;
    return true;
}

fn frame_trace_new(name_ptr: u32, name_len: u32, ops: u32, cmd_prefix: i32) u32 {
    const rec = alloc(FRAME_TRACE_REC_SIZE);
    if (rec == 0) return 0;
    var nbuf: u32 = 0;
    if (name_len > 0 and name_ptr != 0) {
        nbuf = alloc(name_len);
        if (nbuf == 0) {
            obj.free_sized(rec, FRAME_TRACE_REC_SIZE);
            return 0;
        }
        memcpy(nbuf, name_ptr, name_len);
    }
    write_i32(rec + F_OFF_NEXT, 0);
    write_i32(rec + F_OFF_OPS, @bitCast(ops));
    write_i32(rec + F_OFF_CMD, cmd_prefix);
    write_i32(rec + F_OFF_ACTIVE, 0);
    write_i32(rec + F_OFF_NAME_PTR, @bitCast(nbuf));
    write_i32(rec + F_OFF_NAME_LEN, @bitCast(name_len));
    if (cmd_prefix != 0) tcl_obj_retain(cmd_prefix);
    return rec;
}

fn frame_trace_free(rec: u32) void {
    const cmd = read_i32(rec + F_OFF_CMD);
    if (cmd != 0) tcl_obj_release(cmd);
    const nbuf: u32 = @bitCast(read_i32(rec + F_OFF_NAME_PTR));
    const nlen: u32 = @bitCast(read_i32(rec + F_OFF_NAME_LEN));
    if (nbuf != 0 and nlen != 0) obj.free_sized(nbuf, nlen);
    obj.free_sized(rec, FRAME_TRACE_REC_SIZE);
}

/// Install a frame-local trace on ``name`` rooted at ``head_addr``
/// (a pointer to a u32 head slot — typically
/// ``&frame_trace_heads[idx]``).  The list is unsorted; the new
/// record is prepended.  ``cmd_prefix`` is retained.
pub fn add_to_list(head_addr: *u32, name: i32, ops: u32, cmd_prefix: i32) void {
    if (ops == 0 or cmd_prefix == 0) return;
    const sn = obj_ensure_string(name);
    if (sn.len == 0 or sn.ptr == 0) return;
    const rec = frame_trace_new(sn.ptr, sn.len, ops, cmd_prefix);
    if (rec == 0) return;
    write_i32(rec + F_OFF_NEXT, @bitCast(head_addr.*));
    head_addr.* = rec;
}

/// Remove the first matching ``(ops, cmd_prefix)`` trace for ``name``
/// from the list rooted at ``head_addr``.  Returns true on hit.
pub fn remove_from_list(head_addr: *u32, name: i32, ops: u32, cmd_prefix: i32) bool {
    const sn = obj_ensure_string(name);
    if (sn.len == 0 or sn.ptr == 0) return false;
    var prev_addr: *u32 = head_addr;
    var cur: u32 = head_addr.*;
    while (cur != 0) {
        const r_ops: u32 = @bitCast(read_i32(cur + F_OFF_OPS));
        if (r_ops == ops and name_bytes_match(cur, sn.ptr, sn.len)) {
            const r_cmd = read_i32(cur + F_OFF_CMD);
            const cmd_eq = blk: {
                if (r_cmd == cmd_prefix) break :blk true;
                if (r_cmd == 0 or cmd_prefix == 0) break :blk false;
                const a = obj_ensure_string(r_cmd);
                const b = obj_ensure_string(cmd_prefix);
                if (a.len != b.len) break :blk false;
                const ap: [*]const u8 = @ptrFromInt(a.ptr);
                const bp: [*]const u8 = @ptrFromInt(b.ptr);
                for (0..a.len) |i| if (ap[i] != bp[i]) break :blk false;
                break :blk true;
            };
            if (cmd_eq) {
                const next: u32 = @bitCast(read_i32(cur + F_OFF_NEXT));
                prev_addr.* = next;
                frame_trace_free(cur);
                return true;
            }
        }
        prev_addr = @ptrFromInt(cur + F_OFF_NEXT);
        cur = @bitCast(read_i32(cur + F_OFF_NEXT));
    }
    return false;
}

/// Return a Tcl list of ``{ops cmd}`` pairs for every trace on
/// ``name`` in the list rooted at ``head``.  Empty list if no match.
pub fn info_for_list(head: u32, name: i32) i32 {
    const list_mod = @import("../valtypes/tcl_list.zig");
    const sn = obj_ensure_string(name);
    var result = obj_new_string(0, 0);
    if (sn.len == 0 or sn.ptr == 0) return result;
    var cur = head;
    while (cur != 0) : (cur = @bitCast(read_i32(cur + F_OFF_NEXT))) {
        if (!name_bytes_match(cur, sn.ptr, sn.len)) continue;
        const r_ops: u32 = @bitCast(read_i32(cur + F_OFF_OPS));
        const r_cmd = read_i32(cur + F_OFF_CMD);
        const ops_str = ops_to_string(r_ops);
        var pair = list_mod.tcl_list(0, ops_str);
        const pair2 = list_mod.tcl_list(pair, r_cmd);
        tcl_obj_release(pair);
        pair = pair2;
        tcl_obj_release(ops_str);
        const new_result = list_mod.tcl_list(result, pair);
        tcl_obj_release(result);
        tcl_obj_release(pair);
        result = new_result;
    }
    return result;
}

/// Probe for any frame-local trace on ``(name_ptr, name_len)`` whose
/// ops mask intersects ``op``.  O(n) walk over the per-frame chain;
/// the caller's hot path bails out via the ``head == 0`` short-circuit
/// when no traces are installed.
pub fn has_trace_in_list(head: u32, name_ptr: u32, name_len: u32, op: u32) bool {
    if (head == 0 or name_ptr == 0 or name_len == 0) return false;
    var cur = head;
    while (cur != 0) : (cur = @bitCast(read_i32(cur + F_OFF_NEXT))) {
        const r_ops: u32 = @bitCast(read_i32(cur + F_OFF_OPS));
        if ((r_ops & op) == 0) continue;
        if (name_bytes_match(cur, name_ptr, name_len)) return true;
    }
    return false;
}

/// Fire every frame-local callback whose ops mask intersects ``op``
/// for ``(name_ptr, name_len)``.  ``op_char`` is "r"/"w"/"u"/"a".
/// Re-entrancy: each record's ``active`` flag is set across the
/// callback to skip recursive fires from the body itself.
pub fn fire_in_list(
    head: u32,
    name_ptr: u32,
    name_len: u32,
    op: u32,
    op_char: u8,
) void {
    if (head == 0 or name_ptr == 0 or name_len == 0) return;
    var cur = head;
    while (cur != 0) : (cur = @bitCast(read_i32(cur + F_OFF_NEXT))) {
        const r_ops: u32 = @bitCast(read_i32(cur + F_OFF_OPS));
        if ((r_ops & op) == 0) continue;
        if (!name_bytes_match(cur, name_ptr, name_len)) continue;
        const active: u32 = @bitCast(read_i32(cur + F_OFF_ACTIVE));
        if (active != 0) continue;
        write_i32(cur + F_OFF_ACTIVE, 1);
        const cmd = read_i32(cur + F_OFF_CMD);
        // Frame-local traces are scalars only — name1 = the local
        // name, name2 = empty (matching Tcl's ``$VAR_NAME`` pattern
        // for scalar-trace callbacks).
        invoke_cb(cmd, name_ptr, name_len, 0, 0, op_char);
        write_i32(cur + F_OFF_ACTIVE, 0);
    }
}

/// Fire UNSET callbacks for every record in the list rooted at
/// ``head_addr``, then release the chain.  Used by ``frame_pop`` so
/// the caller can drop the frame's trace state in one call.  After
/// return, ``head_addr.*`` is zero.
pub fn drain_list(head_addr: *u32) void {
    var cur = head_addr.*;
    head_addr.* = 0;
    while (cur != 0) {
        const next: u32 = @bitCast(read_i32(cur + F_OFF_NEXT));
        const r_ops: u32 = @bitCast(read_i32(cur + F_OFF_OPS));
        const nptr: u32 = @bitCast(read_i32(cur + F_OFF_NAME_PTR));
        const nlen: u32 = @bitCast(read_i32(cur + F_OFF_NAME_LEN));
        if ((r_ops & OP_UNSET) != 0 and nptr != 0 and nlen != 0) {
            // Re-entrancy guard avoids a recursive fire if the
            // callback re-enters drain_list (e.g. by triggering a
            // nested frame_pop).
            const active: u32 = @bitCast(read_i32(cur + F_OFF_ACTIVE));
            if (active == 0) {
                write_i32(cur + F_OFF_ACTIVE, 1);
                const cmd = read_i32(cur + F_OFF_CMD);
                invoke_cb(cmd, nptr, nlen, 0, 0, 'u');
                write_i32(cur + F_OFF_ACTIVE, 0);
            }
        }
        frame_trace_free(cur);
        cur = next;
    }
}
