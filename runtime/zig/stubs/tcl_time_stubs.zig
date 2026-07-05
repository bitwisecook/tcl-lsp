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

// Time + async-event stubs.  ``clock format`` / ``clock scan`` /
// ``clock add`` require a full timezone database we don't ship.
// Rather than trapping, they return a safe zero-value: ``clock format``
// returns the literal string "0" and ``clock scan`` returns integer 0.
// This lets code like
//
//   set dayStart [clock scan [clock format $t -format 00:00]]
//
// complete without error (dayStart = 0) instead of raising
// "unsupported command: clock format".  The values are incorrect for
// any real timezone-aware use, but they prevent ``counter::init
// -timehist`` (and similar callers) from aborting with code 1.
//
// DELIBERATE SEMANTIC DIVERGENCE FROM TCL 8.4-9.0: real Tcl raises
// an error when the timezone database isn't available; we silently
// return 0 / "0".  Callers that format dates and parse them back get
// epoch-zero (1970-01-01 00:00:00) instead of the intended value.
// This trades correctness for bootability — the counter.tcl tcllib
// module and any other code that only uses ``clock format`` +
// ``clock scan`` as an opaque round-trip will behave plausibly, but
// any user actually reading the formatted output will see wrong dates.
// Track real TZ-aware ``clock`` as a separate KCS work item.
//
// The event-loop commands (``after`` with ms, ``vwait``, ``update``,
// ``coroutine``, ``yield``, ``yieldto``) need a scheduler/event loop
// that has no meaningful implementation inside a single WASM
// module, so they remain stubs but are handled as no-ops by the
// cmd_table (stubs_.zig) before reaching here.

const obj = @import("../valtypes/tcl_obj.zig");
const stubs = @import("tcl_stubs.zig");

// ``clock_format`` is now implemented in ``io/tcl_clock.zig`` with
// a real (UTC-only, no timezone DB) strftime-style formatter.
// Removed from this stubs file to avoid a duplicate symbol export.

pub export fn clock_scan(text: i32, opts: i32) i32 {
    _ = text;
    _ = opts;
    return obj.obj_new_int(0);
}

pub export fn clock_add(base: i32, opts: i32) i32 {
    _ = base;
    _ = opts;
    return obj.obj_new_int(0);
}

pub export fn tcl_cmd_after(ms: i32) i32 {
    _ = ms;
    // No event loop in WASM — silently succeed with an empty TclObj.
    //
    // Returning raw ``0`` breaks callers like ``set ::x [after 1]``
    // because the runtime treats obj==0 as "unset/null" (see
    // ``global_exists`` check ``val != 0``).  Match the interpreter
    // stub path in ``cmds/stubs.zig::eval_after`` which returns
    // ``obj_new_string(0, 0)`` — a real empty-string TclObj.
    return obj.obj_new_string(0, 0);
}

pub export fn tcl_cmd_vwait(var_name: i32) i32 {
    _ = var_name;
    stubs.unsupported("vwait");
    return 0;
}

pub export fn tcl_cmd_update() i32 {
    stubs.unsupported("update");
    return 0;
}

pub export fn tcl_cmd_coroutine(name: i32, body: i32) i32 {
    _ = name;
    _ = body;
    stubs.unsupported("coroutine");
    return 0;
}

pub export fn tcl_cmd_yield(value: i32) i32 {
    _ = value;
    stubs.unsupported("yield");
    return 0;
}

pub export fn tcl_cmd_yieldto(cmd: i32) i32 {
    _ = cmd;
    stubs.unsupported("yieldto");
    return 0;
}
