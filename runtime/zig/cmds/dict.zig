// Tcl ``dict`` built-in command.
//
// Extracted from tcl_interp_string.zig.  Registers itself in the
// central command table via the ``registration`` constant.

const rt = @import("../tcl_runtime.zig");
const frames = @import("../interp/tcl_frames.zig");

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
    if (str_eq(sp, sub.len, "set") and words.len >= 5) {
        const cur = frames.var_resolve(words[2]);
        const result = rt.dict_set(cur, words[3], words[4]);
        _ = frames.var_set(words[2], result);
        return result;
    }
    if (str_eq(sp, sub.len, "exists") and words.len >= 4) return rt.dict_exists(words[2], words[3]);
    if (str_eq(sp, sub.len, "keys")) return rt.dict_keys(words[2]);
    if (str_eq(sp, sub.len, "values")) return rt.dict_values(words[2]);
    if (str_eq(sp, sub.len, "size")) return rt.dict_size(words[2]);
    if (str_eq(sp, sub.len, "create")) return rt.dict_create();
    return 0;
}
