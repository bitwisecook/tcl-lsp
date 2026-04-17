// Formatting + pattern-matching stubs.  These cover the Tcl 8.4–9.0
// commands that deal with string-to-bytes conversion, regular
// expressions, and character-set recoding.  All raise
// ``unsupported command: <name>`` via :func:`tcl_stubs.unsupported`.
//
// Coverage:
//   - format, scan, binary (format / scan / encode / decode)
//   - regexp, regsub
//   - encoding (multiplexed — the subcommand variance is in the
//     sidecar map's args slot)

const stubs = @import("tcl_stubs.zig");
const obj = @import("tcl_obj.zig");

const obj_ensure_string = obj.obj_ensure_string;
const obj_new_int = obj.obj_new_int;
const obj_get_int = obj.obj_get_int;

// ``format`` moved to tcl_format.zig — minimal %d / %s / %c /
// %x / %o / %f / %e / %g with width + precision support.

/// ``scan str fmt`` — parse *str* according to *fmt*, return the
/// matched value (no varname form; the caller uses the return value).
///
/// Supported format specifiers:
///   %c   — return Unicode codepoint of the first character in *str*
///   %d   — parse a decimal integer from *str*
///   %x   — parse a hexadecimal integer from *str*
///   %o   — parse an octal integer from *str*
///   %s   — return the first whitespace-delimited word from *str*
///
/// Anything else is unsupported and will trap.
pub export fn tcl_cmd_scan(str: i32, fmt: i32) i32 {
    const ss = obj_ensure_string(str);
    const fs = obj_ensure_string(fmt);
    if (fs.len == 0) return obj_new_int(0);

    const fp: [*]const u8 = @ptrFromInt(fs.ptr);

    // Find the first conversion specifier — skip literal chars and '%'.
    var fi: u32 = 0;
    while (fi < fs.len and fp[fi] != '%') fi += 1;
    if (fi >= fs.len) return obj_new_int(0);
    fi += 1; // skip '%'
    if (fi >= fs.len) return obj_new_int(0);

    // Skip optional width digits (we don't use them for %c/%d/etc.).
    while (fi < fs.len and fp[fi] >= '0' and fp[fi] <= '9') fi += 1;
    if (fi >= fs.len) return obj_new_int(0);

    const spec = fp[fi];

    if (spec == 'c') {
        // Return the Unicode codepoint of the first character.
        if (ss.len == 0) return obj_new_int(0);
        const sp: [*]const u8 = @ptrFromInt(ss.ptr);
        const b0 = sp[0];
        var cp: i64 = 0;
        if (b0 < 0x80) {
            cp = @as(i64, b0);
        } else if ((b0 & 0xE0) == 0xC0 and ss.len >= 2) {
            cp = @as(i64, b0 & 0x1F);
            cp = (cp << 6) | @as(i64, sp[1] & 0x3F);
        } else if ((b0 & 0xF0) == 0xE0 and ss.len >= 3) {
            cp = @as(i64, b0 & 0x0F);
            cp = (cp << 6) | @as(i64, sp[1] & 0x3F);
            cp = (cp << 6) | @as(i64, sp[2] & 0x3F);
        } else if ((b0 & 0xF8) == 0xF0 and ss.len >= 4) {
            cp = @as(i64, b0 & 0x07);
            cp = (cp << 6) | @as(i64, sp[1] & 0x3F);
            cp = (cp << 6) | @as(i64, sp[2] & 0x3F);
            cp = (cp << 6) | @as(i64, sp[3] & 0x3F);
        } else {
            cp = @as(i64, b0);
        }
        return obj_new_int(cp);
    }

    if (spec == 'd' or spec == 'i') {
        // Parse a decimal (or 0x-prefixed hex) integer with overflow guard.
        if (ss.len == 0) return obj_new_int(0);
        const sp: [*]const u8 = @ptrFromInt(ss.ptr);
        var i: u32 = 0;
        while (i < ss.len and is_space(sp[i])) i += 1;
        var neg: bool = false;
        if (i < ss.len and sp[i] == '-') { neg = true; i += 1; }
        else if (i < ss.len and sp[i] == '+') { i += 1; }
        // Check for 0x prefix
        if (spec == 'i' and i + 1 < ss.len and sp[i] == '0' and
            (sp[i + 1] == 'x' or sp[i + 1] == 'X'))
        {
            i += 2;
            var val: i64 = 0;
            while (i < ss.len) : (i += 1) {
                const c = sp[i];
                var digit: i64 = 0;
                if (c >= '0' and c <= '9') digit = @as(i64, c - '0')
                else if (c >= 'a' and c <= 'f') digit = @as(i64, c - 'a' + 10)
                else if (c >= 'A' and c <= 'F') digit = @as(i64, c - 'A' + 10)
                else break;
                val = accumulate_i64(val, 16, digit) orelse return obj_new_int(if (neg) INT64_MIN else INT64_MAX);
            }
            return obj_new_int(if (neg) -val else val);
        }
        var val: i64 = 0;
        while (i < ss.len and sp[i] >= '0' and sp[i] <= '9') : (i += 1) {
            val = accumulate_i64(val, 10, @as(i64, sp[i] - '0')) orelse return obj_new_int(if (neg) INT64_MIN else INT64_MAX);
        }
        return obj_new_int(if (neg) -val else val);
    }

    if (spec == 'x' or spec == 'X') {
        // Parse a hexadecimal integer with overflow guard.
        if (ss.len == 0) return obj_new_int(0);
        const sp: [*]const u8 = @ptrFromInt(ss.ptr);
        var i: u32 = 0;
        while (i < ss.len and is_space(sp[i])) i += 1;
        var neg: bool = false;
        if (i < ss.len and sp[i] == '-') { neg = true; i += 1; }
        else if (i < ss.len and sp[i] == '+') { i += 1; }
        // Optional 0x prefix
        if (i + 1 < ss.len and sp[i] == '0' and (sp[i + 1] == 'x' or sp[i + 1] == 'X'))
            i += 2;
        var val: i64 = 0;
        while (i < ss.len) : (i += 1) {
            const c = sp[i];
            var digit: i64 = 0;
            if (c >= '0' and c <= '9') digit = @as(i64, c - '0')
            else if (c >= 'a' and c <= 'f') digit = @as(i64, c - 'a' + 10)
            else if (c >= 'A' and c <= 'F') digit = @as(i64, c - 'A' + 10)
            else break;
            val = accumulate_i64(val, 16, digit) orelse return obj_new_int(if (neg) INT64_MIN else INT64_MAX);
        }
        return obj_new_int(if (neg) -val else val);
    }

    if (spec == 'o') {
        // Parse an octal integer with overflow guard.
        if (ss.len == 0) return obj_new_int(0);
        const sp: [*]const u8 = @ptrFromInt(ss.ptr);
        var i: u32 = 0;
        while (i < ss.len and is_space(sp[i])) i += 1;
        var neg: bool = false;
        if (i < ss.len and sp[i] == '-') { neg = true; i += 1; }
        else if (i < ss.len and sp[i] == '+') { i += 1; }
        var val: i64 = 0;
        while (i < ss.len and sp[i] >= '0' and sp[i] <= '7') : (i += 1) {
            val = accumulate_i64(val, 8, @as(i64, sp[i] - '0')) orelse return obj_new_int(if (neg) INT64_MIN else INT64_MAX);
        }
        return obj_new_int(if (neg) -val else val);
    }

    // Unknown format specifier — unsupported.
    stubs.unsupported("scan");
    return 0;
}

fn is_space(c: u8) bool {
    return c == ' ' or c == '\t' or c == '\n' or c == '\r';
}

const INT64_MAX: i64 = 0x7FFF_FFFF_FFFF_FFFF;
const INT64_MIN: i64 = -0x8000_0000_0000_0000;

/// Return ``val * base + digit`` saturated on overflow — ``null`` signals
/// the caller should clamp to ``INT64_MAX``/``INT64_MIN``.  Used by the
/// ``scan`` integer parsers so a 30-digit input doesn't silently wrap
/// into a negative value.
fn accumulate_i64(val: i64, base: i64, digit: i64) ?i64 {
    const mul = @mulWithOverflow(val, base);
    if (mul[1] != 0) return null;
    const add = @addWithOverflow(mul[0], digit);
    if (add[1] != 0) return null;
    return add[0];
}

pub export fn tcl_cmd_binary(sub: i32, arg: i32) i32 {
    _ = sub;
    _ = arg;
    stubs.unsupported("binary");
    return 0;
}

// ``regexp`` moved to tcl_regex.zig — real implementation backed
// by Tcl's Henry-Spencer engine (linked from
// ``runtime/zig/vendor/tcl-regex/``).  Only ``regsub`` remains a
// stub until we add the substitution path.

pub export fn tcl_cmd_regsub(pattern: i32, str: i32) i32 {
    _ = pattern;
    _ = str;
    stubs.unsupported("regsub");
    return 0;
}

// ``encoding`` moved to tcl_encoding.zig — has a real (UTF-8 only)
// implementation for convertfrom / convertto / system / names /
// dirs.  Unknown subcommands / unsupported codecs still trap.
