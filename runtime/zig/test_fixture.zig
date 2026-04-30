// Smoke tests for ``runtime_test_fixture.zig`` — the catch-context
// and interp-state harness from issue #266.
//
// The fixture is the foundation for waves 4 & 5 of the WASM
// runtime test suite (#267, #268), so a regression here would
// silently invalidate every error-path / interp-coupled test
// downstream.  These cases pin the public contract:
//
//   * ``with_catch`` returns ``null`` on success.
//   * ``with_catch`` captures the exact message raised by
//     ``stubs.raise`` and ``stubs.unsupported``.
//   * ``with_interp`` produces a frame that ``local_set`` /
//     ``local_get`` and ``var_resolve`` can reach.
//   * Nested ``with_interp`` calls don't leak frame depth.

const std = @import("std");
const testing = std.testing;

const obj = @import("valtypes/tcl_obj.zig");
const stubs = @import("stubs/tcl_stubs.zig");
const frames = @import("interp/tcl_frames.zig");
const fixture = @import("runtime_test_fixture.zig");

// -- Catch fixture ---------------------------------------------------

fn body_noop() void {}

test "with_catch returns null when body runs cleanly" {
    const result = fixture.with_catch(&body_noop);
    try testing.expect(result == null);
}

fn body_raise_oops() void {
    stubs.raise("oops");
}

test "with_catch captures stubs.raise message" {
    const result = fixture.with_catch(&body_raise_oops);
    try testing.expect(result != null);
    try testing.expectEqualStrings("oops", result.?);
}

fn body_unsupported_format() void {
    stubs.unsupported("format");
}

test "with_catch captures unsupported-command message" {
    const result = fixture.with_catch(&body_unsupported_format);
    try testing.expect(result != null);
    try testing.expectEqualStrings("unsupported command: format", result.?);
}

fn body_unsupported_clock_scan() void {
    stubs.unsupported_sub("clock", "scan");
}

test "with_catch captures unsupported-subcommand message" {
    const result = fixture.with_catch(&body_unsupported_clock_scan);
    try testing.expect(result != null);
    try testing.expectEqualStrings(
        "unsupported command: clock scan",
        result.?,
    );
}

test "consecutive with_catch calls don't leak error state" {
    // The first run sets error_flag + error_msg; the fixture must
    // clear them on leave so the second run starts clean.
    const first = fixture.with_catch(&body_raise_oops);
    try testing.expectEqualStrings("oops", first.?);

    const second = fixture.with_catch(&body_noop);
    try testing.expect(second == null);
}

// -- Interp fixture --------------------------------------------------

// Module-level slots the closure-style bodies write into.  Plain
// fn pointers can't capture locals, so we communicate observed
// state back to the test through these.
var observed_value: i32 = 0;
var observed_depth: u32 = 0;

fn body_set_get_round_trip() void {
    const v = obj.obj_new_int(42);
    fixture.frame.set("x", v);
    observed_value = fixture.frame.get("x");
}

test "with_interp + frame.set/get round-trip" {
    observed_value = 0;
    fixture.with_interp(&body_set_get_round_trip);

    // The handle written via ``frame.set`` must be the exact
    // handle we read back — ``local_set`` stores handles
    // verbatim.
    try testing.expect(observed_value != 0);
    try testing.expectEqual(@as(i64, 42), obj.obj_get_int(observed_value));
}

fn body_var_resolve_hits_frame() void {
    const v = obj.obj_new_int(7);
    fixture.frame.set("count", v);
    const name_bytes = "count";
    const name = obj.obj_new_string(
        @intCast(@intFromPtr(name_bytes.ptr)),
        @intCast(name_bytes.len),
    );
    observed_value = frames.var_resolve(name);
}

test "with_interp lets var_resolve see frame locals" {
    observed_value = 0;
    fixture.with_interp(&body_var_resolve_hits_frame);

    try testing.expect(observed_value != 0);
    try testing.expectEqual(@as(i64, 7), obj.obj_get_int(observed_value));
}

fn body_observe_depth() void {
    observed_depth = frames.frame_depth;
}

test "with_interp pushes exactly one frame" {
    observed_depth = 0;
    const before = frames.frame_depth;
    fixture.with_interp(&body_observe_depth);
    const after = frames.frame_depth;

    try testing.expectEqual(before + 1, observed_depth);
    try testing.expectEqual(before, after);
}

fn body_push_extra_frame() void {
    _ = frames.frame_push();
    observed_depth = frames.frame_depth;
    // Deliberately skip the matching pop — the fixture must
    // drain back to the entry depth on its own.
}

test "with_interp drains imbalanced frame pushes" {
    observed_depth = 0;
    const before = frames.frame_depth;
    fixture.with_interp(&body_push_extra_frame);
    const after = frames.frame_depth;

    // Body pushed one beyond ``with_interp``'s own frame.
    try testing.expectEqual(before + 2, observed_depth);
    // Fixture restored the original depth despite the leak.
    try testing.expectEqual(before, after);
}

// -- Catch + interp interaction -------------------------------------

fn body_raise_inside_interp() void {
    const msg = fixture.with_catch(&body_raise_oops);
    if (msg) |m| {
        if (std.mem.eql(u8, m, "oops")) observed_value = 1;
    }
}

test "with_catch nests cleanly inside with_interp" {
    observed_value = 0;
    fixture.with_interp(&body_raise_inside_interp);
    try testing.expectEqual(@as(i32, 1), observed_value);
}
