// Unit tests for ``dispatch/tcl_diag.zig`` — the source-location
// diagnostic register the codegen pokes before each trap-able call.
//
// The module's surface is two single-slot registers (``current_site_id``
// and the three-tuple ``current_eval_*``) plus their setters and the
// two writers that emit them to a file descriptor.  We pin the
// state-machine half here; the writer half is exercised end-to-end
// by the trap-output regressions in the Python integration suite
// (which can capture stderr while a wasmtime harness drives the
// runtime).  The boolean return of :func:`diag.write_prefix` is
// covered below — that's the part the trap formatter branches on,
// and it doesn't depend on the fd actually accepting bytes.
//
// Wave 5 of the WASM-runtime test suite (#268).

const std = @import("std");
const testing = std.testing;

const diag = @import("dispatch/tcl_diag.zig");

// -- diag_set --------------------------------------------------------

test "diag_set updates current_site_id" {
    diag.current_site_id = 0;
    diag.diag_set(42);
    try testing.expectEqual(@as(u32, 42), diag.current_site_id);
}

test "diag_set overwrites — single-slot register, last wins" {
    diag.diag_set(100);
    diag.diag_set(200);
    try testing.expectEqual(@as(u32, 200), diag.current_site_id);
}

test "diag_set with 0 clears the register" {
    diag.diag_set(123);
    diag.diag_set(0);
    try testing.expectEqual(@as(u32, 0), diag.current_site_id);
}

// -- diag_set_eval_ctx ----------------------------------------------

test "diag_set_eval_ctx fills all three slots" {
    const buf = "set x 42";
    const ptr: i32 = @intCast(@intFromPtr(buf.ptr));
    diag.diag_set_eval_ctx(ptr, @intCast(buf.len), 4);
    try testing.expectEqual(@as(u32, @intCast(ptr)), diag.current_eval_ptr);
    try testing.expectEqual(@as(u32, buf.len), diag.current_eval_len);
    try testing.expectEqual(@as(u32, 4), diag.current_eval_pos);
}

test "diag_set_eval_ctx with all zeros clears the slots" {
    const buf = "x";
    diag.diag_set_eval_ctx(@intCast(@intFromPtr(buf.ptr)), 1, 0);
    diag.diag_set_eval_ctx(0, 0, 0);
    try testing.expectEqual(@as(u32, 0), diag.current_eval_ptr);
    try testing.expectEqual(@as(u32, 0), diag.current_eval_len);
    try testing.expectEqual(@as(u32, 0), diag.current_eval_pos);
}

// -- write_prefix return value --------------------------------------
//
// ``write_prefix`` returns true iff a site was registered.  Trap
// formatters branch on the return to decide whether to emit a
// leading space.  We pin only the no-site-registered case in unit
// tests — the registered-site path emits to a file descriptor and
// is exercised end-to-end by the trap-output regressions in the
// Python integration suite, which can capture stderr while a
// wasmtime harness drives the runtime.

test "write_prefix returns false when no site is registered" {
    diag.current_site_id = 0;
    // No site set means the writer takes the early-exit branch and
    // never touches the fd — safe to pass an arbitrary value.
    try testing.expect(!diag.write_prefix(2));
}

// -- write_eval_ctx no-ops on cleared state -------------------------
//
// The writer's contract is "do nothing if no eval context is set".
// The cleared-state path takes the early-return without writing
// to the fd — safe to call from a unit test; the actual write
// path is exercised by the trap regressions in the Python suite.

test "write_eval_ctx is a no-op when ptr/len are cleared" {
    diag.diag_set_eval_ctx(0, 0, 0);
    diag.write_eval_ctx(2);
}
