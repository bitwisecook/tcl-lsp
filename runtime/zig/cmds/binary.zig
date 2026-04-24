// ``binary`` — binary string manipulation.
//
// Subcommands:
//   binary format fmtStr ?val ...?  — pack values into a binary string
//   binary scan  data fmtStr ?var ...?  — unpack binary data into variables
//
// Format specifiers (WASM is little-endian):
//   c / C   8-bit integer  (C = unsigned — same bytes, different scan signedness)
//   s / S   16-bit  little-endian / big-endian
//   t / T   16-bit  native (little) / big-endian   (Tcl 8.6)
//   i / I   32-bit  little-endian / big-endian
//   n / N   32-bit  native (little) / big-endian   (Tcl 8.6)
//   w / W   64-bit  little-endian / big-endian
//   m / M   64-bit  native (little) / big-endian   (Tcl 8.6)
//   f / r   32-bit IEEE 754 float, native/little-endian
//   R       32-bit float big-endian
//   d / q   64-bit IEEE 754 double, native/little-endian
//   Q       64-bit double big-endian
//   a / A   byte string (a = null-padded, A = space-padded to width)
//   h / H   hex string, low-nibble-first / high-nibble-first
//   b / B   bit string, low-bit-first / high-bit-first
//   x       insert/skip null byte(s)
//   X       move back n bytes in output/input
//   @       set absolute position
//
// Count modifier: a decimal number immediately after the type letter
// specifies how many values to consume (format) or bytes to process
// (scan).  ``*`` means "all remaining".

const rt     = @import("../tcl_runtime.zig");
const frames = @import("../interp/tcl_frames.zig");
const reg    = @import("../dispatch/tcl_cmd_registry.zig");

const obj_new_int       = rt.obj_new_int;
const obj_new_string    = rt.obj_new_string;
const obj_ensure_string = rt.obj_ensure_string;
const obj_get_int       = rt.obj_get_int;
const alloc             = rt.alloc;
const memcpy            = rt.memcpy;

// ─── shared helpers ──────────────────────────────────────────────────────────

/// Parse an optional decimal count from ``fmt`` at position ``*fi``.
/// Returns the count (or ``null`` for ``*`` which means "all remaining").
/// If no digits, returns 1.
fn parse_count(fmt: [*]const u8, fmt_len: u32, fi: *u32) ?u32 {
    if (fi.* >= fmt_len) return 1;
    if (fmt[fi.*] == '*') {
        fi.* += 1;
        return null; // ``*`` = all remaining
    }
    if (fmt[fi.*] < '0' or fmt[fi.*] > '9') return 1;
    var n: u32 = 0;
    while (fi.* < fmt_len and fmt[fi.*] >= '0' and fmt[fi.*] <= '9') : (fi.* += 1) {
        n = n * 10 + @as(u32, fmt[fi.*] - '0');
    }
    return n;
}

/// Byte-width of a numeric specifier (0 = unknown / string type).
fn spec_byte_width(spec: u8) u32 {
    return switch (spec) {
        'c', 'C' => 1,
        's', 'S', 't', 'T' => 2,
        'i', 'I', 'n', 'N', 'f', 'r', 'R' => 4,
        'w', 'W', 'm', 'M', 'd', 'q', 'Q' => 8,
        else => 0,
    };
}

/// Write a little-endian integer ``val`` into ``buf[off..]`` using ``nbytes``
/// bytes.  Clamps to the type width (sign-extends / zero-fills).
fn write_le(buf: u32, off: u32, val: i64, nbytes: u32) void {
    const dst: [*]u8 = @ptrFromInt(buf + off);
    var v: u64 = @bitCast(val);
    for (0..nbytes) |k| {
        dst[k] = @intCast(v & 0xFF);
        v >>= 8;
    }
}

/// Write a big-endian integer into ``buf[off..]`` using ``nbytes`` bytes.
fn write_be(buf: u32, off: u32, val: i64, nbytes: u32) void {
    const dst: [*]u8 = @ptrFromInt(buf + off);
    var v: u64 = @bitCast(val);
    var k: u32 = nbytes;
    while (k > 0) : (k -= 1) {
        dst[k - 1] = @intCast(v & 0xFF);
        v >>= 8;
    }
}

/// Read a signed little-endian integer from ``src[off..]`` of ``nbytes``.
fn read_le_signed(src: u32, off: u32, nbytes: u32) i64 {
    const p: [*]const u8 = @ptrFromInt(src + off);
    var v: u64 = 0;
    for (0..nbytes) |k| {
        v |= @as(u64, p[k]) << @intCast(k * 8);
    }
    if (nbytes >= 8) return @bitCast(v);
    // Sign-extend via arithmetic right shift on i64.
    const shift: u6 = @intCast(64 - nbytes * 8);
    const sval: i64 = @bitCast(v << shift);
    return sval >> @intCast(shift);
}

/// Read an unsigned little-endian integer.
fn read_le_unsigned(src: u32, off: u32, nbytes: u32) i64 {
    const p: [*]const u8 = @ptrFromInt(src + off);
    var v: u64 = 0;
    for (0..nbytes) |k| {
        v |= @as(u64, p[k]) << @intCast(k * 8);
    }
    return @bitCast(v);
}

/// Read a signed big-endian integer.
fn read_be_signed(src: u32, off: u32, nbytes: u32) i64 {
    const p: [*]const u8 = @ptrFromInt(src + off);
    var v: u64 = 0;
    for (0..nbytes) |k| {
        v = (v << 8) | @as(u64, p[k]);
    }
    if (nbytes >= 8) return @bitCast(v);
    // Sign-extend via arithmetic right shift on i64.
    const shift: u6 = @intCast(64 - nbytes * 8);
    const sval: i64 = @bitCast(v << shift);
    return sval >> @intCast(shift);
}

/// Read an unsigned big-endian integer.
fn read_be_unsigned(src: u32, off: u32, nbytes: u32) i64 {
    const p: [*]const u8 = @ptrFromInt(src + off);
    var v: u64 = 0;
    for (0..nbytes) |k| {
        v = (v << 8) | @as(u64, p[k]);
    }
    return @bitCast(v);
}

/// True for big-endian specifiers.
fn is_be(spec: u8) bool {
    return switch (spec) {
        'S', 'I', 'W', 'N', 'T', 'M', 'R', 'Q' => true,
        else => false,
    };
}

// ─── binary format ───────────────────────────────────────────────────────────

/// First pass: calculate the byte length that ``binary format fmtStr args...``
/// will produce.  Advances ``*wi`` and ``*fi`` together — call with wi=0, fi=0
/// to get the total length.  Returns 0 on any sizing error.
fn format_size(fmt: [*]const u8, fmt_len: u32,
               words: []const i32, words_offset: u32) u32 {
    var fi: u32 = 0;
    var wi: u32 = words_offset;
    var total: u32 = 0;

    while (fi < fmt_len) {
        const spec = fmt[fi];
        fi += 1;

        const count_or_null = parse_count(fmt, fmt_len, &fi);

        switch (spec) {
            'c', 'C', 's', 'S', 't', 'T', 'i', 'I', 'n', 'N',
            'w', 'W', 'm', 'M', 'f', 'r', 'R', 'd', 'q', 'Q' => {
                const nbytes = spec_byte_width(spec);
                const cnt: u32 = count_or_null orelse blk: {
                    if (wi >= words.len) break :blk 0;
                    // ``*`` for numeric = remaining words
                    break :blk @intCast(words.len - wi);
                };
                total += nbytes * cnt;
                wi += cnt;
            },
            'a', 'A' => {
                const str_len: u32 = if (wi < words.len) obj_ensure_string(words[wi]).len else 0;
                const cnt: u32 = count_or_null orelse str_len;
                total += cnt;
                if (wi < words.len) wi += 1;
            },
            'h', 'H' => {
                if (wi >= words.len) continue;
                const vs = obj_ensure_string(words[wi]);
                const cnt: u32 = count_or_null orelse vs.len;
                total += (cnt + 1) / 2;
                wi += 1;
            },
            'b', 'B' => {
                if (wi >= words.len) continue;
                const vs = obj_ensure_string(words[wi]);
                const cnt: u32 = count_or_null orelse vs.len;
                total += (cnt + 7) / 8;
                wi += 1;
            },
            'x' => {
                const cnt: u32 = count_or_null orelse 1;
                total += cnt;
            },
            'X' => {
                // Back up — can shrink total (but not below 0).
                const cnt: u32 = count_or_null orelse 1;
                total -|= cnt;
            },
            '@' => {
                // Set absolute position.
                const cnt: u32 = count_or_null orelse 0;
                if (cnt > total) total = cnt;
            },
            else => {}, // ignore unknown
        }
    }
    return total;
}

/// Second pass: fill the already-allocated ``buf`` of ``buf_len`` bytes.
fn format_fill(buf: u32, buf_len: u32,
               fmt: [*]const u8, fmt_len: u32,
               words: []const i32, words_offset: u32) void {
    var fi: u32 = 0;
    var wi: u32 = words_offset;
    var off: u32 = 0;

    while (fi < fmt_len and off <= buf_len) {
        const spec = fmt[fi];
        fi += 1;

        const count_or_null = parse_count(fmt, fmt_len, &fi);

        switch (spec) {
            'c', 'C', 's', 'S', 't', 'T', 'i', 'I', 'n', 'N',
            'w', 'W', 'm', 'M', 'f', 'r', 'R', 'd', 'q', 'Q' => {
                const nbytes = spec_byte_width(spec);
                const cnt: u32 = count_or_null orelse blk: {
                    if (wi >= words.len) break :blk 0;
                    break :blk @intCast(words.len - wi);
                };
                const big_end = is_be(spec);
                for (0..cnt) |_| {
                    if (wi >= words.len or off + nbytes > buf_len) break;
                    const v: i64 = obj_get_int(words[wi]);
                    if (big_end) write_be(buf, off, v, nbytes)
                    else         write_le(buf, off, v, nbytes);
                    off += nbytes;
                    wi += 1;
                }
            },
            'a', 'A' => {
                if (wi >= words.len) continue;
                const vs = obj_ensure_string(words[wi]);
                const cnt: u32 = count_or_null orelse vs.len;
                const copy_len = @min(cnt, vs.len);
                if (copy_len > 0 and off + copy_len <= buf_len) {
                    memcpy(buf + off, vs.ptr, copy_len);
                }
                // Pad the remainder with NUL ('a') or space ('A').
                const pad: u8 = if (spec == 'A') ' ' else 0;
                const dst: [*]u8 = @ptrFromInt(buf);
                var k: u32 = copy_len;
                while (k < cnt and off + k < buf_len) : (k += 1) {
                    dst[off + k] = pad;
                }
                off += cnt;
                wi += 1;
            },
            'h', 'H' => {
                if (wi >= words.len) continue;
                const vs = obj_ensure_string(words[wi]);
                const src_chars: [*]const u8 = @ptrFromInt(vs.ptr);
                const cnt: u32 = count_or_null orelse vs.len;
                const dst: [*]u8 = @ptrFromInt(buf);
                const out_bytes = (cnt + 1) / 2;
                // Zero out the output bytes first.
                var k: u32 = 0;
                while (k < out_bytes and off + k < buf_len) : (k += 1) {
                    dst[off + k] = 0;
                }
                // Fill nibbles.
                k = 0;
                while (k < cnt and off + k / 2 < buf_len) : (k += 1) {
                    const c = if (k < vs.len) src_chars[k] else '0';
                    const nibble: u8 = if (c >= '0' and c <= '9') c - '0'
                        else if (c >= 'a' and c <= 'f') c - 'a' + 10
                        else if (c >= 'A' and c <= 'F') c - 'A' + 10
                        else 0;
                    if (spec == 'h') {
                        // low nibble first: even k → low nibble, odd k → high
                        if (k % 2 == 0) dst[off + k / 2] |= nibble
                        else            dst[off + k / 2] |= nibble << 4;
                    } else {
                        // high nibble first
                        if (k % 2 == 0) dst[off + k / 2] |= nibble << 4
                        else            dst[off + k / 2] |= nibble;
                    }
                }
                off += out_bytes;
                wi += 1;
            },
            'b', 'B' => {
                if (wi >= words.len) continue;
                const vs = obj_ensure_string(words[wi]);
                const src_chars: [*]const u8 = @ptrFromInt(vs.ptr);
                const cnt: u32 = count_or_null orelse vs.len;
                const dst: [*]u8 = @ptrFromInt(buf);
                const out_bytes = (cnt + 7) / 8;
                var k: u32 = 0;
                while (k < out_bytes and off + k < buf_len) : (k += 1) dst[off + k] = 0;
                k = 0;
                while (k < cnt and off + k / 8 < buf_len) : (k += 1) {
                    const c = if (k < vs.len) src_chars[k] else '0';
                    const bit: u8 = if (c == '1') 1 else 0;
                    if (spec == 'b') {
                        dst[off + k / 8] |= bit << @intCast(k % 8);
                    } else {
                        dst[off + k / 8] |= bit << @intCast(7 - k % 8);
                    }
                }
                off += out_bytes;
                wi += 1;
            },
            'x' => {
                const cnt: u32 = count_or_null orelse 1;
                const dst: [*]u8 = @ptrFromInt(buf);
                var k: u32 = 0;
                while (k < cnt and off + k < buf_len) : (k += 1) dst[off + k] = 0;
                off += cnt;
            },
            'X' => {
                const cnt: u32 = count_or_null orelse 1;
                off -|= cnt;
            },
            '@' => {
                const cnt: u32 = count_or_null orelse 0;
                off = @min(cnt, buf_len);
            },
            else => {},
        }
    }
}

fn eval_binary_format(words: []const i32) i32 {
    // words: ["format", fmtStr, val0, val1, ...]
    if (words.len < 2) return obj_new_string(0, 0);
    const fs = obj_ensure_string(words[1]);
    if (fs.len == 0) return obj_new_string(0, 0);
    const fmt: [*]const u8 = @ptrFromInt(fs.ptr);

    const out_len = format_size(fmt, fs.len, words, 2);
    if (out_len == 0) return obj_new_string(0, 0);
    const buf = alloc(out_len);
    // Zero the buffer so unfilled bytes are NUL.
    const dst: [*]u8 = @ptrFromInt(buf);
    for (0..out_len) |k| dst[k] = 0;
    format_fill(buf, out_len, fmt, fs.len, words, 2);
    return obj_new_string(@bitCast(buf), @bitCast(out_len));
}

// ─── binary scan ─────────────────────────────────────────────────────────────

fn eval_binary_scan(words: []const i32) i32 {
    // words: ["scan", data, fmtStr, var0, var1, ...]
    if (words.len < 3) return obj_new_int(0);
    const data_s = obj_ensure_string(words[1]);
    const fs     = obj_ensure_string(words[2]);
    if (fs.len == 0) return obj_new_int(0);

    const src_base = data_s.ptr;
    const src_len  = data_s.len;
    const fmt: [*]const u8 = @ptrFromInt(fs.ptr);
    const fmt_len  = fs.len;

    var off: u32 = 0;   // byte position in data
    var vi:  u32 = 0;   // next variable index (words[3 + vi])
    var assigned: u32 = 0;
    var fi: u32 = 0;

    while (fi < fmt_len) {
        const spec = fmt[fi];
        fi += 1;

        const count_or_null = parse_count(fmt, fmt_len, &fi);

        switch (spec) {
            'c', 'C', 's', 'S', 't', 'T', 'i', 'I', 'n', 'N',
            'w', 'W', 'm', 'M', 'f', 'r', 'R', 'd', 'q', 'Q' => {
                const nbytes = spec_byte_width(spec);
                // ``*`` = read all remaining bytes / nbytes items.
                const cnt: u32 = count_or_null orelse
                    if (nbytes > 0) (src_len -| off) / nbytes else 0;
                const big_end = is_be(spec);
                const signed = switch (spec) {
                    'C' => false,
                    else => true,
                };

                if (cnt == 1) {
                    // Scalar: assign single value to variable.
                    if (off + nbytes > src_len) break;
                    const v: i64 = if (big_end)
                        (if (signed) read_be_signed(src_base, off, nbytes)
                                    else read_be_unsigned(src_base, off, nbytes))
                    else
                        (if (signed) read_le_signed(src_base, off, nbytes)
                                    else read_le_unsigned(src_base, off, nbytes));
                    off += nbytes;
                    if (words.len > 3 + vi) {
                        _ = frames.var_set(words[3 + vi], obj_new_int(v));
                        vi += 1;
                        assigned += 1;
                    }
                } else {
                    // Multiple: build a Tcl list.
                    // Each element is at most 20 chars + space.
                    const list_buf = alloc(cnt * 24);
                    var list_off: u32 = 0;
                    const list_dst: [*]u8 = @ptrFromInt(list_buf);
                    var k: u32 = 0;
                    while (k < cnt and off + nbytes <= src_len) : (k += 1) {
                        const v: i64 = if (big_end)
                            (if (signed) read_be_signed(src_base, off, nbytes)
                                        else read_be_unsigned(src_base, off, nbytes))
                        else
                            (if (signed) read_le_signed(src_base, off, nbytes)
                                        else read_le_unsigned(src_base, off, nbytes));
                        off += nbytes;
                        if (k > 0) { list_dst[list_off] = ' '; list_off += 1; }
                        // Format the integer as decimal.
                        list_off += fmt_i64(list_dst, list_off, v);
                    }
                    if (words.len > 3 + vi) {
                        _ = frames.var_set(words[3 + vi],
                            obj_new_string(@bitCast(list_buf), @bitCast(list_off)));
                        vi += 1;
                        assigned += 1;
                    }
                }
            },
            'a', 'A' => {
                const cnt: u32 = count_or_null orelse (src_len -| off);
                const take = @min(cnt, src_len -| off);
                var end = off + take;
                if (spec == 'A') {
                    // Strip trailing spaces/nulls from result.
                    while (end > off and (
                        @as(*const u8, @ptrFromInt(src_base + end - 1)).* == ' ' or
                        @as(*const u8, @ptrFromInt(src_base + end - 1)).* == 0
                    )) end -= 1;
                }
                if (words.len > 3 + vi) {
                    _ = frames.var_set(words[3 + vi],
                        obj_new_string(@bitCast(src_base + off), @bitCast(end - off)));
                    vi += 1;
                    assigned += 1;
                }
                off += take;
            },
            'h', 'H' => {
                const cnt: u32 = count_or_null orelse ((src_len -| off) * 2);
                const take_bytes = (cnt + 1) / 2;
                if (off + take_bytes > src_len) break;
                const hex_buf = alloc(cnt + 1);
                const hex_dst: [*]u8 = @ptrFromInt(hex_buf);
                const src_p: [*]const u8 = @ptrFromInt(src_base + off);
                const hex_chars = "0123456789abcdef";
                var k: u32 = 0;
                while (k < cnt) : (k += 1) {
                    const byte = src_p[k / 2];
                    const nibble: u4 = if (spec == 'h')
                        @intCast(if (k % 2 == 0) byte & 0xF else (byte >> 4) & 0xF)
                    else
                        @intCast(if (k % 2 == 0) (byte >> 4) & 0xF else byte & 0xF);
                    hex_dst[k] = hex_chars[nibble];
                }
                if (words.len > 3 + vi) {
                    _ = frames.var_set(words[3 + vi],
                        obj_new_string(@bitCast(hex_buf), @bitCast(cnt)));
                    vi += 1;
                    assigned += 1;
                }
                off += take_bytes;
            },
            'b', 'B' => {
                const cnt: u32 = count_or_null orelse ((src_len -| off) * 8);
                const take_bytes = (cnt + 7) / 8;
                if (off + take_bytes > src_len) break;
                const bit_buf = alloc(cnt + 1);
                const bit_dst: [*]u8 = @ptrFromInt(bit_buf);
                const src_p: [*]const u8 = @ptrFromInt(src_base + off);
                var k: u32 = 0;
                while (k < cnt) : (k += 1) {
                    const byte = src_p[k / 8];
                    const bit: u8 = if (spec == 'b')
                        (byte >> @intCast(k % 8)) & 1
                    else
                        (byte >> @intCast(7 - k % 8)) & 1;
                    bit_dst[k] = '0' + bit;
                }
                if (words.len > 3 + vi) {
                    _ = frames.var_set(words[3 + vi],
                        obj_new_string(@bitCast(bit_buf), @bitCast(cnt)));
                    vi += 1;
                    assigned += 1;
                }
                off += take_bytes;
            },
            'x' => {
                const cnt: u32 = count_or_null orelse 1;
                off += @min(cnt, src_len -| off);
            },
            'X' => {
                const cnt: u32 = count_or_null orelse 1;
                off -|= cnt;
            },
            '@' => {
                const cnt: u32 = count_or_null orelse 0;
                off = @min(cnt, src_len);
            },
            else => {},
        }
    }
    return obj_new_int(@intCast(assigned));
}

/// Format a signed 64-bit integer as decimal into ``dst[off..]``.
/// Returns the number of bytes written.
fn fmt_i64(dst: [*]u8, off: u32, val: i64) u32 {
    if (val == 0) { dst[off] = '0'; return 1; }
    var buf: [20]u8 = undefined;
    var pos: u32 = 20;
    // Use wrapping subtraction on the u64 bit pattern to handle INT64_MIN
    // (-9223372036854775808) without overflow: -(INT64_MIN) overflows i64.
    var v: u64 = if (val < 0) 0 -% @as(u64, @bitCast(val)) else @bitCast(val);
    while (v > 0) : (v /= 10) {
        pos -= 1;
        buf[pos] = @intCast('0' + (v % 10));
    }
    var len: u32 = 0;
    if (val < 0) { dst[off] = '-'; len = 1; }
    const digits = 20 - pos;
    for (0..digits) |k| dst[off + len + k] = buf[pos + k];
    return len + digits;
}

// ─── top-level dispatcher ────────────────────────────────────────────────────

fn eval_binary(words: []const i32) i32 {
    if (words.len < 2) return obj_new_string(0, 0);
    const sub = obj_ensure_string(words[1]);
    if (sub.len == 0) return obj_new_string(0, 0);
    const sp: [*]const u8 = @ptrFromInt(sub.ptr);

    // ``binary format``
    if (sub.len == 6 and
        sp[0]=='f' and sp[1]=='o' and sp[2]=='r' and
        sp[3]=='m' and sp[4]=='a' and sp[5]=='t')
    {
        // Shift words: ["binary", "format", fmtStr, val...] → ["format", fmtStr, val...]
        return eval_binary_format(words[1..]);
    }
    // ``binary scan``
    if (sub.len == 4 and
        sp[0]=='s' and sp[1]=='c' and sp[2]=='a' and sp[3]=='n')
    {
        return eval_binary_scan(words[1..]);
    }
    // ``binary encode`` / ``binary decode`` — not implemented
    return obj_new_string(0, 0);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "binary", .arity_min = 1, .arity_max = null, .handler = &eval_binary },
};
