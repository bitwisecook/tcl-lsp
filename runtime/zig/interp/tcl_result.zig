// Phase 2 of the var/frame/ns/exception architecture refactor:
// :type:`InterpResult` is a typed snapshot of the
// (return-value, control-flow, error-message) triple every Tcl
// command produces.  Replaces the scattered ``if (rt.error_flag.*
// != 0) ...`` chains in eval_script / loop bodies / catch / proc
// dispatch with a single ``switch (ir.code)``.
//
// The legacy globals (``tcl_catch.error_flag`` /
// ``tcl_catch.return_flag`` / ``tcl_catch.break_flag`` /
// ``tcl_catch.continue_flag`` / ``tcl_catch.error_msg`` /
// ``tcl_catch.return_val``) are still the storage medium —
// individual command handlers continue to set them on signal —
// but inspection happens through :func:`snapshot` so callers
// don't reach into the globals directly.
//
// Why keep the globals? Migrating every command handler's
// signature from ``fn(words) i32`` to ``fn(words) InterpResult``
// is a 50+ file mechanical change with real regression risk; the
// design doc records that as future work.  This split — typed
// inspection on top of byte-compatible storage — captures the
// architectural value of phase 2 (no module-internal flag-poking)
// without the migration cost.

const tcl_catch = @import("tcl_catch.zig");
const obj = @import("../valtypes/tcl_obj.zig");

/// The five Tcl return codes from ``Tcl_Eval``-family return values.
/// Bit-pattern compatible with ``TCL_OK`` / ``TCL_ERROR`` / etc.
pub const Code = enum(u8) {
    OK = 0,
    ERROR = 1,
    RETURN = 2,
    BREAK = 3,
    CONTINUE = 4,
};

/// Typed snapshot of a command (or eval_script) invocation's outcome.
/// One source of truth for every post-call inspection site — the
/// fields shadow the relevant ``tcl_catch`` globals at the moment
/// :func:`snapshot` ran.  Mutating the snapshot doesn't mutate the
/// globals; use :func:`consume` to clear a signal once it's been
/// handled.
pub const InterpResult = struct {
    /// Highest-priority signal observed in the globals.  ``ERROR``
    /// wins over ``RETURN`` wins over ``BREAK`` wins over
    /// ``CONTINUE``; ``OK`` only if every flag was clear.  This
    /// matches reference Tcl's per-command return-code precedence.
    code: Code,
    /// The command's (or eval_script's) primary value:
    ///   * ``OK``     — the result obj.
    ///   * ``RETURN`` — the return value (``return_val``).
    ///   * ``ERROR``  — the error message (``error_msg``).
    ///   * ``BREAK`` / ``CONTINUE`` — the result obj at signal time.
    value: i32,
    /// Extra-frames for ``return -level N``; copied from
    /// ``tcl_catch.return_level``.  Only meaningful when
    /// ``code == .RETURN``.
    return_level: u32,
};

/// Read the current control-flow flags into a typed snapshot.
/// ``returned_value`` is the i32 the underlying command (or
/// ``eval_script`` invocation) just produced — used to populate
/// ``value`` when no signal flag was set or when the signal is
/// ``BREAK`` / ``CONTINUE`` (which carry no payload of their own).
///
/// Does NOT clear the globals — call :func:`consume` after handling.
pub fn snapshot(returned_value: i32) InterpResult {
    if (tcl_catch.error_flag != 0) {
        return .{
            .code = .ERROR,
            .value = tcl_catch.error_msg,
            .return_level = 0,
        };
    }
    if (tcl_catch.return_flag != 0) {
        return .{
            .code = .RETURN,
            .value = tcl_catch.return_val,
            .return_level = tcl_catch.return_level,
        };
    }
    if (tcl_catch.break_flag != 0) {
        return .{
            .code = .BREAK,
            .value = returned_value,
            .return_level = 0,
        };
    }
    if (tcl_catch.continue_flag != 0) {
        return .{
            .code = .CONTINUE,
            .value = returned_value,
            .return_level = 0,
        };
    }
    return .{
        .code = .OK,
        .value = returned_value,
        .return_level = 0,
    };
}

/// Clear the signal flag that ``snapshot`` reported, leaving the
/// rest of the global state untouched.  Used by sites that *handled*
/// the signal (``while`` body absorbs ``BREAK``; ``catch`` absorbs
/// every code).  Never called for ``OK`` — that's a no-op anyway.
///
/// Note: ``ERROR`` consume does NOT release ``error_msg`` — the
/// caller is responsible for that (catch_leave's last_catch_value
/// retain dance, etc.) since the caller usually wants to forward
/// the message into a result var first.
pub fn consume(code: Code) void {
    switch (code) {
        .OK => {},
        .ERROR => {
            tcl_catch.error_flag = 0;
            // Caller owns the message — don't clear it here, but do
            // reset the log-tracker so a subsequent fresh error gets
            // a clean ``while executing`` traceback.
            tcl_catch.last_log_script = 0;
            tcl_catch.last_log_pos = 0;
        },
        .RETURN => {
            tcl_catch.return_flag = 0;
            // Same ownership story — caller forwards return_val
            // before this clear.
            tcl_catch.return_val = 0;
            tcl_catch.return_level = 0;
        },
        .BREAK => {
            tcl_catch.break_flag = 0;
            tcl_catch.signal_break_flag = 0;
        },
        .CONTINUE => {
            tcl_catch.continue_flag = 0;
            tcl_catch.signal_continue_flag = 0;
        },
    }
}

/// Re-arm a previously-consumed signal so the next layer up
/// observes it.  Mirror of :func:`consume`.  Used when a catch
/// inspects the result via snapshot but decides to propagate (e.g.
/// the body raised ``ERROR`` and the handler doesn't match).
pub fn restore(ir: InterpResult) void {
    switch (ir.code) {
        .OK => {},
        .ERROR => {
            tcl_catch.error_flag = 1;
            tcl_catch.error_msg = ir.value;
        },
        .RETURN => {
            tcl_catch.return_flag = 1;
            tcl_catch.return_val = ir.value;
            tcl_catch.return_level = ir.return_level;
        },
        .BREAK => {
            tcl_catch.break_flag = 1;
        },
        .CONTINUE => {
            tcl_catch.continue_flag = 1;
        },
    }
}

/// Convenience: "did this snapshot carry any non-OK signal?"
/// One-liner for the eval_script command-loop short-circuit.
pub inline fn has_signal(ir: InterpResult) bool {
    return ir.code != .OK;
}
