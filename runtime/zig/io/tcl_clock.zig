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
        const dy: i32 = if (is_leap(year)) 366 else 365;
        if (d < 0) {
            year -= 1;
            d += @as(i64, if (is_leap(year)) 366 else 365);
        } else if (d >= dy) {
            d -= dy;
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

// -- roman numerals ----------------------------------------------------------
//
// The ``en_US_roman`` locale renders alternate (``%O*`` / ``%E*``)
// numeric specs in lowercase Roman numerals.  We carry our own
// encoder + decoder rather than depending on a locale table because
// the corpus is fixed: 0 is rendered as ``?`` (a sentinel the
// reference Tcl uses for "no Roman representation"), 1..3999 maps
// to the standard subtraction form, and out-of-range values fall
// back to ``?`` as well — extremely rare in practice (the upper
// bound of any year we'd format is 9999 and all year-relative
// scratch values are below 3999).

/// Roman digit table — each entry is a ``(value, symbol)`` pair in
/// descending order.  The encoder walks the table once, subtracting
/// each value as many times as fits.  Subtraction shorthand
/// (``IV`` / ``IX`` / ``XL`` etc.) is encoded as a regular table
/// entry rather than a post-pass, which keeps the loop simple.
const ROMAN_TABLE = [_]struct { v: u32, s: []const u8 }{
    .{ .v = 1000, .s = "m" },
    .{ .v = 900,  .s = "cm" },
    .{ .v = 500,  .s = "d" },
    .{ .v = 400,  .s = "cd" },
    .{ .v = 100,  .s = "c" },
    .{ .v = 90,   .s = "xc" },
    .{ .v = 50,   .s = "l" },
    .{ .v = 40,   .s = "xl" },
    .{ .v = 10,   .s = "x" },
    .{ .v = 9,    .s = "ix" },
    .{ .v = 5,    .s = "v" },
    .{ .v = 4,    .s = "iv" },
    .{ .v = 1,    .s = "i" },
};

/// Render *value* as a lowercase Roman numeral into ``out`` starting
/// at ``off``.  Returns the new offset.  ``value == 0`` or
/// ``value >= 4000`` emit ``?`` — the same sentinel reference Tcl
/// produces.
fn write_roman(out: [*]u8, off_in: u32, value: i64) u32 {
    var off = off_in;
    if (value <= 0 or value >= 4000) {
        out[off] = '?';
        return off + 1;
    }
    var v: u32 = @intCast(value);
    inline for (ROMAN_TABLE) |entry| {
        while (v >= entry.v) {
            for (entry.s) |c| {
                out[off] = c;
                off += 1;
            }
            v -= entry.v;
        }
    }
    return off;
}

/// Try to parse a lowercase Roman numeral from ``s[i..]``.  Accepts
/// the strict subtractive form (``iv``, ``ix``, ``xl``, …) but
/// tolerates ascii-case differences (``IV`` matches the same value
/// as ``iv``).  Returns ``(value, end_index)`` or null if no
/// valid Roman numeral starts at ``i_in``.  Empty / non-Roman
/// input returns null.
fn read_roman(s: []const u8, i_in: usize) ?struct { v: i64, i: usize } {
    var i = i_in;
    var v: i64 = 0;
    // Accumulate digits using the same subtractive table; longer
    // matches first (``cm`` before ``c``, ``ix`` before ``i``).
    while (i < s.len) {
        var matched = false;
        inline for (ROMAN_TABLE) |entry| {
            const slen = entry.s.len;
            if (!matched and i + slen <= s.len) {
                var ok = true;
                for (entry.s, 0..) |c, k| {
                    if (ascii_lower(s[i + k]) != c) { ok = false; break; }
                }
                if (ok) {
                    v += @as(i64, @intCast(entry.v));
                    i += slen;
                    matched = true;
                }
            }
        }
        if (!matched) break;
    }
    if (i == i_in) return null;
    return .{ .v = v, .i = i };
}

// -- ISO week computation ----------------------------------------------------
//
// ``%g`` / ``%G`` / ``%V`` need the ISO 8601 week-numbering year.
// Algorithm: take the date's day-of-year, find the Thursday of the
// week (ISO Thursday is the "anchor day" — the year that owns the
// Thursday owns the whole week).  ``%U`` / ``%W`` use a simpler
// week-since-first-Sunday / first-Monday count.

const IsoWeek = struct { year: i32, week: u32 };

fn days_in_year(y: i32) u32 {
    return if (is_leap(y)) 366 else 365;
}

fn iso_week_of(t: BrokenDown) IsoWeek {
    // ISO weekday: Mon=1..Sun=7.
    const iso_dow: i32 = if (t.weekday == 0) 7 else @intCast(t.weekday);
    // Day-of-year of the Thursday in this week.
    const thursday_doy: i32 = @as(i32, @intCast(t.yday)) + 4 - iso_dow;
    if (thursday_doy < 1) {
        const prev_dy: i32 = @intCast(days_in_year(t.year - 1));
        const w: u32 = @intCast(@divFloor(thursday_doy + prev_dy - 1, 7) + 1);
        return .{ .year = t.year - 1, .week = w };
    }
    const this_dy: i32 = @intCast(days_in_year(t.year));
    if (thursday_doy > this_dy) {
        const w: u32 = @intCast(@divFloor(thursday_doy - this_dy - 1, 7) + 1);
        return .{ .year = t.year + 1, .week = w };
    }
    const w: u32 = @intCast(@divFloor(thursday_doy - 1, 7) + 1);
    return .{ .year = t.year, .week = w };
}

/// ``%U`` — week number with Sunday as the first day of week (00..53).
/// Days before the first Sunday are in week 0.
fn week_sunday_first(t: BrokenDown) u32 {
    // 0=Sunday..6=Saturday.
    const dow: i32 = @intCast(t.weekday);
    const yday1: i32 = @as(i32, @intCast(t.yday)) - 1; // 0-based
    const w = @divFloor(yday1 + 7 - dow, 7);
    if (w < 0) return 0;
    return @intCast(w);
}

/// ``%W`` — week number with Monday as the first day of week (00..53).
fn week_monday_first(t: BrokenDown) u32 {
    const dow: i32 = if (t.weekday == 0) 6 else @as(i32, @intCast(t.weekday)) - 1;
    const yday1: i32 = @as(i32, @intCast(t.yday)) - 1;
    const w = @divFloor(yday1 + 7 - dow, 7);
    if (w < 0) return 0;
    return @intCast(w);
}

/// Julian Day Number of the broken-down date (UTC).  JDN 2440588
/// is 1970-01-01 UTC; we compute it directly from the epoch
/// implied by ``t``.
fn julian_day_of(t: BrokenDown) i64 {
    const epoch = pack_epoch(t.year, t.month, t.day, 0, 0, 0);
    return @divFloor(epoch, SECS_PER_DAY) + 2440588;
}

/// Locale tag.  Only two values are honoured — the default
/// ``en_US`` corpus and ``en_US_roman``.  Unknown locales fall
/// through to ``en_US`` (we don't ship a real locale database).
const Locale = enum { en_us, en_us_roman };

fn locale_from(s: []const u8) Locale {
    if (s.len == 0) return .en_us;
    // Case-insensitive match — accepts ``en_US_roman``, ``en_us_roman``,
    // and any case mix.
    if (s.len == 11 and ieq(s, "en_us_roman")) return .en_us_roman;
    return .en_us;
}

/// Locale composite-spec table.  When ``%c`` / ``%x`` / ``%X`` /
/// ``%Ec`` / ``%Ex`` / ``%EX`` is encountered, the renderer
/// substitutes the matching string from this table and recursively
/// expands it.
fn locale_replace(loc: Locale, spec: u8, ext: bool) ?[]const u8 {
    return switch (loc) {
        .en_us => switch (spec) {
            'c' => "%a %b %e %H:%M:%S %Y",
            'x' => "%m/%d/%Y",
            'X' => "%H:%M:%S",
            else => null,
        },
        .en_us_roman => if (ext) switch (spec) {
            'c' => "die %Od mensis %Om annoque %EY %OH h %OM m %OS s",
            'x' => "die %Od mensis %Om annoque %EY",
            'X' => "%OH h %OM m %OS s",
            else => null,
        } else switch (spec) {
            'c' => "%m/%d/%Y %H:%M:%S",
            'x' => "%m/%d/%Y",
            'X' => "%H:%M:%S",
            else => null,
        },
    };
}

/// Recursively expand a strftime template into ``out`` starting at
/// ``off``.  Returns the new offset.  Locale composite specs (``%c``
/// / ``%Ec`` / ``%x`` / ``%Ex`` / ``%X`` / ``%EX`` / ``%+``) re-enter
/// this function with the locale's substitution string, so a ``%c``
/// inside a ``%Ec`` template won't infinite-loop (the substitution
/// strings don't contain those keys themselves).
fn expand_format(
    out: [*]u8,
    off_in: u32,
    fmt: []const u8,
    t: BrokenDown,
    utoff: i32,
    abbr: []const u8,
    loc: Locale,
) u32 {
    var off = off_in;
    var i: usize = 0;
    while (i < fmt.len) {
        const c = fmt[i];
        if (c != '%') {
            out[off] = c;
            off += 1;
            i += 1;
            continue;
        }
        i += 1;
        if (i >= fmt.len) {
            out[off] = '%';
            off += 1;
            break;
        }
        var ext_O = false;
        var ext_E = false;
        var spec = fmt[i];
        i += 1;
        // Locale modifier — only consumed when the next character is
        // an actual spec letter.  A trailing bare ``%O`` / ``%E`` is
        // emitted as the literal two characters so unterminated input
        // doesn't silently drop the tail (Copilot review).
        if (spec == 'O' or spec == 'E') {
            if (i >= fmt.len) {
                out[off] = '%';
                off += 1;
                out[off] = spec;
                off += 1;
                break;
            }
            if (spec == 'O') ext_O = true else ext_E = true;
            spec = fmt[i];
            i += 1;
        }
        if ((spec == 'c' or spec == 'x' or spec == 'X') and !ext_O) {
            const repl = locale_replace(loc, spec, ext_E) orelse "";
            off = expand_format(out, off, repl, t, utoff, abbr, loc);
            continue;
        }
        if (spec == '+' and !ext_O and !ext_E) {
            off = expand_format(out, off, "%a %b %e %H:%M:%S %Z %Y", t, utoff, abbr, loc);
            continue;
        }
        if (spec == 'r' and !ext_O and !ext_E) {
            off = expand_format(out, off, "%I:%M:%S %P", t, utoff, abbr, loc);
            continue;
        }
        const roman_o = ext_O and loc == .en_us_roman;
        const roman_e = ext_E and loc == .en_us_roman;
        switch (spec) {
            '%' => {
                out[off] = '%';
                off += 1;
            },
            'Y' => {
                if (roman_e) off = write_roman(out, off, t.year)
                else off = write_pad_signed(out, off, @intCast(t.year), 4);
            },
            'y' => {
                const y2: u32 = @intCast(@mod(@as(i64, t.year), 100));
                if (roman_o) off = write_roman(out, off, y2)
                else off = write_pad_int(out, off, y2, 2, '0');
            },
            'C' => {
                if (roman_e) {
                    const cyr = @as(i64, t.year) - @mod(@as(i64, t.year), 100);
                    off = write_roman(out, off, cyr);
                } else {
                    const c100: i64 = @divFloor(@as(i64, t.year), 100);
                    off = write_pad_signed(out, off, c100, 2);
                }
            },
            's' => off = write_pad_signed(out, off, render_epoch(t, utoff), 0),
            'm' => {
                if (roman_o) off = write_roman(out, off, t.month)
                else off = write_pad_int(out, off, t.month, 2, '0');
            },
            'N' => {
                if (roman_o) off = write_roman(out, off, t.month)
                else off = write_pad_int(out, off, t.month, 2, ' ');
            },
            'd' => {
                if (roman_o) off = write_roman(out, off, t.day)
                else off = write_pad_int(out, off, t.day, 2, '0');
            },
            'e' => {
                if (roman_o) off = write_roman(out, off, t.day)
                else off = write_pad_int(out, off, t.day, 2, ' ');
            },
            'H' => {
                if (roman_o) off = write_roman(out, off, t.hour)
                else off = write_pad_int(out, off, t.hour, 2, '0');
            },
            'k' => {
                if (roman_o) off = write_roman(out, off, t.hour)
                else off = write_pad_int(out, off, t.hour, 2, ' ');
            },
            'M' => {
                if (roman_o) off = write_roman(out, off, t.minute)
                else off = write_pad_int(out, off, t.minute, 2, '0');
            },
            'S' => {
                if (roman_o) off = write_roman(out, off, t.second)
                else off = write_pad_int(out, off, t.second, 2, '0');
            },
            'I' => {
                var hr12: u32 = t.hour % 12;
                if (hr12 == 0) hr12 = 12;
                if (roman_o) off = write_roman(out, off, hr12)
                else off = write_pad_int(out, off, hr12, 2, '0');
            },
            'l' => {
                var hr12: u32 = t.hour % 12;
                if (hr12 == 0) hr12 = 12;
                if (roman_o) off = write_roman(out, off, hr12)
                else off = write_pad_int(out, off, hr12, 2, ' ');
            },
            'j' => off = write_pad_int(out, off, t.yday, 3, '0'),
            'J' => off = write_pad_signed(out, off, julian_day_of(t), 0),
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
            'g' => {
                const iso = iso_week_of(t);
                const y2: u32 = @intCast(@mod(@as(i64, iso.year), 100));
                off = write_pad_int(out, off, y2, 2, '0');
            },
            'G' => {
                const iso = iso_week_of(t);
                off = write_pad_signed(out, off, @intCast(iso.year), 4);
            },
            'V' => {
                const iso = iso_week_of(t);
                off = write_pad_int(out, off, iso.week, 2, '0');
            },
            'U' => off = write_pad_int(out, off, week_sunday_first(t), 2, '0'),
            'W' => off = write_pad_int(out, off, week_monday_first(t), 2, '0'),
            'n' => {
                out[off] = '\n';
                off += 1;
            },
            't' => {
                out[off] = '\t';
                off += 1;
            },
            'D' => off = expand_format(out, off, "%m/%d/%y", t, utoff, abbr, loc),
            'R' => off = expand_format(out, off, "%H:%M", t, utoff, abbr, loc),
            'T' => off = expand_format(out, off, "%H:%M:%S", t, utoff, abbr, loc),
            'F' => {
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
                if (ext_O) {
                    out[off] = 'O';
                    off += 1;
                } else if (ext_E) {
                    out[off] = 'E';
                    off += 1;
                }
                out[off] = spec;
                off += 1;
            },
        }
    }
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
    loc: Locale,
) i32 {
    const fp: [*]const u8 = @ptrFromInt(fmt_ptr);
    // Buffer cap: locale composites can expand 2-char specs to ~50
    // chars (``%Ec`` → ``die %Od mensis ...``) so we use a generous
    // 24× factor with a 128-byte floor.
    const cap: u32 = fmt_len * 24 + 128;
    const buf = alloc(cap);
    if (buf == 0) return obj_new_string(0, 0);
    const out: [*]u8 = @ptrFromInt(buf);
    const off = expand_format(out, 0, fp[0..fmt_len], t, utoff, abbr, loc);
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
    return render_format(f.ptr, f.len, t, 0, "UTC", .en_us);
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

// -- clock scan -format ------------------------------------------------------
//
// strftime-driven scan path: walks the format string, consuming the
// matching token from the input for each conversion spec, then packs
// the accumulated field set into Unix epoch seconds.  Complements the
// free-form / ISO parser above (``parse_iso`` / ``parse_freeform``)
// which auto-detects the input shape; ``-format`` forces an exact
// strftime template instead.
//
// Supported specs (the subset the tcltest ``clock-6`` / ``clock-7`` /
// ``clock-8`` slices need):
//
//   %s            epoch seconds (signed; takes precedence over fields)
//   %J            Julian day number
//   %Y            4-digit year
//   %y            2-digit year (00..99 → 2000..2069 / 1970..1999 split
//                 on 69 — same convention as Tcl 9 + POSIX strptime)
//   %C            2-digit century
//   %m            zero- or space-padded month (01..12)
//   %N            space-padded month (1..12, leading space optional)
//   %b / %h       abbreviated month name (Jan..Dec, case-insensitive)
//   %B            full month name
//   %d            zero-padded day-of-month (01..31)
//   %e            space-padded day-of-month (1..31)
//   %j            day-of-year (1..366) — accepted but currently ignored
//                 for date packing (future: derive month/day if no
//                 explicit month/day given)
//   %H / %k       24h hour (00..23 / 0..23)
//   %I / %l       12h hour (01..12 / 1..12) — combined with %p
//   %M            minute
//   %S            second
//   %p / %P       AM/PM (case-insensitive)
//   %a / %A       weekday name (case-insensitive, accepted but ignored)
//   %z            zone offset ``±HHMM`` / ``±HH:MM``
//   %Z            zone name (best-effort: skipped over, not validated)
//   %% / %n / %t  literal ``%`` / newline / tab
//
// Locale modifiers (``%O*`` / ``%E*``) under ``-locale en_US_roman``
// route through ``read_roman`` for ``%Od`` / ``%Oe`` / ``%Om`` /
// ``%Oy`` / ``%OH`` / ``%OI`` / ``%OM`` / ``%OS`` and through
// :func:`read_roman` (``%EC`` / ``%EY``) for the ``%E`` variants.
// The 2-letter prefix path (``ja`` for January) handles
// ``en_US_roman``'s ambiguous-prefix scan corpus via
// ``scan_match_month_prefix``.  Composite specs (``%c`` / ``%x`` /
// ``%X`` / ``%Ec`` / ``%Ex`` / ``%EX`` / ``%D`` / ``%R`` / ``%T`` /
// ``%F`` / ``%r``) recurse with the locale's substitution string
// at the same input position.  Unknown locale modifier specs (``%Ej``
// for astronomical Julian, see below) get their own dispatch arm.

const ScanFormatErr = enum {
    none,
    no_match,
    overflow,
};

/// Read either an Arabic decimal integer or a lowercase Roman
/// numeral.  Used by ``%Od`` / ``%Oe`` / ``%Om`` / ``%Oy`` under
/// ``en_US_roman`` (caller has already classified the locale).
/// Falls back to the Arabic reader when the input doesn't start
/// with a Roman digit, matching reference Tcl which accepts either
/// form for the alternate-numeric specs.
fn scan_read_int_or_roman(s: []const u8, i_in: usize, max_digits: usize) ?struct { v: i64, i: usize } {
    var i = i_in;
    if (i < s.len and s[i] == ' ') i += 1;
    if (i < s.len) {
        const c = ascii_lower(s[i]);
        if (c == 'i' or c == 'v' or c == 'x' or c == 'l' or c == 'c' or c == 'd' or c == 'm') {
            if (read_roman(s, i)) |r| return .{ .v = r.v, .i = r.i };
        }
    }
    return scan_read_int(s, i_in, max_digits, false);
}

const EpochField = enum { none, seconds, julian, astro_julian };

const ScanState = struct {
    have_seconds: bool = false,
    seconds: i64 = 0,

    have_julian: bool = false,
    julian: i64 = 0,

    /// Astronomical Julian Day (with fractional days) — set by
    /// ``%Ej``.  Stored as integer day + integer second-within-day
    /// to avoid f64 precision issues at large JDs.  ``aj_secs`` is
    /// the partial-day offset in seconds (0..86399).
    have_astro_julian: bool = false,
    aj_day: i64 = 0,
    aj_secs: i64 = 0,

    /// Which of ``%s`` / ``%J`` / ``%Ej`` was scanned last.  The
    /// upstream reference Tcl applies "last wins" precedence: when
    /// a format string mentions multiple epoch-bearing specs, the
    /// one consumed last from the input drives the result.
    last_epoch: EpochField = .none,

    have_year: bool = false,
    year: i32 = 1970,
    have_century: bool = false,
    century: i32 = 19,
    have_year2: bool = false,
    year2: i32 = 70,

    have_month: bool = false,
    month: u32 = 1,
    have_day: bool = false,
    day: u32 = 1,

    have_hour: bool = false,
    hour: u32 = 0,
    hour_is_12: bool = false,
    pm: bool = false,
    have_minute: bool = false,
    minute: u32 = 0,
    have_second: bool = false,
    second: u32 = 0,

    have_zone: bool = false,
    zone_offset: i32 = 0,
};

/// Skip whitespace in *s* starting at *i*; return the new index.
fn scan_skip_ws(s: []const u8, i_in: usize) usize {
    var i = i_in;
    while (i < s.len and (s[i] == ' ' or s[i] == '\t' or
        s[i] == '\n' or s[i] == '\r')) : (i += 1) {}
    return i;
}

/// Read up to ``max_digits`` decimal digits.  Returns ``(value, n_consumed)``
/// or null on no-digit / non-overflow miss.  An overflow during the
/// digit accumulator surfaces as the dedicated ``IntReadResult.over``
/// variant via :func:`scan_read_int_typed`; the legacy ``?{...}``
/// shape collapses overflow → null for callers that don't care.
const IntReadResult = union(enum) {
    ok: struct { v: i64, i: usize },
    over,
    miss,
};

fn scan_read_int_typed(
    s: []const u8,
    i_in: usize,
    max_digits: usize,
    allow_sign: bool,
) IntReadResult {
    var i = i_in;
    var neg = false;
    if (allow_sign and i < s.len and (s[i] == '-' or s[i] == '+')) {
        if (s[i] == '-') neg = true;
        i += 1;
    }
    // ``%e`` / ``%k`` accept a leading space before the digit.
    if (i < s.len and s[i] == ' ') i += 1;
    // Accumulate magnitude as u64 so the i64::MIN literal
    // (``-9223372036854775808``) survives the lex pass even though
    // its absolute value is one more than i64::MAX.  Overflow only
    // fires when the accumulator can't even hold the magnitude in
    // u64 (>20 digits without saturation).
    var mag: u64 = 0;
    var n: usize = 0;
    while (i < s.len and n < max_digits and s[i] >= '0' and s[i] <= '9') : (i += 1) {
        const m = @mulWithOverflow(mag, @as(u64, 10));
        if (m[1] != 0) return .over;
        const d: u64 = @intCast(s[i] - '0');
        const a = @addWithOverflow(m[0], d);
        if (a[1] != 0) return .over;
        mag = a[0];
        n += 1;
    }
    if (n == 0) return .miss;
    // ``abs(i64::MIN) == 2^63``.  Computed via integer literal so no
    // intermediate signed cast can panic on the boundary value.
    const I64_MIN_ABS: u64 = 9223372036854775808;
    const I64_MAX_U: u64 = 9223372036854775807;
    if (neg) {
        if (mag > I64_MIN_ABS) return .over;
        if (mag == I64_MIN_ABS) {
            return .{ .ok = .{ .v = std.math.minInt(i64), .i = i } };
        }
        return .{ .ok = .{ .v = -@as(i64, @intCast(mag)), .i = i } };
    }
    if (mag > I64_MAX_U) return .over;
    return .{ .ok = .{ .v = @as(i64, @intCast(mag)), .i = i } };
}

fn scan_read_int(
    s: []const u8,
    i_in: usize,
    max_digits: usize,
    allow_sign: bool,
) ?struct { v: i64, i: usize } {
    return switch (scan_read_int_typed(s, i_in, max_digits, allow_sign)) {
        .ok => |r| .{ .v = r.v, .i = r.i },
        .over, .miss => null,
    };
}

/// Consume an unambiguous 2..N character month-name prefix at
/// ``s[i..]``.  Returns the month + end index on hit.  Used by the
/// ``en_US_roman`` locale path which accepts ``ja`` for January
/// (clock-6.13).  Ambiguous prefixes (``j``, ``ma``) deliberately
/// fail — picking one would silently corrupt scan results.
fn scan_match_month_prefix(s: []const u8, i_in: usize) ?struct { m: u32, i: usize } {
    // Use 2 leading bytes; longer matches first via the regular
    // ``scan_match_month`` route, so anything that hits here is
    // shorter than 3 letters.
    if (i_in + 2 > s.len) return null;
    const a = ascii_lower(s[i_in]);
    const b = ascii_lower(s[i_in + 1]);
    // Hand-coded unambiguous 2-letter prefixes.  Tcl's reference
    // ``en_US_roman`` table treats each letter through the abbreviator
    // table; we replicate the unambiguous subset.
    const m: u32 = blk: {
        if (a == 'j' and b == 'a') break :blk 1; // January
        if (a == 'f') break :blk 2; // February
        if (a == 'a' and b == 'p') break :blk 4; // April
        if (a == 'a' and b == 'u') break :blk 8; // August
        if (a == 's') break :blk 9; // September
        if (a == 'o') break :blk 10; // October
        if (a == 'n') break :blk 11; // November
        if (a == 'd') break :blk 12; // December
        return null;
    };
    var end = i_in + 2;
    while (end < s.len) {
        const c = ascii_lower(s[end]);
        if (c >= 'a' and c <= 'z') end += 1 else break;
    }
    return .{ .m = m, .i = end };
}

/// Consume a month name (full or abbreviated, case-insensitive) at
/// ``s[i..]``.  Returns ``(month_1based, end_index)`` or null.
fn scan_match_month(s: []const u8, i_in: usize) ?struct { m: u32, i: usize } {
    // Try each candidate; longer matches first so ``January`` doesn't
    // get truncated to ``Jan``.  The MONTH_FULL / MONTH_ABBR tables in
    // ``render_format`` are MixedCase but we match case-insensitively
    // via :func:`ieq`.
    inline for (.{ MONTH_NAMES_FULL, MONTH_NAMES_ABBR }) |table| {
        for (table, 0..) |name, idx| {
            if (i_in + name.len > s.len) continue;
            if (ieq(s[i_in .. i_in + name.len], name)) {
                return .{ .m = @intCast(idx + 1), .i = i_in + name.len };
            }
        }
    }
    return null;
}

/// Consume ``AM`` / ``PM`` / ``am`` / ``pm`` (exactly two chars).
fn scan_match_am_pm(s: []const u8, i_in: usize) ?struct { pm: bool, i: usize } {
    if (i_in + 2 > s.len) return null;
    const c0 = ascii_lower(s[i_in]);
    const c1 = ascii_lower(s[i_in + 1]);
    if (c1 != 'm') return null;
    if (c0 == 'a') return .{ .pm = false, .i = i_in + 2 };
    if (c0 == 'p') return .{ .pm = true, .i = i_in + 2 };
    return null;
}

/// Consume a weekday name.  Return value is the 0..6 index but
/// callers ignore it for now (not used in the pack step).
fn scan_match_weekday(s: []const u8, i_in: usize) ?struct { i: usize } {
    inline for (.{ WEEKDAY_FULL, WEEKDAY_ABBR }) |table| {
        for (table) |name| {
            if (i_in + name.len > s.len) continue;
            if (ieq(s[i_in .. i_in + name.len], name)) {
                return .{ .i = i_in + name.len };
            }
        }
    }
    return null;
}

/// Parse ``±HHMM`` / ``±HH:MM`` zone offset.
fn scan_match_zone_offset(s: []const u8, i_in: usize) ?struct { off: i32, i: usize } {
    if (i_in >= s.len) return null;
    const c = s[i_in];
    if (c != '+' and c != '-') return null;
    var i = i_in + 1;
    const hh = scan_read_int(s, i, 2, false) orelse return null;
    i = hh.i;
    var mm: i64 = 0;
    if (i < s.len and s[i] == ':') i += 1;
    if (i < s.len and s[i] >= '0' and s[i] <= '9') {
        const m = scan_read_int(s, i, 2, false) orelse return null;
        mm = m.v;
        i = m.i;
    }
    const sign: i32 = if (c == '-') -1 else 1;
    return .{ .off = sign * @as(i32, @intCast(hh.v * 3600 + mm * 60)), .i = i };
}

/// Step past one zone-name token (``UTC`` / ``GMT`` / ``EST`` / …).
/// We don't validate the name — Tcl's reference parser also tolerates
/// any letter run here.
fn scan_skip_zone_name(s: []const u8, i_in: usize) usize {
    var i = i_in;
    while (i < s.len) : (i += 1) {
        const c = s[i];
        if (!((c >= 'A' and c <= 'Z') or (c >= 'a' and c <= 'z') or c == '/' or c == '_')) break;
    }
    return i;
}

/// Drive the format string against the input, populating *st*.
/// Returns ``no_match`` on any mismatch / unknown spec, ``overflow``
/// for arithmetic blow-ups, ``none`` on success.
/// Result of a partial scan: the error code plus the input cursor
/// at the point the matcher stopped.  The top-level entry checks
/// that the cursor lands on (whitespace-trimmed) end-of-input.
const ScanPartialResult = struct {
    err: ScanFormatErr,
    ii: usize,
};

fn scan_drive(fmt: []const u8, input: []const u8, st: *ScanState, loc: Locale) ScanFormatErr {
    const r = scan_drive_part(fmt, input, 0, st, loc);
    if (r.err != .none) return r.err;
    if (r.ii != input.len) {
        // Trailing input that the format didn't consume.  Allow only
        // trailing whitespace (Tcl's scan is lenient here).
        const trail = scan_skip_ws(input, r.ii);
        if (trail != input.len) return .no_match;
    }
    return .none;
}

fn scan_drive_part(
    fmt: []const u8,
    input: []const u8,
    ii_in: usize,
    st: *ScanState,
    loc: Locale,
) ScanPartialResult {
    var fi: usize = 0;
    var ii: usize = ii_in;
    while (fi < fmt.len) {
        const fc = fmt[fi];
        if (fc == ' ' or fc == '\t') {
            // One-or-more whitespace in fmt matches zero-or-more in input.
            fi += 1;
            ii = scan_skip_ws(input, ii);
            continue;
        }
        if (fc != '%') {
            // Literal char must match.
            if (ii >= input.len or input[ii] != fc)
                return .{ .err = .no_match, .ii = ii };
            fi += 1;
            ii += 1;
            continue;
        }
        // ``%`` followed by spec.  A trailing bare ``%`` (with no
        // spec letter behind it) is treated as a literal ``%`` —
        // matches reference Tcl's "no token parsing" behaviour
        // (clock-6.19).
        fi += 1;
        if (fi >= fmt.len) {
            if (ii >= input.len or input[ii] != '%')
                return .{ .err = .no_match, .ii = ii };
            ii += 1;
            continue;
        }
        var spec = fmt[fi];
        fi += 1;
        var ext_O = false;
        var ext_E = false;
        if (spec == 'O' or spec == 'E') {
            // Locale modifier — only consumed when the *next* char is
            // an alphabetic spec letter.  Reference Tcl treats
            // ``%E%`` / ``%O5`` etc. as a *literal* ``%E`` / ``%O``
            // sequence (clock-6.19 ``no token parsing`` case): the
            // ``%`` is the literal-percent escape, ``E`` / ``O`` the
            // following character.  Without this guard our matcher
            // would consume ``%`` as the "spec" and fall over.
            const next_ok = fi < fmt.len and
                ((fmt[fi] >= 'a' and fmt[fi] <= 'z') or
                    (fmt[fi] >= 'A' and fmt[fi] <= 'Z'));
            if (next_ok) {
                if (spec == 'O') ext_O = true else ext_E = true;
                spec = fmt[fi];
                fi += 1;
            } else {
                // Treat as literal ``%`` followed by ``E`` / ``O``.
                if (ii >= input.len or input[ii] != '%')
                    return .{ .err = .no_match, .ii = ii };
                ii += 1;
                if (ii >= input.len or input[ii] != spec)
                    return .{ .err = .no_match, .ii = ii };
                ii += 1;
                continue;
            }
        }
        const roman_o = ext_O and loc == .en_us_roman;
        const roman_e = ext_E and loc == .en_us_roman;
        // Skip leading whitespace for any numeric spec — strptime
        // semantics: ``%d`` matches `` 5`` as well as ``05``.  ``%n`` /
        // ``%t`` get the same treatment (any whitespace).
        switch (spec) {
            '%' => {
                if (ii >= input.len or input[ii] != '%') return .{ .err = .no_match, .ii = ii };
                ii += 1;
            },
            'n', 't' => {
                // Tcl ``%n`` / ``%t`` match any run of whitespace
                // (including ``\t`` literals), not just a single
                // newline / tab.  Skip greedily; the caller's
                // surrounding literals re-anchor.
                ii = scan_skip_ws(input, ii);
            },
            's' => {
                ii = scan_skip_ws(input, ii);
                switch (scan_read_int_typed(input, ii, 20, true)) {
                    .ok => |r| {
                        st.have_seconds = true;
                        st.seconds = r.v;
                        st.last_epoch = .seconds;
                        ii = r.i;
                    },
                    .over => return .{ .err = .overflow, .ii = ii },
                    .miss => return .{ .err = .no_match, .ii = ii },
                }
            },
            'J' => {
                ii = scan_skip_ws(input, ii);
                if (ext_E) {
                    // ``%EJ`` — signed calendar Julian day with
                    // optional fractional part.  Anchored at
                    // midnight (.0 = midnight of Gregorian day);
                    // contrast ``%Ej`` which anchors at noon.
                    // clock-7.20 covers both.
                    const r = scan_read_int(input, ii, 12, true) orelse
                        return .{ .err = .no_match, .ii = ii };
                    var jdn = r.v;
                    var aj_secs: i64 = 0;
                    var ki = r.i;
                    if (ki < input.len and input[ki] == '.') {
                        ki += 1;
                        var frac_num: i64 = 0;
                        var frac_den: i64 = 1;
                        var fdigits: usize = 0;
                        while (ki < input.len and input[ki] >= '0' and
                            input[ki] <= '9' and fdigits < 6) : (ki += 1)
                        {
                            frac_num = frac_num * 10 + (input[ki] - '0');
                            frac_den *= 10;
                            fdigits += 1;
                        }
                        while (ki < input.len and input[ki] >= '0' and input[ki] <= '9')
                            : (ki += 1)
                        {}
                        const frac_secs = @divFloor(frac_num * SECS_PER_DAY, frac_den);
                        var combined = frac_secs;
                        while (combined >= SECS_PER_DAY) {
                            combined -= SECS_PER_DAY;
                            jdn += 1;
                        }
                        aj_secs = combined;
                    }
                    st.have_astro_julian = true;
                    st.aj_day = jdn;
                    st.aj_secs = aj_secs;
                    st.last_epoch = .astro_julian;
                    ii = ki;
                } else {
                    const r = scan_read_int(input, ii, 12, true) orelse
                        return .{ .err = .no_match, .ii = ii };
                    st.have_julian = true;
                    st.julian = r.v;
                    st.last_epoch = .julian;
                    ii = r.i;
                }
            },
            'Y' => {
                ii = scan_skip_ws(input, ii);
                if (roman_e) {
                    const r = read_roman(input, ii) orelse return .{ .err = .no_match, .ii = ii };
                    st.have_year = true;
                    st.year = @intCast(r.v);
                    ii = r.i;
                } else {
                    const r = scan_read_int(input, ii, 4, true) orelse return .{ .err = .no_match, .ii = ii };
                    st.have_year = true;
                    st.year = @intCast(r.v);
                    ii = r.i;
                }
            },
            'y' => {
                ii = scan_skip_ws(input, ii);
                if (roman_o) {
                    const r = read_roman(input, ii) orelse return .{ .err = .no_match, .ii = ii };
                    st.have_year2 = true;
                    st.year2 = @intCast(r.v);
                    ii = r.i;
                } else {
                    // Reference Tcl's strptime is lenient on ``%y``:
                    // accepts 1..4 digits, treating a 3-or-4 digit
                    // run as the full year (``%Y`` semantics) and a
                    // 1-or-2 digit run as the century-split short
                    // form.  clock-8.50 ``%D`` against ``01/02/1970``
                    // relies on this — ``%y`` consumes ``1970``
                    // outright instead of stopping at ``19``.
                    const r = scan_read_int(input, ii, 4, false) orelse return .{ .err = .no_match, .ii = ii };
                    if (r.v > 99) {
                        st.have_year = true;
                        st.year = @intCast(r.v);
                    } else {
                        st.have_year2 = true;
                        st.year2 = @intCast(r.v);
                    }
                    ii = r.i;
                }
            },
            'C' => {
                ii = scan_skip_ws(input, ii);
                if (roman_e) {
                    const r = read_roman(input, ii) orelse return .{ .err = .no_match, .ii = ii };
                    st.have_century = true;
                    // ``%EC`` reads as a multiple of 100 (e.g. ``mdccc``
                    // → 1800), so divide before storing.
                    st.century = @intCast(@divFloor(r.v, 100));
                    ii = r.i;
                } else {
                    const r = scan_read_int(input, ii, 2, false) orelse return .{ .err = .no_match, .ii = ii };
                    st.have_century = true;
                    st.century = @intCast(r.v);
                    ii = r.i;
                }
            },
            'm', 'N' => {
                ii = scan_skip_ws(input, ii);
                if (roman_o) {
                    const r = read_roman(input, ii) orelse return .{ .err = .no_match, .ii = ii };
                    if (r.v < 1 or r.v > 12) return .{ .err = .no_match, .ii = ii };
                    st.have_month = true;
                    st.month = @intCast(r.v);
                    ii = r.i;
                } else {
                    const r = scan_read_int(input, ii, 2, false) orelse return .{ .err = .no_match, .ii = ii };
                    if (r.v < 1 or r.v > 12) return .{ .err = .no_match, .ii = ii };
                    st.have_month = true;
                    st.month = @intCast(r.v);
                    ii = r.i;
                }
            },
            'b', 'B', 'h' => {
                ii = scan_skip_ws(input, ii);
                if (scan_match_month(input, ii)) |r| {
                    st.have_month = true;
                    st.month = r.m;
                    ii = r.i;
                } else if (loc == .en_us_roman) {
                    // ``en_US_roman`` accepts any unambiguous 2-letter
                    // month prefix (``ja`` / ``au`` / etc.).  The
                    // ambiguous prefixes (``j`` matches Jan/Jun/Jul,
                    // ``ma`` matches Mar/May, ``a`` matches Apr/Aug)
                    // need ``-locale`` resolution that we don't ship;
                    // they fail the parse here instead of guessing.
                    const r = scan_match_month_prefix(input, ii) orelse return .{ .err = .no_match, .ii = ii };
                    st.have_month = true;
                    st.month = r.m;
                    ii = r.i;
                } else {
                    return .{ .err = .no_match, .ii = ii };
                }
            },
            'd', 'e' => {
                ii = scan_skip_ws(input, ii);
                if (roman_o) {
                    const r = read_roman(input, ii) orelse return .{ .err = .no_match, .ii = ii };
                    if (r.v < 1 or r.v > 31) return .{ .err = .no_match, .ii = ii };
                    st.have_day = true;
                    st.day = @intCast(r.v);
                    ii = r.i;
                } else {
                    const r = scan_read_int(input, ii, 2, false) orelse return .{ .err = .no_match, .ii = ii };
                    if (r.v < 1 or r.v > 31) return .{ .err = .no_match, .ii = ii };
                    st.have_day = true;
                    st.day = @intCast(r.v);
                    ii = r.i;
                }
            },
            'j' => {
                ii = scan_skip_ws(input, ii);
                if (ext_E) {
                    // ``%Ej`` — astronomical Julian day with optional
                    // fractional part.  Stored as integer day +
                    // partial-day seconds so we don't accumulate f64
                    // rounding error at large JDs.  Astronomical
                    // convention anchors noon at JD .0 (so .5
                    // = midnight of the next Gregorian day).
                    const r = scan_read_int(input, ii, 12, true) orelse return .{ .err = .no_match, .ii = ii };
                    var jdn = r.v;
                    var aj_secs: i64 = @divFloor(SECS_PER_DAY, 2);
                    var ki = r.i;
                    if (ki < input.len and input[ki] == '.') {
                        ki += 1;
                        var frac_num: i64 = 0;
                        var frac_den: i64 = 1;
                        var fdigits: usize = 0;
                        while (ki < input.len and input[ki] >= '0' and input[ki] <= '9' and fdigits < 6) : (ki += 1) {
                            frac_num = frac_num * 10 + (input[ki] - '0');
                            frac_den *= 10;
                            fdigits += 1;
                        }
                        while (ki < input.len and input[ki] >= '0' and input[ki] <= '9') : (ki += 1) {}
                        const total_secs_in_day: i64 = SECS_PER_DAY;
                        const half: i64 = @divFloor(total_secs_in_day, 2);
                        const frac_secs = @divFloor(frac_num * total_secs_in_day, frac_den);
                        var combined = frac_secs + half;
                        while (combined >= total_secs_in_day) {
                            combined -= total_secs_in_day;
                            jdn += 1;
                        }
                        aj_secs = combined;
                    }
                    st.have_astro_julian = true;
                    st.aj_day = jdn;
                    st.aj_secs = aj_secs;
                    st.last_epoch = .astro_julian;
                    ii = ki;
                } else {
                    const r = scan_read_int(input, ii, 3, false) orelse return .{ .err = .no_match, .ii = ii };
                    if (r.v < 1 or r.v > 366) return .{ .err = .no_match, .ii = ii };
                    ii = r.i;
                    // day-of-year not currently fed into pack_epoch.
                }
            },
            'H', 'k' => {
                ii = scan_skip_ws(input, ii);
                if (roman_o) {
                    if (ii < input.len and input[ii] == '?') {
                        ii += 1;
                        st.have_hour = true;
                        st.hour_is_12 = false;
                        st.hour = 0;
                    } else {
                        const r = read_roman(input, ii) orelse return .{ .err = .no_match, .ii = ii };
                        if (r.v < 0 or r.v > 23) return .{ .err = .no_match, .ii = ii };
                        st.have_hour = true;
                        st.hour_is_12 = false;
                        st.hour = @intCast(r.v);
                        ii = r.i;
                    }
                } else {
                    const r = scan_read_int(input, ii, 2, false) orelse return .{ .err = .no_match, .ii = ii };
                    if (r.v < 0 or r.v > 23) return .{ .err = .no_match, .ii = ii };
                    st.have_hour = true;
                    st.hour_is_12 = false;
                    st.hour = @intCast(r.v);
                    ii = r.i;
                }
            },
            'I', 'l' => {
                ii = scan_skip_ws(input, ii);
                if (roman_o) {
                    const r = read_roman(input, ii) orelse return .{ .err = .no_match, .ii = ii };
                    if (r.v < 1 or r.v > 12) return .{ .err = .no_match, .ii = ii };
                    st.have_hour = true;
                    st.hour_is_12 = true;
                    st.hour = @intCast(r.v);
                    ii = r.i;
                } else {
                    const r = scan_read_int(input, ii, 2, false) orelse return .{ .err = .no_match, .ii = ii };
                    if (r.v < 1 or r.v > 12) return .{ .err = .no_match, .ii = ii };
                    st.have_hour = true;
                    st.hour_is_12 = true;
                    st.hour = @intCast(r.v);
                    ii = r.i;
                }
            },
            'M' => {
                ii = scan_skip_ws(input, ii);
                if (roman_o) {
                    if (ii < input.len and input[ii] == '?') {
                        ii += 1;
                        st.have_minute = true;
                        st.minute = 0;
                    } else {
                        const r = read_roman(input, ii) orelse return .{ .err = .no_match, .ii = ii };
                        if (r.v < 0 or r.v > 59) return .{ .err = .no_match, .ii = ii };
                        st.have_minute = true;
                        st.minute = @intCast(r.v);
                        ii = r.i;
                    }
                } else {
                    const r = scan_read_int(input, ii, 2, false) orelse return .{ .err = .no_match, .ii = ii };
                    if (r.v < 0 or r.v > 59) return .{ .err = .no_match, .ii = ii };
                    st.have_minute = true;
                    st.minute = @intCast(r.v);
                    ii = r.i;
                }
            },
            'S' => {
                ii = scan_skip_ws(input, ii);
                if (roman_o) {
                    if (ii < input.len and input[ii] == '?') {
                        ii += 1;
                        st.have_second = true;
                        st.second = 0;
                    } else {
                        const r = read_roman(input, ii) orelse return .{ .err = .no_match, .ii = ii };
                        if (r.v < 0 or r.v > 60) return .{ .err = .no_match, .ii = ii };
                        st.have_second = true;
                        st.second = @intCast(r.v);
                        ii = r.i;
                    }
                } else {
                    const r = scan_read_int(input, ii, 2, false) orelse return .{ .err = .no_match, .ii = ii };
                    if (r.v < 0 or r.v > 60) return .{ .err = .no_match, .ii = ii };
                    st.have_second = true;
                    st.second = @intCast(r.v);
                    ii = r.i;
                }
            },
            'p', 'P' => {
                ii = scan_skip_ws(input, ii);
                const r = scan_match_am_pm(input, ii) orelse return .{ .err = .no_match, .ii = ii };
                st.pm = r.pm;
                ii = r.i;
            },
            'a', 'A' => {
                ii = scan_skip_ws(input, ii);
                const r = scan_match_weekday(input, ii) orelse return .{ .err = .no_match, .ii = ii };
                ii = r.i;
            },
            'z' => {
                ii = scan_skip_ws(input, ii);
                if (ii >= input.len) {
                    // ``%z`` at the very end with no input left
                    // — treat as missing zone (clock-6.18 case
                    // ``2009-06-30T18:30:00`` against ``%...%z``).
                } else if (input[ii] == 'Z' or input[ii] == 'z') {
                    st.have_zone = true;
                    st.zone_offset = 0;
                    ii += 1;
                } else if (input[ii] == '+' or input[ii] == '-') {
                    const r = scan_match_zone_offset(input, ii) orelse return .{ .err = .no_match, .ii = ii };
                    st.have_zone = true;
                    st.zone_offset = r.off;
                    ii = r.i;
                }
                // Anything else: zone is optional, skip silently.
            },
            'Z' => {
                ii = scan_skip_ws(input, ii);
                ii = scan_skip_zone_name(input, ii);
            },
            // Composite specs — recursively expand and re-scan.
            // ``%c`` / ``%x`` / ``%X`` consult the locale table;
            // ``%D`` / ``%R`` / ``%T`` / ``%F`` are fixed templates.
            // ``%r`` is 12-hour AM/PM clock.  Each delegates to a
            // nested ``scan_drive_part`` so the substitution string
            // is matched at the same input cursor and the partial
            // ``ScanState`` accumulates.
            'c', 'x', 'X' => {
                if (ext_O) return .{ .err = .no_match, .ii = ii };
                const sub = locale_replace(loc, spec, ext_E) orelse "";
                if (sub.len == 0) return .{ .err = .no_match, .ii = ii };
                const sub_r = scan_drive_part(sub, input, ii, st, loc);
                if (sub_r.err != .none) return sub_r;
                ii = sub_r.ii;
            },
            'D' => {
                const sub_r = scan_drive_part("%m/%d/%y", input, ii, st, loc);
                if (sub_r.err != .none) return sub_r;
                ii = sub_r.ii;
            },
            'R' => {
                const sub_r = scan_drive_part("%H:%M", input, ii, st, loc);
                if (sub_r.err != .none) return sub_r;
                ii = sub_r.ii;
            },
            'T' => {
                const sub_r = scan_drive_part("%H:%M:%S", input, ii, st, loc);
                if (sub_r.err != .none) return sub_r;
                ii = sub_r.ii;
            },
            'F' => {
                const sub_r = scan_drive_part("%Y-%m-%d", input, ii, st, loc);
                if (sub_r.err != .none) return sub_r;
                ii = sub_r.ii;
            },
            'r' => {
                const sub_r = scan_drive_part("%I:%M:%S %p", input, ii, st, loc);
                if (sub_r.err != .none) return sub_r;
                ii = sub_r.ii;
            },
            else => return .{ .err = .no_match, .ii = ii },
        }
    }
    return .{ .err = .none, .ii = ii };
}

/// Resolve the partial broken-down state into Unix epoch seconds.
/// Year is composed from %Y, %C+%y, or %y alone (Tcl's century split:
/// 00..68 → 2000..2068, 69..99 → 1969..1999).  Calendar fields default
/// to 1970-01-01 00:00:00 when not provided — matches tclsh's
/// "missing fields zero out" behaviour.
fn scan_resolve(st: *const ScanState) ?i64 {
    // ``%s`` / ``%J`` / ``%Ej`` are all "absolute epoch" specs.
    // Tcl's reference applies a precedence here:
    //
    //   * ``%s`` (epoch seconds) outranks ``%J`` (calendar Julian day)
    //     — clock-7.8 ``{%J %s}`` returns the seconds value regardless
    //     of order.
    //   * ``%s`` and ``%Ej`` (astronomical Julian) are equal-rank;
    //     last-wins by scan order — clock-7.18.
    //   * ``%J`` and ``%Ej`` follow the same rule pair-wise.
    //
    // We encode this by giving ``%s`` priority over plain ``%J``,
    // and using ``last_epoch`` as the tiebreaker between any other
    // pair.  ``last_epoch`` defaults to ``.none``; in that case fall
    // through to the calendar pack.
    const have_any_epoch = st.have_seconds or st.have_julian or st.have_astro_julian;
    if (have_any_epoch) {
        // Decide which field wins.
        var winner: EpochField = .none;
        if (st.have_seconds and !st.have_astro_julian) {
            winner = .seconds;
        } else if (st.have_astro_julian and !st.have_seconds and !st.have_julian) {
            winner = .astro_julian;
        } else if (st.have_julian and !st.have_seconds and !st.have_astro_julian) {
            winner = .julian;
        } else {
            // Mixed set — last wins.
            winner = st.last_epoch;
            // ``%s`` always beats plain ``%J`` regardless of order.
            if (st.have_seconds and st.have_julian and !st.have_astro_julian) {
                winner = .seconds;
            }
        }
        return switch (winner) {
            .seconds => st.seconds,
            .astro_julian => blk: {
                const jd_unix: i64 = 2440588;
                const days = st.aj_day - jd_unix;
                const m = @mulWithOverflow(days, @as(i64, SECS_PER_DAY));
                if (m[1] != 0) break :blk null;
                const a = @addWithOverflow(m[0], st.aj_secs);
                if (a[1] != 0) break :blk null;
                break :blk a[0];
            },
            .julian => blk: {
                const jd_unix: i64 = 2440588;
                const days = st.julian - jd_unix;
                const m = @mulWithOverflow(days, @as(i64, SECS_PER_DAY));
                if (m[1] != 0) break :blk null;
                break :blk m[0];
            },
            .none => null,
        };
    }
    var year: i32 = 1970;
    if (st.have_year) {
        year = st.year;
    } else if (st.have_century and st.have_year2) {
        year = st.century * 100 + st.year2;
    } else if (st.have_year2) {
        year = if (st.year2 < 69) 2000 + st.year2 else 1900 + st.year2;
    }
    const month: u32 = if (st.have_month) st.month else 1;
    const day: u32 = if (st.have_day) st.day else 1;
    var hour: u32 = if (st.have_hour) st.hour else 0;
    if (st.have_hour and st.hour_is_12) {
        // %I/%p combo: 12 AM = 00, 12 PM = 12, 1..11 PM = 13..23.
        if (hour == 12) hour = 0;
        if (st.pm) hour += 12;
    }
    const minute: u32 = if (st.have_minute) st.minute else 0;
    const second: u32 = if (st.have_second) st.second else 0;
    if (year < -9999 or year > 9999) return null;
    if (day < 1 or day > days_in_month(year, month)) return null;
    return pack_epoch(year, month, day, hour, minute, second);
}

/// Raise a Tcl error with explicit ``::errorCode`` matching reference
/// Tcl's clock-side diagnostics (``CLOCK badInputString`` /
/// ``CLOCK dateTooLarge``).  The message + code pair is what
/// ``catch ... opt; dict get $opt -errorcode`` observes.
fn raise_clock_error(msg: []const u8, code: []const u8) void {
    const obj_new_string_copy = obj.obj_new_string_copy;
    const msg_obj = obj_new_string_copy(@intCast(@intFromPtr(msg.ptr)), @intCast(msg.len));
    const code_obj = obj_new_string_copy(@intCast(@intFromPtr(code.ptr)), @intCast(code.len));
    const tcl_catch = @import("../interp/tcl_catch.zig");
    tcl_catch.tcl_cmd_error_full(msg_obj, 0, code_obj);
}

/// clock_scan_format — strftime-driven scan.  Returns a TclObj integer
/// holding the resolved epoch.  On failure the error path through
/// ``catch_mod.tcl_cmd_error_full`` stamps ``::errorCode`` with the
/// reference Tcl values (``CLOCK badInputString`` for parse misses,
/// ``CLOCK dateTooLarge`` for arithmetic overflow); when called
/// outside a ``catch`` the runtime traps with that message on stderr.
pub export fn clock_scan_format(
    text_obj: i32,
    fmt_obj: i32,
    gmt_flag: i32,
    zone_obj: i32,
    base_obj: i32,
    locale_obj: i32,
) i32 {
    _ = base_obj; // base only matters when format-less; format path
    // explicitly fills every field it cares about.
    if (text_obj == 0 or fmt_obj == 0) {
        raise_clock_error("input string does not match supplied format", "CLOCK badInputString");
        return obj_new_int(0);
    }
    const t = obj_ensure_string(text_obj);
    const f = obj_ensure_string(fmt_obj);
    if (t.ptr == 0 or t.len == 0 or f.ptr == 0 or f.len == 0) {
        raise_clock_error("input string does not match supplied format", "CLOCK badInputString");
        return obj_new_int(0);
    }
    const tp: [*]const u8 = @ptrFromInt(t.ptr);
    const fp: [*]const u8 = @ptrFromInt(f.ptr);
    const text = tp[0..t.len];
    const fmt = fp[0..f.len];

    const loc_slice: []const u8 = blk: {
        if (locale_obj == 0) break :blk &[_]u8{};
        const s = obj_ensure_string(locale_obj);
        if (s.ptr == 0 or s.len == 0) break :blk &[_]u8{};
        const p: [*]const u8 = @ptrFromInt(s.ptr);
        break :blk p[0..s.len];
    };
    const loc = locale_from(loc_slice);

    var st = ScanState{};
    const err = scan_drive(fmt, text, &st, loc);
    switch (err) {
        .none => {},
        .no_match => {
            raise_clock_error("input string does not match supplied format", "CLOCK badInputString");
            return obj_new_int(0);
        },
        .overflow => {
            raise_clock_error("integer value too large to represent", "CLOCK dateTooLarge");
            return obj_new_int(0);
        },
    }
    const local_epoch = scan_resolve(&st) orelse {
        // Resolve failure is overflow (JD multiplied past i64) or
        // an out-of-range calendar date.  Tag both as
        // ``dateTooLarge`` — the upstream reference uses the same
        // error code for either condition (``tclClockFmt.c``
        // ``ClockScanFmt`` calls ``Tcl_SetObjResult`` with the
        // generic message in both branches).
        raise_clock_error("requested date too large to represent", "CLOCK dateTooLarge");
        return obj_new_int(0);
    };

    // ``%J`` / ``%EJ`` / ``%Ej`` must reject inputs that would
    // overflow the calendar window.  clock-7.6 (``5373485``),
    // 7.7 (``2147483648``), and 7.16 (``%Ej`` astronomical
    // overflow with both integer and fractional inputs above
    // 5373484) expect the dateTooLarge error.
    const jd_max: i64 = 5373484; // last representable Gregorian day
    const jd_min: i64 = -1095;   // far past the proleptic horizon
    if (st.have_julian) {
        if (st.julian > jd_max or st.julian < jd_min) {
            raise_clock_error("requested date too large to represent", "CLOCK dateTooLarge");
            return obj_new_int(0);
        }
    }
    if (st.have_astro_julian) {
        // Astronomical convention shifts ``.0`` from midnight to noon,
        // so the half-day at ``5373484.5`` (=``Ej`` for 9999-12-31
        // 00:00) is at the boundary.  Anything strictly greater is
        // beyond the representable range and must raise.
        if (st.aj_day > jd_max or st.aj_day < jd_min) {
            raise_clock_error("requested date too large to represent", "CLOCK dateTooLarge");
            return obj_new_int(0);
        }
        // Boundary day with a non-zero offset that would push past
        // the end of 9999-12-31 also overflows.
        if (st.aj_day == jd_max and st.aj_secs > @divFloor(SECS_PER_DAY, 2)) {
            raise_clock_error("requested date too large to represent", "CLOCK dateTooLarge");
            return obj_new_int(0);
        }
    }
    if (st.have_seconds) {
        // ``%s`` overflow is reported during the lexer pass; nothing
        // extra to check here.  Fall through.
    }

    if (st.have_zone) {
        return obj_new_int(local_epoch - @as(i64, st.zone_offset));
    }
    if (gmt_flag != 0) return obj_new_int(local_epoch);

    // Epoch-style results (%s, %J, %Ej) bypass zone math — they're
    // already absolute UTC.  Calendar-only results need the zone
    // offset applied once we know it.
    if (st.have_seconds or st.have_julian or st.have_astro_julian)
        return obj_new_int(local_epoch);

    const zone_slice: []const u8 = blk: {
        if (zone_obj == 0) break :blk &[_]u8{};
        const zs = obj_ensure_string(zone_obj);
        if (zs.ptr == 0 or zs.len == 0) break :blk &[_]u8{};
        const zp: [*]const u8 = @ptrFromInt(zs.ptr);
        break :blk zp[0..zs.len];
    };
    const z: *const tz.TimeZone = if (zone_slice.len == 0)
        tz.resolve_default()
    else
        tz.resolve(zone_slice);
    const off = z.offset_at(local_epoch).utoff;
    return obj_new_int(local_epoch - @as(i64, off));
}

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
/// ``$TZ`` / ``/etc/localtime`` / UTC, in that order.  Backwards-
/// compatible 3-arg entry — calls into :func:`clock_format_tz_locale`
/// with ``en_US``.
pub export fn clock_format_tz(
    seconds_obj: i32,
    fmt_obj: i32,
    zone_obj: i32,
) i32 {
    return clock_format_tz_locale(seconds_obj, fmt_obj, zone_obj, 0);
}

/// clock_format_tz_locale — locale-aware ``clock format``.
/// ``locale_obj`` is a TclObj holding the locale name; ``en_US_roman``
/// drives the roman-numeral substitution path for ``%O*`` / ``%E*``
/// specs and the alternate composite forms (``%Ec`` / ``%Ex`` /
/// ``%EX``).  Any other value (including empty / null) is treated as
/// ``en_US``.
pub export fn clock_format_tz_locale(
    seconds_obj: i32,
    fmt_obj: i32,
    zone_obj: i32,
    locale_obj: i32,
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
    const loc_slice: []const u8 = blk: {
        if (locale_obj == 0) break :blk &[_]u8{};
        const s = obj_ensure_string(locale_obj);
        if (s.ptr == 0 or s.len == 0) break :blk &[_]u8{};
        const p: [*]const u8 = @ptrFromInt(s.ptr);
        break :blk p[0..s.len];
    };
    return render_format(f.ptr, f.len, local, off_info.utoff, off_info.abbr, locale_from(loc_slice));
}

