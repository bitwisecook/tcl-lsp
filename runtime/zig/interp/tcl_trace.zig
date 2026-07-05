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

// Legacy ``trace`` fallback for execution-trace sub-commands.
//
// Variable traces (``trace add variable`` / ``trace remove variable``
// / ``trace info variable``, plus the legacy ``trace variable`` /
// ``trace vdelete`` / ``trace vinfo`` forms) are now implemented in
// ``interp/tcl_var_trace.zig`` and dispatched directly from
// ``cmds/inspect.zig::eval_trace``.  Both global / namespace var
// traces (directory-keyed) and proc-local traces (per-frame
// ``frame_trace_heads`` chain) live there.
//
// This module remains the catch-all for the *other* ``trace``
// sub-commands the variable-trace registry doesn't handle:
// ``trace add command`` / ``trace add execution`` / ``trace remove
// command`` / ``trace remove execution`` / ``trace info command`` /
// ``trace info execution``.  Those are command-introspection /
// execution-tracing surfaces that would need a separate hook into
// the dispatcher; until that lands the pass-through here treats
// the operation as benign (``add`` / ``remove`` silently drop the
// callback; ``info`` raises ``unsupported``).
//
// ``tcltest`` and ``tcllib`` lean on variable traces (now real) for
// lazy configuration; execution traces are rarer and the
// pass-through is enough to keep test harnesses loading without
// crashing.

const obj = @import("../valtypes/tcl_obj.zig");
const stubs = @import("../stubs/tcl_stubs.zig");

const obj_ensure_string = obj.obj_ensure_string;
const obj_new_string = obj.obj_new_string;

fn eq(a: [*]const u8, alen: u32, literal: []const u8) bool {
    if (alen != literal.len) return false;
    for (0..literal.len) |i| if (a[i] != literal[i]) return false;
    return true;
}

/// ``trace <sub> ?args…?`` — pass-through add/remove, unsupported
/// for info / variable (legacy) / execution queries.
pub export fn tcl_cmd_trace_cmd(sub: i32, arg: i32) i32 {
    _ = arg;
    if (sub == 0) {
        stubs.unsupported("trace (missing subcommand)");
        return 0;
    }
    const s = obj_ensure_string(sub);
    if (s.len == 0) {
        stubs.unsupported("trace (empty subcommand)");
        return 0;
    }
    const sp: [*]const u8 = @ptrFromInt(s.ptr);
    // Accepted (pass-through, NOP).
    if (eq(sp, s.len, "add") or eq(sp, s.len, "remove")) {
        return obj_new_string(0, 0);
    }
    // Legacy forms also accepted quietly.
    if (eq(sp, s.len, "variable") or
        eq(sp, s.len, "vdelete") or
        eq(sp, s.len, "vinfo"))
    {
        return obj_new_string(0, 0);
    }
    // Info / query — we have nothing useful to return.
    if (eq(sp, s.len, "info")) {
        stubs.unsupported_sub("trace", "info");
        return 0;
    }
    stubs.unsupported_sub("trace", "<unknown-subcmd>");
    return 0;
}
