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
// The event-loop commands (``after`` with ms, ``vwait``, ``update``,
// ``coroutine``, ``yield``, ``yieldto``) need a scheduler/event loop
// that has no meaningful implementation inside a single WASM
// module, so they remain stubs but are handled as no-ops by the
// cmd_table (stubs_.zig) before reaching here.

const obj = @import("tcl_obj.zig");

pub export fn clock_format(seconds: i32, opts: i32) i32 {
    _ = seconds;
    _ = opts;
    // Return the string "0" — a fixed placeholder that clock_scan can
    // parse as integer 0 without erroring.
    const buf = obj.alloc(1);
    const p: [*]u8 = @ptrFromInt(buf);
    p[0] = '0';
    return obj.obj_new_string(@intCast(buf), 1);
}

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

const stubs = @import("tcl_stubs.zig");

pub export fn tcl_cmd_after(ms: i32) i32 {
    _ = ms;
    // No event loop in WASM — silently succeed.
    return 0;
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
