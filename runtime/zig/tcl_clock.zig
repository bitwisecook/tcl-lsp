// Clock helpers: clock seconds / clock clicks — expose WASI's
// clock_time_get as TclObj-returning functions.
//
// Both subcommands return integers:
//   clock seconds → Unix epoch seconds (truncated from nanoseconds)
//   clock clicks  → monotonic counter, in microseconds for Tcl
//                   compatibility (``clock clicks -microseconds``)
//
// Full ``clock format``/``clock scan`` are not attempted here — those
// depend on a timezone DB that's impractical to ship in WASM.  Scripts
// that need formatted output should fall back to the interpreter.

const std = @import("std");
const obj = @import("tcl_obj.zig");
const obj_new_int = obj.obj_new_int;

const NS_PER_SECOND: i64 = 1_000_000_000;
const NS_PER_USEC: i64 = 1_000;

fn clock_ns(clock_id: std.os.wasi.clockid_t) i64 {
    var ts: std.os.wasi.timestamp_t = 0;
    // Request nanosecond precision (last arg) — WASI's contract is
    // "best effort" so the returned timestamp may be coarser, which
    // is fine for seconds/clicks granularity.
    _ = std.os.wasi.clock_time_get(clock_id, 1, &ts);
    return @intCast(@as(i64, @bitCast(ts)));
}

/// clock seconds — Unix epoch seconds.  Returns a boxed integer.
pub export fn clock_seconds() i32 {
    const ns = clock_ns(.REALTIME);
    return obj_new_int(@divTrunc(ns, NS_PER_SECOND));
}

/// clock clicks — monotonic microseconds.  Returns a boxed integer.
/// Matches Tcl's ``clock clicks -microseconds``; plain ``clock clicks``
/// is documented to have "system-dependent granularity", microseconds
/// is a safe unit to always provide.
pub export fn clock_clicks() i32 {
    const ns = clock_ns(.MONOTONIC);
    return obj_new_int(@divTrunc(ns, NS_PER_USEC));
}

/// clock milliseconds — monotonic milliseconds.  Not part of the
/// CAT1 list but free once ``clock_ns`` is written.
pub export fn clock_milliseconds() i32 {
    const ns = clock_ns(.MONOTONIC);
    return obj_new_int(@divTrunc(ns, 1_000_000));
}
