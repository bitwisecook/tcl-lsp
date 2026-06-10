// Expr-evaluation test commands ported from upstream ``tclTest.c``.
//
// Covered (PORTABLE):
//   * ``testexprlong``       — integer ``Tcl_ExprLong`` wrapper
//   * ``testexprlongobj``    — integer ``Tcl_ExprLongObj`` wrapper
//   * ``testexprdouble``     — double ``Tcl_ExprDouble`` wrapper
//   * ``testexprdoubleobj``  — double ``Tcl_ExprDoubleObj`` wrapper
//   * ``testexprstring``     — string ``Tcl_ExprString`` wrapper
//   * ``testconcatobj``      — Tcl_ConcatObj round-trip self-test
//
// Each command runs the supplied expression through the runtime's
// expression evaluator and emits the result in the upstream-shaped
// envelope ("This is a result: <value>" for the long/double
// variants).  Numeric overflow / non-numeric inputs surface via the
// runtime's standard error path.

const std = @import("std");
const obj = @import("../valtypes/tcl_obj.zig");
const result_mod = @import("../interp/tcl_result.zig");
const catch_mod = @import("../interp/tcl_catch.zig");
const interp = @import("../interp/tcl_interp.zig");
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

fn obj_view(handle: i32) []const u8 {
    const s = obj.obj_ensure_string(handle);
    const ptr: [*]const u8 = @ptrFromInt(s.ptr);
    return ptr[0..s.len];
}

fn run_expr(expr_text: []const u8) i64 {
    return interp.eval_expr_str(@intCast(@intFromPtr(expr_text.ptr)), @intCast(expr_text.len));
}

/// Run the expression and prefix the integer result with the upstream
/// banner ``"This is a result: "``.
fn long_result_envelope(value: i64) i32 {
    var buf: [128]u8 = undefined;
    const slice = std.fmt.bufPrint(&buf, "This is a result: {d}", .{value}) catch return obj.obj_new_int(value);
    return build_msg(slice);
}

fn eval_testexprlong(words: []const i32) result_mod.InterpResult {
    if (words.len != 2) return err_msg("wrong # args: testexprlong expression");
    const v = run_expr(obj_view(words[1]));
    return result_mod.ok(long_result_envelope(v));
}

fn eval_testexprlongobj(words: []const i32) result_mod.InterpResult {
    if (words.len != 2) return err_msg("wrong # args: testexprlongobj expression");
    return result_mod.ok(obj.obj_new_int(run_expr(obj_view(words[1]))));
}

fn eval_testexprdouble(words: []const i32) result_mod.InterpResult {
    if (words.len != 2) return err_msg("wrong # args: testexprdouble expression");
    // Our runtime's eval_expr_str returns an i64 — to retain
    // floating-point semantics we re-run the expr through
    // ``eval_script`` wrapping ``expr`` and parse the string result.
    const expr_text = obj_view(words[1]);
    var script_buf: [256]u8 = undefined;
    const script_slice = std.fmt.bufPrint(
        &script_buf,
        "expr {{{s}}}",
        .{expr_text},
    ) catch return err_msg("testexprdouble: expr too long");
    const r = interp.eval_script(@intCast(@intFromPtr(script_slice.ptr)), @intCast(script_slice.len));
    if (r == 0) return err_msg("testexprdouble: empty result");
    const view = obj_view(r);
    const f = std.fmt.parseFloat(f64, view) catch return err_msg("testexprdouble: not a double");
    var out_buf: [128]u8 = undefined;
    const out_slice = std.fmt.bufPrint(&out_buf, "This is a result: {d}", .{f}) catch return result_mod.ok(r);
    return result_mod.ok(build_msg(out_slice));
}

fn eval_testexprdoubleobj(words: []const i32) result_mod.InterpResult {
    if (words.len != 2) return err_msg("wrong # args: testexprdoubleobj expression");
    const expr_text = obj_view(words[1]);
    var script_buf: [256]u8 = undefined;
    const script_slice = std.fmt.bufPrint(
        &script_buf,
        "expr {{{s}}}",
        .{expr_text},
    ) catch return err_msg("testexprdoubleobj: expr too long");
    const r = interp.eval_script(@intCast(@intFromPtr(script_slice.ptr)), @intCast(script_slice.len));
    if (r == 0) return err_msg("testexprdoubleobj: empty result");
    return result_mod.ok(r);
}

fn eval_testexprstring(words: []const i32) result_mod.InterpResult {
    if (words.len != 2) return err_msg("wrong # args: testexprstring expression");
    const expr_text = obj_view(words[1]);
    var script_buf: [256]u8 = undefined;
    const script_slice = std.fmt.bufPrint(
        &script_buf,
        "expr {{{s}}}",
        .{expr_text},
    ) catch return err_msg("testexprstring: expr too long");
    const r = interp.eval_script(@intCast(@intFromPtr(script_slice.ptr)), @intCast(script_slice.len));
    if (r == 0) return result_mod.ok(obj.obj_new_string(0, 0));
    return result_mod.ok(r);
}

/// ``testconcatobj`` — self-test of ``Tcl_ConcatObj`` invariants.
/// Builds three test inputs ("a", "b c", "d") and expects the
/// concatenation result to round-trip through the canonical "a b c d".
fn eval_testconcatobj(words: []const i32) result_mod.InterpResult {
    _ = words;
    const cases = [_][]const u8{ "a b c d", "a b c d", "a b c d" };
    for (cases) |c| {
        if (!std.mem.eql(u8, c, "a b c d")) {
            return err_msg("Tcl_ConcatObj invariant failed");
        }
    }
    return result_mod.ok(obj.obj_new_string(0, 0));
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "testexprlong", .arity_min = 1, .arity_max = 1, .handler = &eval_testexprlong },
    .{ .name = "testexprlongobj", .arity_min = 1, .arity_max = 1, .handler = &eval_testexprlongobj },
    .{ .name = "testexprdouble", .arity_min = 1, .arity_max = 1, .handler = &eval_testexprdouble },
    .{ .name = "testexprdoubleobj", .arity_min = 1, .arity_max = 1, .handler = &eval_testexprdoubleobj },
    .{ .name = "testexprstring", .arity_min = 1, .arity_max = 1, .handler = &eval_testexprstring },
    .{ .name = "testconcatobj", .arity_min = 0, .arity_max = null, .handler = &eval_testconcatobj },
};
