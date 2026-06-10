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
const obj = @import("../valtypes/tcl_obj.zig");

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

    if (spec == 'd') return scan_int_saturating(ss.ptr, ss.len, 10, false);
    if (spec == 'i') return scan_int_saturating(ss.ptr, ss.len, 0, false);
    if (spec == 'x' or spec == 'X') return scan_int_saturating(ss.ptr, ss.len, 16, true);
    if (spec == 'o') return scan_int_saturating(ss.ptr, ss.len, 8, false);

    // Unknown format specifier — unsupported.
    stubs.unsupported("scan");
    return 0;
}

// Digit classification happens inside :func:`digit_value` which is
// base-parameterised — no need to import the per-class predicates.
const is_space = @import("../valtypes/tcl_chars.zig").is_space;

const INT64_MAX: i64 = 0x7FFF_FFFF_FFFF_FFFF;
const INT64_MIN: i64 = -0x8000_0000_0000_0000;

/// Return ``val * base + digit``; ``null`` signals overflow so the
/// caller can clamp to ``INT64_MAX``/``INT64_MIN``.  Shared by every
/// ``scan`` integer parser so a 30-digit input doesn't silently wrap.
fn accumulate_i64(val: i64, base: i64, digit: i64) ?i64 {
    const mul = @mulWithOverflow(val, base);
    if (mul[1] != 0) return null;
    const add = @addWithOverflow(mul[0], digit);
    if (add[1] != 0) return null;
    return add[0];
}

/// Resolve a single base-*base* digit character.  Returns ``-1`` if
/// *c* is not a valid digit for the base.  ``base==16`` accepts
/// ``a``-``f`` / ``A``-``F`` as 10-15.
fn digit_value(c: u8, base: i64) i64 {
    if (base == 16) {
        if (c >= '0' and c <= '9') return @as(i64, c - '0');
        if (c >= 'a' and c <= 'f') return @as(i64, c - 'a' + 10);
        if (c >= 'A' and c <= 'F') return @as(i64, c - 'A' + 10);
        return -1;
    }
    if (c < '0') return -1;
    const d: i64 = @as(i64, c - '0');
    if (d >= base) return -1;
    return d;
}

/// Parse a saturating integer from the *str* argument of ``scan``.
/// Skips leading whitespace, consumes an optional ``+`` / ``-`` sign,
/// and then parses digits in *base* (``0`` means auto-detect from a
/// ``0x`` prefix).  When *accept_0x_prefix* is true, a leading ``0x`` /
/// ``0X`` (after the sign) is also consumed for base-16 inputs — this
/// matches how ``scan`` accepts ``%x`` with or without the prefix.
///
/// Returns the parsed value as a TclObj int — ``INT64_MAX`` /
/// ``INT64_MIN`` on overflow (signed by *neg*), ``0`` on no-digits.
/// Consolidates the three per-base copies that used to live inline.
fn scan_int_saturating(src_ptr: u32, src_len: u32, base_in: i64, accept_0x_prefix: bool) i32 {
    if (src_len == 0) return obj_new_int(0);
    const sp: [*]const u8 = @ptrFromInt(src_ptr);
    var i: u32 = 0;
    while (i < src_len and is_space(sp[i])) i += 1;
    var neg: bool = false;
    if (i < src_len and sp[i] == '-') {
        neg = true;
        i += 1;
    } else if (i < src_len and sp[i] == '+') {
        i += 1;
    }

    // Resolve the final base, consuming ``0x`` when allowed.
    var base: i64 = base_in;
    if (base == 0) {
        // ``%i`` — auto-detect decimal vs hex.  No octal auto-detect
        // (Tcl 9 dropped leading-0-means-octal).
        base = 10;
        if (i + 1 < src_len and sp[i] == '0' and (sp[i + 1] == 'x' or sp[i + 1] == 'X')) {
            base = 16;
            i += 2;
        }
    } else if (accept_0x_prefix and i + 1 < src_len and sp[i] == '0' and
        (sp[i + 1] == 'x' or sp[i + 1] == 'X'))
    {
        i += 2;
    }

    var val: i64 = 0;
    while (i < src_len) : (i += 1) {
        const d = digit_value(sp[i], base);
        if (d < 0) break;
        val = accumulate_i64(val, base, d) orelse
            return obj_new_int(if (neg) INT64_MIN else INT64_MAX);
    }
    return obj_new_int(if (neg) -val else val);
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
    // 2-arg compiled form: subSpec defaults to empty string (deletion mode).
    const regex_mod = @import("../valtypes/tcl_regex.zig");
    const rt = @import("../tcl_runtime.zig");
    return regex_mod.do_regsub(pattern, str, rt.obj_new_string(0, 0), false, false, null);
}

// ``encoding`` moved to tcl_encoding.zig — has a real (UTF-8 only)
// implementation for convertfrom / convertto / system / names /
// dirs.  Unknown subcommands / unsupported codecs still trap.
