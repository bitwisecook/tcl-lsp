// Error handling: ``catch`` scope management + ``@"error"`` trap /
// catch-flag entry point.  Previously this file also carried silent
// stubs for ``format`` / ``regexp`` / ``open`` / ``close`` / ``read``
// / ``gets``; those have moved to area-specific stub files
// (``tcl_io_stubs.zig``, ``tcl_fmt_stubs.zig``) and now raise
// ``unsupported command: <name>`` through :func:`tcl_stubs.unsupported`.

const obj = @import("../valtypes/tcl_obj.zig");
const dict_mod = @import("../valtypes/tcl_dict.zig");
const io = @import("../io/tcl_io.zig");
const diag = @import("../dispatch/tcl_diag.zig");
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
/// Coroutine ``yield`` signal.  Set by ``yield`` / ``yieldto`` to
/// unwind the eval stack back to the enclosing coroutine driver
/// (``sched/tcl_coro.zig::resume_one``).  Distinct from
/// ``return_flag`` so ``apply`` / proc dispatch don't silently
/// absorb it.
pub var yield_flag: u32 = 0;
pub var yield_value: i32 = 0;

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

// Exported: signal a TCL_RETURN unwind from the compiled catch body.
// Mirrors the runtime ``return`` command: stash the value, set the
// flag, and let ``catch_has_error`` / ``catch_leave`` see it.  The
// codegen calls this for ``return ?value?`` inside a ``catch`` body
// so the WASM ``return`` instruction (which would jump past
// ``catch_leave``) is replaced by a flag-driven unwind.  Issue #..:
// fixes ``catch {return foo}`` exiting the surrounding proc.
pub export fn tcl_return_set(value: i32) void {
    return_flag = 1;
    const old = return_val;
    if (value != 0) obj.tcl_obj_retain(value);
    return_val = value;
    if (old != 0 and old != value) obj.tcl_obj_release(old);
}

/// Read-and-clear the pending break flag.  Returns 1 if a break was
/// pending (and clears the flag) so a compiled loop body can br out
/// after an eval-fallback that ran ``break`` inside an interpreter
/// script (e.g. a body passed to ``dict update`` / ``foreach`` /
/// ``catch`` that lives behind an eval-fallback).  Without this hook,
/// the wasm-side loop never noticed the break and kept iterating.
pub export fn flow_consume_break() i32 {
    if (break_flag != 0) {
        break_flag = 0;
        return 1;
    }
    return 0;
}

/// Read-and-clear the pending continue flag — same shape as
/// :func:`flow_consume_break`.  ``continue`` from inside an
/// eval-fallback still leaves the loop body before the
/// continue-block end label, so the compiled loop can simply
/// fall through to the loop-restart ``br`` that follows.
pub export fn flow_consume_continue() i32 {
    if (continue_flag != 0) {
        continue_flag = 0;
        return 1;
    }
    return 0;
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

// Tcl return-code constants — mirrors ``generic/tcl.h``.
const TCL_OK: i64 = 0;
const TCL_ERROR: i64 = 1;
const TCL_RETURN: i64 = 2;
const TCL_BREAK: i64 = 3;
const TCL_CONTINUE: i64 = 4;

// Snapshot of the captured return code from the most recent
// ``catch_leave`` call.  ``catch_options`` reads this when building
// the ``-code`` field of the options dict so a 3-arg ``catch`` sees
// the same code that ``catch`` itself returned (TCL_BREAK / TCL_CONTINUE
// for unwound flow control, not TCL_OK / TCL_ERROR like the legacy
// boolean-only path).
pub var last_catch_code: i64 = 0;

// Exported: leave a catch scope.  Returns the TCL_* return code
// produced by the body — TCL_OK (0), TCL_ERROR (1), TCL_RETURN (2),
// TCL_BREAK (3), or TCL_CONTINUE (4) — matching reference Tcl's
// ``Tcl_CatchObjCmd``.  All four flow-control flags are absorbed
// here so the surrounding (non-catch) compiled code doesn't see
// them and the loop / proc dispatcher above us doesn't double-fire.
pub export fn catch_leave() i32 {
    if (catch_depth > 0) catch_depth -= 1;
    const code: i64 = blk: {
        if (error_flag != 0) break :blk TCL_ERROR;
        if (return_flag != 0) break :blk TCL_RETURN;
        if (break_flag != 0) break :blk TCL_BREAK;
        if (continue_flag != 0) break :blk TCL_CONTINUE;
        break :blk TCL_OK;
    };
    last_catch_had_error = if (error_flag != 0) 1 else 0;
    last_catch_code = code;
    // Absorb every flow-control flag — ``catch`` is the universal sink
    // for non-OK return codes from its body.  Surrounding loops
    // (``flow_consume_break``) and proc dispatchers (``return_flag``)
    // would otherwise re-fire on a code we already converted into a
    // return value.
    error_flag = 0;
    return_flag = 0;
    break_flag = 0;
    continue_flag = 0;
    return obj_new_int(code);
}

// Exported: get the result (or error message) after catch.
// Returns the error message when an error occurred, or the body's
// last-command result (set by ``catch_set_ok_result``) on success.
//
// Issue #280: when a successful catch body's last command returns
// the null TclObj (commands that silently produce no value, e.g.
// ``dict`` with no args under our incomplete arity-check, or
// arity-shy stubs that just ``return 0``), ``catch_result`` used to
// hand a literal 0 to the caller's writeback.  The caller then
// stored 0 into the result-var local slot — at which point any
// subsequent ``$resultVar`` read mistook the 0 for an unset slot
// and trapped with ``can't read "<resultVar>": no such variable``.
// In reference Tcl, ``catch {body} v`` always assigns *some* string
// to ``v`` on success (the body's empty result is the empty string,
// not unset).  Materialise an empty TclObj here so the writeback
// path never plants the unset sentinel.
pub export fn catch_result() i32 {
    if (last_catch_had_error != 0) return error_msg;
    if (catch_ok_result == 0) return obj_new_string(0, 0);
    return catch_ok_result;
}

// Exported: build a Tcl options dict for the most recent ``catch``.
// Compiled 3-arg ``catch BODY result opt`` calls this after
// ``catch_leave`` to populate ``$opt`` with the standard
// ``-code`` / ``-level`` / ``-errorcode`` / ``-errorinfo`` keys.
//
// The dict mirrors ``tclResult.c::Tcl_GetReturnOptions``:
//
//   * ``-code`` — TCL return code (0 = OK, 1 = ERROR; we don't
//     surface RETURN / BREAK / CONTINUE / 5+).
//   * ``-level`` — 0 (always; we don't track ``return -level``).
//   * ``-errorcode`` — last ``::errorCode`` global (defaults to
//     ``NONE`` when no error occurred or no explicit code was
//     given).
//   * ``-errorinfo`` — last ``::errorInfo`` global.
//
// Caller assumes ownership of the returned TclObj.
//
// Memory discipline: ``dict_set`` reads the key's string bytes (copied
// into the dict's list rep) and retains the value via the hash side-
// cache.  Each ``dict_set_str`` call therefore allocates one temporary
// key obj + one (often temporary) value obj that the dict no longer
// needs after the rebuild — both have to be released or they leak per
// call.  The intermediate ``d`` from each ``dict_set`` is also released
// before reassigning, since ``dict_set`` returns a fresh dict for any
// key-modifying path (Copilot review).
pub export fn catch_options() i32 {
    var d: i32 = dict_mod.dict_create();
    d = dict_set_str_take(d, "-code", obj_new_int(last_catch_code));
    d = dict_set_str_take(d, "-level", obj_new_int(0));
    const ec_name = obj_new_string_copy(@intFromPtr("::errorCode".ptr), 11);
    const ec_val = globals.global_get(ec_name);
    obj.tcl_obj_release(ec_name);
    if (ec_val != 0) {
        d = dict_set_str_keep(d, "-errorcode", ec_val);
    } else {
        d = dict_set_str_take(d, "-errorcode", obj_new_string_copy(
            @intFromPtr("NONE".ptr), 4,
        ));
    }
    const ei_name = obj_new_string_copy(@intFromPtr("::errorInfo".ptr), 11);
    const ei_val = globals.global_get(ei_name);
    obj.tcl_obj_release(ei_name);
    if (ei_val != 0) {
        d = dict_set_str_keep(d, "-errorinfo", ei_val);
    } else if (last_catch_had_error != 0 and error_msg != 0) {
        d = dict_set_str_keep(d, "-errorinfo", error_msg);
    }
    return d;
}

/// ``dict set`` with a string-literal key, taking *ownership* of the
/// caller's *value* refcount.  Allocates a temporary key obj, calls
/// ``dict_set``, then releases the key and the source value.
/// Returns the new dict (caller still owns one ref).
fn dict_set_str_take(d: i32, key: []const u8, value: i32) i32 {
    const k = obj_new_string_copy(@intCast(@intFromPtr(key.ptr)), @intCast(key.len));
    const new = dict_mod.dict_set(d, k, value);
    obj.tcl_obj_release(k);
    obj.tcl_obj_release(value);
    if (new != d) obj.tcl_obj_release(d);
    return new;
}

/// Like :func:`dict_set_str_take` but the *value* is borrowed from
/// somewhere else (e.g. ``::errorCode`` global) — we don't release
/// the caller's refcount on it.
fn dict_set_str_keep(d: i32, key: []const u8, value: i32) i32 {
    const k = obj_new_string_copy(@intCast(@intFromPtr(key.ptr)), @intCast(key.len));
    const new = dict_mod.dict_set(d, k, value);
    obj.tcl_obj_release(k);
    if (new != d) obj.tcl_obj_release(d);
    return new;
}

// Exported: check if any flow-control signal is pending — error,
// break, continue, or return.  Catch-body codegen calls this between
// statements to short-circuit the remainder of the body when the
// previous statement raised an error or unwound a non-OK return code.
// Returns 1 when any flag is set, 0 otherwise.  Reference Tcl's
// ``Tcl_CatchObjCmd`` likewise stops at the first non-TCL_OK code.
pub export fn catch_has_error() i32 {
    if (error_flag != 0) return 1;
    if (return_flag != 0) return 1;
    if (break_flag != 0) return 1;
    if (continue_flag != 0) return 1;
    return 0;
}

// Stamp the error-context globals that Tcl scripts inspect after a
// caught error: ``::errorInfo`` (traceback) and ``::errorCode``
// (error class).  We don't yet maintain a real traceback — the
// message itself is used as the info text, which matches the
// observable behaviour for scripts that do ``catch … msg; puts
// $::errorInfo`` without introspecting call frames.  ``::errorCode``
// defaults to ``NONE`` for ``error msg`` with no explicit code.
fn stamp_error_globals(msg: i32, info: i32, code: i32) void {
    // Issue #317: ``global_set`` reads the name's byte span (the
    // var subsystem stores its own copy) and retains the value
    // for the slot.  Without these releases, every error event
    // leaked one TclObj per name (``::errorInfo`` / ``::errorCode``)
    // plus, on the default-code path, one ``NONE`` value TclObj —
    // four fresh allocations per error.  tcltest exercises the
    // catched-error path many times per test, so this scaled
    // linearly with the suite size.
    const info_name = obj_new_string_copy(@intFromPtr("::errorInfo".ptr), 11);
    const info_val = if (info != 0) info else msg;
    _ = globals.global_set(info_name, info_val);
    obj.tcl_obj_release(info_name);

    const code_name = obj_new_string_copy(@intFromPtr("::errorCode".ptr), 11);
    const default_code = code == 0;
    const code_val = if (default_code)
        obj_new_string_copy(@intFromPtr("NONE".ptr), 4)
    else
        code;
    _ = globals.global_set(code_name, code_val);
    obj.tcl_obj_release(code_name);
    if (default_code) obj.tcl_obj_release(code_val);
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
    // Issue #317: ``obj_new_string_take`` so the error TclObj
    // owns ``buf_addr`` and ``release_now`` returns it via
    // ``free_sized``; the older borrowing form leaked one buf per
    // raised error inside a ``catch``.
    const msg = obj.obj_new_string_take(buf_addr, total, total);
    tcl_cmd_error(msg);
}

// Build a ``can't read "<name>": no such variable`` TclObj and route
// it through :func:`tcl_cmd_error`.  Used by the var-read paths
// (``local_get_or_error``, ``global_get_or_error``,
// ``var_unset_error``) when a ``$x`` substitution / ``set x`` /
// ``expr {$x}`` references a variable that has never been set in the
// current scope.  The wording matches reference Tcl exactly so
// existing regression tests can grep the substring.
pub export fn var_unset_error(name_obj: i32) void {
    const prefix: []const u8 = "can't read \"";
    const suffix: []const u8 = "\": no such variable";
    const s = obj_ensure_string(name_obj);
    const total: u32 = @intCast(prefix.len + s.len + suffix.len);
    const buf_addr: u32 = obj.alloc(total);
    const buf: [*]u8 = @ptrFromInt(buf_addr);
    var off: usize = 0;
    for (prefix) |c| {
        buf[off] = c;
        off += 1;
    }
    if (s.len > 0) {
        const src: [*]const u8 = @ptrFromInt(s.ptr);
        for (0..s.len) |i| {
            buf[off] = src[i];
            off += 1;
        }
    }
    for (suffix) |c| {
        buf[off] = c;
        off += 1;
    }
    // Issue #317: see ``error_unknown_command`` above.
    const msg = obj.obj_new_string_take(buf_addr, total, total);
    tcl_cmd_error(msg);
}
