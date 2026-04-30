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
const obj = @import("../valtypes/tcl_obj.zig");
const tz = @import("tcl_tz.zig");
const obj_new_int = obj.obj_new_int;
const obj_new_string = obj.obj_new_string;
const obj_ensure_string = obj.obj_ensure_string;
const alloc = obj.alloc;

comptime {
    // Force the linker to keep tcl_tz.zig's symbols even though
    // nothing calls into them yet — the resolver wave (next commit)
    // hooks them up.  Without this comptime tickle the runtime
    // build would happily drop the entire module.
    _ = &tz.resolve;
    _ = &tz.resolve_default;
    _ = &tz.utc;
}

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

// -- clock format -------------------------------------------------------------

/// Broken-down UTC time computed from Unix epoch seconds.  Computed
/// without relying on a runtime timezone DB — the algorithm uses the
/// proleptic Gregorian calendar.
const BrokenDown = struct {
    year: i32,
    month: u32, // 1..12
    day: u32, // 1..31
    hour: u32,
    minute: u32,
    second: u32,
    weekday: u32, // 0=Sunday .. 6=Saturday
    yday: u32, // 1..366
};

const MONTH_DAYS_NORMAL = [_]u32{ 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31 };
const MONTH_DAYS_LEAP = [_]u32{ 31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31 };

fn is_leap(year: i32) bool {
    return @mod(year, 4) == 0 and (@mod(year, 100) != 0 or @mod(year, 400) == 0);
}

fn break_down(epoch_secs: i64) BrokenDown {
    // Day-of-week for 1970-01-01 is Thursday = 4.
    const SECS_PER_DAY: i64 = 86400;
    const days_since_epoch: i64 = @divFloor(epoch_secs, SECS_PER_DAY);
    const time_of_day_secs: i64 = epoch_secs - days_since_epoch * SECS_PER_DAY;
    const weekday: u32 = @intCast(@mod(days_since_epoch + 4, 7));
    var year: i32 = 1970;
    var d: i64 = days_since_epoch;
    while (true) {
        const days_in_year: i32 = if (is_leap(year)) 366 else 365;
        if (d < 0) {
            year -= 1;
            d += @as(i64, if (is_leap(year)) 366 else 365);
        } else if (d >= days_in_year) {
            d -= days_in_year;
            year += 1;
        } else break;
    }
    const yday: u32 = @intCast(d + 1);
    const months = if (is_leap(year)) MONTH_DAYS_LEAP else MONTH_DAYS_NORMAL;
    var month: u32 = 0;
    var rem: u32 = @intCast(d);
    while (month < 12 and rem >= months[month]) : (month += 1) {
        rem -= months[month];
    }
    return .{
        .year = year,
        .month = month + 1,
        .day = rem + 1,
        .hour = @intCast(@divFloor(time_of_day_secs, 3600)),
        .minute = @intCast(@mod(@divFloor(time_of_day_secs, 60), 60)),
        .second = @intCast(@mod(time_of_day_secs, 60)),
        .weekday = weekday,
        .yday = yday,
    };
}

const WEEKDAY_FULL = [_][]const u8{
    "Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday",
};
const WEEKDAY_ABBR = [_][]const u8{ "Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat" };
const MONTH_FULL = [_][]const u8{
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December",
};
const MONTH_ABBR = [_][]const u8{
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
};

fn write_pad_int(out: [*]u8, off: u32, value: u32, width: u32, pad: u8) u32 {
    var buf: [16]u8 = undefined;
    var n: u32 = 0;
    var v = value;
    if (v == 0) {
        buf[0] = '0';
        n = 1;
    } else {
        while (v > 0) : (n += 1) {
            buf[n] = @intCast('0' + v % 10);
            v /= 10;
        }
    }
    var o = off;
    while (n < width) : (n += 1) {
        @as([*]u8, @ptrFromInt(@intFromPtr(out) + o))[0] = pad;
        o += 1;
    }
    var i = n;
    while (i > 0) {
        i -= 1;
        @as([*]u8, @ptrFromInt(@intFromPtr(out) + o))[0] = buf[i];
        o += 1;
    }
    return o;
}

fn write_str(out: [*]u8, off: u32, s: []const u8) u32 {
    var o = off;
    for (s) |b| {
        @as([*]u8, @ptrFromInt(@intFromPtr(out) + o))[0] = b;
        o += 1;
    }
    return o;
}

/// clock_format — format Unix epoch seconds via a strftime-like
/// pattern.  Always uses UTC (no timezone DB shipped in WASM).
/// Supports the subset of conversion specs Tcl test suites and
/// the project samples use:
///   %Y %y %m %d %H %M %S %j %A %a %B %b %e %p %u %w %z
/// Plus literal ``%%`` and pass-through of non-format bytes.
/// Unknown specs are emitted verbatim (``%X`` -> ``%X``) so no
/// trap regardless of what the caller wrote.
pub export fn clock_format(seconds_obj: i32, fmt_obj: i32) i32 {
    const default_fmt = "%a %b %e %H:%M:%S %z %Y";
    var fmt_ptr: u32 = undefined;
    var fmt_len: u32 = undefined;
    if (fmt_obj == 0) {
        fmt_ptr = @intCast(@intFromPtr(default_fmt.ptr));
        fmt_len = default_fmt.len;
    } else {
        const f = obj_ensure_string(fmt_obj);
        fmt_ptr = f.ptr;
        fmt_len = f.len;
    }
    const epoch: i64 = if (seconds_obj == 0) 0 else obj.obj_get_int(seconds_obj);
    const t = break_down(epoch);
    const fp: [*]const u8 = @ptrFromInt(fmt_ptr);
    // Generously sized buffer — 8x format length covers the worst
    // case where every %x expands to a multi-byte month/weekday name.
    const cap: u32 = fmt_len * 8 + 32;
    const buf = alloc(cap);
    if (buf == 0) return obj_new_string(0, 0);
    const out: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    var i: u32 = 0;
    while (i < fmt_len) {
        const c = fp[i];
        if (c != '%') {
            out[off] = c;
            off += 1;
            i += 1;
            continue;
        }
        i += 1;
        if (i >= fmt_len) {
            out[off] = '%';
            off += 1;
            break;
        }
        const spec = fp[i];
        i += 1;
        switch (spec) {
            '%' => { out[off] = '%'; off += 1; },
            'Y' => off = write_pad_int(out, off, @intCast(t.year), 4, '0'),
            'y' => off = write_pad_int(out, off, @intCast(@mod(t.year, 100)), 2, '0'),
            'm' => off = write_pad_int(out, off, t.month, 2, '0'),
            'd' => off = write_pad_int(out, off, t.day, 2, '0'),
            'e' => off = write_pad_int(out, off, t.day, 2, ' '),
            'H' => off = write_pad_int(out, off, t.hour, 2, '0'),
            'M' => off = write_pad_int(out, off, t.minute, 2, '0'),
            'S' => off = write_pad_int(out, off, t.second, 2, '0'),
            'j' => off = write_pad_int(out, off, t.yday, 3, '0'),
            'u' => off = write_pad_int(out, off, if (t.weekday == 0) 7 else t.weekday, 1, '0'),
            'w' => off = write_pad_int(out, off, t.weekday, 1, '0'),
            'A' => off = write_str(out, off, WEEKDAY_FULL[t.weekday]),
            'a' => off = write_str(out, off, WEEKDAY_ABBR[t.weekday]),
            'B' => off = write_str(out, off, MONTH_FULL[t.month - 1]),
            'b' => off = write_str(out, off, MONTH_ABBR[t.month - 1]),
            'p' => off = write_str(out, off, if (t.hour < 12) "AM" else "PM"),
            'z' => off = write_str(out, off, "+0000"),
            'I' => {
                var hr12 = t.hour % 12;
                if (hr12 == 0) hr12 = 12;
                off = write_pad_int(out, off, hr12, 2, '0');
            },
            else => {
                out[off] = '%';
                off += 1;
                out[off] = spec;
                off += 1;
            },
        }
    }
    // Claim ownership of the output buffer via OBJ_STR_CAP so an
    // eventual ``tcl_obj_release`` reclaims it via ``free_sized``.
    // Without this, ``cap`` is treated as 0 (non-owning) and the
    // ``cap`` bytes leak on every call — pathological under
    // long-running ``clock format`` loops in test bodies.
    const out_obj = obj_new_string(@intCast(buf), @intCast(off));
    if (out_obj == 0) {
        obj.free_sized(buf, cap);
        return 0;
    }
    obj.write_i32(@as(u32, @intCast(out_obj)) + obj.OBJ_STR_CAP, @bitCast(cap));
    return out_obj;
}
