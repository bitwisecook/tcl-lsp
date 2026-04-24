// ``file``, ``pwd``, ``cd`` — filesystem commands.

const fs_mod = @import("../io/tcl_fs.zig");
const reg    = @import("../dispatch/tcl_cmd_registry.zig");

fn eval_file(words: []const i32) i32 {
    const sub = if (words.len >= 2) words[1] else 0;
    const a1  = if (words.len >= 3) words[2] else 0;
    const a2  = if (words.len >= 4) words[3] else 0;
    return fs_mod.tcl_cmd_file(sub, a1, a2);
}

fn eval_pwd(words: []const i32) i32 {
    _ = words;
    return fs_mod.tcl_cmd_pwd();
}

fn eval_cd(words: []const i32) i32 {
    return fs_mod.tcl_cmd_cd(if (words.len >= 2) words[1] else 0);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "file", .arity_min = 1, .arity_max = null, .handler = &eval_file },
    .{ .name = "pwd", .arity_min = 0, .arity_max = 0, .handler = &eval_pwd },
    .{ .name = "cd", .arity_min = 0, .arity_max = 1, .handler = &eval_cd },
};

// ``file <sub>`` sub-commands — mirrors
// ``core/commands/registry/tcl/file.py``.  Cross-checked against
// ``generic/tclFCmd.c`` + ``generic/tclFileName.c`` in C Tcl 9.0 —
// each sub-command's handler enforces its arity via
// ``Tcl_WrongNumArgs``.  ``pwd`` and ``cd`` have no sub-commands.
pub const file_subcommands: []const reg.SubEntry = &.{
    .{ .name = "atime", .arity_min = 1, .arity_max = 2, .handler = &eval_file },
    .{ .name = "attributes", .arity_min = 1, .arity_max = null, .handler = &eval_file },
    .{ .name = "channels", .arity_min = 0, .arity_max = 1, .handler = &eval_file },
    .{ .name = "copy", .arity_min = 2, .arity_max = null, .handler = &eval_file },
    .{ .name = "delete", .arity_min = 1, .arity_max = null, .handler = &eval_file },
    .{ .name = "dirname", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "executable", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "exists", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "extension", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "home", .arity_min = 0, .arity_max = 1, .handler = &eval_file },
    .{ .name = "isdirectory", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "isfile", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "join", .arity_min = 1, .arity_max = null, .handler = &eval_file },
    .{ .name = "link", .arity_min = 1, .arity_max = 2, .handler = &eval_file },
    .{ .name = "lstat", .arity_min = 1, .arity_max = 2, .handler = &eval_file },
    .{ .name = "mkdir", .arity_min = 1, .arity_max = null, .handler = &eval_file },
    .{ .name = "mtime", .arity_min = 1, .arity_max = 2, .handler = &eval_file },
    .{ .name = "nativename", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "normalize", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "owned", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "pathtype", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "readable", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "readlink", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "rename", .arity_min = 2, .arity_max = null, .handler = &eval_file },
    .{ .name = "rootname", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "separator", .arity_min = 0, .arity_max = 1, .handler = &eval_file },
    .{ .name = "size", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "split", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "stat", .arity_min = 1, .arity_max = 2, .handler = &eval_file },
    .{ .name = "system", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "tail", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "tempdir", .arity_min = 0, .arity_max = 1, .handler = &eval_file },
    .{ .name = "tempfile", .arity_min = 0, .arity_max = 2, .handler = &eval_file },
    .{ .name = "tildeexpand", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "type", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "volumes", .arity_min = 0, .arity_max = 0, .handler = &eval_file },
    .{ .name = "writable", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
};
