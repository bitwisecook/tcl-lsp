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

// ``if``, ``while``, ``for``, ``foreach`` — loop and conditional commands.
// All implementations live in tcl_interp.zig (pub fn); this file registers them.

const reg = @import("../dispatch/tcl_cmd_registry.zig");

const result_mod = @import("../interp/tcl_result.zig");
fn eval_if(words: []const i32) result_mod.InterpResult {
    const interp = @import("../interp/tcl_interp.zig");
    return result_mod.from_globals(interp.eval_if(words));
}

fn eval_while(words: []const i32) result_mod.InterpResult {
    const interp = @import("../interp/tcl_interp.zig");
    return result_mod.from_globals(interp.eval_while(words));
}

fn eval_for(words: []const i32) result_mod.InterpResult {
    const interp = @import("../interp/tcl_interp.zig");
    return result_mod.from_globals(interp.eval_for(words));
}

fn eval_foreach(words: []const i32) result_mod.InterpResult {
    const interp = @import("../interp/tcl_interp.zig");
    return result_mod.from_globals(interp.eval_foreach(words));
}

fn eval_switch(words: []const i32) result_mod.InterpResult {
    const interp = @import("../interp/tcl_interp.zig");
    return result_mod.from_globals(interp.eval_switch(words));
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "if", .arity_min = 2, .arity_max = null, .handler = &eval_if },
    .{ .name = "while", .arity_min = 2, .arity_max = 2, .handler = &eval_while },
    .{ .name = "for", .arity_min = 4, .arity_max = 4, .handler = &eval_for },
    .{ .name = "foreach", .arity_min = 3, .arity_max = null, .handler = &eval_foreach },
    // ``switch`` is also handled inline by the codegen (IRSwitch),
    // so this entry only fires when the interpreter evaluates a
    // dynamic ``switch`` string (e.g. test bodies that build
    // pattern/body lists at runtime, or a ``switch`` reached via
    // ``eval``).  Removed from the ``STUB_TRAP`` table now.
    .{ .name = "switch", .arity_min = 2, .arity_max = null, .handler = &eval_switch },
};
