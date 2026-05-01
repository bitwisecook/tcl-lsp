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

pub fn clock_ns(clock_id: std.os.wasi.clockid_t) i64 {
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
    // Emit pad characters first.  ``n`` is the digit count and must
    // stay frozen here — the emit loop below uses it as the bound
    // for buf indexing, and an earlier version of this function
    // bumped ``n`` inside the pad loop, which made the emit phase
    // read uninitialised buf slots and emit garbage bytes alongside
    // the real digits (visible as ``0X1`` in formatted ISO dates).
    var pad_remaining: u32 = if (n < width) width - n else 0;
    while (pad_remaining > 0) : (pad_remaining -= 1) {
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

/// Render a signed integer as a left-zero-padded decimal, prefixed
/// with ``-`` for negative values.  Used for ``%Y`` so ancient
/// (pre-year-1) dates don't panic when ``t.year`` is negative — the
/// tcltest clock-2.* coverage exercises epochs as far back as
/// ``-62135769601`` (year ~1 AD/BC) which used to trip the
/// ``@intCast(i32 → u32)`` safety check.
fn write_pad_signed(out: [*]u8, off_in: u32, value: i64, width: u32) u32 {
    if (value >= 0) {
        return write_pad_int(out, off_in, @intCast(value), width, '0');
    }
    var off = off_in;
    out[off] = '-';
    off += 1;
    const mag: u64 = @intCast(-value);
    return write_pad_int(out, off, @intCast(mag), if (width > 0) width - 1 else 0, '0');
}

/// Render epoch seconds for ``%s``.  Computed by walking the
/// broken-down time back to a Unix epoch (UTC) — convenient because
/// ``render_format`` already has the local-time tuple plus the
/// caller's offset, so reversing is just a pack + offset adjust.
fn render_epoch(t: BrokenDown, utoff: i32) i64 {
    return pack_epoch(t.year, t.month, t.day, t.hour, t.minute, t.second) - @as(i64, utoff);
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
            'Y' => off = write_pad_signed(out, off, @intCast(t.year), 4),
            'y' => {
                // Mathematical mod (Zig ``@mod``) returns non-negative
                // for negative inputs, so casting through u32 is safe.
                const y2: u32 = @intCast(@mod(@as(i64, t.year), 100));
                off = write_pad_int(out, off, y2, 2, '0');
            },
            's' => off = write_pad_signed(out, off, render_epoch(t, utoff), 0),
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
                // %Y-%m-%d.  Year goes through ``write_pad_signed``
                // for the same negative-year reason as the standalone
                // ``%Y`` spec — formatting an ancient epoch with
                // ``%F`` would otherwise trap on the i32 → u32 cast.
                off = write_pad_signed(out, off, @intCast(t.year), 4);
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

// -- clock scan ---------------------------------------------------------------
//
// Free-form ``clock scan`` is a substantial parser in C Tcl
// (``tclGetDate.y``).  We don't reimplement that here — instead we
// support the subset of inputs that the task spec calls for and
// that round-trip with ``clock format``:
//
//   YYYY-MM-DD                       (ISO date, midnight UTC)
//   YYYY-MM-DDTHH:MM:SS              (ISO date+time, UTC)
//   YYYY-MM-DD HH:MM:SS              (RFC 3339-like)
//   YYYY-MM-DDTHH:MM:SSZ             (Zulu)
//   YYYY-MM-DDTHH:MM:SS±HHMM         (offset)
//   YYYY-MM-DDTHH:MM:SS±HH:MM        (offset, RFC 3339)
//
// Anything else falls through to the integer parser (``clock scan
// 12345`` returns 12345 — useful when the input is already epoch
// seconds), and a final failure returns 0 with no error so the
// existing degraded-path callers don't regress.

/// Skip leading whitespace, advancing the cursor.  Returns the
/// new index.
fn skip_ws(s: []const u8, i_in: usize) usize {
    var i = i_in;
    while (i < s.len) : (i += 1) {
        const c = s[i];
        if (c != ' ' and c != '\t') break;
    }
    return i;
}

/// Consume a non-negative decimal integer of up to ``max_digits``
/// digits at ``s[i..]``.  Returns ``(value, next_i)`` or null if no
/// digits were found.
fn read_uint(s: []const u8, i_in: usize, max_digits: usize) ?struct { v: i64, i: usize } {
    var i = i_in;
    var v: i64 = 0;
    var n: usize = 0;
    while (i < s.len and n < max_digits and s[i] >= '0' and s[i] <= '9') : (i += 1) {
        v = v * 10 + (@as(i64, s[i] - '0'));
        n += 1;
    }
    if (n == 0) return null;
    return .{ .v = v, .i = i };
}

/// Try to parse ``s`` as a decimal integer (signed, optional ``+`` /
/// ``-`` prefix).  Returns the value or null on miss.
fn parse_signed(s: []const u8) ?i64 {
    if (s.len == 0) return null;
    var i: usize = 0;
    var neg = false;
    if (s[0] == '-') {
        neg = true;
        i = 1;
    } else if (s[0] == '+') {
        i = 1;
    }
    var v: i64 = 0;
    var n: usize = 0;
    while (i < s.len and s[i] >= '0' and s[i] <= '9') : (i += 1) {
        const d: i64 = @intCast(s[i] - '0');
        const m = @mulWithOverflow(v, @as(i64, 10));
        if (m[1] != 0) return null;
        const a = @addWithOverflow(m[0], d);
        if (a[1] != 0) return null;
        v = a[0];
        n += 1;
    }
    if (n == 0 or i != s.len) return null;
    return if (neg) -v else v;
}

/// Result of a successful scan: epoch seconds in UTC plus a flag
/// indicating whether the input carried an explicit zone (``Z`` /
/// ``±HH:MM``).  Callers use the flag to decide whether to apply
/// the resolver's default zone.
const ScanResult = struct {
    epoch: i64,
    has_zone: bool,
};

/// Try to parse ``s`` as one of the supported ISO-ish forms.
/// Returns null on any deviation — there's no recovery, so the
/// integer fallback can take over.
fn parse_iso(s: []const u8) ?ScanResult {
    if (s.len < 8) return null; // "YYYY-M-D" is 8 chars minimum
    var i: usize = 0;
    const y = read_uint(s, i, 4) orelse return null;
    if (y.i != 4) return null; // require exactly 4 year digits
    if (y.i >= s.len or s[y.i] != '-') return null;
    i = y.i + 1;
    const mo = read_uint(s, i, 2) orelse return null;
    if (mo.i >= s.len or s[mo.i] != '-') return null;
    i = mo.i + 1;
    const da = read_uint(s, i, 2) orelse return null;
    i = da.i;

    var hour: i64 = 0;
    var minute: i64 = 0;
    var second: i64 = 0;

    if (i < s.len and (s[i] == 'T' or s[i] == ' ' or s[i] == 't')) {
        i += 1;
        const hh = read_uint(s, i, 2) orelse return null;
        if (hh.i >= s.len or s[hh.i] != ':') return null;
        i = hh.i + 1;
        const mm = read_uint(s, i, 2) orelse return null;
        i = mm.i;
        hour = hh.v;
        minute = mm.v;
        if (i < s.len and s[i] == ':') {
            i += 1;
            const ss = read_uint(s, i, 2) orelse return null;
            second = ss.v;
            i = ss.i;
            // Optional fractional seconds — recognised and dropped.
            if (i < s.len and s[i] == '.') {
                i += 1;
                while (i < s.len and s[i] >= '0' and s[i] <= '9') : (i += 1) {}
            }
        }
    }

    var zone_offset: i64 = 0;
    var has_zone = false;
    if (i < s.len) {
        const c = s[i];
        if (c == 'Z' or c == 'z') {
            i += 1;
            has_zone = true;
        } else if (c == '+' or c == '-') {
            const sign: i64 = if (c == '-') -1 else 1;
            i += 1;
            const oh = read_uint(s, i, 2) orelse return null;
            i = oh.i;
            var om: i64 = 0;
            if (i < s.len and s[i] == ':') i += 1;
            if (i < s.len and s[i] >= '0' and s[i] <= '9') {
                const omr = read_uint(s, i, 2) orelse return null;
                om = omr.v;
                i = omr.i;
            }
            zone_offset = sign * (oh.v * 3600 + om * 60);
            has_zone = true;
        }
    }
    if (i != s.len) return null; // trailing garbage

    // Field-bounds validation.  Without these checks an input like
    // ``2025-99-01`` would forward ``month = 99`` to ``pack_epoch``
    // which indexes ``MONTH_DAYS_NORMAL[97]`` and traps on the
    // out-of-bounds array access.  Reject malformed dates as a
    // parse miss so ``clock_scan_obj`` falls through to the
    // integer / signed parsers (or returns 0) instead of crashing.
    if (mo.v < 1 or mo.v > 12) return null;
    const dim = days_in_month(@intCast(y.v), @intCast(mo.v));
    if (da.v < 1 or da.v > dim) return null;
    if (hour < 0 or hour > 23) return null;
    if (minute < 0 or minute > 59) return null;
    // Allow second == 60 for leap-second-encoded inputs; tcllib
    // round-trips them and ``pack_epoch`` collapses the extra
    // second into the next minute.
    if (second < 0 or second > 60) return null;

    const epoch_local = pack_epoch(
        @intCast(y.v),
        @intCast(mo.v),
        @intCast(da.v),
        @intCast(hour),
        @intCast(minute),
        @intCast(second),
    );
    return .{ .epoch = epoch_local - zone_offset, .has_zone = has_zone };
}

// -- Free-form clock scan ----------------------------------------------------
//
// A pragmatic subset of Tcl's free-form date grammar covering the
// inputs that real scripts actually hand to ``clock scan`` (the
// full ``GetDate.y`` is ~3 KSLOC of yacc and doesn't earn its keep
// at this layer).  Recognised forms:
//
//   now                            -> base
//   today                          -> midnight of base, in target zone
//   yesterday / tomorrow           -> ±86400 from today's midnight
//   epoch                          -> 0 (Unix epoch)
//   +N unit / -N unit / N unit     -> base ± N units
//   N unit ago                     -> base - N units
//   Month Day, Year                -> calendar date, midnight UTC
//   Day Month Year                 -> calendar date, midnight UTC
//   MM/DD/YYYY                     -> US-style calendar date
//   DD/MM/YYYY                     -> Tcl's ambiguous form; we map
//                                     to MM/DD when month <= 12, else
//                                     fall through (matches Tcl 9
//                                     where MDY is the default
//                                     except in en_GB locale).
//
// Parsing is case-insensitive for English month / weekday / unit
// names.  Whitespace is consumed liberally — the parser doesn't
// care about commas, multiple spaces, or trailing punctuation.

const MONTH_NAMES_FULL = [_][]const u8{
    "january", "february", "march",     "april",   "may",      "june",
    "july",    "august",   "september", "october", "november", "december",
};
const MONTH_NAMES_ABBR = [_][]const u8{
    "jan", "feb", "mar", "apr", "may", "jun",
    "jul", "aug", "sep", "oct", "nov", "dec",
};

fn ascii_lower(b: u8) u8 {
    return if (b >= 'A' and b <= 'Z') b + 32 else b;
}

/// Case-insensitive equality between two byte slices.
fn ieq(a: []const u8, b: []const u8) bool {
    if (a.len != b.len) return false;
    for (a, b) |x, y| {
        if (ascii_lower(x) != ascii_lower(y)) return false;
    }
    return true;
}

/// Match ``s`` against the full + abbreviated month-name table.
/// Returns the 1-based month number on hit, null on miss.
fn match_month(s: []const u8) ?u32 {
    for (MONTH_NAMES_FULL, 0..) |m, i| {
        if (ieq(s, m)) return @intCast(i + 1);
    }
    for (MONTH_NAMES_ABBR, 0..) |m, i| {
        if (ieq(s, m)) return @intCast(i + 1);
    }
    return null;
}

/// Tokeniser context — splits ``s`` into whitespace / punctuation
/// separated tokens.  ``next()`` skips ``,`` and ``.`` after each
/// token so ``Jan 1, 2025`` reads as three tokens.
const Tokens = struct {
    s: []const u8,
    i: usize = 0,

    fn done(self: *const Tokens) bool {
        return self.i >= self.s.len;
    }
    fn peek(self: *Tokens) ?[]const u8 {
        const save = self.i;
        const t = self.next();
        self.i = save;
        return t;
    }
    fn next(self: *Tokens) ?[]const u8 {
        // Skip whitespace + structural punctuation.
        while (self.i < self.s.len) : (self.i += 1) {
            const b = self.s[self.i];
            if (b != ' ' and b != '\t' and b != ',' and b != '.') break;
        }
        if (self.i >= self.s.len) return null;
        const start = self.i;
        const b0 = self.s[start];
        if ((b0 >= '0' and b0 <= '9') or b0 == '+' or b0 == '-') {
            // Numeric token (possibly signed).  Consumes digits +
            // optional ``:`` for clock times so ``12:34:56`` is one
            // token rather than three.
            self.i += 1;
            while (self.i < self.s.len) : (self.i += 1) {
                const b = self.s[self.i];
                if (!((b >= '0' and b <= '9') or b == ':' or b == '/' or b == '-')) break;
            }
        } else {
            // Word token — letters only.
            while (self.i < self.s.len) : (self.i += 1) {
                const b = self.s[self.i];
                if (!((b >= 'a' and b <= 'z') or (b >= 'A' and b <= 'Z'))) break;
            }
        }
        return self.s[start..self.i];
    }
};

/// Map an English unit name to a (count, secs) pair for the
/// fixed-second units.  Calendar units (``month``, ``year``) are
/// signalled via a separate path because their length depends on
/// the target date.
const UnitKind = enum { seconds, minutes, hours, days, weeks, months, years };

fn match_unit(s: []const u8) ?UnitKind {
    if (ieq(s, "second") or ieq(s, "seconds") or ieq(s, "sec")) return .seconds;
    if (ieq(s, "minute") or ieq(s, "minutes") or ieq(s, "min")) return .minutes;
    if (ieq(s, "hour") or ieq(s, "hours") or ieq(s, "hr")) return .hours;
    if (ieq(s, "day") or ieq(s, "days")) return .days;
    if (ieq(s, "week") or ieq(s, "weeks")) return .weeks;
    if (ieq(s, "month") or ieq(s, "months")) return .months;
    if (ieq(s, "year") or ieq(s, "years")) return .years;
    return null;
}

fn unit_to_secs(u: UnitKind) ?i64 {
    return switch (u) {
        .seconds => 1,
        .minutes => SECS_PER_MINUTE,
        .hours => SECS_PER_HOUR,
        .days => SECS_PER_DAY,
        .weeks => SECS_PER_DAY * 7,
        .months, .years => null,
    };
}

/// Apply ``count units`` to ``base``, returning a new epoch.  Used
/// for the relative-form parser; calendar-relative add reuses
/// :func:`clock_add_pair`'s logic.
fn apply_unit(base: i64, count: i64, u: UnitKind) i64 {
    if (unit_to_secs(u)) |s| return base + count * s;
    const t = break_down(base);
    switch (u) {
        .months => {
            var year = t.year;
            var month: i64 = @as(i64, @intCast(t.month)) + count;
            while (month < 1) {
                year -= 1;
                month += 12;
            }
            while (month > 12) {
                year += 1;
                month -= 12;
            }
            const dim = days_in_month(year, @intCast(month));
            const day = if (t.day > dim) dim else t.day;
            return pack_epoch(
                year,
                @intCast(month),
                day,
                t.hour,
                t.minute,
                t.second,
            );
        },
        .years => {
            const year: i32 = @intCast(@as(i64, t.year) + count);
            const dim = days_in_month(year, t.month);
            const day = if (t.day > dim) dim else t.day;
            return pack_epoch(year, t.month, day, t.hour, t.minute, t.second);
        },
        else => unreachable,
    }
}

/// Parse a leading signed integer from a token.  ``"+5"`` / ``"-5"``
/// / ``"5"`` all return 5 / -5 / 5.  Returns null on miss or i64
/// overflow — the upstream tcltest clock suite hands us very long
/// digit-only tokens (epoch-style integers as scan input, plus a
/// few "absurd" tokens to test error handling) that would overflow
/// the running accumulator if left unchecked.
fn token_signed_int(tok: []const u8) ?i64 {
    if (tok.len == 0) return null;
    var i: usize = 0;
    var neg = false;
    if (tok[0] == '+') {
        i = 1;
    } else if (tok[0] == '-') {
        neg = true;
        i = 1;
    }
    if (i >= tok.len) return null;
    var v: i64 = 0;
    while (i < tok.len and tok[i] >= '0' and tok[i] <= '9') : (i += 1) {
        const d: i64 = @intCast(tok[i] - '0');
        const m = @mulWithOverflow(v, @as(i64, 10));
        if (m[1] != 0) return null;
        const a = @addWithOverflow(m[0], d);
        if (a[1] != 0) return null;
        v = a[0];
    }
    if (i != tok.len) return null;
    return if (neg) -v else v;
}

/// Validated calendar pack: month must be 1..12 and day must fit
/// the month's actual length (clamped via :func:`days_in_month`).
/// Year is bounded to 1..9999 (the proleptic-Gregorian range
/// the renderer + format specs cover comfortably).  Returns null
/// on any deviation so callers fall through to the next parse
/// alternative instead of trapping on an out-of-range index.
fn pack_validated(year: i64, month: i64, day: i64) ?i64 {
    if (year < 1 or year > 9999) return null;
    if (month < 1 or month > 12) return null;
    const dim = days_in_month(@intCast(year), @intCast(month));
    if (day < 1 or day > dim) return null;
    return pack_epoch(@intCast(year), @intCast(month), @intCast(day), 0, 0, 0);
}

/// Free-form scan attempt.  Returns an epoch on hit.  Calendar-naive
/// forms (``today`` / ``yesterday`` / ``tomorrow`` / ``Month Day,
/// Year`` / ``MM/DD/YYYY``) need to know the target timezone's offset
/// at ``base`` so that ``today`` resolves to midnight of the *local*
/// date, not the UTC date.  Callers pre-resolve the zone and pass
/// ``base_utoff`` (seconds east of UTC).
///
/// The has_zone flag tells :func:`clock_scan_obj` whether to apply
/// the post-parse offset adjustment.  Relative forms (``+5 days``)
/// stay in UTC and set has_zone=true; calendar-naive forms produce
/// a "local-frame" epoch (midnight as if the zone were UTC) and
/// set has_zone=false so the caller subtracts the offset.
fn parse_freeform(s: []const u8, base: i64, base_utoff: i32) ?ScanResult {
    // Local-frame view of ``base`` so ``break_down`` yields the
    // calendar date a wall clock in the target zone shows.
    const local_base = base + @as(i64, base_utoff);

    var t = Tokens{ .s = s };
    const tok0 = t.peek() orelse return null;
    if (ieq(tok0, "now")) {
        _ = t.next();
        if (!t.done()) return null;
        return .{ .epoch = base, .has_zone = true };
    }
    if (ieq(tok0, "today")) {
        _ = t.next();
        if (!t.done()) return null;
        const bt = break_down(local_base);
        return .{ .epoch = pack_epoch(bt.year, bt.month, bt.day, 0, 0, 0), .has_zone = false };
    }
    if (ieq(tok0, "yesterday")) {
        _ = t.next();
        if (!t.done()) return null;
        const bt = break_down(local_base - SECS_PER_DAY);
        return .{ .epoch = pack_epoch(bt.year, bt.month, bt.day, 0, 0, 0), .has_zone = false };
    }
    if (ieq(tok0, "tomorrow")) {
        _ = t.next();
        if (!t.done()) return null;
        const bt = break_down(local_base + SECS_PER_DAY);
        return .{ .epoch = pack_epoch(bt.year, bt.month, bt.day, 0, 0, 0), .has_zone = false };
    }
    if (ieq(tok0, "epoch")) {
        _ = t.next();
        if (!t.done()) return null;
        return .{ .epoch = 0, .has_zone = true };
    }

    // ``Month Day, Year`` — Jan 1 2025 / January 1 2025
    if (match_month(tok0)) |m| {
        _ = t.next();
        const day_tok = t.next() orelse return null;
        const day = token_signed_int(day_tok) orelse return null;
        const year_tok = t.next() orelse return null;
        const year_v = token_signed_int(year_tok) orelse return null;
        if (!t.done()) return null;
        const ep = pack_validated(year_v, @intCast(m), day) orelse return null;
        return .{ .epoch = ep, .has_zone = false };
    }

    // Numeric leading token — ``+N unit`` / ``N unit`` / ``N unit ago``
    // / ``MM/DD/YYYY`` / ``Day Month Year``.
    if (token_signed_int(tok0)) |n| {
        _ = t.next();
        const tok1 = t.peek() orelse return null;
        if (match_unit(tok1)) |u| {
            _ = t.next();
            // Optional ``ago`` suffix flips the sign.
            var count = n;
            if (t.peek()) |tok2| {
                if (ieq(tok2, "ago")) {
                    _ = t.next();
                    count = -count;
                }
            }
            if (!t.done()) return null;
            return .{ .epoch = apply_unit(base, count, u), .has_zone = true };
        }
        if (match_month(tok1)) |m| {
            _ = t.next();
            const tok2 = t.next() orelse return null;
            const year_v = token_signed_int(tok2) orelse return null;
            if (!t.done()) return null;
            const ep = pack_validated(year_v, @intCast(m), n) orelse return null;
            return .{ .epoch = ep, .has_zone = false };
        }
    }

    // ``MM/DD/YYYY`` — slash-separated US date.
    if (parse_slash_date(tok0)) |r| {
        _ = t.next();
        if (!t.done()) return null;
        return .{ .epoch = r, .has_zone = false };
    }

    return null;
}

/// Parse ``MM/DD/YYYY``.  Returns the epoch (midnight UTC) or null.
fn parse_slash_date(s: []const u8) ?i64 {
    var i: usize = 0;
    const m = read_uint(s, i, 2) orelse return null;
    if (m.i >= s.len or s[m.i] != '/') return null;
    i = m.i + 1;
    const d = read_uint(s, i, 2) orelse return null;
    if (d.i >= s.len or s[d.i] != '/') return null;
    i = d.i + 1;
    const y = read_uint(s, i, 4) orelse return null;
    if (y.i != s.len) return null;
    return pack_validated(y.v, m.v, d.v);
}

/// clock_scan_obj — parse a date/time string into Unix epoch
/// seconds.  ``zone_obj`` is consulted only when the input doesn't
/// carry its own zone (no ``Z`` / no ``±HHMM``).  ``gmt_flag``
/// non-zero forces UTC and overrides ``zone_obj``.  ``base_obj``
/// supplies the reference timestamp for relative forms (``now`` /
/// ``+N unit`` / ``yesterday``); pass ``0`` to use the current
/// wall-clock epoch.
pub export fn clock_scan_obj(
    text_obj: i32,
    zone_obj: i32,
    gmt_flag: i32,
    base_obj: i32,
) i32 {
    if (text_obj == 0) return obj_new_int(0);
    const t = obj_ensure_string(text_obj);
    if (t.ptr == 0 or t.len == 0) return obj_new_int(0);
    const tp: [*]const u8 = @ptrFromInt(t.ptr);
    const ts0 = tp[0..t.len];
    const start = skip_ws(ts0, 0);
    var end = ts0.len;
    while (end > start and (ts0[end - 1] == ' ' or ts0[end - 1] == '\t')) : (end -= 1) {}
    const ts = ts0[start..end];

    const base: i64 = blk: {
        if (base_obj != 0) break :blk obj.obj_get_int(base_obj);
        const ns = clock_ns(.REALTIME);
        break :blk @divTrunc(ns, NS_PER_SECOND);
    };

    // Resolve the target zone up-front: ``parse_freeform`` needs
    // the offset at ``base`` to compute calendar boundaries
    // (``today`` / ``yesterday`` / ``tomorrow``) in the local
    // frame rather than UTC.  Without this the boundary fires off
    // by one whole day for any base near midnight UTC in a
    // negative-offset zone (e.g. ``today`` with base = 02:00 UTC
    // and -timezone :America/New_York lands on the wrong
    // calendar day, since 02:00 UTC = 21:00 EST the previous
    // evening).
    const zone_slice: []const u8 = blk: {
        if (zone_obj == 0) break :blk &[_]u8{};
        const zs = obj_ensure_string(zone_obj);
        if (zs.ptr == 0 or zs.len == 0) break :blk &[_]u8{};
        const zp: [*]const u8 = @ptrFromInt(zs.ptr);
        break :blk zp[0..zs.len];
    };
    const z: *const tz.TimeZone = if (gmt_flag != 0)
        &tz_utc_zone
    else if (zone_slice.len == 0)
        tz.resolve_default()
    else
        tz.resolve(zone_slice);
    const base_utoff: i32 = z.offset_at(base).utoff;

    const result: ?ScanResult = parse_iso(ts) orelse parse_freeform(ts, base, base_utoff);
    if (result) |r| {
        if (r.has_zone) return obj_new_int(r.epoch);
        if (gmt_flag != 0) return obj_new_int(r.epoch);
        // Apply the zone's offset at the *target* time.  Two-pass:
        // assume UTC first, look up the offset there, subtract it.
        // Close enough for non-DST-transition timestamps; full
        // disambiguation needs the offset-at-local-time logic from
        // tclClock.c which can land in a follow-up.
        const off_info = z.offset_at(r.epoch);
        return obj_new_int(r.epoch - @as(i64, off_info.utoff));
    }

    // Fallback: integer epoch passes through unchanged so
    // ``clock scan [clock format $t -gmt 1 -format %s]`` round-trips.
    if (parse_signed(ts)) |v| return obj_new_int(v);
    return obj_new_int(0);
}

/// File-local synthetic UTC zone used by ``clock_scan_obj`` when
/// ``-gmt 1`` is in effect — picks the same zero-offset, zero-DST
/// shape as ``tz.utc()`` without paying for a fresh allocation
/// every scan.
const tz_utc_zone: tz.TimeZone = tz.utc();

// -- clock add ----------------------------------------------------------------

const SECS_PER_MINUTE: i64 = 60;
const SECS_PER_HOUR: i64 = 3600;
const SECS_PER_DAY: i64 = 86_400;

/// Days in (year, 1-based month).  Caller is responsible for
/// passing 1..12 — we don't bounds-check here.
fn days_in_month(year: i32, month: u32) u32 {
    return if (is_leap(year)) MONTH_DAYS_LEAP[month - 1] else MONTH_DAYS_NORMAL[month - 1];
}

/// Re-pack a broken-down (year, month, day, hms…) tuple into Unix
/// epoch seconds.  Inverse of ``break_down``.  Used by ``clock add``
/// for the calendar-relative units (``months``, ``years``) where
/// adding seconds isn't sufficient (a "month" varies in length).
fn pack_epoch(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32) i64 {
    // Accumulate days from 1970-01-01 to target date.
    var d: i64 = 0;
    if (year >= 1970) {
        var y: i32 = 1970;
        while (y < year) : (y += 1) {
            d += if (is_leap(y)) 366 else 365;
        }
    } else {
        var y: i32 = year;
        while (y < 1970) : (y += 1) {
            d -= if (is_leap(y)) 366 else 365;
        }
    }
    var m: u32 = 1;
    while (m < month) : (m += 1) {
        d += days_in_month(year, m);
    }
    d += @as(i64, day) - 1;
    return d * SECS_PER_DAY +
        @as(i64, hour) * SECS_PER_HOUR +
        @as(i64, minute) * SECS_PER_MINUTE +
        @as(i64, second);
}

/// Compare two byte slices for case-sensitive equality.  Tcl unit
/// names are lowercase (``weeks`` / ``days`` / …); we don't bother
/// with case-folding in the hot path.
fn unit_eq(s: []const u8, lit: []const u8) bool {
    return std.mem.eql(u8, s, lit);
}

/// clock_add_pair — add ``count`` of ``unit`` to ``base`` epoch
/// seconds and return a new TclObj integer.  Unit names match the
/// Tcl ``clock add`` reference:
///
///   seconds, minutes, hours, days, weeks   — fixed-second units.
///   months, years                          — calendar-relative;
///                                            require a UTC-only
///                                            re-pack (timezone
///                                            -aware month math is
///                                            too much for one
///                                            commit).
///
/// Unknown units fall through to a no-op (returns ``base`` unchanged)
/// so a malformed call doesn't trap the whole interpreter.  Singular
/// forms (``second`` / ``day``) are accepted — Tcl's parser is
/// tolerant.
pub export fn clock_add_pair(base_obj: i32, count_obj: i32, unit_obj: i32) i32 {
    const base: i64 = if (base_obj == 0) 0 else obj.obj_get_int(base_obj);
    const count: i64 = if (count_obj == 0) 0 else obj.obj_get_int(count_obj);
    if (unit_obj == 0) return obj_new_int(base);
    const u = obj_ensure_string(unit_obj);
    if (u.ptr == 0 or u.len == 0) return obj_new_int(base);
    const up: [*]const u8 = @ptrFromInt(u.ptr);
    const us = up[0..u.len];
    if (unit_eq(us, "seconds") or unit_eq(us, "second")) {
        return obj_new_int(base + count);
    }
    if (unit_eq(us, "minutes") or unit_eq(us, "minute")) {
        return obj_new_int(base + count * SECS_PER_MINUTE);
    }
    if (unit_eq(us, "hours") or unit_eq(us, "hour")) {
        return obj_new_int(base + count * SECS_PER_HOUR);
    }
    if (unit_eq(us, "days") or unit_eq(us, "day")) {
        return obj_new_int(base + count * SECS_PER_DAY);
    }
    if (unit_eq(us, "weeks") or unit_eq(us, "week")) {
        return obj_new_int(base + count * SECS_PER_DAY * 7);
    }
    if (unit_eq(us, "months") or unit_eq(us, "month")) {
        // Calendar month math: break down (UTC), bump month, re-pack.
        // Day-of-month clamping matches Tcl: adding 1 month to
        // 2025-01-31 lands on 2025-02-28, not "March 3".
        const t = break_down(base);
        var year = t.year;
        var month: i64 = @as(i64, @intCast(t.month)) + count;
        // Normalise to 1..12.
        while (month < 1) {
            year -= 1;
            month += 12;
        }
        while (month > 12) {
            year += 1;
            month -= 12;
        }
        const dim = days_in_month(year, @intCast(month));
        const day = if (t.day > dim) dim else t.day;
        return obj_new_int(pack_epoch(
            year,
            @intCast(month),
            day,
            t.hour,
            t.minute,
            t.second,
        ));
    }
    if (unit_eq(us, "years") or unit_eq(us, "year")) {
        const t = break_down(base);
        const year: i32 = @intCast(@as(i64, t.year) + count);
        const dim = days_in_month(year, t.month);
        const day = if (t.day > dim) dim else t.day;
        return obj_new_int(pack_epoch(
            year,
            t.month,
            day,
            t.hour,
            t.minute,
            t.second,
        ));
    }
    // Unknown unit — leave the timestamp untouched.  Tcl errors
    // here but the existing ``clock_add`` stub returned 0; this is
    // strictly less surprising.
    return obj_new_int(base);
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

