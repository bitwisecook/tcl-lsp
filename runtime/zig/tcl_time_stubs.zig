// Time + async-event stubs.  ``clock format`` / ``clock scan`` /
// ``clock add`` require a full timezone database we don't ship, so
// they raise ``unsupported command: clock <sub>``.  The *arithmetic*
// clock commands (``clock seconds`` / ``clock clicks`` /
// ``clock milliseconds``) are implemented in ``tcl_clock.zig``.
//
// The event-loop commands (``after`` with ms, ``vwait``, ``update``,
// ``coroutine``, ``yield``, ``yieldto``) need a scheduler/event loop
// that has no meaningful implementation inside a single WASM
// module, so they trap too.

const stubs = @import("tcl_stubs.zig");

pub export fn clock_format(seconds: i32, opts: i32) i32 {
    _ = seconds;
    _ = opts;
    stubs.unsupported_sub("clock", "format");
    return 0;
}

pub export fn clock_scan(text: i32, opts: i32) i32 {
    _ = text;
    _ = opts;
    stubs.unsupported_sub("clock", "scan");
    return 0;
}

pub export fn clock_add(base: i32, opts: i32) i32 {
    _ = base;
    _ = opts;
    stubs.unsupported_sub("clock", "add");
    return 0;
}

pub export fn after(ms: i32) i32 {
    _ = ms;
    stubs.unsupported("after");
    return 0;
}

pub export fn vwait(var_name: i32) i32 {
    _ = var_name;
    stubs.unsupported("vwait");
    return 0;
}

pub export fn update() i32 {
    stubs.unsupported("update");
    return 0;
}

pub export fn coroutine(name: i32, body: i32) i32 {
    _ = name;
    _ = body;
    stubs.unsupported("coroutine");
    return 0;
}

pub export fn yield(value: i32) i32 {
    _ = value;
    stubs.unsupported("yield");
    return 0;
}

pub export fn yieldto(cmd: i32) i32 {
    _ = cmd;
    stubs.unsupported("yieldto");
    return 0;
}
