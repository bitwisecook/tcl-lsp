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

// ``rename``, ``interp`` — interpreter management commands.

const interp_impl = @import("./tcl_cmd_interp.zig");
const result_mod = @import("../interp/tcl_result.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");

fn eval_rename(words: []const i32) result_mod.InterpResult {
    return result_mod.from_globals(interp_impl.eval_rename(words));
}

fn eval_interp(words: []const i32) result_mod.InterpResult {
    const interp = @import("../interp/tcl_interp.zig");
    return result_mod.from_globals(interp.eval_interp(words));
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "rename", .arity_min = 2, .arity_max = 2, .handler = &eval_rename },
    .{ .name = "interp", .arity_min = 1, .arity_max = null, .handler = &eval_interp },
};

// Sub-command arities for ``interp`` — mirrors
// ``dialects/tcl/interp.py``.  ``rename`` has no
// sub-commands.  Named table so ``parse_zig_builtins`` attributes
// these to ``interp`` only, not to ``rename``.
pub const interp_subcommands: []const reg.SubEntry = &.{
    .{ .name = "alias", .arity_min = 2, .arity_max = null, .handler = &eval_interp },
    .{ .name = "aliases", .arity_min = 0, .arity_max = 1, .handler = &eval_interp },
    .{ .name = "bgerror", .arity_min = 1, .arity_max = 2, .handler = &eval_interp },
    .{ .name = "cancel", .arity_min = 0, .arity_max = 2, .handler = &eval_interp },
    .{ .name = "children", .arity_min = 0, .arity_max = 1, .handler = &eval_interp },
    .{ .name = "create", .arity_min = 0, .arity_max = 1, .handler = &eval_interp },
    .{ .name = "debug", .arity_min = 1, .arity_max = null, .handler = &eval_interp },
    .{ .name = "delete", .arity_min = 0, .arity_max = null, .handler = &eval_interp },
    .{ .name = "eval", .arity_min = 2, .arity_max = null, .handler = &eval_interp },
    .{ .name = "exists", .arity_min = 1, .arity_max = 1, .handler = &eval_interp },
    .{ .name = "expose", .arity_min = 2, .arity_max = 3, .handler = &eval_interp },
    .{ .name = "hidden", .arity_min = 1, .arity_max = 1, .handler = &eval_interp },
    .{ .name = "hide", .arity_min = 2, .arity_max = 3, .handler = &eval_interp },
    .{ .name = "invokehidden", .arity_min = 2, .arity_max = null, .handler = &eval_interp },
    .{ .name = "issafe", .arity_min = 0, .arity_max = 1, .handler = &eval_interp },
    .{ .name = "limit", .arity_min = 2, .arity_max = null, .handler = &eval_interp },
    .{ .name = "marktrusted", .arity_min = 1, .arity_max = 1, .handler = &eval_interp },
    .{ .name = "recursionlimit", .arity_min = 1, .arity_max = 2, .handler = &eval_interp },
    .{ .name = "share", .arity_min = 3, .arity_max = 3, .handler = &eval_interp },
    .{ .name = "slaves", .arity_min = 0, .arity_max = 1, .handler = &eval_interp },
    .{ .name = "target", .arity_min = 2, .arity_max = 2, .handler = &eval_interp },
    .{ .name = "transfer", .arity_min = 3, .arity_max = 3, .handler = &eval_interp },
};
