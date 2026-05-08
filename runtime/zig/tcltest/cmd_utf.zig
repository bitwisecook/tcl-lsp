// UTF-8 / Unicode character helpers ported from upstream
// ``tclTest.c`` — the suite uses these to validate Tcl 9's
// UTF parsing primitives.  Most map to Zig's ``std.unicode`` directly.
//
// Covered (PORTABLE):
//   * ``testutfnext``    — byte offset of the next code point
//   * ``testutfprev``    — byte offset of the previous code point
//   * ``testnumutfchars`` — count code points up to ``limit`` bytes
//   * ``testgetunichar`` — decode the i-th code point
//   * ``testfindfirst``  — sub-string scan from the start
//   * ``testfindlast``   — sub-string scan from the end
//   * ``testuniclass``   — case mapping + character-class predicates
//
// Each replaces the matching stub row in ``cmd_stubs.zig``.

const std = @import("std");
const obj = @import("../valtypes/tcl_obj.zig");
const result_mod = @import("../interp/tcl_result.zig");
const catch_mod = @import("../interp/tcl_catch.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");

fn build_msg(buf: []const u8) i32 {
    const dst = obj.alloc(@intCast(buf.len));
    if (dst == 0) return 0;
    const dst_p: [*]u8 = @ptrFromInt(dst);
    for (buf, 0..) |c, i| dst_p[i] = c;
    return obj.obj_new_string_take(dst, @intCast(buf.len), @intCast(buf.len));
}

fn err_msg(buf: []const u8) result_mod.InterpResult {
    catch_mod.tcl_cmd_error(build_msg(buf));
    return result_mod.from_globals(0);
}

fn obj_view(handle: i32) []const u8 {
    const s = obj.obj_ensure_string(handle);
    const ptr: [*]const u8 = @ptrFromInt(s.ptr);
    return ptr[0..s.len];
}

// -- testutfnext ---------------------------------------------------------

/// Byte offset of the next UTF-8 code point in *bytes*, starting
/// from byte 0.  Mirrors upstream's ``Tcl_UtfNext(buffer)`` —
/// returns the difference in bytes, capped at the string length.
fn eval_testutfnext(words: []const i32) result_mod.InterpResult {
    if (words.len != 2) return err_msg("wrong # args: testutfnext bytes");
    const view = obj_view(words[1]);
    if (view.len == 0) return result_mod.ok(obj.obj_new_int(0));
    const seq_len = std.unicode.utf8ByteSequenceLength(view[0]) catch 1;
    const adv = @min(@as(u32, seq_len), @as(u32, @intCast(view.len)));
    return result_mod.ok(obj.obj_new_int(@intCast(adv)));
}

// -- testutfprev ---------------------------------------------------------

/// Byte offset of the previous UTF-8 code point — i.e. the start
/// byte of the multi-byte sequence ending at *offset* (default end
/// of string).  Walks backwards looking for a UTF-8 lead byte.
fn eval_testutfprev(words: []const i32) result_mod.InterpResult {
    if (words.len < 2 or words.len > 3) return err_msg("wrong # args: testutfprev bytes ?offset?");
    const view = obj_view(words[1]);
    var off: u32 = @intCast(view.len);
    if (words.len == 3) {
        const o = obj.obj_get_int(words[2]);
        if (o < 0) {
            off = 0;
        } else if (@as(u32, @intCast(o)) > view.len) {
            off = @intCast(view.len);
        } else {
            off = @intCast(o);
        }
    }
    if (off == 0) return result_mod.ok(obj.obj_new_int(0));
    var i = off - 1;
    // Walk back while in continuation bytes (0b10xxxxxx).
    while (i > 0 and (view[i] & 0xC0) == 0x80) i -= 1;
    return result_mod.ok(obj.obj_new_int(@intCast(i)));
}

// -- testnumutfchars -----------------------------------------------------

/// Number of UTF-8 code points in *bytes* up to ``limit`` bytes
/// (default: whole string).
fn eval_testnumutfchars(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) return result_mod.from_globals(0);
    const view = obj_view(words[1]);
    var limit: u32 = @intCast(view.len);
    if (words.len > 2) {
        const lim_i = obj.obj_get_int(words[2]);
        if (lim_i >= 0 and @as(u32, @intCast(lim_i)) < limit) limit = @intCast(lim_i);
    }
    var i: u32 = 0;
    var count: i64 = 0;
    while (i < limit) {
        const seq_len = std.unicode.utf8ByteSequenceLength(view[i]) catch 1;
        const advance = @min(@as(u32, seq_len), limit - i);
        if (advance == 0) break;
        i += advance;
        count += 1;
    }
    return result_mod.ok(obj.obj_new_int(count));
}

// -- testgetunichar ------------------------------------------------------

/// Decode the i-th code point of *string*.  Returns -1 for an
/// out-of-range index, matching upstream's overflow behaviour.
fn eval_testgetunichar(words: []const i32) result_mod.InterpResult {
    if (words.len != 3) return err_msg("wrong # args: testgetunichar string index");
    const view = obj_view(words[1]);
    const idx = obj.obj_get_int(words[2]);
    if (idx < 0) return result_mod.ok(obj.obj_new_int(-1));
    var i: u32 = 0;
    var seen: i64 = 0;
    while (i < view.len) {
        const seq_len = std.unicode.utf8ByteSequenceLength(view[i]) catch 1;
        if (i + seq_len > view.len) break;
        if (seen == idx) {
            const cp = std.unicode.utf8Decode(view[i .. i + seq_len]) catch return result_mod.ok(obj.obj_new_int(-1));
            return result_mod.ok(obj.obj_new_int(@intCast(cp)));
        }
        seen += 1;
        i += seq_len;
    }
    return result_mod.ok(obj.obj_new_int(-1));
}

// -- testfindfirst / testfindlast ----------------------------------------

/// Parse the second argument as either a single character (the byte
/// to scan for) or a numeric code point.  Returns the byte to look
/// for, or null on a multi-byte / out-of-range input.
fn parse_scan_byte(handle: i32) ?u8 {
    const v = obj_view(handle);
    if (v.len == 1) return v[0];
    // Numeric code point — only ASCII codepoints are addressable as
    // a single byte; reject anything else (multi-byte codepoints).
    const cp = obj.try_parse_int(@intCast(@intFromPtr(v.ptr)), @intCast(v.len)) orelse return null;
    if (cp < 0 or cp > 0x7F) return null;
    return @intCast(cp);
}

/// ``testfindfirst bytes char ?length?`` — return the suffix of
/// *bytes* starting at the first occurrence of *char*, or the empty
/// string if not found / *length* is 0.  Mirrors upstream's
/// ``Tcl_UtfFindFirst`` shape on the ASCII subset.
fn eval_testfindfirst(words: []const i32) result_mod.InterpResult {
    if (words.len < 2 or words.len > 4) return err_msg("wrong # args: testfindfirst bytes ?char ?length??");
    const view = obj_view(words[1]);
    if (view.len == 0 or words.len < 3) return result_mod.ok(obj.obj_new_string(0, 0));
    const target = parse_scan_byte(words[2]) orelse return err_msg("testfindfirst: char must be a single byte / ASCII codepoint");
    var limit: u32 = @intCast(view.len);
    if (words.len == 4) {
        const n = obj.obj_get_int(words[3]);
        if (n >= 0 and @as(u32, @intCast(n)) < limit) limit = @intCast(n);
    }
    var i: u32 = 0;
    while (i < limit) : (i += 1) {
        if (view[i] == target) {
            return result_mod.ok(obj.obj_new_string_copy(
                @intCast(@intFromPtr(&view[i])),
                limit - i,
            ));
        }
    }
    return result_mod.ok(obj.obj_new_string(0, 0));
}

/// ``testfindlast bytes char ?length?`` — return the suffix starting
/// at the *last* occurrence of *char* in the first *length* bytes,
/// or the empty string when not found.  Matches upstream's
/// ``Tcl_UtfFindLast`` on the ASCII subset.
fn eval_testfindlast(words: []const i32) result_mod.InterpResult {
    if (words.len < 2 or words.len > 4) return err_msg("wrong # args: testfindlast bytes ?char ?length??");
    const view = obj_view(words[1]);
    if (view.len == 0 or words.len < 3) return result_mod.ok(obj.obj_new_string(0, 0));
    const target = parse_scan_byte(words[2]) orelse return err_msg("testfindlast: char must be a single byte / ASCII codepoint");
    var limit: u32 = @intCast(view.len);
    if (words.len == 4) {
        const n = obj.obj_get_int(words[3]);
        if (n >= 0 and @as(u32, @intCast(n)) < limit) limit = @intCast(n);
    }
    var found: ?u32 = null;
    var i: u32 = 0;
    while (i < limit) : (i += 1) {
        if (view[i] == target) found = i;
    }
    if (found) |idx| {
        return result_mod.ok(obj.obj_new_string_copy(
            @intCast(@intFromPtr(&view[idx])),
            limit - idx,
        ));
    }
    return result_mod.ok(obj.obj_new_string(0, 0));
}

// -- testuniclass --------------------------------------------------------

/// Append *value* to the accumulator at *out_ref*, replacing the
/// accumulator with the new handle when ``tcl_cmd_lappend`` returns
/// a different obj (canonical-rebuild path) and releasing the
/// previous accumulator so it doesn't leak.  ``tcl_cmd_lappend``
/// only reads *value*'s string rep — it doesn't take a reference —
/// so the caller is also responsible for releasing *value* when
/// they own it; that release stays with the call sites.
fn lappend_into(out_ref: *i32, value: i32) void {
    const tcl_list = @import("../valtypes/tcl_list.zig");
    const next = tcl_list.tcl_cmd_lappend(out_ref.*, value);
    if (next != out_ref.*) {
        obj.tcl_obj_release(out_ref.*);
        out_ref.* = next;
    }
}

/// Append a literal class-name token to the accumulator.  Pairs the
/// :fn:`build_msg` allocation with a release after the lappend has
/// consumed its string rep — the previous shape leaked one TclObj
/// per matched class (Copilot review, PR #363).
fn lappend_lit(out_ref: *i32, lit: []const u8) void {
    const tmp = build_msg(lit);
    lappend_into(out_ref, tmp);
    obj.tcl_obj_release(tmp);
}

/// Returns ``[lower upper title class ...]`` where ``classN`` are the
/// matched character classes (lower / upper / alnum / alpha / digit
/// / space / word).  Upstream emits a long list of every matching
/// class — we cover the common ASCII-class subset; non-ASCII code
/// points report only what Zig's ``std.ascii`` can identify.
fn eval_testuniclass(words: []const i32) result_mod.InterpResult {
    if (words.len != 2) return err_msg("wrong # args: testuniclass integer");
    const v = obj.obj_get_int(words[1]);
    if (v < 0 or v > 0x10FFFF) return err_msg("code point out of range");
    const cp: u21 = @intCast(v);

    var out: i32 = obj.obj_new_string(0, 0);

    // case mappings — Zig stdlib doesn't expose unicode case mapping
    // tables; we use ASCII-only mapping which matches upstream output
    // for the ASCII range and is a no-op (returns input) elsewhere.
    const lower: u21 = if (cp >= 'A' and cp <= 'Z') cp + 32 else cp;
    const upper: u21 = if (cp >= 'a' and cp <= 'z') cp - 32 else cp;
    const title: u21 = upper;
    const lower_obj = obj.obj_new_int(@intCast(lower));
    lappend_into(&out, lower_obj);
    obj.tcl_obj_release(lower_obj);
    const upper_obj = obj.obj_new_int(@intCast(upper));
    lappend_into(&out, upper_obj);
    obj.tcl_obj_release(upper_obj);
    const title_obj = obj.obj_new_int(@intCast(title));
    lappend_into(&out, title_obj);
    obj.tcl_obj_release(title_obj);

    if (cp < 0x80) {
        const c: u8 = @intCast(cp);
        if (std.ascii.isLower(c)) lappend_lit(&out, "lower");
        if (std.ascii.isUpper(c)) lappend_lit(&out, "upper");
        if (std.ascii.isAlphanumeric(c)) lappend_lit(&out, "alnum");
        if (std.ascii.isAlphabetic(c)) lappend_lit(&out, "alpha");
        if (std.ascii.isDigit(c)) lappend_lit(&out, "digit");
        if (std.ascii.isWhitespace(c)) lappend_lit(&out, "space");
        if (std.ascii.isAlphanumeric(c) or c == '_') lappend_lit(&out, "word");
    }
    return result_mod.ok(out);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "testutfnext", .arity_min = 1, .arity_max = 1, .handler = &eval_testutfnext },
    .{ .name = "testutfprev", .arity_min = 1, .arity_max = 2, .handler = &eval_testutfprev },
    .{ .name = "testnumutfchars", .arity_min = 0, .arity_max = 2, .handler = &eval_testnumutfchars },
    .{ .name = "testgetunichar", .arity_min = 2, .arity_max = 2, .handler = &eval_testgetunichar },
    .{ .name = "testfindfirst", .arity_min = 0, .arity_max = 2, .handler = &eval_testfindfirst },
    .{ .name = "testfindlast", .arity_min = 0, .arity_max = 2, .handler = &eval_testfindlast },
    .{ .name = "testuniclass", .arity_min = 1, .arity_max = 1, .handler = &eval_testuniclass },
};
