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

pub fn eval(words: []const i32) result_mod.InterpResult {
    // ``array SUB ?args?`` — minimum is ``array SUB ARRAYNAME``.
    // Bare ``array`` (no subcommand) raises ``wrong # args``;
    // ``array SUB`` with no arrayName raises a subcommand-specific
    // diagnostic (set-old-8.1 / 8.2).
    if (words.len < 2) {
        raise_array_error("wrong # args: should be \"array subcommand ?arg ...?\"");
        return result_mod.from_globals(0);
    }
    if (words.len < 3) {
        // Pick the wording that matches the bare ``array SUB`` form
        // for the well-known subcommands; fall through to a generic
        // diagnostic otherwise (matches the upstream switch in
        // ``tclVar.c`` ``Tcl_ArrayObjCmd``).
        const sub2 = obj_ensure_string(words[1]);
        const usage = pick_array_usage(sub2.ptr, sub2.len);
        raise_array_error(usage);
        return result_mod.from_globals(0);
    }
    const sub = obj_ensure_string(words[1]);
    const sp: [*]const u8 = @ptrFromInt(sub.ptr);
    const array_mod = @import("../valtypes/tcl_array.zig");
    const frames_mod = @import("../interp/tcl_frames.zig");
    // Phase 1: every array subcommand routes its array-name argument
    // through ``frame_resolve_array_name`` so the local / global /
    // namespace / aliased cases all reach the right table.  The
    // resolved obj is owned here; release after use unless it
    // aliases the input.
    const resolved_name: i32 = frames_mod.frame_resolve_array_name(words[2]);
    const obj_mod = @import("../valtypes/tcl_obj.zig");
    defer if (resolved_name != words[2]) obj_mod.tcl_obj_release(resolved_name);
    if (str_eq(sp, sub.len, "get")) {
        // ``array get arrName ?pattern?`` — returns the whole array as a
        // flat ``{k v k v …}`` list.  ``words[3]`` is an optional glob
        // *pattern* (filter) — *not* an element key.  Earlier wiring
        // misused ``array_get`` (single-element lookup) here, which
        // matched only the empty-string element and produced an empty
        // result for every well-formed array.
        const pat: i32 = if (words.len >= 4) words[3] else 0;
        return result_mod.from_globals(array_mod.array_get_all(resolved_name, pat));
    }
    if (str_eq(sp, sub.len, "set") and words.len >= 4) {
        // ``array set arr pairlist`` — payload is a flat
        // ``{k v k v …}`` list, always routed through
        // ``array_set_list`` even for the single-pair shape so
        // tcltest's ``ArrayDefault`` initialiser populates each
        // element individually.
        return result_mod.from_globals(array_mod.array_set_list(resolved_name, words[3]));
    }
    if (str_eq(sp, sub.len, "exists")) return result_mod.from_globals(array_mod.array_exists(resolved_name));
    if (str_eq(sp, sub.len, "names")) {
        const pat: i32 = if (words.len >= 4) words[3] else 0;
        return result_mod.from_globals(array_mod.array_names(resolved_name, pat));
    }
    if (str_eq(sp, sub.len, "size")) return result_mod.from_globals(array_mod.array_size(resolved_name));
    if (str_eq(sp, sub.len, "unset")) {
        if (words.len >= 4) return result_mod.from_globals(array_mod.array_unset_element(resolved_name, words[3]));
        return result_mod.from_globals(array_mod.array_unset(resolved_name));
    }
    // Other subcommands (statistics, startsearch, …) not yet wired —
    // fall through to the stub dispatch which raises the exception.
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

/// Pick the ``wrong # args`` usage string for ``array SUB`` calls
/// with no array-name argument.  Mirrors the per-subcommand
/// diagnostics in Tcl 9 ``tclVar.c`` ``Tcl_ArrayObjCmd``.
fn pick_array_usage(sub_ptr: u32, sub_len: u32) []const u8 {
    const sp: [*]const u8 = @ptrFromInt(sub_ptr);
    inline for (.{
        .{ "anymore", "wrong # args: should be \"array anymore arrayName searchId\"" },
        .{ "donesearch", "wrong # args: should be \"array donesearch arrayName searchId\"" },
        .{ "exists", "wrong # args: should be \"array exists arrayName\"" },
        .{ "get", "wrong # args: should be \"array get arrayName ?pattern?\"" },
        .{ "names", "wrong # args: should be \"array names arrayName ?mode? ?pattern?\"" },
        .{ "nextelement", "wrong # args: should be \"array nextelement arrayName searchId\"" },
        .{ "set", "wrong # args: should be \"array set arrayName list\"" },
        .{ "size", "wrong # args: should be \"array size arrayName\"" },
        .{ "startsearch", "wrong # args: should be \"array startsearch arrayName\"" },
        .{ "statistics", "wrong # args: should be \"array statistics arrayName\"" },
        .{ "unset", "wrong # args: should be \"array unset arrayName ?pattern?\"" },
        .{ "default", "wrong # args: should be \"array default subcommand arrayName ?args?\"" },
    }) |entry| {
        const name = entry[0];
        const usage = entry[1];
        if (sub_len == name.len) {
            var matches = true;
            inline for (name, 0..) |c, i| {
                if (sp[i] != c) {
                    matches = false;
                    break;
                }
            }
            if (matches) return usage;
        }
    }
    return "wrong # args: should be \"array subcommand ?arg ...?\"";
}
