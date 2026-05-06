// ``auto_load``, ``auto_reset``, ``auto_mkindex``, ``auto_import``,
// ``auto_execok``, ``auto_qualify``, ``package``, ``after``,
// ``vwait``, ``update``, ``clock`` — stub / degraded-implementation
// commands.
//
// **Why silent instead of trapping.**  The Phase-A parity baseline
// (``tests/baselines/wasm_command_parity.json``) flags these as
// ``SILENT_STUB``.  That classification is a deliberate book-keeping
// signal — it lists the commands whose degraded behaviour differs
// from full Tcl — not a bug to fix.  Every handler below returns a
// value that is *correct under the sandbox's capability envelope*;
// promoting any of them to ``stubs.unsupported(...)`` would break
// existing scripts that rely on graceful degradation (notably
// tcltest's ``cleanupTests``, which does ``update`` / ``after cancel``
// / ``auto_execok`` as part of its per-file teardown).
//
// Per-handler rationale:
//
//   * ``auto_load`` returns integer 0 meaning "not found".  Without the
//     Tcl stdlib there is nothing to auto-load; the 0 signal lets
//     ``unknown`` / ``package require`` paths see the expected "proc
//     not in index" response and fall back cleanly.
//
//   * ``auto_reset`` / ``auto_mkindex`` / ``auto_import`` /
//     ``auto_execok`` / ``auto_qualify`` — no index to reset, no
//     filesystem to scan for executables, no namespace imports to
//     compile.  Returning empty string matches "successful no-op",
//     which is what scripts using these for setup/cleanup expect.
//
//   * ``package`` — returns a null TclObj (0).  Full ``package
//     require`` / ``package provide`` support would need the stdlib
//     fetch mechanism.  Currently accepted as "package system not
//     implemented yet"; promotion to real implementation is tracked
//     separately.  Note: returning 0 means callers that unconditionally
//     use the result as an i32 are safe; callers that expect a version
//     string will see empty, consistent with no-version-declared.
//
//   * ``after`` — no event-loop backend in WASM.  ``after ms`` (sleep),
//     ``after ms script`` (schedule), ``after idle script``,
//     ``after cancel id``, and ``after info`` all return empty string.
//     ``after ms`` silently skipping the delay is acceptable for test
//     harnesses; scripts that depend on wall-clock pauses for
//     correctness are outside the sandbox's scope.
//
//   * ``vwait`` / ``update`` — event loop is absent, so waiting returns
//     immediately with empty string.  tcltest uses ``update`` to drain
//     stray events at teardown; the empty-string return matches Tcl's
//     behaviour when there are no events queued.
//
//   * ``clock`` — ``seconds`` / ``clicks`` / ``milliseconds`` have real
//     WASI-backed values.  ``format`` / ``add`` return the string
//     ``"0"`` and ``scan`` returns integer 0; these are deliberate
//     stubs because a proper timezone-aware formatter pulls in ~20 KB
//     of tzdata.
//
// When a future Tcl release ships an event-loop or package-manager
// backend for WASI, these handlers become real implementations and
// the parity classifier will notice the promotion (SILENT_STUB →
// IMPLEMENTED) without further configuration.

const std = @import("std");
const result_mod = @import("../interp/tcl_result.zig");
const rt = @import("../tcl_runtime.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");
const clock = @import("../io/tcl_clock.zig");

fn eval_auto_load(words: []const i32) result_mod.InterpResult {
    _ = words;
    return result_mod.from_globals(rt.obj_new_int(0));
}

fn eval_auto_noop(words: []const i32) result_mod.InterpResult {
    _ = words;
    return result_mod.from_globals(rt.obj_new_string(0, 0));
}

fn eval_package(words: []const i32) result_mod.InterpResult {
    _ = words;
    return result_mod.from_globals(0);
}

// Interpreter-side ``clock`` dispatcher.
//
// The Tcl compiler emits ``_emit_eval_fallback("clock", args)`` for
// subcommands it can't compile directly (``format``, ``scan``, ``add``).
// The fallback invokes the interpreter, which sees ``clock`` as the
// command.  Without this entry the interpreter errors "unknown command:
// clock".
//
// Subcommand routing:
//   seconds / clicks / milliseconds → WASI-backed real values.
//   format / add                    → return the string "0" (stub).
//   scan                            → return integer 0 (stub).
//   All others                      → empty string.
//
// ``format`` / ``scan`` / ``add`` stubbing mirrors ``tcl_time_stubs.zig``
// (the WASM-export path used by compiled procs).  See that file's
// header for the deliberate divergence from real Tcl semantics.
fn eval_clock(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) return result_mod.from_globals(clock.clock_seconds());
    const sub = rt.obj_ensure_string(words[1]);
    const sp: []const u8 = if (sub.ptr == 0) "" else @as([*]const u8, @ptrFromInt(sub.ptr))[0..sub.len];
    if (std.mem.eql(u8, sp, "seconds")) return result_mod.from_globals(clock.clock_seconds());
    if (std.mem.eql(u8, sp, "clicks")) return result_mod.from_globals(clock.clock_clicks());
    if (std.mem.eql(u8, sp, "milliseconds")) return result_mod.from_globals(clock.clock_milliseconds());
    if (std.mem.eql(u8, sp, "microseconds")) {
        // micro = clicks (already microseconds from the fast clock)
        return result_mod.from_globals(clock.clock_clicks());
    }
    if (std.mem.eql(u8, sp, "scan")) {
        // Form: clock scan TEXT ?-base T? ?-format F? ?-gmt 0|1? ?-timezone Z? ?-locale L?
        if (words.len < 3) return result_mod.from_globals(rt.obj_new_int(0));
        var zone_obj: i32 = 0;
        var base_obj: i32 = 0;
        var fmt_obj: i32 = 0;
        var locale_obj: i32 = 0;
        var gmt: i32 = 0;
        var ai: usize = 3;
        while (ai + 1 < words.len) : (ai += 2) {
            const optn = rt.obj_ensure_string(words[ai]);
            const op: []const u8 = if (optn.ptr == 0) "" else @as([*]const u8, @ptrFromInt(optn.ptr))[0..optn.len];
            if (std.mem.eql(u8, op, "-timezone")) {
                zone_obj = words[ai + 1];
            } else if (std.mem.eql(u8, op, "-base")) {
                base_obj = words[ai + 1];
            } else if (std.mem.eql(u8, op, "-format")) {
                fmt_obj = words[ai + 1];
            } else if (std.mem.eql(u8, op, "-locale")) {
                locale_obj = words[ai + 1];
            } else if (std.mem.eql(u8, op, "-gmt")) {
                const v = rt.obj_ensure_string(words[ai + 1]);
                if (v.ptr != 0 and v.len > 0) {
                    const vp: [*]const u8 = @ptrFromInt(v.ptr);
                    const vs = vp[0..v.len];
                    gmt = if (std.mem.eql(u8, vs, "0") or
                        std.mem.eql(u8, vs, "false") or
                        std.mem.eql(u8, vs, "no")) 0 else 1;
                }
            }
        }
        if (fmt_obj != 0) {
            // Strftime-driven path.  The locale flag controls roman-
            // numeral parsing of ``%Od`` / ``%Oe`` / ``%Om`` / ``%Oy``;
            // an unrecognised locale falls back to plain numeric.
            const f = rt.obj_ensure_string(fmt_obj);
            if (f.ptr != 0 and f.len > 0) {
                return result_mod.from_globals(clock.clock_scan_format(words[2], fmt_obj, gmt, zone_obj, base_obj, locale_obj));
            }
        }
        return result_mod.from_globals(clock.clock_scan_obj(words[2], zone_obj, gmt, base_obj));
    }
    if (std.mem.eql(u8, sp, "format")) {
        // Parse: clock format SECONDS ?-format FMT? ?-gmt 0|1? ?-timezone Z? ?-locale L?
        if (words.len < 3) return result_mod.from_globals(rt.obj_new_string(0, 0));
        var fmt_obj: i32 = 0;
        var zone_obj: i32 = 0;
        var locale_obj: i32 = 0;
        var gmt: bool = false;
        var ai: usize = 3;
        while (ai + 1 < words.len) : (ai += 2) {
            const optn = rt.obj_ensure_string(words[ai]);
            const op: []const u8 = if (optn.ptr == 0) "" else @as([*]const u8, @ptrFromInt(optn.ptr))[0..optn.len];
            if (std.mem.eql(u8, op, "-format")) {
                fmt_obj = words[ai + 1];
            } else if (std.mem.eql(u8, op, "-timezone")) {
                zone_obj = words[ai + 1];
            } else if (std.mem.eql(u8, op, "-locale")) {
                locale_obj = words[ai + 1];
            } else if (std.mem.eql(u8, op, "-gmt")) {
                // Tcl accepts ``0`` / ``1`` / ``true`` / ``false``;
                // anything non-empty starting with a non-``0`` digit
                // counts as truthy for our purposes.
                const v = rt.obj_ensure_string(words[ai + 1]);
                if (v.ptr != 0 and v.len > 0) {
                    const vp: [*]const u8 = @ptrFromInt(v.ptr);
                    const vs = vp[0..v.len];
                    gmt = !(std.mem.eql(u8, vs, "0") or
                        std.mem.eql(u8, vs, "false") or
                        std.mem.eql(u8, vs, "no"));
                }
            }
            // ``-validate`` ignored — moot for the strftime path.
        }
        if (gmt) {
            // ``-gmt 1`` overrides ``-timezone``.  Reference Tcl
            // resolves this to a synthetic ``GMT`` zone (with
            // ``GMT`` — not ``UTC`` — as the abbreviation rendered
            // by ``%Z``).  We pass ``:GMT`` so the resolver's
            // ``Etc/GMT`` alias picks up the right abbr.
            const gmt_name = rt.obj_new_string_copy(
                @intCast(@intFromPtr(":GMT".ptr)),
                4,
            );
            return result_mod.from_globals(clock.clock_format_tz_locale(words[2], fmt_obj, gmt_name, locale_obj));
        }
        return result_mod.from_globals(clock.clock_format_tz_locale(words[2], fmt_obj, zone_obj, locale_obj));
    }
    if (std.mem.eql(u8, sp, "add")) {
        // Form: clock add BASE ?COUNT UNIT?* ?-gmt 0|1? ?-timezone Z?
        // Iterate (count, unit) pairs and accumulate.  Option flags
        // (``-gmt`` / ``-timezone`` / ``-locale``) are recognised and
        // skipped — they affect calendar-month math semantics but the
        // current implementation does month arithmetic in UTC, which
        // matches Tcl's behaviour for non-DST-crossing month adds.
        if (words.len < 3) return result_mod.from_globals(rt.obj_new_int(0));
        var acc: i32 = words[2];
        var ai: usize = 3;
        while (ai < words.len) {
            const w = rt.obj_ensure_string(words[ai]);
            const ws: []const u8 = if (w.ptr == 0) "" else @as([*]const u8, @ptrFromInt(w.ptr))[0..w.len];
            // Distinguish a real option flag (``-gmt`` / ``-timezone``)
            // from a negative count (``-1 days``).  An option starts
            // with ``-`` followed by an alphabetic character; anything
            // else (``-``, ``-1``, ``-0.5``) is a numeric count.
            if (ws.len >= 2 and ws[0] == '-' and
                ((ws[1] >= 'a' and ws[1] <= 'z') or
                    (ws[1] >= 'A' and ws[1] <= 'Z')))
            {
                ai += 2; // skip option + value
                continue;
            }
            if (ai + 1 >= words.len) break;
            acc = clock.clock_add_pair(acc, words[ai], words[ai + 1]);
            ai += 2;
        }
        return result_mod.from_globals(acc);
    }
    return result_mod.from_globals(rt.obj_new_string(0, 0));
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "auto_load", .arity_min = 1, .arity_max = 1, .handler = &eval_auto_load },
    .{ .name = "auto_reset", .arity_min = 0, .arity_max = 0, .handler = &eval_auto_noop },
    .{ .name = "auto_mkindex", .arity_min = 1, .arity_max = null, .handler = &eval_auto_noop },
    .{ .name = "auto_import", .arity_min = 1, .arity_max = null, .handler = &eval_auto_noop },
    .{ .name = "auto_execok", .arity_min = 1, .arity_max = 1, .handler = &eval_auto_noop },
    .{ .name = "auto_qualify", .arity_min = 2, .arity_max = 2, .handler = &eval_auto_noop },
    .{ .name = "package", .arity_min = 1, .arity_max = null, .handler = &eval_package },
    // ``after`` / ``vwait`` / ``update`` moved to ``cmds/after.zig`` +
    // ``cmds/event.zig`` (real scheduler-backed implementations).
    .{ .name = "clock", .arity_min = 1, .arity_max = null, .handler = &eval_clock },
};

// ``clock <sub>`` sub-commands — mirrors
// ``core/commands/registry/tcl/clock.py``.  Cross-checked against
// C Tcl 9.0 ``generic/tclClock.c`` + ``tclClockFmt.c`` (ensemble
// implementations invoke ``Tcl_WrongNumArgs`` on bad arg counts).
pub const clock_subcommands: []const reg.SubEntry = &.{
    .{ .name = "add", .arity_min = 1, .arity_max = null, .handler = &eval_clock },
    .{ .name = "clicks", .arity_min = 0, .arity_max = 1, .handler = &eval_clock },
    .{ .name = "format", .arity_min = 1, .arity_max = null, .handler = &eval_clock },
    .{ .name = "microseconds", .arity_min = 0, .arity_max = 0, .handler = &eval_clock },
    .{ .name = "milliseconds", .arity_min = 0, .arity_max = 0, .handler = &eval_clock },
    .{ .name = "scan", .arity_min = 1, .arity_max = null, .handler = &eval_clock },
    .{ .name = "seconds", .arity_min = 0, .arity_max = 0, .handler = &eval_clock },
};

// ``package <sub>`` sub-commands — mirrors
// ``core/commands/registry/tcl/package.py``.  Cross-checked against
// C Tcl 9.0 ``generic/tclPkg.c``.
pub const package_subcommands: []const reg.SubEntry = &.{
    .{ .name = "files", .arity_min = 1, .arity_max = 1, .handler = &eval_package },
    .{ .name = "forget", .arity_min = 0, .arity_max = null, .handler = &eval_package },
    .{ .name = "ifneeded", .arity_min = 2, .arity_max = 3, .handler = &eval_package },
    .{ .name = "names", .arity_min = 0, .arity_max = 0, .handler = &eval_package },
    .{ .name = "prefer", .arity_min = 0, .arity_max = 1, .handler = &eval_package },
    .{ .name = "present", .arity_min = 1, .arity_max = null, .handler = &eval_package },
    .{ .name = "provide", .arity_min = 1, .arity_max = 2, .handler = &eval_package },
    .{ .name = "require", .arity_min = 1, .arity_max = null, .handler = &eval_package },
    .{ .name = "unknown", .arity_min = 0, .arity_max = 1, .handler = &eval_package },
    .{ .name = "vcompare", .arity_min = 2, .arity_max = 2, .handler = &eval_package },
    .{ .name = "versions", .arity_min = 1, .arity_max = 1, .handler = &eval_package },
    .{ .name = "vsatisfies", .arity_min = 2, .arity_max = null, .handler = &eval_package },
};
