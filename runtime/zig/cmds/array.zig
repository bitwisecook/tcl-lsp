// Tcl ``array`` built-in command.
//
// Extracted from tcl_interp_string.zig.  Registers itself in the
// central command table via the ``registration`` constant.

const rt = @import("../tcl_runtime.zig");

const result_mod = @import("../interp/tcl_result.zig");
const obj_ensure_string = rt.obj_ensure_string;
const obj_new_string = rt.obj_new_string;

const str_eq = @import("../valtypes/tcl_chars.zig").str_eq;

const reg = @import("../dispatch/tcl_cmd_registry.zig");

pub const registration = reg.CmdEntry{
    .name = "array",
    .arity_min = 1,
    .arity_max = null,
    .handler = &eval,
};

// Sub-command arities — mirrors ``core/commands/registry/tcl/array.py``.
// Arity counts args *after* the sub-command name, cross-checked against
// C Tcl 9.0 by ``scripts/check_wasm_command_parity.py``.
pub const subcommands: []const reg.SubEntry = &.{
    .{ .name = "anymore", .arity_min = 2, .arity_max = 2, .handler = &eval },
    .{ .name = "default", .arity_min = 2, .arity_max = 3, .handler = &eval },
    .{ .name = "donesearch", .arity_min = 2, .arity_max = 2, .handler = &eval },
    .{ .name = "exists", .arity_min = 1, .arity_max = 1, .handler = &eval },
    .{ .name = "for", .arity_min = 3, .arity_max = 3, .handler = &eval },
    .{ .name = "get", .arity_min = 1, .arity_max = 2, .handler = &eval },
    .{ .name = "names", .arity_min = 1, .arity_max = 3, .handler = &eval },
    .{ .name = "nextelement", .arity_min = 2, .arity_max = 2, .handler = &eval },
    .{ .name = "set", .arity_min = 2, .arity_max = 2, .handler = &eval },
    .{ .name = "size", .arity_min = 1, .arity_max = 1, .handler = &eval },
    .{ .name = "startsearch", .arity_min = 1, .arity_max = 1, .handler = &eval },
    .{ .name = "statistics", .arity_min = 1, .arity_max = 1, .handler = &eval },
    .{ .name = "unset", .arity_min = 1, .arity_max = 2, .handler = &eval },
};

/// Per-subcommand metadata: how many args after ``array SUB`` are
/// required / accepted, the canonical ``wrong # args`` wording, and
/// whether the subcommand demands a *live* array (e.g. ``array
/// anymore``) — if so and the named array doesn't exist, raise the
/// canonical ``"X" isn't an array`` error.
const SubRule = struct {
    name: []const u8,
    /// Total ``words.len`` minimum (including ``array`` + sub).
    min_words: u32,
    /// Total ``words.len`` maximum (null = unbounded).
    max_words: ?u32,
    usage: []const u8,
    /// True when the subcommand operates on an existing array and
    /// should error with ``"NAME" isn't an array`` when ``NAME``
    /// doesn't resolve to one.
    needs_array: bool,
};

const SUB_RULES: []const SubRule = &.{
    .{ .name = "anymore", .min_words = 4, .max_words = 4, .usage = "wrong # args: should be \"array anymore arrayName searchId\"", .needs_array = true },
    .{ .name = "default", .min_words = 4, .max_words = 5, .usage = "wrong # args: should be \"array default subcommand arrayName ?args?\"", .needs_array = false },
    .{ .name = "donesearch", .min_words = 4, .max_words = 4, .usage = "wrong # args: should be \"array donesearch arrayName searchId\"", .needs_array = true },
    .{ .name = "exists", .min_words = 3, .max_words = 3, .usage = "wrong # args: should be \"array exists arrayName\"", .needs_array = false },
    .{ .name = "for", .min_words = 5, .max_words = 5, .usage = "wrong # args: should be \"array for {key value} arrayName script\"", .needs_array = true },
    .{ .name = "get", .min_words = 3, .max_words = 4, .usage = "wrong # args: should be \"array get arrayName ?pattern?\"", .needs_array = false },
    .{ .name = "names", .min_words = 3, .max_words = 5, .usage = "wrong # args: should be \"array names arrayName ?mode? ?pattern?\"", .needs_array = false },
    .{ .name = "nextelement", .min_words = 4, .max_words = 4, .usage = "wrong # args: should be \"array nextelement arrayName searchId\"", .needs_array = true },
    .{ .name = "set", .min_words = 4, .max_words = 4, .usage = "wrong # args: should be \"array set arrayName list\"", .needs_array = false },
    .{ .name = "size", .min_words = 3, .max_words = 3, .usage = "wrong # args: should be \"array size arrayName\"", .needs_array = false },
    .{ .name = "startsearch", .min_words = 3, .max_words = 3, .usage = "wrong # args: should be \"array startsearch arrayName\"", .needs_array = true },
    .{ .name = "statistics", .min_words = 3, .max_words = 3, .usage = "wrong # args: should be \"array statistics arrayName\"", .needs_array = true },
    .{ .name = "unset", .min_words = 3, .max_words = 4, .usage = "wrong # args: should be \"array unset arrayName ?pattern?\"", .needs_array = false },
};

/// Subcommand list used in the ``unknown subcommand`` diagnostic.
const SUB_LIST: []const u8 = "anymore, default, donesearch, exists, for, get, names, nextelement, set, size, startsearch, statistics, or unset";

fn sub_rule_for(sp: [*]const u8, slen: u32) ?*const SubRule {
    if (slen == 0) return null;
    // Exact match wins outright.
    for (SUB_RULES) |*rule| {
        if (slen != rule.name.len) continue;
        var ok = true;
        for (rule.name, 0..) |c, i| if (sp[i] != c) {
            ok = false;
            break;
        };
        if (ok) return rule;
    }
    // Prefix-match.  Tcl 9 ``Tcl_GetIndexFromObj`` accepts any unique
    // unambiguous prefix; if multiple subcommands share the prefix
    // we fall through to the unknown-subcommand error.
    var match: ?*const SubRule = null;
    var ambiguous = false;
    for (SUB_RULES) |*rule| {
        if (slen >= rule.name.len) continue;
        var ok = true;
        for (0..slen) |i| {
            if (sp[i] != rule.name[i]) {
                ok = false;
                break;
            }
        }
        if (!ok) continue;
        if (match != null) {
            ambiguous = true;
            break;
        }
        match = rule;
    }
    if (ambiguous) return null;
    return match;
}

pub fn eval(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) {
        raise_array_error("wrong # args: should be \"array subcommand ?arg ...?\"");
        return result_mod.from_globals(0);
    }
    const sub = obj_ensure_string(words[1]);
    const sp_in: [*]const u8 = @ptrFromInt(sub.ptr);
    const rule_opt = sub_rule_for(sp_in, sub.len);
    if (rule_opt == null) {
        // Unknown subcommand — canonical diagnostic (set-old-8.6).
        raise_unknown_subcmd(sp_in, sub.len);
        return result_mod.from_globals(0);
    }
    const rule = rule_opt.?;
    if (words.len < rule.min_words or (rule.max_words != null and words.len > rule.max_words.?)) {
        raise_array_error(rule.usage);
        return result_mod.from_globals(0);
    }
    // Use the canonical resolved name for dispatch comparisons —
    // user-typed prefixes like ``array next ...`` should land in
    // the ``nextelement`` branch.
    const sp: [*]const u8 = rule.name.ptr;
    const sub_len: u32 = @intCast(rule.name.len);
    const array_mod = @import("../valtypes/tcl_array.zig");
    const frames_mod = @import("../interp/tcl_frames.zig");
    const obj_mod = @import("../valtypes/tcl_obj.zig");
    const resolved_name: i32 = frames_mod.frame_resolve_array_name(words[2]);
    defer if (resolved_name != words[2]) obj_mod.tcl_obj_release(resolved_name);
    if (rule.needs_array) {
        const exists_obj = array_mod.array_exists(resolved_name);
        if (obj_mod.obj_get_int(exists_obj) == 0) {
            raise_not_an_array(words[2]);
            return result_mod.from_globals(0);
        }
    }
    if (str_eq(sp, sub_len, "get")) {
        const pat: i32 = if (words.len >= 4) words[3] else 0;
        return result_mod.from_globals(array_mod.array_get_all(resolved_name, pat));
    }
    if (str_eq(sp, sub_len, "set")) {
        // ``array set arr pairlist`` — also reject ``set X LIST`` when
        // X is a scalar (Tcl 9 ``Tcl_ArrayObjCmd`` raises ``can't
        // array set "X": variable isn't array``).
        // We trust ``array_set_list`` to detect the odd-length list
        // case and surface the ``list must have an even number of
        // elements`` error.
        return result_mod.from_globals(array_mod.array_set_list(resolved_name, words[3]));
    }
    if (str_eq(sp, sub_len, "exists")) return result_mod.from_globals(array_mod.array_exists(resolved_name));
    if (str_eq(sp, sub_len, "names")) {
        // ``array names arr ?mode? ?pattern?`` — mode is one of
        // ``-exact`` / ``-glob`` / ``-regexp``.  Without mode the
        // default is glob; the rule's ``max_words`` allows 5 (mode +
        // pattern).  Our underlying ``array_names`` only supports
        // glob filtering today; route mode through here so we can
        // validate at least the option keyword.
        if (words.len == 4) {
            return result_mod.from_globals(array_mod.array_names(resolved_name, words[3]));
        }
        if (words.len == 5) {
            const mode_s = obj_ensure_string(words[3]);
            if (!is_valid_names_mode(mode_s.ptr, mode_s.len)) {
                raise_bad_names_mode(words[3]);
                return result_mod.from_globals(0);
            }
            // ``-exact`` falls through to glob (we exact-match by
            // glob with no metacharacters).  ``-regexp`` is not yet
            // implemented; fall back to glob too — slightly wrong
            // for regex patterns but better than empty.
            return result_mod.from_globals(array_mod.array_names(resolved_name, words[4]));
        }
        return result_mod.from_globals(array_mod.array_names(resolved_name, 0));
    }
    if (str_eq(sp, sub_len, "size")) return result_mod.from_globals(array_mod.array_size(resolved_name));
    if (str_eq(sp, sub_len, "unset")) {
        if (words.len >= 4) return result_mod.from_globals(array_mod.array_unset_element(resolved_name, words[3]));
        return result_mod.from_globals(array_mod.array_unset(resolved_name));
    }
    // ``anymore`` / ``donesearch`` / ``nextelement``.  The
    // ``needs_array`` probe above already handled the missing-array
    // case.  Validate the SID shape against the operand array name,
    // then look up the search record — if it's not currently
    // registered the caller sees ``couldn't find search "..."``
    // (set-old-9.5 / 9.6 / 9.8 / 9.10).
    if (str_eq(sp, sub_len, "anymore") or
        str_eq(sp, sub_len, "donesearch") or
        str_eq(sp, sub_len, "nextelement"))
    {
        const sid_obj = obj_ensure_string(words[3]);
        const arr_obj = obj_ensure_string(words[2]);
        const validation = validate_search_id(sid_obj.ptr, sid_obj.len, arr_obj.ptr, arr_obj.len);
        switch (validation) {
            .illegal => {
                raise_illegal_search_id(words[3]);
                return result_mod.from_globals(0);
            },
            .wrong_array => {
                raise_search_wrong_array(words[3], words[2]);
                return result_mod.from_globals(0);
            },
            .ok => {},
        }
        // SID is well-formed and the VARNAME matches the operand
        // array.  Now resolve the (array, id) pair to a live
        // search record — keyed by the *resolved* storage name so
        // proc-local arrays and ``upvar`` aliases find their
        // records (lookup must match what startsearch wrote, which
        // is the post-:func:`frame_resolve_array_name` form).
        const id = parse_search_id_number(sid_obj.ptr, sid_obj.len);
        const rec = array_mod.array_search_lookup(resolved_name, id);
        if (rec == 0) {
            raise_search_not_found(words[3]);
            return result_mod.from_globals(0);
        }
        if (str_eq(sp, sub_len, "anymore")) {
            const has = array_mod.array_search_anymore(rec);
            return result_mod.from_globals(obj_mod.obj_new_int(if (has) @as(i32, 1) else @as(i32, 0)));
        }
        if (str_eq(sp, sub_len, "donesearch")) {
            array_mod.array_search_done(rec);
            return result_mod.from_globals(0);
        }
        // nextelement
        return result_mod.from_globals(array_mod.array_search_next(resolved_name, rec));
    }
    if (str_eq(sp, sub_len, "startsearch")) {
        return result_mod.from_globals(array_mod.array_search_start(resolved_name, words[2]));
    }
    if (str_eq(sp, sub_len, "statistics")) {
        return result_mod.from_globals(obj_new_string(0, 0));
    }
    // ``for`` / ``default`` — not implemented; fall through to the
    // stub-trap path so the test surface remains visible.
    const stubs_mod = @import("../stubs/tcl_stubs.zig");
    const sub_slice: []const u8 = (@as([*]const u8, @ptrFromInt(sub.ptr)))[0..sub.len];
    stubs_mod.unsupported_sub("array", sub_slice);
    return result_mod.from_globals(0);
}

fn raise_array_error(msg: []const u8) void {
    const obj_mod = @import("../valtypes/tcl_obj.zig");
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

fn raise_not_an_array(name_obj: i32) void {
    const obj_mod = @import("../valtypes/tcl_obj.zig");
    const catch_mod = @import("../interp/tcl_catch.zig");
    const sn = obj_mod.obj_ensure_string(name_obj);
    const prefix: []const u8 = "\"";
    const suffix: []const u8 = "\" isn't an array";
    const total: u32 = @intCast(prefix.len + sn.len + suffix.len);
    const buf = obj_mod.alloc(total);
    if (buf == 0) {
        catch_mod.tcl_cmd_error(obj_mod.obj_new_string(0, 0));
        return;
    }
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    const np: [*]const u8 = @ptrFromInt(sn.ptr);
    for (0..sn.len) |k| {
        dst[off] = np[k];
        off += 1;
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const e = obj_mod.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(e);
}

fn raise_unknown_subcmd(sp: [*]const u8, slen: u32) void {
    const obj_mod = @import("../valtypes/tcl_obj.zig");
    const catch_mod = @import("../interp/tcl_catch.zig");
    const prefix: []const u8 = "unknown or ambiguous subcommand \"";
    const mid: []const u8 = "\": must be ";
    const total: u32 = @intCast(prefix.len + slen + mid.len + SUB_LIST.len);
    const buf = obj_mod.alloc(total);
    if (buf == 0) {
        catch_mod.tcl_cmd_error(obj_mod.obj_new_string(0, 0));
        return;
    }
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    for (0..slen) |k| {
        dst[off] = sp[k];
        off += 1;
    }
    for (mid) |c| {
        dst[off] = c;
        off += 1;
    }
    for (SUB_LIST) |c| {
        dst[off] = c;
        off += 1;
    }
    const e = obj_mod.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(e);
}

fn raise_illegal_search_id(id_obj: i32) void {
    const obj_mod = @import("../valtypes/tcl_obj.zig");
    const catch_mod = @import("../interp/tcl_catch.zig");
    const sn = obj_mod.obj_ensure_string(id_obj);
    const prefix: []const u8 = "illegal search identifier \"";
    const suffix: []const u8 = "\"";
    const total: u32 = @intCast(prefix.len + sn.len + suffix.len);
    const buf = obj_mod.alloc(total);
    if (buf == 0) {
        catch_mod.tcl_cmd_error(obj_mod.obj_new_string(0, 0));
        return;
    }
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    const np: [*]const u8 = @ptrFromInt(sn.ptr);
    for (0..sn.len) |k| {
        dst[off] = np[k];
        off += 1;
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const e = obj_mod.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(e);
}

fn raise_bad_names_mode(mode_obj: i32) void {
    const obj_mod = @import("../valtypes/tcl_obj.zig");
    const catch_mod = @import("../interp/tcl_catch.zig");
    const sn = obj_mod.obj_ensure_string(mode_obj);
    const prefix: []const u8 = "bad option \"";
    const suffix: []const u8 = "\": must be -exact, -glob, or -regexp";
    const total: u32 = @intCast(prefix.len + sn.len + suffix.len);
    const buf = obj_mod.alloc(total);
    if (buf == 0) {
        catch_mod.tcl_cmd_error(obj_mod.obj_new_string(0, 0));
        return;
    }
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    const np: [*]const u8 = @ptrFromInt(sn.ptr);
    for (0..sn.len) |k| {
        dst[off] = np[k];
        off += 1;
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const e = obj_mod.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(e);
}

const SidStatus = enum { illegal, wrong_array, ok };

/// Validate an ``array next/done/anymore`` search-ID against the
/// canonical Tcl 9 ``s-N-VARNAME`` shape.  Returns:
///   * ``illegal``     — malformed SID (raise "illegal search identifier")
///   * ``wrong_array`` — well-formed but VARNAME doesn't match the
///                       array operand (raise "search identifier ...
///                       isn't for variable ...")
///   * ``ok``          — shape matches; caller then checks whether
///                       a live search record exists, raising
///                       ``couldn't find search ...`` if not.
fn validate_search_id(sid_ptr: u32, sid_len: u32, arr_ptr: u32, arr_len: u32) SidStatus {
    if (sid_len < 4) return .illegal;
    const sp: [*]const u8 = @ptrFromInt(sid_ptr);
    // Must start with ``s-``.
    if (sp[0] != 's' or sp[1] != '-') return .illegal;
    // Walk digits.
    var i: u32 = 2;
    var ndigits: u32 = 0;
    while (i < sid_len and sp[i] >= '0' and sp[i] <= '9') : (i += 1) ndigits += 1;
    if (ndigits == 0) return .illegal;
    // Next must be ``-``.
    if (i >= sid_len or sp[i] != '-') return .illegal;
    i += 1;
    // Remainder is the array name — must be non-empty and exactly
    // match the array operand.
    const name_len: u32 = sid_len - i;
    if (name_len == 0) return .illegal;
    if (name_len != arr_len) return .wrong_array;
    const ap: [*]const u8 = @ptrFromInt(arr_ptr);
    for (0..name_len) |k| {
        if (sp[i + k] != ap[k]) return .wrong_array;
    }
    return .ok;
}

/// Parse the numeric component out of a validated ``s-N-NAME``
/// search-ID.  Assumes :fn:`validate_search_id` returned ``.ok``
/// (or ``.wrong_array``) — i.e. the shape is well-formed.
fn parse_search_id_number(sid_ptr: u32, sid_len: u32) u32 {
    var n: u32 = 0;
    const sp: [*]const u8 = @ptrFromInt(sid_ptr);
    var i: u32 = 2;
    while (i < sid_len and sp[i] >= '0' and sp[i] <= '9') : (i += 1) {
        n = n * 10 + (sp[i] - '0');
    }
    return n;
}

fn raise_search_wrong_array(sid: i32, arr: i32) void {
    const obj_mod = @import("../valtypes/tcl_obj.zig");
    const catch_mod = @import("../interp/tcl_catch.zig");
    const sid_s = obj_mod.obj_ensure_string(sid);
    const arr_s = obj_mod.obj_ensure_string(arr);
    const prefix: []const u8 = "search identifier \"";
    const mid: []const u8 = "\" isn't for variable \"";
    const suffix: []const u8 = "\"";
    const total: u32 = @intCast(prefix.len + sid_s.len + mid.len + arr_s.len + suffix.len);
    const buf = obj_mod.alloc(total);
    if (buf == 0) {
        catch_mod.tcl_cmd_error(obj_mod.obj_new_string(0, 0));
        return;
    }
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    const sp: [*]const u8 = @ptrFromInt(sid_s.ptr);
    for (0..sid_s.len) |k| {
        dst[off] = sp[k];
        off += 1;
    }
    for (mid) |c| {
        dst[off] = c;
        off += 1;
    }
    const ap: [*]const u8 = @ptrFromInt(arr_s.ptr);
    for (0..arr_s.len) |k| {
        dst[off] = ap[k];
        off += 1;
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const e = obj_mod.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(e);
}

fn raise_search_not_found(sid: i32) void {
    const obj_mod = @import("../valtypes/tcl_obj.zig");
    const catch_mod = @import("../interp/tcl_catch.zig");
    const sn = obj_mod.obj_ensure_string(sid);
    const prefix: []const u8 = "couldn't find search \"";
    const suffix: []const u8 = "\"";
    const total: u32 = @intCast(prefix.len + sn.len + suffix.len);
    const buf = obj_mod.alloc(total);
    if (buf == 0) {
        catch_mod.tcl_cmd_error(obj_mod.obj_new_string(0, 0));
        return;
    }
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    const np: [*]const u8 = @ptrFromInt(sn.ptr);
    for (0..sn.len) |k| {
        dst[off] = np[k];
        off += 1;
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const e = obj_mod.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(e);
}

fn is_valid_names_mode(p: u32, len: u32) bool {
    if (p == 0 or len == 0) return false;
    const sp: [*]const u8 = @ptrFromInt(p);
    if (sp[0] != '-') return false;
    inline for (.{ "-exact", "-glob", "-regexp" }) |lit| {
        if (len == lit.len) {
            var matches = true;
            inline for (lit, 0..) |c, i| {
                if (sp[i] != c) {
                    matches = false;
                    break;
                }
            }
            if (matches) return true;
        }
    }
    return false;
}
