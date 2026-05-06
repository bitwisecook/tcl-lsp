// ``after`` — schedule scripts on the WASM event loop.
//
// Forms:
//   ``after MS``                 — block (sleep) for MS ms.
//   ``after MS script ?script…?``— concat scripts with spaces, schedule.
//   ``after idle script ?…?``    — schedule on the idle queue.
//   ``after cancel id|script``   — cancel a pending script.
//   ``after info ?id?``          — list pending ids, or describe one.

const std = @import("std");
const result_mod = @import("../interp/tcl_result.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");
const obj = @import("../valtypes/tcl_obj.zig");
const sched = @import("../sched/tcl_sched.zig");
const stubs = @import("../stubs/tcl_stubs.zig");

fn parse_int_word(o: i32) ?i64 {
    // Use the runtime's shared integer parser so ``after`` accepts
    // every shape Tcl does — leading sign, leading whitespace, hex /
    // octal / binary prefixes — instead of a bespoke digit loop that
    // rejected ``after +5`` (Copilot review on PR #284).
    const s = obj.obj_ensure_string(o);
    if (s.len == 0) return null;
    return obj.try_parse_int(s.ptr, s.len);
}

fn concat_scripts(words: []const i32) i32 {
    const interp = @import("../interp/tcl_interp.zig");
    return interp.concat_words(words);
}

fn eval_after(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) {
        stubs.raise("wrong # args: should be \"after option ?arg ...?\"");
        return result_mod.from_globals(0);
    }
    const sub = obj.obj_ensure_string(words[1]);
    const sp: []const u8 = if (sub.ptr == 0) "" else @as([*]const u8, @ptrFromInt(sub.ptr))[0..sub.len];

    // ``after cancel ...``
    if (std.mem.eql(u8, sp, "cancel")) {
        if (words.len < 3) {
            stubs.raise("wrong # args: should be \"after cancel id|script\"");
            return result_mod.from_globals(0);
        }
        // Concatenate remaining words; either a single id or a
        // multi-word script.  ``concat_scripts`` returns ``words[2]``
        // verbatim for the 1-arg case (no allocation) and a fresh
        // TclObj for the multi-arg case — release the latter once
        // we're done so MM-B.4 word cleanup doesn't leak it.
        const multi_word = words.len > 3;
        const arg = if (multi_word) concat_scripts(words[2..]) else words[2];
        const ar = obj.obj_ensure_string(arg);
        _ = sched.cancel_by_token(ar.ptr, ar.len);
        if (multi_word) obj.tcl_obj_release(arg);
        return result_mod.from_globals(obj.obj_new_string(0, 0));
    }

    // ``after info ?id?``
    if (std.mem.eql(u8, sp, "info")) {
        if (words.len == 2) return result_mod.from_globals(sched.info_all());
        if (words.len == 3) {
            const ar = obj.obj_ensure_string(words[2]);
            const r = sched.info_one(ar.ptr, ar.len);
            if (r == 0) {
                stubs.raise("event not found");
                return result_mod.from_globals(0);
            }
            return result_mod.from_globals(r);
        }
        stubs.raise("wrong # args: should be \"after info ?id?\"");
        return result_mod.from_globals(0);
    }

    // ``after idle script ?script ...?``
    if (std.mem.eql(u8, sp, "idle")) {
        if (words.len < 3) {
            stubs.raise("wrong # args: should be \"after idle script ?script ...?\"");
            return result_mod.from_globals(0);
        }
        // Multi-word form allocates a fresh concat TclObj.
        // ``schedule_idle`` retains it; we release our +1 so the
        // scheduler is the sole owner.  Single-word form passes
        // ``words[2]`` straight through (parsed-array ref released
        // by MM-B.4 word cleanup).
        const multi_word = words.len > 3;
        const body = if (multi_word) concat_scripts(words[2..]) else words[2];
        const id = sched.schedule_idle(body);
        if (multi_word) obj.tcl_obj_release(body);
        return result_mod.from_globals(id);
    }

    // First arg is ms.  Either ``after MS`` (sleep) or ``after MS script...``.
    const ms = parse_int_word(words[1]) orelse {
        stubs.raise("bad argument: must be cancel, idle, info, or an integer");
        return result_mod.from_globals(0);
    };
    if (ms < 0) {
        stubs.raise("argument must be non-negative");
        return result_mod.from_globals(0);
    }
    if (words.len == 2) {
        // Sleep form — drain any due events while we wait so a
        // concurrent timer scheduled to fire during the sleep
        // doesn't starve.  Implementation: yield to the scheduler's
        // tick loop with a deadline.
        sched.sleep_ms(ms);
        return result_mod.from_globals(obj.obj_new_string(0, 0));
    }
    // Same temp-TclObj ownership pattern as the idle / cancel
    // branches above — release our +1 from concat_scripts so
    // ``schedule_after``'s retain is the sole owner.
    const multi_word = words.len > 3;
    const body = if (multi_word) concat_scripts(words[2..]) else words[2];
    const id = sched.schedule_after(ms, body);
    if (multi_word) obj.tcl_obj_release(body);
    return result_mod.from_globals(id);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "after", .arity_min = 1, .arity_max = null, .handler = &eval_after },
};
