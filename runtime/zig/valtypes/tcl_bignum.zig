// Bignum (arbitrary-precision integer) support for Tcl ``expr``.
//
// **Stage 1 — i128 promotion (this module).**  C Tcl 9.0 uses
// libtommath for true arbitrary-precision integers; we promote
// only as far as i128 (~38 decimal digits) before clamping.  i128
// covers the most common overflow cases that appear in the
// upstream test suite:
//
//   * ``1 << 63``, ``-(1 << 63)``, ``1 << 127`` — bit-shift
//     overflows produced by the ``<<`` and ``**`` operators.
//   * Addition / multiplication that crosses the i64 boundary
//     once (e.g. ``9223372036854775807 + 1``).
//   * Literal parsing of decimal numbers up to ``2^127 - 1``.
//
// Values that exceed i128 (e.g. expr-old-36.16's 152-bit hex
// literal) still saturate to the i128 boundary — Stage 2 will
// land arbitrary precision via libtommath.  The boundary is
// deliberate: a single-word i128 fits in 16 bytes off-heap and
// keeps the TclObj header at its current 32 bytes, so we don't
// disturb the size-class allocator.
//
// Storage model
// -------------
//
// A ``TYPE_BIGNUM`` TclObj keeps the i128 value in a 16-byte
// off-heap buffer pointed to by ``OBJ_DICT_EXT`` (offset 28).
// String-rep caching reuses the same ``OBJ_STR_PTR`` /
// ``OBJ_STR_LEN`` / ``OBJ_STR_CAP`` slots that ``TYPE_INT`` and
// ``TYPE_FLOAT`` use, so rendering is amortised across reads.
//
// Public API
// ----------
//
//   * ``parse_i128(ptr, len) -> ?i128`` — parse a decimal / hex /
//     octal / binary integer literal as i128.  Returns ``null``
//     when the input is not a valid integer or doesn't fit in
//     i128.
//   * ``format_i128(value, buf) -> usize`` — write the decimal
//     representation of ``value`` into ``buf`` and return the
//     byte count.  ``buf`` must be at least 41 bytes long
//     (i128::MIN is ``-170141183460469231731687303715884105728``,
//     40 chars + sign).
//   * ``add_overflow(a, b) -> ?i128`` / ``sub_overflow`` /
//     ``mul_overflow`` — i128 arithmetic with overflow detection.
//     Returns ``null`` on overflow.
//   * ``shl_overflow(a, b) -> ?i128`` — left-shift with overflow
//     detection.  Counts ``>= 128`` overflow.

const std = @import("std");

/// Maximum byte length of an i128 decimal rendering, including
/// the optional leading minus sign.  ``-2^127`` renders to
/// ``-170141183460469231731687303715884105728`` (40 chars +
/// sign).
pub const I128_DECIMAL_MAX: usize = 41;

/// True iff ``c`` is ASCII whitespace recognised by Tcl integer
/// parsing.  Mirrors :func:`tcl_obj.is_space` so the bignum
/// parser accepts the same surrounding whitespace as the i64
/// parser.
fn is_space(c: u8) bool {
    return c == ' ' or c == '\t' or c == '\n' or c == '\r' or c == 0x0c or c == 0x0b;
}

fn is_digit(c: u8) bool {
    return c >= '0' and c <= '9';
}

fn is_hex_digit(c: u8) bool {
    return (c >= '0' and c <= '9') or (c >= 'a' and c <= 'f') or (c >= 'A' and c <= 'F');
}

fn hex_value(c: u8) u8 {
    if (c >= '0' and c <= '9') return c - '0';
    if (c >= 'a' and c <= 'f') return c - 'a' + 10;
    return c - 'A' + 10;
}

/// Parse a Tcl integer literal as an i128.  Accepts:
///
///   * Decimal:    ``[+-]?[0-9]+``
///   * Hex:        ``[+-]?0[xX][0-9a-fA-F]+``
///   * Octal:      ``[+-]?0[oO][0-7]+``
///   * Binary:     ``[+-]?0[bB][01]+``
///
/// Surrounding whitespace is permitted (matches
/// :func:`tcl_obj.try_parse_int`'s discipline).  Returns ``null``
/// on syntax error or magnitude overflow.
pub fn parse_i128(ptr: u32, len: u32) ?i128 {
    if (len == 0) return null;
    const src: [*]const u8 = @ptrFromInt(ptr);
    var i: u32 = 0;
    while (i < len and is_space(src[i])) i += 1;
    if (i >= len) return null;
    var negative = false;
    if (src[i] == '-') {
        negative = true;
        i += 1;
    } else if (src[i] == '+') {
        i += 1;
    }
    if (i >= len) return null;

    // Detect base prefix.
    var base: u32 = 10;
    if (src[i] == '0' and i + 1 < len) {
        const c = src[i + 1];
        if (c == 'x' or c == 'X') {
            base = 16;
            i += 2;
        } else if (c == 'o' or c == 'O') {
            base = 8;
            i += 2;
        } else if (c == 'b' or c == 'B') {
            base = 2;
            i += 2;
        }
    }
    if (i >= len) return null;

    // Reject if the very first body char isn't valid for the
    // chosen base.  ``parse_with_base_u128`` would otherwise
    // accept a stray sign character or trailing whitespace as
    // "no digits" silently — keep the discipline tight.
    const first = src[i];
    const valid_first = switch (base) {
        2 => first == '0' or first == '1',
        8 => first >= '0' and first <= '7',
        10 => is_digit(first),
        16 => is_hex_digit(first),
        else => unreachable,
    };
    if (!valid_first) return null;

    var mag: u128 = 0;
    while (i < len) {
        const c = src[i];
        if (is_space(c)) break;
        const digit: u8 = switch (base) {
            2 => if (c == '0' or c == '1') c - '0' else return null,
            8 => if (c >= '0' and c <= '7') c - '0' else return null,
            10 => if (is_digit(c)) c - '0' else return null,
            16 => if (is_hex_digit(c)) hex_value(c) else return null,
            else => unreachable,
        };
        const m = @mulWithOverflow(mag, @as(u128, base));
        if (m[1] != 0) return null;
        const a = @addWithOverflow(m[0], @as(u128, digit));
        if (a[1] != 0) return null;
        mag = a[0];
        i += 1;
    }
    while (i < len and is_space(src[i])) i += 1;
    if (i != len) return null;

    // Convert magnitude to signed i128.  ``i128::MIN`` has
    // magnitude ``2^127`` which is one above ``i128::MAX``; we
    // handle that boundary explicitly so it round-trips.
    const I128_MIN_ABS: u128 = @as(u128, 1) << 127;
    const I128_MAX_U: u128 = (@as(u128, 1) << 127) - 1;
    if (negative) {
        if (mag > I128_MIN_ABS) return null;
        if (mag == I128_MIN_ABS) {
            // ``-(2^127)`` = ``i128::MIN``.  Build via two's
            // complement bit pattern to avoid the ``-i128::MIN``
            // negate-overflow.
            return @as(i128, @bitCast(@as(u128, 1) << 127));
        }
        return -@as(i128, @intCast(mag));
    }
    if (mag > I128_MAX_U) return null;
    return @as(i128, @intCast(mag));
}

/// Render an i128 as decimal into ``buf``.  Returns the number
/// of bytes written; the caller must reserve at least
/// :const:`I128_DECIMAL_MAX` bytes.  The returned slice starts
/// at ``buf[0]``.
pub fn format_i128(value: i128, buf: []u8) usize {
    std.debug.assert(buf.len >= I128_DECIMAL_MAX);
    var negative = false;
    var mag: u128 = blk: {
        if (value < 0) {
            negative = true;
            // ``-i128::MIN`` would overflow.  Bitcast trick: in
            // unsigned space, ``0 -% v`` yields the magnitude
            // for any negative i128, including i128::MIN.
            const u: u128 = @bitCast(value);
            break :blk @as(u128, 0) -% u;
        }
        break :blk @as(u128, @intCast(value));
    };
    // Render least-significant-digit first into a scratch
    // sub-buffer, then reverse into ``buf``.
    var scratch: [I128_DECIMAL_MAX]u8 = undefined;
    var n: usize = 0;
    if (mag == 0) {
        scratch[n] = '0';
        n += 1;
    } else {
        while (mag > 0) {
            scratch[n] = @as(u8, @intCast(mag % 10)) + '0';
            mag /= 10;
            n += 1;
        }
    }
    var off: usize = 0;
    if (negative) {
        buf[off] = '-';
        off += 1;
    }
    var k: usize = n;
    while (k > 0) {
        k -= 1;
        buf[off] = scratch[k];
        off += 1;
    }
    return off;
}

/// Add two i128 values, returning ``null`` on overflow.
pub fn add_overflow(a: i128, b: i128) ?i128 {
    const r = @addWithOverflow(a, b);
    if (r[1] != 0) return null;
    return r[0];
}

/// Subtract two i128 values, returning ``null`` on overflow.
pub fn sub_overflow(a: i128, b: i128) ?i128 {
    const r = @subWithOverflow(a, b);
    if (r[1] != 0) return null;
    return r[0];
}

/// Multiply two i128 values, returning ``null`` on overflow.
pub fn mul_overflow(a: i128, b: i128) ?i128 {
    const r = @mulWithOverflow(a, b);
    if (r[1] != 0) return null;
    return r[0];
}

/// Left-shift ``a`` by ``count`` bits.  Returns ``null`` if the
/// shift would overflow the i128 range (``count >= 128`` for any
/// non-zero ``a``, or the shifted value's magnitude exceeds
/// ``2^127 - 1`` for non-negative ``a`` / ``2^127`` for
/// negative).  Negative counts are caller-rejected.
pub fn shl_overflow(a: i128, count: u32) ?i128 {
    if (a == 0) return 0;
    if (count == 0) return a;
    if (count >= 128) return null;
    const shift: u7 = @intCast(count);
    // Determine the signed-magnitude of ``a`` and shift in
    // unsigned space; reattach the sign at the end.  This makes
    // the overflow check a simple "did the magnitude land in
    // the legal range" comparison.
    var negative = false;
    var mag: u128 = blk: {
        if (a < 0) {
            negative = true;
            const u: u128 = @bitCast(a);
            break :blk @as(u128, 0) -% u;
        }
        break :blk @as(u128, @intCast(a));
    };
    // Overflow if the top bit of mag is already too high for
    // the requested shift amount.
    const leading = @clz(mag);
    const limit: u32 = if (negative) 128 else 127;
    if (count > leading + (128 - limit)) return null;
    mag <<= shift;
    if (negative) {
        if (mag > (@as(u128, 1) << 127)) return null;
        if (mag == (@as(u128, 1) << 127)) {
            return @as(i128, @bitCast(@as(u128, 1) << 127));
        }
        return -@as(i128, @intCast(mag));
    }
    if (mag > ((@as(u128, 1) << 127) - 1)) return null;
    return @as(i128, @intCast(mag));
}

// ---- tests --------------------------------------------------------

const testing = std.testing;

fn slice_to_addr_len(s: []const u8) struct { ptr: u32, len: u32 } {
    return .{
        .ptr = @intFromPtr(s.ptr),
        .len = @intCast(s.len),
    };
}

test "parse_i128 — small decimals" {
    const a = slice_to_addr_len("42");
    try testing.expectEqual(@as(?i128, 42), parse_i128(a.ptr, a.len));
    const b = slice_to_addr_len("-7");
    try testing.expectEqual(@as(?i128, -7), parse_i128(b.ptr, b.len));
}

test "parse_i128 — i64 boundary" {
    // i64 max + 1 fits in i128, doesn't fit in i64.
    const s = slice_to_addr_len("9223372036854775808");
    try testing.expectEqual(@as(?i128, 9223372036854775808), parse_i128(s.ptr, s.len));
}

test "parse_i128 — i64 min - 1" {
    const s = slice_to_addr_len("-9223372036854775809");
    try testing.expectEqual(@as(?i128, -9223372036854775809), parse_i128(s.ptr, s.len));
}

test "parse_i128 — i128 max boundary" {
    const max_str = "170141183460469231731687303715884105727";
    const s = slice_to_addr_len(max_str);
    const r = parse_i128(s.ptr, s.len);
    try testing.expectEqual(@as(i128, std.math.maxInt(i128)), r.?);
}

test "parse_i128 — i128 min boundary (handles abs == 2^127)" {
    const min_str = "-170141183460469231731687303715884105728";
    const s = slice_to_addr_len(min_str);
    const r = parse_i128(s.ptr, s.len);
    try testing.expectEqual(@as(i128, std.math.minInt(i128)), r.?);
}

test "parse_i128 — i128 max + 1 overflows" {
    const s = slice_to_addr_len("170141183460469231731687303715884105728");
    try testing.expectEqual(@as(?i128, null), parse_i128(s.ptr, s.len));
}

test "parse_i128 — hex literal" {
    const s = slice_to_addr_len("0xFF");
    try testing.expectEqual(@as(?i128, 255), parse_i128(s.ptr, s.len));
    const s2 = slice_to_addr_len("0x8000000000000000");
    try testing.expectEqual(@as(?i128, 0x8000000000000000), parse_i128(s2.ptr, s2.len));
}

test "parse_i128 — octal literal" {
    const s = slice_to_addr_len("0o777");
    try testing.expectEqual(@as(?i128, 511), parse_i128(s.ptr, s.len));
}

test "parse_i128 — binary literal" {
    const s = slice_to_addr_len("0b1010");
    try testing.expectEqual(@as(?i128, 10), parse_i128(s.ptr, s.len));
}

test "parse_i128 — leading/trailing whitespace" {
    const s = slice_to_addr_len("  42  ");
    try testing.expectEqual(@as(?i128, 42), parse_i128(s.ptr, s.len));
}

test "parse_i128 — empty input" {
    try testing.expectEqual(@as(?i128, null), parse_i128(0, 0));
}

test "parse_i128 — invalid base prefix digit" {
    const s = slice_to_addr_len("0o89");
    try testing.expectEqual(@as(?i128, null), parse_i128(s.ptr, s.len));
}

test "parse_i128 — bare sign rejected" {
    const s = slice_to_addr_len("-");
    try testing.expectEqual(@as(?i128, null), parse_i128(s.ptr, s.len));
}

test "format_i128 — small values" {
    var buf: [I128_DECIMAL_MAX]u8 = undefined;
    {
        const n = format_i128(0, &buf);
        try testing.expectEqualStrings("0", buf[0..n]);
    }
    {
        const n = format_i128(42, &buf);
        try testing.expectEqualStrings("42", buf[0..n]);
    }
    {
        const n = format_i128(-7, &buf);
        try testing.expectEqualStrings("-7", buf[0..n]);
    }
}

test "format_i128 — i128 max" {
    var buf: [I128_DECIMAL_MAX]u8 = undefined;
    const n = format_i128(std.math.maxInt(i128), &buf);
    try testing.expectEqualStrings("170141183460469231731687303715884105727", buf[0..n]);
}

test "format_i128 — i128 min" {
    var buf: [I128_DECIMAL_MAX]u8 = undefined;
    const n = format_i128(std.math.minInt(i128), &buf);
    try testing.expectEqualStrings("-170141183460469231731687303715884105728", buf[0..n]);
}

test "format_i128 — i64 max + 1" {
    var buf: [I128_DECIMAL_MAX]u8 = undefined;
    const n = format_i128(@as(i128, std.math.maxInt(i64)) + 1, &buf);
    try testing.expectEqualStrings("9223372036854775808", buf[0..n]);
}

test "format_i128 — round-trips" {
    var buf: [I128_DECIMAL_MAX]u8 = undefined;
    const cases = [_]i128{
        0,
        1,
        -1,
        std.math.maxInt(i64),
        @as(i128, std.math.maxInt(i64)) + 1,
        std.math.minInt(i64),
        @as(i128, std.math.minInt(i64)) - 1,
        std.math.maxInt(i128),
        std.math.minInt(i128),
    };
    for (cases) |v| {
        const n = format_i128(v, &buf);
        const sl = slice_to_addr_len(buf[0..n]);
        const parsed = parse_i128(sl.ptr, sl.len);
        try testing.expectEqual(@as(?i128, v), parsed);
    }
}

test "add_overflow — small sums fit" {
    try testing.expectEqual(@as(?i128, 5), add_overflow(2, 3));
    try testing.expectEqual(@as(?i128, 0), add_overflow(7, -7));
}

test "add_overflow — i128 max + 1 overflows" {
    try testing.expectEqual(@as(?i128, null), add_overflow(std.math.maxInt(i128), 1));
}

test "sub_overflow — i128 min - 1 overflows" {
    try testing.expectEqual(@as(?i128, null), sub_overflow(std.math.minInt(i128), 1));
}

test "mul_overflow — small products fit" {
    try testing.expectEqual(@as(?i128, 12), mul_overflow(3, 4));
    try testing.expectEqual(@as(?i128, -12), mul_overflow(-3, 4));
}

test "mul_overflow — promotes past i64" {
    const big = @as(i128, std.math.maxInt(i64)) + 1;
    try testing.expectEqual(@as(?i128, big * 2), mul_overflow(big, 2));
}

test "mul_overflow — i128 saturation" {
    try testing.expectEqual(@as(?i128, null), mul_overflow(std.math.maxInt(i128), 2));
}

test "shl_overflow — 1 << 63 fits" {
    try testing.expectEqual(@as(?i128, @as(i128, 1) << 63), shl_overflow(1, 63));
}

test "shl_overflow — 1 << 126 fits" {
    try testing.expectEqual(@as(?i128, @as(i128, 1) << 126), shl_overflow(1, 126));
}

test "shl_overflow — 1 << 127 overflows positive range" {
    try testing.expectEqual(@as(?i128, null), shl_overflow(1, 127));
}

test "shl_overflow — count >= 128 always overflows non-zero" {
    try testing.expectEqual(@as(?i128, null), shl_overflow(1, 128));
    try testing.expectEqual(@as(?i128, 0), shl_overflow(0, 200));
}
