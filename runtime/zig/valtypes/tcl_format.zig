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
//   %c        character from int (full Unicode code point, encoded UTF-8)
//   %x %X     hex integer (lower/upper case)
//   %o        octal integer
//   %u        unsigned decimal integer (32-bit wrap)
//   %b        binary integer
//   %f %e %g  float
//   %%        literal percent
//
// Supports ``-`` (left-align), ``0`` (zero-pad), ``+`` (sign),
// `` `` (space-before-positive), ``#`` (alternate form: ``0x`` / ``0X``
// / ``0o`` / ``0b`` / ``0d`` prefixes for hex / octal / binary /
// decimal, and forced trailing ``.`` for ``%#.0f``),
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

/// Validate that *arg* is a numeric integer (or empty / null).
/// Returns the integer value on success, ``null`` after raising
/// ``expected integer but got "<arg>"`` on failure.  Used by
/// format's star-width / star-precision parsing and the ``%d``-family
/// conversions so a non-numeric arg surfaces the reference Tcl 9
/// wording instead of silently formatting as ``0`` (format-8.9 /
/// 8.10 / 8.11 / 8.12 / 8.13 / 8.14).
fn checked_int(arg: i32) ?i64 {
    if (arg == 0) return 0;
    if (obj.is_immediate(arg)) return obj.immediate_unbox(arg);
    const t = obj.obj_type(arg);
    if (t == obj.TYPE_INT or t == obj.TYPE_FLOAT or t == obj.TYPE_BIGNUM) return obj_get_int(arg);
    const s = obj_ensure_string(arg);
    if (obj.try_parse_int(s.ptr, s.len)) |v| return v;
    // Accept hex / octal / binary literals (``0xff`` / ``0o17`` /
    // ``0b101``) and decimals beyond i64 via the bignum parser —
    // format-10.1 / 10.5 feed ``0xffff`` etc. to ``%hd``.
    if (bignum.parse_i128(s.ptr, s.len)) |v128| {
        return @truncate(v128);
    }
    raise_expected_numeric(s.ptr, s.len, false);
    return null;
}

fn checked_float(arg: i32) ?f64 {
    if (arg == 0) return 0.0;
    if (obj.is_immediate(arg)) return @floatFromInt(obj.immediate_unbox(arg));
    const t = obj.obj_type(arg);
    if (t == obj.TYPE_INT or t == obj.TYPE_FLOAT or t == obj.TYPE_BIGNUM) return obj.obj_get_float(arg);
    const s = obj_ensure_string(arg);
    if (obj.try_parse_int(s.ptr, s.len)) |v| return @floatFromInt(v);
    if (obj.try_parse_float(s.ptr, s.len)) |fv| return fv;
    if (bignum.parse_i128(s.ptr, s.len)) |v128| return @floatFromInt(v128);
    raise_expected_numeric(s.ptr, s.len, true);
    return null;
}

/// Emit ``expected integer but got "<bytes>"`` (or
/// ``expected floating-point number but got "<bytes>"`` when
/// *as_float* is true).  Matches the message Tcl 9 raises for
/// scalar-conversion failures inside ``format`` (test format-8.x).
fn raise_expected_numeric(bytes_ptr: u32, bytes_len: u32, as_float: bool) void {
    const int_prefix: []const u8 = "expected integer but got \"";
    const flt_prefix: []const u8 = "expected floating-point number but got \"";
    const suffix: []const u8 = "\"";
    const prefix = if (as_float) flt_prefix else int_prefix;
    const total: u32 = @intCast(prefix.len + bytes_len + suffix.len);
    const buf = alloc(total);
    if (buf == 0) {
        stubs.raise(prefix);
        return;
    }
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    if (bytes_len > 0) {
        const src: [*]const u8 = @ptrFromInt(bytes_ptr);
        var i: u32 = 0;
        while (i < bytes_len) : (i += 1) {
            dst[off] = src[i];
            off += 1;
        }
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const msg = obj.obj_new_string_take(buf, total, total);
    const catch_mod = @import("../interp/tcl_catch.zig");
    catch_mod.tcl_cmd_error(msg);
}

/// ``format fmt ?a1? ?a2? ?a3?``.  Signature pushes three optional
/// args; missing slots come in as ``0`` which we treat as empty
/// string.  Returns a freshly-allocated TclObj with the formatted
/// string.
pub export fn tcl_cmd_format(fmt: i32, a1: i32, a2: i32, a3: i32) i32 {
    const args = [_]i32{ a1, a2, a3 };
    return format_internal(fmt, args[0..]);
}

/// Variadic ``format`` — args supplied as a slice of TclObj handles.
/// Used by ``eval_format`` when the caller has more than 3 arguments.
pub fn tcl_cmd_format_args(fmt: i32, args: []const i32) i32 {
    return format_internal(fmt, args);
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
    //
    // Every ``tcl_cmd_list_index`` returns a fresh +1 owned obj
    // and every immediate ``obj_new_int(i)`` past 2^30 is also
    // heap-allocated.  Previously these were just dropped on the
    // floor — N+1 leaked TclObjs per ``format`` call plus the
    // ``objs_addr`` indexer buffer.  Release each element AFTER
    // ``format_internal`` is done reading it, and free the buffer.
    const objs_size: u32 = n * 4;
    const objs_addr = alloc(objs_size);
    if (objs_addr == 0) return format_internal(fmt, &[_]i32{});
    defer obj.free_sized(objs_addr, objs_size);
    const objs: [*]i32 = @ptrFromInt(objs_addr);
    var i: u32 = 0;
    while (i < n) : (i += 1) {
        const idx_obj = obj.obj_new_int(@intCast(i));
        objs[i] = list_mod.tcl_cmd_list_index(args_list, idx_obj);
        obj.tcl_obj_release(idx_obj);
    }
    const result = format_internal(fmt, objs[0..n]);
    i = 0;
    while (i < n) : (i += 1) obj.tcl_obj_release(objs[i]);
    return result;
}

/// Scan a format string for the maximum literal width / precision
/// any conversion specifies.  Used to pre-size the output buffer so
/// per-conversion emitters can write padding without bounds-checks.
/// ``*`` widths are bounded by a conservative cap (1 MiB); pathologic
/// run-time values are clamped at the same level by the emitters.
fn scan_format_max_width(fp: [*]const u8, len: u32) u32 {
    var max_w: u32 = 0;
    var i: u32 = 0;
    while (i < len) : (i += 1) {
        if (fp[i] != '%') continue;
        i += 1;
        if (i >= len) break;
        if (fp[i] == '%') continue;
        // Skip positional spec ``N$``.
        if (fp[i] >= '1' and fp[i] <= '9') {
            var k: u32 = i;
            while (k < len and fp[k] >= '0' and fp[k] <= '9') : (k += 1) {}
            if (k < len and fp[k] == '$') i = k + 1;
        }
        // Skip flags.
        while (i < len) : (i += 1) {
            switch (fp[i]) {
                '-', '+', '0', ' ', '#' => {},
                else => break,
            }
        }
        // Width (literal or ``*``).
        var w: u32 = 0;
        if (i < len and fp[i] == '*') {
            // Star — bound at 64 KiB.  Run-time values bigger than
            // this would corrupt memory; the emitters cap matching.
            w = 65536;
            i += 1;
        } else {
            while (i < len and fp[i] >= '0' and fp[i] <= '9') : (i += 1) {
                w = w * 10 + (fp[i] - '0');
                if (w > 1 << 20) {
                    w = 1 << 20; // saturate
                }
            }
        }
        // Precision.
        var p: u32 = 0;
        if (i < len and fp[i] == '.') {
            i += 1;
            if (i < len and fp[i] == '*') {
                p = 65536;
                i += 1;
            } else {
                while (i < len and fp[i] >= '0' and fp[i] <= '9') : (i += 1) {
                    p = p * 10 + (fp[i] - '0');
                    if (p > 1 << 20) {
                        p = 1 << 20;
                    }
                }
            }
        }
        const big = if (w > p) w else p;
        if (big > max_w) max_w = big;
    }
    return max_w;
}

fn format_internal(fmt: i32, args: []const i32) i32 {
    if (fmt == 0) return obj_new_string(0, 0);
    const fs = obj_ensure_string(fmt);
    if (fs.len == 0) return obj_new_string(0, 0);
    const fp: [*]const u8 = @ptrFromInt(fs.ptr);

    var arg_idx: u32 = 0;

    // Preallocate a generously-sized buffer.  Per-conversion writers
    // don't bounds-check, so the buffer must be large enough for
    // every possible expansion.  We fold three sources of growth:
    //
    //   * literal text in the format string (1 byte per byte);
    //   * each conversion's potential width / precision (parsed
    //     here so star widths fed via ``*`` show up too);
    //   * each string-typed argument's full bytes.
    //
    // The pre-scan keeps things linear and avoids relying on the
    // emitters to grow / clamp dynamically.
    const max_width = scan_format_max_width(fp, fs.len);
    var bufsize: u32 = fs.len * 8 + 128;
    if (max_width > 0) {
        // Each conversion can emit at most ``max_width`` characters
        // plus a small per-spec prefix (``0x``, sign, ``.`` etc).
        // ``fs.len`` is the conservative upper bound on conversion
        // count; multiply by ``max_width`` for the worst case.
        bufsize += @as(u32, @intCast(max_width)) * fs.len + 64;
    }
    // Add room for each arg's content.  Strings can be much wider
    // than their printed form (UTF-8 multi-byte), and bignum-typed
    // ints may render to thousands of digits.
    for (args) |a| {
        if (a == 0) continue;
        const as = obj_ensure_string(a);
        bufsize += as.len + 64;
    }
    const buf_addr: u32 = alloc(bufsize);
    const out: [*]u8 = @ptrFromInt(buf_addr);
    var off: u32 = 0;

    // Track positional / sequential mode: each spec must use the
    // same shape, otherwise Tcl 9 raises ``cannot mix "%" and "%n$"
    // conversion specifiers`` (test format-11.5 / 11.6 / 11.7).
    var saw_positional = false;
    var saw_sequential = false;
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
        if (i >= fs.len) {
            stubs.raise("format string ended in middle of field specifier");
            return obj_new_string(0, 0);
        }
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
        var space_sign = false;
        var alt_form = false;
        while (i < fs.len) : (i += 1) {
            switch (fp[i]) {
                '-' => left_align = true,
                '+' => show_sign = true,
                '0' => zero_pad = true,
                ' ' => space_sign = true,
                '#' => alt_form = true,
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
            if (arg_idx >= args.len) {
                stubs.raise("not enough arguments for all format specifiers");
                return obj_new_string(0, 0);
            }
            const w_arg = args[arg_idx];
            arg_idx += 1;
            const w_opt = checked_int(w_arg);
            if (w_opt == null) return obj_new_string(0, 0);
            const w = w_opt.?;
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
                if (arg_idx >= args.len) {
                    stubs.raise("not enough arguments for all format specifiers");
                    return obj_new_string(0, 0);
                }
                const p_arg = args[arg_idx];
                arg_idx += 1;
                const p_opt = checked_int(p_arg);
                if (p_opt == null) return obj_new_string(0, 0);
                const p = p_opt.?;
                precision = if (p < 0) 0 else @intCast(p);
            } else {
                precision = 0;
                while (i < fs.len and fp[i] >= '0' and fp[i] <= '9') : (i += 1) {
                    precision = precision * 10 + @as(i32, fp[i] - '0');
                }
            }
        }
        if (i >= fs.len) {
            stubs.raise("format string ended in middle of field specifier");
            return obj_new_string(0, 0);
        }
        // C-style length modifiers (l, ll, h, hh, L, j, z, t, q).
        // Reference Tcl 9 reads all integers as wide so most are no-ops,
        // but ``h`` / ``hh`` truncate to short / signed-char before
        // formatting (test format-10.x).
        var h_mod: u8 = 0;
        while (i < fs.len) : (i += 1) {
            switch (fp[i]) {
                'h' => h_mod += 1,
                'l', 'L', 'j', 'z', 't', 'q' => {},
                else => break,
            }
        }
        if (i >= fs.len) {
            stubs.raise("format string ended in middle of field specifier");
            return obj_new_string(0, 0);
        }
        const conv = fp[i];
        i += 1;

        // Pull the arg: explicit positional if requested, else the
        // next sequential.  Mixing positional and sequential specs in
        // the same format string is an error.
        var pick: u32 = arg_idx;
        if (explicit_arg) |idx| {
            saw_positional = true;
            pick = idx;
        } else {
            saw_sequential = true;
        }
        if (saw_positional and saw_sequential) {
            stubs.raise("cannot mix \"%\" and \"%n$\" conversion specifiers");
            return obj_new_string(0, 0);
        }
        if (pick >= args.len) {
            if (explicit_arg != null) {
                stubs.raise("\"%n$\" argument index out of range");
            } else {
                stubs.raise("not enough arguments for all format specifiers");
            }
            return obj_new_string(0, 0);
        }
        const a = args[pick];
        if (explicit_arg == null) arg_idx += 1;

        // Validate the conversion arg up front so any
        // "expected integer / float" diagnostic surfaces with the
        // arg's original byte form, not a downstream silent zero
        // (format-8.9 / 8.11 / 8.13 / 8.15).  ``%s`` / ``%%`` and the
        // unknown-spec branch don't need numeric coercion, so skip
        // the check for those.
        switch (conv) {
            'd', 'i', 'u', 'x', 'X', 'o', 'b', 'p', 'c' => {
                if (checked_int(a) == null) return obj_new_string(0, 0);
            },
            'f', 'e', 'g', 'E', 'G', 'F' => {
                if (checked_float(a) == null) return obj_new_string(0, 0);
            },
            else => {},
        }

        off = emit_conversion(
            out,
            off,
            bufsize,
            conv,
            a,
            left_align,
            zero_pad,
            show_sign,
            space_sign,
            alt_form,
            width,
            precision,
            h_mod,
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
    space_sign: bool,
    alt_form: bool,
    width: u32,
    precision: i32,
    h_mod: u8,
) u32 {
    switch (conv) {
        'd', 'i' => return emit_int(out, off, cap, arg, left_align, zero_pad, show_sign, space_sign, alt_form, width, precision, 10, false, false, h_mod),
        'u' => return emit_int(out, off, cap, arg, left_align, zero_pad, false, false, alt_form, width, precision, 10, false, true, h_mod),
        'x' => return emit_int(out, off, cap, arg, left_align, zero_pad, false, false, alt_form, width, precision, 16, false, true, h_mod),
        'X' => return emit_int(out, off, cap, arg, left_align, zero_pad, false, false, alt_form, width, precision, 16, true, true, h_mod),
        'o' => return emit_int(out, off, cap, arg, left_align, zero_pad, false, false, alt_form, width, precision, 8, false, true, h_mod),
        'b' => return emit_int(out, off, cap, arg, left_align, zero_pad, false, false, alt_form, width, precision, 2, false, true, h_mod),
        // ``%p`` / ``%zd`` / ``%td`` — pointer / size_t / ptrdiff_t.
        // Tcl 9 renders ``%p`` as ``0x`` + lowercase hex of the wide
        // value; ``%zd`` / ``%td`` are decimal aliases.
        'p' => return emit_int(out, off, cap, arg, left_align, zero_pad, false, false, true, width, precision, 16, false, true, 0),
        's' => return emit_str(out, off, cap, arg, left_align, zero_pad, width, precision),
        'c' => return emit_char(out, off, cap, arg, left_align, zero_pad, width),
        'f', 'e', 'g', 'E', 'G', 'F' => return emit_float(out, off, cap, arg, left_align, zero_pad, show_sign, space_sign, alt_form, width, precision, conv),
        else => {
            // Unknown conversion — raise the Tcl 9 wording.
            raise_bad_specifier(conv);
            return off;
        },
    }
}

fn raise_bad_specifier(conv: u8) void {
    // Reference Tcl 9 wording: ``bad field specifier "X"`` (test
    // format-8.20).  Build the message inline so we can ship the
    // exact byte that triggered the error.
    var buf: [40]u8 = undefined;
    const prefix = "bad field specifier \"";
    var n: usize = 0;
    for (prefix) |c| {
        buf[n] = c;
        n += 1;
    }
    buf[n] = conv;
    n += 1;
    buf[n] = '"';
    n += 1;
    stubs.raise(buf[0..n]);
}

/// Alternate-form prefix for the requested base.  Returns ``"0x"`` /
/// ``"0o"`` / ``"0b"`` / ``"0d"`` per Tcl 9 — Tcl always lower-cases
/// the prefix even for ``%#X`` (``%#X 12`` → ``0xC``: upper-case
/// digits but lower-case prefix).  Empty when the alt-form is a
/// no-op (zero value, or no flag set).
fn alt_prefix(base: u8, alt_form: bool, value_is_zero: bool, _: bool) []const u8 {
    if (!alt_form) return "";
    return switch (base) {
        16 => if (value_is_zero) "" else "0x",
        8 => if (value_is_zero) "" else "0o",
        2 => if (value_is_zero) "" else "0b",
        10 => if (value_is_zero) "" else "0d",
        else => "",
    };
}

fn emit_int(
    out: [*]u8,
    off_in: u32,
    cap: u32,
    arg: i32,
    left_align: bool,
    zero_pad: bool,
    show_sign: bool,
    space_sign: bool,
    alt_form: bool,
    width: u32,
    precision: i32,
    base: u8,
    upper: bool,
    unsigned_conv: bool,
    h_mod: u8,
) u32 {
    var off = off_in;
    // Bignum-aware path: when the operand exceeds i64, render via
    // ``Managed.toString`` in the requested base so ``format "%d"
    // [expr {1<<200}]`` produces the full 61-digit value rather
    // than the truncated low-64-bits view (which renders to ``0``
    // for any large power of two).  ``%h*`` truncates to short
    // first, so a bignum operand passed through ``%hd`` falls
    // back to the i64 path below.
    if (h_mod == 0 and arg != 0 and obj.obj_type(arg) == obj.TYPE_BIGNUM) {
        return emit_int_bignum(out, off_in, cap, arg, left_align, zero_pad, show_sign, space_sign, alt_form, width, precision, base, upper, unsigned_conv);
    }
    var n: i64 = 0;
    if (arg != 0) n = obj_get_int(arg);
    // ``h`` / ``hh`` length modifiers — truncate to short / signed
    // char (sign-extended) before formatting.  ``%hd 0xffff`` →
    // ``-1``, ``%hx 0x10fff`` → ``fff``, ``%hu 0x100000000`` → ``0``.
    if (h_mod == 1) {
        if (unsigned_conv) {
            n = @as(i64, @intCast(@as(u16, @truncate(@as(u64, @bitCast(n))))));
        } else {
            n = @as(i64, @as(i16, @bitCast(@as(u16, @truncate(@as(u64, @bitCast(n)))))));
        }
    } else if (h_mod >= 2) {
        if (unsigned_conv) {
            n = @as(i64, @intCast(@as(u8, @truncate(@as(u64, @bitCast(n))))));
        } else {
            n = @as(i64, @as(i8, @bitCast(@as(u8, @truncate(@as(u64, @bitCast(n)))))));
        }
    }
    var digits: [80]u8 = undefined;
    var dlen: u32 = 0;
    var negative = n < 0;
    // For unsigned conversions (``%u`` / ``%x`` / ``%o`` / ``%b``),
    // Tcl 9 truncates the value to its 32-bit two's-complement form
    // and renders it as an unsigned 32-bit integer — ``format "%x"
    // -12`` → ``fffffff4``, ``format "%u" -12`` → ``4294967284``.
    var u: u64 = undefined;
    if (unsigned_conv) {
        const masked: u32 = @bitCast(@as(i32, @truncate(n)));
        u = @as(u64, masked);
        negative = false;
    } else {
        u = if (negative) @intCast(-n) else @intCast(n);
    }
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
    // Honour an explicit precision on integer conversions: pad with
    // leading zeros up to ``precision`` digits.  ``precision == 0``
    // with value ``0`` yields the empty digit string per C99.
    var min_digits: u32 = dlen;
    var explicit_precision = false;
    if (precision >= 0) {
        explicit_precision = true;
        const p: u32 = @intCast(precision);
        if (p > min_digits) min_digits = p;
        if (precision == 0 and dlen == 1 and digits[0] == '0') {
            dlen = 0;
            min_digits = 0;
        }
    }
    const value_is_zero = dlen == 0 or (dlen == 1 and digits[0] == '0');
    const prefix = alt_prefix(base, alt_form, value_is_zero, upper);
    var sign_ch: u8 = 0;
    if (negative) {
        sign_ch = '-';
    } else if (show_sign) {
        sign_ch = '+';
    } else if (space_sign) {
        sign_ch = ' ';
    }
    const sign_len: u32 = if (sign_ch == 0) 0 else 1;
    const prefix_total: u32 = sign_len + @as(u32, @intCast(prefix.len));
    const total: u32 = prefix_total + min_digits;
    const pad: u32 = if (width > total) width - total else 0;
    // Explicit precision suppresses the ``0`` flag (matches C / Tcl).
    // For integer conversions Tcl 9 honours the ``0`` flag even when
    // ``-`` is also set — see test format-1.15.  The standard C rule
    // (``-`` overrides ``0``) is *not* what Tcl ships with; the
    // formatter prefers zero-pad with no left-align when both flags
    // are present and there's no explicit precision.
    const use_zero_pad = zero_pad and !explicit_precision;
    const effective_left_align = left_align and !use_zero_pad;
    if (!effective_left_align and !use_zero_pad) {
        var k: u32 = 0;
        while (k < pad) : (k += 1) {
            out[off] = ' ';
            off += 1;
        }
    }
    if (sign_ch != 0) {
        out[off] = sign_ch;
        off += 1;
    }
    for (prefix) |pc| {
        out[off] = pc;
        off += 1;
    }
    if (use_zero_pad) {
        var k: u32 = 0;
        while (k < pad) : (k += 1) {
            out[off] = '0';
            off += 1;
        }
    }
    // Precision zero-padding (between sign/prefix and the digits).
    var zp: u32 = 0;
    while (zp + dlen < min_digits) : (zp += 1) {
        out[off] = '0';
        off += 1;
    }
    // digits are stored reversed
    var j: u32 = dlen;
    while (j > 0) {
        j -= 1;
        out[off] = digits[j];
        off += 1;
    }
    if (effective_left_align) {
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
    _: u32,
    arg: i32,
    left_align: bool,
    zero_pad: bool,
    show_sign: bool,
    space_sign: bool,
    alt_form: bool,
    width: u32,
    precision: i32,
    base: u8,
    upper: bool,
    unsigned_conv: bool,
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
    // ``%llu -1`` and friends: an unsigned conversion never accepts a
    // negative bignum.  Match Tcl 9's "unsigned bignum format is
    // invalid" wording.
    if (unsigned_conv and negative) {
        stubs.raise("unsigned bignum format is invalid");
        return off;
    }

    const dlen: u32 = @intCast(digits.len);
    var min_digits: u32 = dlen;
    var explicit_precision = false;
    if (precision >= 0) {
        explicit_precision = true;
        const p: u32 = @intCast(precision);
        if (p > min_digits) min_digits = p;
    }
    const value_is_zero = dlen == 0 or (dlen == 1 and digits[0] == '0');
    const prefix = alt_prefix(base, alt_form, value_is_zero, upper);
    var sign_ch: u8 = 0;
    if (negative) {
        sign_ch = '-';
    } else if (show_sign) {
        sign_ch = '+';
    } else if (space_sign) {
        sign_ch = ' ';
    }
    const sign_len: u32 = if (sign_ch == 0) 0 else 1;
    const total: u32 = sign_len + @as(u32, @intCast(prefix.len)) + min_digits;
    const pad: u32 = if (width > total) width - total else 0;
    // Tcl 9 honours the ``0`` flag even when ``-`` is set (see
    // format-1.15) — same quirk as ``emit_int``.
    const use_zero_pad = zero_pad and !explicit_precision;
    const effective_left_align = left_align and !use_zero_pad;
    if (!effective_left_align and !use_zero_pad) {
        var k: u32 = 0;
        while (k < pad) : (k += 1) {
            out[off] = ' ';
            off += 1;
        }
    }
    if (sign_ch != 0) {
        out[off] = sign_ch;
        off += 1;
    }
    for (prefix) |pc| {
        out[off] = pc;
        off += 1;
    }
    if (use_zero_pad) {
        var k: u32 = 0;
        while (k < pad) : (k += 1) {
            out[off] = '0';
            off += 1;
        }
    }
    var zp: u32 = 0;
    while (zp + dlen < min_digits) : (zp += 1) {
        out[off] = '0';
        off += 1;
    }
    for (digits) |c| {
        out[off] = c;
        off += 1;
    }
    if (effective_left_align) {
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
    zero_pad: bool,
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
        // Width / precision in ``%s`` are character-counts, not byte-
        // counts.  Walk the buffer one UTF-8 sequence at a time so
        // ``%.5s`` against ``abc﻿d`` (4 chars + "FEFF" 3-byte
        // sequence + "d") slices on character boundaries.
        if (precision >= 0) {
            const p: u32 = @intCast(precision);
            slen = utf8_byte_prefix(sptr, slen, p);
        }
    }
    // ``char_count`` is the number of code-points actually written —
    // ``slen`` has been truncated to ``precision`` chars above, so
    // the count of multi-byte starts in the truncated buffer is the
    // right denominator for the width-padding arithmetic.  Using
    // ``precision`` directly produced wrong padding when the string
    // was shorter than the requested precision (test format-15.1
    // ``%05s a``).
    const char_count = utf8_char_count(sptr, slen);
    const pad_count = if (width > char_count) width - char_count else 0;
    const pad_byte: u8 = if (zero_pad and !left_align) '0' else ' ';
    if (!left_align) {
        var k: u32 = 0;
        while (k < pad_count) : (k += 1) {
            out[off] = pad_byte;
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
        while (k < pad_count) : (k += 1) {
            out[off] = ' ';
            off += 1;
        }
    }
    return off;
}

/// Count the number of UTF-8 code points in *len* bytes starting at
/// *ptr*.  Continuation bytes (10xxxxxx) are skipped.  Used for
/// width / precision arithmetic in ``%s``.
fn utf8_char_count(ptr: u32, len: u32) u32 {
    if (len == 0) return 0;
    const sp: [*]const u8 = @ptrFromInt(ptr);
    var n: u32 = 0;
    var i: u32 = 0;
    while (i < len) : (i += 1) {
        if ((sp[i] & 0xC0) != 0x80) n += 1;
    }
    return n;
}

/// Return the byte-offset of the *max_chars*-th UTF-8 code point
/// boundary within ``len`` bytes at *ptr*.  Returns *len* if the
/// buffer holds fewer than *max_chars* code points.
fn utf8_byte_prefix(ptr: u32, len: u32, max_chars: u32) u32 {
    if (max_chars == 0) return 0;
    if (len == 0) return 0;
    const sp: [*]const u8 = @ptrFromInt(ptr);
    var seen: u32 = 0;
    var i: u32 = 0;
    while (i < len) : (i += 1) {
        if ((sp[i] & 0xC0) != 0x80) {
            if (seen == max_chars) return i;
            seen += 1;
        }
    }
    return len;
}

/// ``%c`` — encode a Unicode code-point as UTF-8 (Tcl 9 emits four-
/// byte sequences for the supplementary planes and uses the lone
/// surrogate range too).  Accepts negative / out-of-range values by
/// truncating to the low 21 bits and falling back to U+FFFD when the
/// resulting value is non-Unicode.
fn emit_char(
    out: [*]u8,
    off_in: u32,
    _: u32,
    arg: i32,
    left_align: bool,
    zero_pad: bool,
    width: u32,
) u32 {
    var off = off_in;
    var n: i64 = 0;
    if (arg != 0) n = obj_get_int(arg);
    // Tcl 9 truncates to the low 21 bits before encoding, so values
    // beyond U+10FFFF map back into the BMP / supplementary range.
    const masked: u32 = @as(u32, @bitCast(@as(i32, @truncate(n)))) & 0x1FFFFF;
    var cp: u32 = masked;
    // Code points above U+10FFFF round to U+FFFD (the replacement
    // character) — matches Tcl's TCL_COMBINE handling.
    if (cp > 0x10FFFF) cp = 0xFFFD;
    var seq: [4]u8 = undefined;
    var seq_len: u32 = 0;
    if (cp < 0x80) {
        seq[0] = @intCast(cp);
        seq_len = 1;
    } else if (cp < 0x800) {
        seq[0] = @intCast(0xC0 | (cp >> 6));
        seq[1] = @intCast(0x80 | (cp & 0x3F));
        seq_len = 2;
    } else if (cp < 0x10000) {
        seq[0] = @intCast(0xE0 | (cp >> 12));
        seq[1] = @intCast(0x80 | ((cp >> 6) & 0x3F));
        seq[2] = @intCast(0x80 | (cp & 0x3F));
        seq_len = 3;
    } else {
        seq[0] = @intCast(0xF0 | (cp >> 18));
        seq[1] = @intCast(0x80 | ((cp >> 12) & 0x3F));
        seq[2] = @intCast(0x80 | ((cp >> 6) & 0x3F));
        seq[3] = @intCast(0x80 | (cp & 0x3F));
        seq_len = 4;
    }
    // ``%c`` width counts characters — exactly one code point.
    const pad: u32 = if (width > 1) width - 1 else 0;
    const pad_byte: u8 = if (zero_pad and !left_align) '0' else ' ';
    if (!left_align) {
        var k: u32 = 0;
        while (k < pad) : (k += 1) {
            out[off] = pad_byte;
            off += 1;
        }
    }
    var k: u32 = 0;
    while (k < seq_len) : (k += 1) {
        out[off] = seq[k];
        off += 1;
    }
    if (left_align) {
        var p: u32 = 0;
        while (p < pad) : (p += 1) {
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
    left_align: bool,
    zero_pad: bool,
    show_sign: bool,
    space_sign: bool,
    alt_form: bool,
    width: u32,
    precision: i32,
    conv: u8,
) u32 {
    var off = off_in;
    if (arg == 0) return off;
    const fval = obj.obj_get_float(arg);
    const prec: usize = if (precision >= 0) @intCast(precision) else 6;
    // Buffer needs slack above the longest reasonable f64 render
    // (``-1.7e308`` plus 17 fractional digits is ~25 bytes; the
    // ``%.30f`` worst case caps prec at 17 inside fmt_float_decimal).
    // 192 bytes leaves room for two follow-up mutations: the
    // sign-prepend (1 byte) and the alt-form ``.``/``e`` insert
    // (1 byte) without overflowing the stack array.
    var tmp_buf: [192]u8 = undefined;
    const cap: u32 = tmp_buf.len;
    var slen: u32 = switch (conv) {
        'e', 'E' => fmt_float_e(&tmp_buf, fval, prec, conv == 'E'),
        'g', 'G' => fmt_float_g(&tmp_buf, fval, prec, conv == 'G'),
        else => @intCast(fmt_float_decimal(&tmp_buf, fval, prec)),
    };
    if (slen == 0) return off;
    // Sign handling — Zig's stdlib already emits ``-`` for negatives.
    // Replace ``+``/`` `` requests by inserting at offset 0 when the
    // mantissa starts with a digit.  Bail out if the buffer is full.
    const negative = tmp_buf[0] == '-';
    if (!negative and (show_sign or space_sign) and slen + 1 < cap) {
        // shift right by 1 and prepend
        var k: u32 = slen;
        while (k > 0) : (k -= 1) tmp_buf[k] = tmp_buf[k - 1];
        tmp_buf[0] = if (show_sign) '+' else ' ';
        slen += 1;
    }
    // ``#`` flag for ``%f`` / ``%e`` / ``%g``: force a decimal point
    // even when the precision elides one.  For ``%g`` it also disables
    // trailing-zero trimming — currently fmt_float_g always trims, so
    // for ``%#g`` we re-render as ``%f`` / ``%e`` with explicit prec.
    if (alt_form and (conv == 'f' or conv == 'F') and !mantissa_has_dot(tmp_buf[0..slen]) and slen + 1 < cap) {
        tmp_buf[slen] = '.';
        slen += 1;
    }
    if (alt_form and (conv == 'e' or conv == 'E') and slen + 1 < cap) {
        // Insert ``.`` before the ``e`` if absent.
        var e_idx: u32 = 0;
        while (e_idx < slen and tmp_buf[e_idx] != 'e' and tmp_buf[e_idx] != 'E') : (e_idx += 1) {}
        if (e_idx < slen) {
            // Check for an existing ``.`` before the exponent.
            var has_dot = false;
            var k: u32 = 0;
            while (k < e_idx) : (k += 1) {
                if (tmp_buf[k] == '.') {
                    has_dot = true;
                    break;
                }
            }
            if (!has_dot) {
                // Shift exponent right by one and insert ``.``.
                var src: u32 = slen;
                while (src > e_idx) : (src -= 1) tmp_buf[src] = tmp_buf[src - 1];
                tmp_buf[e_idx] = '.';
                slen += 1;
            }
        }
    }
    const pad: u32 = if (width > slen) width - slen else 0;
    const use_zero_pad = zero_pad and !left_align;
    if (!left_align and !use_zero_pad) {
        var k: u32 = 0;
        while (k < pad) : (k += 1) {
            out[off] = ' ';
            off += 1;
        }
    }
    if (use_zero_pad) {
        // Sign / sign-space stays leftmost; zero-pad sits between
        // sign and the leading mantissa digit.
        var src: u32 = 0;
        if (slen > 0 and (tmp_buf[0] == '-' or tmp_buf[0] == '+' or tmp_buf[0] == ' ')) {
            out[off] = tmp_buf[0];
            off += 1;
            src = 1;
        }
        var k: u32 = 0;
        while (k < pad) : (k += 1) {
            out[off] = '0';
            off += 1;
        }
        while (src < slen) : (src += 1) {
            out[off] = tmp_buf[src];
            off += 1;
        }
    } else {
        var k: u32 = 0;
        while (k < slen) : (k += 1) {
            out[off] = tmp_buf[k];
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

fn mantissa_has_dot(s: []const u8) bool {
    for (s) |c| {
        if (c == '.') return true;
        if (c == 'e' or c == 'E') return false;
    }
    return false;
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
