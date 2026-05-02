// Tcl ``dict`` built-in command.
//
// Extracted from tcl_interp_string.zig.  Registers itself in the
// central command table via the ``registration`` constant.

const rt = @import("../tcl_runtime.zig");
const frames = @import("../interp/tcl_frames.zig");
const interp = @import("../interp/tcl_interp.zig");
const obj = @import("../valtypes/tcl_obj.zig");

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
    if (str_eq(sp, sub.len, "create")) return eval_dict_create(words);
    if (str_eq(sp, sub.len, "append") and words.len >= 4) return eval_dict_append(words);
    if (str_eq(sp, sub.len, "lappend") and words.len >= 4) return eval_dict_lappend(words);
    if (str_eq(sp, sub.len, "incr") and words.len >= 4) return eval_dict_incr(words);
    if (str_eq(sp, sub.len, "merge")) return eval_dict_merge(words);
    if (str_eq(sp, sub.len, "remove") and words.len >= 3) return eval_dict_remove(words);
    if (str_eq(sp, sub.len, "replace") and words.len >= 3) return eval_dict_replace(words);
    if (str_eq(sp, sub.len, "info") and words.len >= 3) return eval_dict_info(words);
    if (str_eq(sp, sub.len, "for") and words.len >= 5) return eval_dict_for(words);
    if (str_eq(sp, sub.len, "map") and words.len >= 5) return eval_dict_map(words);
    if (str_eq(sp, sub.len, "with") and words.len >= 4) return eval_dict_with(words);
    if (str_eq(sp, sub.len, "filter") and words.len >= 4) return eval_dict_filter(words);
    return 0;
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
        d = rt.dict_set(d, words[wi], words[wi + 1]);
    }
    return d;
}

/// ``dict append DICTVAR KEY ?STRING ...?`` — append the string args
/// to the value at KEY in the dict held by DICTVAR.  When KEY is
/// absent the value is treated as the empty string.  Mirrors
/// ``tclDictObj.c::DictAppendCmd``.
fn eval_dict_append(words: []const i32) i32 {
    var cur = frames.var_resolve(words[2]);
    if (cur == 0) cur = rt.dict_create();
    var existing = rt.dict_get(cur, words[3]);
    var wi: u32 = 4;
    while (wi < words.len) : (wi += 1) {
        existing = rt.tcl_cmd_append(existing, words[wi]);
    }
    const result = rt.dict_set(cur, words[3], existing);
    _ = frames.var_set(words[2], result);
    return result;
}

/// ``dict lappend DICTVAR KEY ?VALUE ...?`` — list-append the values
/// onto the list value at KEY in the dict held by DICTVAR.  When KEY
/// is absent the value starts as the empty list.  Mirrors
/// ``tclDictObj.c::DictLappendCmd``.
fn eval_dict_lappend(words: []const i32) i32 {
    var cur = frames.var_resolve(words[2]);
    if (cur == 0) cur = rt.dict_create();
    var existing = rt.dict_get(cur, words[3]);
    var wi: u32 = 4;
    while (wi < words.len) : (wi += 1) {
        existing = rt.tcl_cmd_lappend(existing, words[wi]);
    }
    const result = rt.dict_set(cur, words[3], existing);
    _ = frames.var_set(words[2], result);
    return result;
}

/// ``dict incr DICTVAR KEY ?INCREMENT?`` — add INCREMENT (default 1)
/// to the integer value at KEY.  When KEY is absent treats the value
/// as 0 before incrementing.  Mirrors ``tclDictObj.c::DictIncrCmd``.
fn eval_dict_incr(words: []const i32) i32 {
    var cur = frames.var_resolve(words[2]);
    if (cur == 0) cur = rt.dict_create();
    const incr_amount: i64 = if (words.len >= 5) rt.obj_get_int(words[4]) else 1;
    const has_key = rt.obj_get_int(rt.dict_exists(cur, words[3])) != 0;
    const old_val: i64 = if (has_key) rt.obj_get_int(rt.dict_get(cur, words[3])) else 0;
    const new_val = obj.obj_new_int(old_val + incr_amount);
    const result = rt.dict_set(cur, words[3], new_val);
    _ = frames.var_set(words[2], result);
    return result;
}

/// ``dict merge ?DICT ...?`` — merge multiple dicts; for duplicate
/// keys the later value wins.  Mirrors ``tclDictObj.c::DictMergeCmd``.
fn eval_dict_merge(words: []const i32) i32 {
    if (words.len <= 2) return rt.dict_create();
    if (words.len == 3) return words[2];
    var result: i32 = words[2];
    var wi: u32 = 3;
    while (wi < words.len) : (wi += 1) {
        result = rt.dict_merge_pair(result, words[wi]);
    }
    return result;
}

/// ``dict remove DICT ?KEY ...?`` — return a copy of DICT with each
/// listed KEY removed.  Mirrors ``tclDictObj.c::DictRemoveCmd``.
fn eval_dict_remove(words: []const i32) i32 {
    var result: i32 = words[2];
    var wi: u32 = 3;
    while (wi < words.len) : (wi += 1) {
        result = rt.dict_unset(result, words[wi]);
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
    var result: i32 = words[2];
    var wi: u32 = 3;
    while (wi + 1 < words.len) : (wi += 2) {
        result = rt.dict_set(result, words[wi], words[wi + 1]);
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
    const kvar = obj.obj_new_string_copy(varlist.ptr + kelem.start, kelem.len);
    const vvar = obj.obj_new_string_copy(varlist.ptr + velem.start, velem.len);

    const dict = words[3];
    const sd = obj_ensure_string(dict);
    const n = rt.list_count_elements(sd.ptr, sd.len);
    if (n <= 0 or (n & 1) != 0) {
        return if (collect) rt.dict_create() else 0;
    }

    const body = obj_ensure_string(words[4]);
    var collected: i32 = if (collect) rt.dict_create() else 0;
    var idx: i64 = 0;
    while (idx + 1 < n) : (idx += 2) {
        const k = rt.list_element_at(sd.ptr, sd.len, idx);
        const v = rt.list_element_at(sd.ptr, sd.len, idx + 1);
        const key_obj = obj.obj_new_string_copy(sd.ptr + k.start, k.len);
        const val_obj = obj.obj_new_string_copy(sd.ptr + v.start, v.len);
        _ = frames.var_set(kvar, key_obj);
        _ = frames.var_set(vvar, val_obj);
        const body_result = interp.eval_script(body.ptr, body.len);
        if (rt.error_flag.* != 0) return 0;
        if (rt.return_flag.* != 0) return 0;
        if (rt.break_flag.* != 0) {
            rt.break_flag.* = 0;
            break;
        }
        if (rt.continue_flag.* != 0) {
            rt.continue_flag.* = 0;
            continue;
        }
        if (collect) {
            collected = rt.dict_set(collected, key_obj, body_result);
        }
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

    // Resolve the sub-dict at the optional key path.
    var cur = frames.var_resolve(dict_var);
    if (cur == 0) return 0;
    var ki: u32 = 3;
    while (ki + 1 < words.len) : (ki += 1) {
        if (rt.obj_get_int(rt.dict_exists(cur, words[ki])) == 0) return 0;
        cur = rt.dict_get(cur, words[ki]);
    }

    // Snapshot keys and bind each as a same-named local.
    const sd = obj_ensure_string(cur);
    const n = rt.list_count_elements(sd.ptr, sd.len);
    var idx: i64 = 0;
    while (idx + 1 < n) : (idx += 2) {
        const k = rt.list_element_at(sd.ptr, sd.len, idx);
        const v = rt.list_element_at(sd.ptr, sd.len, idx + 1);
        const key_obj = obj.obj_new_string_copy(sd.ptr + k.start, k.len);
        const val_obj = obj.obj_new_string_copy(sd.ptr + v.start, v.len);
        _ = frames.var_set(key_obj, val_obj);
    }

    _ = interp.eval_script(body.ptr, body.len);
    if (rt.error_flag.* != 0) return 0;

    // Write back each key from the (potentially modified) locals.
    var sub = cur;
    idx = 0;
    while (idx + 1 < n) : (idx += 2) {
        const k = rt.list_element_at(sd.ptr, sd.len, idx);
        const key_obj = obj.obj_new_string_copy(sd.ptr + k.start, k.len);
        const new_val = frames.var_resolve(key_obj);
        if (new_val != 0) {
            sub = rt.dict_set(sub, key_obj, new_val);
        }
    }

    // Re-write the (possibly relocated) sub-dict back through the key path.
    if (words.len == 4) {
        // No key path — sub is the top-level dict.
        _ = frames.var_set(dict_var, sub);
    } else {
        var top = frames.var_resolve(dict_var);
        if (top == 0) top = rt.dict_create();
        // Walk back up: rebuild each ancestor with the new sub-dict
        // at the corresponding key.  Build nested writes innermost-first.
        var depth: u32 = words.len - 1;  // index of the deepest key + 1
        // depth is currently the body index; the deepest key is depth - 1.
        depth -= 1;
        // Rewrite from deepest to shallowest.
        var inner = sub;
        while (depth > 3) : (depth -= 1) {
            const key = words[depth];
            // Get the parent dict by walking from top to depth - 1.
            var parent = top;
            var pi: u32 = 3;
            while (pi < depth) : (pi += 1) {
                parent = rt.dict_get(parent, words[pi]);
            }
            inner = rt.dict_set(parent, key, inner);
        }
        const final = rt.dict_set(top, words[3], inner);
        _ = frames.var_set(dict_var, final);
    }
    return 0;
}

/// ``dict filter DICT FILTERTYPE ARG ?ARG ...?`` — filter the dict
/// by FILTERTYPE.  Supports ``key PATTERN ?PATTERN ...?`` and
/// ``value PATTERN ?PATTERN ...?`` (glob match against any
/// supplied pattern).  The ``script`` form is not yet implemented;
/// raise an explicit error rather than silently returning the input
/// so a script that depends on the script-form filter doesn't
/// silently produce wrong results (Copilot/Codex review).  Mirrors
/// ``tclDictObj.c::DictFilterCmd`` for the pattern forms.
fn eval_dict_filter(words: []const i32) i32 {
    const ft = obj_ensure_string(words[3]);
    const fp: [*]const u8 = @ptrFromInt(ft.ptr);
    const filter_keys = str_eq(fp, ft.len, "key");
    const filter_values = str_eq(fp, ft.len, "value");
    const filter_script = str_eq(fp, ft.len, "script");
    if (filter_script) {
        const stubs = @import("../stubs/tcl_stubs.zig");
        stubs.unsupported_sub("dict filter", "script");
        return 0;
    }
    if (!filter_keys and !filter_values) {
        const stubs = @import("../stubs/tcl_stubs.zig");
        stubs.raise("bad filterType: must be key, script, or value");
        return 0;
    }
    if (words.len < 5) return rt.dict_create();

    const sd = obj_ensure_string(words[2]);
    const n = rt.list_count_elements(sd.ptr, sd.len);
    var result: i32 = rt.dict_create();
    var idx: i64 = 0;
    while (idx + 1 < n) : (idx += 2) {
        const k = rt.list_element_at(sd.ptr, sd.len, idx);
        const v = rt.list_element_at(sd.ptr, sd.len, idx + 1);
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
            result = rt.dict_set(result, key_obj, val_obj);
        }
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
