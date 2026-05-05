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
    .arity_min = 1, .arity_max = null, .handler = &eval,
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
    if (words.len < 3) return result_mod.from_globals(0);
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
        if (words.len >= 4) return result_mod.from_globals(array_mod.array_get(resolved_name, words[3]));
        return result_mod.from_globals(array_mod.array_get(resolved_name, obj_new_string(0, 0)));
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
