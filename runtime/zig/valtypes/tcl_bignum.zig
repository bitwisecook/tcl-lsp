// Bignum (arbitrary-precision integer) support for Tcl ``expr``.
//
// **Stage 1 — i128 helpers (lower half of this file).**  Bounded-
// width integer parsing / formatting / overflow-detected arithmetic
// for the fast common case where an overflow promotion stays within
// 128 bits.  These helpers are kept after Stage 2 because the i64-
// overflow path can compute the result entirely in i128 (i64 + i64
// always fits in i65 << i128) and avoid a heap allocation when the
// promoted value still fits the i128 range.
//
// **Stage 2 — *BigInt (upper half).**  True arbitrary-precision
// integers backed by Zig stdlib's ``std.math.big.int.Managed``.
// Used when an operand is already wider than i128 (e.g. the upstream
// ``expr-old-36.{11,14,16}`` literals that exceed 80 / 90 / 152
// bits) or when an operation lifts a Stage 1 result above the i128
// boundary (e.g. ``(1 << 130) * (1 << 130)``).
//
// Storage convention for ``TYPE_BIGNUM`` (see ``tcl_obj.zig``):
//
//   * The OBJ_DICT_EXT slot holds a ``*BigInt`` (a heap-allocated
//     Managed struct that itself owns its limb buffer through
//     ``std.heap.c_allocator`` → wasi-libc malloc / free).  This
//     keeps the bignum heap consistent with the rest of the
//     runtime's libc-routed allocator.
//   * On obj release, ``release_now`` calls ``bignum.destroy(m)``
//     which deinits the Managed (frees the limb buffer) and
//     destroys the struct.
//
// API surface (Stage 1 — i128 fast path):
//
//   * ``parse_i128(ptr, len) -> ?i128``
//   * ``format_i128(value, buf) -> usize``
//   * ``add_overflow``, ``sub_overflow``, ``mul_overflow``,
//     ``shl_overflow`` — overflow-detected i128 ops returning ``?i128``
//
// API surface (Stage 2 — Managed):
//
//   * ``alloc_from_int(value: i128) -> ?*BigInt`` —
//     allocate + ``initSet``
//   * ``alloc_from_string(ptr, len) -> ?*BigInt`` —
//     parse decimal / hex / octal / binary into a fresh ``Managed``
//   * ``destroy(m: *BigInt)`` — ``deinit`` + ``destroy``
//   * ``alloc_format(m: *const BigInt) -> ?[]u8`` —
//     decimal toString into a heap-allocated buffer
//   * ``alloc_add / sub / mul / div_trunc / div_floor / shl / neg``
//   * ``mod_floor`` — Tcl's "result has divisor's sign" semantic
//   * ``fits_i64 / fits_i128 / to_i64 / to_i128 / to_f64``

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

// ====================================================================
// Stage 2 — *BigInt (std.math.big.int.Managed) helpers
// ====================================================================
//
// Allocation discipline: every ``alloc_*`` function returns a freshly-
// initialised heap-allocated ``Managed`` whose limb buffer is owned
// through ``std.heap.c_allocator`` (wasi-libc malloc).  The TclObj
// layer takes ownership and is responsible for calling :func:`destroy`
// when the obj is released.  Errors (OOM, parse failure) return
// ``null`` so callers can convert to a Tcl-level error rather than
// trapping the WASM module.
//
// Why a heap-allocated ``Managed`` (not by-value):
//   * The TclObj header is 32 bytes; the ``Managed`` struct alone is
//     ~20 bytes on wasm32 plus a separately-allocated limb buffer.
//   * Storing ``Managed`` by value inside the TclObj would force
//     callers to update the obj header on every operation that
//     reallocates the limb buffer, breaking the TYPE_INT shimmer
//     contract that lets two readers race-read the same obj.
//   * A pointer keeps the obj header read-only after construction;
//     limb-buffer growth happens behind the pointer.

pub const BigInt = std.math.big.int.Managed;

/// Allocator used for every heap allocation made by this module.
/// ``std.heap.c_allocator`` routes through wasi-libc malloc / free,
/// which is the same allocator the rest of the runtime uses for
/// TclObj headers and string buffers — keeps the heap coherent.
pub const allocator = std.heap.c_allocator;

/// Allocate a new BigInt and initialise it from an i128 value.
/// Returns ``null`` on OOM.
pub fn alloc_from_int(value: i128) ?*BigInt {
    const m = allocator.create(BigInt) catch return null;
    m.* = BigInt.initSet(allocator, value) catch {
        allocator.destroy(m);
        return null;
    };
    return m;
}

/// Allocate a new BigInt and initialise it as zero.  Returns
/// ``null`` on OOM.
pub fn alloc_zero() ?*BigInt {
    const m = allocator.create(BigInt) catch return null;
    m.* = BigInt.init(allocator) catch {
        allocator.destroy(m);
        return null;
    };
    return m;
}

/// Free a BigInt allocated by any of the ``alloc_*`` constructors
/// in this module.  Calls ``deinit`` (releasing the limb buffer)
/// then destroys the struct itself.  Null is a no-op.
pub fn destroy(m: ?*BigInt) void {
    if (m) |p| {
        p.deinit();
        allocator.destroy(p);
    }
}

/// Parse a Tcl integer literal (decimal / hex / octal / binary,
/// optional leading sign, optional surrounding whitespace) into a
/// freshly-allocated BigInt.  Returns ``null`` on parse error or
/// OOM.  The returned BigInt has unbounded precision.
pub fn alloc_from_string(ptr: u32, len: u32) ?*BigInt {
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

    var base: u8 = 10;
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

    // Find the end of the digit run, tolerating only trailing
    // whitespace beyond it (matches the i128 parser's discipline so
    // both routes accept the same set of strings).
    var end = i;
    while (end < len and !is_space(src[end])) end += 1;
    var trail = end;
    while (trail < len and is_space(src[trail])) trail += 1;
    if (trail != len) return null;

    const body_len = end - i;
    if (body_len == 0) return null;

    // Validate digits up-front so a body like ``0xZZ`` is rejected
    // before we incur the allocation; ``Managed.setString`` only
    // checks once it starts consuming digits and would surface a
    // partial-state error.
    for (0..body_len) |k| {
        const c = src[i + k];
        const ok = switch (base) {
            2 => c == '0' or c == '1',
            8 => c >= '0' and c <= '7',
            10 => c >= '0' and c <= '9',
            16 => (c >= '0' and c <= '9') or (c >= 'a' and c <= 'f') or (c >= 'A' and c <= 'F'),
            else => unreachable,
        };
        if (!ok) return null;
    }

    // Copy the digit body into an owned slice so ``setString`` sees
    // a contiguous []const u8 (it can't take a (ptr, len) directly).
    const body_buf = allocator.alloc(u8, body_len) catch return null;
    defer allocator.free(body_buf);
    for (0..body_len) |k| body_buf[k] = src[i + k];

    const m = allocator.create(BigInt) catch return null;
    m.* = BigInt.init(allocator) catch {
        allocator.destroy(m);
        return null;
    };
    m.setString(base, body_buf) catch {
        m.deinit();
        allocator.destroy(m);
        return null;
    };
    if (negative and !m.eqlZero()) m.negate();
    return m;
}

/// True iff the input string parses as an integer literal that
/// requires more than 64 bits to represent.  Used by the
/// arithmetic helpers' ``is_bignum`` check to decide between the
/// fast i64 / i128 paths and the Managed path.
///
/// Parses internally; allocates only if the digit body needs more
/// than i128 magnitude.  Frees its scratch BigInt before returning
/// to keep this a pure predicate.
pub fn string_needs_bignum(ptr: u32, len: u32) bool {
    // Quick path: the i64 parser succeeds → fits i64 → not bignum.
    // (Caller has often already done this check, but repeating it
    // here makes ``string_needs_bignum`` self-contained.)
    return alloc_from_string(ptr, len) != null and blk: {
        const m = alloc_from_string(ptr, len).?;
        defer destroy(m);
        break :blk !m.fits(i64);
    };
}

/// Render the BigInt as a decimal string into a heap-allocated
/// buffer owned by the caller.  Returns ``null`` on OOM.
pub fn alloc_format(m: *const BigInt) ?[]u8 {
    return m.toString(allocator, 10, .lower) catch null;
}

/// Render the BigInt in *base* (2, 8, 10, or 16) into a heap-
/// allocated buffer.  The Zig 0.16 ``Managed.toString`` has a
/// limb-boundary bug for power-of-two bases that don't evenly
/// divide ``Limb`` (notably base 8 on 32-bit wasm limbs), so for
/// base 8 we walk the bit array directly.  Bases 10 and 16 route
/// through the stdlib path, which is correct on those.
pub fn alloc_format_base(m: *const BigInt, base: u8, case: std.fmt.Case) ?[]u8 {
    if (base != 8) return m.toString(allocator, base, case) catch null;
    // Octal has no upper/lower variant — ``case`` is consumed by
    // the non-octal early return above; nothing to do with it here.
    // Custom base-8 path.  Walk the bit array MSB-first, emitting
    // 3-bit groups.  This avoids the per-limb extraction bug that
    // gives wrong digits for bits straddling limb boundaries.
    const c = m.toConst();
    if (c.eqlZero()) return allocator.dupe(u8, "0") catch null;
    // Find total significant bit count.
    const Limb = std.math.big.Limb;
    const limb_bits: usize = @bitSizeOf(Limb);
    const len = c.limbs.len;
    var top_limb_bits: usize = limb_bits;
    if (len > 0) {
        var lz: usize = 0;
        var top = c.limbs[len - 1];
        while (top != 0) : (top >>= 1) lz += 1;
        top_limb_bits = lz;
    }
    const total_bits = (len - 1) * limb_bits + top_limb_bits;
    if (total_bits == 0) return allocator.dupe(u8, "0") catch null;

    // Number of octal digits needed: ceil(total_bits / 3).
    const ndigits: usize = (total_bits + 2) / 3;
    const sign_bytes: usize = if (!c.positive) 1 else 0;
    const buf = allocator.alloc(u8, ndigits + sign_bytes) catch return null;
    errdefer allocator.free(buf);

    if (!c.positive) buf[0] = '-';
    var off: usize = sign_bytes;

    // Walk bits MSB-first.  For each output digit, accumulate up to
    // 3 bits.  Start at the top of the highest significant bit and
    // work down.
    // The very first (most-significant) digit may have 1 or 2 bits
    // (when total_bits % 3 != 0).
    const first_bits: usize = ((total_bits - 1) % 3) + 1;
    var bits_left = total_bits;
    var first = true;
    while (bits_left > 0) {
        const take: usize = if (first) first_bits else 3;
        first = false;
        var digit: u8 = 0;
        var k: usize = 0;
        while (k < take) : (k += 1) {
            bits_left -= 1;
            const bit_idx = bits_left;
            const limb_idx = bit_idx / limb_bits;
            const bit_in_limb = bit_idx % limb_bits;
            const bit: u1 = @intCast((c.limbs[limb_idx] >> @intCast(bit_in_limb)) & 1);
            digit = (digit << 1) | bit;
        }
        buf[off] = '0' + digit;
        off += 1;
    }
    return buf[0..off];
}

/// True iff the BigInt's value fits exactly in an i64.
pub fn fits_i64(m: *const BigInt) bool {
    return m.fits(i64);
}

/// True iff the BigInt's value fits exactly in an i128.
pub fn fits_i128(m: *const BigInt) bool {
    return m.fits(i128);
}

/// Convert the BigInt to i64.  Returns the truncated low 64 bits
/// when the value is too wide; matches the existing ``obj_get_int``
/// behaviour for TYPE_BIGNUM operands.
pub fn to_i64(m: *const BigInt) i64 {
    return m.toInt(i64) catch blk: {
        // Truncate by reading the low limb(s) directly from the
        // little-endian limb array.  Limb is u32 on wasm32 so we
        // need the bottom two limbs to recover an i64; subtract
        // for negative magnitudes.
        const c = m.toConst();
        var u: u64 = 0;
        const Limb = std.math.big.Limb;
        const limb_bytes = @sizeOf(Limb);
        const limbs_for_i64 = @divExact(8, limb_bytes);
        var k: usize = 0;
        while (k < @min(c.limbs.len, limbs_for_i64)) : (k += 1) {
            u |= @as(u64, c.limbs[k]) << @intCast(k * limb_bytes * 8);
        }
        const i: i64 = @bitCast(u);
        break :blk if (c.positive) i else -%i;
    };
}

/// Convert the BigInt to i128.  Same truncation discipline as
/// :func:`to_i64`.
pub fn to_i128(m: *const BigInt) i128 {
    return m.toInt(i128) catch blk: {
        const c = m.toConst();
        var u: u128 = 0;
        const Limb = std.math.big.Limb;
        const limb_bytes = @sizeOf(Limb);
        const limbs_for_i128 = @divExact(16, limb_bytes);
        var k: usize = 0;
        while (k < @min(c.limbs.len, limbs_for_i128)) : (k += 1) {
            u |= @as(u128, c.limbs[k]) << @intCast(k * limb_bytes * 8);
        }
        const i: i128 = @bitCast(u);
        break :blk if (c.positive) i else -%i;
    };
}

/// Convert the BigInt to f64 (round-to-nearest-even).
pub fn to_f64(m: *const BigInt) f64 {
    return m.toFloat(f64, .nearest_even).@"0";
}

// Arithmetic — each ``alloc_<op>`` returns a freshly-allocated
// BigInt holding the result.  Returns ``null`` on OOM (limb buffer
// allocation failure inside the op).  Callers are responsible for
// destroying the returned BigInt.

fn alloc_binop(comptime op: enum { add, sub, mul }) fn (a: *const BigInt, b: *const BigInt) ?*BigInt {
    return struct {
        fn run(a: *const BigInt, b: *const BigInt) ?*BigInt {
            const r = alloc_zero() orelse return null;
            const res = switch (op) {
                .add => r.add(a, b),
                .sub => r.sub(a, b),
                .mul => r.mul(a, b),
            };
            res catch {
                destroy(r);
                return null;
            };
            return r;
        }
    }.run;
}

pub const alloc_add = alloc_binop(.add);
pub const alloc_sub = alloc_binop(.sub);
pub const alloc_mul = alloc_binop(.mul);

/// Truncated integer division (toward zero).  Returns ``null`` on
/// division by zero or OOM.
pub fn alloc_div_trunc(a: *const BigInt, b: *const BigInt) ?*BigInt {
    if (b.eqlZero()) return null;
    const q = alloc_zero() orelse return null;
    const r = alloc_zero() orelse {
        destroy(q);
        return null;
    };
    defer destroy(r);
    BigInt.divTrunc(q, r, a, b) catch {
        destroy(q);
        return null;
    };
    return q;
}

/// Tcl-correct mod (result has the divisor's sign).  Mirrors
/// ``tclExecute.c`` INST_MOD: ``r = a % b; if r != 0 and signs
/// differ then r += b``.  Returns ``null`` on division by zero or
/// OOM.
pub fn alloc_mod_floor(a: *const BigInt, b: *const BigInt) ?*BigInt {
    if (b.eqlZero()) return null;
    const q = alloc_zero() orelse return null;
    defer destroy(q);
    const r = alloc_zero() orelse return null;
    BigInt.divTrunc(q, r, a, b) catch {
        destroy(r);
        return null;
    };
    // Sign-fixup: if remainder is non-zero and its sign differs
    // from the divisor's, add the divisor.  Both BigInt.add and
    // the destroy on the swap path can OOM; we propagate as null.
    if (!r.eqlZero() and r.isPositive() != b.isPositive()) {
        const r2 = alloc_zero() orelse {
            destroy(r);
            return null;
        };
        r2.add(r, b) catch {
            destroy(r);
            destroy(r2);
            return null;
        };
        destroy(r);
        return r2;
    }
    return r;
}

/// Left-shift by an arbitrary count.  Returns ``null`` on OOM or
/// if the shift count exceeds ``usize`` (impossible in practice for
/// any well-formed Tcl program — the TclObj stack would overflow
/// long before we tried to ``1 << 2^64``).
pub fn alloc_shl(a: *const BigInt, count: u64) ?*BigInt {
    const r = alloc_zero() orelse return null;
    r.shiftLeft(a, @intCast(count)) catch {
        destroy(r);
        return null;
    };
    return r;
}

/// Unary negate.  Returns ``null`` on OOM (the clone may need to
/// allocate a fresh limb buffer).
pub fn alloc_neg(a: *const BigInt) ?*BigInt {
    const r = a.clone() catch return null;
    const r_heap = allocator.create(BigInt) catch {
        var rr = r;
        rr.deinit();
        return null;
    };
    r_heap.* = r;
    if (!r_heap.eqlZero()) r_heap.negate();
    return r_heap;
}

/// ``a ** b`` for non-negative exponent ``b``.  Returns ``null``
/// on OOM.  Uses ``Managed.pow`` which takes a u32 exponent —
/// callers must reject huge ``b`` values (no real Tcl program
/// computes ``x ** (1 << 32)``; the limb buffer would not fit
/// anyway).
pub fn alloc_pow(a: *const BigInt, b: u32) ?*BigInt {
    const r = alloc_zero() orelse return null;
    r.pow(a, b) catch {
        destroy(r);
        return null;
    };
    return r;
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

// ====================================================================
// Stage 2 — BigInt (Managed) tests
// ====================================================================
//
// Coverage focus: arbitrary-precision parse / format / arithmetic
// for values that exceed i128.  The Stage 1 tests above already
// pin the i128 fast path; these check that the Managed path
// matches the same semantics and extends them to magnitudes the
// upstream test suite needs (``expr-old-36.{11,14,16}``).

test "BigInt — alloc_from_int and to_i64" {
    const m = alloc_from_int(42).?;
    defer destroy(m);
    try testing.expectEqual(@as(i64, 42), to_i64(m));
    try testing.expect(fits_i64(m));
}

test "BigInt — alloc_from_string parses i64 boundary" {
    const s = "9223372036854775807";
    const sl = slice_to_addr_len(s);
    const m = alloc_from_string(sl.ptr, sl.len).?;
    defer destroy(m);
    try testing.expect(fits_i64(m));
    try testing.expectEqual(@as(i64, std.math.maxInt(i64)), to_i64(m));
}

test "BigInt — alloc_from_string parses 80-bit literal (expr-old-36.11)" {
    // ``665802003400000000000000`` from the upstream regression
    // test — fits in 80 bits, exceeds i64.
    const s = "665802003400000000000000";
    const sl = slice_to_addr_len(s);
    const m = alloc_from_string(sl.ptr, sl.len).?;
    defer destroy(m);
    try testing.expect(!fits_i64(m));
    try testing.expect(fits_i128(m));
    const buf = alloc_format(m).?;
    defer allocator.free(buf);
    try testing.expectEqualStrings(s, buf);
}

test "BigInt — alloc_from_string parses 152-bit hex literal (expr-old-36.16)" {
    const s = "0xffffffffffffffffffffffffffffffffffffff";
    const sl = slice_to_addr_len(s);
    const m = alloc_from_string(sl.ptr, sl.len).?;
    defer destroy(m);
    try testing.expect(!fits_i128(m));
    const buf = alloc_format(m).?;
    defer allocator.free(buf);
    try testing.expectEqualStrings("5708990770823839524233143877797980545530986495", buf);
}

test "BigInt — alloc_from_string negative literal" {
    const s = "-12345678901234567890123456";
    const sl = slice_to_addr_len(s);
    const m = alloc_from_string(sl.ptr, sl.len).?;
    defer destroy(m);
    try testing.expect(!m.isPositive());
    const buf = alloc_format(m).?;
    defer allocator.free(buf);
    try testing.expectEqualStrings(s, buf);
}

test "BigInt — alloc_from_string rejects garbage" {
    const cases = [_][]const u8{ "", "abc", "12abc", "  ", "0o9", "0xZZ" };
    for (cases) |c| {
        const sl = slice_to_addr_len(c);
        try testing.expectEqual(@as(?*BigInt, null), alloc_from_string(sl.ptr, sl.len));
    }
}

test "BigInt — alloc_add chains past i128 boundary" {
    const a = alloc_from_int(@as(i128, 1) << 100).?;
    defer destroy(a);
    const b = alloc_from_int(@as(i128, 1) << 100).?;
    defer destroy(b);
    const r = alloc_add(a, b).?;
    defer destroy(r);
    try testing.expect(fits_i128(r));
    const buf = alloc_format(r).?;
    defer allocator.free(buf);
    try testing.expectEqualStrings("2535301200456458802993406410752", buf);
}

test "BigInt — alloc_mul produces values exceeding i128" {
    // ``2^100 * 2^100`` = ``2^200`` — needs > 128 bits.
    const a = alloc_from_int(@as(i128, 1) << 100).?;
    defer destroy(a);
    const r = alloc_mul(a, a).?;
    defer destroy(r);
    try testing.expect(!fits_i128(r));
    const buf = alloc_format(r).?;
    defer allocator.free(buf);
    try testing.expectEqualStrings("1606938044258990275541962092341162602522202993782792835301376", buf);
}

test "BigInt — alloc_shl by huge count" {
    const a = alloc_from_int(1).?;
    defer destroy(a);
    const r = alloc_shl(a, 200).?;
    defer destroy(r);
    try testing.expect(!fits_i128(r));
    const buf = alloc_format(r).?;
    defer allocator.free(buf);
    try testing.expectEqualStrings("1606938044258990275541962092341162602522202993782792835301376", buf);
}

test "BigInt — alloc_div_trunc rejects divide by zero" {
    const a = alloc_from_int(10).?;
    defer destroy(a);
    const b = alloc_from_int(0).?;
    defer destroy(b);
    try testing.expectEqual(@as(?*BigInt, null), alloc_div_trunc(a, b));
}

test "BigInt — alloc_mod_floor matches Tcl semantics on mixed signs" {
    // ``-1 % (1 << 63)`` = ``9223372036854775807`` (Tcl mod has divisor's sign).
    const a = alloc_from_int(-1).?;
    defer destroy(a);
    const b = alloc_from_int(@as(i128, 1) << 63).?;
    defer destroy(b);
    const r = alloc_mod_floor(a, b).?;
    defer destroy(r);
    const buf = alloc_format(r).?;
    defer allocator.free(buf);
    try testing.expectEqualStrings("9223372036854775807", buf);
}

test "BigInt — alloc_neg of large value" {
    const a = alloc_from_int(@as(i128, std.math.maxInt(i64)) + 1).?;
    defer destroy(a);
    const r = alloc_neg(a).?;
    defer destroy(r);
    const buf = alloc_format(r).?;
    defer allocator.free(buf);
    try testing.expectEqualStrings("-9223372036854775808", buf);
}

test "BigInt — round-trip large value through string" {
    const s = "123456789012345678901234567890";
    const sl = slice_to_addr_len(s);
    const m = alloc_from_string(sl.ptr, sl.len).?;
    defer destroy(m);
    const buf = alloc_format(m).?;
    defer allocator.free(buf);
    try testing.expectEqualStrings(s, buf);
}

test "BigInt — destroy(null) is a no-op" {
    destroy(null);
}
