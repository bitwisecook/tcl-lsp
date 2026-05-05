// Unit tests for ``dispatch/tcl_cmd_registry.zig`` — the linear-scan
// command lookup helpers used by ``tcl_cmd_table.zig`` and the
// per-command sub-dispatchers.
//
// Wave 5 of the WASM-runtime test suite (#268).  These cover the
// cheapest entry point of the dispatch layer: pure functions that
// consume a (name_ptr, name_len) byte slice and a static table,
// returning either a handler / entry or ``null``.  No interpreter
// state, no globals — keeps the test surface small enough to pin
// down regressions in the linear-scan logic by name.

const std = @import("std");
const testing = std.testing;

const reg = @import("dispatch/tcl_cmd_registry.zig");
const result_mod = @import("interp/tcl_result.zig");
const InterpResult = result_mod.InterpResult;

// -- Handlers --------------------------------------------------------
//
// Each test handler returns a unique sentinel so we can confirm
// ``lookup`` returned the right ``CmdEntry`` rather than just *some*
// entry whose name happened to match.  Returning a fresh integer per
// handler keeps the test independent of the runtime's TclObj
// allocator — the sentinel rides the ``InterpResult.value`` field that
// the dispatcher would otherwise treat as a TclObj handle.

fn h_set(_: []const i32) InterpResult {
    return result_mod.ok(1);
}

fn h_unset(_: []const i32) InterpResult {
    return result_mod.ok(2);
}

fn h_setvar(_: []const i32) InterpResult {
    return result_mod.ok(3);
}

fn h_string(_: []const i32) InterpResult {
    return result_mod.ok(4);
}

fn h_sub_length(_: []const i32) InterpResult {
    return result_mod.ok(100);
}

fn h_sub_index(_: []const i32) InterpResult {
    return result_mod.ok(101);
}

fn h_sub_match(_: []const i32) InterpResult {
    return result_mod.ok(102);
}

const TABLE = [_]reg.CmdEntry{
    .{ .name = "set", .arity_min = 1, .arity_max = 2, .handler = &h_set },
    .{ .name = "unset", .arity_min = 1, .arity_max = null, .handler = &h_unset },
    .{ .name = "setvar", .arity_min = 2, .arity_max = 2, .handler = &h_setvar },
    .{ .name = "string", .arity_min = 1, .arity_max = null, .handler = &h_string },
};

const SUBS = [_]reg.SubEntry{
    .{ .name = "length", .arity_min = 1, .arity_max = 1, .handler = &h_sub_length },
    .{ .name = "index", .arity_min = 2, .arity_max = 2, .handler = &h_sub_index },
    .{ .name = "match", .arity_min = 2, .arity_max = 3, .handler = &h_sub_match },
};

// Pass a slice's bytes via the (ptr, len) pair the lookup helpers
// expect.  ``@intFromPtr`` over a string literal gives us a stable
// address inside the test binary's data segment for the duration of
// the call.
fn ptr_of(s: []const u8) u32 {
    return @intCast(@intFromPtr(s.ptr));
}

// -- lookup ----------------------------------------------------------

test "lookup returns the registered handler on hit" {
    const name = "set";
    const h = reg.lookup(&TABLE, ptr_of(name), name.len) orelse return error.TestUnexpectedNull;
    try testing.expectEqual(@as(i32, 1), h(&[_]i32{}).value);
}

test "lookup returns null on miss" {
    const name = "nonexistent";
    try testing.expect(reg.lookup(&TABLE, ptr_of(name), name.len) == null);
}

test "lookup returns null for empty name" {
    // Without the early-exit guard a zero-length input would match
    // any zero-length entry name (none exist today, but the guard
    // makes the contract explicit).
    const name = "set";
    try testing.expect(reg.lookup(&TABLE, ptr_of(name), 0) == null);
}

test "lookup distinguishes names that share a prefix" {
    // ``set`` and ``setvar`` are length-distinct.  The length-first
    // skip in :func:`reg.lookup` should rule out the wrong one
    // before any byte compare.
    const set_name = "set";
    const setvar_name = "setvar";

    const h_for_set = reg.lookup(&TABLE, ptr_of(set_name), set_name.len) orelse return error.TestUnexpectedNull;
    const h_for_setvar = reg.lookup(&TABLE, ptr_of(setvar_name), setvar_name.len) orelse return error.TestUnexpectedNull;

    try testing.expectEqual(@as(i32, 1), h_for_set(&[_]i32{}).value);
    try testing.expectEqual(@as(i32, 3), h_for_setvar(&[_]i32{}).value);
}

test "lookup is case-sensitive" {
    // Tcl command names are case-sensitive (``Set`` is not ``set``).
    const upper = "Set";
    try testing.expect(reg.lookup(&TABLE, ptr_of(upper), upper.len) == null);
}

test "lookup honours name_len boundary — does not over-read" {
    // The table has ``"set"`` (length 3).  Pass the prefix of a longer
    // buffer with len=3 and confirm the lookup matches without
    // reading past the boundary.  Embedding the prefix inside a
    // longer buffer catches a regression where the linear scan
    // walked beyond ``name_len``.
    const buf = "setvar";
    const h = reg.lookup(&TABLE, ptr_of(buf), 3) orelse return error.TestUnexpectedNull;
    try testing.expectEqual(@as(i32, 1), h(&[_]i32{}).value);
}

test "lookup against an empty table is always null" {
    const empty = [_]reg.CmdEntry{};
    const name = "set";
    try testing.expect(reg.lookup(&empty, ptr_of(name), name.len) == null);
}

// -- lookup_entry ----------------------------------------------------

test "lookup_entry surfaces arity bounds for an exact match" {
    const name = "set";
    const e = reg.lookup_entry(&TABLE, ptr_of(name), name.len) orelse return error.TestUnexpectedNull;
    try testing.expectEqual(@as(u32, 1), e.arity_min);
    try testing.expectEqual(@as(?u32, 2), e.arity_max);
}

test "lookup_entry exposes variadic upper bound as null" {
    const name = "string";
    const e = reg.lookup_entry(&TABLE, ptr_of(name), name.len) orelse return error.TestUnexpectedNull;
    try testing.expect(e.arity_max == null);
    try testing.expectEqual(@as(u32, 1), e.arity_min);
}

test "lookup_entry returns null on miss" {
    const name = "nope";
    try testing.expect(reg.lookup_entry(&TABLE, ptr_of(name), name.len) == null);
}

test "lookup_entry returns null on zero-length name" {
    const name = "set";
    try testing.expect(reg.lookup_entry(&TABLE, ptr_of(name), 0) == null);
}

// -- lookup_sub ------------------------------------------------------

test "lookup_sub matches a registered sub-command" {
    const name = "length";
    const s = reg.lookup_sub(&SUBS, ptr_of(name), name.len) orelse return error.TestUnexpectedNull;
    try testing.expectEqual(@as(u32, 1), s.arity_min);
    try testing.expectEqual(@as(?u32, 1), s.arity_max);
    try testing.expectEqual(@as(i32, 100), s.handler(&[_]i32{}).value);
}

test "lookup_sub returns null on miss" {
    const name = "wibble";
    try testing.expect(reg.lookup_sub(&SUBS, ptr_of(name), name.len) == null);
}

test "lookup_sub returns null on zero-length name" {
    const name = "length";
    try testing.expect(reg.lookup_sub(&SUBS, ptr_of(name), 0) == null);
}

test "lookup_sub picks the correct entry across length-distinct names" {
    const len_name = "length";
    const idx_name = "index";

    const len_entry = reg.lookup_sub(&SUBS, ptr_of(len_name), len_name.len) orelse return error.TestUnexpectedNull;
    const idx_entry = reg.lookup_sub(&SUBS, ptr_of(idx_name), idx_name.len) orelse return error.TestUnexpectedNull;

    try testing.expectEqual(@as(i32, 100), len_entry.handler(&[_]i32{}).value);
    try testing.expectEqual(@as(i32, 101), idx_entry.handler(&[_]i32{}).value);
}

test "lookup_sub honours sub-arity range" {
    const name = "match";
    const s = reg.lookup_sub(&SUBS, ptr_of(name), name.len) orelse return error.TestUnexpectedNull;
    try testing.expectEqual(@as(u32, 2), s.arity_min);
    try testing.expectEqual(@as(?u32, 3), s.arity_max);
}

// -- defaults --------------------------------------------------------

test "CmdEntry default arity is 0..variadic" {
    // The defaults exist so unannotated registrations compile; the
    // parity tool fills explicit values in.  This pins the contract
    // so a future change to the defaults requires updating both
    // sides intentionally.
    const e = reg.CmdEntry{ .name = "x", .handler = &h_set };
    try testing.expectEqual(@as(u32, 0), e.arity_min);
    try testing.expect(e.arity_max == null);
}

test "SubEntry default arity is 0..variadic" {
    const s = reg.SubEntry{ .name = "y", .handler = &h_sub_length };
    try testing.expectEqual(@as(u32, 0), s.arity_min);
    try testing.expect(s.arity_max == null);
}
