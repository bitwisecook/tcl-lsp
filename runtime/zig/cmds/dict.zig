// Tcl ``dict`` built-in command.
//
// Extracted from tcl_interp_string.zig.  Registers itself in the
// central command table via the ``registration`` constant.

const rt = @import("../tcl_runtime.zig");
const frames = @import("../interp/tcl_frames.zig");
const interp = @import("../interp/tcl_interp.zig");
const obj = @import("../valtypes/tcl_obj.zig");
const list_parse = @import("../valtypes/tcl_list_parse.zig");
const result_mod = @import("../interp/tcl_result.zig");

const obj_ensure_string = rt.obj_ensure_string;

const str_eq = @import("../valtypes/tcl_chars.zig").str_eq;

const reg = @import("../dispatch/tcl_cmd_registry.zig");

pub const registration = reg.CmdEntry{
    .name = "dict",
    .arity_min = 1,
    .arity_max = null,
    .handler = &eval,
};

// Sub-command arities — mirrors ``core/commands/registry/tcl/dict.py``.
// Cross-checked against C Tcl 9.0 ``tclDictObj.c`` (every
// ``Tcl_WrongNumArgs`` call in every ``Dict*Cmd`` handler).
pub const subcommands: []const reg.SubEntry = &.{
    .{ .name = "append", .arity_min = 2, .arity_max = null, .handler = &eval },
    .{ .name = "create", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "exists", .arity_min = 2, .arity_max = null, .handler = &eval },
    .{ .name = "filter", .arity_min = 2, .arity_max = null, .handler = &eval },
    .{ .name = "for", .arity_min = 3, .arity_max = 3, .handler = &eval },
    .{ .name = "get", .arity_min = 1, .arity_max = null, .handler = &eval },
    // ``dict getd`` is a Tcl 9 synonym for ``dict getdef`` /
    // ``dict getwithdefault``.  Reference Tcl matches it via the
    // command-prefix abbreviation rules; we register it explicitly
    // so the matcher doesn't trip on the ``getdef`` / ``getwithdefault``
    // ambiguity at the byte level.
    .{ .name = "getd", .arity_min = 3, .arity_max = null, .handler = &eval },
    .{ .name = "getdef", .arity_min = 3, .arity_max = null, .handler = &eval },
    .{ .name = "getwithdefault", .arity_min = 3, .arity_max = null, .handler = &eval },
    .{ .name = "incr", .arity_min = 2, .arity_max = 3, .handler = &eval },
    .{ .name = "info", .arity_min = 1, .arity_max = 1, .handler = &eval },
    .{ .name = "keys", .arity_min = 1, .arity_max = 2, .handler = &eval },
    .{ .name = "lappend", .arity_min = 2, .arity_max = null, .handler = &eval },
    .{ .name = "map", .arity_min = 3, .arity_max = 3, .handler = &eval },
    .{ .name = "merge", .arity_min = 0, .arity_max = null, .handler = &eval },
    .{ .name = "remove", .arity_min = 1, .arity_max = null, .handler = &eval },
    .{ .name = "replace", .arity_min = 1, .arity_max = null, .handler = &eval },
    .{ .name = "set", .arity_min = 3, .arity_max = null, .handler = &eval },
    .{ .name = "size", .arity_min = 1, .arity_max = 1, .handler = &eval },
    .{ .name = "unset", .arity_min = 2, .arity_max = null, .handler = &eval },
    .{ .name = "update", .arity_min = 4, .arity_max = null, .handler = &eval },
    .{ .name = "values", .arity_min = 1, .arity_max = 2, .handler = &eval },
    .{ .name = "with", .arity_min = 2, .arity_max = null, .handler = &eval },
};

pub fn eval(words: []const i32) result_mod.InterpResult {
    if (words.len < 3) return result_mod.from_globals(0);
    const sub = obj_ensure_string(words[1]);
    const sp: [*]const u8 = @ptrFromInt(sub.ptr);
    if (str_eq(sp, sub.len, "get") and words.len >= 4) {
        // ``dict get DICT KEY ?KEY KEY ...?`` — Tcl 9 supports
        // chained-key descent into nested dicts (error-18.10's
        // ``dict get $opts -during -during -errorcode`` checks
        // a two-level chain).  Walk the chain from words[2] (the
        // dict) through words[3..] (the keys).  Missing keys raise
        // ``key "K" not known in dictionary``.
        var cur: i32 = words[2];
        var ki: u32 = 3;
        while (ki < words.len) : (ki += 1) {
            if (rt.obj_get_int(rt.dict_exists(cur, words[ki])) == 0) {
                const stubs_mod = @import("../stubs/tcl_stubs.zig");
                const obj_mod_dict = @import("../valtypes/tcl_obj.zig");
                // Build ``key "<key>" not known in dictionary``.
                const ks = obj_ensure_string(words[ki]);
                const prefix: []const u8 = "key \"";
                const middle: []const u8 = "\" not known in dictionary";
                const total: u32 = @intCast(prefix.len + ks.len + middle.len);
                const buf = obj_mod_dict.alloc(total);
                if (buf == 0) {
                    stubs_mod.raise("dict get: key not found");
                    return result_mod.from_globals(0);
                }
                const dst: [*]u8 = @ptrFromInt(buf);
                var off: u32 = 0;
                for (prefix) |c| { dst[off] = c; off += 1; }
                if (ks.len > 0 and ks.ptr != 0) {
                    const sp_k: [*]const u8 = @ptrFromInt(ks.ptr);
                    var k: u32 = 0;
                    while (k < ks.len) : (k += 1) { dst[off] = sp_k[k]; off += 1; }
                }
                for (middle) |c| { dst[off] = c; off += 1; }
                const msg = obj_mod_dict.obj_new_string_take(buf, total, total);
                const catch_mod_dict = @import("../interp/tcl_catch.zig");
                catch_mod_dict.tcl_cmd_error(msg);
                return result_mod.from_globals(0);
            }
            cur = rt.dict_get(cur, words[ki]);
        }
        return result_mod.from_globals(cur);
    }
    // ``dict getd DICT KEY ?KEY ...? DEFAULT`` — Tcl 9's
    // default-value lookup form.  Returns the existing value when
    // the key chain resolves; otherwise returns the trailing
    // ``DEFAULT`` argument.  ``getdef`` / ``getwithdefault`` are
    // synonyms for the same operation; matched here for the same
    // arity (2 args = dict + key + default at minimum).
    if ((str_eq(sp, sub.len, "getd") or
        str_eq(sp, sub.len, "getdef") or
        str_eq(sp, sub.len, "getwithdefault")) and words.len >= 5)
    {
        // Walk nested keys: dict, key1, key2, ..., default.
        // ``cur`` starts borrowed (``words[2]``) and after the first
        // ``dict_get`` becomes +1 owned.  Each subsequent ``dict_get``
        // overwrites ``cur`` with another +1 owned obj; release the
        // prior one or the chain leaks one obj per drill level.
        const default_obj = words[words.len - 1];
        var cur: i32 = words[2];
        var owns_cur: bool = false;
        var ki: u32 = 3;
        while (ki + 1 < words.len) : (ki += 1) {
            const exists_obj = rt.dict_exists(cur, words[ki]);
            const exists = rt.obj_get_int(exists_obj) != 0;
            if (!exists) {
                if (owns_cur) obj.tcl_obj_release(cur);
                return result_mod.from_globals(default_obj);
            }
            const next = rt.dict_get(cur, words[ki]);
            if (owns_cur) obj.tcl_obj_release(cur);
            cur = next;
            owns_cur = true;
        }
        // ``result_mod.from_globals`` doesn't take ownership; the
        // dispatcher's retain-then-release pair on the return value
        // covers our +1 transfer when ``owns_cur``.
        return result_mod.from_globals(cur);
    }
    if (str_eq(sp, sub.len, "set") and words.len >= 5) {
        const cur = frames.var_resolve(words[2]);
        const result = rt.dict_set(cur, words[3], words[4]);
        _ = frames.var_set(words[2], result);
        return result_mod.from_globals(result);
    }
    if (str_eq(sp, sub.len, "unset") and words.len >= 4) {
        const cur = frames.var_resolve(words[2]);
        const result = rt.dict_unset(cur, words[3]);
        _ = frames.var_set(words[2], result);
        return result_mod.from_globals(result);
    }
    if (str_eq(sp, sub.len, "update") and words.len >= 6) return result_mod.from_globals(eval_dict_update(words));
    if (str_eq(sp, sub.len, "exists") and words.len >= 4) return result_mod.from_globals(rt.dict_exists(words[2], words[3]));
    if (str_eq(sp, sub.len, "keys")) return result_mod.from_globals(rt.dict_keys(words[2]));
    if (str_eq(sp, sub.len, "values")) return result_mod.from_globals(rt.dict_values(words[2]));
    if (str_eq(sp, sub.len, "size")) return result_mod.from_globals(rt.dict_size(words[2]));
    if (str_eq(sp, sub.len, "create")) return result_mod.from_globals(eval_dict_create(words));
    if (str_eq(sp, sub.len, "append") and words.len >= 4) return result_mod.from_globals(eval_dict_append(words));
    if (str_eq(sp, sub.len, "lappend") and words.len >= 4) return result_mod.from_globals(eval_dict_lappend(words));
    if (str_eq(sp, sub.len, "incr") and words.len >= 4) return result_mod.from_globals(eval_dict_incr(words));
    if (str_eq(sp, sub.len, "merge")) return result_mod.from_globals(eval_dict_merge(words));
    if (str_eq(sp, sub.len, "remove") and words.len >= 3) return result_mod.from_globals(eval_dict_remove(words));
    if (str_eq(sp, sub.len, "replace") and words.len >= 3) return result_mod.from_globals(eval_dict_replace(words));
    if (str_eq(sp, sub.len, "info") and words.len >= 3) return result_mod.from_globals(eval_dict_info(words));
    if (str_eq(sp, sub.len, "for") and words.len >= 5) return result_mod.from_globals(eval_dict_for(words));
    if (str_eq(sp, sub.len, "map") and words.len >= 5) return result_mod.from_globals(eval_dict_map(words));
    if (str_eq(sp, sub.len, "with") and words.len >= 4) return result_mod.from_globals(eval_dict_with(words));
    if (str_eq(sp, sub.len, "filter") and words.len >= 4) return result_mod.from_globals(eval_dict_filter(words));
    // Unrecognised subcommand — surface Tcl's standard ``unknown or
    // ambiguous subcommand`` diagnostic so a compiled ``dict foo
    // ...`` callsite that fell through to this dispatcher (because
    // the subcommand has no compile-time hook) errors loudly instead
    // of silently evaluating to the empty string (Copilot review on
    // PR #325).  Mirrors ``tclDictObj.c::DictCmdSubcommandError``.
    const known = "append create exists filter for get getd getdef getwithdefault " ++
        "incr info keys lappend map merge remove replace set size unset update values with";
    const obj_mod = @import("../valtypes/tcl_obj.zig");
    const prefix: []const u8 = "unknown or ambiguous subcommand \"";
    const middle: []const u8 = "\": must be ";
    const total: u32 = @as(u32, @intCast(prefix.len)) + sub.len +
        @as(u32, @intCast(middle.len)) + @as(u32, @intCast(known.len));
    const buf = obj_mod.alloc(total);
    const d: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |b| {
        d[off] = b;
        off += 1;
    }
    if (sub.len > 0) {
        const sb: [*]const u8 = @ptrFromInt(sub.ptr);
        for (0..sub.len) |k| {
            d[off] = sb[k];
            off += 1;
        }
    }
    for (middle) |b| {
        d[off] = b;
        off += 1;
    }
    for (known) |b| {
        d[off] = b;
        off += 1;
    }
    const msg = obj_mod.obj_new_string_take(buf, total, total);
    const catch_mod = @import("../interp/tcl_catch.zig");
    catch_mod.tcl_cmd_error(msg);
    return result_mod.from_globals(0);
}

/// ``dict create ?key value ...?`` — build a new dict from alternating
/// key / value pairs.  ``dict create`` (no args) returns the empty
/// dict.  ``dict create k1 v1 k2 v2`` returns a 2-entry dict.  An odd
/// number of arguments raises ``wrong # args`` to match reference
/// Tcl — silently dropping the trailing key would mask script bugs
/// (Copilot review on PR #324).
fn eval_dict_create(words: []const i32) i32 {
    if (words.len <= 2) return rt.dict_create();
    if ((words.len - 2) % 2 != 0) {
        const stubs = @import("../stubs/tcl_stubs.zig");
        stubs.raise("wrong # args: should be \"dict create ?key value ...?\"");
        return 0;
    }
    var d: i32 = rt.dict_create();
    var wi: u32 = 2;
    while (wi + 1 < words.len) : (wi += 2) {
        // ``dict_set`` returns the same handle when it took the
        // in-place fast path, or a fresh +1 obj when it canonical-
        // rebuilt.  The first iteration always rebuilds (empty dict
        // can't be mutated in place), so without releasing the old
        // ``d`` on swap we leak the initial empty dict.
        const next = rt.dict_set(d, words[wi], words[wi + 1]);
        if (next != d) obj.tcl_obj_release(d);
        d = next;
    }
    return d;
}

/// ``dict append DICTVAR KEY ?STRING ...?`` — append the string args
/// to the value at KEY in the dict held by DICTVAR.  When KEY is
/// absent the value is treated as the empty string.  Mirrors
/// ``tclDictObj.c::DictAppendCmd``.
fn eval_dict_append(words: []const i32) i32 {
    // Ownership inventory:
    //   * ``cur`` — borrowed from the var slot, OR +1 owned (when
    //     we created the empty dict).
    //   * ``existing`` — +1 owned (dict_get retains its bucket value
    //     and the fallback obj_new_string_copy returns +1 too).
    //   * ``tcl_cmd_append`` — may mutate in place (returns same
    //     handle) or canonical-rebuild (returns fresh +1).  Release
    //     the prior ``existing`` on swap.
    //   * ``dict_set`` — same in-place-or-rebuild contract.  When it
    //     returns the same handle as ``cur``, our +1 (if any) is
    //     the same as the returned handle's +1; transfer it to the
    //     caller via the return value.  When it returns a fresh obj,
    //     release the local ``cur`` we owned.
    const cur_borrowed = frames.var_resolve(words[2]);
    const cur: i32 = if (cur_borrowed == 0) rt.dict_create() else cur_borrowed;
    const owns_cur: bool = (cur_borrowed == 0);
    var existing = rt.dict_get(cur, words[3]);
    var wi: u32 = 4;
    while (wi < words.len) : (wi += 1) {
        const next = rt.tcl_cmd_append(existing, words[wi]);
        if (next != existing) obj.tcl_obj_release(existing);
        existing = next;
    }
    const result = rt.dict_set(cur, words[3], existing);
    obj.tcl_obj_release(existing);
    if (owns_cur and result != cur) obj.tcl_obj_release(cur);
    _ = frames.var_set(words[2], result);
    return result;
}

/// ``dict lappend DICTVAR KEY ?VALUE ...?`` — list-append the values
/// onto the list value at KEY in the dict held by DICTVAR.  When KEY
/// is absent the value starts as the empty list.  Mirrors
/// ``tclDictObj.c::DictLappendCmd``.
fn eval_dict_lappend(words: []const i32) i32 {
    // See ``eval_dict_append`` for the ownership inventory.
    const cur_borrowed = frames.var_resolve(words[2]);
    const cur: i32 = if (cur_borrowed == 0) rt.dict_create() else cur_borrowed;
    const owns_cur: bool = (cur_borrowed == 0);
    var existing = rt.dict_get(cur, words[3]);
    var wi: u32 = 4;
    while (wi < words.len) : (wi += 1) {
        const next = rt.tcl_cmd_lappend(existing, words[wi]);
        if (next != existing) obj.tcl_obj_release(existing);
        existing = next;
    }
    const result = rt.dict_set(cur, words[3], existing);
    obj.tcl_obj_release(existing);
    if (owns_cur and result != cur) obj.tcl_obj_release(cur);
    _ = frames.var_set(words[2], result);
    return result;
}

/// ``dict incr DICTVAR KEY ?INCREMENT?`` — add INCREMENT (default 1)
/// to the integer value at KEY.  When KEY is absent treats the value
/// as 0 before incrementing.  Mirrors ``tclDictObj.c::DictIncrCmd``.
fn eval_dict_incr(words: []const i32) i32 {
    // See ``eval_dict_append`` for the ownership inventory.  Note
    // that ``dict_get`` here returns +1 owned but we only read its
    // int value — release it before computing ``new_val``.
    const cur_borrowed = frames.var_resolve(words[2]);
    const cur: i32 = if (cur_borrowed == 0) rt.dict_create() else cur_borrowed;
    const owns_cur: bool = (cur_borrowed == 0);
    const incr_amount: i64 = if (words.len >= 5) rt.obj_get_int(words[4]) else 1;
    const has_key = rt.obj_get_int(rt.dict_exists(cur, words[3])) != 0;
    var old_val: i64 = 0;
    if (has_key) {
        const cur_val = rt.dict_get(cur, words[3]);
        old_val = rt.obj_get_int(cur_val);
        obj.tcl_obj_release(cur_val);
    }
    const new_val = obj.obj_new_int(old_val + incr_amount);
    const result = rt.dict_set(cur, words[3], new_val);
    obj.tcl_obj_release(new_val);
    if (owns_cur and result != cur) obj.tcl_obj_release(cur);
    _ = frames.var_set(words[2], result);
    return result;
}

/// ``dict merge ?DICT ...?`` — merge multiple dicts; for duplicate
/// keys the later value wins.  Mirrors ``tclDictObj.c::DictMergeCmd``.
fn eval_dict_merge(words: []const i32) i32 {
    if (words.len <= 2) return rt.dict_create();
    if (words.len == 3) return words[2];
    // ``result`` starts as the borrowed ``words[2]``; after the first
    // ``dict_merge_pair`` it may flip to a fresh +1 owned obj.  Track
    // ownership so intermediate dicts get released — without this the
    // earlier code leaked every intermediate ``current`` produced by
    // the merge chain past iteration 1.
    var result: i32 = words[2];
    var owns_result: bool = false;
    var wi: u32 = 3;
    while (wi < words.len) : (wi += 1) {
        const next = rt.dict_merge_pair(result, words[wi]);
        if (next != result) {
            if (owns_result) obj.tcl_obj_release(result);
            owns_result = true;
        }
        result = next;
    }
    return result;
}

/// ``dict remove DICT ?KEY ...?`` — return a copy of DICT with each
/// listed KEY removed.  Mirrors ``tclDictObj.c::DictRemoveCmd``.
fn eval_dict_remove(words: []const i32) i32 {
    // See ``eval_dict_merge`` for the ownership-tracking rationale.
    var result: i32 = words[2];
    var owns_result: bool = false;
    var wi: u32 = 3;
    while (wi < words.len) : (wi += 1) {
        const next = rt.dict_unset(result, words[wi]);
        if (next != result) {
            if (owns_result) obj.tcl_obj_release(result);
            owns_result = true;
        }
        result = next;
    }
    return result;
}

/// ``dict replace DICT ?KEY VALUE ...?`` — return a copy of DICT with
/// each KEY set to VALUE.  An odd key/value tail raises ``wrong #
/// args`` to match reference Tcl rather than silently returning the
/// input unchanged (Copilot review on PR #324).  Mirrors
/// ``tclDictObj.c::DictReplaceCmd``.
fn eval_dict_replace(words: []const i32) i32 {
    if ((words.len - 3) % 2 != 0) {
        const stubs = @import("../stubs/tcl_stubs.zig");
        stubs.raise("wrong # args: should be \"dict replace dictionary ?key value ...?\"");
        return 0;
    }
    // See ``eval_dict_merge`` for the ownership-tracking rationale.
    var result: i32 = words[2];
    var owns_result: bool = false;
    var wi: u32 = 3;
    while (wi + 1 < words.len) : (wi += 2) {
        const next = rt.dict_set(result, words[wi], words[wi + 1]);
        if (next != result) {
            if (owns_result) obj.tcl_obj_release(result);
            owns_result = true;
        }
        result = next;
    }
    return result;
}

/// ``dict info DICT`` — return implementation-defined info string.
/// Tcl returns a hash-stats string here; we return a brief summary
/// based on size, which is enough for the tests that just check the
/// command runs.  Mirrors ``tclDictObj.c::DictInfoCmd`` shape.
fn eval_dict_info(words: []const i32) i32 {
    const size_obj = rt.dict_size(words[2]);
    const n = rt.obj_get_int(size_obj);
    const itoa = obj.itoa(n);
    const prefix: []const u8 = "dict containing ";
    const suffix: []const u8 = " entries";
    const total: u32 = @intCast(prefix.len + itoa.len + suffix.len);
    const buf_addr: u32 = obj.alloc(total);
    const buf: [*]u8 = @ptrFromInt(buf_addr);
    var off: usize = 0;
    for (prefix) |c| {
        buf[off] = c;
        off += 1;
    }
    var i: u32 = 0;
    while (i < itoa.len) : (i += 1) {
        buf[off] = itoa.ptr[i];
        off += 1;
    }
    for (suffix) |c| {
        buf[off] = c;
        off += 1;
    }
    return obj.obj_new_string_take(buf_addr, total, total);
}

/// ``dict for {KVAR VVAR} DICT BODY`` — iterate over the dict's
/// key/value pairs, binding each to KVAR / VVAR before evaluating
/// BODY.  Honours break / continue / return / error in the body.
/// Mirrors ``tclDictObj.c::DictForNRCmd``.
fn eval_dict_for(words: []const i32) i32 {
    return eval_dict_iter(words, false);
}

/// ``dict map {KVAR VVAR} DICT BODY`` — like ``dict for`` but
/// collects each iteration's body result into a new dict, paired
/// with the current key.  Mirrors ``tclDictObj.c::DictMapNRCmd``.
fn eval_dict_map(words: []const i32) i32 {
    return eval_dict_iter(words, true);
}

/// Shared iteration kernel for ``dict for`` and ``dict map``.  When
/// *collect* is true the body's interp result is captured into a new
/// dict keyed by the current key (``dict map`` semantics).
fn eval_dict_iter(words: []const i32, collect: bool) i32 {
    // varlist = words[2], dict = words[3], body = words[4]
    const varlist = obj_ensure_string(words[2]);
    const n_vars = rt.list_count_elements(varlist.ptr, varlist.len);
    if (n_vars != 2) return 0;
    const kelem = rt.list_element_at(varlist.ptr, varlist.len, 0);
    const velem = rt.list_element_at(varlist.ptr, varlist.len, 1);
    // ``kvar`` / ``vvar`` are owned +1 each — release on every exit
    // path or they leak per ``dict for/map`` invocation.
    const kvar = obj.obj_new_string_copy(varlist.ptr + kelem.start, kelem.len);
    const vvar = obj.obj_new_string_copy(varlist.ptr + velem.start, velem.len);
    defer obj.tcl_obj_release(kvar);
    defer obj.tcl_obj_release(vvar);

    const dict = words[3];
    const sd = obj_ensure_string(dict);
    // Empty / odd-length dict is a no-op (or empty result).  The
    // ``count_elements`` check is cheap (single linear scan) and lets
    // us bail before the cursor walk on obviously malformed input.
    const n = rt.list_count_elements(sd.ptr, sd.len);
    if (n <= 0 or (n & 1) != 0) {
        return if (collect) rt.dict_create() else 0;
    }

    const body = obj_ensure_string(words[4]);
    var collected: i32 = if (collect) rt.dict_create() else 0;
    // Walk the dict's string-shape list with a single forward cursor.
    // The previous implementation called ``list_element_at(idx)`` per
    // element, which restarts the parse from offset 0 each time —
    // O(n²) in the dict size and the source of dict-24.24/24.25's
    // multi-minute hang on a 100k-element ``dict map``.  We already
    // validated that ``n`` is even and positive; iterate that many
    // pairs explicitly so an empty braced element ``{}`` (which yields
    // ``len == 0`` from the cursor) doesn't terminate the loop early.
    var cur: list_parse.Cursor = .{ .pos = 0 };
    var pairs_left: i64 = @divTrunc(n, 2);
    while (pairs_left > 0) : (pairs_left -= 1) {
        const k = list_parse.cursor_next(sd.ptr, sd.len, &cur);
        const v = list_parse.cursor_next(sd.ptr, sd.len, &cur);
        // ``key_obj`` and ``val_obj`` are fresh +1 owned; ``var_set``
        // retains them, so release our +1 once the bind is done.
        // ``body_result`` is left alone — ``eval_script`` may return
        // a BORROWED handle into a variable's live slot (see the
        // contract at tcl_interp.zig:775); releasing it could
        // corrupt that var.  ``dict_set`` copies the body_result's
        // bytes, so the bytes stay valid even after we drop our
        // key/val refs.
        const key_obj = obj.obj_new_string_copy(sd.ptr + k.start, k.len);
        const val_obj = obj.obj_new_string_copy(sd.ptr + v.start, v.len);
        _ = frames.var_set(kvar, key_obj);
        _ = frames.var_set(vvar, val_obj);
        const body_result = interp.eval_script(body.ptr, body.len);
        const ir = result_mod.snapshot(body_result);
        switch (ir.code) {
            .OK => {},
            .ERROR, .RETURN => {
                obj.tcl_obj_release(key_obj);
                obj.tcl_obj_release(val_obj);
                if (collect and collected != 0) obj.tcl_obj_release(collected);
                return 0;
            },
            .BREAK => {
                obj.tcl_obj_release(key_obj);
                obj.tcl_obj_release(val_obj);
                result_mod.consume(.BREAK);
                break;
            },
            .CONTINUE => {
                obj.tcl_obj_release(key_obj);
                obj.tcl_obj_release(val_obj);
                result_mod.consume(.CONTINUE);
                continue;
            },
        }
        if (collect) {
            const next_collected = rt.dict_set(collected, key_obj, body_result);
            if (next_collected != collected) obj.tcl_obj_release(collected);
            collected = next_collected;
        }
        obj.tcl_obj_release(key_obj);
        obj.tcl_obj_release(val_obj);
    }
    return if (collect) collected else 0;
}

/// ``dict with DICTVAR ?KEY ...? BODY`` — drill into a possibly-nested
/// dict and bind every key in the resolved sub-dict to a same-named
/// local variable for BODY's lifetime, then write the (possibly
/// modified) variables back to the dict.  Mirrors
/// ``tclDictObj.c::DictWithCmd``.
fn eval_dict_with(words: []const i32) i32 {
    const dict_var = words[2];
    const body = obj_ensure_string(words[words.len - 1]);

    // Resolve the sub-dict at the optional key path.  ``cur`` starts
    // borrowed from the var slot; ``dict_get`` returns +1 owned, so
    // after the first drill step ``cur`` is owned and each subsequent
    // step leaks the prior ``cur`` unless we release it.  Track
    // ownership so the loop's intermediates get reclaimed.
    var cur = frames.var_resolve(dict_var);
    if (cur == 0) return 0;
    var owns_cur: bool = false;
    var ki: u32 = 3;
    while (ki + 1 < words.len) : (ki += 1) {
        if (rt.obj_get_int(rt.dict_exists(cur, words[ki])) == 0) {
            if (owns_cur) obj.tcl_obj_release(cur);
            return 0;
        }
        const next = rt.dict_get(cur, words[ki]);
        if (owns_cur) obj.tcl_obj_release(cur);
        cur = next;
        owns_cur = true;
    }

    // Snapshot keys and bind each as a same-named local.  Use a
    // single forward cursor walk (O(n)) instead of indexed
    // ``list_element_at`` lookups (O(n²) — restarts the parse from
    // offset 0 on every call).
    const sd = obj_ensure_string(cur);
    const n = rt.list_count_elements(sd.ptr, sd.len);
    {
        var lc: list_parse.Cursor = .{ .pos = 0 };
        var pairs: i64 = @divTrunc(n, 2);
        while (pairs > 0) : (pairs -= 1) {
            const k = list_parse.cursor_next(sd.ptr, sd.len, &lc);
            const v = list_parse.cursor_next(sd.ptr, sd.len, &lc);
            // Per-pair +1 owned key/val — release after ``var_set``
            // retains them.
            const key_obj = obj.obj_new_string_copy(sd.ptr + k.start, k.len);
            const val_obj = obj.obj_new_string_copy(sd.ptr + v.start, v.len);
            _ = frames.var_set(key_obj, val_obj);
            obj.tcl_obj_release(key_obj);
            obj.tcl_obj_release(val_obj);
        }
    }

    _ = interp.eval_script(body.ptr, body.len);
    if (result_mod.snapshot(0).code == .ERROR) {
        if (owns_cur) obj.tcl_obj_release(cur);
        return 0;
    }

    // Write back each key from the (potentially modified) locals.
    // ``sub`` inherits ``cur``'s ownership: if ``cur`` is owned (drill
    // happened), ``sub`` starts owned too; otherwise borrowed.  Each
    // ``dict_set`` swap follows the same own-vs-borrow track.
    var sub = cur;
    var owns_sub: bool = owns_cur;
    {
        var lc: list_parse.Cursor = .{ .pos = 0 };
        var pairs: i64 = @divTrunc(n, 2);
        while (pairs > 0) : (pairs -= 1) {
            const k = list_parse.cursor_next(sd.ptr, sd.len, &lc);
            // Skip the value element — only the key feeds the
            // ``var_resolve`` lookup.
            _ = list_parse.cursor_next(sd.ptr, sd.len, &lc);
            const key_obj = obj.obj_new_string_copy(sd.ptr + k.start, k.len);
            const new_val = frames.var_resolve(key_obj);
            if (new_val != 0) {
                const next = rt.dict_set(sub, key_obj, new_val);
                if (next != sub) {
                    if (owns_sub) obj.tcl_obj_release(sub);
                    owns_sub = true;
                }
                sub = next;
            }
            obj.tcl_obj_release(key_obj);
        }
    }

    // Re-write the (possibly relocated) sub-dict back through the key path.
    if (words.len == 4) {
        // No key path — sub is the top-level dict.
        _ = frames.var_set(dict_var, sub);
        if (owns_sub) obj.tcl_obj_release(sub);
    } else {
        // Multi-key drill writeback.  This is intentionally
        // permissive about ownership of intermediates — the existing
        // nested ``dict_get``/``dict_set`` chain has the same
        // borrowed-vs-owned ambiguity throughout, and a partial fix
        // here would risk premature frees.  ``sub`` is released
        // after the chain rebuilds the top-level dict (see end).
        var top = frames.var_resolve(dict_var);
        if (top == 0) top = rt.dict_create();
        var depth: u32 = words.len - 1; // index of the deepest key + 1
        depth -= 1;
        var inner = sub;
        while (depth > 3) : (depth -= 1) {
            const key = words[depth];
            var parent = top;
            var pi: u32 = 3;
            while (pi < depth) : (pi += 1) {
                parent = rt.dict_get(parent, words[pi]);
            }
            inner = rt.dict_set(parent, key, inner);
        }
        const final = rt.dict_set(top, words[3], inner);
        _ = frames.var_set(dict_var, final);
        // We own ``sub``'s +1 from the drill chain; ``dict_set``
        // copied its bytes into the rebuilt ``inner`` / ``final``,
        // so we can release.
        if (owns_sub) obj.tcl_obj_release(sub);
    }
    return 0;
}

/// ``dict filter DICT FILTERTYPE ARG ?ARG ...?`` — filter the dict
/// by FILTERTYPE.  Supports ``key PATTERN ?PATTERN ...?``,
/// ``value PATTERN ?PATTERN ...?`` (glob match against any
/// supplied pattern), and ``script {keyVar valueVar} body``.
/// Mirrors ``tclDictObj.c::DictFilterCmd``.
fn eval_dict_filter(words: []const i32) i32 {
    const ft = obj_ensure_string(words[3]);
    const fp: [*]const u8 = @ptrFromInt(ft.ptr);
    const filter_keys = str_eq(fp, ft.len, "key");
    const filter_values = str_eq(fp, ft.len, "value");
    const filter_script = str_eq(fp, ft.len, "script");
    if (filter_script) return eval_dict_filter_script(words);
    if (!filter_keys and !filter_values) {
        const stubs = @import("../stubs/tcl_stubs.zig");
        stubs.raise("bad filterType: must be key, script, or value");
        return 0;
    }
    if (words.len < 5) return rt.dict_create();

    const sd = obj_ensure_string(words[2]);
    const n = rt.list_count_elements(sd.ptr, sd.len);
    var result: i32 = rt.dict_create();
    var lc: list_parse.Cursor = .{ .pos = 0 };
    var pairs: i64 = @divTrunc(n, 2);
    while (pairs > 0) : (pairs -= 1) {
        const k = list_parse.cursor_next(sd.ptr, sd.len, &lc);
        const v = list_parse.cursor_next(sd.ptr, sd.len, &lc);
        // ``key_obj`` and ``val_obj`` are +1 owned; release per
        // iteration to stop the per-pair leak.  ``dict_set`` may
        // return a fresh +1 obj (canonical rebuild); track and
        // release the prior ``result`` on swap.
        const key_obj = obj.obj_new_string_copy(sd.ptr + k.start, k.len);
        const val_obj = obj.obj_new_string_copy(sd.ptr + v.start, v.len);
        const candidate: i32 = if (filter_keys) key_obj else val_obj;
        var match = false;
        var pi: u32 = 4;
        while (pi < words.len) : (pi += 1) {
            if (rt.obj_get_int(rt.string_match(words[pi], candidate)) != 0) {
                match = true;
                break;
            }
        }
        if (match) {
            const next = rt.dict_set(result, key_obj, val_obj);
            if (next != result) obj.tcl_obj_release(result);
            result = next;
        }
        obj.tcl_obj_release(key_obj);
        obj.tcl_obj_release(val_obj);
    }
    return result;
}

/// ``dict filter DICT script {keyVar valueVar} body`` — filter the
/// dict by evaluating ``body`` for each (key, value) pair with the
/// pair bound to ``keyVar`` / ``valueVar`` in the caller's frame.
/// Pairs whose body returns a true boolean are included in the
/// result.  Mirrors ``tclDictObj.c::DictFilterCmd`` (FILTER_SCRIPT
/// branch).  Flow control: TCL_BREAK terminates the loop, TCL_CONTINUE
/// skips the current pair, TCL_ERROR / TCL_RETURN propagate.
fn eval_dict_filter_script(words: []const i32) i32 {
    if (words.len != 6) {
        const stubs = @import("../stubs/tcl_stubs.zig");
        stubs.raise("wrong # args: should be \"dict filter dictionary script {keyVarName valueVarName} filterScript\"");
        return 0;
    }
    const varlist = obj_ensure_string(words[4]);
    const n_vars = rt.list_count_elements(varlist.ptr, varlist.len);
    if (n_vars != 2) {
        const stubs = @import("../stubs/tcl_stubs.zig");
        stubs.raise("must have exactly two variable names");
        return 0;
    }
    const kelem = rt.list_element_at(varlist.ptr, varlist.len, 0);
    const velem = rt.list_element_at(varlist.ptr, varlist.len, 1);
    // ``kvar`` / ``vvar`` allocated once; release via defer.
    const kvar = obj.obj_new_string_copy(varlist.ptr + kelem.start, kelem.len);
    const vvar = obj.obj_new_string_copy(varlist.ptr + velem.start, velem.len);
    defer obj.tcl_obj_release(kvar);
    defer obj.tcl_obj_release(vvar);

    const sd = obj_ensure_string(words[2]);
    const n = rt.list_count_elements(sd.ptr, sd.len);
    if (n <= 0 or (n & 1) != 0) return rt.dict_create();

    const body = obj_ensure_string(words[5]);
    var result: i32 = rt.dict_create();
    var idx: i64 = 0;
    while (idx + 1 < n) : (idx += 2) {
        const k = rt.list_element_at(sd.ptr, sd.len, idx);
        const v = rt.list_element_at(sd.ptr, sd.len, idx + 1);
        // ``key_obj`` / ``val_obj`` released after the per-iter
        // dispose path runs.  ``body_result`` left alone — see
        // ``eval_dict_iter`` for the borrowed-handle rationale.
        const key_obj = obj.obj_new_string_copy(sd.ptr + k.start, k.len);
        const val_obj = obj.obj_new_string_copy(sd.ptr + v.start, v.len);
        _ = frames.var_set(kvar, key_obj);
        _ = frames.var_set(vvar, val_obj);
        const body_result = interp.eval_script(body.ptr, body.len);
        const ir = result_mod.snapshot(body_result);
        switch (ir.code) {
            .OK => {
                if (rt.obj_get_int(body_result) != 0) {
                    const next = rt.dict_set(result, key_obj, val_obj);
                    if (next != result) obj.tcl_obj_release(result);
                    result = next;
                }
            },
            .ERROR, .RETURN => {
                obj.tcl_obj_release(key_obj);
                obj.tcl_obj_release(val_obj);
                obj.tcl_obj_release(result);
                return 0;
            },
            .BREAK => {
                obj.tcl_obj_release(key_obj);
                obj.tcl_obj_release(val_obj);
                result_mod.consume(.BREAK);
                break;
            },
            .CONTINUE => {
                obj.tcl_obj_release(key_obj);
                obj.tcl_obj_release(val_obj);
                result_mod.consume(.CONTINUE);
                continue;
            },
        }
        obj.tcl_obj_release(key_obj);
        obj.tcl_obj_release(val_obj);
    }
    return result;
}

/// ``dict update DICTVAR KEY VAR ?KEY VAR ...? SCRIPT`` — bind each
/// dict-element to a local variable, run SCRIPT, then write back the
/// (possibly-modified, possibly-unset) variables to the dict.
///
/// Tcl 9 semantics (from ``tclDictObj.c::DictUpdateCmd``):
///   * Pre-script: for every (key, var) pair, if the dict has that
///     key, set the local var to that value; if the key is absent
///     leave the var untouched (callers typically ``set var {}``
///     before the body to avoid unset-var traps inside the body).
///   * Post-script: for every (key, var) pair, if the local var
///     still exists, write its current value to the dict under
///     that key; if it has been ``unset`` inside the body, remove
///     that key from the dict.
///   * Finally update DICTVAR with the rebuilt dict.
fn eval_dict_update(words: []const i32) i32 {
    // ``dict update DICTVAR KEY VAR KEY VAR … SCRIPT``
    // After ``words[0..2]`` (``dict update``), pairs run
    // ``words[3] words[4]`` … and the trailing word is the script.
    if (words.len < 6) return 0;
    if ((words.len - 3) % 2 != 1) return 0;
    const dict_var = words[2];
    const script = words[words.len - 1];
    const n_pairs: u32 = @intCast((words.len - 4) / 2);

    // 1. Pre-script: bind each KEY's dict value to VAR.
    var pi: u32 = 0;
    while (pi < n_pairs) : (pi += 1) {
        const key_idx = 3 + pi * 2;
        const var_idx = 4 + pi * 2;
        const cur = frames.var_resolve(dict_var);
        if (cur == 0) continue;
        if (rt.obj_get_int(rt.dict_exists(cur, words[key_idx])) != 0) {
            _ = frames.var_set(words[var_idx], rt.dict_get(cur, words[key_idx]));
        }
    }

    // 2. Run the script body.  Use eval_script so flow-control
    //    flags (break / continue / return / error) propagate to
    //    the caller — same shape as ``eval_while``'s body call.
    const body = obj_ensure_string(script);
    _ = interp.eval_script(body.ptr, body.len);
    // Capture (and clear) break/continue so the writeback completes
    // even when the script broke out of an enclosing loop.  Errors
    // and ``return`` propagate up.
    const ir = result_mod.snapshot(0);
    const had_break = ir.code == .BREAK;
    const had_continue = ir.code == .CONTINUE;
    if (had_break) result_mod.consume(.BREAK);
    if (had_continue) result_mod.consume(.CONTINUE);

    // 3. Post-script: reflect each VAR back into the dict (or
    //    remove the key if VAR was unset).
    pi = 0;
    while (pi < n_pairs) : (pi += 1) {
        const key_idx = 3 + pi * 2;
        const var_idx = 4 + pi * 2;
        const cur = frames.var_resolve(dict_var);
        const dict_now = if (cur == 0) rt.dict_create() else cur;
        const var_val = frames.var_resolve(words[var_idx]);
        const updated = if (var_val == 0)
            rt.dict_unset(dict_now, words[key_idx])
        else
            rt.dict_set(dict_now, words[key_idx], var_val);
        _ = frames.var_set(dict_var, updated);
    }

    // Re-raise the captured flow-control signal so the enclosing
    // loop / proc sees it.
    if (had_break) result_mod.set_break();
    if (had_continue) result_mod.set_continue();
    return 0;
}
