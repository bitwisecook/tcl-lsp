// ``after`` — schedule scripts on the WASM event loop.
//
// Forms:
//   ``after MS``                 — block (sleep) for MS ms.
//   ``after MS script ?script…?``— concat scripts with spaces, schedule.
//   ``after idle script ?…?``    — schedule on the idle queue.
//   ``after cancel id|script``   — cancel a pending script.
//   ``after info ?id?``          — list pending ids, or describe one.

const std = @import("std");
const reg = @import("../dispatch/tcl_cmd_registry.zig");
const obj = @import("../valtypes/tcl_obj.zig");
const sched = @import("../sched/tcl_sched.zig");
const stubs = @import("../stubs/tcl_stubs.zig");

fn parse_int_word(o: i32) ?i64 {
    const s = obj.obj_ensure_string(o);
    if (s.len == 0) return null;
    const p: [*]const u8 = @ptrFromInt(s.ptr);
    var i: u32 = 0;
    var neg = false;
    if (p[0] == '-') { neg = true; i = 1; }
    if (i >= s.len) return null;
    var v: i64 = 0;
    while (i < s.len) : (i += 1) {
        const c = p[i];
        if (c < '0' or c > '9') return null;
        v = v * 10 + (c - '0');
    }
    return if (neg) -v else v;
}

fn concat_scripts(words: []const i32) i32 {
    const interp = @import("../interp/tcl_interp.zig");
    return interp.concat_words(words);
}

fn eval_after(words: []const i32) i32 {
    if (words.len < 2) {
        stubs.raise("wrong # args: should be \"after option ?arg ...?\"");
        return 0;
    }
    const sub = obj.obj_ensure_string(words[1]);
    const sp: []const u8 = if (sub.ptr == 0) "" else
        @as([*]const u8, @ptrFromInt(sub.ptr))[0..sub.len];

    // ``after cancel ...``
    if (std.mem.eql(u8, sp, "cancel")) {
        if (words.len < 3) {
            stubs.raise("wrong # args: should be \"after cancel id|script\"");
            return 0;
        }
        // Concatenate remaining words; either a single id or a
        // multi-word script.
        const arg = if (words.len == 3) words[2] else concat_scripts(words[2..]);
        const ar = obj.obj_ensure_string(arg);
        _ = sched.cancel_by_token(ar.ptr, ar.len);
        return obj.obj_new_string(0, 0);
    }

    // ``after info ?id?``
    if (std.mem.eql(u8, sp, "info")) {
        if (words.len == 2) return sched.info_all();
        if (words.len == 3) {
            const ar = obj.obj_ensure_string(words[2]);
            const r = sched.info_one(ar.ptr, ar.len);
            if (r == 0) {
                stubs.raise("event not found");
                return 0;
            }
            return r;
        }
        stubs.raise("wrong # args: should be \"after info ?id?\"");
        return 0;
    }

    // ``after idle script ?script ...?``
    if (std.mem.eql(u8, sp, "idle")) {
        if (words.len < 3) {
            stubs.raise("wrong # args: should be \"after idle script ?script ...?\"");
            return 0;
        }
        const body = concat_scripts(words[2..]);
        return sched.schedule_idle(body);
    }

    // First arg is ms.  Either ``after MS`` (sleep) or ``after MS script...``.
    const ms = parse_int_word(words[1]) orelse {
        stubs.raise("bad argument: must be cancel, idle, info, or an integer");
        return 0;
    };
    if (ms < 0) {
        stubs.raise("argument must be non-negative");
        return 0;
    }
    if (words.len == 2) {
        // Sleep form — drain any due events while we wait so a
        // concurrent timer scheduled to fire during the sleep
        // doesn't starve.  Implementation: yield to the scheduler's
        // tick loop with a deadline.
        sched.sleep_ms(ms);
        return obj.obj_new_string(0, 0);
    }
    const body = concat_scripts(words[2..]);
    return sched.schedule_after(ms, body);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "after", .arity_min = 1, .arity_max = null, .handler = &eval_after },
};
