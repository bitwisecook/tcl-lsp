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

// ``vwait`` / ``update`` — event-loop driver commands.
//
//   ``vwait varName``         — block until ``varName`` is written.
//   ``update``                — drain all currently-ready events.
//   ``update idletasks``      — drain only the idle queue.

const std = @import("std");
const result_mod = @import("../interp/tcl_result.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");
const obj = @import("../valtypes/tcl_obj.zig");
const sched = @import("../sched/tcl_sched.zig");
const stubs = @import("../stubs/tcl_stubs.zig");

fn eval_vwait(words: []const i32) result_mod.InterpResult {
    if (words.len != 2) {
        stubs.raise("wrong # args: should be \"vwait name\"");
        return result_mod.from_globals(0);
    }
    const n = obj.obj_ensure_string(words[1]);
    const status = sched.vwait_drain(n.ptr, n.len);
    if (status == -1) {
        stubs.raise("vwait nesting too deep");
        return result_mod.from_globals(0);
    }
    if (status == -2) {
        stubs.raise("can't wait for variable: would wait forever");
        return result_mod.from_globals(0);
    }
    return result_mod.from_globals(obj.obj_new_string(0, 0));
}

fn eval_update(words: []const i32) result_mod.InterpResult {
    if (words.len > 2) {
        stubs.raise("wrong # args: should be \"update ?idletasks?\"");
        return result_mod.from_globals(0);
    }
    if (words.len == 2) {
        const sub = obj.obj_ensure_string(words[1]);
        const sp: []const u8 = if (sub.ptr == 0) "" else @as([*]const u8, @ptrFromInt(sub.ptr))[0..sub.len];
        if (std.mem.eql(u8, sp, "idletasks")) {
            _ = sched.drain_idle();
            return result_mod.from_globals(obj.obj_new_string(0, 0));
        }
        stubs.raise("bad option: must be idletasks");
        return result_mod.from_globals(0);
    }
    _ = sched.drain_all();
    return result_mod.from_globals(obj.obj_new_string(0, 0));
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "vwait", .arity_min = 1, .arity_max = 1, .handler = &eval_vwait },
    .{ .name = "update", .arity_min = 0, .arity_max = 1, .handler = &eval_update },
};
