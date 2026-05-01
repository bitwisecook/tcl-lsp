// Unit tests for ``interp/tcl_frames.zig`` — the call-frame stack
// (push/pop, depth tracking, per-frame namespace context, argv) and
// the local-variable / alias resolution machinery used by every
// proc dispatch and ``upvar`` / ``global`` / ``variable`` chain.
//
// Wave 5 of the WASM-runtime test suite (#268).  Builds on the
// fixture from #266 — every test runs inside ``with_interp`` so
// the global frame is set up and torn down deterministically, and
// the loop-flow flags are reset between cases.
//
// Coverage focus: frame depth boundaries, alias resolution
// (ALIAS_GLOBAL via ``frame_alias_global`` and named globals via
// ``frame_alias_named``), and the depth-stash / depth-restore
// pair used by ``uplevel``.  These are the paths most likely to
// drift when refactoring frame storage and the hardest to debug
// from a Python-level integration failure.

const std = @import("std");
const testing = std.testing;

const obj = @import("valtypes/tcl_obj.zig");
const frames = @import("interp/tcl_frames.zig");
const ns = @import("interp/tcl_ns.zig");
const fixture = @import("runtime_test_fixture.zig");

// ---- helpers ------------------------------------------------------

fn name_obj(s: []const u8) i32 {
    return obj.obj_new_string_copy(
        @intCast(@intFromPtr(s.ptr)),
        @intCast(s.len),
    );
}

// Module-level slots for closure-style capture out of test bodies.
// ``observed_int`` holds i64s returned by ``obj.obj_get_int``;
// ``observed_handle`` holds raw TclObj IDs; ``observed_depth`` the
// frame depth difference; ``observed_ns`` the per-frame ns slot.
var observed_depth: u32 = 0;
var observed_int: i64 = 0;
var observed_int2: i64 = 0;
var observed_handle: i32 = 0;
var observed_handle2: i32 = 0;
var observed_ns: u32 = 0;

// ---- frame_push / frame_pop / frame_get_depth ---------------------

fn body_push_pop_balance() void {
    const before = frames.frame_depth;
    _ = frames.frame_push();
    _ = frames.frame_push();
    _ = frames.frame_push();
    observed_depth = frames.frame_depth - before;
    frames.frame_pop();
    frames.frame_pop();
    frames.frame_pop();
    observed_int = @intCast(frames.frame_depth - before);
}

test "frame_push/frame_pop balance — three nested pushes restore depth" {
    observed_depth = 0;
    observed_int = 0;
    fixture.with_interp(&body_push_pop_balance);
    try testing.expectEqual(@as(u32, 3), observed_depth);
    try testing.expectEqual(@as(i64, 0), observed_int);
}

fn body_get_depth_matches() void {
    const d1: i32 = frames.frame_get_depth();
    _ = frames.frame_push();
    const d2: i32 = frames.frame_get_depth();
    observed_int = @as(i64, d2 - d1);
    frames.frame_pop();
    const d3: i32 = frames.frame_get_depth();
    observed_int2 = @as(i64, d3 - d1);
}

test "frame_get_depth tracks live frame stack height" {
    observed_int = 0;
    observed_int2 = 0;
    fixture.with_interp(&body_get_depth_matches);
    try testing.expectEqual(@as(i64, 1), observed_int);
    try testing.expectEqual(@as(i64, 0), observed_int2);
}

// ---- local_set / local_get isolation across frames ----------------

fn body_locals_isolate_per_frame() void {
    fixture.frame.set("x", obj.obj_new_int(1));
    _ = frames.frame_push();
    fixture.frame.set("x", obj.obj_new_int(2));
    observed_int = obj.obj_get_int(frames.local_get(name_obj("x")));
    frames.frame_pop();
    observed_int2 = obj.obj_get_int(frames.local_get(name_obj("x")));
}

test "local_set/local_get isolate values across frame boundaries" {
    observed_int = 0;
    observed_int2 = 0;
    fixture.with_interp(&body_locals_isolate_per_frame);
    try testing.expectEqual(@as(i64, 2), observed_int);
    try testing.expectEqual(@as(i64, 1), observed_int2);
}

fn body_local_get_misses_return_zero() void {
    observed_handle = frames.local_get(name_obj("never_set"));
}

test "local_get returns 0 when name is missing" {
    observed_handle = -1;
    fixture.with_interp(&body_local_get_misses_return_zero);
    try testing.expectEqual(@as(i32, 0), observed_handle);
}

// ---- local_exists -------------------------------------------------

fn body_local_exists_true_false() void {
    fixture.frame.set("present", obj.obj_new_int(99));
    observed_int = obj.obj_get_int(frames.local_exists(name_obj("present")));
    observed_int2 = obj.obj_get_int(frames.local_exists(name_obj("absent")));
}

test "local_exists distinguishes set vs unset" {
    observed_int = -1;
    observed_int2 = -1;
    fixture.with_interp(&body_local_exists_true_false);
    try testing.expectEqual(@as(i64, 1), observed_int);
    try testing.expectEqual(@as(i64, 0), observed_int2);
}

// ---- ALIAS_GLOBAL via frame_alias_global --------------------------

fn body_alias_global_propagates_to_global_table() void {
    // Promote local ``g`` to alias the same-name global.  Subsequent
    // local writes must land in globals.
    frames.frame_alias_global(name_obj("g"));
    _ = frames.local_set(name_obj("g"), obj.obj_new_int(42));
    // Read via the global table directly to confirm propagation.
    observed_int = obj.obj_get_int(ns.global_get(name_obj("g")));
    // Read via the local alias — must mirror the global.
    observed_int2 = obj.obj_get_int(frames.local_get(name_obj("g")));
}

test "frame_alias_global routes local writes to the global table" {
    observed_int = 0;
    observed_int2 = 0;
    fixture.with_interp(&body_alias_global_propagates_to_global_table);
    try testing.expectEqual(@as(i64, 42), observed_int);
    try testing.expectEqual(@as(i64, 42), observed_int2);
}

// ---- ALIAS_EXT via frame_alias_named ------------------------------

fn body_alias_named_remaps_local_to_other_global() void {
    // Local ``alias_local`` aliases the global ``target_global``.
    frames.frame_alias_named(name_obj("alias_local"), name_obj("target_global"));
    _ = frames.local_set(name_obj("alias_local"), obj.obj_new_int(7));
    // The aliased global should hold the value.
    observed_int = obj.obj_get_int(ns.global_get(name_obj("target_global")));
    // The local *name* should NOT show up in globals — only the
    // aliased target does.
    observed_int2 = obj.obj_get_int(ns.global_exists(name_obj("alias_local")));
}

test "frame_alias_named writes through to the aliased global name" {
    observed_int = 0;
    observed_int2 = -1;
    fixture.with_interp(&body_alias_named_remaps_local_to_other_global);
    try testing.expectEqual(@as(i64, 7), observed_int);
    try testing.expectEqual(@as(i64, 0), observed_int2);
}

// ---- var_resolve precedence ---------------------------------------

fn body_var_resolve_prefers_frame_over_global() void {
    _ = ns.global_set(name_obj("collide"), obj.obj_new_int(99));
    fixture.frame.set("collide", obj.obj_new_int(11));
    observed_int = obj.obj_get_int(frames.var_resolve(name_obj("collide")));
}

test "var_resolve prefers a frame local over a same-name global" {
    observed_int = 0;
    fixture.with_interp(&body_var_resolve_prefers_frame_over_global);
    try testing.expectEqual(@as(i64, 11), observed_int);
}

fn body_var_resolve_qualified_goes_global() void {
    // ``::``-qualified names bypass the frame — even when a same-name
    // local exists.  Important: this is what makes ``$::env(FOO)`` /
    // ``$::someGlobal`` resolve correctly inside procs.
    _ = ns.global_set(name_obj("only_global"), obj.obj_new_int(55));
    fixture.frame.set("::only_global", obj.obj_new_int(0)); // would-be confusion
    observed_int = obj.obj_get_int(frames.var_resolve(name_obj("::only_global")));
}

test "var_resolve sends ::-qualified names directly to globals" {
    observed_int = 0;
    fixture.with_interp(&body_var_resolve_qualified_goes_global);
    try testing.expectEqual(@as(i64, 55), observed_int);
}

// ---- var_set in current frame -------------------------------------

fn body_var_set_lands_in_frame_when_present() void {
    _ = frames.var_set(name_obj("v"), obj.obj_new_int(3));
    // The local must hold the value; the global must not.
    observed_int = obj.obj_get_int(frames.local_get(name_obj("v")));
    observed_int2 = obj.obj_get_int(ns.global_exists(name_obj("v")));
}

test "var_set targets the current frame when one is active" {
    observed_int = 0;
    observed_int2 = -1;
    fixture.with_interp(&body_var_set_lands_in_frame_when_present);
    try testing.expectEqual(@as(i64, 3), observed_int);
    try testing.expectEqual(@as(i64, 0), observed_int2);
}

// ---- frame_set_argv / frame_get_argv ------------------------------

fn body_argv_round_trip() void {
    const argv = obj.obj_new_string_copy(
        @intCast(@intFromPtr("foo bar baz".ptr)),
        @intCast("foo bar baz".len),
    );
    frames.frame_set_argv(argv);
    observed_handle = frames.frame_get_argv(0);
}

test "frame_set_argv / frame_get_argv round-trip the recorded list" {
    observed_handle = 0;
    fixture.with_interp(&body_argv_round_trip);
    try testing.expect(observed_handle != 0);
    const s = obj.obj_ensure_string(observed_handle);
    try testing.expectEqual(@as(u32, "foo bar baz".len), s.len);
}

fn body_argv_unset_returns_zero() void {
    // Fresh frame — no argv recorded.
    observed_handle = frames.frame_get_argv(0);
}

test "frame_get_argv returns 0 when argv is unrecorded" {
    observed_handle = -1;
    fixture.with_interp(&body_argv_unset_returns_zero);
    try testing.expectEqual(@as(i32, 0), observed_handle);
}

fn body_argv_offset_oob_returns_zero() void {
    // Negative and out-of-range offsets must return 0, not trap.
    observed_handle = frames.frame_get_argv(-1);
    observed_handle2 = frames.frame_get_argv(@intCast(frames.frame_depth + 5));
}

test "frame_get_argv returns 0 for out-of-range offsets" {
    observed_handle = -1;
    observed_handle2 = -1;
    fixture.with_interp(&body_argv_offset_oob_returns_zero);
    try testing.expectEqual(@as(i32, 0), observed_handle);
    try testing.expectEqual(@as(i32, 0), observed_handle2);
}

// ---- frame_set_ns / frame_get_ns ----------------------------------

fn body_frame_ns_round_trip() void {
    const fake_ns: u32 = 0xDEADBEEF; // just a stable sentinel — we
    // never deref it; ``frame_set_ns`` stores u32 verbatim.
    frames.frame_set_ns(fake_ns);
    observed_ns = frames.frame_get_ns(0);
}

test "frame_set_ns / frame_get_ns store + read the per-frame ns slot" {
    observed_ns = 0;
    fixture.with_interp(&body_frame_ns_round_trip);
    try testing.expectEqual(@as(u32, 0xDEADBEEF), observed_ns);
}

fn body_frame_ns_oob_zero() void {
    observed_ns = frames.frame_get_ns(-1);
}

test "frame_get_ns returns 0 for negative offsets" {
    observed_ns = 0xCAFEBABE;
    fixture.with_interp(&body_frame_ns_oob_zero);
    try testing.expectEqual(@as(u32, 0), observed_ns);
}

// ---- frame_depth_stash / frame_depth_restore ----------------------

fn body_depth_stash_restore_cycles_depth() void {
    _ = frames.frame_push();
    _ = frames.frame_push();
    const before = frames.frame_depth;
    const saved = frames.frame_depth_stash(2);
    observed_int = @intCast(before - frames.frame_depth);
    frames.frame_depth_restore(saved);
    observed_int2 = @intCast(frames.frame_depth - (before - 2));
    // Drain the two extra pushes so ``with_interp`` can teardown
    // cleanly back to its entry depth.
    frames.frame_pop();
    frames.frame_pop();
}

test "frame_depth_stash drops by relative_up; restore returns to saved" {
    observed_int = 0;
    observed_int2 = 0;
    fixture.with_interp(&body_depth_stash_restore_cycles_depth);
    try testing.expectEqual(@as(i64, 2), observed_int);
    try testing.expectEqual(@as(i64, 2), observed_int2);
}

fn body_depth_stash_clamps_relative_up() void {
    // Stashing further than the current depth must clamp to 0
    // rather than wrap.  Without the clamp, an over-deep
    // ``uplevel #N`` would underflow ``frame_depth``.
    _ = frames.frame_push();
    _ = frames.frame_depth_stash(99);
    observed_int = @intCast(frames.frame_depth);
    frames.frame_depth_restore(@intCast(frames.frame_depth));
    // After restore we still need to drop the extra push.
    if (frames.frame_depth > 1) frames.frame_pop();
}

test "frame_depth_stash clamps over-deep relative_up to zero" {
    observed_int = -1;
    fixture.with_interp(&body_depth_stash_clamps_relative_up);
    try testing.expectEqual(@as(i64, 0), observed_int);
}
