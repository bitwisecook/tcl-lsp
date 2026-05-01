// Unit tests for ``interp/tcl_procs.zig`` — the proc registry that
// every ``proc`` keyword and every compiled WASM proc register-at-
// startup hook lands in.
//
// Wave 5 of the WASM-runtime test suite (#268).
//
// Coverage: proc_register / proc_register_compiled — both end up in
// the same ``Command`` slot inside the namespace tree's cmd_table —
// and the per-field accessors that the dispatcher reads
// (``proc_get_n_params``, ``proc_get_n_required``,
// ``proc_get_args_tail``, ``proc_get_func_idx``,
// ``proc_get_params``, ``proc_get_body``, ``proc_get_name_*``).
// ``proc_lookup`` round-trips registration → resolution.
//
// We pick test-specific proc names per case so leftover registrations
// from earlier tests don't collide.  The bump allocator never
// reclaims, so a stale slot is harmless.

const std = @import("std");
const testing = std.testing;

const obj = @import("valtypes/tcl_obj.zig");
const procs = @import("interp/tcl_procs.zig");
const ns = @import("interp/tcl_ns.zig");
const fixture = @import("runtime_test_fixture.zig");

// -- helpers --------------------------------------------------------

fn name_obj(s: []const u8) i32 {
    return obj.obj_new_string_copy(
        @intCast(@intFromPtr(s.ptr)),
        @intCast(s.len),
    );
}

fn slice_at(p: u32, l: u32) []const u8 {
    if (l == 0) return &[_]u8{};
    const src: [*]const u8 = @ptrFromInt(p);
    return src[0..l];
}

// Module-level capture slots — the `with_interp` body callbacks need
// somewhere to write their observations.  Reset at the top of each
// test so a stale value from a previous case can't leak through.
var captured_bucket: i32 = 0;
var captured_int: i64 = 0;
var captured_int2: i64 = 0;
var captured_handle: i32 = 0;
var captured_name: [128]u8 = undefined;
var captured_name_len: usize = 0;

// -- proc_register (interpreted procs) ------------------------------

fn body_register_then_lookup() void {
    _ = procs.proc_register(
        name_obj("test_proc_register_basic"),
        name_obj("a b c"),
        name_obj("set x 1"),
    );
    captured_bucket = procs.proc_lookup(name_obj("test_proc_register_basic"));
}

test "proc_register followed by proc_lookup returns the registered Command" {
    captured_bucket = 0;
    fixture.with_interp(&body_register_then_lookup);
    try testing.expect(captured_bucket != 0);
}

fn body_register_records_n_params() void {
    _ = procs.proc_register(
        name_obj("test_proc_n_params"),
        name_obj("alpha beta gamma delta"),
        name_obj("expr {1}"),
    );
    const bucket = procs.proc_lookup(name_obj("test_proc_n_params"));
    captured_int = @as(i64, procs.proc_get_n_params(bucket));
}

test "proc_register infers n_params from the params list length" {
    captured_int = 0;
    fixture.with_interp(&body_register_records_n_params);
    try testing.expectEqual(@as(i64, 4), captured_int);
}

fn body_register_returns_body_handle() void {
    const body = name_obj("set x 7");
    _ = procs.proc_register(
        name_obj("test_proc_body_round_trip"),
        name_obj(""),
        body,
    );
    const bucket = procs.proc_lookup(name_obj("test_proc_body_round_trip"));
    captured_handle = procs.proc_get_body(bucket);
}

test "proc_get_body returns a TclObj whose string is the registered body" {
    captured_handle = 0;
    fixture.with_interp(&body_register_returns_body_handle);
    try testing.expect(captured_handle != 0);
    const s = obj.obj_ensure_string(captured_handle);
    try testing.expectEqualStrings("set x 7", slice_at(s.ptr, s.len));
}

fn body_register_returns_params_handle() void {
    _ = procs.proc_register(
        name_obj("test_proc_params_round_trip"),
        name_obj("x y"),
        name_obj("expr {$x + $y}"),
    );
    const bucket = procs.proc_lookup(name_obj("test_proc_params_round_trip"));
    captured_handle = procs.proc_get_params(bucket);
}

test "proc_get_params returns a TclObj whose string is the params list" {
    captured_handle = 0;
    fixture.with_interp(&body_register_returns_params_handle);
    try testing.expect(captured_handle != 0);
    const s = obj.obj_ensure_string(captured_handle);
    try testing.expectEqualStrings("x y", slice_at(s.ptr, s.len));
}

fn body_register_redefine_overwrites_body() void {
    _ = procs.proc_register(
        name_obj("test_proc_redefine"),
        name_obj("a"),
        name_obj("set first 1"),
    );
    _ = procs.proc_register(
        name_obj("test_proc_redefine"),
        name_obj("a b"),
        name_obj("set second 2"),
    );
    const bucket = procs.proc_lookup(name_obj("test_proc_redefine"));
    captured_int = @as(i64, procs.proc_get_n_params(bucket));
    captured_handle = procs.proc_get_body(bucket);
}

test "proc_register on an existing name overwrites params + body" {
    captured_int = 0;
    captured_handle = 0;
    fixture.with_interp(&body_register_redefine_overwrites_body);
    // Second registration declared two params, so the slot reflects
    // the new definition rather than the old.
    try testing.expectEqual(@as(i64, 2), captured_int);
    const s = obj.obj_ensure_string(captured_handle);
    try testing.expectEqualStrings("set second 2", slice_at(s.ptr, s.len));
}

// -- proc_register_compiled ----------------------------------------

fn body_register_compiled_records_func_idx() void {
    _ = procs.proc_register_compiled(
        name_obj("test_proc_compiled_func"),
        3, // n_params
        77, // func_idx
        0, // args_tail
        2, // n_required
    );
    const bucket = procs.proc_lookup(name_obj("test_proc_compiled_func"));
    captured_bucket = bucket;
    captured_int = @as(i64, procs.proc_get_func_idx(bucket));
    captured_int2 = @as(i64, procs.proc_get_n_required(bucket));
}

test "proc_register_compiled stamps func_idx and n_required" {
    captured_bucket = 0;
    captured_int = 0;
    captured_int2 = 0;
    fixture.with_interp(&body_register_compiled_records_func_idx);
    try testing.expect(captured_bucket != 0);
    try testing.expectEqual(@as(i64, 77), captured_int);
    try testing.expectEqual(@as(i64, 2), captured_int2);
}

fn body_register_compiled_with_args_tail() void {
    _ = procs.proc_register_compiled(
        name_obj("test_proc_args_tail"),
        2,
        88,
        1, // args_tail = true
        1, // n_required (one fixed arg before the tail)
    );
    const bucket = procs.proc_lookup(name_obj("test_proc_args_tail"));
    captured_int = @as(i64, procs.proc_get_args_tail(bucket));
    captured_int2 = @as(i64, procs.proc_get_n_params(bucket));
}

test "proc_register_compiled records the args-tail variadic flag" {
    captured_int = -1;
    captured_int2 = 0;
    fixture.with_interp(&body_register_compiled_with_args_tail);
    try testing.expectEqual(@as(i64, 1), captured_int);
    try testing.expectEqual(@as(i64, 2), captured_int2);
}

fn body_register_compiled_stamps_export_name() void {
    _ = procs.proc_register_compiled(
        name_obj("test_proc_export_name"),
        0,
        12,
        0,
        0,
    );
    const bucket = procs.proc_lookup(name_obj("test_proc_export_name"));
    const en = procs.proc_get_export_name(@bitCast(bucket));
    captured_name_len = if (en.len < captured_name.len) en.len else captured_name.len;
    if (captured_name_len > 0) {
        const src: [*]const u8 = @ptrFromInt(en.ptr);
        var i: usize = 0;
        while (i < captured_name_len) : (i += 1) captured_name[i] = src[i];
    }
}

test "proc_register_compiled stamps the registration-time export name in the sidecar" {
    captured_name_len = 0;
    fixture.with_interp(&body_register_compiled_stamps_export_name);
    try testing.expect(captured_name_len > 0);
    try testing.expectEqualStrings(
        "test_proc_export_name",
        captured_name[0..captured_name_len],
    );
}

// -- proc_lookup miss path -----------------------------------------

fn body_lookup_unknown_returns_zero() void {
    captured_bucket = procs.proc_lookup(name_obj("test_proc_truly_does_not_exist"));
}

test "proc_lookup returns 0 for an unregistered name" {
    captured_bucket = 1; // sentinel — must be overwritten
    fixture.with_interp(&body_lookup_unknown_returns_zero);
    try testing.expectEqual(@as(i32, 0), captured_bucket);
}

// -- proc_exists ---------------------------------------------------

fn body_proc_exists_round_trip() void {
    _ = procs.proc_register(
        name_obj("test_proc_exists_yes"),
        name_obj(""),
        name_obj("return 1"),
    );
    captured_int = obj.obj_get_int(procs.proc_exists(name_obj("test_proc_exists_yes")));
    captured_int2 = obj.obj_get_int(procs.proc_exists(name_obj("test_proc_exists_no")));
}

test "proc_exists returns 1 for registered, 0 for unknown" {
    captured_int = -1;
    captured_int2 = -1;
    fixture.with_interp(&body_proc_exists_round_trip);
    try testing.expectEqual(@as(i64, 1), captured_int);
    try testing.expectEqual(@as(i64, 0), captured_int2);
}

// -- proc_get_name_ptr / _len round-trip ---------------------------

fn body_get_name_round_trip() void {
    _ = procs.proc_register(
        name_obj("test_proc_name_round_trip"),
        name_obj(""),
        name_obj(""),
    );
    const bucket = procs.proc_lookup(name_obj("test_proc_name_round_trip"));
    const np: u32 = @bitCast(procs.proc_get_name_ptr(bucket));
    const nl: u32 = @bitCast(procs.proc_get_name_len(bucket));
    captured_name_len = if (nl < captured_name.len) nl else captured_name.len;
    if (captured_name_len > 0) {
        const src: [*]const u8 = @ptrFromInt(np);
        var i: usize = 0;
        while (i < captured_name_len) : (i += 1) captured_name[i] = src[i];
    }
}

test "proc_get_name_ptr / _len return the heap-copied registered name" {
    captured_name_len = 0;
    fixture.with_interp(&body_get_name_round_trip);
    // Top-level procs register with bare names; the FQN is materialised
    // lazily by ``ns_full_name``.  Here we just confirm the simple
    // name is round-tripped.
    try testing.expectEqualStrings(
        "test_proc_name_round_trip",
        captured_name[0..captured_name_len],
    );
}

// -- proc_get_* on a 0 bucket all return 0 (defensive) --------------

test "proc_get_* accessors on a 0 bucket all return 0" {
    try testing.expectEqual(@as(i32, 0), procs.proc_get_func_idx(0));
    try testing.expectEqual(@as(i32, 0), procs.proc_get_n_params(0));
    try testing.expectEqual(@as(i32, 0), procs.proc_get_args_tail(0));
    try testing.expectEqual(@as(i32, 0), procs.proc_get_n_required(0));
    try testing.expectEqual(@as(i32, 0), procs.proc_get_params(0));
    try testing.expectEqual(@as(i32, 0), procs.proc_get_body(0));
    try testing.expectEqual(@as(i32, 0), procs.proc_get_name_ptr(0));
    try testing.expectEqual(@as(i32, 0), procs.proc_get_name_len(0));
}

// -- proc_set_body_source ------------------------------------------

fn body_set_body_source_overwrites() void {
    _ = procs.proc_register_compiled(
        name_obj("test_proc_set_body_source"),
        0,
        99,
        0,
        0,
    );
    const new_body = name_obj("set updated 1");
    _ = procs.proc_set_body_source(name_obj("test_proc_set_body_source"), new_body);
    const bucket = procs.proc_lookup(name_obj("test_proc_set_body_source"));
    captured_handle = procs.proc_get_body(bucket);
}

test "proc_set_body_source attaches a body TclObj to a compiled proc" {
    captured_handle = 0;
    fixture.with_interp(&body_set_body_source_overwrites);
    try testing.expect(captured_handle != 0);
    const s = obj.obj_ensure_string(captured_handle);
    try testing.expectEqualStrings("set updated 1", slice_at(s.ptr, s.len));
}

// -- ns-resolution: qualified registration --------------------------

fn body_register_qualified_lands_in_target_ns() void {
    // ``::myns::myproc`` should land in the ``::myns`` namespace's
    // cmd_table, not in the root.  After registration ``proc_lookup``
    // from inside that ns must find it.
    _ = ns.ns_create_from_fqn(@intCast(@intFromPtr("::test_proc_qualified_ns".ptr)),
                              @intCast("::test_proc_qualified_ns".len));
    _ = procs.proc_register(
        name_obj("::test_proc_qualified_ns::myproc"),
        name_obj(""),
        name_obj("return ok"),
    );
    captured_bucket = procs.proc_lookup(name_obj("::test_proc_qualified_ns::myproc"));
}

test "proc_register on a ::-qualified name lands in the target namespace" {
    captured_bucket = 0;
    fixture.with_interp(&body_register_qualified_lands_in_target_ns);
    try testing.expect(captured_bucket != 0);
}
