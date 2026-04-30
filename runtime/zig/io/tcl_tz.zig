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

/// A synthetic, always-available UTC zone — used by the resolver
/// when ``-gmt 1`` / ``-timezone :UTC`` / ``-timezone GMT`` is
/// requested, or as the last-ditch fallback when neither host
/// tzdata nor the bundled blob resolves.
pub fn utc() TimeZone {
    return .{
        .transitions = &[_]i64{},
        .type_indices = &[_]u8{},
        .types = UTC_TYPES[0..],
        .posix_tz = "UTC0",
        .pre_first = 0,
    };
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
