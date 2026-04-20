// Error handling: ``catch`` scope management + ``@"error"`` trap /
// catch-flag entry point.  Previously this file also carried silent
// stubs for ``format`` / ``regexp`` / ``open`` / ``close`` / ``read``
// / ``gets``; those have moved to area-specific stub files
// (``tcl_io_stubs.zig``, ``tcl_fmt_stubs.zig``) and now raise
// ``unsupported command: <name>`` through :func:`tcl_stubs.unsupported`.

const obj = @import("tcl_obj.zig");
const io = @import("tcl_io.zig");
const diag = @import("tcl_diag.zig");
const globals = @import("tcl_ns.zig"); // global_set lives in tcl_ns post-P3.4
const obj_ensure_string = obj.obj_ensure_string;
const obj_new_int = obj.obj_new_int;
const obj_new_string = obj.obj_new_string;
const obj_new_string_copy = obj.obj_new_string_copy;
const fd_write_all = io.fd_write_all;

// Control flow signals — picol-style return codes as mutable flags.
// Each flag is checked by eval_script after every command.
// Loops catch break/continue; proc dispatch catches return; catch catches error.
pub var catch_depth: u32 = 0;
pub var error_flag: u32 = 0; // 0 = no error, 1 = error pending
pub var error_msg: i32 = 0; // TclObj with error message
pub var return_flag: u32 = 0; // 1 = return pending (absorbed by proc dispatch)
pub var return_val: i32 = 0; // TclObj return value
pub var break_flag: u32 = 0; // 1 = break pending (absorbed by loops)
pub var continue_flag: u32 = 0; // 1 = continue pending (absorbed by loops)

// ``catch body result`` success path: when no error occurs, the result
// variable should receive the return value of the body's last command,
// not the error message (which stays 0).  Compiled catch bodies record
// this value via ``catch_set_ok_result`` so that ``catch_result()`` can
// return the correct value for both success and error cases.
pub var catch_ok_result: i32 = 0;

// Exported: enter a catch scope.
pub export fn catch_enter() void {
    catch_depth += 1;
    error_flag = 0;
    error_msg = 0;
    catch_ok_result = 0;
}

// Exported: record the success result of the catch body's last statement.
// Called by compiled catch bodies after their last statement when a
// result variable is needed.  Ignored if an error already occurred.
pub export fn catch_set_ok_result(val: i32) void {
    if (error_flag == 0) {
        catch_ok_result = val;
    }
}

// ``catch_leave`` clears ``error_flag`` so surrounding (non-catch)
// code doesn't see a stale pending error.  But ``catch_result`` is
// called AFTER ``catch_leave`` to populate the result variable, so it
// needs its own snapshot of "did this catch fire?".  Without the
// snapshot, ``catch_result`` saw ``error_flag == 0`` and returned
// ``catch_ok_result`` (0) for every error-catching case, so
// ``catch {error boom} msg`` produced ``msg == ""``.
pub var last_catch_had_error: u32 = 0;

// Exported: leave a catch scope. Returns 0 (TCL_OK) or 1 (TCL_ERROR).
pub export fn catch_leave() i32 {
    if (catch_depth > 0) catch_depth -= 1;
    const had_error = error_flag;
    last_catch_had_error = had_error;
    error_flag = 0;
    return obj_new_int(@intCast(had_error));
}

// Exported: get the result (or error message) after catch.
// Returns the error message when an error occurred, or the body's
// last-command result (set by ``catch_set_ok_result``) on success.
pub export fn catch_result() i32 {
    if (last_catch_had_error != 0) return error_msg;
    return catch_ok_result;
}

// Exported: check if an error is pending (for early exit from catch body).
pub export fn catch_has_error() i32 {
    return @as(i32, @intCast(error_flag));
}

// Stamp the error-context globals that Tcl scripts inspect after a
// caught error: ``::errorInfo`` (traceback) and ``::errorCode``
// (error class).  We don't yet maintain a real traceback — the
// message itself is used as the info text, which matches the
// observable behaviour for scripts that do ``catch … msg; puts
// $::errorInfo`` without introspecting call frames.  ``::errorCode``
// defaults to ``NONE`` for ``error msg`` with no explicit code.
fn stamp_error_globals(msg: i32, info: i32, code: i32) void {
    const info_name = obj_new_string_copy(@intFromPtr("::errorInfo".ptr), 11);
    const info_val = if (info != 0) info else msg;
    _ = globals.global_set(info_name, info_val);

    const code_name = obj_new_string_copy(@intFromPtr("::errorCode".ptr), 11);
    const code_val = if (code != 0)
        code
    else
        obj_new_string_copy(@intFromPtr("NONE".ptr), 4);
    _ = globals.global_set(code_name, code_val);
}

// Exported: error — write message to stderr and trap, OR set error flag in catch.
//
// On an out-of-catch error we prefix the stderr line with
// ``tcl trap: site=<id> `` when the codegen has registered a site;
// a companion sidecar map resolves the site to a source location.
//
// ``::errorInfo`` and ``::errorCode`` are stamped on every error so
// ``catch { error boom } msg; puts $::errorInfo`` observes the
// message even though we don't construct a full traceback yet.
pub export fn tcl_cmd_error(msg: i32) void {
    stamp_error_globals(msg, 0, 0);
    if (catch_depth > 0) {
        error_flag = 1;
        error_msg = msg;
        return;
    }
    fd_write_all(2, "tcl trap: ", 10);
    _ = diag.write_prefix(2);
    const s = obj_ensure_string(msg);
    if (s.len > 0) {
        fd_write_all(2, @ptrFromInt(s.ptr), s.len);
    }
    fd_write_all(2, "\n", 1);
    diag.write_eval_ctx(2);
    @trap();
}

// Exported: error with explicit ``info`` / ``code`` arguments —
// matches the full 3-arg ``error`` command form
// (``error msg ?info? ?code?``).  Either or both extras may be
// ``0`` to use the defaults.
pub export fn tcl_cmd_error_full(msg: i32, info: i32, code: i32) void {
    stamp_error_globals(msg, info, code);
    if (catch_depth > 0) {
        error_flag = 1;
        error_msg = msg;
        return;
    }
    fd_write_all(2, "tcl trap: ", 10);
    _ = diag.write_prefix(2);
    const s = obj_ensure_string(msg);
    if (s.len > 0) {
        fd_write_all(2, @ptrFromInt(s.ptr), s.len);
    }
    fd_write_all(2, "\n", 1);
    diag.write_eval_ctx(2);
    @trap();
}

// Build a "unknown command: <name>" TclObj and route it through
// @"error".  Used by the interpreter fallback when a word doesn't
// match any builtin or registered proc.  Keeping the formatting here
// rather than in tcl_interp.zig avoids duplicating the obj-allocation
// dance and guarantees every "unknown command" trap looks the same.
pub fn error_unknown_command(cmd_obj: i32) void {
    const prefix: []const u8 = "unknown command: ";
    const s = obj_ensure_string(cmd_obj);
    const total: u32 = @intCast(prefix.len + s.len);
    // Allocate a fresh byte buffer in the bump allocator so the
    // TclObj's string data outlives this frame.
    const buf_addr: u32 = obj.alloc(total);
    const buf: [*]u8 = @ptrFromInt(buf_addr);
    for (prefix, 0..) |c, i| buf[i] = c;
    if (s.len > 0) {
        const src: [*]const u8 = @ptrFromInt(s.ptr);
        for (0..s.len) |i| buf[prefix.len + i] = src[i];
    }
    const msg = obj.obj_new_string(@intCast(buf_addr), @intCast(total));
    tcl_cmd_error(msg);
}
