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

/// Render a numeric ``%z`` offset (``±HHMM``) into ``out``.
fn write_offset(out: [*]u8, off_in: u32, utoff_secs: i32) u32 {
    var off = off_in;
    const sign: u8 = if (utoff_secs < 0) '-' else '+';
    const abs: u32 = if (utoff_secs < 0)
        @intCast(-@as(i32, utoff_secs))
    else
        @intCast(utoff_secs);
    const hh = abs / 3600;
    const mm = (abs % 3600) / 60;
    out[off] = sign;
    off += 1;
    off = write_pad_int(out, off, hh, 2, '0');
    off = write_pad_int(out, off, mm, 2, '0');
    return off;
}

/// Strftime over a caller-supplied broken-down time + offset/abbr.
/// Shared between ``clock_format`` (the WASM-export entry, default
/// UTC) and ``clock_format_tz`` (timezone-aware via the resolver).
fn render_format(
    fmt_ptr: u32,
    fmt_len: u32,
    t: BrokenDown,
    utoff: i32,
    abbr: []const u8,
) i32 {
    const fp: [*]const u8 = @ptrFromInt(fmt_ptr);
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
            '%' => {
                out[off] = '%';
                off += 1;
            },
            'Y' => off = write_pad_int(out, off, @intCast(t.year), 4, '0'),
            'y' => off = write_pad_int(out, off, @intCast(@mod(t.year, 100)), 2, '0'),
            'm' => off = write_pad_int(out, off, t.month, 2, '0'),
            'd' => off = write_pad_int(out, off, t.day, 2, '0'),
            'e' => off = write_pad_int(out, off, t.day, 2, ' '),
            'H' => off = write_pad_int(out, off, t.hour, 2, '0'),
            'k' => off = write_pad_int(out, off, t.hour, 2, ' '),
            'M' => off = write_pad_int(out, off, t.minute, 2, '0'),
            'S' => off = write_pad_int(out, off, t.second, 2, '0'),
            'j' => off = write_pad_int(out, off, t.yday, 3, '0'),
            'u' => off = write_pad_int(out, off, if (t.weekday == 0) 7 else t.weekday, 1, '0'),
            'w' => off = write_pad_int(out, off, t.weekday, 1, '0'),
            'A' => off = write_str(out, off, WEEKDAY_FULL[t.weekday]),
            'a' => off = write_str(out, off, WEEKDAY_ABBR[t.weekday]),
            'B' => off = write_str(out, off, MONTH_FULL[t.month - 1]),
            'b', 'h' => off = write_str(out, off, MONTH_ABBR[t.month - 1]),
            'p' => off = write_str(out, off, if (t.hour < 12) "AM" else "PM"),
            'P' => off = write_str(out, off, if (t.hour < 12) "am" else "pm"),
            'z' => off = write_offset(out, off, utoff),
            'Z' => off = write_str(out, off, abbr),
            'n' => {
                out[off] = '\n';
                off += 1;
            },
            't' => {
                out[off] = '\t';
                off += 1;
            },
            'I' => {
                var hr12 = t.hour % 12;
                if (hr12 == 0) hr12 = 12;
                off = write_pad_int(out, off, hr12, 2, '0');
            },
            'l' => {
                var hr12 = t.hour % 12;
                if (hr12 == 0) hr12 = 12;
                off = write_pad_int(out, off, hr12, 2, ' ');
            },
            'D' => {
                // %m/%d/%y
                off = write_pad_int(out, off, t.month, 2, '0');
                out[off] = '/';
                off += 1;
                off = write_pad_int(out, off, t.day, 2, '0');
                out[off] = '/';
                off += 1;
                off = write_pad_int(out, off, @intCast(@mod(t.year, 100)), 2, '0');
            },
            'R' => {
                // %H:%M
                off = write_pad_int(out, off, t.hour, 2, '0');
                out[off] = ':';
                off += 1;
                off = write_pad_int(out, off, t.minute, 2, '0');
            },
            'T' => {
                // %H:%M:%S
                off = write_pad_int(out, off, t.hour, 2, '0');
                out[off] = ':';
                off += 1;
                off = write_pad_int(out, off, t.minute, 2, '0');
                out[off] = ':';
                off += 1;
                off = write_pad_int(out, off, t.second, 2, '0');
            },
            'F' => {
                // %Y-%m-%d
                off = write_pad_int(out, off, @intCast(t.year), 4, '0');
                out[off] = '-';
                off += 1;
                off = write_pad_int(out, off, t.month, 2, '0');
                out[off] = '-';
                off += 1;
                off = write_pad_int(out, off, t.day, 2, '0');
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

/// Helper: pull (ptr, len) out of a TclObj, falling back to the
/// supplied default when the obj is null / empty.
fn fmt_or_default(fmt_obj: i32, default_fmt: []const u8) struct { ptr: u32, len: u32 } {
    if (fmt_obj == 0) {
        return .{
            .ptr = @intCast(@intFromPtr(default_fmt.ptr)),
            .len = default_fmt.len,
        };
    }
    const f = obj_ensure_string(fmt_obj);
    if (f.len == 0) {
        return .{
            .ptr = @intCast(@intFromPtr(default_fmt.ptr)),
            .len = default_fmt.len,
        };
    }
    return .{ .ptr = f.ptr, .len = f.len };
}

/// clock_format — format Unix epoch seconds via a strftime-like
/// pattern.  Backwards-compatible UTC-only entry kept so existing
/// compiled imports (``WasmRuntimeImport(export_name="clock_format")``)
/// keep working.  New callers should prefer
/// :func:`clock_format_tz` which honours ``-gmt`` / ``-timezone``.
pub export fn clock_format(seconds_obj: i32, fmt_obj: i32) i32 {
    const default_fmt = "%a %b %e %H:%M:%S %z %Y";
    const f = fmt_or_default(fmt_obj, default_fmt);
    const epoch: i64 = if (seconds_obj == 0) 0 else obj.obj_get_int(seconds_obj);
    const t = break_down(epoch);
    return render_format(f.ptr, f.len, t, 0, "UTC");
}

/// clock_format_tz — timezone-aware ``clock format``.  ``zone_obj``
/// is a TclObj holding the zone name (``UTC``, ``:America/New_York``,
/// …); when it's zero / empty the resolver falls through to
/// ``$TZ`` / ``/etc/localtime`` / UTC, in that order.
pub export fn clock_format_tz(
    seconds_obj: i32,
    fmt_obj: i32,
    zone_obj: i32,
) i32 {
    const default_fmt = "%a %b %e %H:%M:%S %Z %Y";
    const f = fmt_or_default(fmt_obj, default_fmt);
    const epoch: i64 = if (seconds_obj == 0) 0 else obj.obj_get_int(seconds_obj);
    const zone_slice: []const u8 = blk: {
        if (zone_obj == 0) break :blk &[_]u8{};
        const s = obj_ensure_string(zone_obj);
        if (s.ptr == 0 or s.len == 0) break :blk &[_]u8{};
        const p: [*]const u8 = @ptrFromInt(s.ptr);
        break :blk p[0..s.len];
    };
    const z: *const tz.TimeZone = if (zone_slice.len == 0)
        tz.resolve_default()
    else
        tz.resolve(zone_slice);
    const off_info = z.offset_at(epoch);
    const local = break_down(epoch + @as(i64, off_info.utoff));
    return render_format(f.ptr, f.len, local, off_info.utoff, off_info.abbr);
}

