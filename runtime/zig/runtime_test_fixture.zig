// Test fixtures for catch-context + interp-state coupled paths.
//
// Issue #266: lets tests assert that a function raises a specific
// Tcl-level error (``stubs.raise`` / ``tcl_cmd_error``) without
// trapping the WASM module, and lets tests build the minimal
// interpreter state (root namespace + global frame) needed by
// ``frames.var_resolve`` / ``frames.var_set`` / ``eval_script``.
//
// File location: ``runtime/zig/`` rather than ``runtime/zig/testing/``.
// Zig 0.16's module system rejects ``@import`` of files above the
// test module's root_source_file directory ("import of file
// outside module path"), and the ``zig build test`` step compiles
// each ``test_*.zig`` as its own module rooted at the test file
// itself.  Keeping the fixture beside the existing root-level
// ``test_tcl_arith.zig`` / ``test_tcl_dict.zig`` lets it reach
// into ``valtypes/`` / ``interp/`` / ``stubs/`` the same way those
// tests already do.  The ``runtime_test_fixture`` filename
// deliberately avoids the ``test_`` prefix so the build's
// ``test_*.zig`` walker doesn't mistake the fixture itself for a
// test binary — only the companion ``test_fixture.zig`` is
// discovered and run.

const std = @import("std");

const obj = @import("valtypes/tcl_obj.zig");
const catch_mod = @import("interp/tcl_catch.zig");
const frames = @import("interp/tcl_frames.zig");
const ns = @import("interp/tcl_ns.zig");

// -- Catch fixture ---------------------------------------------------

/// Maximum size of the captured error-message slice returned by
/// :func:`catch_leave` / :func:`with_catch`.  Real Tcl error
/// messages are short ("divide by zero", "unsupported command:
/// regsub", …); 4 KB is generous and keeps the fixture's
/// statically-allocated buffer trivially cheap.  Longer messages
/// are truncated rather than allocated dynamically — tests that
/// genuinely need the full text can call :func:`catch_mod.error_msg`
/// directly.
const ERROR_MSG_CAP: usize = 4096;

/// Fixture-owned buffer the most recent ``catch_leave`` copies the
/// raised error message into.  We deliberately don't return a
/// slice into the runtime's TclObj string store: ``obj.alloc``
/// routes through libc ``malloc`` with size-class recycling, so a
/// freed message buffer can be reused for the next allocation;
/// and ``obj_ensure_string`` for tagged-immediate values returns
/// pointers into a wrapping ring buffer (see ``S6.4`` in
/// ``valtypes/tcl_obj.zig``).  Copying into a fixture-owned slot
/// pins the slice's lifetime to the fixture, with the trade-off
/// that consecutive ``with_catch`` / ``catch_leave`` calls
/// overwrite earlier slices — assertions must consume the message
/// before the next leave.
var error_msg_buf: [ERROR_MSG_CAP]u8 = undefined;
var error_msg_len: usize = 0;

/// Install a top-level catch frame, run *body*, and report the
/// outcome.  Returns ``null`` when *body* completed without raising
/// a Tcl-level error, or the raised error message as a byte slice
/// into the fixture-owned buffer otherwise.  The slice is valid
/// until the next ``with_catch`` / ``catch_leave`` call (which
/// overwrites the buffer) — long enough for tests that assert
/// against the message immediately, but assertions must be made
/// before another fixture call mutates the slot.
///
/// Why ``?[]const u8`` rather than a ``error{TclError}`` boundary:
/// tests routinely assert the *exact* message
/// (``"divide by zero"``, ``"unsupported command: format"``); a
/// Zig error value loses the message and forces every test to
/// thread a separate buffer.  The optional-slice shape lets a
/// successful run still ``try testing.expectEqual(null, …)`` and
/// an error case ``try testing.expectEqualStrings("…", msg.?)``.
pub fn with_catch(body: *const fn () void) ?[]const u8 {
    catch_enter();
    body();
    return catch_leave();
}

/// Lower-level primitive: enter a catch scope.  Pair with
/// :func:`catch_leave` so tests that prefer ``defer``-based
/// teardown — or need to inspect intermediate state between the
/// raise and the leave — can use the same plumbing as
/// :func:`with_catch`.
pub fn catch_enter() void {
    catch_mod.catch_enter();
}

/// Leave the catch scope and report what happened.  Returns the
/// raised error message as a byte slice into the fixture-owned
/// buffer (see :var:`error_msg_buf`) or ``null`` on success.
/// Always clears the runtime's catch / error globals
/// (``error_flag``, ``error_msg``, ``last_catch_had_error``,
/// ``catch_ok_result``) so the next test starts with a clean
/// slate — leaving any of them set could leak into a subsequent
/// ``catch_result()`` call, which keys its return value on
/// ``last_catch_had_error``.
pub fn catch_leave() ?[]const u8 {
    const had_error = catch_mod.state.error_flag;
    const msg_obj = catch_mod.state.error_msg;
    _ = catch_mod.catch_leave();
    catch_mod.state.error_flag = 0;
    catch_mod.state.error_msg = 0;
    catch_mod.state.last_catch_had_error = 0;
    catch_mod.state.catch_ok_result = 0;
    if (had_error == 0) return null;
    if (msg_obj == 0) {
        error_msg_len = 0;
        return error_msg_buf[0..0];
    }
    const s = obj.obj_ensure_string(msg_obj);
    if (s.len == 0) {
        error_msg_len = 0;
        return error_msg_buf[0..0];
    }
    const src: [*]const u8 = @ptrFromInt(s.ptr);
    const len = if (s.len > ERROR_MSG_CAP) ERROR_MSG_CAP else s.len;
    var i: usize = 0;
    while (i < len) : (i += 1) error_msg_buf[i] = src[i];
    error_msg_len = len;
    return error_msg_buf[0..len];
}

// -- Interp fixture --------------------------------------------------

/// Initialise a minimal interpreter, run *body* inside it, and
/// tear the state back down on exit.  The fixture:
///
///   1. resets the loop-flow flags (``return_flag``,
///      ``break_flag``, ``continue_flag``) so prior tests can't
///      leak loop or proc-dispatch state in,
///   2. ensures the root namespace exists and re-anchors
///      ``current_ns`` to it, and
///   3. pushes a fresh global frame so ``var_resolve`` /
///      ``var_set`` / ``local_set`` / ``local_get`` have somewhere
///      to land.
///
/// On exit the fixture pops back to the depth captured before
/// *body* ran (defensive against bodies that imbalanced
/// ``frame_push`` / ``frame_pop`` on their own) and clears the
/// loop-flow flags again.
///
/// Catch state (``catch_depth`` / ``error_flag`` / ``error_msg``)
/// is owned by :func:`with_catch` and is *not* touched here —
/// otherwise a test wrapping ``with_interp`` inside an outer
/// ``with_catch`` would lose the outer catch scope and any
/// raise inside the interp body would trap the WASM binary
/// instead of setting ``error_flag``.  The two fixtures must
/// compose in either nesting order.
///
/// State sharing: every ``test_*.zig`` compiles to its own WASM
/// binary (see ``build.zig``'s test step), so global state is
/// isolated *across* test binaries.  Within one binary, sharing
/// the bump allocator across tests is fine — the state we touch
/// is the call-frame stack, the namespace tree, and the
/// loop-flow flags, all of which this fixture resets between
/// runs.  Per-test setup was preferred over a single
/// shared-interp lifecycle so a leak in one test surfaces in that
/// test rather than smearing into the next.
pub fn with_interp(body: *const fn () void) void {
    reset_loop_flags();
    _ = ns.ns_root();
    ns.current_ns = ns.ns_root();
    const saved_depth = frames.frame_depth;
    _ = frames.frame_push();
    body();
    // Defensive teardown.  Two failure modes:
    //   * Body pushed extra frames and forgot to pop — drain back
    //     to the entry depth so the next test sees a clean stack.
    //   * Body popped *more* frames than it pushed — the fixture's
    //     own frame is gone and the depth is below where we
    //     started.  Trap loudly rather than silently corrupt
    //     subsequent tests; the bug is in the test body and a
    //     surfaced trap names the offending file.
    if (frames.frame_depth < saved_depth) {
        std.debug.panic(
            "with_interp: body popped past entry depth (saved={d}, now={d})",
            .{ saved_depth, frames.frame_depth },
        );
    }
    while (frames.frame_depth > saved_depth) frames.frame_pop();
    reset_loop_flags();
}

fn reset_loop_flags() void {
    catch_mod.state.return_flag = 0;
    catch_mod.state.return_val = 0;
    catch_mod.state.break_flag = 0;
    catch_mod.state.continue_flag = 0;
}

// -- Frame helpers ---------------------------------------------------

/// Convenience accessors for the topmost frame's locals.  Tests
/// that just need to seed a few variables before calling into a
/// function under test reach for ``frame.set`` / ``frame.get``
/// rather than hand-rolling the ``obj_new_string`` + ``local_set``
/// dance.
///
/// Reads and writes go through the same ``local_set`` /
/// ``local_get`` path the interpreter uses, so aliases set up via
/// ``frame_alias_global`` / ``frame_alias_named`` are honoured.
pub const frame = struct {
    /// Set local *name* to *value* in the current frame.  The
    /// fixture does not retain ownership of the value handle; if
    /// it should outlive the test, retain it explicitly.
    pub fn set(name: []const u8, value: i32) void {
        _ = frames.local_set(name_obj(name), value);
    }

    /// Read local *name* from the current frame.  Returns 0 when
    /// the name is unset or no frame is active.
    pub fn get(name: []const u8) i32 {
        return frames.local_get(name_obj(name));
    }
};

/// Build a TclObj wrapping a *copy* of *name*.  Frames hash on the
/// stored ``name_ptr`` / ``name_len`` and keep the pointer live in
/// the bucket without taking a copy; passing a stack or temporary
/// slice via plain ``obj_new_string`` would leave the bucket
/// holding dangling key bytes once the slice's backing storage
/// goes out of scope.  ``obj_new_string_copy`` allocates a
/// runtime-owned buffer (or stashes short strings inline in the
/// obj header), so the key bytes outlive the caller.
fn name_obj(name: []const u8) i32 {
    return obj.obj_new_string_copy(
        @intCast(@intFromPtr(name.ptr)),
        @intCast(name.len),
    );
}
