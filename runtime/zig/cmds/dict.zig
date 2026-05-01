// Tcl ``dict`` built-in command.
//
// Extracted from tcl_interp_string.zig.  Registers itself in the
// central command table via the ``registration`` constant.

const rt = @import("../tcl_runtime.zig");
const frames = @import("../interp/tcl_frames.zig");
const interp = @import("../interp/tcl_interp.zig");

const obj_ensure_string = rt.obj_ensure_string;

const str_eq = @import("../valtypes/tcl_chars.zig").str_eq;

const reg = @import("../dispatch/tcl_cmd_registry.zig");

pub const registration = reg.CmdEntry{
    .name = "dict",
    .arity_min = 1, .arity_max = null, .handler = &eval,
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

pub fn eval(words: []const i32) i32 {
    if (words.len < 3) return 0;
    const sub = obj_ensure_string(words[1]);
    const sp: [*]const u8 = @ptrFromInt(sub.ptr);
    if (str_eq(sp, sub.len, "get") and words.len >= 4) return rt.dict_get(words[2], words[3]);
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
        const default_obj = words[words.len - 1];
        var cur: i32 = words[2];
        var ki: u32 = 3;
        while (ki + 1 < words.len) : (ki += 1) {
            if (rt.obj_get_int(rt.dict_exists(cur, words[ki])) == 0) {
                return default_obj;
            }
            cur = rt.dict_get(cur, words[ki]);
        }
        return cur;
    }
    if (str_eq(sp, sub.len, "set") and words.len >= 5) {
        const cur = frames.var_resolve(words[2]);
        const result = rt.dict_set(cur, words[3], words[4]);
        _ = frames.var_set(words[2], result);
        return result;
    }
    if (str_eq(sp, sub.len, "unset") and words.len >= 4) {
        const cur = frames.var_resolve(words[2]);
        const result = rt.dict_unset(cur, words[3]);
        _ = frames.var_set(words[2], result);
        return result;
    }
    if (str_eq(sp, sub.len, "update") and words.len >= 6) return eval_dict_update(words);
    if (str_eq(sp, sub.len, "exists") and words.len >= 4) return rt.dict_exists(words[2], words[3]);
    if (str_eq(sp, sub.len, "keys")) return rt.dict_keys(words[2]);
    if (str_eq(sp, sub.len, "values")) return rt.dict_values(words[2]);
    if (str_eq(sp, sub.len, "size")) return rt.dict_size(words[2]);
    if (str_eq(sp, sub.len, "create")) return rt.dict_create();
    if (str_eq(sp, sub.len, "append") and words.len >= 4) return eval_dict_append(words);
    if (str_eq(sp, sub.len, "lappend") and words.len >= 4) return eval_dict_lappend(words);
    if (str_eq(sp, sub.len, "incr") and words.len >= 4) return eval_dict_incr(words);
    if (str_eq(sp, sub.len, "for") and words.len == 5) return eval_dict_for(words);
    if (str_eq(sp, sub.len, "merge")) return eval_dict_merge(words);
    if (str_eq(sp, sub.len, "remove")) return eval_dict_remove(words);
    if (str_eq(sp, sub.len, "replace")) return eval_dict_replace(words);
    if (str_eq(sp, sub.len, "info") and words.len == 3) return eval_dict_info(words);
    return 0;
}

/// ``dict append DICTVAR KEY ?VALUE ...?`` — concatenate VALUEs onto
/// the existing string at KEY (creating an empty entry first if the
/// key isn't present, matching Tcl 9 semantics).
fn eval_dict_append(words: []const i32) i32 {
    var cur = frames.var_resolve(words[2]);
    if (cur == 0) cur = rt.dict_create();
    const key = words[3];
    var existing = if (rt.obj_get_int(rt.dict_exists(cur, key)) != 0)
        rt.dict_get(cur, key)
    else
        rt.obj_new_string(0, 0);
    var wi: u32 = 4;
    while (wi < words.len) : (wi += 1) {
        existing = rt.tcl_cmd_append(existing, words[wi]);
    }
    const updated = rt.dict_set(cur, key, existing);
    _ = frames.var_set(words[2], updated);
    return updated;
}

/// ``dict lappend DICTVAR KEY ?VALUE ...?`` — list-append VALUEs onto
/// the existing list at KEY (creating an empty list first if the key
/// isn't present).
fn eval_dict_lappend(words: []const i32) i32 {
    var cur = frames.var_resolve(words[2]);
    if (cur == 0) cur = rt.dict_create();
    const key = words[3];
    var existing = if (rt.obj_get_int(rt.dict_exists(cur, key)) != 0)
        rt.dict_get(cur, key)
    else
        rt.obj_new_string(0, 0);
    var wi: u32 = 4;
    while (wi < words.len) : (wi += 1) {
        existing = rt.tcl_cmd_lappend(existing, words[wi]);
    }
    const updated = rt.dict_set(cur, key, existing);
    _ = frames.var_set(words[2], updated);
    return updated;
}

/// ``dict incr DICTVAR KEY ?INCREMENT?`` — increment integer at KEY by
/// INCREMENT (default 1).  Creates KEY=0 first if missing.
fn eval_dict_incr(words: []const i32) i32 {
    var cur = frames.var_resolve(words[2]);
    if (cur == 0) cur = rt.dict_create();
    const key = words[3];
    const existing_i: i64 = if (rt.obj_get_int(rt.dict_exists(cur, key)) != 0)
        rt.obj_get_int(rt.dict_get(cur, key))
    else
        0;
    const inc_i: i64 = if (words.len >= 5) rt.obj_get_int(words[4]) else 1;
    const new_val = rt.obj_new_int(existing_i + inc_i);
    const updated = rt.dict_set(cur, key, new_val);
    _ = frames.var_set(words[2], updated);
    return updated;
}

/// ``dict for {keyVar valueVar} dict body`` — iterate every key/value
/// pair binding them to the named locals around BODY.  Tcl semantics:
/// returns the empty string on completion, propagates ``return`` /
/// ``error`` upward, treats ``break`` as a clean exit and ``continue``
/// as next-iteration.
fn eval_dict_for(words: []const i32) i32 {
    const var_pair_s = obj_ensure_string(words[2]);
    const valtypes_list = @import("../valtypes/tcl_list_parse.zig");
    const npair = valtypes_list.count_elements(var_pair_s.ptr, var_pair_s.len);
    if (npair != 2) return 0;
    const e0 = valtypes_list.element_at(var_pair_s.ptr, var_pair_s.len, 0);
    const e1 = valtypes_list.element_at(var_pair_s.ptr, var_pair_s.len, 1);
    const key_name = rt.obj_new_string_copy(var_pair_s.ptr + e0.start, e0.len);
    const val_name = rt.obj_new_string_copy(var_pair_s.ptr + e1.start, e1.len);

    const dict = words[3];
    const body = words[4];
    const keys_list = rt.dict_keys(dict);
    const keys_s = obj_ensure_string(keys_list);
    const nkeys = valtypes_list.count_elements(keys_s.ptr, keys_s.len);
    const body_s = obj_ensure_string(body);

    var ki: u32 = 0;
    while (ki < nkeys) : (ki += 1) {
        const ke = valtypes_list.element_at(keys_s.ptr, keys_s.len, @intCast(ki));
        const key_obj = rt.obj_new_string_copy(keys_s.ptr + ke.start, ke.len);
        const val_obj = rt.dict_get(dict, key_obj);
        _ = frames.var_set(key_name, key_obj);
        _ = frames.var_set(val_name, val_obj);
        _ = interp.eval_script(body_s.ptr, body_s.len);
        if (rt.break_flag.* != 0) {
            rt.break_flag.* = 0;
            break;
        }
        if (rt.continue_flag.* != 0) {
            rt.continue_flag.* = 0;
            continue;
        }
        if (rt.return_flag.* != 0) return 0;
    }
    return rt.obj_new_string(0, 0);
}

/// ``dict merge ?dict ...?`` — flatten N dicts into one (later wins
/// on key conflict).  Empty arg list returns the empty dict.
fn eval_dict_merge(words: []const i32) i32 {
    if (words.len < 3) return rt.obj_new_string(0, 0);
    var cur = words[2];
    var wi: u32 = 3;
    while (wi < words.len) : (wi += 1) {
        cur = rt.dict_merge_pair(cur, words[wi]);
    }
    return cur;
}

/// ``dict remove dict ?key ...?`` — return a copy of dict with the
/// listed keys removed.  Missing keys are silently skipped.
fn eval_dict_remove(words: []const i32) i32 {
    if (words.len < 3) return rt.obj_new_string(0, 0);
    var cur = words[2];
    var wi: u32 = 3;
    while (wi < words.len) : (wi += 1) {
        cur = rt.dict_unset(cur, words[wi]);
    }
    return cur;
}

/// ``dict replace dict ?key value ...?`` — return a copy of dict with
/// each key/value pair set.  Equivalent to ``dict set`` chained over
/// many pairs but operating on a value rather than a variable.
fn eval_dict_replace(words: []const i32) i32 {
    if (words.len < 3) return rt.obj_new_string(0, 0);
    if ((words.len - 3) % 2 != 0) return 0;
    var cur = words[2];
    var i: u32 = 3;
    while (i + 1 < words.len) : (i += 2) {
        cur = rt.dict_set(cur, words[i], words[i + 1]);
    }
    return cur;
}

/// ``dict info dict`` — return a human-readable description of the
/// dict's internal representation.  Real Tcl prints hash-bucket
/// statistics; we emit the size + first key as a stub.
fn eval_dict_info(words: []const i32) i32 {
    const size_obj = rt.dict_size(words[2]);
    const size_i = rt.obj_get_int(size_obj);
    const prefix = "dict (";
    const suffix = " entries)";
    var buf: [64]u8 = undefined;
    var off: usize = 0;
    for (prefix) |c| {
        buf[off] = c;
        off += 1;
    }
    // itoa
    var n = size_i;
    if (n < 0) {
        buf[off] = '-';
        off += 1;
        n = -n;
    }
    var digits: [20]u8 = undefined;
    var dlen: usize = 0;
    if (n == 0) {
        digits[0] = '0';
        dlen = 1;
    } else while (n > 0) : (n = @divTrunc(n, 10)) {
        digits[dlen] = @intCast(@as(i64, '0') + @rem(n, 10));
        dlen += 1;
    }
    var di: usize = dlen;
    while (di > 0) {
        di -= 1;
        buf[off] = digits[di];
        off += 1;
    }
    for (suffix) |c| {
        buf[off] = c;
        off += 1;
    }
    return rt.obj_new_string_copy(@intFromPtr(&buf[0]), @intCast(off));
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
    const had_break = rt.break_flag.* != 0;
    const had_continue = rt.continue_flag.* != 0;
    rt.break_flag.* = 0;
    rt.continue_flag.* = 0;

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
    if (had_break) rt.break_flag.* = 1;
    if (had_continue) rt.continue_flag.* = 1;
    return 0;
}
