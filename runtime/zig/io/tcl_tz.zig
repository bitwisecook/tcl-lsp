// TZif (RFC 8536) parser + zone cache.
//
// Parses the binary tzdata files shipped under /usr/share/zoneinfo
// (and friends) so the WASM runtime can resolve a UTC offset and
// abbreviation for any (zone, epoch) pair without dragging in the
// full Tcl 9 ``library/clock.tcl`` glue.
//
// Scope of this file:
//   * Header + body parsing for v1 / v2 / v3 / v4 TZif blobs
//     (we always prefer the v2+ block when present — its 64-bit
//     transition times cover dates outside the v1 32-bit window).
//   * ``offset_at(secs)`` lookup: binary search the transition
//     table, fall back to the first non-DST type for pre-history,
//     fall back to the trailing POSIX TZ string for post-history
//     (or the last transition's type if the footer is absent /
//     unparseable).
//   * Built-in synthetic UTC zone (no I/O, no allocations) so the
//     ``-gmt 1`` / ``-timezone :UTC`` paths always succeed even on
//     hosts that haven't preopened ``/usr/share/zoneinfo`` into the
//     WASI sandbox.
//
// Out of scope here:
//   * Filesystem probing — that lives in ``tcl_clock.zig`` so this
//     module stays pure-parse and trivially unit-testable.
//   * The bundled trimmed-tzdata fallback blob — also wired up by
//     the resolver in ``tcl_clock.zig``.
//   * POSIX TZ string parsing — supported as a *fallback* (only the
//     fixed-offset ``"<NAME><OFFSET>"`` form, e.g. ``UTC0``).  The
//     full ``"STD3DST,M3.2.0,M11.1.0"`` DST-rule grammar can land
//     once the basic path is in.

const std = @import("std");
const obj = @import("../valtypes/tcl_obj.zig");

/// One ``ttinfo`` record after parsing — UTC offset in seconds,
/// the DST flag, and a slice into the abbreviation char block.
pub const LocalType = struct {
    utoff: i32, // seconds east of UTC (positive = ahead of UTC)
    isdst: bool,
    abbr: []const u8, // borrowed from the parsed blob's abbr area
};

/// Result of an ``offset_at`` lookup — what callers actually need
/// to render a local-time tuple.
pub const Offset = struct {
    utoff: i32,
    isdst: bool,
    abbr: []const u8,
};

/// A parsed TZif blob.  Slice fields point INTO ``raw`` — callers
/// must keep the backing buffer alive for as long as the
/// ``TimeZone`` value.  The cache layer in :file:`tcl_clock.zig`
/// owns lifetime and only frees a ``TimeZone`` when the cache
/// itself is wiped, so this borrow is safe in practice.
pub const TimeZone = struct {
    /// 64-bit transition times (seconds since the Unix epoch).
    /// Sorted ascending — the lookup uses binary search.
    transitions: []const i64,
    /// One byte per transition; index into ``types``.
    type_indices: []const u8,
    /// The zone's local-time-type table.
    types: []const LocalType,
    /// Optional POSIX TZ string from the v2+ trailer (without the
    /// leading newline).  Empty when the blob is v1 or has no
    /// footer.
    posix_tz: []const u8,
    /// First-non-DST type used for timestamps before the first
    /// transition (RFC 8536 §3.3).  -1 means "no transitions, use
    /// types[0]" — handled by ``offset_at``.
    pre_first: i32,

    pub fn offset_at(self: *const TimeZone, secs: i64) Offset {
        if (self.transitions.len == 0) {
            // No history — pick first type, or fall back to UTC.
            if (self.types.len > 0) {
                const t = self.types[0];
                return .{ .utoff = t.utoff, .isdst = t.isdst, .abbr = t.abbr };
            }
            return .{ .utoff = 0, .isdst = false, .abbr = "UTC" };
        }
        if (secs < self.transitions[0]) {
            const idx: usize = if (self.pre_first >= 0)
                @intCast(self.pre_first)
            else
                0;
            const t = self.types[idx];
            return .{ .utoff = t.utoff, .isdst = t.isdst, .abbr = t.abbr };
        }
        // Past the last transition: try the POSIX TZ rule for
        // extrapolation.  Reference Tcl uses the trailer to handle
        // any timestamp beyond the last v2 record (in our test suite,
        // tests like clock-5.432 query 2038 dates that all sit after
        // the v2 horizon of ~2037-11).  Without this extrapolation
        // the resolver would freeze on the last-recorded type and
        // miss every DST transition the rule predicts.
        const last_idx = self.transitions.len - 1;
        if (secs > self.transitions[last_idx] and self.posix_tz.len > 0) {
            if (apply_posix_rule(self.posix_tz, secs)) |r| return r;
        }
        // Binary search for the largest transition <= secs.
        var lo: usize = 0;
        var hi: usize = self.transitions.len;
        while (lo + 1 < hi) {
            const mid = lo + (hi - lo) / 2;
            if (self.transitions[mid] <= secs) lo = mid else hi = mid;
        }
        const ti = self.type_indices[lo];
        if (ti >= self.types.len) {
            // Malformed file — fall back to type 0.
            const t = self.types[0];
            return .{ .utoff = t.utoff, .isdst = t.isdst, .abbr = t.abbr };
        }
        const t = self.types[ti];
        return .{ .utoff = t.utoff, .isdst = t.isdst, .abbr = t.abbr };
    }
};

// -- POSIX TZ-string parser + extrapolator ----------------------------------
//
// The trailer of a v2/v3 TZif file holds a POSIX TZ string that
// describes the zone's behaviour beyond the last 64-bit transition.
// Format (a strict subset of POSIX.1-2017 §8.3):
//
//   STD<offset>[DST[<offset>][,start[/time],end[/time]]]
//
// where ``STD`` / ``DST`` are abbreviations (3+ letters, or
// ``<sign-prefix>`` in angle brackets), ``<offset>`` is hours
// optionally with minutes / seconds, and the rules are
// ``M<month>.<week>.<dow>``.  Sign convention for the offset is
// **west of UTC** (the opposite of TZif's ``utoff``).
//
// We only implement the M-form rule with ``/HH`` time syntax.
// Julian-day forms (``J<n>`` / plain ``<n>``) aren't used by any
// curated zone we ship, so we return null on unrecognised shapes
// and let the resolver fall back to its last recorded transition.

const PosixOff = struct { secs: i32 };

const PosixRule = struct {
    month: u8, // 1..12
    week: u8,  // 1..5 (5 = last week of month)
    dow: u8,   // 0..6 (Sun = 0)
    /// Local-clock seconds since midnight (default 7200 = 02:00).
    time: i32,
};

const PosixTz = struct {
    std_abbr: []const u8,
    std_off_east: i32, // seconds east of UTC
    has_dst: bool,
    dst_abbr: []const u8,
    dst_off_east: i32,
    start_rule: PosixRule,
    end_rule: PosixRule,
};

/// Read an abbreviation: either ``+05`` style or the ``<...>`` form.
fn posix_read_abbr(s: []const u8, i_in: usize) ?struct { abbr: []const u8, i: usize } {
    var i = i_in;
    if (i >= s.len) return null;
    if (s[i] == '<') {
        const start = i + 1;
        i += 1;
        while (i < s.len and s[i] != '>') : (i += 1) {}
        if (i >= s.len) return null;
        return .{ .abbr = s[start..i], .i = i + 1 };
    }
    const start = i;
    while (i < s.len) : (i += 1) {
        const c = s[i];
        if ((c >= 'A' and c <= 'Z') or (c >= 'a' and c <= 'z')) continue;
        break;
    }
    if (i == start) return null;
    return .{ .abbr = s[start..i], .i = i };
}

/// Read a signed POSIX offset (``[-+]?H[:MM[:SS]]``).  The result is
/// in seconds **west of UTC** — POSIX's sign convention.
fn posix_read_offset(s: []const u8, i_in: usize) ?struct { secs: i32, i: usize } {
    var i = i_in;
    if (i >= s.len) return null;
    var sign: i32 = 1;
    if (s[i] == '-') {
        sign = -1;
        i += 1;
    } else if (s[i] == '+') {
        i += 1;
    }
    var h: i32 = 0;
    var n: usize = 0;
    while (i < s.len and s[i] >= '0' and s[i] <= '9') : (i += 1) {
        h = h * 10 + @as(i32, @intCast(s[i] - '0'));
        n += 1;
    }
    if (n == 0) return null;
    var m: i32 = 0;
    var sec: i32 = 0;
    if (i < s.len and s[i] == ':') {
        i += 1;
        while (i < s.len and s[i] >= '0' and s[i] <= '9') : (i += 1) {
            m = m * 10 + @as(i32, @intCast(s[i] - '0'));
        }
        if (i < s.len and s[i] == ':') {
            i += 1;
            while (i < s.len and s[i] >= '0' and s[i] <= '9') : (i += 1) {
                sec = sec * 10 + @as(i32, @intCast(s[i] - '0'));
            }
        }
    }
    return .{ .secs = sign * (h * 3600 + m * 60 + sec), .i = i };
}

fn posix_read_rule(s: []const u8, i_in: usize) ?struct { rule: PosixRule, i: usize } {
    var i = i_in;
    if (i >= s.len or s[i] != 'M') return null;
    i += 1;
    var month: i32 = 0;
    var week: i32 = 0;
    var dow: i32 = 0;
    while (i < s.len and s[i] >= '0' and s[i] <= '9') : (i += 1) {
        month = month * 10 + @as(i32, @intCast(s[i] - '0'));
    }
    if (i >= s.len or s[i] != '.') return null;
    i += 1;
    while (i < s.len and s[i] >= '0' and s[i] <= '9') : (i += 1) {
        week = week * 10 + @as(i32, @intCast(s[i] - '0'));
    }
    if (i >= s.len or s[i] != '.') return null;
    i += 1;
    while (i < s.len and s[i] >= '0' and s[i] <= '9') : (i += 1) {
        dow = dow * 10 + @as(i32, @intCast(s[i] - '0'));
    }
    if (month < 1 or month > 12 or week < 1 or week > 5 or dow < 0 or dow > 6) {
        return null;
    }
    var time: i32 = 7200; // default 02:00
    if (i < s.len and s[i] == '/') {
        i += 1;
        const off = posix_read_offset(s, i) orelse return null;
        time = off.secs;
        i = off.i;
    }
    return .{
        .rule = .{
            .month = @intCast(month),
            .week = @intCast(week),
            .dow = @intCast(dow),
            .time = time,
        },
        .i = i,
    };
}

fn parse_posix_tz(s: []const u8) ?PosixTz {
    var i: usize = 0;
    const std_a = posix_read_abbr(s, i) orelse return null;
    i = std_a.i;
    const std_o = posix_read_offset(s, i) orelse return null;
    i = std_o.i;
    var pz = PosixTz{
        .std_abbr = std_a.abbr,
        .std_off_east = -std_o.secs, // east-of-UTC = -(west-of-UTC)
        .has_dst = false,
        .dst_abbr = std_a.abbr,
        .dst_off_east = -std_o.secs + 3600,
        .start_rule = .{ .month = 0, .week = 0, .dow = 0, .time = 0 },
        .end_rule = .{ .month = 0, .week = 0, .dow = 0, .time = 0 },
    };
    if (i >= s.len) return pz;
    // DST abbreviation.
    const dst_a = posix_read_abbr(s, i) orelse return pz;
    i = dst_a.i;
    pz.has_dst = true;
    pz.dst_abbr = dst_a.abbr;
    // Optional explicit DST offset; default is std + 1h east.
    if (i < s.len and s[i] != ',') {
        const dst_o = posix_read_offset(s, i) orelse return pz;
        pz.dst_off_east = -dst_o.secs;
        i = dst_o.i;
    }
    if (i >= s.len or s[i] != ',') return pz;
    i += 1;
    const start = posix_read_rule(s, i) orelse return pz;
    pz.start_rule = start.rule;
    i = start.i;
    if (i >= s.len or s[i] != ',') return pz;
    i += 1;
    const end = posix_read_rule(s, i) orelse return pz;
    pz.end_rule = end.rule;
    return pz;
}

/// Days in (year, 1-based month).  Local copy — :func:`days_in_month`
/// in :file:`tcl_clock.zig` is the authoritative version, but we
/// duplicate a tiny one here so the TZ resolver doesn't need an
/// import cycle.
fn tz_days_in_month(year: i32, month: u8) u8 {
    const normal = [_]u8{ 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31 };
    if (month == 2) {
        const leap = (@mod(year, 4) == 0 and @mod(year, 100) != 0) or @mod(year, 400) == 0;
        return if (leap) 29 else 28;
    }
    return normal[month - 1];
}

/// Days from 1970-01-01 to the start of *year*.
fn tz_days_to_year(year: i32) i64 {
    var d: i64 = 0;
    if (year >= 1970) {
        var y: i32 = 1970;
        while (y < year) : (y += 1) {
            const leap = (@mod(y, 4) == 0 and @mod(y, 100) != 0) or @mod(y, 400) == 0;
            d += if (leap) 366 else 365;
        }
    } else {
        var y: i32 = year;
        while (y < 1970) : (y += 1) {
            const leap = (@mod(y, 4) == 0 and @mod(y, 100) != 0) or @mod(y, 400) == 0;
            d -= if (leap) 366 else 365;
        }
    }
    return d;
}

/// Compute the day-of-month for the *week*-th *dow* in *year*-*month*.
/// ``week == 5`` means "last".  Result is 1..31.
fn tz_nth_weekday(year: i32, month: u8, week: u8, dow: u8) u8 {
    // 1970-01-01 is Thursday = 4.
    const days = tz_days_to_year(year);
    var pre: i64 = 0;
    var m: u8 = 1;
    while (m < month) : (m += 1) pre += @as(i64, tz_days_in_month(year, m));
    const dow_first: i64 = @mod(days + pre + 4, 7); // Sun=0
    const target_dow: i64 = @intCast(dow);
    var first_match: i64 = (target_dow - dow_first + 7);
    while (first_match > 6) first_match -= 7;
    var day = first_match + 1; // 1-based
    var added_weeks: i64 = 0;
    while (added_weeks < @as(i64, @intCast(week)) - 1) {
        if (day + 7 > tz_days_in_month(year, month)) break;
        day += 7;
        added_weeks += 1;
    }
    if (week == 5) {
        // "Last" = furthest valid occurrence.
        while (day + 7 <= tz_days_in_month(year, month)) day += 7;
    }
    return @intCast(day);
}

/// Pack a calendar date + time-of-day into Unix epoch seconds (UTC).
fn tz_pack_epoch(year: i32, month: u8, day: u8, secs_in_day: i32) i64 {
    var d: i64 = tz_days_to_year(year);
    var m: u8 = 1;
    while (m < month) : (m += 1) d += @as(i64, tz_days_in_month(year, m));
    d += @as(i64, day) - 1;
    return d * 86400 + @as(i64, secs_in_day);
}

/// Year of *secs* (UTC) — coarse-grained, used to anchor the rule
/// computation.  Only needs to be accurate enough to pick the right
/// year window; a one-day error doesn't matter because we evaluate
/// both the previous and next year's transitions.
fn tz_year_of(secs: i64) i32 {
    // Rough estimate via average-year length + iterative correction.
    var year: i32 = @intCast(@divFloor(secs, 31_557_600) + 1970);
    while (true) {
        const y_start = tz_pack_epoch(year, 1, 1, 0);
        if (secs < y_start) {
            year -= 1;
        } else {
            const next_start = tz_pack_epoch(year + 1, 1, 1, 0);
            if (secs >= next_start) {
                year += 1;
            } else break;
        }
    }
    return year;
}

/// Given a rule + a UTC offset in effect when the rule fires, return
/// the UTC instant of the transition for *year*.
fn tz_rule_to_utc(rule: PosixRule, year: i32, prev_off_east: i32) i64 {
    const day = tz_nth_weekday(year, rule.month, rule.week, rule.dow);
    const local_secs = tz_pack_epoch(year, rule.month, day, rule.time);
    return local_secs - @as(i64, prev_off_east);
}

/// Apply the POSIX TZ string to extrapolate an offset for *secs*.
/// Returns null on unparseable trailer.
fn apply_posix_rule(posix: []const u8, secs: i64) ?Offset {
    const pz = parse_posix_tz(posix) orelse return null;
    if (!pz.has_dst) {
        return .{ .utoff = pz.std_off_east, .isdst = false, .abbr = pz.std_abbr };
    }
    const year = tz_year_of(secs);
    // DST start: standard offset is in effect immediately before;
    // DST end: dst offset is in effect immediately before.  Matters
    // because the local "wall clock" reference for the transition
    // time is in the offset that's *currently* applied.
    const start_utc = tz_rule_to_utc(pz.start_rule, year, pz.std_off_east);
    const end_utc = tz_rule_to_utc(pz.end_rule, year, pz.dst_off_east);
    const in_dst = if (start_utc < end_utc)
        (secs >= start_utc and secs < end_utc)
    else
        // Southern hemisphere: DST wraps year-end.
        (secs >= start_utc or secs < end_utc);
    if (in_dst) {
        return .{ .utoff = pz.dst_off_east, .isdst = true, .abbr = pz.dst_abbr };
    }
    return .{ .utoff = pz.std_off_east, .isdst = false, .abbr = pz.std_abbr };
}

pub const ParseError = error{
    BadMagic,
    Truncated,
    BadCounts,
    NoTypes,
};

const HEADER_SIZE: usize = 44;

/// Big-endian readers — TZif counts/offsets are network byte order.
fn beU32(b: []const u8) !u32 {
    if (b.len < 4) return error.Truncated;
    return (@as(u32, b[0]) << 24) | (@as(u32, b[1]) << 16) |
        (@as(u32, b[2]) << 8) | @as(u32, b[3]);
}

fn beI32(b: []const u8) !i32 {
    return @bitCast(try beU32(b));
}

fn beI64(b: []const u8) !i64 {
    if (b.len < 8) return error.Truncated;
    var v: u64 = 0;
    inline for (0..8) |i| v = (v << 8) | @as(u64, b[i]);
    return @bitCast(v);
}

/// One header block (44 bytes).
const Header = struct {
    version: u8, // 0=v1 only, '2'/'3'/'4' = v2+ trailer follows
    isutcnt: u32,
    isstdcnt: u32,
    leapcnt: u32,
    timecnt: u32,
    typecnt: u32,
    charcnt: u32,
};

fn parse_header(b: []const u8) !Header {
    if (b.len < HEADER_SIZE) return error.Truncated;
    if (!(b[0] == 'T' and b[1] == 'Z' and b[2] == 'i' and b[3] == 'f')) {
        return error.BadMagic;
    }
    const v = b[4];
    return .{
        .version = v,
        .isutcnt = try beU32(b[20..24]),
        .isstdcnt = try beU32(b[24..28]),
        .leapcnt = try beU32(b[28..32]),
        .timecnt = try beU32(b[32..36]),
        .typecnt = try beU32(b[36..40]),
        .charcnt = try beU32(b[40..44]),
    };
}

/// Size of one body (header excluded) for the given header and
/// time-size (4 for v1, 8 for v2+).
fn body_size(h: Header, time_size: usize) usize {
    return h.timecnt * time_size +
        h.timecnt * 1 +
        h.typecnt * 6 +
        h.charcnt +
        h.leapcnt * (time_size + 4) +
        h.isstdcnt +
        h.isutcnt;
}

/// Parse one body block at ``buf[off..]`` using the given
/// ``time_size``.  Slice fields in the returned ``TimeZone`` point
/// into ``buf`` — keep the buffer alive.
fn parse_body(
    buf: []const u8,
    off: usize,
    h: Header,
    time_size: usize,
    types_storage: []LocalType,
    transitions_storage: []i64,
) !TimeZone {
    if (h.typecnt == 0) return error.NoTypes;
    if (h.typecnt > types_storage.len) return error.BadCounts;
    if (h.timecnt > transitions_storage.len) return error.BadCounts;
    const need = body_size(h, time_size);
    if (off + need > buf.len) return error.Truncated;

    var p = off;
    // Transitions
    var i: u32 = 0;
    while (i < h.timecnt) : (i += 1) {
        const slice = buf[p .. p + time_size];
        transitions_storage[i] = if (time_size == 8)
            try beI64(slice)
        else
            @as(i64, try beI32(slice));
        p += time_size;
    }
    const transitions = transitions_storage[0..h.timecnt];

    // Type indices (one byte per transition)
    const type_indices = buf[p .. p + h.timecnt];
    p += h.timecnt;

    // Types: typecnt × { i32 utoff, u8 isdst, u8 abbr_idx }
    const types_start = p;
    p += h.typecnt * 6;

    // Abbreviation chars
    const abbr_block = buf[p .. p + h.charcnt];
    p += h.charcnt;

    // Skip leap second records / std/ut indicator arrays — we don't
    // surface leap seconds (POSIX time pretends they don't exist
    // and Tcl's clock follows that convention).
    // Trailing leap-second + std/ut indicator blocks are skipped —
    // their offsets feed nothing in the lookup path.

    // Materialise ``types`` from the typecnt × 6 block.
    i = 0;
    var pre_first: i32 = -1;
    while (i < h.typecnt) : (i += 1) {
        const tp = types_start + i * 6;
        const utoff = try beI32(buf[tp .. tp + 4]);
        const isdst = buf[tp + 4] != 0;
        const aidx = buf[tp + 5];
        const abbr = nul_terminated_at(abbr_block, aidx);
        types_storage[i] = .{ .utoff = utoff, .isdst = isdst, .abbr = abbr };
        if (pre_first < 0 and !isdst) pre_first = @intCast(i);
    }
    const types = types_storage[0..h.typecnt];

    return .{
        .transitions = transitions,
        .type_indices = type_indices,
        .types = types,
        .posix_tz = "",
        .pre_first = pre_first,
    };
}

/// Slice into ``abbr_block`` starting at ``idx``, stopping at the
/// next NUL.  Returns an empty slice if ``idx`` is out of range.
fn nul_terminated_at(abbr_block: []const u8, idx: u8) []const u8 {
    if (idx >= abbr_block.len) return abbr_block[0..0];
    var end: usize = idx;
    while (end < abbr_block.len and abbr_block[end] != 0) : (end += 1) {}
    return abbr_block[idx..end];
}

/// Parse a TZif blob into a ``TimeZone``.  Storage for ``types``
/// and ``transitions`` is provided by the caller (the cache in
/// :file:`tcl_clock.zig` allocates per-zone arenas via
/// :func:`obj.alloc`).  Returns the v2+ block when present, falling
/// back to v1 only if the file lacks a v2 header.
///
/// ``posix_tz`` will point into ``buf`` if a v2+ trailer is
/// present; callers that need to keep it after ``buf`` is freed
/// should copy it themselves.
pub fn parse(
    buf: []const u8,
    types_storage: []LocalType,
    transitions_storage: []i64,
) !TimeZone {
    const h1 = try parse_header(buf);
    const v1_size = body_size(h1, 4);
    if (h1.version == '2' or h1.version == '3' or h1.version == '4') {
        const v2_off = HEADER_SIZE + v1_size;
        if (v2_off + HEADER_SIZE > buf.len) return error.Truncated;
        const h2 = try parse_header(buf[v2_off..]);
        var z = try parse_body(
            buf,
            v2_off + HEADER_SIZE,
            h2,
            8,
            types_storage,
            transitions_storage,
        );
        // Trailing POSIX TZ string lives between two newlines after
        // the v2 body.  ``after`` points at the first newline.
        const after = v2_off + HEADER_SIZE + body_size(h2, 8);
        if (after < buf.len and buf[after] == '\n') {
            const start = after + 1;
            var end = start;
            while (end < buf.len and buf[end] != '\n') : (end += 1) {}
            z.posix_tz = buf[start..end];
        }
        return z;
    }
    return try parse_body(buf, HEADER_SIZE, h1, 4, types_storage, transitions_storage);
}

// -- Built-in UTC -------------------------------------------------------------

const UTC_TYPES: [1]LocalType = .{
    .{ .utoff = 0, .isdst = false, .abbr = "UTC" },
};

const GMT_TYPES: [1]LocalType = .{
    .{ .utoff = 0, .isdst = false, .abbr = "GMT" },
};

/// A synthetic, always-available UTC zone — used as the last-ditch
/// fallback when neither host tzdata nor the bundled blob resolves.
pub fn utc() TimeZone {
    return .{
        .transitions = &[_]i64{},
        .type_indices = &[_]u8{},
        .types = UTC_TYPES[0..],
        .posix_tz = "UTC0",
        .pre_first = 0,
    };
}

/// A synthetic, always-available GMT zone — abbr is ``GMT`` instead
/// of ``UTC`` so ``clock format ... -gmt true -format %Z`` produces
/// the reference Tcl spelling.  ``-gmt 1`` and ``-timezone :GMT``
/// route here; ``-timezone UTC`` / ``Etc/UTC`` keep the ``UTC``
/// abbr by routing to :func:`utc` instead.
pub fn gmt_zone() TimeZone {
    return .{
        .transitions = &[_]i64{},
        .type_indices = &[_]u8{},
        .types = GMT_TYPES[0..],
        .posix_tz = "GMT0",
        .pre_first = 0,
    };
}

// -- Filesystem resolver + cache ---------------------------------------------
//
// ``resolve(name)`` returns a ``*const TimeZone`` for the requested
// zone, probing in order:
//
//   1. Built-in synthetic UTC (``UTC`` / ``GMT`` / ``:UTC`` /
//      ``:GMT`` / ``Etc/UTC`` / ``Etc/GMT``).  No I/O.
//   2. ``$TZ`` (only consulted when ``name`` is empty / NULL — a
//      caller-supplied name always wins so explicit ``-timezone``
//      isn't bulldozed by the host environment).
//   3. ``/usr/share/zoneinfo/<name>``,
//      ``/usr/share/lib/zoneinfo/<name>``,
//      ``/etc/zoneinfo/<name>`` — typical Linux / BSD layouts.
//   4. For empty ``name`` only: ``/etc/localtime`` (system default).
//   5. Bundled tzdata blob compiled into the wasm binary via
//      ``@embedFile("data/tzdata.bin")``.  Generated at build time
//      from ``/usr/share/zoneinfo`` by
//      ``scripts/build_tzdata_bundle.py`` — see the
//      ``Bundle binary format`` block in that script for the
//      on-disk shape.  Covers ~115 common Olson zones; sandboxed
//      hosts that can't preopen tzdata still get correct local-time
//      output for the curated list.
//   6. Last-ditch: the synthetic UTC zone, with a non-zero error
//      flag the caller can surface.
//
// Cache: an 8-slot LRU keyed on the requested name.  Each slot owns
// the raw TZif blob (allocated via ``obj.alloc``) plus the
// ``types`` / ``transitions`` storage backing the parsed
// ``TimeZone``.  Cache entries are never freed — the cache lives
// for the entire WASM-module lifetime, same as the file-descriptor
// table.  Eight slots covers the common multi-zone usage (logging
// in UTC + reporting in two regional zones) without special-casing.

// libc externs (resolved against wasi-libc — see runtime/zig/build.zig).
extern fn open(path: [*:0]const u8, flags: i32, mode: u32) i32;
extern fn read(fd: i32, buf: [*]u8, n: usize) isize;
extern fn close(fd: i32) i32;
extern fn getenv(name: [*:0]const u8) ?[*:0]const u8;

// Match the wasi-libc fcntl values used in tcl_fs.zig — these are
// derived from the underlying ``__WASI_OFLAGS_*`` bits, not POSIX
// musl bit positions.
const O_RDONLY: i32 = 0x04000000;

const MAX_ZONE_NAME: usize = 64;
const MAX_BLOB: usize = 64 * 1024; // largest tzdata file is ~10 KB
const MAX_TRANSITIONS: usize = 2048;
const MAX_TYPES: usize = 64;
const CACHE_SLOTS: usize = 8;

const Slot = struct {
    /// Zone name (NUL-padded).  ``name_len == 0`` means "free slot".
    name: [MAX_ZONE_NAME]u8 = [_]u8{0} ** MAX_ZONE_NAME,
    name_len: u8 = 0,
    /// Pointer + length to the raw TZif blob owned by this slot.
    blob_ptr: u32 = 0,
    blob_len: u32 = 0,
    /// Storage backing TimeZone.transitions / types.
    transitions: [MAX_TRANSITIONS]i64 = [_]i64{0} ** MAX_TRANSITIONS,
    types: [MAX_TYPES]LocalType = [_]LocalType{
        .{ .utoff = 0, .isdst = false, .abbr = "" },
    } ** MAX_TYPES,
    parsed: TimeZone = .{
        .transitions = &[_]i64{},
        .type_indices = &[_]u8{},
        .types = &[_]LocalType{},
        .posix_tz = "",
        .pre_first = 0,
    },
    valid: bool = false,
};

var cache: [CACHE_SLOTS]Slot = [_]Slot{.{}} ** CACHE_SLOTS;
var utc_cache: TimeZone = utc();
var gmt_cache: TimeZone = gmt_zone();

/// Last-error sentinel for the resolver — non-zero when the most
/// recent ``resolve()`` call had to fall back to UTC.  Callers
/// (e.g. the compiled-Tcl ``-timezone`` path) can surface this as
/// a Tcl-side warning if they care; today nothing reads it but the
/// hook is here to make wiring trivial later.
pub var last_error: i32 = 0;

/// Try to open + read a file into a fresh blob.  Returns null on
/// any I/O failure.  Strict size cap to keep degenerate or
/// malicious files out of the cache.
fn read_file(path: [*:0]const u8) ?[]u8 {
    const fd = open(path, O_RDONLY, 0);
    if (fd < 0) return null;
    defer _ = close(fd);

    const cap: u32 = MAX_BLOB;
    const ptr = obj.alloc(cap);
    if (ptr == 0) return null;
    const buf: [*]u8 = @ptrFromInt(ptr);

    var total: usize = 0;
    while (total < cap) {
        const n = read(fd, buf + total, cap - total);
        if (n < 0) {
            obj.free_sized(ptr, cap);
            return null;
        }
        if (n == 0) break;
        total += @intCast(n);
    }
    if (total < HEADER_SIZE) {
        obj.free_sized(ptr, cap);
        return null;
    }
    return buf[0..total];
}

/// Build a NUL-terminated path "<prefix><name>" in scratch.  Prefix
/// is a literal compiled into the wasm binary; ``name`` is the
/// caller-supplied zone name.  Rejects names that contain NUL bytes
/// or look like absolute / parent-traversal paths.
fn join_path(prefix: []const u8, name: []const u8, scratch: *[256]u8) ?[*:0]const u8 {
    if (name.len == 0) return null;
    if (prefix.len + name.len + 1 > scratch.len) return null;
    if (name[0] == '/' or name[0] == '.') {
        // No leading slash or dot — only safe relative zone names
        // (``America/New_York``, ``Etc/UTC``) are allowed through.
        // Absolute paths and ``..`` walks would let the script
        // exfiltrate arbitrary host files.
        return null;
    }
    var i: usize = 0;
    while (i < name.len) : (i += 1) {
        const b = name[i];
        if (b == 0 or b == '\\') return null;
        if (b == '.' and i + 1 < name.len and name[i + 1] == '.') return null;
    }
    @memcpy(scratch[0..prefix.len], prefix);
    @memcpy(scratch[prefix.len .. prefix.len + name.len], name);
    scratch[prefix.len + name.len] = 0;
    return @ptrCast(&scratch[0]);
}

/// Strip a leading ':' from a zone name.  Tcl accepts both
/// ``:America/New_York`` (POSIX-style "look up Olson") and the
/// bare ``America/New_York`` form; treat them identically.
fn strip_colon(name: []const u8) []const u8 {
    if (name.len > 0 and name[0] == ':') return name[1..];
    return name;
}

/// True if ``name`` (after colon-stripping) is one of the synthetic
/// UTC aliases.
fn is_utc_alias(name: []const u8) bool {
    const n = strip_colon(name);
    return std.mem.eql(u8, n, "UTC") or
        std.mem.eql(u8, n, "Etc/UTC") or
        std.mem.eql(u8, n, "Universal") or
        std.mem.eql(u8, n, "Zulu");
}

/// True if ``name`` (after colon-stripping) is one of the synthetic
/// GMT aliases — same offset as UTC but a different ``%Z`` abbr.
fn is_gmt_alias(name: []const u8) bool {
    const n = strip_colon(name);
    return std.mem.eql(u8, n, "GMT") or
        std.mem.eql(u8, n, "Etc/GMT");
}

/// Find a cache slot matching ``name``.  Returns null on miss.
fn cache_find(name: []const u8) ?*Slot {
    for (&cache) |*s| {
        if (s.valid and s.name_len == name.len and
            std.mem.eql(u8, s.name[0..s.name_len], name))
            return s;
    }
    return null;
}

/// Pick a free slot, or evict slot 0 if all are taken.  Simple
/// FIFO eviction is fine — zones are looked up rarely and the
/// usual workload has ≤ 3 distinct zones.
fn cache_pick_slot() *Slot {
    for (&cache) |*s| {
        if (!s.valid) return s;
    }
    // All taken — evict slot 0.  Free its blob and reset.
    const s = &cache[0];
    if (s.blob_ptr != 0) obj.free_sized(s.blob_ptr, s.blob_len);
    s.* = .{};
    return s;
}

/// Parse ``blob`` into ``slot.parsed``.  On failure, leaves the
/// slot invalidated and returns false.
fn install_blob(slot: *Slot, blob: []u8, name: []const u8) bool {
    const z = parse(blob, slot.types[0..], slot.transitions[0..]) catch {
        return false;
    };
    slot.parsed = z;
    slot.blob_ptr = @intCast(@intFromPtr(blob.ptr));
    slot.blob_len = @intCast(blob.len);
    const copy_len = @min(name.len, MAX_ZONE_NAME);
    @memcpy(slot.name[0..copy_len], name[0..copy_len]);
    slot.name_len = @intCast(copy_len);
    slot.valid = true;
    return true;
}

// -- Bundled tzdata blob ------------------------------------------------------
//
// ``data/tzdata.bin`` is generated by ``scripts/build_tzdata_bundle.py``
// from the host's ``/usr/share/zoneinfo``.  See the script's
// docstring for the on-disk format; the lookup here reads the same
// header layout (4-byte ``TZBL`` magic, version u8, n_entries u32 LE,
// then index records, then concatenated TZif payloads).
//
// The bundle is embedded as a comptime byte slice — no I/O, no
// allocation; lookup is a binary search over the index records.

// ``@embedFile`` resolves relative to this source file, hence the
// ``../`` to climb out of ``io/`` and into ``data/``.
const TZDATA_BUNDLE: []const u8 = @embedFile("../data/tzdata.bin");
const BUNDLE_MAGIC: [4]u8 = .{ 'T', 'Z', 'B', 'L' };
const BUNDLE_HEADER_SIZE: usize = 12;

/// Read a little-endian u32 from ``b[off..off+4]``.  The bundle is
/// LE because the wasm32 target is LE; the format spec pins this
/// so a future big-endian wasm port still lays out the bundle the
/// same way.
fn leU32(b: []const u8, off: usize) u32 {
    return @as(u32, b[off]) |
        (@as(u32, b[off + 1]) << 8) |
        (@as(u32, b[off + 2]) << 16) |
        (@as(u32, b[off + 3]) << 24);
}

/// True iff the embedded bundle is well-formed (magic + version
/// match).  Returns false on any deviation so the resolver falls
/// through to UTC instead of trapping.
fn bundle_ok() bool {
    if (TZDATA_BUNDLE.len < BUNDLE_HEADER_SIZE) return false;
    if (!std.mem.eql(u8, TZDATA_BUNDLE[0..4], BUNDLE_MAGIC[0..])) return false;
    if (TZDATA_BUNDLE[4] != 1) return false;
    return true;
}

const BundleEntry = struct {
    name: []const u8,
    blob_off: u32,
    blob_len: u32,
};

/// Read one index record at ``off``, returning the entry plus the
/// next-record offset.  ``null`` if the record runs off the end.
fn bundle_read_entry(off: usize) ?struct { e: BundleEntry, next: usize } {
    if (off + 1 > TZDATA_BUNDLE.len) return null;
    const name_len: usize = TZDATA_BUNDLE[off];
    const name_start = off + 1;
    const name_end = name_start + name_len;
    if (name_end + 8 > TZDATA_BUNDLE.len) return null;
    const blob_off = leU32(TZDATA_BUNDLE, name_end);
    const blob_len = leU32(TZDATA_BUNDLE, name_end + 4);
    const next = name_end + 8;
    return .{
        .e = .{
            .name = TZDATA_BUNDLE[name_start..name_end],
            .blob_off = blob_off,
            .blob_len = blob_len,
        },
        .next = next,
    };
}

/// Linear-scan the bundle for ``name``.  The bundle script sorts
/// the index by name so a binary search is technically possible,
/// but variable-length records make it awkward; the scan runs over
/// at most ~115 short strings and is O(zone-count) which is fine
/// for one-shot resolves cached for the rest of the run.
fn bundle_find(name: []const u8) ?[]const u8 {
    if (!bundle_ok()) return null;
    const n = leU32(TZDATA_BUNDLE, 8);
    var off: usize = BUNDLE_HEADER_SIZE;
    var i: u32 = 0;
    while (i < n) : (i += 1) {
        const r = bundle_read_entry(off) orelse return null;
        if (std.mem.eql(u8, r.e.name, name)) {
            const start = r.e.blob_off;
            const end = start + r.e.blob_len;
            if (end > TZDATA_BUNDLE.len) return null;
            return TZDATA_BUNDLE[start..end];
        }
        off = r.next;
    }
    return null;
}

/// Copy a bundled (read-only) TZif blob into a freshly-allocated
/// buffer the cache slot can own.  Returns null on OOM.  We can't
/// hand the slot a slice into the embedded blob because the cache
/// slot's ``free_sized`` path would try to free read-only data on
/// eviction.
fn bundle_clone_blob(src: []const u8) ?[]u8 {
    const ptr = obj.alloc(@intCast(src.len));
    if (ptr == 0) return null;
    const dst: [*]u8 = @ptrFromInt(ptr);
    @memcpy(dst[0..src.len], src);
    return dst[0..src.len];
}

/// Try the bundled blob for ``name``.  Same return contract as
/// :func:`try_load_host`: a freshly-allocated blob the cache slot
/// can own, or null on miss / OOM.
fn try_load_bundle(name: []const u8) ?[]u8 {
    const slice = bundle_find(name) orelse return null;
    return bundle_clone_blob(slice);
}

/// Search a list of host directories for ``<dir>/<name>``.  First
/// hit wins.
fn try_load_host(name: []const u8) ?[]u8 {
    var scratch: [256]u8 = undefined;
    const dirs = [_][]const u8{
        "/usr/share/zoneinfo/",
        "/usr/share/lib/zoneinfo/",
        "/etc/zoneinfo/",
    };
    for (dirs) |d| {
        const p = join_path(d, name, &scratch) orelse continue;
        if (read_file(p)) |blob| return blob;
    }
    return null;
}

/// Public resolver entry-point.  ``name`` is the caller's zone
/// request — empty / null falls back to ``$TZ`` or ``/etc/localtime``.
/// The returned ``TimeZone`` is owned by the cache; do not free.
pub fn resolve(name: []const u8) *const TimeZone {
    last_error = 0;
    const stripped = strip_colon(name);

    // 1. Synthetic UTC — no I/O, no cache footprint.
    if (stripped.len == 0 or is_utc_alias(name)) {
        return &utc_cache;
    }
    // 1b. Synthetic GMT — same offset as UTC but a distinct ``%Z``
    // abbreviation.  Reference Tcl's ``-gmt true`` resolves to
    // ``GMT`` (not ``UTC``); we route ``-gmt 1`` through this name
    // from ``cmds/stubs.zig``.
    if (is_gmt_alias(name)) {
        return &gmt_cache;
    }

    // 2. Cache hit?
    if (cache_find(stripped)) |s| return &s.parsed;

    // 3. Host filesystem lookup.  Real-time tzdata wins over the
    //    embedded bundle so a host that gets timezone updates
    //    (DST-rule changes, new historical data) reflects them
    //    without re-shipping the wasm binary.
    if (try_load_host(stripped)) |blob| {
        const slot = cache_pick_slot();
        if (install_blob(slot, blob, stripped)) return &slot.parsed;
        obj.free_sized(@intCast(@intFromPtr(blob.ptr)), @intCast(blob.len));
    }

    // 4. Bundled tzdata blob.  Comptime-embedded ~133 KB index of
    //    ~115 common zones; covers sandboxed environments where
    //    the embedder didn't preopen any host tzdata.
    if (try_load_bundle(stripped)) |blob| {
        const slot = cache_pick_slot();
        if (install_blob(slot, blob, stripped)) return &slot.parsed;
        obj.free_sized(@intCast(@intFromPtr(blob.ptr)), @intCast(blob.len));
    }

    // 5. Last-ditch: UTC and a non-zero error sentinel.
    last_error = 1;
    return &utc_cache;
}

/// Resolve with no caller-supplied name.  Honours ``$TZ`` when set,
/// falls back to ``/etc/localtime``.  Returns the synthetic UTC
/// zone when neither resolves.
pub fn resolve_default() *const TimeZone {
    last_error = 0;
    if (getenv("TZ")) |tz_c| {
        const tz_slice = std.mem.sliceTo(tz_c, 0);
        if (tz_slice.len > 0) return resolve(tz_slice);
    }
    // ``/etc/localtime`` is the conventional system-default tzdata
    // file.  Read directly (no name-resolution dance).
    if (read_file("/etc/localtime")) |blob| {
        const slot = cache_pick_slot();
        if (install_blob(slot, blob, "localtime")) return &slot.parsed;
        obj.free_sized(@intCast(@intFromPtr(blob.ptr)), @intCast(blob.len));
    }
    last_error = 1;
    return &utc_cache;
}

// -- Tests --------------------------------------------------------------------

test "utc zone returns 0 offset for any time" {
    const z = utc();
    const o1 = z.offset_at(0);
    try std.testing.expectEqual(@as(i32, 0), o1.utoff);
    try std.testing.expectEqual(false, o1.isdst);
    try std.testing.expectEqualStrings("UTC", o1.abbr);

    const o2 = z.offset_at(2_000_000_000);
    try std.testing.expectEqual(@as(i32, 0), o2.utoff);
}

test "parse_header rejects bad magic" {
    const bad = [_]u8{ 'X', 'X', 'X', 'X' } ++ ([_]u8{0} ** 40);
    try std.testing.expectError(error.BadMagic, parse_header(bad[0..]));
}

test "parse_header rejects truncated input" {
    const tiny = [_]u8{ 'T', 'Z', 'i', 'f' };
    try std.testing.expectError(error.Truncated, parse_header(tiny[0..]));
}

test "be readers" {
    const buf = [_]u8{ 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08 };
    try std.testing.expectEqual(@as(u32, 0x01020304), try beU32(buf[0..4]));
    try std.testing.expectEqual(@as(i32, 0x01020304), try beI32(buf[0..4]));
    try std.testing.expectEqual(@as(i64, 0x0102030405060708), try beI64(buf[0..8]));
}

test "synthetic single-type zone" {
    const types = [_]LocalType{
        .{ .utoff = 3600, .isdst = false, .abbr = "TST" },
    };
    const z = TimeZone{
        .transitions = &[_]i64{},
        .type_indices = &[_]u8{},
        .types = types[0..],
        .posix_tz = "",
        .pre_first = 0,
    };
    const o = z.offset_at(123_456_789);
    try std.testing.expectEqual(@as(i32, 3600), o.utoff);
    try std.testing.expectEqualStrings("TST", o.abbr);
}

test "transition lookup picks correct type" {
    const types = [_]LocalType{
        .{ .utoff = -18000, .isdst = false, .abbr = "EST" },
        .{ .utoff = -14400, .isdst = true, .abbr = "EDT" },
    };
    // Two transitions: enter EDT at t=100, return to EST at t=200.
    const trans = [_]i64{ 100, 200 };
    const idx = [_]u8{ 1, 0 };
    const z = TimeZone{
        .transitions = trans[0..],
        .type_indices = idx[0..],
        .types = types[0..],
        .posix_tz = "",
        .pre_first = 0,
    };
    // Before any transition: pre_first (EST).
    try std.testing.expectEqual(@as(i32, -18000), z.offset_at(50).utoff);
    // Right at first transition: EDT.
    try std.testing.expectEqual(@as(i32, -14400), z.offset_at(100).utoff);
    try std.testing.expectEqual(@as(i32, -14400), z.offset_at(150).utoff);
    // At second transition: EST.
    try std.testing.expectEqual(@as(i32, -18000), z.offset_at(200).utoff);
    try std.testing.expectEqual(@as(i32, -18000), z.offset_at(99_999).utoff);
}
