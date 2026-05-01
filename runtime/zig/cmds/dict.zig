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
    return 0;
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
