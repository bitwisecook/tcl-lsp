// ``fileevent`` — registration round-trip only (Stage 1 scope).
//
// Real fd-readiness dispatch lands in Stage 3 once the channel layer
// exposes WASI fd numbers.  Until then this command simply records
// (channel, kind) → script mappings and lets ``fileevent chan kind``
// query them — enough for tcltest to register / deregister handlers
// without trapping.
//
//     fileevent chan readable ?script?
//     fileevent chan writable ?script?
//
// Setting an empty script clears the handler.

const std = @import("std");
const reg = @import("../dispatch/tcl_cmd_registry.zig");
const obj = @import("../valtypes/tcl_obj.zig");
const sched = @import("../sched/tcl_sched.zig");
const stubs = @import("../stubs/tcl_stubs.zig");

fn eval_fileevent(words: []const i32) i32 {
    if (words.len < 3 or words.len > 4) {
        stubs.raise("wrong # args: should be \"fileevent channelId event ?script?\"");
        return 0;
    }
    const chan = obj.obj_ensure_string(words[1]);
    const ev = obj.obj_ensure_string(words[2]);
    const ev_s: []const u8 = if (ev.ptr == 0) "" else
        @as([*]const u8, @ptrFromInt(ev.ptr))[0..ev.len];
    const is_read = std.mem.eql(u8, ev_s, "readable");
    const is_write = std.mem.eql(u8, ev_s, "writable");
    if (!is_read and !is_write) {
        stubs.raise("bad event: must be readable or writable");
        return 0;
    }
    if (words.len == 3) {
        // Query form.  The dispatcher contract is "+1 for caller" on
        // command results; the scheduler returns its stored handle
        // without bumping refcount, so retain before returning to
        // avoid the caller's eventual release dropping the handle
        // out from under the still-registered fileevent table.
        const cur = if (is_read)
            sched.fileevent_get_readable(chan.ptr, chan.len)
        else
            sched.fileevent_get_writable(chan.ptr, chan.len);
        if (cur == 0) return obj.obj_new_string(0, 0);
        obj.tcl_obj_retain(cur);
        return cur;
    }
    // Set form — empty script deregisters.
    const script = words[3];
    const ss = obj.obj_ensure_string(script);
    const effective: i32 = if (ss.len == 0) 0 else script;
    if (is_read) {
        sched.fileevent_set_readable(chan.ptr, chan.len, effective);
    } else {
        sched.fileevent_set_writable(chan.ptr, chan.len, effective);
    }
    return obj.obj_new_string(0, 0);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "fileevent", .arity_min = 2, .arity_max = 3, .handler = &eval_fileevent },
};
