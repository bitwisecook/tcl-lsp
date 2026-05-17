// ``set``, ``incr``, ``unset`` — variable read/write commands.

const rt = @import("../tcl_runtime.zig");
const result_mod = @import("../interp/tcl_result.zig");
const frames = @import("../interp/tcl_frames.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");
const tcl_array = @import("../valtypes/tcl_array.zig");
const tcl_ns = @import("../interp/tcl_ns.zig");

const obj_ensure_string = rt.obj_ensure_string;
const obj_new_string = rt.obj_new_string;

fn eval_set(words: []const i32) result_mod.InterpResult {
    // Reference Tcl: ``set varName ?newValue?`` takes 1 or 2 args.
    // Out-of-range arities raise ``wrong # args`` (parse-9.2).
    if (words.len < 2 or words.len > 3) {
        const catch_mod = @import("../interp/tcl_catch.zig");
        const msg_text: []const u8 = "wrong # args: should be \"set varName ?newValue?\"";
        const buf = rt.alloc(@intCast(msg_text.len));
        if (buf == 0) {
            catch_mod.tcl_cmd_error(0);
            return result_mod.from_globals(0);
        }
        const dst: [*]u8 = @ptrFromInt(buf);
        for (msg_text, 0..) |b, k| dst[k] = b;
        const msg = rt.obj_new_string_take(buf, @intCast(msg_text.len), @intCast(msg_text.len));
        catch_mod.tcl_cmd_error(msg);
        return result_mod.from_globals(0);
    }
    if (words.len == 3) {
        _ = frames.var_set(words[1], words[2]);
        return result_mod.from_globals(words[2]);
    }
    // Read form: ``set varName``.  When the variable doesn't exist,
    // raise ``can't read "<name>": no such variable`` per Tcl 9
    // semantics (set-1.13).  ``var_resolve`` returns 0 silently for
    // missing slots — we re-raise via ``var_unset_error``.
    const v = frames.var_resolve(words[1]);
    if (v == 0) {
        const catch_mod = @import("../interp/tcl_catch.zig");
        catch_mod.var_unset_error(words[1]);
        return result_mod.from_globals(0);
    }
    return result_mod.from_globals(v);
}

fn eval_incr(words: []const i32) result_mod.InterpResult {
    if (words.len < 2 or words.len > 3) {
        const stubs = @import("../stubs/tcl_stubs.zig");
        stubs.raise("wrong # args: should be \"incr varName ?increment?\"");
        return result_mod.from_globals(0);
    }
    const amt_obj = if (words.len >= 3) words[2] else rt.obj_new_int(1);
    const cur = frames.var_resolve(words[1]);
    const result = rt.tcl_incr(cur, amt_obj);
    _ = frames.var_set(words[1], result);
    return result_mod.from_globals(result);
}

fn eval_unset(words: []const i32) result_mod.InterpResult {
    var i: u32 = 1;
    var nocomplain = false;
    // Consume option arguments first (``-nocomplain``, ``--``).
    while (i < words.len) : (i += 1) {
        const w = obj_ensure_string(words[i]);
        if (w.len == 0) continue;
        const wp: [*]const u8 = @ptrFromInt(w.ptr);
        if (wp[0] != '-') break;
        if (w.len == 2 and wp[1] == '-') {
            i += 1;
            break;
        }
        if (w.len == 11) {
            const lit = "-nocomplain";
            var matches = true;
            for (lit, 0..) |c, k2| {
                if (wp[k2] != c) {
                    matches = false;
                    break;
                }
            }
            if (matches) {
                nocomplain = true;
                continue;
            }
        }
        // Unknown option — Tcl 9 ``unset`` is permissive and treats
        // anything else (or any past-option name) as a variable name.
        break;
    }
    while (i < words.len) : (i += 1) {
        const w = obj_ensure_string(words[i]);
        if (w.len == 0) continue;
        // Tcl 9: ``unset X`` raises ``can't unset "X": no such
        // variable`` when X doesn't exist (unless ``-nocomplain``
        // was passed).  For ``arr(key)`` forms, additional
        // discrimination — ``variable isn't array`` when ``arr``
        // is a scalar, ``no such element in array`` when ``arr``
        // is an array but the element is missing.
        if (!nocomplain and !var_exists_for_unset(words[i])) {
            raise_unset_error(words[i]);
            return result_mod.from_globals(0);
        }
        const wp: [*]const u8 = @ptrFromInt(w.ptr);
        // Clear the array table before nulling the variable so
        // ``info exists arr`` returns 0 after ``unset arr``.
        const resolved_arr = frames.frame_resolve_array_name(words[i]);
        const tcl_obj_mod = @import("../valtypes/tcl_obj.zig");
        defer if (resolved_arr != words[i]) tcl_obj_mod.tcl_obj_release(resolved_arr);
        _ = tcl_array.array_unset(resolved_arr);
        var is_global: bool = (w.len >= 2 and wp[0] == ':' and wp[1] == ':');
        if (!is_global) {
            for (0..w.len - 1) |k| {
                if (wp[k] == ':' and wp[k + 1] == ':') {
                    is_global = true;
                    break;
                }
            }
        }
        if (is_global) {
            _ = tcl_ns.global_set(words[i], 0);
        } else {
            _ = frames.var_set(words[i], 0);
        }
    }
    return result_mod.from_globals(obj_new_string(0, 0));
}

/// Does *name* refer to a live variable (scalar or array element)?
/// Used by ``unset`` to decide whether to raise the canonical
/// "can't unset X: no such variable" diagnostic.
fn var_exists_for_unset(name: i32) bool {
    const obj_mod = @import("../valtypes/tcl_obj.zig");
    const r = frames.var_exists(name);
    return obj_mod.obj_get_int(r) != 0;
}

fn raise_unset_error(name: i32) void {
    const obj_mod = @import("../valtypes/tcl_obj.zig");
    const catch_mod = @import("../interp/tcl_catch.zig");
    const sn = obj_mod.obj_ensure_string(name);
    const prefix: []const u8 = "can't unset \"";
    // Pick the suffix the same way var_unset_error (read path)
    // does — array-element vs whole-array vs missing-scalar.
    const suffix = pick_unset_suffix(sn);
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
    const sp: [*]const u8 = @ptrFromInt(sn.ptr);
    for (0..sn.len) |k| {
        dst[off] = sp[k];
        off += 1;
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const msg = obj_mod.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(msg);
}

fn pick_unset_suffix(s: anytype) []const u8 {
    if (s.len == 0 or s.ptr == 0) return "\": no such variable";
    const sp: [*]const u8 = @ptrFromInt(s.ptr);
    var paren: u32 = 0;
    var has_paren = false;
    var k: u32 = 0;
    while (k < s.len) : (k += 1) {
        if (sp[k] == '(') {
            paren = k;
            has_paren = true;
            break;
        }
    }
    if (has_paren and paren > 0 and sp[s.len - 1] == ')') {
        const arr_len = paren;
        var probe_ptr: u32 = s.ptr;
        var probe_len: u32 = arr_len;
        if (arr_len >= 2 and sp[0] == ':' and sp[1] == ':') {
            probe_ptr += 2;
            probe_len -= 2;
        }
        if (tcl_array.array_exists_raw(probe_ptr, probe_len)) {
            return "\": no such element in array";
        }
        const obj_helpers = @import("../valtypes/tcl_obj.zig");
        const arr_obj = obj_helpers.obj_new_string(@bitCast(s.ptr), @bitCast(arr_len));
        const exists = obj_helpers.obj_get_int(tcl_ns.global_exists(arr_obj)) != 0;
        obj_helpers.tcl_obj_release(arr_obj);
        if (exists) {
            return "\": variable isn't array";
        }
        return "\": no such variable";
    }
    return "\": no such variable";
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "set", .arity_min = 1, .arity_max = 2, .handler = &eval_set },
    .{ .name = "incr", .arity_min = 1, .arity_max = 2, .handler = &eval_incr },
    .{ .name = "unset", .arity_min = 1, .arity_max = null, .handler = &eval_unset },
};
