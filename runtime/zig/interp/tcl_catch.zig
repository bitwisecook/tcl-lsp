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

// Phase 2 (final): control-flow signal storage is now collapsed
// into a single ``State`` struct.  Previously every flag /
// payload was a loose ``pub var``; that worked but spread the
// "pending interpreter result" across ~14 module-level globals
// nobody could relocate together.  Now a single ``state: State``
// holds the lot, and the legacy field names are kept inside the
// struct so existing accesses that go through the typed mutation
// API (``result_mod.set_break`` / ``snapshot`` / etc.) keep
// working unchanged.
//
// Why this matters: the design doc's phase-3+ goal — "globals
// become read-through views over the active frame's pending
// result" — is now a one-line change.  Replace ``state`` with
// ``current_frame().pending`` in the State struct's accessor
// (or wrap the field through a getter) and every caller observes
// per-frame state without further migration.  We deliberately
// keep ``state`` global today because that's the existing
// runtime contract; the relocation is tracked as a follow-up.

/// Aggregated control-flow / payload state for the currently
/// executing command chain.  Every field carries the same
/// semantics it did as a loose global; the struct just gives
/// them one home.
pub const State = struct {
    catch_depth: u32 = 0,
    /// 0 = no error, 1 = error pending.  Companion to ``error_msg``.
    error_flag: u32 = 0,
    /// TclObj handle for the pending error message (only meaningful
    /// when ``error_flag != 0``).
    error_msg: i32 = 0,
    /// Last ``(script_ptr, cmd_start)`` pair that ``log_command_info``
    /// stamped onto ``::errorInfo`` for the current error event.
    /// The next stamp dedupes against this and uses
    /// ``invoked from within`` instead of ``while executing``.
    last_log_script: u32 = 0,
    last_log_pos: u32 = 0,
    /// Non-zero when ``error msg info ?code?`` supplied an explicit
    /// ``info`` argument.  Reference Tcl's ``Tcl_ErrorObjCmd`` sets
    /// ``iPtr->flags |= ERR_LEGACY_COPY`` in that case, which
    /// ``Tcl_LogCommandInfo`` reads to skip its "first increment"
    /// (the immediate ``error`` callsite frame the user already
    /// captured in *info*).  Without the suppression
    /// ``$::errorInfo`` after ``error msg1 msg2 msg3`` reports the
    /// auto-traceback ``msg1\n    while executing\n"error ..."``
    /// instead of the verbatim ``msg2`` reference Tcl expects
    /// (error-4.1 / -4.2 / -4.5 / -5.1).  Cleared by every
    /// ``catch_leave`` / ``catch_enter`` and after the first
    /// ``log_command_info`` skip.
    error_info_supplied: u32 = 0,
    /// Non-zero when the most recent command-handler return is from
    /// a *transparent* command — ``while`` / ``for`` / ``foreach``
    /// in their bytecode-compiled C-Tcl equivalents — and the outer
    /// :func:`log_command_info` should NOT add an ``invoked from
    /// within "while ..."`` frame for this command.  The body's own
    /// commands have already populated ``::errorInfo`` with the
    /// proper context.  Mirrors Tcl 9's bytecode behaviour where
    /// the loop command is inlined and never appears in the
    /// traceback.  Consumed (cleared) by the first
    /// :func:`log_command_info` call after the handler returns.
    transparent_error: u32 = 0,
    /// The 1-based line number of the most recent command to raise
    /// an error, captured at error time (before the proc body unwinds
    /// or the frame pops).  Used by ``eval_proc_call_bucket`` to
    /// stamp the ``(procedure "X" line N)`` errorInfo frame after a
    /// proc body raises.  Set by :func:`tcl_cmd_error` /
    /// :func:`tcl_cmd_error_full` from the current frame's tracked
    /// line.  Cleared on every ``catch_leave`` so a subsequent error
    /// doesn't reuse the previous error's line.
    error_frame_line: u32 = 0,
    /// 1 = the most-recent error was raised via ``return -code error``
    /// (or ``return -options {-code error ...}``), NOT a direct
    /// ``error`` / ``throw`` call.  Tcl 9's ``InterpProcNR2`` skips
    /// ``MakeProcError`` for ``return -code error`` because the
    /// proc body returned via ``TclUpdateReturnInfo``-converted
    /// TCL_RETURN rather than a body-level TCL_ERROR — proc-old-7.2
    /// expects no ``(procedure "tproc" line N)`` frame in this path.
    /// :func:`proc_stamp_error_frame` and
    /// :func:`proc_dispatch_stamp_error_frame` consult the flag and
    /// skip the procedure-frame stamp when set, then clear the slot.
    return_via_error_code: u32 = 0,
    /// Pending arbitrary ``-OPT VALUE`` pairs supplied to ``return``
    /// (cmdMZ-return-2.1: ``return -bar soom``).  Encoded as a
    /// pre-built dict TclObj.  Snapshotted into
    /// :attr:`last_return_extras` by :func:`catch_leave` so the
    /// surrounding catch's :func:`catch_options` can merge them
    /// into its output dict.
    pending_return_extras: i32 = 0,
    /// Tcl 9 ``-during`` options-dict slot for ``try ... finally ...``
    /// exception chaining.  When a ``try`` body (or matched handler)
    /// completes and its options dict is captured here just before
    /// the ``finally`` body runs, an error raised by the finally body
    /// propagates as the catch's effective error AND its options dict
    /// gains a ``-during <captured>`` entry pointing at the original
    /// completion's full options dict (error-16.21 / -18.7 etc.).
    /// :func:`catch_options` consumes it into the output dict; the
    /// slot's lifecycle is owned by ``eval_try`` which snapshots a
    /// retained handle here and clears it again on the way out.
    during_options: i32 = 0,
    /// Snapshot of :attr:`pending_return_extras` at catch_leave
    /// time.  :func:`catch_options` reads this to populate the
    /// options dict with the user-supplied ``-OPT VALUE`` pairs.
    /// Cleared on each catch_leave after a fresh snapshot replaces
    /// the previous capture.
    last_return_extras: i32 = 0,
    /// 1 = return pending (absorbed by proc dispatch).
    return_flag: u32 = 0,
    /// TclObj return value paired with ``return_flag``.
    return_val: i32 = 0,
    /// Extra proc levels for ``return -level N`` propagation —
    /// see :func:`tcl_return_set` for the wire-up.
    return_level: u32 = 0,
    /// ``return``'s requested ``-level`` argument verbatim (no
    /// "minus one" transform).  Default ``1`` for plain ``return``;
    /// ``return -level 0`` sets it to ``0``; ``return -level N``
    /// sets it to ``N``.  Distinct from :attr:`return_level` (which
    /// counts *extra* unwind levels) so the TIP 90 options-dict
    /// ``-level`` slot can report the verbatim user-supplied value
    /// after :func:`catch_leave` has cleared the extra-level
    /// counter.  Set by :func:`eval_return`, snapshotted by
    /// :func:`catch_leave` into :attr:`last_return_level`.
    pending_return_level: u32 = 1,
    /// ``return``'s requested ``-code`` argument as a numeric Tcl
    /// status (0=OK, 1=ERROR, 2=RETURN, 3=BREAK, 4=CONTINUE, N>=5
    /// custom).  Default ``0`` for plain ``return``.  Set by
    /// :func:`eval_return`, snapshotted by :func:`catch_leave`
    /// into :attr:`last_return_code` so the TIP 90 options dict
    /// reports the user-supplied code rather than the catch's
    /// effective code.
    pending_return_code: i64 = 0,
    /// ``1`` after :func:`eval_return` has populated the
    /// ``pending_return_*`` slots from a real ``return`` call.
    /// :func:`catch_leave` reads this to decide whether to
    /// snapshot the pending values into ``last_return_*`` (the
    /// user issued ``return …``) or to apply the conventional
    /// fallback mapping for an error / raw ``break`` / raw
    /// ``continue`` / clean exit.  Cleared on every catch_leave.
    pending_return_armed: u32 = 0,
    /// Custom numeric return code.  ``0`` means "use the standard
    /// ``return_flag`` → TCL_RETURN (2)" mapping; any other value
    /// is the user-specified ``return -code N`` numeric form.  Set
    /// by :func:`eval_return` for codes ≥ 5 (codes 0..4 keep the
    /// dedicated keyword / flag wiring) and read by
    /// :func:`catch_leave` to surface the exact code through
    /// ``catch``.  Cleared whenever ``return_flag`` is.
    return_code: i64 = 0,
    /// 1 = break pending (absorbed by loops).
    break_flag: u32 = 0,
    /// 1 = continue pending (absorbed by loops).
    continue_flag: u32 = 0,
    /// ``return -code break`` / ``return -code continue`` side-channel.
    /// Distinguishes a signalled break/continue (route to caller's
    /// enclosing loop) from a raw ``break``/``continue`` outside any
    /// loop (route to the "invoked outside of a loop" diagnostic).
    signal_break_flag: u32 = 0,
    signal_continue_flag: u32 = 0,
    /// Coroutine ``yield`` signal.  Distinct from ``return_flag`` so
    /// ``apply`` / proc dispatch don't silently absorb it.
    yield_flag: u32 = 0,
    yield_value: i32 = 0,
    /// ``catch BODY result``'s success-result holding slot.
    catch_ok_result: i32 = 0,
    /// Snapshot fields populated by :func:`catch_leave`; read by
    /// :func:`catch_result` / :func:`catch_options` after the
    /// scope is unwound.
    last_catch_had_error: u32 = 0,
    last_catch_code: i64 = 0,
    last_catch_value: i32 = 0,
    /// ``return``'s requested ``-code`` argument at the time of the
    /// most-recently-completed catch (TIP 90 options-dict ``-code``
    /// field, distinct from :attr:`last_catch_code` which is what
    /// ``catch`` itself returns).  Default ``0`` (TCL_OK) — set by
    /// :func:`eval_return` from the user-supplied keyword/numeric
    /// and snapshotted into :func:`catch_options`'s ``-code`` slot.
    /// Cleared on every catch_enter so a stale value can't leak
    /// from a prior catch into a new ``-options`` dict read.
    last_return_code: i64 = 0,
    /// ``return``'s requested ``-level`` (default ``1``).  Snapshot
    /// of :attr:`return_level` + the implicit unwind level so the
    /// catch-side options dict reads ``-level 1`` for plain
    /// ``return``, ``-level 0`` for ``return -level 0 …``, and
    /// ``-level N`` for ``return -level N …``.
    last_return_level: u32 = 1,
};

/// The single mutable state slot.  Module-private storage; external
/// callers go through ``result_mod`` (typed inspection / mutation)
/// or the per-field accessor pointers re-exported by
/// ``tcl_runtime.zig``.
pub var state: State = .{};

// Field-level convenience constants — let internal tcl_catch code
// keep its existing flat references (``error_flag = 1``) without
// the verbose ``state.error_flag = 1`` form.  Each one is a pointer
// that aliases a slot of the global ``state``.
//
// External callers MUST go through ``state.X`` (or, preferably, the
// typed mutation API in ``tcl_result.zig``).  The unqualified
// variable names below are NOT re-exported.

/// Read-and-clear helper for the compiled proc body's per-callsite
/// break/continue propagation.  Returns 1 if a *signalled*
/// break/continue is pending — the caller proc should ``return``
/// from its WASM function so the proc dispatcher can translate
/// the signal into the caller's ``break_flag`` / ``continue_flag``
/// for an enclosing loop to catch.  Leaves the signal flag set so
/// the dispatcher's translation runs.
pub export fn flow_check_signal_loop() i32 {
    if (state.signal_break_flag != 0 or state.signal_continue_flag != 0) {
        return 1;
    }
    return 0;
}

/// Lightweight accessor: callers that want to know whether they're
/// running under a Tcl-level catch use this to decide whether to
/// raise a Tcl error (caught) or stay silent (top-level — raising
/// would trap the whole bundle).
pub fn catch_depth_get() u32 {
    return state.catch_depth;
}

// Exported: enter a catch scope.
pub export fn catch_enter() void {
    state.catch_depth += 1;
    state.error_flag = 0;
    state.error_msg = 0;
    state.catch_ok_result = 0;
    // Release any captured payload from a previous catch so a
    // drain between catches doesn't see a stale +1 hold.
    if (state.last_catch_value != 0) {
        const old = state.last_catch_value;
        state.last_catch_value = 0;
        obj.tcl_obj_release(old);
    }
    // Reset pending ``return -code/-level/-errorinfo/-OPT`` metadata
    // before the new body runs.  ``eval_return`` arms these slots on
    // every ``return``, but proc-dispatch usually absorbs that
    // ``return`` without ever reaching ``catch_leave`` to clear them.
    // A subsequent unrelated ``catch BODY result options`` then
    // reports the prior proc's ``-code`` / ``-level`` (and dict-
    // extra options) — Codex review on PR #413.  Clearing on entry
    // means the next catch starts from a clean slate; if THIS body
    // executes a fresh ``return`` the values are stamped here again.
    state.pending_return_code = 0;
    state.pending_return_level = 1;
    state.pending_return_armed = 0;
    if (state.pending_return_extras != 0) {
        const old_extras = state.pending_return_extras;
        state.pending_return_extras = 0;
        obj.tcl_obj_release(old_extras);
    }
}

// Exported: signal a TCL_RETURN unwind from the compiled catch body.
// Mirrors the runtime ``return`` command: stash the value, set the
// flag, and let ``catch_has_error`` / ``catch_leave`` see it.  The
// codegen calls this for ``return ?value?`` inside a ``catch`` body
// so the WASM ``return`` instruction (which would jump past
// ``catch_leave``) is replaced by a flag-driven unwind.  Issue #..:
// fixes ``catch {return foo}`` exiting the surrounding proc.
pub export fn tcl_return_set(value: i32) void {
    state.return_flag = 1;
    // The codegen path that emits ``return value`` inside a catch
    // body always means a default ``return -level 1 -code ok``, so
    // reset the extra-level counter to 0 — leaving a stale value
    // from a previous ``return -code return`` would erroneously
    // propagate this regular return past the immediate proc.
    state.return_level = 0;
    // Stamp the TIP 90 options-dict slots with the default-return
    // values so a surrounding ``catch BODY result options`` sees
    // ``-code 0 -level 1``.  Mirrors :func:`eval_return`'s default
    // path; without this the codegen-emitted return path would
    // leave ``pending_return_armed = 0`` and the catch options
    // would fall through to the conventional ``state.return_flag``
    // mapping (which historically reported ``-level 0`` for a
    // RETURN-class unwind).
    state.pending_return_code = 0;
    state.pending_return_level = 1;
    state.pending_return_armed = 1;
    const old = state.return_val;
    if (value != 0) obj.tcl_obj_retain(value);
    state.return_val = value;
    if (old != 0 and old != value) obj.tcl_obj_release(old);
}

/// Read-and-clear the pending break flag.  Returns 1 if a break was
/// pending (and clears the flag) so a compiled loop body can br out
/// after an eval-fallback that ran ``break`` inside an interpreter
/// script (e.g. a body passed to ``dict update`` / ``foreach`` /
/// ``catch`` that lives behind an eval-fallback).  Without this hook,
/// the wasm-side loop never noticed the break and kept iterating.
pub export fn flow_consume_break() i32 {
    if (state.break_flag != 0) {
        state.break_flag = 0;
        // ``return -code break`` paired ``break_flag`` with the
        // ``signal_break_flag`` side-channel.  Clear both so the
        // proc dispatcher's post-call sweep doesn't re-fire on a
        // signal the loop already absorbed.
        state.signal_break_flag = 0;
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
    if (state.continue_flag != 0) {
        state.continue_flag = 0;
        state.signal_continue_flag = 0;
        return 1;
    }
    return 0;
}

/// Read the pending return flag without clearing it.  Compiled
/// procs use this to detect that an eval-fallback or callee proc
/// raised ``return`` — the caller proc must exit immediately
/// with the pending ``return_val`` as its result, leaving the
/// flag set so the standard proc-dispatch absorption (in the
/// caller's caller) clears it.
pub export fn flow_check_return() i32 {
    return @as(i32, @intCast(state.return_flag));
}

/// Compiled ``for`` loops call this once after the next-clause
/// has run to decide whether to iterate again or exit the loop.
///
/// Mirrors ``tclCmdAH.c`` ForPostNextCallback (Tcl 9.0.3 line ~2713):
///   * BREAK in next     → consume the flag, exit loop with TCL_OK
///   * CONTINUE in next  → leave the flag set, exit loop (caller's
///                         catch / proc dispatcher reports TCL_CONTINUE)
///   * ERROR / RETURN    → leave the flag set, exit loop (propagates)
///
/// Returns 1 when the loop should exit, 0 when it should continue.
/// Without this hook ``for {} {1} {break} {}`` and
/// ``for {} {1} {continue} {}`` (upstream for-6.17 / for-6.18) span-
/// lock the wasm-compiled loop because next-clause flow signals
/// from an eval-fallback never become a wasm ``br``.
pub export fn flow_for_next_post_check() i32 {
    if (state.break_flag != 0) {
        state.break_flag = 0;
        state.signal_break_flag = 0;
        return 1;
    }
    if (state.continue_flag != 0) return 1;
    if (state.error_flag != 0) return 1;
    if (state.return_flag != 0) return 1;
    return 0;
}

/// Non-consuming "is any control flow pending?" probe.  Compiled
/// ``for`` loops call this after the *init* clause to decide
/// whether to enter the loop or propagate immediately — init's
/// BREAK / CONTINUE / ERROR / RETURN all surface as the for
/// command's own exit code (per ``tclCmdAH.c`` Tcl_ForObjCmd line
/// ~2598), so we leave the flag set and just signal the compiled
/// emitter to br to the break target.
pub export fn flow_check_any_signal() i32 {
    if (state.break_flag != 0) return 1;
    if (state.continue_flag != 0) return 1;
    if (state.error_flag != 0) return 1;
    if (state.return_flag != 0) return 1;
    return 0;
}

/// Read-and-clear the pending return state, returning the value
/// the body intended to return.  Used at the compiled-proc-call
/// boundary in the caller to absorb a callee's ``return`` so
/// surrounding code resumes normally with the returned value as
/// the call result — same role :func:`tcl_interp.eval_proc_call`
/// plays for interpreted callees.
///
/// Returns 0 when no return is pending.  Note 0 is also a valid
/// ``return_val`` for ``return ""`` / ``return`` with no arg —
/// callers that need to distinguish must call
/// :func:`flow_check_return` first.
pub export fn flow_take_return() i32 {
    if (state.return_flag != 0) {
        const v = state.return_val;
        // ``return -code return`` / ``-level N>1`` propagates past the
        // immediate proc.  Decrement the level counter; only clear the
        // flag once the unwind is complete (counter at 0).
        if (state.return_level > 0) {
            state.return_level -= 1;
            // Leave ``return_flag`` and ``return_val`` set so the next
            // proc up on the call stack also unwinds.
            return v;
        }
        state.return_flag = 0;
        state.return_val = 0;
        return v;
    }
    return 0;
}

// Exported: record the success result of the catch body's last statement.
// Called by compiled catch bodies after their last statement when a
// result variable is needed.  Ignored if an error already occurred.
pub export fn catch_set_ok_result(val: i32) void {
    if (state.error_flag == 0) {
        state.catch_ok_result = val;
    }
}

// ``catch_leave`` clears ``error_flag`` so surrounding (non-catch)
// code doesn't see a stale pending error.  But ``catch_result`` is
// Tcl return-code constants — mirrors ``generic/tcl.h``.
const TCL_OK: i64 = 0;
const TCL_ERROR: i64 = 1;
const TCL_RETURN: i64 = 2;
const TCL_BREAK: i64 = 3;
const TCL_CONTINUE: i64 = 4;

// Exported: leave a catch scope.  Returns the TCL_* return code
// produced by the body — TCL_OK (0), TCL_ERROR (1), TCL_RETURN (2),
// TCL_BREAK (3), or TCL_CONTINUE (4) — matching reference Tcl's
// ``Tcl_CatchObjCmd``.  All four flow-control flags are absorbed
// here so the surrounding (non-catch) compiled code doesn't see
// them and the loop / proc dispatcher above us doesn't double-fire.
pub export fn catch_leave() i32 {
    if (state.catch_depth > 0) state.catch_depth -= 1;
    const code: i64 = blk: {
        if (state.error_flag != 0) break :blk TCL_ERROR;
        if (state.return_flag != 0) {
            // ``return -code N`` for numeric N ≥ 5 sets
            // ``return_code`` to the exact value the user passed —
            // catch surfaces that code instead of the generic
            // ``TCL_RETURN``.  Tcl 9 ``Tcl_CatchObjCmd`` reads the
            // body's return code from ``iPtr->returnCode`` for
            // the same purpose.  coroutine.test 7.5 round-trips
            // codes 2..5 through this path.
            if (state.return_code != 0) break :blk state.return_code;
            break :blk TCL_RETURN;
        }
        if (state.break_flag != 0) break :blk TCL_BREAK;
        if (state.continue_flag != 0) break :blk TCL_CONTINUE;
        break :blk TCL_OK;
    };
    state.last_catch_had_error = if (state.error_flag != 0) 1 else 0;
    state.last_catch_code = code;
    // Snapshot the TIP 90 ``-code`` / ``-level`` slots so
    // :func:`catch_options` can report them after the per-event
    // state below is cleared.  When ``eval_return`` armed the
    // pending slots (the body executed ``return …`` in any form),
    // the snapshot picks up the user-supplied values verbatim.
    // For everything else (raw ``break`` / ``continue``, a body
    // that raised via ``error``, or a clean exit) fall back to the
    // conventional mapping reference Tcl stamps on the dict.
    if (state.pending_return_armed != 0) {
        state.last_return_code = state.pending_return_code;
        state.last_return_level = state.pending_return_level;
    } else if (state.error_flag != 0) {
        state.last_return_code = 1;
        state.last_return_level = 0;
    } else if (state.break_flag != 0) {
        state.last_return_code = 3;
        state.last_return_level = 1;
    } else if (state.continue_flag != 0) {
        state.last_return_code = 4;
        state.last_return_level = 1;
    } else {
        state.last_return_code = 0;
        state.last_return_level = 0;
    }
    // Clear the pending slots so a subsequent ``catch`` body that
    // doesn't issue a ``return`` doesn't inherit stale values.
    state.pending_return_code = 0;
    state.pending_return_level = 1;
    state.pending_return_armed = 0;
    // Snapshot the user-supplied ``-OPT VALUE`` pairs into
    // ``last_return_extras`` so :func:`catch_options` (called
    // after ``catch_leave`` clears the pending slot) sees them.
    // Release any prior snapshot first.
    if (state.last_return_extras != 0) {
        obj.tcl_obj_release(state.last_return_extras);
        state.last_return_extras = 0;
    }
    state.last_return_extras = state.pending_return_extras;
    state.pending_return_extras = 0;
    // Snapshot the body's payload so ``catch_result`` can hand it back
    // to the caller's result-var writeback.  TCL_RETURN carries
    // ``return_val``; TCL_ERROR carries ``error_msg``.  Retain the
    // handle so a drain between ``catch_leave`` and ``catch_result``
    // can't free it out from under the writeback (Codex/Copilot
    // review on PR #324).  The previous occupant of
    // ``last_catch_value`` is released.
    const old_value = state.last_catch_value;
    var new_value: i32 = 0;
    if (state.return_flag != 0) {
        new_value = state.return_val;
    } else if (state.error_flag != 0) {
        new_value = state.error_msg;
    }
    if (new_value != 0 and new_value != old_value) obj.tcl_obj_retain(new_value);
    state.last_catch_value = new_value;
    if (old_value != 0 and old_value != new_value) obj.tcl_obj_release(old_value);
    // Absorb every flow-control flag — ``catch`` is the universal sink
    // for non-OK return codes from its body.  Surrounding loops
    // (``flow_consume_break``) and proc dispatchers (``return_flag``)
    // would otherwise re-fire on a code we already converted into a
    // return value.  Also clear ``return_val`` so a stale handle
    // doesn't leak into the next ``return`` site's release of ``old``.
    state.error_flag = 0;
    state.last_log_script = 0;
    state.last_log_pos = 0;
    state.error_info_supplied = 0;
    state.return_via_error_code = 0;
    // ``error_frame_line`` is NOT reset here — reference Tcl's
    // ``Tcl_ResetResult`` (called from ``INST_END_CATCH``) clears
    // ``errorInfo`` and the ``ERR_ALREADY_LOGGED`` flag, but leaves
    // ``iPtr->errorLine`` intact.  A subsequent ``error msg info``
    // (whose options carry no ``-errorline``) then propagates with
    // the preserved line, so the procedure frame's ``line N`` matches
    // the *original* inner error rather than the rethrow's own line
    // (error-2.6).  Fresh errors via ``tcl_cmd_error`` /
    // ``tcl_cmd_error_full`` (without info) overwrite the field
    // themselves.
    state.return_flag = 0;
    state.break_flag = 0;
    state.continue_flag = 0;
    state.return_level = 0;
    state.return_code = 0;
    state.signal_break_flag = 0;
    state.signal_continue_flag = 0;
    if (state.return_val != 0) {
        // Hand the +1 retain that ``return_val`` was holding to
        // ``last_catch_value`` (we already retained above).  Release
        // here cancels the source-side hold.
        const rv = state.return_val;
        state.return_val = 0;
        obj.tcl_obj_release(rv);
    }
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
    // TCL_ERROR / TCL_RETURN: hand back the payload captured at
    // ``catch_leave`` time (error message or ``return`` value).  TCL_OK
    // (and the bare TCL_BREAK / TCL_CONTINUE no-payload codes) fall
    // through to the body's last-statement value or the empty
    // sentinel.  This keeps ``catch {return foo} r`` populating ``r``
    // with ``foo``, matching reference Tcl's ``Tcl_CatchObjCmd``
    // (Codex/Copilot review on PR #324).
    if (state.last_catch_value != 0) return state.last_catch_value;
    if (state.last_catch_had_error != 0) return state.error_msg;
    if (state.catch_ok_result == 0) return obj_new_string(0, 0);
    return state.catch_ok_result;
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
    // Merge in any arbitrary ``-OPT VALUE`` pairs the caller
    // supplied to ``return`` (cmdMZ-return-2.1 / 2.18 etc.).  These
    // come FIRST so they sit at the head of the dict in insertion
    // order before the standard ``-code`` / ``-level`` slots — Tcl
    // 9's tcltest expects ``-bar soom -code 0 -level 1`` not the
    // other way around.
    if (state.last_return_extras != 0) {
        const keys = dict_mod.dict_keys(state.last_return_extras);
        if (keys != 0) {
            const ks = obj.obj_ensure_string(keys);
            const n = obj.list_count_elements(ks.ptr, ks.len);
            var i: i64 = 0;
            while (i < n) : (i += 1) {
                const elem = obj.list_element_at(ks.ptr, ks.len, i);
                const key_obj = obj.obj_new_string_copy(ks.ptr + elem.start, elem.len);
                const val_obj = dict_mod.dict_get(state.last_return_extras, key_obj);
                if (val_obj != 0) {
                    const new_d = dict_mod.dict_set(d, key_obj, val_obj);
                    if (new_d != d) obj.tcl_obj_release(d);
                    d = new_d;
                }
                obj.tcl_obj_release(key_obj);
            }
            obj.tcl_obj_release(keys);
        }
    }
    // TIP 90: the ``-code`` / ``-level`` slots report the
    // user-supplied codes from ``return`` (or the conventional
    // mapping for raw break/continue/error), NOT the catch's
    // effective code in ``last_catch_code``.  See the snapshot
    // logic in :func:`catch_leave`.
    d = dict_set_str_take(d, "-code", obj_new_int(state.last_return_code));
    d = dict_set_str_take(d, "-level", obj_new_int(@intCast(state.last_return_level)));
    // ``-errorcode`` / ``-errorinfo`` only appear on the catch
    // options dict when the body raised an error (``error msg`` /
    // ``throw`` / runtime trap absorbed by catch).  Adding them
    // for a clean ``return`` (cmdMZ-return-2.x) inflates the dict
    // by two extra keys, breaking exact-match tests that expect
    // ``-code 0 -level 1`` verbatim.  Match reference Tcl's
    // ``Tcl_GetReturnOptions`` which only stamps the error
    // metadata onto the dict when ``iPtr->returnCode == TCL_ERROR``.
    if (state.last_catch_had_error != 0) {
        const ec_name = obj_new_string_copy(@intFromPtr("::errorCode".ptr), 11);
        const ec_val = globals.global_get(ec_name);
        obj.tcl_obj_release(ec_name);
        if (ec_val != 0) {
            d = dict_set_str_keep(d, "-errorcode", ec_val);
        } else {
            d = dict_set_str_take(d, "-errorcode", obj_new_string_copy(
                @intFromPtr("NONE".ptr),
                4,
            ));
        }
        const ei_name = obj_new_string_copy(@intFromPtr("::errorInfo".ptr), 11);
        const ei_val = globals.global_get(ei_name);
        obj.tcl_obj_release(ei_name);
        if (ei_val != 0) {
            d = dict_set_str_keep(d, "-errorinfo", ei_val);
        } else if (state.error_msg != 0) {
            d = dict_set_str_keep(d, "-errorinfo", state.error_msg);
        }
    }
    // ``try { ... } finally { ... }`` chaining (error-16.21 /
    // -18.7..10): if ``eval_try`` captured the body's completion
    // options before running the finally and the finally raised,
    // attach the captured dict here under ``-during``.  We consume
    // the slot — dict_set retains for the new dict, and we release
    // our handle so a subsequent try doesn't double-add or leak.
    if (state.during_options != 0) {
        const during = state.during_options;
        state.during_options = 0;
        d = dict_set_str_keep(d, "-during", during);
        obj.tcl_obj_release(during);
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
    if (state.error_flag != 0) return 1;
    if (state.return_flag != 0) return 1;
    if (state.break_flag != 0) return 1;
    if (state.continue_flag != 0) return 1;
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
    // ``error msg {}`` (empty info) reverts to the standard
    // auto-generated ``msg\n    while executing\n"..."`` form, so
    // seed ``::errorInfo`` from ``msg`` and let the unwind path
    // append the per-frame trace.  Reference Tcl's
    // ``Tcl_ErrorObjCmd`` checks ``Tcl_GetStringFromObj(info,
    // &length)`` and only uses ``info`` when ``length > 0``.
    var info_val: i32 = msg;
    if (info != 0) {
        const is = obj.obj_ensure_string(info);
        if (is.len > 0) info_val = info;
    }
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

/// Probe error message text for well-known prefixes/values that
/// reference Tcl decorates with a specific ``::errorCode``.
///
/// Detected:
///   * ``wrong # args:``  → ``TCL WRONGARGS``
///     (``Tcl_WrongNumArgs`` stamps this on every arity error;
///     rename.test 3.1 / 3.2 etc. grep for the exact code.)
///   * ``divide by zero``
///       → ``ARITH DIVZERO {divide by zero}``
///   * ``domain error: argument not in valid range``
///       → ``ARITH DOMAIN {domain error: argument not in valid range}``
///   * ``floating-point value too large to represent``
///       → ``ARITH OVERFLOW {floating-point value too large to represent}``
///   * ``integer value too large to represent``
///       → ``ARITH IOVERFLOW {integer value too large to represent}``
///
/// Centralising the detection at the error sink keeps every
/// ``stubs.raise(...)`` call site free of errorCode plumbing —
/// arith helpers in ``tcl_arith.zig`` / ``tcl_mathop.zig``, the
/// math funcs, and the bignum overflow paths all benefit without
/// changes (expr-old 26.8/26.9, 34.9/34.10, etc.).
fn detect_error_code(msg: i32) i32 {
    const s = obj_ensure_string(msg);
    if (s.len == 0) return 0;
    const sp: [*]const u8 = @ptrFromInt(s.ptr);
    if (starts_with(sp, s.len, "wrong # args:")) {
        const code_text: []const u8 = "TCL WRONGARGS";
        return obj_new_string_copy(@intFromPtr(code_text.ptr), code_text.len);
    }
    if (slice_eq_lit(sp, s.len, "divide by zero")) {
        const code_text: []const u8 = "ARITH DIVZERO {divide by zero}";
        return obj_new_string_copy(@intFromPtr(code_text.ptr), code_text.len);
    }
    if (slice_eq_lit(sp, s.len, "domain error: argument not in valid range")) {
        const code_text: []const u8 =
            "ARITH DOMAIN {domain error: argument not in valid range}";
        return obj_new_string_copy(@intFromPtr(code_text.ptr), code_text.len);
    }
    if (slice_eq_lit(sp, s.len, "floating-point value too large to represent")) {
        const code_text: []const u8 =
            "ARITH OVERFLOW {floating-point value too large to represent}";
        return obj_new_string_copy(@intFromPtr(code_text.ptr), code_text.len);
    }
    if (slice_eq_lit(sp, s.len, "integer value too large to represent")) {
        const code_text: []const u8 =
            "ARITH IOVERFLOW {integer value too large to represent}";
        return obj_new_string_copy(@intFromPtr(code_text.ptr), code_text.len);
    }
    return 0;
}

fn starts_with(sp: [*]const u8, slen: u32, prefix: []const u8) bool {
    if (slen < prefix.len) return false;
    for (prefix, 0..) |c, i| {
        if (sp[i] != c) return false;
    }
    return true;
}

fn slice_eq_lit(sp: [*]const u8, slen: u32, lit: []const u8) bool {
    if (slen != lit.len) return false;
    for (lit, 0..) |c, i| {
        if (sp[i] != c) return false;
    }
    return true;
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
// ``::errorCode`` defaults to ``NONE`` except when the message
// starts with ``wrong # args:``, in which case it is auto-tagged
// ``TCL WRONGARGS`` to match reference Tcl's ``Tcl_WrongNumArgs``.
pub export fn tcl_cmd_error(msg: i32) void {
    state.last_log_script = 0;
    state.last_log_pos = 0;
    state.error_info_supplied = 0;
    // Capture the current frame's tracked line so a surrounding proc
    // can stamp ``(procedure "X" line N)`` on its errorInfo frame
    // (error-2.3 / error-2.6).  Codegen emits ``frame_set_line`` per
    // statement; the interpreter does the same at eval-time, so the
    // freshest line is always available here.
    {
        const frames_mod = @import("tcl_frames.zig");
        state.error_frame_line = frames_mod.frame_get_line(0);
    }
    stamp_error_globals(msg, 0, detect_error_code(msg));
    if (state.catch_depth > 0) {
        state.error_flag = 1;
        state.error_msg = msg;
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

// Exported: ``return -code error msg`` shortcut — equivalent to
// :func:`tcl_cmd_error` but ALSO sets the
// :attr:`return_via_error_code` flag so the procedure-frame stamps
// (compiled epilogue + interpreted dispatch) skip the
// ``(procedure "X" line N)`` line.  Mirrors Tcl 9's distinction
// between body-level ``error msg`` (which ``MakeProcError``
// annotates) and ``return -code error msg`` (which goes through
// ``TclUpdateReturnInfo`` and bypasses the frame stamp) — proc-old-
// 7.2's expected ``::errorInfo`` lacks the ``(procedure …)``
// line in this exact form.
pub export fn tcl_cmd_error_via_return(msg: i32) void {
    state.return_via_error_code = 1;
    tcl_cmd_error(msg);
}

// Exported: error with explicit ``info`` / ``code`` arguments —
// matches the full 3-arg ``error`` command form
// (``error msg ?info? ?code?``).  Either or both extras may be
// ``0`` to use the defaults.
pub export fn tcl_cmd_error_full(msg: i32, info: i32, code: i32) void {
    state.last_log_script = 0;
    state.last_log_pos = 0;
    // ``error msg info ?code?`` with a NON-EMPTY *info* arg should
    // suppress the auto-generated first-frame ``while executing``
    // append — the user supplied the desired traceback text via
    // *info*.  Reference Tcl's ``Tcl_ErrorObjCmd`` checks the obj
    // length:: ``Tcl_GetStringFromObj(info, &len); if (len > 0)
    // ... ERR_LEGACY_COPY``.  An empty ``{}`` info reverts to the
    // standard auto-generated traceback (error-4.2/4.3/4.4 want
    // ``msg1\n    while executing\n"error msg1 {}"``-style text
    // even when ``info`` is supplied as the empty string).
    var suppress: u32 = 0;
    if (info != 0) {
        const is = obj.obj_ensure_string(info);
        if (is.len > 0) suppress = 1;
    }
    state.error_info_supplied = suppress;
    // Capture the current frame's tracked line — see ``tcl_cmd_error``
    // for the rationale.  When *info* IS supplied (suppress=1) the
    // user is re-raising a previous error's text; reference Tcl's
    // ``Tcl_ErrorObjCmd`` builds ``options`` WITHOUT a ``-errorline``
    // key, so ``TclProcessReturn`` leaves ``iPtr->errorLine`` at
    // whatever the prior error set it to.  Preserve that semantics
    // here so error-2.6's surrounding proc frame reports the original
    // inner ``error glorp2`` line (body line 3) rather than the
    // re-raise's own line (body line 4).
    if (suppress == 0) {
        const frames_mod = @import("tcl_frames.zig");
        state.error_frame_line = frames_mod.frame_get_line(0);
    }
    stamp_error_globals(msg, info, code);
    if (state.catch_depth > 0) {
        state.error_flag = 1;
        state.error_msg = msg;
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

// Build a ``invalid command name "<name>"`` TclObj and route it
// through @"error".  Used by the interpreter fallback when a word
// doesn't match any builtin or registered proc, and by the
// rename-masked dispatch arm in
// ``tcl_interp.eval_proc_call_bucket``.  The wording matches
// reference Tcl's ``TclEvalObjvInternal`` verbatim so tcltest's
// error-string matchers (``rename.test`` rename-1.2 / 2.1, etc.)
// agree on the surface.
//
// Historical note: this was named ``error_unknown_command`` and
// emitted ``unknown command: <name>`` until the rename-builtin
// work landed; it was renamed but the old export name is kept as
// an alias to avoid breaking the ~10 ``cmds/*.zig`` callers in the
// alias and stub paths.  See tcl_catch.zig comment block above.
pub fn error_invalid_command_name(cmd_obj: i32) void {
    const prefix: []const u8 = "invalid command name \"";
    const suffix: []const u8 = "\"";
    const s = obj_ensure_string(cmd_obj);
    const total: u32 = @intCast(prefix.len + s.len + suffix.len);
    // Allocate a fresh byte buffer in the bump allocator so the
    // TclObj's string data outlives this frame.
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
    // Issue #317: ``obj_new_string_take`` so the error TclObj
    // owns ``buf_addr`` and ``release_now`` returns it via
    // ``free_sized``; the older borrowing form leaked one buf per
    // raised error inside a ``catch``.
    const msg = obj.obj_new_string_take(buf_addr, total, total);
    tcl_cmd_error(msg);
}

/// Backwards-compatible alias preserving the historical export name
/// used by the alias dispatch path and the eval-fallback's unknown
/// command surface.  Both names produce the same ``invalid command
/// name "<name>"`` text now.  Kept until the call sites are
/// migrated; the duplication is one line of code, not a behaviour
/// difference.
pub fn error_unknown_command(cmd_obj: i32) void {
    error_invalid_command_name(cmd_obj);
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
    const s = obj_ensure_string(name_obj);
    // Tcl 9 picks a different suffix depending on the variable's
    // shape:
    //   * ``arr(key)`` where ``arr`` is an array → "no such element
    //     in array"
    //   * ``arr(key)`` where ``arr`` exists as a scalar → "variable
    //     isn't array"
    //   * ``arr`` (whole-array read) where ``arr`` is an array →
    //     "variable is array"
    //   * everything else → "no such variable"
    const suffix = pick_var_read_suffix(s);
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

fn pick_var_read_suffix(s: anytype) []const u8 {
    if (s.len == 0 or s.ptr == 0) return "\": no such variable";
    const sp: [*]const u8 = @ptrFromInt(s.ptr);
    // Find a ``(``...``)`` element form.
    var paren: u32 = 0;
    var has_paren = false;
    var k: u32 = 0;
    while (k < s.len) : (k += 1) {
        if (sp[k] == '(') {
            paren = k;
            has_paren = true;
            break;
        }
    }
    const tcl_array = @import("../valtypes/tcl_array.zig");
    if (has_paren and paren > 0 and sp[s.len - 1] == ')') {
        // ``arr(key)`` form — see if ``arr`` exists as an array.
        // ``array_exists_raw`` probes the array directory by the raw
        // (post-strip) name; if it returns true, the array exists
        // but the element doesn't.  If false, check whether a scalar
        // named ``arr`` exists — that's the "variable isn't array"
        // case.
        const arr_ptr = s.ptr;
        const arr_len = paren;
        const arr_p: [*]const u8 = @ptrFromInt(arr_ptr);
        // Strip a leading ``::`` for the array directory probe.
        var probe_ptr: u32 = arr_ptr;
        var probe_len: u32 = arr_len;
        if (arr_len >= 2 and arr_p[0] == ':' and arr_p[1] == ':') {
            probe_ptr += 2;
            probe_len -= 2;
        }
        if (tcl_array.array_exists_raw(probe_ptr, probe_len)) {
            return "\": no such element in array";
        }
        // Scalar with the same name?  ``global_exists`` probes the
        // flat-key store under both bare and FQ spellings.
        const obj_helpers = @import("../valtypes/tcl_obj.zig");
        const arr_obj = obj_helpers.obj_new_string(@bitCast(arr_ptr), @bitCast(arr_len));
        const tcl_ns = @import("tcl_ns.zig");
        const exists = obj_helpers.obj_get_int(tcl_ns.global_exists(arr_obj)) != 0;
        obj_helpers.tcl_obj_release(arr_obj);
        if (exists) {
            return "\": variable isn't array";
        }
        return "\": no such variable";
    }
    // Whole-name form: if a same-named array exists, that's the
    // "variable is array" case.
    var probe_ptr: u32 = s.ptr;
    var probe_len: u32 = s.len;
    if (s.len >= 2 and sp[0] == ':' and sp[1] == ':') {
        probe_ptr += 2;
        probe_len -= 2;
    }
    if (tcl_array.array_exists_raw(probe_ptr, probe_len)) {
        return "\": variable is array";
    }
    return "\": no such variable";
}
