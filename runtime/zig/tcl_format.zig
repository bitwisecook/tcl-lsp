// Minimal ``format`` implementation.
//
// Real Tcl's ``format`` supports a sprintf-flavoured spec
// (%d, %s, %f, %e, %g, %c, %x, %o, %b with width / precision /
// flags).  Scripts overwhelmingly use the simple single-conversion
// forms — ``format %d $n``, ``format %s $x``, ``format %3.1f $v``,
// ``format "(%d,%d)" $x $y`` — and tcltest specifically uses
// ``format %3.1f`` / ``format %13.10f`` / ``format %g`` for its
// arithmetic tests, plus ``%02X`` / ``%08X`` / ``%04X`` in its
// debug-dump paths.
//
// This module handles:
//   %d        decimal integer
//   %s        string
//   %c        character from int (0..127)
//   %x %X     hex integer (lower/upper case)
//   %o        octal integer
//   %f %e %g  float — STRING-PASSTHROUGH ONLY.  The runtime has no
//             float TclObj type, so the arg's original text is
//             copied verbatim with width padding; precision and
//             conversion variant are IGNORED.  Sufficient for
//             ``format %3.1f 5.1`` where the input is already in
//             the desired form, but NOT for real rounding / %g
//             trailing-zero trimming / scientific conversion.
//   %%        literal percent
//
// Supports ``-`` (left-align), ``0`` (zero-pad), ``+`` (sign),
// width and precision for integer + string formats.  The
// implementation takes ``(fmt, arg1_obj, arg2_obj, arg3_obj)`` —
// up to three args — matching the codegen's dispatch.  Extra
// conversions than we have args for pull empty strings.

const std = @import("std");
const obj = @import("tcl_obj.zig");
const stubs = @import("tcl_stubs.zig");

const obj_ensure_string = obj.obj_ensure_string;
const obj_new_string = obj.obj_new_string;
const obj_new_string_copy = obj.obj_new_string_copy;
const obj_get_int = obj.obj_get_int;
const alloc = obj.alloc;

/// ``format fmt ?a1? ?a2? ?a3?``.  Signature pushes three optional
/// args; missing slots come in as ``0`` which we treat as empty
/// string.  Returns a freshly-allocated TclObj with the formatted
/// string.
pub export fn tcl_cmd_format(fmt: i32, a1: i32, a2: i32, a3: i32) i32 {
    if (fmt == 0) return obj_new_string(0, 0);
    const fs = obj_ensure_string(fmt);
    if (fs.len == 0) return obj_new_string(0, 0);
    const fp: [*]const u8 = @ptrFromInt(fs.ptr);

    const args = [_]i32{ a1, a2, a3 };
    var arg_idx: u32 = 0;

    // Preallocate a generously-sized buffer.  4× the format string
    // + 64 bytes slack is enough for plain %s / %d / %f spans and
    // a reasonable number of integer conversions.
    var bufsize: u32 = fs.len * 8 + 128;
    // Add room for each string arg's content.
    for (args) |a| {
        if (a == 0) continue;
        const as = obj_ensure_string(a);
        bufsize += as.len + 32;
    }
    const buf_addr: u32 = alloc(bufsize);
    const out: [*]u8 = @ptrFromInt(buf_addr);
    var off: u32 = 0;

    var i: u32 = 0;
    while (i < fs.len) {
        const c = fp[i];
        if (c != '%') {
            out[off] = c;
            off += 1;
            i += 1;
            continue;
        }
        i += 1;
        if (i >= fs.len) break;
        if (fp[i] == '%') {
            out[off] = '%';
            off += 1;
            i += 1;
            continue;
        }
        // Parse flags, width, precision.
        var left_align = false;
        var zero_pad = false;
        var show_sign = false;
        while (i < fs.len) : (i += 1) {
            switch (fp[i]) {
                '-' => left_align = true,
                '+' => show_sign = true,
                '0' => zero_pad = true,
                ' ' => {},
                '#' => {},
                else => break,
            }
        }
        var width: u32 = 0;
        while (i < fs.len and fp[i] >= '0' and fp[i] <= '9') : (i += 1) {
            width = width * 10 + @as(u32, fp[i] - '0');
        }
        var precision: i32 = -1;
        if (i < fs.len and fp[i] == '.') {
            i += 1;
            precision = 0;
            while (i < fs.len and fp[i] >= '0' and fp[i] <= '9') : (i += 1) {
                precision = precision * 10 + @as(i32, fp[i] - '0');
            }
        }
        if (i >= fs.len) break;
        const conv = fp[i];
        i += 1;

        // Pull the next arg.
        const a = if (arg_idx < args.len) args[arg_idx] else 0;
        arg_idx += 1;

        off = emit_conversion(
            out,
            off,
            bufsize,
            conv,
            a,
            left_align,
            zero_pad,
            show_sign,
            width,
            precision,
        );
    }

    return obj_new_string(@intCast(buf_addr), @intCast(off));
}

fn emit_conversion(
    out: [*]u8,
    off: u32,
    cap: u32,
    conv: u8,
    arg: i32,
    left_align: bool,
    zero_pad: bool,
    show_sign: bool,
    width: u32,
    precision: i32,
) u32 {
    switch (conv) {
        'd', 'i' => return emit_int(out, off, cap, arg, left_align, zero_pad, show_sign, width, 10, false),
        'x' => return emit_int(out, off, cap, arg, left_align, zero_pad, false, width, 16, false),
        'X' => return emit_int(out, off, cap, arg, left_align, zero_pad, false, width, 16, true),
        'o' => return emit_int(out, off, cap, arg, left_align, zero_pad, false, width, 8, false),
        's' => return emit_str(out, off, cap, arg, left_align, width, precision),
        'c' => {
            if (arg == 0) return off;
            const n = obj_get_int(arg);
            if (n < 0 or n > 127) return off; // ASCII-only for now
            out[off] = @intCast(n);
            return off + 1;
        },
        'f', 'e', 'g', 'E', 'G' => return emit_float(out, off, cap, arg, width, precision, conv),
        else => {
            // Unknown conversion — raise so the caller sees a clear
            // error instead of silently dropping data.
            var name_buf: [3]u8 = .{ '%', 0, 0 };
            name_buf[1] = conv;
            stubs.unsupported_sub("format", name_buf[0..2]);
            return off;
        },
    }
}

fn emit_int(
    out: [*]u8,
    off_in: u32,
    _: u32,
    arg: i32,
    left_align: bool,
    zero_pad: bool,
    show_sign: bool,
    width: u32,
    base: u8,
    upper: bool,
) u32 {
    var off = off_in;
    var n: i64 = 0;
    if (arg != 0) n = obj_get_int(arg);
    var digits: [32]u8 = undefined;
    var dlen: u32 = 0;
    const negative = n < 0;
    var u: u64 = if (negative) @intCast(-n) else @intCast(n);
    if (u == 0) {
        digits[dlen] = '0';
        dlen += 1;
    } else {
        while (u > 0) : (u /= base) {
            const d: u8 = @intCast(u % base);
            if (d < 10) {
                digits[dlen] = d + '0';
            } else {
                digits[dlen] = d - 10 + (if (upper) @as(u8, 'A') else @as(u8, 'a'));
            }
            dlen += 1;
        }
    }
    var prefix_len: u32 = 0;
    if (negative) prefix_len = 1 else if (show_sign) prefix_len = 1;
    const total: u32 = prefix_len + dlen;
    const pad: u32 = if (width > total) width - total else 0;
    if (!left_align and !zero_pad) {
        var k: u32 = 0;
        while (k < pad) : (k += 1) {
            out[off] = ' ';
            off += 1;
        }
    }
    if (negative) {
        out[off] = '-';
        off += 1;
    } else if (show_sign) {
        out[off] = '+';
        off += 1;
    }
    if (!left_align and zero_pad) {
        var k: u32 = 0;
        while (k < pad) : (k += 1) {
            out[off] = '0';
            off += 1;
        }
    }
    // digits are stored reversed
    var j: u32 = dlen;
    while (j > 0) {
        j -= 1;
        out[off] = digits[j];
        off += 1;
    }
    if (left_align) {
        var k: u32 = 0;
        while (k < pad) : (k += 1) {
            out[off] = ' ';
            off += 1;
        }
    }
    return off;
}

fn emit_str(
    out: [*]u8,
    off_in: u32,
    _: u32,
    arg: i32,
    left_align: bool,
    width: u32,
    precision: i32,
) u32 {
    var off = off_in;
    var slen: u32 = 0;
    var sptr: u32 = 0;
    if (arg != 0) {
        const s = obj_ensure_string(arg);
        slen = s.len;
        sptr = s.ptr;
        if (precision >= 0 and @as(u32, @intCast(precision)) < slen) {
            slen = @intCast(precision);
        }
    }
    const pad: u32 = if (width > slen) width - slen else 0;
    if (!left_align) {
        var k: u32 = 0;
        while (k < pad) : (k += 1) {
            out[off] = ' ';
            off += 1;
        }
    }
    if (slen > 0) {
        const sp: [*]const u8 = @ptrFromInt(sptr);
        var k: u32 = 0;
        while (k < slen) : (k += 1) {
            out[off] = sp[k];
            off += 1;
        }
    }
    if (left_align) {
        var k: u32 = 0;
        while (k < pad) : (k += 1) {
            out[off] = ' ';
            off += 1;
        }
    }
    return off;
}

/// Format a float value into buf with the given decimal precision.
/// Returns number of bytes written.
fn fmt_float_decimal(buf: []u8, value: f64, precision: usize) usize {
    // Cap precision at 17 — the maximum number of decimal digits a
    // 64-bit IEEE 754 double can represent distinctly.  This is also
    // below the fractional ``fbuf: [20]u8`` capacity, so the loop
    // below can't overflow.  User-supplied ``format %.30f 1.2`` falls
    // back to 17 fractional digits rather than panicking on the
    // out-of-bounds write that ``[20]u8`` would otherwise permit.
    const cap_precision: usize = if (precision > 17) 17 else precision;
    // Handle sign.
    var off: usize = 0;
    var v = value;
    if (v < 0.0) {
        buf[off] = '-';
        off += 1;
        v = -v;
    }
    // Scale to extract integer and fractional parts at the desired precision.
    // Use a power-of-10 multiplier.
    var scale: f64 = 1.0;
    for (0..cap_precision) |_| scale *= 10.0;
    const rounded = @round(v * scale);
    const int_part: u64 = @intFromFloat(@trunc(rounded / scale));
    const frac_raw: u64 = @intFromFloat(@round(rounded - @as(f64, @floatFromInt(int_part)) * scale));
    // Write integer part.
    var ibuf: [20]u8 = undefined;
    var ilen: usize = 0;
    var tmp = int_part;
    if (tmp == 0) {
        ibuf[0] = '0';
        ilen = 1;
    } else {
        while (tmp > 0) {
            ibuf[ilen] = @as(u8, @intCast(tmp % 10)) + '0';
            tmp /= 10;
            ilen += 1;
        }
        // Reverse.
        var l: usize = 0;
        var r: usize = ilen - 1;
        while (l < r) {
            const c = ibuf[l];
            ibuf[l] = ibuf[r];
            ibuf[r] = c;
            l += 1;
            r -= 1;
        }
    }
    for (0..ilen) |i| {
        if (off >= buf.len) return off;
        buf[off] = ibuf[i];
        off += 1;
    }
    if (cap_precision == 0) return off;
    // Decimal point.
    if (off >= buf.len) return off;
    buf[off] = '.';
    off += 1;
    // Write fractional digits (zero-padded on the left to `cap_precision`
    // digits).  ``cap_precision`` is guaranteed ≤ 17 (see above) so this
    // fits in the 20-byte fbuf.
    var fbuf: [20]u8 = undefined;
    var flen: usize = 0;
    var ftmp = frac_raw;
    for (0..cap_precision) |_| {
        fbuf[flen] = @as(u8, @intCast(ftmp % 10)) + '0';
        ftmp /= 10;
        flen += 1;
    }
    // fbuf is digits in reverse (LSB first), so reverse and write.
    var fi: usize = flen;
    while (fi > 0) {
        fi -= 1;
        if (off >= buf.len) return off;
        buf[off] = fbuf[fi];
        off += 1;
    }
    return off;
}

fn emit_float(
    out: [*]u8,
    off_in: u32,
    _: u32,
    arg: i32,
    width: u32,
    precision: i32,
    conv: u8,
) u32 {
    var off = off_in;
    if (arg == 0) return off;
    const fval = obj.obj_get_float(arg);
    const prec: usize = if (precision >= 0) @intCast(precision) else 6;
    var tmp_buf: [64]u8 = undefined;
    const slen: u32 = switch (conv) {
        'e', 'E' => blk: {
            // Scientific notation: use std.fmt with fixed precision inline.
            // For now fall back to decimal; tcltest tests don't use %e.
            const n = fmt_float_decimal(&tmp_buf, fval, prec);
            break :blk @as(u32, @intCast(n));
        },
        'g', 'G' => blk: {
            // %g: shortest representation. Use std.fmt default.
            const s = std.fmt.bufPrint(&tmp_buf, "{d}", .{fval}) catch tmp_buf[0..1];
            break :blk @as(u32, @intCast(s.len));
        },
        else => blk: {
            const n = fmt_float_decimal(&tmp_buf, fval, prec);
            break :blk @as(u32, @intCast(n));
        },
    };
    const pad: u32 = if (width > slen) width - slen else 0;
    var k: u32 = 0;
    while (k < pad) : (k += 1) {
        out[off] = ' ';
        off += 1;
    }
    k = 0;
    while (k < slen) : (k += 1) {
        out[off] = tmp_buf[k];
        off += 1;
    }
    return off;
}
