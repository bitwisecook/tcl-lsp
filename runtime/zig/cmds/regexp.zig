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

// ``regexp`` — regular expression match / capture command.
//
// ``regsub`` lives in its own module (``regsub.zig``) so the BUILTINS
// table only sees it once.  Both modules delegate to the canonical
// impl in ``valtypes/tcl_regex.zig``.

const regex_mod = @import("../valtypes/tcl_regex.zig");
const result_mod = @import("../interp/tcl_result.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");

fn eval_regexp(words: []const i32) result_mod.InterpResult {
    return result_mod.from_globals(regex_mod.eval_regexp_cmd(words));
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "regexp", .arity_min = 1, .arity_max = null, .handler = &eval_regexp },
};
