// Execution traces — ``trace add execution CMD {enter|leave|
// enterstep|leavestep} CALLBACK``.
//
// A traced command fires its ``enter`` callback before it runs and
// ``leave`` after; ``enterstep`` / ``leavestep`` fire around *every*
// command executed inside its body (recursively).  Callbacks are
// invoked as a script:
//   enter:      CALLBACK <command-string> enter
//   leave:      CALLBACK <command-string> <code> <result> leave
//   enterstep:  CALLBACK <step-command-string> enterstep
//   leavestep:  CALLBACK <step-command-string> <code> <result> leavestep
//
// Storage is a small linear registry keyed by the traced command's
// bucket address (its identity), which the dispatcher already has — no
// per-command name canonicalisation.  ``any_traces`` gates the hot
// dispatch path so untraced code pays a single i32 load.

const obj = @import("../valtypes/tcl_obj.zig");
const alloc = obj.alloc;
const obj_ensure_string = obj.obj_ensure_string;
const obj_new_string = obj.obj_new_string;
const tcl_obj_retain = obj.tcl_obj_retain;
const tcl_obj_release = obj.tcl_obj_release;

pub const OP_ENTER: u32 = 1;
pub const OP_LEAVE: u32 = 2;
pub const OP_ENTERSTEP: u32 = 4;
pub const OP_LEAVESTEP: u32 = 8;

const Entry = struct {
    bucket: u32,
    ops: u32,
    callback: i32,
};

var entries: [128]Entry = undefined;
var count: u32 = 0;

/// Fast-path gate read by the command dispatcher: when zero, no
/// execution traces exist and the dispatcher skips all trace work.
pub var any_traces: u32 = 0;

// Step-trace context stack: each entry is a traced command's bucket
// whose body is currently executing.  While non-empty, the dispatcher
// fires ``enterstep`` / ``leavestep`` for every command it runs.
var step_stack: [256]u32 = undefined;
pub var step_depth: u32 = 0;

// Re-entrancy guard: non-zero while a trace callback is being
// evaluated, so commands run *inside* the callback don't recursively
// fire traces.
pub var suppress: u32 = 0;

pub fn push_step(bucket: u32) void {
    if (step_depth < step_stack.len) {
        step_stack[step_depth] = bucket;
        step_depth += 1;
    }
}

pub fn pop_step() void {
    if (step_depth > 0) step_depth -= 1;
}

/// Fire ``op`` (ENTERSTEP / LEAVESTEP) for every command whose body is
/// currently on the step stack.
pub fn fire_step(op: u32, cmd_str: i32, code: i32, result: i32) void {
    if (step_depth == 0) return;
    // Snapshot the depth: a callback must not see step frames pushed by
    // a deeper command after it.
    var i: u32 = 0;
    const d = step_depth;
    while (i < d) : (i += 1) {
        fire(step_stack[i], op, cmd_str, code, result);
    }
}

fn cb_eq(a: i32, b: i32) bool {
    if (a == b) return true;
    if (a == 0 or b == 0) return false;
    const sa = obj_ensure_string(a);
    const sb = obj_ensure_string(b);
    if (sa.len != sb.len) return false;
    const pa: [*]const u8 = @ptrFromInt(sa.ptr);
    const pb: [*]const u8 = @ptrFromInt(sb.ptr);
    var i: u32 = 0;
    while (i < sa.len) : (i += 1) {
        if (pa[i] != pb[i]) return false;
    }
    return true;
}

/// Add (merge) ``ops`` for ``callback`` on the command at ``bucket``.
pub fn add(bucket: u32, ops: u32, callback: i32) void {
    if (ops == 0 or callback == 0 or bucket == 0) return;
    var i: u32 = 0;
    while (i < count) : (i += 1) {
        if (entries[i].bucket == bucket and cb_eq(entries[i].callback, callback)) {
            entries[i].ops |= ops;
            return;
        }
    }
    if (count >= entries.len) return;
    tcl_obj_retain(callback);
    entries[count] = .{ .bucket = bucket, .ops = ops, .callback = callback };
    count += 1;
    any_traces = count;
}

/// Remove ``ops`` for ``callback`` on ``bucket``; drop the entry when
/// its op set empties.
pub fn remove(bucket: u32, ops: u32, callback: i32) void {
    var i: u32 = 0;
    while (i < count) : (i += 1) {
        if (entries[i].bucket == bucket and cb_eq(entries[i].callback, callback)) {
            entries[i].ops &= ~ops;
            if (entries[i].ops == 0) {
                tcl_obj_release(entries[i].callback);
                count -= 1;
                if (i != count) entries[i] = entries[count];
                any_traces = count;
            }
            return;
        }
    }
}

/// Drop every execution trace on ``bucket`` (command deleted / renamed).
pub fn remove_all_for(bucket: u32) void {
    if (count == 0 or bucket == 0) return;
    var i: u32 = 0;
    while (i < count) {
        if (entries[i].bucket == bucket) {
            tcl_obj_release(entries[i].callback);
            count -= 1;
            if (i != count) entries[i] = entries[count];
            continue;
        }
        i += 1;
    }
    any_traces = count;
}

/// Bitwise-OR of every op registered on ``bucket``.
pub fn ops_for(bucket: u32) u32 {
    if (count == 0 or bucket == 0) return 0;
    var acc: u32 = 0;
    var i: u32 = 0;
    while (i < count) : (i += 1) {
        if (entries[i].bucket == bucket) acc |= entries[i].ops;
    }
    return acc;
}

/// Fire every callback on ``bucket`` whose op set intersects ``op``.
/// ``cmd_str`` is the command invocation string; for leave/leavestep
/// ``code``/``result`` are appended before the op word.  Re-entrancy
/// is bounded by the caller (the dispatcher clears the bucket's active
/// op before firing leave so a recursive call doesn't re-loop).
pub fn fire(bucket: u32, op: u32, cmd_str: i32, code: i32, result: i32) void {
    if (count == 0) return;
    const interp = @import("tcl_interp.zig");
    const op_word: []const u8 = switch (op) {
        OP_ENTER => "enter",
        OP_LEAVE => "leave",
        OP_ENTERSTEP => "enterstep",
        OP_LEAVESTEP => "leavestep",
        else => return,
    };
    const with_result = (op == OP_LEAVE or op == OP_LEAVESTEP);
    var i: u32 = 0;
    while (i < count) : (i += 1) {
        if (entries[i].bucket != bucket) continue;
        if ((entries[i].ops & op) == 0) continue;
        fire_one(entries[i].callback, cmd_str, op_word, with_result, code, result, interp);
    }
}

fn fire_one(
    callback: i32,
    cmd_str: i32,
    op_word: []const u8,
    with_result: bool,
    code: i32,
    result: i32,
    interp: anytype,
) void {
    const quote = @import("../valtypes/tcl_list_quote.zig");
    const cb = obj_ensure_string(callback);
    const cmd = obj_ensure_string(cmd_str);
    var code_ptr: u32 = 0;
    var code_len: u32 = 0;
    var res_ptr: u32 = 0;
    var res_len: u32 = 0;
    if (with_result) {
        const cs = obj_ensure_string(code);
        code_ptr = cs.ptr;
        code_len = cs.len;
        const rs = obj_ensure_string(result);
        res_ptr = rs.ptr;
        res_len = rs.len;
    }
    // Worst-case: cb verbatim + each appended arg as a list element
    // (≤ 2*len + 2) plus a separating space.
    var size: u32 = cb.len + (1 + 2 * cmd.len + 2) + (1 + @as(u32, @intCast(op_word.len)) + 2);
    if (with_result) size += (1 + 2 * code_len + 2) + (1 + 2 * res_len + 2);
    const buf = alloc(size);
    if (buf == 0) return;
    var off: u32 = 0;
    if (cb.len > 0) {
        const cbp: [*]const u8 = @ptrFromInt(cb.ptr);
        var k: u32 = 0;
        while (k < cb.len) : (k += 1) {
            @as([*]u8, @ptrFromInt(buf))[off] = cbp[k];
            off += 1;
        }
    }
    off = append_sp_elem(buf, off, cmd.ptr, cmd.len, quote);
    if (with_result) {
        off = append_sp_elem(buf, off, code_ptr, code_len, quote);
        off = append_sp_elem(buf, off, res_ptr, res_len, quote);
    }
    off = append_sp_elem(buf, off, @intFromPtr(op_word.ptr), @intCast(op_word.len), quote);
    const script = obj.obj_new_string_take(buf, off, size);
    if (script == 0) {
        obj.free_sized(buf, size);
        return;
    }
    // Commands run inside the callback must not recursively fire traces.
    suppress += 1;
    _ = interp.tcl_eval(script);
    if (suppress > 0) suppress -= 1;
    tcl_obj_release(script);
}

fn append_sp_elem(buf: u32, off_in: u32, ptr: u32, len: u32, quote: anytype) u32 {
    var off = off_in;
    @as([*]u8, @ptrFromInt(buf))[off] = ' ';
    off += 1;
    return quote.list_elem_quote(buf, off, ptr, len);
}

/// Tcl list of ``{ops callback}`` pairs for ``trace info execution``.
pub fn info(bucket: u32) i32 {
    const list_mod = @import("../valtypes/tcl_list.zig");
    var result = obj_new_string(0, 0);
    if (bucket == 0) return result;
    var i: u32 = 0;
    while (i < count) : (i += 1) {
        if (entries[i].bucket != bucket) continue;
        const ops_str = ops_to_string(entries[i].ops);
        var pair = list_mod.tcl_list(0, ops_str);
        const pair2 = list_mod.tcl_list(pair, entries[i].callback);
        tcl_obj_release(pair);
        pair = pair2;
        tcl_obj_release(ops_str);
        const nr = list_mod.tcl_list(result, pair);
        tcl_obj_release(result);
        tcl_obj_release(pair);
        result = nr;
    }
    return result;
}

fn ops_to_string(ops: u32) i32 {
    var buf: [48]u8 = undefined;
    var off: u32 = 0;
    const pieces = [_]struct { op: u32, s: []const u8 }{
        .{ .op = OP_ENTER, .s = "enter" },
        .{ .op = OP_LEAVE, .s = "leave" },
        .{ .op = OP_ENTERSTEP, .s = "enterstep" },
        .{ .op = OP_LEAVESTEP, .s = "leavestep" },
    };
    for (pieces) |p| {
        if ((ops & p.op) == 0) continue;
        if (off > 0) {
            buf[off] = ' ';
            off += 1;
        }
        for (p.s) |c| {
            buf[off] = c;
            off += 1;
        }
    }
    return obj.obj_new_string_copy(@intFromPtr(&buf), off);
}
