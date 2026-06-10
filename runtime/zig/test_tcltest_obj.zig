// Unit tests for the ``tcltest/cmd_obj.zig`` ports — the int /
// boolean / double object-tier ``test*`` commands.  Drives the
// handlers directly; full end-to-end coverage (via
// ``::eval_command`` dispatch) lives in ``tests/test_wasm_bundle.py``.
//
// Compiled only when the test build sees ``build_options.with_tcltest``
// — the lean runtime build's BUILTINS doesn't include the tcltest
// handlers, so testing them there would be meaningless.  The build
// flag flips on for ``zig build test`` because the test target
// shares the ``build_options`` set with the
// ``tcl_runtime_with_tcltest`` exe.

const std = @import("std");
const testing = std.testing;
const build_options = @import("build_options");

const obj = @import("valtypes/tcl_obj.zig");
const fixture = @import("runtime_test_fixture.zig");

// Skip the whole file when the lean build runs the test target —
// nothing here would compile against a runtime that excluded the
// tcltest module.
const have_tcltest = build_options.with_tcltest;

// Lazy import so the lean test build doesn't pull tcltest paths in
// at compile time.
fn cmds() type {
    return @import("tcltest/cmd_obj.zig");
}
fn slots() type {
    return @import("tcltest/slots.zig");
}

fn intObj(v: i64) i32 {
    return obj.obj_new_int(v);
}

fn stringObj(s: []const u8) i32 {
    return obj.obj_new_string(@bitCast(@intFromPtr(s.ptr)), @bitCast(s.len));
}

test "testintobj set + get round-trip" {
    if (!have_tcltest) return error.SkipZigTest;
    fixture.with_interp(struct {
        fn body() void {
            slots().reset_for_tests();
            const set_words = [_]i32{
                stringObj("testintobj"),
                stringObj("set"),
                intObj(0),
                intObj(42),
            };
            const r = cmds().registrations[0].handler(&set_words);
            testing.expectEqual(@as(i64, 42), obj.obj_get_int(r.value)) catch unreachable;

            const get_words = [_]i32{
                stringObj("testintobj"),
                stringObj("get"),
                intObj(0),
            };
            const g = cmds().registrations[0].handler(&get_words);
            testing.expectEqual(@as(i64, 42), obj.obj_get_int(g.value)) catch unreachable;
        }
    }.body);
}

test "testintobj mult10 multiplies in-place" {
    if (!have_tcltest) return error.SkipZigTest;
    fixture.with_interp(struct {
        fn body() void {
            slots().reset_for_tests();
            const set_words = [_]i32{
                stringObj("testintobj"),
                stringObj("set"),
                intObj(1),
                intObj(7),
            };
            _ = cmds().registrations[0].handler(&set_words);

            const mul_words = [_]i32{
                stringObj("testintobj"),
                stringObj("mult10"),
                intObj(1),
            };
            const r = cmds().registrations[0].handler(&mul_words);
            testing.expectEqual(@as(i64, 70), obj.obj_get_int(r.value)) catch unreachable;
        }
    }.body);
}

test "testintobj get on unset slot raises" {
    if (!have_tcltest) return error.SkipZigTest;
    const msg = fixture.with_catch(struct {
        fn body() void {
            slots().reset_for_tests();
            const get_words = [_]i32{
                stringObj("testintobj"),
                stringObj("get"),
                intObj(5),
            };
            _ = cmds().registrations[0].handler(&get_words);
        }
    }.body);
    if (msg == null) return error.TestExpectedError;
    try testing.expect(std.mem.indexOf(u8, msg.?, "unset") != null);
}

test "testbooleanobj not toggles 0 ↔ 1" {
    if (!have_tcltest) return error.SkipZigTest;
    fixture.with_interp(struct {
        fn body() void {
            slots().reset_for_tests();
            const set_words = [_]i32{
                stringObj("testbooleanobj"),
                stringObj("set"),
                intObj(2),
                intObj(0),
            };
            _ = cmds().registrations[1].handler(&set_words);

            const not_words = [_]i32{
                stringObj("testbooleanobj"),
                stringObj("not"),
                intObj(2),
            };
            const r1 = cmds().registrations[1].handler(&not_words);
            testing.expectEqual(@as(i64, 1), obj.obj_get_int(r1.value)) catch unreachable;
            const r2 = cmds().registrations[1].handler(&not_words);
            testing.expectEqual(@as(i64, 0), obj.obj_get_int(r2.value)) catch unreachable;
        }
    }.body);
}

test "testdoubleobj mult10 promotes through float" {
    if (!have_tcltest) return error.SkipZigTest;
    fixture.with_interp(struct {
        fn body() void {
            slots().reset_for_tests();
            const set_words = [_]i32{
                stringObj("testdoubleobj"),
                stringObj("set"),
                intObj(3),
                obj.obj_new_float(1.5),
            };
            _ = cmds().registrations[2].handler(&set_words);

            const mul_words = [_]i32{
                stringObj("testdoubleobj"),
                stringObj("mult10"),
                intObj(3),
            };
            const r = cmds().registrations[2].handler(&mul_words);
            const got = obj.obj_get_float(r.value);
            testing.expect(@abs(got - 15.0) < 1e-9) catch unreachable;
        }
    }.body);
}

test "testintobj bad slot index raises" {
    if (!have_tcltest) return error.SkipZigTest;
    const msg = fixture.with_catch(struct {
        fn body() void {
            slots().reset_for_tests();
            const set_words = [_]i32{
                stringObj("testintobj"),
                stringObj("set"),
                intObj(@intCast(slots().NUM_SLOTS)),
                intObj(1),
            };
            _ = cmds().registrations[0].handler(&set_words);
        }
    }.body);
    if (msg == null) return error.TestExpectedError;
    try testing.expect(std.mem.indexOf(u8, msg.?, "bad slot index") != null);
}
