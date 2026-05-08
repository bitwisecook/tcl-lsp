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
    const sub = obj_ensure_string(words[1]);
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
    if (str_eq(sp, sub.len, "index") and words.len >= 4) return result_mod.from_globals(rt.string_index(words[2], words[3]));
    if (str_eq(sp, sub.len, "range") and words.len >= 5) return result_mod.from_globals(rt.string_range(words[2], words[3], words[4]));
    if (str_eq(sp, sub.len, "compare") and words.len >= 4) return result_mod.from_globals(rt.string_compare(words[2], words[3]));
    if (str_eq(sp, sub.len, "equal") and words.len >= 4) return result_mod.from_globals(rt.string_equal(words[2], words[3]));
    if (str_eq(sp, sub.len, "match") and words.len >= 4) return result_mod.from_globals(rt.string_match(words[2], words[3]));
    if (str_eq(sp, sub.len, "map") and words.len >= 4) {
        // ``string map ?-nocase? CHARMAP STRING`` — accept the
        // optional ``-nocase`` flag.  Without this branch, the
        // flag was passed as the CHARMAP argument and the actual
        // CHARMAP was treated as the STRING, which silently
        // mangled tcltest's return-code translation
        // (``string map -nocase {ok 0 ...} {0 2}`` returned the
        // MAP itself).
        var map_idx: u32 = 2;
        var nocase = false;
        if (words.len >= 5) {
            const ws = obj_ensure_string(words[2]);
            const wp: [*]const u8 = @ptrFromInt(ws.ptr);
            if (str_eq(wp, ws.len, "-nocase")) {
                nocase = true;
                map_idx = 3;
            }
        }
        if (map_idx + 1 >= words.len) return result_mod.from_globals(obj_new_string(0, 0));
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
    if (str_eq(sp, sub.len, "first") and words.len >= 4) return result_mod.from_globals(rt.string_first(words[2], words[3]));
    if (str_eq(sp, sub.len, "last") and words.len >= 4) return result_mod.from_globals(rt.string_last(words[2], words[3]));
    if (str_eq(sp, sub.len, "toupper")) return result_mod.from_globals(rt.string_toupper(words[2]));
    if (str_eq(sp, sub.len, "tolower")) return result_mod.from_globals(rt.string_tolower(words[2]));
    if (str_eq(sp, sub.len, "reverse")) return result_mod.from_globals(rt.string_reverse(words[2]));
    if (str_eq(sp, sub.len, "repeat") and words.len >= 4) return result_mod.from_globals(rt.string_repeat(words[2], words[3]));
    if (str_eq(sp, sub.len, "replace") and words.len >= 6) return result_mod.from_globals(rt.string_replace(words[2], words[3], words[4], words[5]));
    if (str_eq(sp, sub.len, "insert") and words.len >= 5) return result_mod.from_globals(rt.string_insert(words[2], words[3], words[4]));
    if (str_eq(sp, sub.len, "is")) {
        // ``string is class ?-strict? ?-failindex var? str``
        // Find the class name (words[2]) and the final string arg.
        // Skip any -strict / -failindex flags and their args.
        if (words.len < 4) return result_mod.from_globals(obj_new_int(1)); // empty string: non-strict default is 1
        const cls = obj_ensure_string(words[2]);
        const clsp: [*]const u8 = @ptrFromInt(cls.ptr);
        var str_idx: u32 = 3;
        while (str_idx + 1 < words.len) {
            const a = obj_ensure_string(words[str_idx]);
            const ap: [*]const u8 = @ptrFromInt(a.ptr);
            if (a.len > 0 and ap[0] == '-') {
                // -strict: no extra arg; -failindex: consumes next arg
                if (str_eq(ap, a.len, "-failindex")) str_idx += 1;
                str_idx += 1;
            } else break;
        }
        if (str_idx >= words.len) return result_mod.from_globals(obj_new_int(1));
        const sv = obj_ensure_string(words[str_idx]);
        if (sv.len == 0) {
            // non-strict: empty is 1 for all; strict: 0
            return result_mod.from_globals(obj_new_int(1));
        }
        const svp: [*]const u8 = @ptrFromInt(sv.ptr);
        if (str_eq(clsp, cls.len, "print")) {
            // printable: 0x20-0x7E ASCII, or any multibyte UTF-8
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                const b = svp[i];
                if (b >= 0x80) continue; // multibyte UTF-8 — treat as printable
                if (b < 0x20 or b == 0x7F) return result_mod.from_globals(obj_new_int(0));
            }
            return result_mod.from_globals(obj_new_int(1));
        }
        if (str_eq(clsp, cls.len, "alpha")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                const b = svp[i];
                if (b >= 0x80) {
                    i += 1;
                    continue;
                }
                if (!((b >= 'a' and b <= 'z') or (b >= 'A' and b <= 'Z'))) return result_mod.from_globals(obj_new_int(0));
            }
            return result_mod.from_globals(obj_new_int(1));
        }
        if (str_eq(clsp, cls.len, "digit")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                if (svp[i] < '0' or svp[i] > '9') return result_mod.from_globals(obj_new_int(0));
            }
            return result_mod.from_globals(obj_new_int(1));
        }
        if (str_eq(clsp, cls.len, "alnum")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                const b = svp[i];
                if (b >= 0x80) {
                    i += 1;
                    continue;
                }
                if (!((b >= 'a' and b <= 'z') or (b >= 'A' and b <= 'Z') or (b >= '0' and b <= '9'))) return result_mod.from_globals(obj_new_int(0));
            }
            return result_mod.from_globals(obj_new_int(1));
        }
        if (str_eq(clsp, cls.len, "space") or str_eq(clsp, cls.len, "whitespace")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                const b = svp[i];
                if (b != ' ' and b != '\t' and b != '\n' and b != '\r' and b != 0x0C and b != 0x0B) return result_mod.from_globals(obj_new_int(0));
            }
            return result_mod.from_globals(obj_new_int(1));
        }
        // ``integer`` and ``wideinteger`` accept the same set of
        // string forms: optional whitespace, optional sign, then
        // either a decimal run or a ``0x``-prefixed hex run.  In Tcl
        // 9 with bignum support the magnitude is unbounded, so the
        // inline path doesn't need to bound-check — any all-digit
        // (or hex-digit) run after the prefix passes.
        if (str_eq(clsp, cls.len, "integer") or str_eq(clsp, cls.len, "wideinteger")) {
            var i: u32 = 0;
            while (i < sv.len and (svp[i] == ' ' or svp[i] == '\t')) i += 1;
            if (i < sv.len and (svp[i] == '+' or svp[i] == '-')) i += 1;
            if (i < sv.len and svp[i] == '0' and i + 1 < sv.len and (svp[i + 1] == 'x' or svp[i + 1] == 'X')) {
                i += 2;
                if (i >= sv.len) return result_mod.from_globals(obj_new_int(0));
                while (i < sv.len) : (i += 1) {
                    const b = svp[i];
                    if (!((b >= '0' and b <= '9') or (b >= 'a' and b <= 'f') or (b >= 'A' and b <= 'F'))) return result_mod.from_globals(obj_new_int(0));
                }
                return result_mod.from_globals(obj_new_int(1));
            }
            if (i >= sv.len) return result_mod.from_globals(obj_new_int(0));
            while (i < sv.len) : (i += 1) {
                if (svp[i] < '0' or svp[i] > '9') return result_mod.from_globals(obj_new_int(0));
            }
            return result_mod.from_globals(obj_new_int(1));
        }
        if (str_eq(clsp, cls.len, "boolean")) {
            if (str_eq(svp, sv.len, "1") or str_eq(svp, sv.len, "0") or
                str_eq(svp, sv.len, "true") or str_eq(svp, sv.len, "false") or
                str_eq(svp, sv.len, "yes") or str_eq(svp, sv.len, "no") or
                str_eq(svp, sv.len, "on") or str_eq(svp, sv.len, "off") or
                str_eq(svp, sv.len, "True") or str_eq(svp, sv.len, "False") or
                str_eq(svp, sv.len, "TRUE") or str_eq(svp, sv.len, "FALSE")) return result_mod.from_globals(obj_new_int(1));
            return result_mod.from_globals(obj_new_int(0));
        }
        if (str_eq(clsp, cls.len, "ascii")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                if (svp[i] > 0x7F) return result_mod.from_globals(obj_new_int(0));
            }
            return result_mod.from_globals(obj_new_int(1));
        }
        if (str_eq(clsp, cls.len, "control")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                const b = svp[i];
                if (b >= 0x80) return result_mod.from_globals(obj_new_int(0));
                if (b >= 0x20 and b != 0x7F) return result_mod.from_globals(obj_new_int(0));
            }
            return result_mod.from_globals(obj_new_int(1));
        }
        if (str_eq(clsp, cls.len, "graph")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                const b = svp[i];
                if (b >= 0x80) {
                    i += 1;
                    continue;
                }
                if (b <= 0x20 or b == 0x7F) return result_mod.from_globals(obj_new_int(0));
            }
            return result_mod.from_globals(obj_new_int(1));
        }
        if (str_eq(clsp, cls.len, "lower")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                const b = svp[i];
                if (b >= 0x80) {
                    i += 1;
                    continue;
                }
                if (b < 'a' or b > 'z') return result_mod.from_globals(obj_new_int(0));
            }
            return result_mod.from_globals(obj_new_int(1));
        }
        if (str_eq(clsp, cls.len, "upper")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                const b = svp[i];
                if (b >= 0x80) {
                    i += 1;
                    continue;
                }
                if (b < 'A' or b > 'Z') return result_mod.from_globals(obj_new_int(0));
            }
            return result_mod.from_globals(obj_new_int(1));
        }
        if (str_eq(clsp, cls.len, "punct")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                const b = svp[i];
                if (b >= 0x80) {
                    i += 1;
                    continue;
                }
                const is_punct = (b >= '!' and b <= '/') or (b >= ':' and b <= '@') or
                    (b >= '[' and b <= '`') or (b >= '{' and b <= '~');
                if (!is_punct) return result_mod.from_globals(obj_new_int(0));
            }
            return result_mod.from_globals(obj_new_int(1));
        }
        if (str_eq(clsp, cls.len, "xdigit")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                const b = svp[i];
                if (!((b >= '0' and b <= '9') or (b >= 'a' and b <= 'f') or (b >= 'A' and b <= 'F'))) return result_mod.from_globals(obj_new_int(0));
            }
            return result_mod.from_globals(obj_new_int(1));
        }
        if (str_eq(clsp, cls.len, "double") or str_eq(clsp, cls.len, "float")) {
            // Very basic: try to parse as number with optional decimal/exponent
            var i: u32 = 0;
            while (i < sv.len and (svp[i] == ' ' or svp[i] == '\t')) i += 1;
            if (i < sv.len and (svp[i] == '+' or svp[i] == '-')) i += 1;
            var has_digit = false;
            while (i < sv.len and svp[i] >= '0' and svp[i] <= '9') {
                i += 1;
                has_digit = true;
            }
            if (i < sv.len and svp[i] == '.') {
                i += 1;
                while (i < sv.len and svp[i] >= '0' and svp[i] <= '9') {
                    i += 1;
                    has_digit = true;
                }
            }
            if (!has_digit) return result_mod.from_globals(obj_new_int(0));
            if (i < sv.len and (svp[i] == 'e' or svp[i] == 'E')) {
                i += 1;
                if (i < sv.len and (svp[i] == '+' or svp[i] == '-')) i += 1;
                if (i >= sv.len or svp[i] < '0' or svp[i] > '9') return result_mod.from_globals(obj_new_int(0));
                while (i < sv.len and svp[i] >= '0' and svp[i] <= '9') i += 1;
            }
            if (i != sv.len) return result_mod.from_globals(obj_new_int(0));
            return result_mod.from_globals(obj_new_int(1));
        }
        // Unknown class — return 0
        return result_mod.from_globals(obj_new_int(0));
    }
    return result_mod.from_globals(0);
}
