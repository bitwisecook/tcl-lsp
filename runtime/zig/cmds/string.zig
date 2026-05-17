// Tcl ``string`` built-in command.
//
// Extracted from tcl_interp_string.zig.  Registers itself in the
// central command table via the ``registration`` constant so
// tcl_cmd_table.zig can assemble the dispatch array without naming
// this file explicitly in every dispatch switch.

const rt = @import("../tcl_runtime.zig");

const result_mod = @import("../interp/tcl_result.zig");
const obj_ensure_string = rt.obj_ensure_string;
const obj_new_string = rt.obj_new_string;
const obj_new_int = rt.obj_new_int;

const str_eq = @import("../valtypes/tcl_chars.zig").str_eq;

const reg = @import("../dispatch/tcl_cmd_registry.zig");

pub const registration = reg.CmdEntry{
    .name = "string",
    .arity_min = 1,
    .arity_max = null,
    .handler = &eval,
};

// Sub-command arities — mirrors ``core/commands/registry/tcl/string.py``.
// Cross-checked against C Tcl 9.0 ``tclCmdMZ.c`` every ``String*Cmd``
// (``StringCmpOpts`` for compare/equal; the remainder have direct
// ``if (objc != N)`` / ``objc < A || objc > B`` checks).
pub const subcommands: []const reg.SubEntry = &.{
    .{ .name = "bytelength", .arity_min = 1, .arity_max = 1, .handler = &eval },
    .{ .name = "cat", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "compare", .arity_min = 2, .arity_max = 5, .handler = &eval },
    .{ .name = "equal", .arity_min = 2, .arity_max = 5, .handler = &eval },
    .{ .name = "first", .arity_min = 2, .arity_max = 3, .handler = &eval },
    .{ .name = "index", .arity_min = 2, .arity_max = 2, .handler = &eval },
    .{ .name = "insert", .arity_min = 3, .arity_max = 3, .handler = &eval },
    .{ .name = "is", .arity_min = 2, .arity_max = 5, .handler = &eval },
    .{ .name = "last", .arity_min = 2, .arity_max = 3, .handler = &eval },
    .{ .name = "length", .arity_min = 1, .arity_max = 1, .handler = &eval },
    .{ .name = "map", .arity_min = 2, .arity_max = 3, .handler = &eval },
    .{ .name = "match", .arity_min = 2, .arity_max = 3, .handler = &eval },
    .{ .name = "range", .arity_min = 3, .arity_max = 3, .handler = &eval },
    .{ .name = "repeat", .arity_min = 2, .arity_max = 2, .handler = &eval },
    .{ .name = "replace", .arity_min = 3, .arity_max = 4, .handler = &eval },
    .{ .name = "reverse", .arity_min = 1, .arity_max = 1, .handler = &eval },
    .{ .name = "tolower", .arity_min = 1, .arity_max = 3, .handler = &eval },
    .{ .name = "totitle", .arity_min = 1, .arity_max = 3, .handler = &eval },
    .{ .name = "toupper", .arity_min = 1, .arity_max = 3, .handler = &eval },
    .{ .name = "trim", .arity_min = 1, .arity_max = 2, .handler = &eval },
    .{ .name = "trimleft", .arity_min = 1, .arity_max = 2, .handler = &eval },
    .{ .name = "trimright", .arity_min = 1, .arity_max = 2, .handler = &eval },
    .{ .name = "wordend", .arity_min = 2, .arity_max = 2, .handler = &eval },
    .{ .name = "wordstart", .arity_min = 2, .arity_max = 2, .handler = &eval },
};

/// Wording table for the ``wrong # args`` surface raised when a
/// ``string SUB`` call doesn't satisfy the subcommand's arity.
/// Mirrors reference Tcl's per-subcommand ``Tcl_WrongNumArgs``
/// strings — see ``tclCmdMZ.c``'s ``StringCmd`` switch.
const SubArityRule = struct {
    name: []const u8,
    /// Minimum *total* word count (command + args) for the
    /// subcommand to dispatch.  Below this threshold we raise
    /// ``wrong # args`` with ``message`` and bail.
    min_words: u32,
    /// Maximum total word count (or ``null`` for variadic).
    max_words: ?u32,
    message: []const u8,
};

const sub_arity_table: []const SubArityRule = &.{
    .{
        .name = "compare",
        .min_words = 4,
        .max_words = 7,
        .message = "wrong # args: should be \"string compare ?-nocase? ?-length int? string1 string2\"",
    },
    .{
        .name = "equal",
        .min_words = 4,
        .max_words = 7,
        .message = "wrong # args: should be \"string equal ?-nocase? ?-length int? string1 string2\"",
    },
    .{
        .name = "first",
        .min_words = 4,
        .max_words = 5,
        .message = "wrong # args: should be \"string first needleString haystackString ?startIndex?\"",
    },
    .{
        .name = "last",
        .min_words = 4,
        .max_words = 5,
        .message = "wrong # args: should be \"string last needleString haystackString ?lastIndex?\"",
    },
    .{
        .name = "index",
        .min_words = 4,
        .max_words = 4,
        .message = "wrong # args: should be \"string index string charIndex\"",
    },
    .{
        .name = "length",
        .min_words = 3,
        .max_words = 3,
        .message = "wrong # args: should be \"string length string\"",
    },
    .{
        .name = "range",
        .min_words = 5,
        .max_words = 5,
        .message = "wrong # args: should be \"string range string first last\"",
    },
    .{
        .name = "repeat",
        .min_words = 4,
        .max_words = 4,
        .message = "wrong # args: should be \"string repeat string count\"",
    },
    .{
        .name = "replace",
        .min_words = 5,
        .max_words = 6,
        .message = "wrong # args: should be \"string replace string first last ?string?\"",
    },
    .{
        .name = "insert",
        .min_words = 5,
        .max_words = 5,
        .message = "wrong # args: should be \"string insert string index insertString\"",
    },
    .{
        .name = "match",
        .min_words = 4,
        .max_words = 5,
        .message = "wrong # args: should be \"string match ?-nocase? pattern string\"",
    },
    .{
        .name = "is",
        .min_words = 4,
        .max_words = null,
        .message = "wrong # args: should be \"string is class ?-strict? ?-failindex var? str\"",
    },
    .{
        .name = "map",
        .min_words = 4,
        .max_words = 5,
        .message = "wrong # args: should be \"string map ?-nocase? charMap string\"",
    },
    .{
        .name = "tolower",
        .min_words = 3,
        .max_words = 5,
        .message = "wrong # args: should be \"string tolower string ?first? ?last?\"",
    },
    .{
        .name = "toupper",
        .min_words = 3,
        .max_words = 5,
        .message = "wrong # args: should be \"string toupper string ?first? ?last?\"",
    },
    .{
        .name = "totitle",
        .min_words = 3,
        .max_words = 5,
        .message = "wrong # args: should be \"string totitle string ?first? ?last?\"",
    },
    .{
        .name = "trim",
        .min_words = 3,
        .max_words = 4,
        .message = "wrong # args: should be \"string trim string ?chars?\"",
    },
    .{
        .name = "trimleft",
        .min_words = 3,
        .max_words = 4,
        .message = "wrong # args: should be \"string trimleft string ?chars?\"",
    },
    .{
        .name = "trimright",
        .min_words = 3,
        .max_words = 4,
        .message = "wrong # args: should be \"string trimright string ?chars?\"",
    },
    .{
        .name = "reverse",
        .min_words = 3,
        .max_words = 3,
        .message = "wrong # args: should be \"string reverse string\"",
    },
    .{
        .name = "wordstart",
        .min_words = 4,
        .max_words = 4,
        .message = "wrong # args: should be \"string wordstart string index\"",
    },
    .{
        .name = "wordend",
        .min_words = 4,
        .max_words = 4,
        .message = "wrong # args: should be \"string wordend string index\"",
    },
};

/// Validate the call's word count against the subcommand's arity
/// rule.  Returns ``null`` and raises ``wrong # args`` if the call
/// is short / long; returns ``some unit`` when the call is legal.
fn check_subcommand_arity(sp: [*]const u8, sub_len: u32, n_words: usize) ?void {
    for (sub_arity_table) |rule| {
        if (slice_eq_runtime(sp, sub_len, rule.name)) {
            if (n_words < rule.min_words) {
                raise_string_wrong_args(rule.message);
                return null;
            }
            if (rule.max_words) |max| {
                if (n_words > max) {
                    raise_string_wrong_args(rule.message);
                    return null;
                }
            }
            return;
        }
    }
    // Unknown subcommand or one without an arity rule — let the
    // dispatch fall through and either succeed or no-op.
    return;
}

fn slice_eq_runtime(a: [*]const u8, alen: u32, b: []const u8) bool {
    if (alen != b.len) return false;
    for (0..alen) |i| {
        if (a[i] != b[i]) return false;
    }
    return true;
}

/// Parse ``string compare`` / ``string equal`` flags and dispatch
/// through the full-arity runtime helpers.  Accepted flags
/// (matching reference Tcl 9 ``StringCmpOpts`` in tclCmdMZ.c):
///   * ``-nocase`` (no value)
///   * ``-length N`` (positional integer)
/// Stops flag parsing at ``--`` or the first non-flag word; the
/// next two words are the comparison operands.  When the parse
/// fails (unknown flag, missing operand, bad ``-length`` value)
/// we raise the canonical wording and return 0.
fn dispatch_compare(words: []const i32, eq: bool) i32 {
    var i: u32 = 2;
    var nocase: bool = false;
    var len_limit: i32 = -1;
    while (i < words.len) : (i += 1) {
        const w = obj_ensure_string(words[i]);
        if (w.len == 0) break;
        const wp: [*]const u8 = @ptrFromInt(w.ptr);
        if (wp[0] != '-') break;
        if (slice_eq_runtime(wp, w.len, "--")) {
            i += 1;
            break;
        }
        if (slice_eq_runtime(wp, w.len, "-nocase")) {
            nocase = true;
            continue;
        }
        if (slice_eq_runtime(wp, w.len, "-length")) {
            i += 1;
            if (i >= words.len) {
                raise_string_wrong_args(if (eq)
                    "wrong # args: should be \"string equal ?-nocase? ?-length int? string1 string2\""
                else
                    "wrong # args: should be \"string compare ?-nocase? ?-length int? string1 string2\"");
                return obj_new_int(0);
            }
            const lv = rt.obj_get_int(words[i]);
            // Tcl 9 treats ``-length`` <= 0 as "no limit".  Cap at
            // INT32_MAX so the ``string compare`` byte indexing stays
            // within u32 in our impl.
            len_limit = if (lv >= @as(i64, std.math.maxInt(i32)))
                std.math.maxInt(i32)
            else if (lv < 0) -1 else @intCast(lv);
            continue;
        }
        // Unknown flag — surface the wrong # args wording so the
        // caller sees a recognisable diagnostic.
        raise_string_wrong_args(if (eq)
            "wrong # args: should be \"string equal ?-nocase? ?-length int? string1 string2\""
        else
            "wrong # args: should be \"string compare ?-nocase? ?-length int? string1 string2\"");
        return obj_new_int(0);
    }
    if (i + 1 >= words.len) {
        raise_string_wrong_args(if (eq)
            "wrong # args: should be \"string equal ?-nocase? ?-length int? string1 string2\""
        else
            "wrong # args: should be \"string compare ?-nocase? ?-length int? string1 string2\"");
        return obj_new_int(0);
    }
    const a = words[i];
    const b = words[i + 1];
    const nc: i32 = if (nocase) 1 else 0;
    if (eq) return rt.string_equal_full(a, b, nc, len_limit);
    return rt.string_compare_full(a, b, nc, len_limit);
}

const std = @import("std");

fn raise_string_wrong_args(msg: []const u8) void {
    const obj_mod = @import("../valtypes/tcl_obj.zig");
    const catch_mod = @import("../interp/tcl_catch.zig");
    const total: u32 = @intCast(msg.len);
    const buf = obj_mod.alloc(total);
    if (buf == 0) {
        catch_mod.tcl_cmd_error(0);
        return;
    }
    const dst: [*]u8 = @ptrFromInt(buf);
    for (msg, 0..) |c, i| dst[i] = c;
    const m = obj_mod.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(m);
}

pub fn eval(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) {
        // ``string`` with no subcommand raises the canonical
        // ``wrong # args`` (string-1.2: ``string`` → ``wrong # args:
        // should be "string subcommand ?arg ...?"``).
        raise_string_wrong_args(
            "wrong # args: should be \"string subcommand ?arg ...?\"",
        );
        return result_mod.from_globals(0);
    }
    const sub_raw = obj_ensure_string(words[1]);
    // Resolve abbreviated subcommand to its canonical form.  Tcl
    // 9's StringCmd uses TclGetIndexFromObj which accepts any
    // unambiguous prefix (string-2.7 ``string co abcde ABCDE`` →
    // ``string compare``; string-7.4 ``string la …`` →
    // ``string last``).  On ambiguous or unknown input we raise
    // the canonical "bad option" wording.
    var sub_buf: [32]u8 = undefined;
    var sub_ptr: u32 = sub_raw.ptr;
    var sub_len: u32 = sub_raw.len;
    if (resolve_string_sub(sub_raw)) |canon| {
        var k: u32 = 0;
        while (k < canon.len and k < sub_buf.len) : (k += 1) {
            sub_buf[k] = canon[k];
        }
        sub_ptr = @intFromPtr(&sub_buf);
        sub_len = @intCast(canon.len);
    } else {
        // resolve_string_sub already raised the diagnostic.
        return result_mod.from_globals(0);
    }
    const sub: struct { ptr: u32, len: u32 } = .{ .ptr = sub_ptr, .len = sub_len };
    const sp: [*]const u8 = @ptrFromInt(sub.ptr);
    // Per-subcommand arity check — emits the upstream wording so
    // tests like ``string compare`` (arity 0) raise ``wrong # args:
    // should be "string compare ?-nocase? ?-length int? string1
    // string2"`` instead of returning silently (string-2.1, 3.9,
    // 4.1, etc.).
    if (check_subcommand_arity(sp, sub.len, words.len)) |_| {} else {
        // arity error already raised; bail out.
        return result_mod.from_globals(0);
    }
    // ``string cat ?str1? ?str2? …`` — concatenate the string
    // arguments verbatim (no separators).  ``cat`` is the only
    // string subcommand that takes 0+ args, so it has to be
    // dispatched before the ``words.len < 3`` short-circuit
    // below.  WASM-codegen has its own inline path for static
    // calls (see ``cmds/string_.py``); this branch is what the
    // eval-fallback / interpreter path inside coroutine /
    // script bodies hits.  Reproduces in coroutine.test 11.1
    // (``yieldto string cat "PHASE 2"``).
    if (str_eq(sp, sub.len, "cat")) {
        if (words.len == 2) return result_mod.from_globals(obj_new_string(0, 0));
        if (words.len == 3) return result_mod.from_globals(words[2]);
        var total: u32 = 0;
        var i: u32 = 2;
        while (i < words.len) : (i += 1) {
            const ws = obj_ensure_string(words[i]);
            total += ws.len;
        }
        const buf = rt.alloc(total);
        var off: u32 = 0;
        i = 2;
        while (i < words.len) : (i += 1) {
            const ws = obj_ensure_string(words[i]);
            if (ws.len > 0) {
                rt.memcpy(buf + off, ws.ptr, ws.len);
                off += ws.len;
            }
        }
        return result_mod.from_globals(obj_new_string(@bitCast(buf), @bitCast(total)));
    }
    if (words.len < 3) return result_mod.from_globals(0);
    if (str_eq(sp, sub.len, "length")) return result_mod.from_globals(rt.string_length(words[2]));
    if (str_eq(sp, sub.len, "index") and words.len >= 4) {
        if (!is_valid_string_index(words[3])) {
            raise_bad_string_index(words[3]);
            return result_mod.from_globals(0);
        }
        return result_mod.from_globals(rt.string_index(words[2], words[3]));
    }
    if (str_eq(sp, sub.len, "range") and words.len >= 5) {
        if (!is_valid_string_index(words[3])) {
            raise_bad_string_index(words[3]);
            return result_mod.from_globals(0);
        }
        if (!is_valid_string_index(words[4])) {
            raise_bad_string_index(words[4]);
            return result_mod.from_globals(0);
        }
        return result_mod.from_globals(rt.string_range(words[2], words[3], words[4]));
    }
    if (str_eq(sp, sub.len, "compare")) {
        return result_mod.from_globals(eval_compare_or_equal(words, .compare));
    }
    if (str_eq(sp, sub.len, "equal")) {
        return result_mod.from_globals(eval_compare_or_equal(words, .equal));
    }
    if (str_eq(sp, sub.len, "match")) {
        return result_mod.from_globals(eval_string_match(words));
    }
    if (str_eq(sp, sub.len, "map") and words.len >= 4) {
        // ``string map ?-nocase? CHARMAP STRING`` — the trailing two
        // words are always CHARMAP / STRING (per upstream
        // ``StringMapCmd``); any earlier non-final words are options.
        // Accept ``-nocase`` as a prefix abbreviation (``-no`` works,
        // string-10.9.0) and surface anything else as
        // ``bad option "X": must be -nocase`` (string-10.2.0).
        var nocase = false;
        var ai: u32 = 2;
        const opt_end: u32 = words.len - 2;
        while (ai < opt_end) : (ai += 1) {
            const ws = obj_ensure_string(words[ai]);
            const wp: [*]const u8 = @ptrFromInt(ws.ptr);
            if (ws.len >= 2 and wp[0] == '-' and wp[1] == 'n' and is_prefix_of(wp, ws.len, "-nocase")) {
                nocase = true;
                continue;
            }
            raise_bad_option(words[ai], "must be -nocase");
            return result_mod.from_globals(0);
        }
        const map_idx: u32 = words.len - 2;
        // Validate the CHARMAP has an even number of elements —
        // Tcl 9 raises ``char map list unbalanced`` when the
        // pair-count is odd (string-10.10.0).
        const obj_mod_r = @import("../valtypes/tcl_obj.zig");
        const map_str = obj_mod_r.obj_ensure_string(words[map_idx]);
        const map_count = obj_mod_r.list_count_elements(map_str.ptr, map_str.len);
        if (@rem(map_count, 2) != 0) {
            raise_string_wrong_args("char map list unbalanced");
            return result_mod.from_globals(0);
        }
        if (nocase) return result_mod.from_globals(rt.string_map_nocase(words[map_idx], words[map_idx + 1]));
        return result_mod.from_globals(rt.string_map(words[map_idx], words[map_idx + 1]));
    }
    if (str_eq(sp, sub.len, "trim")) {
        const chars = if (words.len >= 4) words[3] else 0;
        return result_mod.from_globals(rt.string_trim(words[2], chars));
    }
    if (str_eq(sp, sub.len, "trimleft")) {
        const chars = if (words.len >= 4) words[3] else 0;
        return result_mod.from_globals(rt.string_trimleft(words[2], chars));
    }
    if (str_eq(sp, sub.len, "trimright")) {
        const chars = if (words.len >= 4) words[3] else 0;
        return result_mod.from_globals(rt.string_trimright(words[2], chars));
    }
    if (str_eq(sp, sub.len, "first") and words.len >= 4) {
        if (words.len < 5) return result_mod.from_globals(rt.string_first(words[2], words[3]));
        if (!is_valid_string_index(words[4])) {
            raise_bad_string_index(words[4]);
            return result_mod.from_globals(0);
        }
        // ``string first`` measures startIndex in codepoints (Tcl 9
        // ``tclCmdMZ.c`` ``StringFirstCmd``).  Resolve ``end``/``end-N``
        // against the haystack codepoint count.
        const list_mod = @import("../valtypes/tcl_list.zig");
        const cp_count: i64 = @intCast(rt.string_codepoint_count(words[3]));
        const start_cp = list_mod.resolve_list_index(words[4], cp_count);
        return result_mod.from_globals(rt.string_first_indexed(words[2], words[3], start_cp));
    }
    if (str_eq(sp, sub.len, "last") and words.len >= 4) {
        if (words.len < 5) return result_mod.from_globals(rt.string_last(words[2], words[3]));
        if (!is_valid_string_index(words[4])) {
            raise_bad_string_index(words[4]);
            return result_mod.from_globals(0);
        }
        const list_mod = @import("../valtypes/tcl_list.zig");
        const cp_count: i64 = @intCast(rt.string_codepoint_count(words[3]));
        const last_cp = list_mod.resolve_list_index(words[4], cp_count);
        return result_mod.from_globals(rt.string_last_indexed(words[2], words[3], last_cp));
    }
    if (str_eq(sp, sub.len, "toupper") or str_eq(sp, sub.len, "tolower") or str_eq(sp, sub.len, "totitle")) {
        // Tcl 9 ``string tolower/toupper/totitle STRING ?first ?last??`` —
        // if only *first* is given, *last* defaults to *first* (a single
        // codepoint).  If neither is given, the whole string is affected.
        // We thread a special ``0`` sentinel through the range helpers to
        // mean "whole string"; the dispatch here picks the right pair.
        const has_first = words.len >= 4;
        const has_last = words.len >= 5;
        const first: i32 = if (has_first) blk: {
            if (!is_valid_string_index(words[3])) {
                raise_bad_string_index(words[3]);
                return result_mod.from_globals(0);
            }
            break :blk words[3];
        } else 0;
        const last: i32 = if (has_last) blk: {
            if (!is_valid_string_index(words[4])) {
                raise_bad_string_index(words[4]);
                return result_mod.from_globals(0);
            }
            break :blk words[4];
        } else if (has_first) first else 0;
        if (!has_first) {
            // Whole-string variant — original helpers.
            if (str_eq(sp, sub.len, "toupper")) return result_mod.from_globals(rt.string_toupper(words[2]));
            if (str_eq(sp, sub.len, "tolower")) return result_mod.from_globals(rt.string_tolower(words[2]));
            return result_mod.from_globals(rt.string_totitle(words[2]));
        }
        if (str_eq(sp, sub.len, "toupper")) return result_mod.from_globals(rt.string_toupper_range(words[2], first, last));
        if (str_eq(sp, sub.len, "tolower")) return result_mod.from_globals(rt.string_tolower_range(words[2], first, last));
        return result_mod.from_globals(rt.string_totitle_range(words[2], first, last));
    }
    if (str_eq(sp, sub.len, "reverse")) return result_mod.from_globals(rt.string_reverse(words[2]));
    if (str_eq(sp, sub.len, "repeat") and words.len >= 4) {
        // ``string repeat string count`` — count must parse as an
        // integer (the runtime helper ``string_repeat`` reads via
        // ``obj_get_int`` which silently returns 0 for garbage).
        const obj_mod_r = @import("../valtypes/tcl_obj.zig");
        const cs = obj_mod_r.obj_ensure_string(words[3]);
        if (obj_mod_r.try_parse_int(cs.ptr, cs.len) == null) {
            raise_expected_integer(words[3]);
            return result_mod.from_globals(0);
        }
        return result_mod.from_globals(rt.string_repeat(words[2], words[3]));
    }
    if (str_eq(sp, sub.len, "replace") and words.len >= 5) {
        if (!is_valid_string_index(words[3])) {
            raise_bad_string_index(words[3]);
            return result_mod.from_globals(0);
        }
        if (!is_valid_string_index(words[4])) {
            raise_bad_string_index(words[4]);
            return result_mod.from_globals(0);
        }
        const replace_arg: i32 = if (words.len >= 6) words[5] else obj_new_string(0, 0);
        return result_mod.from_globals(rt.string_replace(words[2], words[3], words[4], replace_arg));
    }
    if (str_eq(sp, sub.len, "insert") and words.len >= 5) {
        if (!is_valid_string_index(words[3])) {
            raise_bad_string_index(words[3]);
            return result_mod.from_globals(0);
        }
        return result_mod.from_globals(rt.string_insert(words[2], words[3], words[4]));
    }
    if (str_eq(sp, sub.len, "is")) {
        return eval_string_is(words);
    }
    return result_mod.from_globals(0);
}

// ``string is`` class names in the canonical order Tcl 9 uses for
// the ``bad class`` / ``ambiguous class`` error wording (test
// string-6.5 / 6.6).  This is *not* alphabetical — it groups
// related classes (e.g. ``boolean`` between the character and
// digit groups; ``true`` / ``false`` between numeric and case
// classes).  Order is significant because (a) the unique-prefix
// matcher in ``resolve_is_class`` uses it to detect ambiguity and
// (b) the suffix in ``raise_class_error`` quotes the same list
// verbatim.
const STRING_SUBS: []const []const u8 = &.{
    "bytelength", "cat",    "compare",  "equal",     "first",   "index",
    "insert",     "is",     "last",     "length",    "map",     "match",
    "range",      "repeat", "replace",  "reverse",   "tolower", "totitle",
    "toupper",    "trim",   "trimleft", "trimright", "wordend", "wordstart",
};

const StringSpan = struct { ptr: u32, len: u32 };

/// Resolve a possibly-abbreviated string subcommand name to its
/// canonical form.  Returns null after raising ``bad option`` /
/// ``ambiguous option`` for unknown / ambiguous input — matching
/// reference Tcl 9's TclGetIndexFromObj-based dispatch.
fn resolve_string_sub(name: anytype) ?[]const u8 {
    if (name.len == 0 or name.ptr == 0) {
        raise_bad_string_sub_obj_empty();
        return null;
    }
    const ns: [*]const u8 = @ptrFromInt(name.ptr);
    var match: ?[]const u8 = null;
    var ambiguous: bool = false;
    for (STRING_SUBS) |sub| {
        if (name.len > sub.len) continue;
        var ok: bool = true;
        var k: u32 = 0;
        while (k < name.len) : (k += 1) {
            if (ns[k] != sub[k]) {
                ok = false;
                break;
            }
        }
        if (!ok) continue;
        if (name.len == sub.len) return sub;
        if (match == null) {
            match = sub;
        } else {
            ambiguous = true;
        }
    }
    if (ambiguous) {
        raise_bad_string_sub(name, true);
        return null;
    }
    if (match) |m| return m;
    raise_bad_string_sub(name, false);
    return null;
}

fn raise_bad_string_sub_obj_empty() void {
    const catch_mod = @import("../interp/tcl_catch.zig");
    const obj_mod = @import("../valtypes/tcl_obj.zig");
    const msg_text: []const u8 = "bad option \"\": must be bytelength, cat, compare, equal, first, index, insert, is, last, length, map, match, range, repeat, replace, reverse, tolower, totitle, toupper, trim, trimleft, trimright, wordend, or wordstart";
    const m = obj_mod.obj_new_string_copy(@intFromPtr(msg_text.ptr), @intCast(msg_text.len));
    catch_mod.tcl_cmd_error(m);
}

fn raise_bad_string_sub(name: anytype, ambiguous: bool) void {
    const obj_mod = @import("../valtypes/tcl_obj.zig");
    const catch_mod = @import("../interp/tcl_catch.zig");
    const prefix: []const u8 = if (ambiguous) "ambiguous option \"" else "bad option \"";
    const suffix: []const u8 = "\": must be bytelength, cat, compare, equal, first, index, insert, is, last, length, map, match, range, repeat, replace, reverse, tolower, totitle, toupper, trim, trimleft, trimright, wordend, or wordstart";
    const total: u32 = @intCast(prefix.len + name.len + suffix.len);
    const buf = obj_mod.alloc(total);
    if (buf == 0) {
        catch_mod.tcl_cmd_error(0);
        return;
    }
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    if (name.len > 0 and name.ptr != 0) {
        const src: [*]const u8 = @ptrFromInt(name.ptr);
        var k: u32 = 0;
        while (k < name.len) : (k += 1) {
            dst[off + k] = src[k];
        }
        off += @intCast(name.len);
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const m = obj_mod.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(m);
}

const IS_CLASSES: []const []const u8 = &.{
    "alnum", "alpha",       "ascii",    "control", "boolean", "dict",
    "digit", "double",      "entier",   "false",   "graph",   "integer",
    "list",  "lower",       "print",    "punct",   "space",   "true",
    "upper", "wideinteger", "wordchar", "xdigit",
};

/// Match the candidate class name against the canonical class list
/// using Tcl's "unique-prefix" rule.  Returns the canonical name on
/// match, ``null`` on bad / ambiguous class — in those cases the
/// caller has already raised the matching diagnostic.
fn resolve_is_class(name: []const u8) ?[]const u8 {
    var match: ?[]const u8 = null;
    var ambiguous = false;
    for (IS_CLASSES) |c| {
        if (name.len > c.len) continue;
        var ok = true;
        for (0..name.len) |k| {
            if (name[k] != c[k]) {
                ok = false;
                break;
            }
        }
        if (!ok) continue;
        if (name.len == c.len) {
            return c;
        }
        if (match == null) {
            match = c;
        } else {
            ambiguous = true;
        }
    }
    if (ambiguous) {
        raise_class_error(name, "ambiguous class");
        return null;
    }
    if (match) |m| return m;
    raise_class_error(name, "bad class");
    return null;
}

fn raise_class_error(name: []const u8, kind: []const u8) void {
    const obj_mod = @import("../valtypes/tcl_obj.zig");
    const catch_mod = @import("../interp/tcl_catch.zig");
    const suffix = "\": must be alnum, alpha, ascii, control, boolean, dict, digit, double, entier, false, graph, integer, list, lower, print, punct, space, true, upper, wideinteger, wordchar, or xdigit";
    const total: u32 = @intCast(kind.len + 2 + name.len + suffix.len);
    const buf = obj_mod.alloc(total);
    if (buf == 0) {
        catch_mod.tcl_cmd_error(0);
        return;
    }
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: usize = 0;
    for (kind) |c| {
        dst[off] = c;
        off += 1;
    }
    dst[off] = ' ';
    off += 1;
    dst[off] = '"';
    off += 1;
    for (name) |c| {
        dst[off] = c;
        off += 1;
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const m = obj_mod.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(m);
}

/// True when ``arg[0..arg_len]`` is a non-empty prefix of *full*.
/// Used for option-abbreviation matching (``-fail`` → ``-failindex``).
fn is_prefix_of(arg: [*]const u8, arg_len: u32, full: []const u8) bool {
    if (arg_len == 0 or arg_len > full.len) return false;
    var k: u32 = 0;
    while (k < arg_len) : (k += 1) {
        if (arg[k] != full[k]) return false;
    }
    return true;
}

const CompareKind = enum { compare, equal };

/// ``string compare ?-nocase? ?-length N? string1 string2`` and
/// ``string equal`` share the same option-parsing surface.  Both
/// accept option-prefix abbreviation (``-noc`` → ``-nocase``,
/// ``-l`` → ``-length``).
fn eval_compare_or_equal(words: []const i32, kind: CompareKind) i32 {
    const wrong_args = if (kind == .compare)
        "wrong # args: should be \"string compare ?-nocase? ?-length int? string1 string2\""
    else
        "wrong # args: should be \"string equal ?-nocase? ?-length int? string1 string2\"";
    // Need at minimum: string compare a b → 4 words.
    if (words.len < 4) {
        raise_string_wrong_args(wrong_args);
        return obj_new_int(0);
    }
    // Reference Tcl's StringCmpCmd reserves the LAST TWO words as the
    // comparison operands; option parsing only runs over the words in
    // between.  ``string compare -1 -1`` therefore treats the
    // leading ``-1`` as a string, NOT as a bad option.  When fewer
    // than two trailing strings are available, we still surface
    // ``wrong # args`` (too few operands) rather than ``bad option``.
    var nocase: i32 = 0;
    var len_limit: i32 = -1;
    var ai: u32 = 2;
    const opt_end: u32 = words.len - 2;
    while (ai < opt_end) {
        const a = obj_ensure_string(words[ai]);
        if (a.len == 0 or a.ptr == 0) break;
        const ap: [*]const u8 = @ptrFromInt(a.ptr);
        if (ap[0] != '-') break;
        // ``--`` ends option processing (Tcl 9 norm).
        if (a.len == 2 and ap[1] == '-') {
            ai += 1;
            break;
        }
        if (a.len >= 2 and ap[1] == 'n' and is_prefix_of(ap, a.len, "-nocase")) {
            nocase = 1;
            ai += 1;
            continue;
        }
        if (a.len >= 2 and ap[1] == 'l' and is_prefix_of(ap, a.len, "-length")) {
            if (ai + 1 >= words.len) {
                raise_string_wrong_args(wrong_args);
                return obj_new_int(0);
            }
            const lv = obj_ensure_string(words[ai + 1]);
            if (lv.len == 0 or lv.ptr == 0) {
                raise_expected_integer(words[ai + 1]);
                return obj_new_int(0);
            }
            const lp: [*]const u8 = @ptrFromInt(lv.ptr);
            // Parse optional sign + digits.
            var neg = false;
            var k: u32 = 0;
            if (lp[0] == '+') {
                k = 1;
            } else if (lp[0] == '-') {
                neg = true;
                k = 1;
            }
            if (k >= lv.len) {
                raise_expected_integer(words[ai + 1]);
                return obj_new_int(0);
            }
            var n: i64 = 0;
            while (k < lv.len) : (k += 1) {
                if (lp[k] < '0' or lp[k] > '9') {
                    raise_expected_integer(words[ai + 1]);
                    return obj_new_int(0);
                }
                n = n * 10 + (lp[k] - '0');
            }
            if (neg) n = -n;
            // Clamp to i32 range used by string_compare_full.  Negative
            // / over-long becomes "unlimited" sentinel (-1).
            if (n < 0 or n > std.math.maxInt(i31)) {
                len_limit = -1;
            } else {
                len_limit = @intCast(n);
            }
            ai += 2;
            continue;
        }
        // Unknown option — reference Tcl raises ``bad option "X":
        // must be -nocase or -length``.
        raise_bad_option(words[ai], if (kind == .compare)
            "must be -nocase or -length"
        else
            "must be -nocase or -length");
        return obj_new_int(0);
    }
    // After option parsing we should be at the two trailing
    // operand words.  ``opt_end`` was words.len - 2 so the only
    // way ``ai != opt_end`` is when option processing stopped
    // early on a non-flag word (handled by the ``--`` break above
    // or by a non-``-`` word at the start of the option region).
    // Reference Tcl emits ``bad option "X"`` for the first such
    // word — string-2.2 (``string compare a b c``) wants ``bad
    // option "a"``.
    if (ai > opt_end) {
        // Option processing consumed too many words and we no longer
        // have two trailing operand strings — string-2.5 (``string
        // compare -length 10 10`` should raise ``wrong # args``,
        // not ``bad option "10"``).  ``ai`` overshooting opt_end is
        // the canonical signal for this shape.
        raise_string_wrong_args(wrong_args);
        return obj_new_int(0);
    }
    if (ai != opt_end) {
        raise_bad_option(words[ai], "must be -nocase or -length");
        return obj_new_int(0);
    }
    // ``ai`` lands on opt_end (== words.len - 2) so the two operands
    // are the last two words.
    const cmp = rt.string_compare_full(words[opt_end], words[opt_end + 1], nocase, len_limit);
    if (kind == .equal) {
        // string equal returns 1 when cmp is "equal" (string_compare
        // returns 0 for equal strings).
        const v = rt.obj_get_int(cmp);
        return obj_new_int(if (v == 0) 1 else 0);
    }
    return cmp;
}

/// Validate *idx* as a Tcl string-index argument.  Accepts integers
/// (with optional ``+``/``-`` sign, possibly hex / octal), ``end``,
/// ``end-N``, ``end+N``, and the ``int+int`` / ``int-int`` arithmetic
/// forms the C tcl parser accepts.  Returns ``true`` on a valid
/// index, ``false`` otherwise.  Used by ``string first`` / ``string
/// last`` / ``string range`` / ``string replace`` / ``string index``
/// / ``string repeat`` to reject garbage indices with the canonical
/// ``bad index "X": must be integer?[+-]integer? or end?[+-]integer?``
/// diagnostic rather than silently treating them as 0.
fn is_valid_string_index(idx: i32) bool {
    const obj_mod = @import("../valtypes/tcl_obj.zig");
    const s = obj_mod.obj_ensure_string(idx);
    if (s.len == 0) return false;
    const sp: [*]const u8 = @ptrFromInt(s.ptr);
    // Allow ``end`` / ``end-N`` / ``end+N`` (plus optional arithmetic
    // tail like ``end-1+0`` which C tcl folds at parse time).
    if (s.len >= 3 and sp[0] == 'e' and sp[1] == 'n' and sp[2] == 'd') {
        if (s.len == 3) return true;
        // ``end+N`` / ``end-N`` — N is an integer literal (digits +
        // optional further ``±N`` arithmetic chain).
        if (sp[3] != '+' and sp[3] != '-') return false;
        return is_int_arith_tail(sp, s.len, 4);
    }
    // Pure integer arithmetic: optional sign, digits, optional
    // ``±N`` continuation.
    var i: u32 = 0;
    if (sp[i] == '+' or sp[i] == '-') i += 1;
    if (i >= s.len) return false;
    return is_int_arith_tail(sp, s.len, i);
}

fn is_int_arith_tail(sp: [*]const u8, len: u32, start: u32) bool {
    var i = start;
    // First digit run (required at least 1 digit).
    if (i >= len) return false;
    if (!(sp[i] >= '0' and sp[i] <= '9')) return false;
    while (i < len and sp[i] >= '0' and sp[i] <= '9') i += 1;
    // Optional ``±N`` continuation runs.
    while (i < len) {
        if (sp[i] != '+' and sp[i] != '-') return false;
        i += 1;
        if (i >= len or !(sp[i] >= '0' and sp[i] <= '9')) return false;
        while (i < len and sp[i] >= '0' and sp[i] <= '9') i += 1;
    }
    return true;
}

fn raise_bad_string_index(idx: i32) void {
    const obj_mod = @import("../valtypes/tcl_obj.zig");
    const catch_mod = @import("../interp/tcl_catch.zig");
    const s = obj_mod.obj_ensure_string(idx);
    const prefix = "bad index \"";
    const suffix = "\": must be integer?[+-]integer? or end?[+-]integer?";
    const total: u32 = @intCast(prefix.len + s.len + suffix.len);
    const buf = obj_mod.alloc(total);
    if (buf == 0) {
        catch_mod.tcl_cmd_error(0);
        return;
    }
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    if (s.len > 0 and s.ptr != 0) {
        const src: [*]const u8 = @ptrFromInt(s.ptr);
        var k: u32 = 0;
        while (k < s.len) : (k += 1) {
            dst[off + k] = src[k];
        }
        off += s.len;
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const m = obj_mod.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(m);
}

fn raise_expected_integer(operand: i32) void {
    const obj_mod = @import("../valtypes/tcl_obj.zig");
    const catch_mod = @import("../interp/tcl_catch.zig");
    const s = obj_mod.obj_ensure_string(operand);
    const prefix = "expected integer but got \"";
    const suffix = "\"";
    const total: u32 = @intCast(prefix.len + s.len + suffix.len);
    const buf = obj_mod.alloc(total);
    if (buf == 0) {
        catch_mod.tcl_cmd_error(0);
        return;
    }
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    if (s.len > 0 and s.ptr != 0) {
        const src: [*]const u8 = @ptrFromInt(s.ptr);
        var k: u32 = 0;
        while (k < s.len) : (k += 1) {
            dst[off + k] = src[k];
        }
        off += s.len;
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const m = obj_mod.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(m);
}

fn raise_bad_option(option: i32, expected_text: []const u8) void {
    const obj_mod = @import("../valtypes/tcl_obj.zig");
    const catch_mod = @import("../interp/tcl_catch.zig");
    const s = obj_mod.obj_ensure_string(option);
    const prefix = "bad option \"";
    const middle = "\": ";
    const total: u32 = @intCast(prefix.len + s.len + middle.len + expected_text.len);
    const buf = obj_mod.alloc(total);
    if (buf == 0) {
        catch_mod.tcl_cmd_error(0);
        return;
    }
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    if (s.len > 0 and s.ptr != 0) {
        const src: [*]const u8 = @ptrFromInt(s.ptr);
        var k: u32 = 0;
        while (k < s.len) : (k += 1) {
            dst[off + k] = src[k];
        }
        off += s.len;
    }
    for (middle) |c| {
        dst[off] = c;
        off += 1;
    }
    for (expected_text) |c| {
        dst[off] = c;
        off += 1;
    }
    const m = obj_mod.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(m);
}

/// ``string match ?-nocase? pattern string`` — glob match with
/// option-prefix abbreviation for ``-nocase``.  Like
/// :func:`eval_compare_or_equal`, the last two words are the
/// pattern and the string (reserved by reference Tcl's
/// StringMatchCmd), so ``-`` prefixes between them don't get
/// rejected as bad options.
fn eval_string_match(words: []const i32) i32 {
    const wrong_args = "wrong # args: should be \"string match ?-nocase? pattern string\"";
    if (words.len < 4 or words.len > 5) {
        raise_string_wrong_args(wrong_args);
        return obj_new_int(0);
    }
    var nocase: bool = false;
    var pat_idx: u32 = 2;
    if (words.len == 5) {
        const a = obj_ensure_string(words[2]);
        if (a.len == 0 or a.ptr == 0) {
            raise_string_wrong_args(wrong_args);
            return obj_new_int(0);
        }
        const ap: [*]const u8 = @ptrFromInt(a.ptr);
        if (ap[0] == '-' and a.len >= 2 and ap[1] == 'n' and is_prefix_of(ap, a.len, "-nocase")) {
            nocase = true;
            pat_idx = 3;
        } else {
            raise_bad_option(words[2], "must be -nocase");
            return obj_new_int(0);
        }
    }
    const pat = obj_ensure_string(words[pat_idx]);
    const val = obj_ensure_string(words[pat_idx + 1]);
    if (nocase) {
        return obj_new_int(if (glob_match_nocase(pat.ptr, pat.len, val.ptr, val.len)) @as(i64, 1) else 0);
    }
    return obj_new_int(if (glob_match_plain(pat.ptr, pat.len, val.ptr, val.len)) @as(i64, 1) else 0);
}

fn glob_match_plain(pp: u32, plen: u32, vp: u32, vlen: u32) bool {
    const tcl_string = @import("../valtypes/tcl_string.zig");
    return tcl_string.glob_match(pp, plen, vp, vlen);
}

fn glob_match_nocase(pp: u32, plen: u32, vp: u32, vlen: u32) bool {
    // Lower-case both pattern and value into bump-allocated scratch
    // buffers, then run the regular glob_match.  Avoids touching the
    // underlying string objs (the case fold is per-call).
    if (plen == 0) return vlen == 0;
    const obj_mod = @import("../valtypes/tcl_obj.zig");
    const buf_pat = obj_mod.alloc(if (plen == 0) 1 else plen);
    const buf_val = if (vlen > 0) obj_mod.alloc(vlen) else 0;
    if (buf_pat == 0 or (vlen > 0 and buf_val == 0)) {
        if (buf_pat != 0) obj_mod.free_sized(buf_pat, plen);
        if (buf_val != 0) obj_mod.free_sized(buf_val, vlen);
        // Fallback: case-sensitive match.
        return glob_match_plain(pp, plen, vp, vlen);
    }
    const dp: [*]u8 = @ptrFromInt(buf_pat);
    const sp: [*]const u8 = @ptrFromInt(pp);
    var i: u32 = 0;
    while (i < plen) : (i += 1) {
        dp[i] = if (sp[i] >= 'A' and sp[i] <= 'Z') sp[i] + 32 else sp[i];
    }
    if (vlen > 0) {
        const dv: [*]u8 = @ptrFromInt(buf_val);
        const sv: [*]const u8 = @ptrFromInt(vp);
        i = 0;
        while (i < vlen) : (i += 1) {
            dv[i] = if (sv[i] >= 'A' and sv[i] <= 'Z') sv[i] + 32 else sv[i];
        }
    }
    const r = glob_match_plain(buf_pat, plen, buf_val, vlen);
    obj_mod.free_sized(buf_pat, plen);
    if (vlen > 0) obj_mod.free_sized(buf_val, vlen);
    return r;
}

fn eval_string_is(words: []const i32) result_mod.InterpResult {
    // Arity contract: ``string is class ?-strict? ?-failindex var? str``.
    // ``words[0] = string``, ``words[1] = is``, ``words[2] = class``.
    // The trailing argument is the candidate string; everything in
    // between must match the documented flag set.
    if (words.len < 4) {
        raise_string_wrong_args(
            "wrong # args: should be \"string is class ?-strict? ?-failindex var? str\"",
        );
        // Match the rest of the error paths in this file — returning 0
        // (false) keeps the result consistent with the standard error
        // sentinel even if the caller's snapshot somehow drops the
        // error flag.
        return result_mod.from_globals(obj_new_int(0));
    }
    const cls_arg = obj_ensure_string(words[2]);
    // ``obj_ensure_string`` can return ``ptr=0`` for an empty string
    // / null obj handle.  Treat the empty class name as a bad class.
    if (cls_arg.ptr == 0 or cls_arg.len == 0) {
        raise_class_error("", "bad class");
        return result_mod.from_globals(obj_new_int(0));
    }
    const cls_slice: []const u8 = (@as([*]const u8, @ptrFromInt(cls_arg.ptr)))[0..cls_arg.len];
    const class_name = resolve_is_class(cls_slice) orelse return result_mod.from_globals(obj_new_int(0));

    var strict = false;
    var failindex_var: i32 = 0;
    var i: u32 = 3;
    while (i + 1 < words.len) {
        const a = obj_ensure_string(words[i]);
        const ap: [*]const u8 = @ptrFromInt(a.ptr);
        if (a.len == 0 or ap[0] != '-') break;
        // ``-strict`` / ``-failindex`` accept unambiguous prefixes
        // (reference Tcl's TclGetIndexFromObj allows option-prefix
        // matching).  ``-fail`` / ``-faili`` / etc. all map to
        // ``-failindex``; ``-s`` / ``-str`` map to ``-strict``.
        // The two flags share no prefix beyond ``-`` so any
        // ``-f...`` resolves uniquely to ``-failindex`` and any
        // ``-s...`` to ``-strict``.
        if (a.len >= 2 and ap[1] == 's' and is_prefix_of(ap, a.len, "-strict")) {
            strict = true;
            i += 1;
            continue;
        }
        if (a.len >= 2 and ap[1] == 'f' and is_prefix_of(ap, a.len, "-failindex")) {
            if (i + 1 >= words.len) {
                raise_string_wrong_args(
                    "wrong # args: should be \"string is class ?-strict? ?-failindex var? str\"",
                );
                return result_mod.from_globals(obj_new_int(0));
            }
            failindex_var = words[i + 1];
            i += 2;
            continue;
        }
        // Unknown flag — Tcl 9's StringIsCmd substitutes the
        // resolved class name into the diagnostic for this path
        // (string-6.3.0: ``string is alpha -failin str`` →
        // ``string is alpha ?...``).
        raise_string_is_args(class_name);
        return result_mod.from_globals(obj_new_int(0));
    }
    if (i + 1 != words.len) {
        // Trailing-arg count mismatch after option parsing.  C tcl
        // splits the wording by direction:
        //   - "missing candidate string" (i > words.len-1, i.e. no
        //     trailing arg at all) uses the resolved class name
        //     (string-6.3.0).
        //   - "extra trailing args" (i < words.len-1, more than one
        //     trailing word remains) uses the generic ``class``
        //     placeholder (string-6.4.0).
        if (i >= words.len) {
            raise_string_is_args(class_name);
        } else {
            raise_string_is_args("class");
        }
        return result_mod.from_globals(obj_new_int(0));
    }
    const sv = obj_ensure_string(words[i]);

    // Empty input: non-strict accepts every class as 1.  Strict
    // rejects.  ``-failindex`` set to -1 in either case (no failing
    // character to point at).
    if (sv.len == 0 or sv.ptr == 0) {
        const result_value: i32 = if (strict) 0 else 1;
        if (failindex_var != 0 and result_value == 0) {
            store_failindex(failindex_var, -1);
        }
        return result_mod.from_globals(obj_new_int(result_value));
    }
    const svp: [*]const u8 = @ptrFromInt(sv.ptr);

    // Per-class checks.  Each branch returns the (truth, fail-index)
    // pair; ``fail_index`` is < 0 when the class accepts the input.
    var fail_index: i64 = -1;
    var ok: bool = false;
    if (slice_eq(class_name.ptr, @intCast(class_name.len), "alnum")) {
        ok = check_class_byte(svp, sv.len, &fail_index, isAlnum);
    } else if (slice_eq(class_name.ptr, @intCast(class_name.len), "alpha")) {
        ok = check_class_byte(svp, sv.len, &fail_index, isAlpha);
    } else if (slice_eq(class_name.ptr, @intCast(class_name.len), "ascii")) {
        ok = check_class_byte(svp, sv.len, &fail_index, isAscii);
    } else if (slice_eq(class_name.ptr, @intCast(class_name.len), "control")) {
        ok = check_class_byte(svp, sv.len, &fail_index, isControl);
    } else if (slice_eq(class_name.ptr, @intCast(class_name.len), "digit")) {
        ok = check_class_byte(svp, sv.len, &fail_index, isDigit);
    } else if (slice_eq(class_name.ptr, @intCast(class_name.len), "graph")) {
        ok = check_class_byte(svp, sv.len, &fail_index, isGraph);
    } else if (slice_eq(class_name.ptr, @intCast(class_name.len), "lower")) {
        ok = check_class_byte(svp, sv.len, &fail_index, isLower);
    } else if (slice_eq(class_name.ptr, @intCast(class_name.len), "print")) {
        ok = check_class_byte(svp, sv.len, &fail_index, isPrint);
    } else if (slice_eq(class_name.ptr, @intCast(class_name.len), "punct")) {
        ok = check_class_byte(svp, sv.len, &fail_index, isPunct);
    } else if (slice_eq(class_name.ptr, @intCast(class_name.len), "space")) {
        ok = check_class_byte(svp, sv.len, &fail_index, isSpace);
    } else if (slice_eq(class_name.ptr, @intCast(class_name.len), "upper")) {
        ok = check_class_byte(svp, sv.len, &fail_index, isUpper);
    } else if (slice_eq(class_name.ptr, @intCast(class_name.len), "wordchar")) {
        ok = check_class_byte(svp, sv.len, &fail_index, isWordchar);
    } else if (slice_eq(class_name.ptr, @intCast(class_name.len), "xdigit")) {
        ok = check_class_byte(svp, sv.len, &fail_index, isXdigit);
    } else if (slice_eq(class_name.ptr, @intCast(class_name.len), "boolean")) {
        ok = check_boolean(svp, sv.len);
        if (!ok) fail_index = 0;
    } else if (slice_eq(class_name.ptr, @intCast(class_name.len), "true")) {
        ok = check_boolean_value(svp, sv.len, true);
        if (!ok) fail_index = 0;
    } else if (slice_eq(class_name.ptr, @intCast(class_name.len), "false")) {
        ok = check_boolean_value(svp, sv.len, false);
        if (!ok) fail_index = 0;
    } else if (slice_eq(class_name.ptr, @intCast(class_name.len), "integer") or
        slice_eq(class_name.ptr, @intCast(class_name.len), "wideinteger") or
        slice_eq(class_name.ptr, @intCast(class_name.len), "entier"))
    {
        ok = check_integer(svp, sv.len, &fail_index);
    } else if (slice_eq(class_name.ptr, @intCast(class_name.len), "double")) {
        ok = check_double(svp, sv.len, &fail_index);
    } else if (slice_eq(class_name.ptr, @intCast(class_name.len), "list")) {
        ok = check_list(sv.ptr, sv.len, &fail_index);
    } else if (slice_eq(class_name.ptr, @intCast(class_name.len), "dict")) {
        ok = check_dict(sv.ptr, sv.len, &fail_index);
    } else {
        ok = false;
    }

    const result_value: i32 = if (ok) 1 else 0;
    if (!ok and failindex_var != 0) {
        store_failindex(failindex_var, fail_index);
    }
    return result_mod.from_globals(obj_new_int(result_value));
}

fn raise_string_is_args(class_name: []const u8) void {
    const obj_mod = @import("../valtypes/tcl_obj.zig");
    const catch_mod = @import("../interp/tcl_catch.zig");
    const prefix = "wrong # args: should be \"string is ";
    const suffix = " ?-strict? ?-failindex var? str\"";
    const total: u32 = @intCast(prefix.len + class_name.len + suffix.len);
    const buf = obj_mod.alloc(total);
    if (buf == 0) {
        catch_mod.tcl_cmd_error(0);
        return;
    }
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: usize = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    for (class_name) |c| {
        dst[off] = c;
        off += 1;
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const m = obj_mod.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(m);
}

fn store_failindex(var_obj: i32, idx: i64) void {
    const obj_mod = @import("../valtypes/tcl_obj.zig");
    const frames_mod = @import("../interp/tcl_frames.zig");
    const idx_obj = obj_mod.obj_new_int(idx);
    _ = frames_mod.var_set(var_obj, idx_obj);
}

const ByteCheck = *const fn (b: u8) bool;

fn check_class_byte(ptr: [*]const u8, len: u32, fail: *i64, pred: ByteCheck) bool {
    var ci: u32 = 0; // character (codepoint) index
    var bi: u32 = 0; // byte index
    while (bi < len) {
        const b = ptr[bi];
        if (b < 0x80) {
            if (!pred(b)) {
                fail.* = ci;
                return false;
            }
            bi += 1;
        } else {
            // Multi-byte UTF-8 sequence — we approximate by treating
            // it as a generic character which passes alpha/wordchar/
            // print/graph but fails ascii/digit/space etc.  Predicate
            // returns ``false`` for byte ``b >= 0x80`` for restrictive
            // classes, which is the right answer for them.
            const cont = utf8_cont_len(b);
            if (!pred(b)) {
                fail.* = ci;
                return false;
            }
            bi += 1 + cont;
        }
        ci += 1;
    }
    return true;
}

fn utf8_cont_len(b: u8) u32 {
    if ((b & 0xE0) == 0xC0) return 1;
    if ((b & 0xF0) == 0xE0) return 2;
    if ((b & 0xF8) == 0xF0) return 3;
    return 0;
}

fn isAlnum(b: u8) bool {
    return isAlpha(b) or isDigit(b);
}
fn isAlpha(b: u8) bool {
    return (b >= 'a' and b <= 'z') or (b >= 'A' and b <= 'Z') or b >= 0x80;
}
fn isAscii(b: u8) bool {
    return b <= 0x7F;
}
fn isControl(b: u8) bool {
    return b < 0x20 or b == 0x7F;
}
fn isDigit(b: u8) bool {
    return b >= '0' and b <= '9';
}
fn isGraph(b: u8) bool {
    if (b >= 0x80) return true;
    return b > 0x20 and b != 0x7F;
}
fn isLower(b: u8) bool {
    return b >= 'a' and b <= 'z';
}
fn isPrint(b: u8) bool {
    if (b >= 0x80) return true;
    return b >= 0x20 and b != 0x7F;
}
fn isPunct(b: u8) bool {
    if (b >= 0x80) return false;
    return (b >= '!' and b <= '/') or (b >= ':' and b <= '@') or
        (b >= '[' and b <= '`') or (b >= '{' and b <= '~');
}
fn isSpace(b: u8) bool {
    return b == ' ' or b == '\t' or b == '\n' or b == '\r' or b == 0x0B or b == 0x0C;
}
fn isUpper(b: u8) bool {
    return b >= 'A' and b <= 'Z';
}
fn isWordchar(b: u8) bool {
    if (b >= 0x80) return true;
    return isAlnum(b) or b == '_';
}
fn isXdigit(b: u8) bool {
    return isDigit(b) or (b >= 'a' and b <= 'f') or (b >= 'A' and b <= 'F');
}

fn slice_eq(ptr: [*]const u8, len: u32, lit: []const u8) bool {
    if (len != lit.len) return false;
    for (0..len) |k| {
        if (ptr[k] != lit[k]) return false;
    }
    return true;
}

fn check_boolean(ptr: [*]const u8, len: u32) bool {
    return check_boolean_value(ptr, len, true) or check_boolean_value(ptr, len, false);
}

fn check_boolean_value(ptr: [*]const u8, len: u32, want_true: bool) bool {
    // Tcl ``Tcl_GetBoolean`` accepts non-empty case-insensitive
    // *prefixes* of the canonical literals — ``f`` / ``fa`` / ``fal``
    // are all valid for "false", and ``string is false N`` /
    // ``string is boolean f`` show up in the upstream test suite
    // (string-6.21 / 6.42).  ``1`` and ``0`` are the digit literals
    // and only match exactly.
    if (len == 0) return false;
    if (want_true) {
        if (len == 1 and ptr[0] == '1') return true;
        return iprefix(ptr, len, "true") or iprefix(ptr, len, "yes") or iprefix(ptr, len, "on");
    }
    if (len == 1 and ptr[0] == '0') return true;
    return iprefix(ptr, len, "false") or iprefix(ptr, len, "no") or iprefix(ptr, len, "off");
}

/// Case-insensitive prefix match: returns true when ``lit`` starts
/// with the buffer's bytes (length ``len``).  Used by the Tcl
/// boolean prefix-acceptance rule above.
fn iprefix(ptr: [*]const u8, len: u32, lit: []const u8) bool {
    if (len == 0 or len > lit.len) return false;
    for (0..len) |k| {
        const a = ptr[k];
        const al: u8 = if (a >= 'A' and a <= 'Z') a + 32 else a;
        const b = lit[k];
        const bl: u8 = if (b >= 'A' and b <= 'Z') b + 32 else b;
        if (al != bl) return false;
    }
    return true;
}

fn icmp(ptr: [*]const u8, len: u32, lit: []const u8) bool {
    if (len != lit.len) return false;
    for (0..len) |k| {
        const a = ptr[k];
        const al: u8 = if (a >= 'A' and a <= 'Z') a + 32 else a;
        const b = lit[k];
        const bl: u8 = if (b >= 'A' and b <= 'Z') b + 32 else b;
        if (al != bl) return false;
    }
    return true;
}

fn check_integer(ptr: [*]const u8, len: u32, fail: *i64) bool {
    var i: u32 = 0;
    // Leading whitespace is allowed and not counted in the fail
    // index pointing at the first invalid character.
    while (i < len and isSpace(ptr[i])) i += 1;
    const start = i;
    if (i < len and (ptr[i] == '+' or ptr[i] == '-')) i += 1;
    if (i < len and ptr[i] == '0' and i + 1 < len and (ptr[i + 1] == 'x' or ptr[i + 1] == 'X')) {
        i += 2;
        if (i >= len) {
            fail.* = i - 1;
            return false;
        }
        while (i < len and isXdigit(ptr[i])) : (i += 1) {}
        const after = i;
        while (i < len and isSpace(ptr[i])) i += 1;
        if (i != len) {
            fail.* = after;
            return false;
        }
        return true;
    } else if (i < len and ptr[i] == '0' and i + 1 < len and (ptr[i + 1] == 'b' or ptr[i + 1] == 'B')) {
        i += 2;
        if (i >= len) {
            fail.* = i - 1;
            return false;
        }
        while (i < len and (ptr[i] == '0' or ptr[i] == '1')) : (i += 1) {}
        const after = i;
        while (i < len and isSpace(ptr[i])) i += 1;
        if (i != len) {
            fail.* = after;
            return false;
        }
        return true;
    } else if (i < len and ptr[i] == '0' and i + 1 < len and (ptr[i + 1] == 'o' or ptr[i + 1] == 'O')) {
        i += 2;
        if (i >= len) {
            fail.* = i - 1;
            return false;
        }
        while (i < len and ptr[i] >= '0' and ptr[i] <= '7') : (i += 1) {}
        const after = i;
        while (i < len and isSpace(ptr[i])) i += 1;
        if (i != len) {
            fail.* = after;
            return false;
        }
        return true;
    } else {
        if (i >= len) {
            fail.* = if (start < len) @intCast(start) else 0;
            return false;
        }
        while (i < len and isDigit(ptr[i])) : (i += 1) {}
        // Trailing whitespace is allowed by Tcl integer parsing
        // (``tcl_bignum.parse_i128`` strips both ends).  Anything
        // else after the digits is a parse failure.
        const after_digits = i;
        while (i < len and isSpace(ptr[i])) i += 1;
        if (i != len) {
            fail.* = after_digits;
            return false;
        }
    }
    return true;
}

fn check_double(ptr: [*]const u8, len: u32, fail: *i64) bool {
    var i: u32 = 0;
    while (i < len and isSpace(ptr[i])) i += 1;
    if (i < len and (ptr[i] == '+' or ptr[i] == '-')) i += 1;
    var has_digit = false;
    while (i < len and isDigit(ptr[i])) {
        i += 1;
        has_digit = true;
    }
    if (i < len and ptr[i] == '.') {
        i += 1;
        while (i < len and isDigit(ptr[i])) {
            i += 1;
            has_digit = true;
        }
    }
    if (!has_digit) {
        fail.* = i;
        return false;
    }
    if (i < len and (ptr[i] == 'e' or ptr[i] == 'E')) {
        i += 1;
        if (i < len and (ptr[i] == '+' or ptr[i] == '-')) i += 1;
        if (i >= len or !isDigit(ptr[i])) {
            fail.* = i;
            return false;
        }
        while (i < len and isDigit(ptr[i])) i += 1;
    }
    // Trailing whitespace is permitted (matches Tcl's ``Tcl_GetDouble``
    // surroundings handling); anything else points at the offending
    // byte via ``fail``.
    const after_number = i;
    while (i < len and isSpace(ptr[i])) i += 1;
    if (i != len) {
        fail.* = after_number;
        return false;
    }
    return true;
}

/// ``string is list`` — accepts any string the list parser can
/// tokenise.  Sets ``fail`` to the byte index of the first parse
/// error (open brace without close, brace not followed by space).
fn check_list(ptr: u32, len: u32, fail: *i64) bool {
    if (len == 0) return true;
    const sp: [*]const u8 = @ptrFromInt(ptr);
    // Track the character index of each element start so failure
    // reports point at the element rather than the byte where the
    // parser noticed the problem (string-32.10/.11/.13/.14).
    var i: u32 = 0;
    while (i < len) {
        while (i < len and list_parse_is_space(sp[i])) i += 1;
        if (i >= len) break;
        const elem_start_byte = i;
        if (sp[i] == '{') {
            i += 1;
            var depth: u32 = 1;
            while (i < len and depth > 0) {
                if (sp[i] == '\\' and i + 1 < len) {
                    i += 2;
                    continue;
                }
                if (sp[i] == '{') depth += 1 else if (sp[i] == '}') depth -= 1;
                i += 1;
            }
            if (depth > 0) {
                fail.* = byte_to_char_idx(sp, len, elem_start_byte);
                return false;
            }
            // Brace must be followed by whitespace or end.
            if (i < len and !list_parse_is_space(sp[i])) {
                fail.* = byte_to_char_idx(sp, len, elem_start_byte);
                return false;
            }
        } else if (sp[i] == '"') {
            i += 1;
            var closed = false;
            while (i < len) {
                if (sp[i] == '\\' and i + 1 < len) {
                    i += 2;
                    continue;
                }
                if (sp[i] == '"') {
                    i += 1;
                    closed = true;
                    break;
                }
                i += 1;
            }
            if (!closed) {
                // ``"abc`` — unterminated quoted element.  Tcl's list
                // parser raises ``unmatched open quote in list``;
                // ``string is list`` reports that as a parse failure
                // pointing at the opening quote.
                fail.* = byte_to_char_idx(sp, len, elem_start_byte);
                return false;
            }
            // After a closed quote the next byte must be whitespace
            // or end (mirrors the brace branch's check).
            if (i < len and !list_parse_is_space(sp[i])) {
                fail.* = byte_to_char_idx(sp, len, elem_start_byte);
                return false;
            }
        } else {
            while (i < len and !list_parse_is_space(sp[i])) {
                if (sp[i] == '\\' and i + 1 < len) i += 2 else i += 1;
            }
        }
    }
    return true;
}

/// Convert a byte offset within a UTF-8 buffer to its 0-based
/// character (code-point) index.  Continuation bytes don't advance
/// the count.
fn byte_to_char_idx(ptr: [*]const u8, len: u32, byte_off: u32) i64 {
    var i: u32 = 0;
    var ci: i64 = 0;
    while (i < len and i < byte_off) : (i += 1) {
        if ((ptr[i] & 0xC0) != 0x80) ci += 1;
    }
    return ci;
}

fn list_parse_is_space(b: u8) bool {
    return b == ' ' or b == '\t' or b == '\n' or b == '\r' or b == 0x0B or b == 0x0C;
}

/// ``string is dict`` — must parse as a list and have an even number
/// of elements.  ``fail`` is set to the parse offset on tokenisation
/// error or ``-1`` when the parse succeeds but the element count is
/// odd (matches Tcl 9 — see test string-32.9a → ``-1``).
fn check_dict(ptr: u32, len: u32, fail: *i64) bool {
    var parse_fail: i64 = -1;
    if (!check_list(ptr, len, &parse_fail)) {
        fail.* = parse_fail;
        return false;
    }
    const list_parse = @import("../valtypes/tcl_list_parse.zig");
    const n = list_parse.count_elements(ptr, len);
    if (@mod(n, 2) != 0) {
        fail.* = -1;
        return false;
    }
    return true;
}
