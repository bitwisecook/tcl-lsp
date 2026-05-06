// Phase 2 of the var/frame/ns/exception architecture refactor:
// :type:`InterpResult` is a typed snapshot of the
// (return-value, control-flow, error-message) triple every Tcl
// command produces.  Replaces the scattered ``if (rt.error_flag.*
// != 0) ...`` chains in eval_script / loop bodies / catch / proc
// dispatch with a single ``switch (ir.code)``.
//
// Sub-phase 2a — typed inspection — installed
// :func:`snapshot` / :func:`consume` / :func:`restore` so callers
// READ the flags through this module instead of poking the globals.
//
// Sub-phase 2b/2c — typed mutation — installed :func:`set_break`,
// :func:`set_continue`, :func:`set_return`, :func:`set_signal_break`,
// :func:`set_signal_continue`, plus :func:`signal_save_and_clear` /
// :func:`signal_restore` and :func:`break_continue_save_and_clear` /
// :func:`break_continue_restore` for the save/restore patterns
// (try/finally, scheduler-isolated eval, proc-body break/continue
// preservation).  Every WRITE outside ``tcl_catch.zig`` (which owns
// the storage) routes through one of these setters — the
// architectural goal of "no module-internal flag-poking" now holds
// in both directions.
//
// The legacy globals (``tcl_catch.{error,return,break,continue}_flag``
// + ``error_msg`` / ``return_val`` / ``return_level`` /
// ``signal_*_flag``) remain as the storage medium, but they're now
// fully encapsulated: only ``tcl_catch.zig`` (which IS the storage)
// and this module touch them directly.  The design doc records the
// next-phase goal of moving the storage onto the active frame's
// ``pending_result`` field — that's a follow-up that's now safe to
// land without touching any caller, since this module is the only
// abstraction layer.

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
    /// ``tcl_catch.state.return_level``.  Only meaningful when
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
    if (tcl_catch.state.error_flag != 0) {
        return .{
            .code = .ERROR,
            .value = tcl_catch.state.error_msg,
            .return_level = 0,
        };
    }
    if (tcl_catch.state.return_flag != 0) {
        return .{
            .code = .RETURN,
            .value = tcl_catch.state.return_val,
            .return_level = tcl_catch.state.return_level,
        };
    }
    if (tcl_catch.state.break_flag != 0) {
        return .{
            .code = .BREAK,
            .value = returned_value,
            .return_level = 0,
        };
    }
    if (tcl_catch.state.continue_flag != 0) {
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
            tcl_catch.state.error_flag = 0;
            // Caller owns the message — don't clear it here, but do
            // reset the log-tracker so a subsequent fresh error gets
            // a clean ``while executing`` traceback.
            tcl_catch.state.last_log_script = 0;
            tcl_catch.state.last_log_pos = 0;
        },
        .RETURN => {
            tcl_catch.state.return_flag = 0;
            // Same ownership story — caller forwards return_val
            // before this clear.
            tcl_catch.state.return_val = 0;
            tcl_catch.state.return_level = 0;
            tcl_catch.state.return_code = 0;
        },
        .BREAK => {
            tcl_catch.state.break_flag = 0;
            tcl_catch.state.signal_break_flag = 0;
        },
        .CONTINUE => {
            tcl_catch.state.continue_flag = 0;
            tcl_catch.state.signal_continue_flag = 0;
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
            tcl_catch.state.error_flag = 1;
            tcl_catch.state.error_msg = ir.value;
        },
        .RETURN => {
            tcl_catch.state.return_flag = 1;
            tcl_catch.state.return_val = ir.value;
            tcl_catch.state.return_level = ir.return_level;
        },
        .BREAK => {
            tcl_catch.state.break_flag = 1;
        },
        .CONTINUE => {
            tcl_catch.state.continue_flag = 1;
        },
    }
}

/// Convenience: "did this snapshot carry any non-OK signal?"
/// One-liner for the eval_script command-loop short-circuit.
pub inline fn has_signal(ir: InterpResult) bool {
    return ir.code != .OK;
}

// --- Handler return-value constructors (Phase 2b/2c) -------------------
//
// Every migrated ``HandlerFn`` returns an ``InterpResult`` by ending
// with one of these constructors.  ``from_globals`` is the bridge for
// handlers that internally call ``tcl_cmd_error`` / ``set_break`` /
// etc. (which still write the globals as transient storage) — it
// snapshots the post-mutation state into a typed result so the
// dispatcher doesn't need to peek at the globals separately.  The
// explicit ``ok`` / ``err`` / ``ret`` / ``brk`` / ``cont``
// constructors are for handlers that don't touch the globals at all
// and want to surface the outcome directly.

/// Wrap the i32 result of a handler that may have written through the
/// legacy globals (``tcl_cmd_error`` / ``set_break`` / ``set_return``
/// / ``set_continue``) into a typed ``InterpResult``.  Identical to
/// :func:`snapshot` — exists as a named alias so handler return
/// statements read intent (``return result_mod.from_globals(v)``)
/// rather than dispatch-loop intent (``snapshot(v)``).
pub inline fn from_globals(value: i32) InterpResult {
    return snapshot(value);
}

/// Build an OK ``InterpResult`` carrying ``value`` as the command
/// result.  Use when the handler doesn't touch any flag state.
pub inline fn ok(value: i32) InterpResult {
    return .{ .code = .OK, .value = value, .return_level = 0 };
}

/// Build an ERROR ``InterpResult`` carrying ``msg`` as the error
/// message.  Note: this does NOT call ``tcl_cmd_error`` — it does
/// not stamp ``::errorInfo`` / ``::errorCode`` and it does not
/// write to stderr / @trap when out of a catch scope.  Most
/// handlers continue to call ``tcl_cmd_error`` for the side
/// effects and end with :func:`from_globals` to capture the
/// resulting state.  Use this constructor only when you've
/// already arranged for the side effects elsewhere.
pub inline fn err(msg: i32) InterpResult {
    return .{ .code = .ERROR, .value = msg, .return_level = 0 };
}

/// Build a RETURN ``InterpResult`` carrying ``value`` and the
/// optional ``return -level N`` extra-frame counter.
pub inline fn ret(value: i32, level: u32) InterpResult {
    return .{ .code = .RETURN, .value = value, .return_level = level };
}

/// Build a BREAK ``InterpResult``.
pub inline fn brk() InterpResult {
    return .{ .code = .BREAK, .value = 0, .return_level = 0 };
}

/// Build a CONTINUE ``InterpResult``.
pub inline fn cont() InterpResult {
    return .{ .code = .CONTINUE, .value = 0, .return_level = 0 };
}

// --- Typed setters (Phase 2b/2c) ---------------------------------------
//
// The legacy ``tcl_catch.{error,return,break,continue}_flag`` globals
// remain as the storage medium, but this module is the single
// abstraction layer.  Every WRITER outside ``tcl_catch.zig`` itself
// (which owns the storage) goes through one of the setters below
// instead of poking the flags directly.  After this layer is in
// place we can change the storage backing without touching any
// caller — e.g. moving the bookkeeping onto a per-frame
// ``pending_result: InterpResult`` field per the design doc.

/// Raise a ``BREAK`` signal (semantic equivalent of running ``break``
/// outside a loop body).  Used by:
///   * compiled-proc dispatch when a ``return -code break`` survives
///     the proc boundary and needs to surface to the caller's loop;
///   * ``dict update`` / ``dict with`` re-raising a body's break;
///   * any site that absorbed a break and decides to propagate.
pub fn set_break() void {
    tcl_catch.state.break_flag = 1;
}

/// Raise a ``CONTINUE`` signal — counterpart of :func:`set_break`.
pub fn set_continue() void {
    tcl_catch.state.continue_flag = 1;
}

/// Raise a ``RETURN`` signal with payload ``value`` and the optional
/// ``return -level N`` extra-frame counter.  Used by ``tailcall`` and
/// the codegen-side ``tcl_return_set`` (which still lives in
/// tcl_catch.zig because it has special retain bookkeeping for the
/// payload obj).
pub fn set_return(value: i32, level: u32) void {
    tcl_catch.state.return_flag = 1;
    tcl_catch.state.return_val = value;
    tcl_catch.state.return_level = level;
}

/// Raise the ``signal_break_flag`` side-channel — paired with the
/// ordinary ``break_flag`` to mark a *signalled* break (``return
/// -code break``) that should surface to the caller's enclosing loop
/// rather than triggering the "invoked break outside of a loop"
/// diagnostic at the proc boundary.
pub fn set_signal_break() void {
    tcl_catch.state.signal_break_flag = 1;
}

/// Counterpart of :func:`set_signal_break` for ``return -code continue``.
pub fn set_signal_continue() void {
    tcl_catch.state.signal_continue_flag = 1;
}

/// All four signal flags collected for save/restore around an
/// isolated eval (e.g. ``try``'s finally clause, the scheduler's
/// background-script invocation, the proc-body break/continue
/// preservation in ``eval_apply`` / ``eval_proc_call_bucket``).
/// ``error_msg`` / ``return_val`` / ``yield_value`` are also
/// captured because the wrapped eval may overwrite them and the
/// caller needs the original payload back on restore — without
/// this, a payload obj would either leak or get released twice.
pub const SignalSnapshot = struct {
    error_flag: u32,
    error_msg: i32,
    return_flag: u32,
    return_val: i32,
    return_level: u32,
    break_flag: u32,
    continue_flag: u32,
    signal_break_flag: u32,
    signal_continue_flag: u32,
    yield_flag: u32,
    yield_value: i32,
};

/// Capture every control-flow flag and payload into a struct, then
/// clear the live state so the wrapped eval starts in a clean OK
/// scope.  Mirror of :func:`signal_restore`.  Used by the scheduler
/// when running a background script (no flow signal from the
/// scheduled body should leak to the timer driver).
pub fn signal_save_and_clear() SignalSnapshot {
    const snap = SignalSnapshot{
        .error_flag = tcl_catch.state.error_flag,
        .error_msg = tcl_catch.state.error_msg,
        .return_flag = tcl_catch.state.return_flag,
        .return_val = tcl_catch.state.return_val,
        .return_level = tcl_catch.state.return_level,
        .break_flag = tcl_catch.state.break_flag,
        .continue_flag = tcl_catch.state.continue_flag,
        .signal_break_flag = tcl_catch.state.signal_break_flag,
        .signal_continue_flag = tcl_catch.state.signal_continue_flag,
        .yield_flag = tcl_catch.state.yield_flag,
        .yield_value = tcl_catch.state.yield_value,
    };
    tcl_catch.state.error_flag = 0;
    tcl_catch.state.error_msg = 0;
    tcl_catch.state.return_flag = 0;
    tcl_catch.state.return_val = 0;
    tcl_catch.state.return_level = 0;
    tcl_catch.state.break_flag = 0;
    tcl_catch.state.continue_flag = 0;
    tcl_catch.state.signal_break_flag = 0;
    tcl_catch.state.signal_continue_flag = 0;
    tcl_catch.state.yield_flag = 0;
    tcl_catch.state.yield_value = 0;
    return snap;
}

/// Counterpart of :func:`signal_save_and_clear`: restore every flag
/// and payload from the captured snapshot.  Caller is responsible
/// for any pending obj-release on intermediate values; this just
/// rewrites the slots verbatim.
pub fn signal_restore(snap: SignalSnapshot) void {
    tcl_catch.state.error_flag = snap.error_flag;
    tcl_catch.state.error_msg = snap.error_msg;
    tcl_catch.state.return_flag = snap.return_flag;
    tcl_catch.state.return_val = snap.return_val;
    tcl_catch.state.return_level = snap.return_level;
    tcl_catch.state.break_flag = snap.break_flag;
    tcl_catch.state.continue_flag = snap.continue_flag;
    tcl_catch.state.signal_break_flag = snap.signal_break_flag;
    tcl_catch.state.signal_continue_flag = snap.signal_continue_flag;
    tcl_catch.state.yield_flag = snap.yield_flag;
    tcl_catch.state.yield_value = snap.yield_value;
}

/// Lighter-weight save/restore for break/continue only — used by the
/// proc-body absorption path which only needs to preserve the
/// caller's enclosing loop signal (not error/return).
pub const BreakContinueSnapshot = struct {
    break_flag: u32,
    continue_flag: u32,
};

pub fn break_continue_save_and_clear() BreakContinueSnapshot {
    const snap = BreakContinueSnapshot{
        .break_flag = tcl_catch.state.break_flag,
        .continue_flag = tcl_catch.state.continue_flag,
    };
    tcl_catch.state.break_flag = 0;
    tcl_catch.state.continue_flag = 0;
    return snap;
}

pub fn break_continue_restore(snap: BreakContinueSnapshot) void {
    tcl_catch.state.break_flag = snap.break_flag;
    tcl_catch.state.continue_flag = snap.continue_flag;
}

/// Three-field save/restore for return + break + continue — used by
/// ``try``'s finally clause to preserve a handler's return/break/
/// continue signal across the finally body without touching error or
/// yield state.  ``error_flag`` is already 0 at the call site
/// (post-handler-match) and ``yield_flag`` is owned by the coroutine
/// driver, so neither participates here.
pub const FlowSnapshot = struct {
    return_flag: u32,
    return_val: i32,
    return_level: u32,
    break_flag: u32,
    continue_flag: u32,
};

pub fn flow_save_and_clear() FlowSnapshot {
    const snap = FlowSnapshot{
        .return_flag = tcl_catch.state.return_flag,
        .return_val = tcl_catch.state.return_val,
        .return_level = tcl_catch.state.return_level,
        .break_flag = tcl_catch.state.break_flag,
        .continue_flag = tcl_catch.state.continue_flag,
    };
    tcl_catch.state.return_flag = 0;
    tcl_catch.state.return_val = 0;
    tcl_catch.state.return_level = 0;
    tcl_catch.state.break_flag = 0;
    tcl_catch.state.continue_flag = 0;
    return snap;
}

pub fn flow_restore(snap: FlowSnapshot) void {
    tcl_catch.state.return_flag = snap.return_flag;
    tcl_catch.state.return_val = snap.return_val;
    tcl_catch.state.return_level = snap.return_level;
    tcl_catch.state.break_flag = snap.break_flag;
    tcl_catch.state.continue_flag = snap.continue_flag;
}

/// Read-and-clear the ``signal_break_flag`` / ``signal_continue_flag``
/// side-channels in one go.  Used by the proc-dispatch boundary to
/// detect a body-raised ``return -code break/continue`` that the
/// caller's enclosing loop should handle.  Returns ``(was_break,
/// was_continue)``.
pub fn take_signal_break_continue() struct { was_break: bool, was_continue: bool } {
    const wb = tcl_catch.state.signal_break_flag != 0;
    const wc = tcl_catch.state.signal_continue_flag != 0;
    tcl_catch.state.signal_break_flag = 0;
    tcl_catch.state.signal_continue_flag = 0;
    return .{ .was_break = wb, .was_continue = wc };
}
