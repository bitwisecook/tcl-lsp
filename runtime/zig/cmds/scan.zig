// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

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
//   %[  character-class match (``%[abc]`` / ``%[^x]`` / ``%[a-z]``)
//   %f, %e, %g  floating-point (real TYPE_FLOAT result)
//   %n  no-op count specifier (not consuming; assigned current position)
//   %%  literal percent in format (skip in input)
//
// A leading ``N$`` (``%2$d``) selects the XPG3 target variable.  Width
// fields (e.g. %3d) limit the characters consumed for %s, %d, %f, and
// %[ conversions.

const rt = @import("../tcl_runtime.zig");
const std = @import("std");
const result_mod = @import("../interp/tcl_result.zig");
const frames = @import("../interp/tcl_frames.zig");
const stubs = @import("../stubs/tcl_stubs.zig");
const chars = @import("../valtypes/tcl_chars.zig");
const obj_mod_scan = @import("../valtypes/tcl_obj.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");
const obj_mod = @import("../valtypes/tcl_obj.zig");
const bignum = @import("../valtypes/tcl_bignum.zig");

const obj_new_int = rt.obj_new_int;
const obj_new_string = rt.obj_new_string;
const obj_ensure_string = rt.obj_ensure_string;
const alloc = rt.alloc;
const memcpy = rt.memcpy;
const is_space = chars.is_space;

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
fn scan_int(src: [*]const u8, src_len: u32, pos: *u32, base_in: i64, accept_0x: bool, matched: *bool) i32 {
    var i = pos.*;
    var neg = false;
    if (i < src_len and src[i] == '-') {
        neg = true;
        i += 1;
    } else if (i < src_len and src[i] == '+') {
        i += 1;
    }

    var base = base_in;
    if (base == 0) {
        base = 10;
        if (i + 1 < src_len and src[i] == '0' and (src[i + 1] == 'x' or src[i + 1] == 'X')) {
            base = 16;
            i += 2;
        }
    } else if (accept_0x and i + 1 < src_len and src[i] == '0' and
        (src[i + 1] == 'x' or src[i + 1] == 'X'))
    {
        i += 2;
    }

    const start = i;
    var val: i64 = 0;
    while (i < src_len) : (i += 1) {
        const d = digit_val(src[i], base);
        if (d < 0) break;
        val = accumulate(val, base, d) orelse {
            // Overflow — promote to bignum.  Walk forward to the
            // end of the digit run, then build a Managed via
            // ``setString`` from the captured slice.  Keeps Tcl 9
            // semantics where ``[scan 99999999999999999999999 %d
            // n]`` lets ``$n`` carry the full magnitude rather
            // than the saturate-to-i64::MAX behaviour we shipped
            // before bignum support landed.
            while (i < src_len and digit_val(src[i], base) >= 0) i += 1;
            pos.* = i;
            matched.* = true;
            return scan_bignum(src, start, i, base, neg);
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

/// Build a TYPE_BIGNUM TclObj from the digit run ``src[start..end]``
/// in *base* (10 / 16 / 8).  Returns 0 on OOM.
fn scan_bignum(
    src: [*]const u8,
    start: u32,
    end: u32,
    base: i64,
    neg: bool,
) i32 {
    const body_len: u32 = end - start;
    if (body_len == 0) return obj_new_int(0);
    // Copy the digits into an owned slice for ``setString``; it
    // can't take a raw (ptr, len) and we may need to skip a leading
    // sign / base prefix that ``scan_int`` already consumed.
    const body_buf = bignum.allocator.alloc(u8, body_len) catch return obj_new_int(0);
    defer bignum.allocator.free(body_buf);
    for (0..body_len) |k| body_buf[k] = src[start + k];

    const m = bignum.allocator.create(bignum.BigInt) catch return obj_new_int(0);
    m.* = bignum.BigInt.init(bignum.allocator) catch {
        bignum.allocator.destroy(m);
        return obj_new_int(0);
    };
    m.setString(@intCast(base), body_buf) catch {
        m.deinit();
        bignum.allocator.destroy(m);
        return obj_new_int(0);
    };
    if (neg and !m.eqlZero()) m.negate();
    return obj_mod.obj_new_bignum_take(m);
}

// ── main handler ─────────────────────────────────────────────────────────────

// XPG3 positional indices are limited to INT_MAX-1 (Tcl 9 supports
// fewer than INT_MAX command arguments); a larger ``%N$`` index is an
// error rather than an invitation to allocate billions of slots.
const SCAN_INDEX_LIMIT: u64 = 0x7FFF_FFFF;

// Scratch buffer for the dynamic ``bad scan conversion character "X"``
// message — single-threaded WASM, so a module-level buffer is safe.
var g_bad_conv_buf: [64]u8 = undefined;

fn bad_conv_msg(ch: u8) []const u8 {
    const prefix = "bad scan conversion character \"";
    @memcpy(g_bad_conv_buf[0..prefix.len], prefix);
    g_bad_conv_buf[prefix.len] = ch;
    g_bad_conv_buf[prefix.len + 1] = '"';
    return g_bad_conv_buf[0 .. prefix.len + 2];
}

const ValidateResult = struct {
    total_vars: u32 = 0,
    err: ?[]const u8 = null,
};

/// Pre-parse the format string, mirroring C ``ValidateFormat`` in
/// ``tclScan.c``.  Computes the number of result slots (*total_vars*)
/// and rejects malformed formats with the canonical Tcl 9 error
/// messages.  ``has_vars`` / ``n_vars`` describe the classic
/// (variable) form; the inline form passes ``has_vars=false``.  The
/// slot count drives the result-list / objs[] sizing the matcher uses,
/// and the index-range guard keeps a stray ``%9999999999$`` from
/// triggering a multi-gigabyte allocation.
fn validate_scan_format(
    fmt: [*]const u8,
    fs_len: u32,
    has_vars: bool,
    n_vars: u32,
) ValidateResult {
    var fi: u32 = 0;
    var obj_index: u32 = 0; // next sequential slot
    var got_xpg = false;
    var got_seq = false;
    var xpg_size: u32 = 0; // highest XPG position seen (1-based)
    var seq_count: u32 = 0; // count of sequential non-suppressed conversions

    while (fi < fs_len) {
        if (fmt[fi] != '%') {
            fi += 1;
            continue;
        }
        fi += 1;
        if (fi >= fs_len) break;
        if (fmt[fi] == '%') {
            fi += 1;
            continue;
        }

        var suppress = false;
        if (fmt[fi] == '*') {
            suppress = true;
            fi += 1;
        } else if (fmt[fi] >= '0' and fmt[fi] <= '9') {
            // Possible XPG3 ``%N$`` index.
            var j = fi;
            var num: u64 = 0;
            while (j < fs_len and fmt[j] >= '0' and fmt[j] <= '9') : (j += 1) {
                num = num * 10 + @as(u64, fmt[j] - '0');
                if (num > SCAN_INDEX_LIMIT) num = SCAN_INDEX_LIMIT; // saturate
            }
            if (j < fs_len and fmt[j] == '$') {
                got_xpg = true;
                if (got_seq)
                    return .{ .err = "cannot mix \"%\" and \"%n$\" conversion specifiers" };
                if (num == 0 or num >= SCAN_INDEX_LIMIT)
                    return .{ .err = "\"%n$\" argument index out of range" };
                obj_index = @intCast(num - 1);
                if (has_vars and obj_index >= n_vars)
                    return .{ .err = "\"%n$\" argument index out of range" };
                if (!has_vars and num > xpg_size) xpg_size = @intCast(num);
                fi = j + 1;
            } else {
                got_seq = true;
                if (got_xpg)
                    return .{ .err = "cannot mix \"%\" and \"%n$\" conversion specifiers" };
            }
        } else {
            got_seq = true;
            if (got_xpg)
                return .{ .err = "cannot mix \"%\" and \"%n$\" conversion specifiers" };
        }

        // Width field.
        var has_width = false;
        while (fi < fs_len and fmt[fi] >= '0' and fmt[fi] <= '9') : (fi += 1) has_width = true;

        // Size modifiers (h / l / ll / L) — accepted but ignored.
        while (fi < fs_len and (fmt[fi] == 'h' or fmt[fi] == 'l' or fmt[fi] == 'L')) : (fi += 1) {}

        if (fi >= fs_len) return .{ .err = bad_conv_msg(' ') };
        const spec = fmt[fi];
        fi += 1;

        switch (spec) {
            'c' => {
                if (has_width)
                    return .{ .err = "field width may not be specified in %c conversion" };
            },
            'n', 's', 'd', 'i', 'o', 'x', 'X', 'b', 'u', 'e', 'E', 'f', 'g', 'G' => {},
            '[' => {
                if (fi >= fs_len) return .{ .err = "unmatched [ in format string" };
                if (fmt[fi] == '^') {
                    fi += 1;
                    if (fi >= fs_len) return .{ .err = "unmatched [ in format string" };
                }
                if (fmt[fi] == ']') {
                    fi += 1;
                    if (fi >= fs_len) return .{ .err = "unmatched [ in format string" };
                }
                while (fi < fs_len and fmt[fi] != ']') fi += 1;
                if (fi >= fs_len) return .{ .err = "unmatched [ in format string" };
                fi += 1; // closing ]
            },
            else => return .{ .err = bad_conv_msg(spec) },
        }

        if (!suppress) {
            // ``%N$`` and sequential conversions both advance the cursor.
            if (has_vars and !got_xpg and obj_index >= n_vars)
                return .{ .err = "different numbers of variable names and field specifiers" };
            obj_index += 1;
            if (!got_xpg) seq_count += 1;
        }
    }

    const total: u32 = if (has_vars) n_vars else (if (got_xpg) xpg_size else seq_count);

    // Detect multiply / unassigned slots — mirrors the final nassign
    // sweep in C.  We re-walk to populate per-slot counts; ``total`` is
    // bounded by the index-range guard above so the temporary array is
    // small.
    if (total > 0) {
        const counts = bignum.allocator.alloc(u16, total) catch
            return .{ .total_vars = total };
        defer bignum.allocator.free(counts);
        @memset(counts, 0);
        count_assignments(fmt, fs_len, counts);
        var i: u32 = 0;
        while (i < total) : (i += 1) {
            if (counts[i] > 1)
                return .{ .err = "variable is assigned by multiple \"%n$\" conversion specifiers" };
            // An unassigned slot is an error in the *variable* form —
            // both for an XPG gap (``scan {1} {%2$d} a b``) and for
            // surplus variables (``scan {1 2} {%d %d} a b c``).  Tcl 9
            // reports both as "variable is not assigned ...".  The inline
            // (no-variable) form is exempt: it legitimately yields leading
            // empty list elements (``scan {1} {%2$d}`` -> ``{} 1``).
            if (has_vars and counts[i] == 0)
                return .{ .err = "variable is not assigned by any conversion specifiers" };
        }
    }

    return .{ .total_vars = total };
}

/// Re-walk the (already-validated) format counting non-suppressed
/// assignments per slot, so the caller can flag multiply-assigned or
/// unassigned positions.
fn count_assignments(fmt: [*]const u8, fs_len: u32, counts: []u16) void {
    var fi: u32 = 0;
    var obj_index: u32 = 0;
    while (fi < fs_len) {
        if (fmt[fi] != '%') {
            fi += 1;
            continue;
        }
        fi += 1;
        if (fi >= fs_len) break;
        if (fmt[fi] == '%') {
            fi += 1;
            continue;
        }
        var suppress = false;
        if (fmt[fi] == '*') {
            suppress = true;
            fi += 1;
        } else if (fmt[fi] >= '0' and fmt[fi] <= '9') {
            var j = fi;
            var num: u64 = 0;
            while (j < fs_len and fmt[j] >= '0' and fmt[j] <= '9') : (j += 1) {
                num = num * 10 + @as(u64, fmt[j] - '0');
                if (num > SCAN_INDEX_LIMIT) num = SCAN_INDEX_LIMIT;
            }
            if (j < fs_len and fmt[j] == '$') {
                obj_index = @intCast(num - 1);
                fi = j + 1;
            }
        }
        while (fi < fs_len and fmt[fi] >= '0' and fmt[fi] <= '9') : (fi += 1) {}
        while (fi < fs_len and (fmt[fi] == 'h' or fmt[fi] == 'l' or fmt[fi] == 'L')) : (fi += 1) {}
        if (fi >= fs_len) break;
        const spec = fmt[fi];
        fi += 1;
        if (spec == '[') {
            if (fi < fs_len and fmt[fi] == '^') fi += 1;
            if (fi < fs_len and fmt[fi] == ']') fi += 1;
            while (fi < fs_len and fmt[fi] != ']') fi += 1;
            if (fi < fs_len) fi += 1;
        }
        if (!suppress) {
            if (obj_index < counts.len) counts[obj_index] += 1;
            obj_index += 1;
        }
    }
}

/// Store one converted value into the result slot array.  ``val`` is
/// always consumed.  Suppressed conversions release the value but still
/// count toward ``nconversions`` (so a successful ``%*d`` defeats the
/// "nothing matched" underflow rule, matching C).
fn scan_store(
    objs: []i32,
    obj_index: *u32,
    nconversions: *u32,
    val: i32,
    suppress: bool,
    xpg_slot: i64,
) void {
    if (suppress) {
        obj_mod_scan.tcl_obj_release(val);
    } else {
        const slot: u32 = if (xpg_slot >= 0) @intCast(xpg_slot) else obj_index.*;
        if (slot < objs.len) {
            if (objs[slot] != 0) obj_mod_scan.tcl_obj_release(objs[slot]);
            objs[slot] = val;
        } else {
            // Validation guarantees this never happens; be defensive.
            obj_mod_scan.tcl_obj_release(val);
        }
        if (xpg_slot < 0) obj_index.* += 1;
    }
    nconversions.* += 1;
}

fn eval_scan(words: []const i32) result_mod.InterpResult {
    if (words.len < 3) return result_mod.from_globals(obj_new_int(0));

    const ss = obj_ensure_string(words[1]);
    const fs = obj_ensure_string(words[2]);

    const has_vars = (words.len > 3);
    // vars start at words[3]; N = words.len - 3.
    const n_vars: u32 = if (has_vars) @intCast(words.len - 3) else 0;

    const src: [*]const u8 = if (ss.len > 0) @ptrFromInt(ss.ptr) else @ptrFromInt(1);
    const fmt: [*]const u8 = if (fs.len > 0) @ptrFromInt(fs.ptr) else @ptrFromInt(1);

    // Validate the format up front (mirrors C ``ValidateFormat``): this
    // both rejects malformed specs and yields the result-slot count that
    // sizes the objs[] array.  Errors are raised exactly as Tcl 9.
    const vr = validate_scan_format(fmt, fs.len, has_vars, n_vars);
    if (vr.err) |msg| {
        raise_scan_error(msg);
        return result_mod.from_globals(0);
    }
    const total_vars = vr.total_vars;

    // One slot per conversion (sequential count, or highest XPG index).
    // Each successful conversion drops its value into ``objs[slot]``;
    // unfilled slots become empty strings (inline form) or stay unset
    // (classic form), mirroring Tcl's inline empty-fill semantics.
    const objs: []i32 = if (total_vars == 0)
        &[_]i32{}
    else
        bignum.allocator.alloc(i32, total_vars) catch {
            raise_scan_error("out of memory");
            return result_mod.from_globals(0);
        };
    defer if (total_vars != 0) bignum.allocator.free(objs);
    @memset(objs, 0);

    var si: u32 = 0; // position in the source string
    var fi: u32 = 0; // position in the format string
    var obj_index: u32 = 0; // next sequential slot
    var nconversions: u32 = 0; // successful conversions (incl. suppressed)
    var underflow = false; // ran out of input mid-conversion

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
            // Literal character match.  Running out of input here is an
            // underflow (C ``ValidateFormat``/scan: a literal at EOF);
            // a plain character mismatch is not.
            if (si >= ss.len) {
                underflow = true;
                break;
            }
            if (src[si] != fmt[fi]) break;
            si += 1;
            fi += 1;
            continue;
        }
        fi += 1; // skip '%'
        if (fi >= fs.len) break;

        // %% — match a literal percent.
        if (fmt[fi] == '%') {
            if (si >= ss.len) {
                underflow = true;
                break;
            }
            if (src[si] != '%') break;
            si += 1;
            fi += 1;
            continue;
        }

        // XPG3 positional conversion: ``%N$spec`` assigns to the N-th
        // variable (1-based) rather than the next sequential one.  The
        // position precedes the suppress / width / size fields.
        var xpg_slot: i64 = -1;
        {
            var j = fi;
            var num: u32 = 0;
            while (j < fs.len and fmt[j] >= '0' and fmt[j] <= '9') : (j += 1) {
                num = num * 10 + @as(u32, fmt[j] - '0');
            }
            if (j > fi and num > 0 and j < fs.len and fmt[j] == '$') {
                xpg_slot = @intCast(num - 1);
                fi = j + 1;
                if (fi >= fs.len) break;
            }
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

        // POSIX size modifiers — ``h`` / ``l`` / ``ll`` / ``L``.
        // Tcl 9 accepts these but ignores them at the value level
        // (every integer obj is i64 internally, and the host C
        // varargs distinction is moot in the WASM runtime).  Skip
        // them so ``scan X %lld`` parses as ``%d`` rather than
        // attempting to consume ``l`` as the spec letter.
        while (fi < fs.len and (fmt[fi] == 'h' or fmt[fi] == 'l' or fmt[fi] == 'L')) : (fi += 1) {}
        if (fi >= fs.len) break;

        const spec = fmt[fi];
        fi += 1;

        // %n — number of chars consumed so far (no input consumed).
        if (spec == 'n') {
            const val = obj_new_int(@intCast(si));
            scan_store(objs, &obj_index, &nconversions, val, suppress, xpg_slot);
            continue;
        }

        // ``%[...]`` — character set.  Matches a run of input characters
        // that are members (or, with a leading ``^``, non-members) of the
        // set defined inline in the format string.  Like ``%c`` it does
        // not skip leading whitespace.  Scan-charset quirks: a ``]``
        // immediately after the ``[`` (or after ``^``) is a literal
        // member rather than the close; ``a-z`` is an (order-normalised)
        // range; a ``-`` adjacent to ``]`` is literal.  Matching is
        // byte-wise, consistent with ``%s`` above.
        if (spec == '[') {
            // ``%[`` does not skip whitespace, but it still needs at
            // least one character — running out here is an underflow.
            if (si >= ss.len) {
                underflow = true;
                break;
            }
            var in_set = [_]bool{false} ** 256;
            var negate = false;
            if (fi < fs.len and fmt[fi] == '^') {
                negate = true;
                fi += 1;
            }
            // A ``]`` as the first set member is literal, not the close.
            if (fi < fs.len and fmt[fi] == ']') {
                in_set[']'] = true;
                fi += 1;
            }
            while (fi < fs.len and fmt[fi] != ']') {
                const lo = fmt[fi];
                // Range ``lo-hi`` — but a ``-`` just before ``]`` is literal.
                if (fi + 2 < fs.len and fmt[fi + 1] == '-' and fmt[fi + 2] != ']') {
                    const hi = fmt[fi + 2];
                    const a: u32 = if (lo < hi) lo else hi;
                    const b: u32 = if (lo < hi) hi else lo;
                    var ci: u32 = a;
                    while (ci <= b) : (ci += 1) in_set[ci] = true;
                    fi += 3;
                } else {
                    in_set[lo] = true;
                    fi += 1;
                }
            }
            // Consume the closing ``]`` (an unclosed set implicitly closes
            // at end-of-format, mirroring the glob char-class handling).
            if (fi < fs.len and fmt[fi] == ']') fi += 1;

            const max_chars: u32 = if (width > 0) width else ss.len;
            const start_si = si;
            var taken: u32 = 0;
            while (si < ss.len and taken < max_chars and (in_set[src[si]] != negate)) {
                si += 1;
                taken += 1;
            }
            if (si == start_si) break; // no characters matched — conversion fails (not underflow)
            const val = obj_new_string(@bitCast(ss.ptr + start_si), @bitCast(si - start_si));
            scan_store(objs, &obj_index, &nconversions, val, suppress, xpg_slot);
            continue;
        }

        // At this point a conversion needs at least one input character.
        // ``%c`` consumes the next character verbatim (no whitespace
        // skip); everything else skips leading whitespace first.  Hitting
        // end-of-input either before or after the skip is an underflow.
        if (si >= ss.len) {
            underflow = true;
            break;
        }
        if (spec != 'c') {
            si = skip_space(src, ss.len, si);
            if (si >= ss.len) {
                underflow = true;
                break;
            }
        }

        if (spec == 'c') {
            var tmp_i = si;
            const cp = read_utf8_cp(src, ss.len, &tmp_i);
            const val = obj_new_int(cp);
            scan_store(objs, &obj_index, &nconversions, val, suppress, xpg_slot);
            si = tmp_i;
            continue;
        }

        if (spec == 'd' or spec == 'i' or spec == 'x' or spec == 'X' or
            spec == 'o' or spec == 'u' or spec == 'b')
        {
            const base: i64 = switch (spec) {
                'i' => 0,
                'x', 'X' => 16,
                'o' => 8,
                'b' => 2,
                // ``%d`` and ``%u`` are decimal; we carry full i64 range,
                // so the unsigned/signed distinction only matters for the
                // (currently unhandled) modular-wrap edge.
                else => 10,
            };
            const accept_0x = (spec == 'x' or spec == 'X');
            var matched = false;
            // A width field caps the digits consumed (``%3d``) — bound
            // ``scan_int`` to ``si + width`` rather than the whole input.
            const int_limit: u32 = if (width > 0) @min(ss.len, si + width) else ss.len;
            const val = scan_int(src, int_limit, &si, base, accept_0x, &matched);
            if (!matched) {
                obj_mod_scan.tcl_obj_release(val);
                break;
            }
            scan_store(objs, &obj_index, &nconversions, val, suppress, xpg_slot);
            continue;
        }

        if (spec == 'f' or spec == 'e' or spec == 'g' or spec == 'E' or spec == 'G') {
            // Floating-point.  Capture the float token (optional sign,
            // then digits / ``.`` / exponent, or ``Inf`` / ``Infinity`` /
            // ``NaN``) and box a real ``TYPE_FLOAT`` value.  Earlier this
            // truncated to an integer because the runtime had no fp path,
            // so ``scan 0.2 %f`` returned ``0`` (scan-4.49 / scan-11.*).
            const start_si = si;
            // A width field caps the characters consumed (``%3f``).
            const f_limit: u32 = if (width > 0) @min(ss.len, si + width) else ss.len;
            if (si < f_limit and (src[si] == '-' or src[si] == '+')) si += 1;
            if (si < f_limit and ((src[si] | 0x20) == 'i' or (src[si] | 0x20) == 'n')) {
                // ``inf`` / ``infinity`` / ``nan`` — consume the alpha run.
                while (si < f_limit and (src[si] | 0x20) >= 'a' and (src[si] | 0x20) <= 'z') si += 1;
            } else {
                while (si < f_limit and src[si] >= '0' and src[si] <= '9') si += 1;
                if (si < f_limit and src[si] == '.') {
                    si += 1;
                    while (si < f_limit and src[si] >= '0' and src[si] <= '9') si += 1;
                }
                if (si < f_limit and (src[si] == 'e' or src[si] == 'E')) {
                    // Only consume the exponent when at least one digit
                    // follows (after an optional sign).  An incomplete
                    // ``1e`` keeps ``1`` as the float and leaves ``e`` for
                    // the next conversion (``scan 1e {%f%s}`` → 1 e).
                    var j = si + 1;
                    if (j < f_limit and (src[j] == '+' or src[j] == '-')) j += 1;
                    if (j < f_limit and src[j] >= '0' and src[j] <= '9') {
                        si = j;
                        while (si < f_limit and src[si] >= '0' and src[si] <= '9') si += 1;
                    }
                }
            }
            const tok_len = si - start_si;
            if (tok_len == 0 or tok_len > 64) break;
            var fbuf: [64]u8 = undefined;
            var k: u32 = 0;
            while (k < tok_len) : (k += 1) fbuf[k] = src[start_si + k];
            const fv = std.fmt.parseFloat(f64, fbuf[0..tok_len]) catch break;
            const val = obj_mod_scan.obj_new_float(fv);
            scan_store(objs, &obj_index, &nconversions, val, suppress, xpg_slot);
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
            scan_store(objs, &obj_index, &nconversions, val, suppress, xpg_slot);
            si = end_si;
            continue;
        }

        // Unreachable: validation rejects unsupported specifiers.
        break;
    }

    // When the scan ran out of input before *any* conversion matched,
    // Tcl reports total failure: -1 for the classic form, an empty list
    // for the inline form — regardless of how many slots the format
    // declared.  A plain mismatch (no underflow) instead yields the
    // declared number of empty-filled slots.
    const totally_failed = underflow and nconversions == 0;

    if (has_vars) {
        var result_count: i64 = 0;
        var i: u32 = 0;
        while (i < objs.len) : (i += 1) {
            if (objs[i] != 0) {
                _ = frames.var_set(words[3 + i], objs[i]);
                obj_mod_scan.tcl_obj_release(objs[i]);
                result_count += 1;
            }
        }
        if (totally_failed) return result_mod.from_globals(obj_new_int(-1));
        return result_mod.from_globals(obj_new_int(result_count));
    }

    // Inline (no-vars) form.  When the scan totally failed, the result
    // is an empty list regardless of declared slots.
    if (totally_failed) {
        var i: u32 = 0;
        while (i < objs.len) : (i += 1) {
            if (objs[i] != 0) obj_mod_scan.tcl_obj_release(objs[i]);
        }
        return result_mod.from_globals(obj_new_string(0, 0));
    }

    // Build a list with one element per slot, filling unset slots with
    // empty strings.  We assemble the string-rep directly rather than
    // threading ``tcl_list``: its empty-accumulator fast path collapses a
    // *leading* empty element to "" (scan-13.4 wants ``{} abc``, not
    // ``abc``), and the canonical quoter does not brace empty elements
    // (that ``{}`` special case lives in ``append_list_element``).
    const quote_mod = @import("../valtypes/tcl_list_quote.zig");
    var total_bytes: u32 = 1;
    {
        var i: u32 = 0;
        while (i < objs.len) : (i += 1) {
            if (objs[i] != 0) {
                const sv = obj_ensure_string(objs[i]);
                total_bytes += sv.len * 2 + 3; // quoted element + separator
            } else {
                total_bytes += 3; // "{}" + separator
            }
        }
    }
    const buf = alloc(total_bytes);
    if (buf == 0) {
        var i: u32 = 0;
        while (i < objs.len) : (i += 1) {
            if (objs[i] != 0) obj_mod_scan.tcl_obj_release(objs[i]);
        }
        return result_mod.from_globals(obj_new_string(0, 0));
    }
    var off: u32 = 0;
    {
        var i: u32 = 0;
        while (i < objs.len) : (i += 1) {
            if (off != 0) {
                const d: [*]u8 = @ptrFromInt(buf + off);
                d[0] = ' ';
                off += 1;
            }
            const sv_len: u32 = if (objs[i] != 0) obj_ensure_string(objs[i]).len else 0;
            if (sv_len == 0) {
                const d: [*]u8 = @ptrFromInt(buf + off);
                d[0] = '{';
                d[1] = '}';
                off += 2;
            } else {
                const sv = obj_ensure_string(objs[i]);
                off = if (i == 0)
                    quote_mod.list_elem_quote(buf, off, sv.ptr, sv.len)
                else
                    quote_mod.list_elem_quote_nth(buf, off, sv.ptr, sv.len);
            }
            if (objs[i] != 0) obj_mod_scan.tcl_obj_release(objs[i]);
        }
    }
    return result_mod.from_globals(obj_mod.obj_new_string_take(buf, off, total_bytes));
}

fn raise_scan_error(msg: []const u8) void {
    const catch_mod = @import("../interp/tcl_catch.zig");
    const buf = obj_mod.alloc(@intCast(msg.len));
    if (buf == 0) {
        catch_mod.tcl_cmd_error(obj_mod.obj_new_string(0, 0));
        return;
    }
    const dst: [*]u8 = @ptrFromInt(buf);
    for (msg, 0..) |c, i| dst[i] = c;
    const e = obj_mod.obj_new_string_take(buf, @intCast(msg.len), @intCast(msg.len));
    catch_mod.tcl_cmd_error(e);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "scan", .arity_min = 2, .arity_max = null, .handler = &eval_scan },
};
