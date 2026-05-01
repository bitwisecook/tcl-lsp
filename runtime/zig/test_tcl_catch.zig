// Unit tests for ``interp/tcl_catch.zig`` — the error-flag /
// error-message state machine, the loop-flow flags
// (``break_flag`` / ``continue_flag`` / ``return_flag``), and the
// ``tcl_cmd_error`` / ``error_unknown_command`` / ``var_unset_error``
// formatting helpers.
//
// Wave 5 of the WASM-runtime test suite (#268).
//
// Every error-raising case runs inside :func:`fixture.with_catch` so
// the raise sets ``error_flag`` instead of trapping the WASM binary.
// :func:`fixture.with_interp` is used for tests that need a frame
// stack (``var_unset_error`` stamps ``::errorInfo`` /
// ``::errorCode`` via the global table).
//
// We deliberately don't exercise the out-of-catch trap path here —
// that's an integration concern and the trap formatter writes to
// stderr, which the Zig test runner can't easily inspect.  The
// in-catch path is what every Tcl-level ``catch { … }`` exercises,
// so it's the highest-leverage behaviour to pin down.

const std = @import("std");
const testing = std.testing;

const obj = @import("valtypes/tcl_obj.zig");
const catch_mod = @import("interp/tcl_catch.zig");
const stubs = @import("stubs/tcl_stubs.zig");
const fixture = @import("runtime_test_fixture.zig");

// -- helpers --------------------------------------------------------

fn name_obj(s: []const u8) i32 {
    return obj.obj_new_string_copy(
        @intCast(@intFromPtr(s.ptr)),
        @intCast(s.len),
    );
}

// -- error_flag / error_msg state machine ---------------------------

fn body_clean_run() void {}

test "catch_enter / catch_leave on a clean run leaves error_flag at 0" {
    const result = fixture.with_catch(&body_clean_run);
    try testing.expect(result == null);
    try testing.expectEqual(@as(u32, 0), catch_mod.error_flag);
    try testing.expectEqual(@as(i32, 0), catch_mod.error_msg);
}

fn body_raise() void {
    stubs.raise("boom");
}

test "raise inside a catch sets error_flag and error_msg" {
    fixture.catch_enter();
    body_raise();
    // ``error_flag`` and ``error_msg`` must be live BEFORE the leave —
    // the leave's job is to read + clear them.
    try testing.expectEqual(@as(u32, 1), catch_mod.error_flag);
    try testing.expect(catch_mod.error_msg != 0);

    const had_error = catch_mod.catch_leave();
    try testing.expectEqual(@as(i64, 1), obj.obj_get_int(had_error));

    // ``catch_leave`` clears ``error_flag`` for the surrounding
    // (non-catch) code; ``last_catch_had_error`` remains so
    // ``catch_result`` can still report.
    try testing.expectEqual(@as(u32, 0), catch_mod.error_flag);
    try testing.expectEqual(@as(u32, 1), catch_mod.last_catch_had_error);

    // Reset for subsequent tests.
    catch_mod.last_catch_had_error = 0;
    catch_mod.error_msg = 0;
    catch_mod.catch_ok_result = 0;
}

test "catch_result returns error_msg after a caught error" {
    fixture.catch_enter();
    stubs.raise("oops");
    _ = catch_mod.catch_leave();
    const r = catch_mod.catch_result();
    try testing.expect(r != 0);
    const s = obj.obj_ensure_string(r);
    const src: [*]const u8 = @ptrFromInt(s.ptr);
    try testing.expectEqualStrings("oops", src[0..s.len]);
    catch_mod.last_catch_had_error = 0;
    catch_mod.error_msg = 0;
}

test "catch_result returns ok_result on a clean run" {
    fixture.catch_enter();
    catch_mod.catch_set_ok_result(obj.obj_new_int(99));
    _ = catch_mod.catch_leave();
    const r = catch_mod.catch_result();
    try testing.expectEqual(@as(i64, 99), obj.obj_get_int(r));
    catch_mod.catch_ok_result = 0;
}

test "catch_set_ok_result is ignored once an error has been raised" {
    fixture.catch_enter();
    stubs.raise("first");
    // After the raise, ``error_flag`` is set; subsequent
    // ``catch_set_ok_result`` calls must be no-ops so the body's
    // last-statement value can't overwrite the captured error.
    catch_mod.catch_set_ok_result(obj.obj_new_int(7));
    _ = catch_mod.catch_leave();
    const r = catch_mod.catch_result();
    // Result must still be the error message, not 7.
    try testing.expect(r != 0);
    const s = obj.obj_ensure_string(r);
    const src: [*]const u8 = @ptrFromInt(s.ptr);
    try testing.expectEqualStrings("first", src[0..s.len]);
    catch_mod.last_catch_had_error = 0;
    catch_mod.error_msg = 0;
}

test "catch_has_error mirrors error_flag while inside the catch scope" {
    fixture.catch_enter();
    try testing.expectEqual(@as(i32, 0), catch_mod.catch_has_error());
    stubs.raise("x");
    try testing.expectEqual(@as(i32, 1), catch_mod.catch_has_error());
    _ = catch_mod.catch_leave();
    catch_mod.last_catch_had_error = 0;
    catch_mod.error_msg = 0;
}

// -- nested catches --------------------------------------------------

test "nested catch_enter increments catch_depth then decrements on leave" {
    const depth0 = catch_mod.catch_depth;
    catch_mod.catch_enter();
    try testing.expectEqual(depth0 + 1, catch_mod.catch_depth);
    catch_mod.catch_enter();
    try testing.expectEqual(depth0 + 2, catch_mod.catch_depth);
    _ = catch_mod.catch_leave();
    try testing.expectEqual(depth0 + 1, catch_mod.catch_depth);
    _ = catch_mod.catch_leave();
    try testing.expectEqual(depth0, catch_mod.catch_depth);
}

fn body_inner_raises_outer_does_not() void {
    const inner = fixture.with_catch(&body_raise);
    // Inner catch saw ``boom``.  We're now back in the outer scope.
    _ = inner;
}

test "inner catch absorbs the error; outer scope sees no pending error" {
    const result = fixture.with_catch(&body_inner_raises_outer_does_not);
    try testing.expect(result == null);
    try testing.expectEqual(@as(u32, 0), catch_mod.error_flag);
}

// -- loop-flow flags -------------------------------------------------

test "flow_consume_break clears the flag and reports it once" {
    catch_mod.break_flag = 1;
    try testing.expectEqual(@as(i32, 1), catch_mod.flow_consume_break());
    // Second consume must report 0 — the previous call cleared the
    // flag.  Otherwise a single ``break`` could escape multiple
    // enclosing loops.
    try testing.expectEqual(@as(i32, 0), catch_mod.flow_consume_break());
    try testing.expectEqual(@as(u32, 0), catch_mod.break_flag);
}

test "flow_consume_break returns 0 when no break is pending" {
    catch_mod.break_flag = 0;
    try testing.expectEqual(@as(i32, 0), catch_mod.flow_consume_break());
}

test "flow_consume_continue clears the flag and reports it once" {
    catch_mod.continue_flag = 1;
    try testing.expectEqual(@as(i32, 1), catch_mod.flow_consume_continue());
    try testing.expectEqual(@as(i32, 0), catch_mod.flow_consume_continue());
    try testing.expectEqual(@as(u32, 0), catch_mod.continue_flag);
}

test "flow_consume_continue returns 0 when no continue is pending" {
    catch_mod.continue_flag = 0;
    try testing.expectEqual(@as(i32, 0), catch_mod.flow_consume_continue());
}

// -- error_unknown_command formatting ------------------------------

fn body_unknown_command() void {
    catch_mod.error_unknown_command(name_obj("frobozz"));
}

test "error_unknown_command formats 'unknown command: <name>'" {
    const msg = fixture.with_catch(&body_unknown_command);
    try testing.expect(msg != null);
    try testing.expectEqualStrings("unknown command: frobozz", msg.?);
}

// -- var_unset_error formatting ------------------------------------

fn body_var_unset_then_assert() void {
    catch_mod.var_unset_error(name_obj("missingVar"));
}

fn body_var_unset_inside_interp() void {
    // ``var_unset_error`` calls ``stamp_error_globals`` →
    // ``ns.global_set``, which needs a live root namespace.  The
    // interp fixture sets that up.
    const _msg = fixture.with_catch(&body_var_unset_then_assert);
    // Communicate the captured message back via a module slot so
    // the outer test body can assert on it.
    if (_msg) |m| {
        captured_unset_msg_len = m.len;
        var i: usize = 0;
        while (i < m.len and i < captured_unset_msg.len) : (i += 1) {
            captured_unset_msg[i] = m[i];
        }
    } else {
        captured_unset_msg_len = 0;
    }
}

var captured_unset_msg: [128]u8 = undefined;
var captured_unset_msg_len: usize = 0;

test "var_unset_error formats reference Tcl's exact wording" {
    captured_unset_msg_len = 0;
    fixture.with_interp(&body_var_unset_inside_interp);
    try testing.expectEqualStrings(
        "can't read \"missingVar\": no such variable",
        captured_unset_msg[0..captured_unset_msg_len],
    );
}

// -- consecutive catch / leave hygiene -----------------------------

test "consecutive catches reset state cleanly between iterations" {
    // Three back-to-back error catches; each one must see the prior
    // raise cleared and report only its own message.
    var i: u32 = 0;
    while (i < 3) : (i += 1) {
        const msg = fixture.with_catch(&body_raise);
        try testing.expect(msg != null);
        try testing.expectEqualStrings("boom", msg.?);
    }
}
