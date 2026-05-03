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
const bignum = @import("tcl_bignum.zig");
const stubs = @import("../stubs/tcl_stubs.zig");

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
    const args = [_]i32{ a1, a2, a3 };
    return format_internal(fmt, args[0..]);
}

/// Variadic ``format`` — args supplied as a Tcl list TclObj.  Used
/// by callers that exceed the three-slot ``tcl_cmd_format``
/// signature, e.g. ``[format "%-*s %-*s %-*s %s" $nl $name $tl $type
/// $dl $dv $help]`` in opt.test's ``OptTree``.  Each list element is
/// pulled in turn as the next sequential argument.
pub export fn tcl_cmd_format_list(fmt: i32, args_list: i32) i32 {
    const list_mod = @import("tcl_list.zig");
    const list_parse = @import("tcl_list_parse.zig");
    if (args_list == 0) return format_internal(fmt, &[_]i32{});
    const ls = obj_ensure_string(args_list);
    const n_signed = list_parse.count_elements(ls.ptr, ls.len);
    if (n_signed <= 0) return format_internal(fmt, &[_]i32{});
    const n: u32 = @intCast(n_signed);
    // Materialise each element as a fresh TclObj via
    // ``tcl_cmd_list_index`` (handles braced + quoted elements
    // and backslash decoding) so the format routine can pull
    // integer widths and string values uniformly.
    const objs_addr = alloc(n * 4);
    const objs: [*]i32 = @ptrFromInt(objs_addr);
    var i: u32 = 0;
    while (i < n) : (i += 1) {
        objs[i] = list_mod.tcl_cmd_list_index(args_list, obj.obj_new_int(@intCast(i)));
    }
    return format_internal(fmt, objs[0..n]);
}

fn format_internal(fmt: i32, args: []const i32) i32 {
    if (fmt == 0) return obj_new_string(0, 0);
    const fs = obj_ensure_string(fmt);
    if (fs.len == 0) return obj_new_string(0, 0);
    const fp: [*]const u8 = @ptrFromInt(fs.ptr);

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
        // Optional positional spec: ``%N$...`` selects the Nth (1-indexed)
        // argument instead of the next sequential one.  ``N`` is parsed
        // greedily as decimal digits; the trailing ``$`` confirms the
        // form (without it we restart and treat the digits as a width).
        var explicit_arg: ?u32 = null;
        if (i < fs.len and fp[i] >= '1' and fp[i] <= '9') {
            // Look ahead for a trailing ``$``.  If absent, this is a
            // width spec — back-track and let the width parser below
            // pick it up.
            var k: u32 = i;
            var idx: u32 = 0;
            while (k < fs.len and fp[k] >= '0' and fp[k] <= '9') : (k += 1) {
                idx = idx * 10 + @as(u32, fp[k] - '0');
            }
            if (k < fs.len and fp[k] == '$' and idx >= 1) {
                explicit_arg = idx - 1;
                i = k + 1; // step past ``$``
            }
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
        if (i < fs.len and fp[i] == '*') {
            // ``%-*s`` / ``%*s`` — consume the next arg as the width.
            // Matches reference Tcl's ``Tcl_FormatObjCmd`` star-width
            // semantics; opt.test's ``OptTree`` (``format "%-*s"``)
            // depends on this for column alignment.
            i += 1;
            const w_arg = if (arg_idx < args.len) args[arg_idx] else 0;
            arg_idx += 1;
            const w = obj_get_int(w_arg);
            // Negative widths are treated as left-aligned + abs(width).
            if (w < 0) {
                left_align = true;
                width = @intCast(-w);
            } else {
                width = @intCast(w);
            }
        } else {
            while (i < fs.len and fp[i] >= '0' and fp[i] <= '9') : (i += 1) {
                width = width * 10 + @as(u32, fp[i] - '0');
            }
        }
        var precision: i32 = -1;
        if (i < fs.len and fp[i] == '.') {
            i += 1;
            if (i < fs.len and fp[i] == '*') {
                // ``%.*s`` — dynamic precision from the next arg.
                i += 1;
                const p_arg = if (arg_idx < args.len) args[arg_idx] else 0;
                arg_idx += 1;
                const p = obj_get_int(p_arg);
                precision = if (p < 0) 0 else @intCast(p);
            } else {
                precision = 0;
                while (i < fs.len and fp[i] >= '0' and fp[i] <= '9') : (i += 1) {
                    precision = precision * 10 + @as(i32, fp[i] - '0');
                }
            }
        }
        if (i >= fs.len) break;
        // Skip C-style length modifiers (l, ll, h, hh, L, j, z, t, q).
        // Reference Tcl reads all integers as i64, so modifiers are no-ops.
        while (i < fs.len) : (i += 1) {
            switch (fp[i]) {
                'l', 'h', 'L', 'j', 'z', 't', 'q' => {},
                else => break,
            }
        }
        if (i >= fs.len) break;
        const conv = fp[i];
        i += 1;

        // Pull the arg: explicit positional if requested, else the next
        // sequential.  Out-of-range positionals fall through to 0 (empty
        // string / zero), matching the fallthrough we already do for
        // missing sequential args.
        var pick: u32 = arg_idx;
        if (explicit_arg) |idx| pick = idx;
        const a = if (pick < args.len) args[pick] else 0;
        if (explicit_arg == null) arg_idx += 1;

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

    // Issue #317: claim ownership of the format buffer so the
    // resulting TclObj's release frees it via ``free_sized``.
    // The older borrowing form leaked one buf per ``format`` call;
    // tcltest's progress / diagnostic output exercises this path
    // heavily.
    return obj.obj_new_string_take(buf_addr, off, bufsize);
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
    // Bignum-aware path: when the operand exceeds i64, render via
    // ``Managed.toString`` in the requested base so ``format "%d"
    // [expr {1<<200}]`` produces the full 61-digit value rather
    // than the truncated low-64-bits view (which renders to ``0``
    // for any large power of two).
    if (arg != 0 and obj.obj_type(arg) == obj.TYPE_BIGNUM) {
        return emit_int_bignum(out, off_in, arg, left_align, zero_pad, show_sign, width, base, upper);
    }
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

/// Render a TYPE_BIGNUM operand into the format buffer using
/// ``Managed.toString`` for the requested base (10 / 16 / 8 / 2).
/// Mirrors :func:`emit_int`'s padding logic but works on the
/// arbitrary-length digit string returned by the bignum formatter.
fn emit_int_bignum(
    out: [*]u8,
    off_in: u32,
    arg: i32,
    left_align: bool,
    zero_pad: bool,
    show_sign: bool,
    width: u32,
    base: u8,
    upper: bool,
) u32 {
    var off = off_in;
    const m = obj.obj_get_bignum_managed(arg) orelse return off;
    const case: std.fmt.Case = if (upper) .upper else .lower;
    // Route through ``alloc_format_base`` so the base-8 limb-
    // boundary bug in Zig 0.16's ``Managed.toString`` (extracts
    // bits per-limb without crossing boundaries — wrong for
    // 3-bit digits on 32-bit limbs) gets the bit-walking
    // workaround.  Bases 10 / 16 / 2 still go through stdlib.
    const rendered = bignum.alloc_format_base(m, base, case) orelse return off;
    defer bignum.allocator.free(rendered);

    // ``Managed.toString`` emits a leading ``-`` for negative values.
    // Strip it from the digit slice so padding / sign-handling can
    // run uniformly with the i64 path above.
    var digits = rendered;
    var negative = false;
    if (digits.len > 0 and digits[0] == '-') {
        negative = true;
        digits = digits[1..];
    }

    var prefix_len: u32 = 0;
    if (negative) prefix_len = 1 else if (show_sign) prefix_len = 1;
    const dlen: u32 = @intCast(digits.len);
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
    for (digits) |c| {
        out[off] = c;
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
        'e', 'E' => fmt_float_e(&tmp_buf, fval, prec, conv == 'E'),
        'g', 'G' => fmt_float_g(&tmp_buf, fval, prec, conv == 'G'),
        else => @intCast(fmt_float_decimal(&tmp_buf, fval, prec)),
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

/// ``%e`` / ``%E`` — scientific notation with *prec* digits after
/// the decimal point.  Uses Zig's stdlib float-to-decimal printer
/// in the matching format spec.  Returns the byte length written.
fn fmt_float_e(out: []u8, fval: f64, prec: usize, upper: bool) u32 {
    const slice = std.fmt.bufPrint(out, "{e:.[1]}", .{ fval, prec }) catch return 0;
    if (upper) {
        for (slice) |*ch| {
            if (ch.* == 'e') ch.* = 'E';
        }
    }
    return @intCast(slice.len);
}

/// ``%g`` / ``%G`` — shortest representation: scientific when the
/// exponent is ``< -4`` or ``>= prec``, else fixed-point.  Trailing
/// zeros after the decimal point are trimmed; an isolated trailing
/// ``.`` is dropped.  Mirrors C's ``printf("%.<prec>g", ...)``
/// (which is what Tcl 9 uses for ``format %.6g``).
///
/// For ``prec == 0`` we substitute 1 — C's ``%g`` treats prec=0 as
/// "1 significant digit" rather than "no digits".
fn fmt_float_g(out: []u8, fval: f64, prec_in: usize, upper: bool) u32 {
    const prec: usize = if (prec_in == 0) 1 else prec_in;
    if (std.math.isNan(fval)) {
        const s = std.fmt.bufPrint(out, "{d}", .{fval}) catch return 0;
        return @intCast(s.len);
    }
    if (fval == 0.0) {
        out[0] = '0';
        return 1;
    }
    // Determine the decimal exponent.  ``floor(log10(|fval|))``.
    const abs_v = @abs(fval);
    const exp10: i32 = @intFromFloat(@floor(std.math.log10(abs_v)));
    // ``%g`` chooses scientific when exp < -4 or exp >= prec.
    const use_sci = exp10 < -4 or exp10 >= @as(i32, @intCast(prec));
    var len: u32 = 0;
    if (use_sci) {
        // Scientific with (prec - 1) digits after the decimal.
        const e_prec: usize = if (prec > 0) prec - 1 else 0;
        const s = std.fmt.bufPrint(out, "{e:.[1]}", .{ fval, e_prec }) catch return 0;
        len = @intCast(s.len);
    } else {
        // Fixed-point with (prec - 1 - exp10) digits after the
        // decimal point — choosing total significant digits = prec.
        const after: i32 = @as(i32, @intCast(prec)) - 1 - exp10;
        const after_u: usize = if (after < 0) 0 else @intCast(after);
        const s = std.fmt.bufPrint(out, "{d:.[1]}", .{ fval, after_u }) catch return 0;
        len = @intCast(s.len);
    }
    // Trim trailing zeros / lonely ``.`` from the mantissa portion.
    // For scientific output the mantissa is everything before ``e``;
    // for fixed-point it's the whole string.
    const e_idx: ?u32 = blk: {
        var i: u32 = 0;
        while (i < len) : (i += 1) {
            if (out[i] == 'e' or out[i] == 'E') break :blk i;
        }
        break :blk null;
    };
    const mantissa_end: u32 = if (e_idx) |e| e else len;
    // Only trim if there's a ``.`` in the mantissa.
    var has_dot = false;
    var i: u32 = 0;
    while (i < mantissa_end) : (i += 1) {
        if (out[i] == '.') {
            has_dot = true;
            break;
        }
    }
    if (has_dot) {
        var trim_end = mantissa_end;
        while (trim_end > 0 and out[trim_end - 1] == '0') trim_end -= 1;
        if (trim_end > 0 and out[trim_end - 1] == '.') trim_end -= 1;
        // Slide the exponent suffix (if any) up to the trimmed end.
        if (e_idx) |e| {
            var j: u32 = 0;
            while (e + j < len) : (j += 1) {
                out[trim_end + j] = out[e + j];
            }
            len = trim_end + (len - e);
        } else {
            len = trim_end;
        }
    }
    if (upper) {
        var k: u32 = 0;
        while (k < len) : (k += 1) {
            if (out[k] == 'e') out[k] = 'E';
        }
    }
    return len;
}
