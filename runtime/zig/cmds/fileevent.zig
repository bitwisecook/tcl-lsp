// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

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
const result_mod = @import("../interp/tcl_result.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");
const obj = @import("../valtypes/tcl_obj.zig");
const sched = @import("../sched/tcl_sched.zig");
const stubs = @import("../stubs/tcl_stubs.zig");

fn eval_fileevent(words: []const i32) result_mod.InterpResult {
    if (words.len < 3 or words.len > 4) {
        stubs.raise("wrong # args: should be \"fileevent channelId event ?script?\"");
        return result_mod.from_globals(0);
    }
    const chan = obj.obj_ensure_string(words[1]);
    const ev = obj.obj_ensure_string(words[2]);
    const ev_s: []const u8 = if (ev.ptr == 0) "" else @as([*]const u8, @ptrFromInt(ev.ptr))[0..ev.len];
    const is_read = std.mem.eql(u8, ev_s, "readable");
    const is_write = std.mem.eql(u8, ev_s, "writable");
    if (!is_read and !is_write) {
        stubs.raise("bad event: must be readable or writable");
        return result_mod.from_globals(0);
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
        if (cur == 0) return result_mod.from_globals(obj.obj_new_string(0, 0));
        obj.tcl_obj_retain(cur);
        return result_mod.from_globals(cur);
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
    return result_mod.from_globals(obj.obj_new_string(0, 0));
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "fileevent", .arity_min = 2, .arity_max = 3, .handler = &eval_fileevent },
};
