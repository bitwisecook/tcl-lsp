// ``scan`` — parse a string according to a format string, storing matched
// values into named variables.
//
// Synopsis:
//   scan string format ?var1 var2 ...?
//
// If no variables are given, returns a list of matched values (single-value
// form only; Tcl 8.5+ extension).  With variables, stores each parsed value
// and returns the count of successfully assigned fields.
//
// Supported format specifiers:
//   %d  decimal integer
//   %i  integer (auto-detect 0x hex / 0 octal / decimal)
//   %x, %X  hex integer (optional 0x prefix)
//   %o  octal integer
//   %c  Unicode codepoint of next character
//   %s  whitespace-delimited word
//   %[  character-class match (not implemented; traps gracefully)
//   %f, %e, %g  floating-point (parsed as integer truncation — no fp runtime)
//   %n  no-op count specifier (not consuming; assigned current position)
//   %%  literal percent in format (skip in input)
//
// Width fields (e.g. %5d) are parsed but ignored for non-%s specifiers.
// For %s a width field limits the number of characters taken.

const rt     = @import("../tcl_runtime.zig");
const frames = @import("../interp/tcl_frames.zig");
const stubs  = @import("../stubs/tcl_stubs.zig");
const chars  = @import("../valtypes/tcl_chars.zig");
const reg    = @import("../dispatch/tcl_cmd_registry.zig");

const obj_new_int       = rt.obj_new_int;
const obj_new_string    = rt.obj_new_string;
const obj_ensure_string = rt.obj_ensure_string;
const alloc             = rt.alloc;
const memcpy            = rt.memcpy;
const is_space          = chars.is_space;

// Re-use the saturating integer parser from tcl_fmt_stubs.zig via its
// module-level helpers.
const fmt_mod = @import("../stubs/tcl_fmt_stubs.zig");

// ── helpers ──────────────────────────────────────────────────────────────────

/// Skip leading ASCII whitespace in ``src`` starting at ``pos``.
inline fn skip_space(src: [*]const u8, len: u32, pos: u32) u32 {
    var i = pos;
    while (i < len and is_space(src[i])) i += 1;
    return i;
}

/// Return codepoint of the UTF-8 character starting at ``src[i]``; advance
/// ``i`` past it.  On bad encoding, returns the byte value.
fn read_utf8_cp(src: [*]const u8, len: u32, i: *u32) i64 {
    if (i.* >= len) return 0;
    const b0 = src[i.*];
    if (b0 < 0x80) {
        i.* += 1;
        return @as(i64, b0);
    } else if ((b0 & 0xE0) == 0xC0 and i.* + 1 < len) {
        const cp = (@as(i64, b0 & 0x1F) << 6) | @as(i64, src[i.* + 1] & 0x3F);
        i.* += 2;
        return cp;
    } else if ((b0 & 0xF0) == 0xE0 and i.* + 2 < len) {
        const cp = (@as(i64, b0 & 0x0F) << 12) |
                   (@as(i64, src[i.* + 1] & 0x3F) << 6) |
                   @as(i64, src[i.* + 2] & 0x3F);
        i.* += 3;
        return cp;
    } else if ((b0 & 0xF8) == 0xF0 and i.* + 3 < len) {
        const cp = (@as(i64, b0 & 0x07) << 18) |
                   (@as(i64, src[i.* + 1] & 0x3F) << 12) |
                   (@as(i64, src[i.* + 2] & 0x3F) << 6) |
                   @as(i64, src[i.* + 3] & 0x3F);
        i.* += 4;
        return cp;
    }
    i.* += 1;
    return @as(i64, b0);
}

// Overflow-safe i64 accumulator.
const INT64_MAX: i64 = 0x7FFF_FFFF_FFFF_FFFF;
const INT64_MIN: i64 = -0x8000_0000_0000_0000;

fn accumulate(val: i64, base: i64, digit: i64) ?i64 {
    const m = @mulWithOverflow(val, base);
    if (m[1] != 0) return null;
    const a = @addWithOverflow(m[0], digit);
    if (a[1] != 0) return null;
    return a[0];
}

fn digit_val(c: u8, base: i64) i64 {
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

/// Parse an integer at ``src[pos..]``, advancing ``pos`` past the digits.
/// Returns the TclObj with the parsed value, or sets ``matched`` to false on
/// total failure (no digit consumed at all).
fn scan_int(src: [*]const u8, src_len: u32, pos: *u32,
            base_in: i64, accept_0x: bool, matched: *bool) i32 {
    var i = pos.*;
    var neg = false;
    if (i < src_len and src[i] == '-') { neg = true; i += 1; }
    else if (i < src_len and src[i] == '+') { i += 1; }

    var base = base_in;
    if (base == 0) {
        base = 10;
        if (i + 1 < src_len and src[i] == '0' and (src[i + 1] == 'x' or src[i + 1] == 'X')) {
            base = 16;
            i += 2;
        }
    } else if (accept_0x and i + 1 < src_len and src[i] == '0' and
               (src[i + 1] == 'x' or src[i + 1] == 'X')) {
        i += 2;
    }

    const start = i;
    var val: i64 = 0;
    while (i < src_len) : (i += 1) {
        const d = digit_val(src[i], base);
        if (d < 0) break;
        val = accumulate(val, base, d) orelse {
            // Overflow — skip remaining digits and return saturated value.
            while (i < src_len and digit_val(src[i], base) >= 0) i += 1;
            pos.* = i;
            matched.* = true;
            return obj_new_int(if (neg) INT64_MIN else INT64_MAX);
        };
    }
    if (i == start) {
        matched.* = false;
        return 0;
    }
    pos.* = i;
    matched.* = true;
    return obj_new_int(if (neg) -val else val);
}

// ── main handler ─────────────────────────────────────────────────────────────

fn eval_scan(words: []const i32) i32 {
    if (words.len < 3) return obj_new_int(0);

    const ss = obj_ensure_string(words[1]);
    const fs = obj_ensure_string(words[2]);

    const has_vars = (words.len > 3);
    // vars start at words[3]; N = words.len - 3.
    const n_vars: u32 = if (has_vars) @intCast(words.len - 3) else 0;

    const src: [*]const u8 = if (ss.len > 0) @ptrFromInt(ss.ptr) else @ptrFromInt(1);
    const fmt: [*]const u8 = if (fs.len > 0) @ptrFromInt(fs.ptr) else @ptrFromInt(1);

    var si: u32 = 0;   // position in the source string
    var fi: u32 = 0;   // position in the format string
    var vi: u32 = 0;   // next variable index
    var assigned: u32 = 0;
    // In no-variable form, collected values are appended here.
    var list_result: i32 = obj_new_string(0, 0);

    while (fi < fs.len) {
        // Non-% literal: must match a character in the source.
        if (fmt[fi] != '%') {
            // Whitespace in format matches any amount of whitespace in input.
            if (is_space(fmt[fi])) {
                si = skip_space(src, ss.len, si);
                fi += 1;
                while (fi < fs.len and is_space(fmt[fi])) fi += 1;
                continue;
            }
            // Literal character match.
            if (si >= ss.len or src[si] != fmt[fi]) break;
            si += 1;
            fi += 1;
            continue;
        }
        fi += 1; // skip '%'
        if (fi >= fs.len) break;

        // %% — match a literal percent.
        if (fmt[fi] == '%') {
            if (si >= ss.len or src[si] != '%') break;
            si += 1;
            fi += 1;
            continue;
        }

        // Suppress flag: %* means match but don't assign.
        var suppress = false;
        if (fmt[fi] == '*') {
            suppress = true;
            fi += 1;
            if (fi >= fs.len) break;
        }

        // Width field.
        var width: u32 = 0;
        while (fi < fs.len and fmt[fi] >= '0' and fmt[fi] <= '9') : (fi += 1) {
            width = width * 10 + @as(u32, fmt[fi] - '0');
        }
        if (fi >= fs.len) break;

        const spec = fmt[fi];
        fi += 1;

        // %n — number of chars consumed so far (no input consumed).
        if (spec == 'n') {
            if (!suppress) {
                const val = obj_new_int(@intCast(si));
                if (has_vars) {
                    if (vi >= n_vars) break;
                    _ = frames.var_set(words[3 + vi], val);
                } else {
                    list_result = rt.tcl_list(list_result, val);
                }
                vi += 1;
                assigned += 1;
            }
            continue;
        }

        // All other specifiers consume input.
        si = skip_space(src, ss.len, si);
        if (si >= ss.len) break;

        if (spec == 'c') {
            var tmp_i = si;
            const cp = read_utf8_cp(src, ss.len, &tmp_i);
            const val = obj_new_int(cp);
            if (!suppress) {
                if (has_vars) {
                    if (vi >= n_vars) break;
                    _ = frames.var_set(words[3 + vi], val);
                } else {
                    list_result = rt.tcl_list(list_result, val);
                }
                vi += 1;
                assigned += 1;
            }
            si = tmp_i;
            continue;
        }

        if (spec == 'd' or spec == 'i' or spec == 'x' or spec == 'X' or spec == 'o') {
            const base: i64 = switch (spec) {
                'd' => 10, 'i' => 0, 'x', 'X' => 16, else => 8,
            };
            const accept_0x = (spec == 'x' or spec == 'X');
            var matched = false;
            const val = scan_int(src, ss.len, &si, base, accept_0x, &matched);
            if (!matched) break;
            if (!suppress) {
                if (has_vars) {
                    if (vi >= n_vars) break;
                    _ = frames.var_set(words[3 + vi], val);
                } else {
                    list_result = rt.tcl_list(list_result, val);
                }
                vi += 1;
                assigned += 1;
            }
            continue;
        }

        if (spec == 'f' or spec == 'e' or spec == 'g' or spec == 'E' or spec == 'G') {
            // Floating-point: parse as much as looks like a float, return int
            // truncation (we have no fp runtime).
            const neg = if (si < ss.len and src[si] == '-') blk: { si += 1; break :blk true; }
                        else if (si < ss.len and src[si] == '+') blk: { si += 1; break :blk false; }
                        else false;
            var int_val: i64 = 0;
            var has_digits = false;
            while (si < ss.len and src[si] >= '0' and src[si] <= '9') : (si += 1) {
                int_val = int_val * 10 + @as(i64, src[si] - '0');
                has_digits = true;
            }
            // Skip fractional part.
            if (si < ss.len and src[si] == '.') {
                si += 1;
                while (si < ss.len and src[si] >= '0' and src[si] <= '9') si += 1;
                has_digits = true;
            }
            // Skip exponent.
            if (si < ss.len and (src[si] == 'e' or src[si] == 'E')) {
                si += 1;
                if (si < ss.len and (src[si] == '+' or src[si] == '-')) si += 1;
                while (si < ss.len and src[si] >= '0' and src[si] <= '9') si += 1;
            }
            if (!has_digits) break;
            const val = obj_new_int(if (neg) -int_val else int_val);
            if (!suppress) {
                if (has_vars) {
                    if (vi >= n_vars) break;
                    _ = frames.var_set(words[3 + vi], val);
                } else {
                    list_result = rt.tcl_list(list_result, val);
                }
                vi += 1;
                assigned += 1;
            }
            continue;
        }

        if (spec == 's') {
            // Scan a whitespace-delimited word.
            const start_si = si;
            var end_si = si;
            const max_chars: u32 = if (width > 0) width else ss.len;
            var chars_taken: u32 = 0;
            while (end_si < ss.len and !is_space(src[end_si]) and chars_taken < max_chars) {
                end_si += 1;
                chars_taken += 1;
            }
            if (end_si == start_si) break; // no word
            const val = obj_new_string(@bitCast(ss.ptr + start_si), @bitCast(end_si - start_si));
            if (!suppress) {
                if (has_vars) {
                    if (vi >= n_vars) break;
                    _ = frames.var_set(words[3 + vi], val);
                } else {
                    list_result = rt.tcl_list(list_result, val);
                }
                vi += 1;
                assigned += 1;
            }
            si = end_si;
            continue;
        }

        // Unsupported specifier (e.g. %[, %u, %p) — stop.
        break;
    }

    if (!has_vars) {
        // No-variable form: return the list of parsed values.
        return list_result;
    }
    return obj_new_int(@intCast(assigned));
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "scan", .arity_min = 2, .arity_max = null, .handler = &eval_scan },
};
