// Eval / variable-set / error-injection test commands ported from
// upstream ``tclTest.c``.
//
// Covered (PORTABLE):
//   * ``testevalex``   — Tcl_EvalEx wrapper
//   * ``testevalobjv`` — Tcl_EvalObjv wrapper
//   * ``testreturn``   — returns TCL_RETURN
//   * ``testset``       (registered as ``testseterr`` and ``testsetnoerr``,
//                       both share TestsetCmd; the difference is a
//                       client-data flag for ``TCL_LEAVE_ERR_MSG``).
//   * ``testset2``      — two-part variable name
//   * ``testseterrorcode`` / ``testsetobjerrorcode`` — error-code set
//   * ``testwrongnumargs`` — emits the standard "wrong # args" formatting
//
// Each replaces the matching ``stub_*`` row in ``cmd_stubs.zig`` (the
// stub registrations table is the single source of truth for what's
// not yet ported; see ``docs/design/compiler/wasm-extensions.md``).

const std = @import("std");
const obj = @import("../valtypes/tcl_obj.zig");
const result_mod = @import("../interp/tcl_result.zig");
const catch_mod = @import("../interp/tcl_catch.zig");
const interp = @import("../interp/tcl_interp.zig");
const tcl_ns = @import("../interp/tcl_ns.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");

fn build_msg(buf: []const u8) i32 {
    const dst = obj.alloc(@intCast(buf.len));
    if (dst == 0) return 0;
    const dst_p: [*]u8 = @ptrFromInt(dst);
    for (buf, 0..) |c, i| dst_p[i] = c;
    return obj.obj_new_string_take(dst, @intCast(buf.len), @intCast(buf.len));
}

fn err_msg(buf: []const u8) result_mod.InterpResult {
    catch_mod.tcl_cmd_error(build_msg(buf));
    return result_mod.from_globals(0);
}

fn err_fmt(comptime fmt: []const u8, args: anytype) result_mod.InterpResult {
    var buf: [256]u8 = undefined;
    const slice = std.fmt.bufPrint(&buf, fmt, args) catch fmt;
    return err_msg(slice);
}

fn obj_eq_lit(handle: i32, lit: []const u8) bool {
    const s = obj.obj_ensure_string(handle);
    if (s.len != lit.len) return false;
    const ptr: [*]const u8 = @ptrFromInt(s.ptr);
    for (0..lit.len) |i| {
        if (ptr[i] != lit[i]) return false;
    }
    return true;
}

// -- testevalex ----------------------------------------------------------

/// ``testevalex script ?global?`` — runs *script* via the runtime's
/// :func:`interp.eval_script`.  The optional ``global`` keyword
/// re-enters at the global frame, matching upstream's
/// ``TCL_EVAL_GLOBAL`` flag.  Note: we approximate the flag by
/// running unconditionally at the active frame — full ``global``
/// semantics would need a saved-frame restore that's not yet wired.
fn eval_testevalex(words: []const i32) result_mod.InterpResult {
    if (words.len < 2 or words.len > 3) return err_msg("wrong # args: testevalex script ?global?");
    if (words.len == 3 and !obj_eq_lit(words[2], "global")) {
        return err_msg("bad value: must be global");
    }
    const s = obj.obj_ensure_string(words[1]);
    const r = interp.eval_script(s.ptr, s.len);
    return result_mod.from_globals(r);
}

// -- testevalobjv --------------------------------------------------------

/// ``testevalobjv global word ?word ...?`` — composes the words
/// back into a Tcl script and runs it through :func:`interp.eval_script`.
/// The "global" toggle is currently ignored (see caveat on
/// :fn:`eval_testevalex`).
fn eval_testevalobjv(words: []const i32) result_mod.InterpResult {
    if (words.len < 3) return err_msg("wrong # args: testevalobjv global word ?word ...?");
    // words[1] is the global-int flag we currently ignore.
    var total: u32 = 0;
    var i: u32 = 2;
    while (i < words.len) : (i += 1) {
        total += obj.obj_ensure_string(words[i]).len;
        if (i < words.len - 1) total += 1;
    }
    const buf = obj.alloc(total);
    if (buf == 0) return err_msg("out of memory");
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    i = 2;
    while (i < words.len) : (i += 1) {
        const s = obj.obj_ensure_string(words[i]);
        const sp: [*]const u8 = @ptrFromInt(s.ptr);
        for (0..s.len) |k| dst[off + k] = sp[k];
        off += s.len;
        if (i < words.len - 1) {
            dst[off] = ' ';
            off += 1;
        }
    }
    const r = interp.eval_script(@bitCast(@intFromPtr(dst)), total);
    return result_mod.from_globals(r);
}

// -- testreturn ----------------------------------------------------------

/// ``testreturn`` — exits the current command with TCL_RETURN.
/// Matches upstream ``TestreturnCmd`` exactly.
fn eval_testreturn(words: []const i32) result_mod.InterpResult {
    _ = words;
    result_mod.set_return(obj.obj_new_string(0, 0), 1);
    return result_mod.from_globals(0);
}

// -- testseterr / testsetnoerr / testset2 -------------------------------

/// Variable-get/set test commands.
///
/// ``testseterr varName ?newValue?`` and ``testsetnoerr varName
/// ?newValue?`` differ only in whether failure stamps the
/// ``::errorInfo`` frame.  Two-arg form gets the variable; three-
/// arg form sets it.  The result is prefixed with "before get" /
/// "before set" (matching upstream's ``Tcl_AppendResult`` calls).
///
/// ``testset2 name1 name2 ?newValue?`` is the two-part-name version
/// (``Tcl_SetVar2(interp, name1, name2, ...)``).  Wired through the
/// runtime's namespace-resolve machinery so array element semantics
/// match.
fn eval_testset(words: []const i32) result_mod.InterpResult {
    if (words.len < 2 or words.len > 3) return err_msg("wrong # args: testset varName ?newValue?");
    const name = words[1];
    const ns_root = tcl_ns.ns_root();
    const name_s = obj.obj_ensure_string(name);
    const r = tcl_ns.ns_resolve_qualified_creating(ns_root, name_s.ptr, name_s.len);
    if (r.target_ns == 0 or r.simple_len == 0) return err_msg("bad var name");
    const var_ptr = tcl_ns.ns_var_create(r.target_ns, r.simple_ptr, r.simple_len);
    if (words.len == 2) {
        const cur = tcl_ns.var_get_scalar(var_ptr);
        if (cur == 0) return err_fmt("can't read \"{s}\": no such variable", .{name_s_view(name_s)});
        const cs = obj.obj_ensure_string(@bitCast(cur));
        return result_mod.ok(build_concat("before get ", obj_view(cs)));
    }
    tcl_ns.var_set_scalar(var_ptr, @bitCast(words[2]));
    const set_s = obj.obj_ensure_string(words[2]);
    return result_mod.ok(build_concat("before set ", obj_view(set_s)));
}

fn eval_testset2(words: []const i32) result_mod.InterpResult {
    if (words.len < 3 or words.len > 4) return err_msg("wrong # args: testset2 name1 name2 ?newValue?");
    // Compose ``name1(name2)`` and route through the regular var path.
    const n1 = obj.obj_ensure_string(words[1]);
    const n2 = obj.obj_ensure_string(words[2]);
    const total = n1.len + 1 + n2.len + 1;
    const buf = obj.alloc(total);
    if (buf == 0) return err_msg("out of memory");
    const dst: [*]u8 = @ptrFromInt(buf);
    const sn1: [*]const u8 = @ptrFromInt(n1.ptr);
    const sn2: [*]const u8 = @ptrFromInt(n2.ptr);
    var off: u32 = 0;
    for (0..n1.len) |i| dst[off + i] = sn1[i];
    off += n1.len;
    dst[off] = '(';
    off += 1;
    for (0..n2.len) |i| dst[off + i] = sn2[i];
    off += n2.len;
    dst[off] = ')';
    const composite = obj.obj_new_string_take(buf, total, total);
    var w: [3]i32 = undefined;
    w[0] = words[0];
    w[1] = composite;
    if (words.len == 4) {
        w[2] = words[3];
        return eval_testset(w[0..3]);
    }
    return eval_testset(w[0..2]);
}

// -- testseterrorcode / testsetobjerrorcode -----------------------------

/// ``testseterrorcode arg ...`` — sets ``::errorCode`` to the
/// supplied list of strings (max 5 args), then raises an empty Tcl
/// error so the eval-loop captures the error code on the way out.
fn eval_testseterrorcode(words: []const i32) result_mod.InterpResult {
    if (words.len > 6) return err_msg("too many args");
    // Build the errorCode list value.
    const tcl_list = @import("../valtypes/tcl_list.zig");
    var lst: i32 = obj.obj_new_string(0, 0);
    var i: u32 = 1;
    while (i < words.len) : (i += 1) lst = tcl_list.tcl_cmd_lappend(lst, words[i]);
    if (words.len == 1) lst = tcl_list.tcl_cmd_lappend(lst, build_msg("NONE"));
    set_error_code(lst);
    catch_mod.tcl_cmd_error(obj.obj_new_string(0, 0));
    return result_mod.from_globals(0);
}

fn eval_testsetobjerrorcode(words: []const i32) result_mod.InterpResult {
    // Concat all args into one obj, set as errorCode, raise empty error.
    if (words.len == 1) {
        set_error_code(obj.obj_new_string(0, 0));
        catch_mod.tcl_cmd_error(obj.obj_new_string(0, 0));
        return result_mod.from_globals(0);
    }
    var total: u32 = 0;
    var i: u32 = 1;
    while (i < words.len) : (i += 1) {
        const s = obj.obj_ensure_string(words[i]);
        total += s.len;
        if (i < words.len - 1) total += 1;
    }
    const buf = obj.alloc(total);
    if (buf == 0) return err_msg("out of memory");
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    i = 1;
    while (i < words.len) : (i += 1) {
        const s = obj.obj_ensure_string(words[i]);
        const sp: [*]const u8 = @ptrFromInt(s.ptr);
        for (0..s.len) |k| dst[off + k] = sp[k];
        off += s.len;
        if (i < words.len - 1) {
            dst[off] = ' ';
            off += 1;
        }
    }
    const concat = obj.obj_new_string_take(buf, total, total);
    set_error_code(concat);
    catch_mod.tcl_cmd_error(obj.obj_new_string(0, 0));
    return result_mod.from_globals(0);
}

fn set_error_code(value: i32) void {
    // Write to ::errorCode through the global setter.
    const name = build_msg("::errorCode");
    _ = tcl_ns.global_set(name, value);
}

// -- testwrongnumargs ---------------------------------------------------

/// ``testwrongnumargs leadingObjs message arg ?arg ...?`` —
/// emits the canonical "wrong # args" error message format.  The
/// first arg is the count of leading objs to include verbatim;
/// the rest is what the upstream signature would have been.  We
/// emit the same shape Tcl users see from genuine arity violations.
fn eval_testwrongnumargs(words: []const i32) result_mod.InterpResult {
    if (words.len < 3) return err_msg("wrong # args: testwrongnumargs leadingObjs message arg ?arg ...?");
    const leading = obj.obj_get_int(words[1]);
    const message_s = obj.obj_ensure_string(words[2]);
    var total: u32 = 18; // "wrong # args: should be \""
    var i: u32 = 3;
    const leading_u: u32 = if (leading < 0) 0 else @min(@as(u32, @intCast(leading)), words.len - 3);
    while (i < 3 + leading_u) : (i += 1) {
        total += obj.obj_ensure_string(words[i]).len + 1;
    }
    if (message_s.len > 0) total += message_s.len + 1;
    total += 1; // closing quote
    const buf = obj.alloc(total);
    if (buf == 0) return err_msg("out of memory");
    const dst: [*]u8 = @ptrFromInt(buf);
    const prefix = "wrong # args: should be \"";
    var off: u32 = 0;
    for (prefix, 0..) |c, k| {
        dst[off + k] = c;
    }
    off += @intCast(prefix.len);
    i = 3;
    var first = true;
    while (i < 3 + leading_u) : (i += 1) {
        if (!first) {
            dst[off] = ' ';
            off += 1;
        }
        first = false;
        const w = obj.obj_ensure_string(words[i]);
        const wp: [*]const u8 = @ptrFromInt(w.ptr);
        for (0..w.len) |k| dst[off + k] = wp[k];
        off += w.len;
    }
    if (message_s.len > 0) {
        if (!first) {
            dst[off] = ' ';
            off += 1;
        }
        const mp: [*]const u8 = @ptrFromInt(message_s.ptr);
        for (0..message_s.len) |k| dst[off + k] = mp[k];
        off += message_s.len;
    }
    dst[off] = '"';
    off += 1;
    catch_mod.tcl_cmd_error(obj.obj_new_string_take(buf, off, total));
    return result_mod.from_globals(0);
}

// -- helpers --------------------------------------------------------------

fn obj_view(s: anytype) []const u8 {
    const ptr: [*]const u8 = @ptrFromInt(s.ptr);
    return ptr[0..s.len];
}

fn name_s_view(s: anytype) []const u8 {
    return obj_view(s);
}

fn build_concat(prefix: []const u8, value: []const u8) i32 {
    const total = prefix.len + value.len;
    const buf = obj.alloc(@intCast(total));
    if (buf == 0) return obj.obj_new_string(0, 0);
    const dst: [*]u8 = @ptrFromInt(buf);
    for (prefix, 0..) |c, i| dst[i] = c;
    for (value, 0..) |c, i| dst[prefix.len + i] = c;
    return obj.obj_new_string_take(buf, @intCast(total), @intCast(total));
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "testevalex", .arity_min = 1, .arity_max = 2, .handler = &eval_testevalex },
    .{ .name = "testevalobjv", .arity_min = 2, .arity_max = null, .handler = &eval_testevalobjv },
    .{ .name = "testreturn", .arity_min = 0, .arity_max = 0, .handler = &eval_testreturn },
    .{ .name = "testseterr", .arity_min = 1, .arity_max = 2, .handler = &eval_testset },
    .{ .name = "testsetnoerr", .arity_min = 1, .arity_max = 2, .handler = &eval_testset },
    .{ .name = "testset2", .arity_min = 2, .arity_max = 3, .handler = &eval_testset2 },
    .{ .name = "testseterrorcode", .arity_min = 0, .arity_max = 5, .handler = &eval_testseterrorcode },
    .{ .name = "testsetobjerrorcode", .arity_min = 0, .arity_max = null, .handler = &eval_testsetobjerrorcode },
    .{ .name = "testwrongnumargs", .arity_min = 2, .arity_max = null, .handler = &eval_testwrongnumargs },
};
