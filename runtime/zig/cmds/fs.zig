// ``file``, ``pwd``, ``cd`` — filesystem commands.

const fs_mod = @import("../tcl_fs.zig");
const reg    = @import("../tcl_cmd_registry.zig");

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
    .{ .name = "file", .handler = &eval_file },
    .{ .name = "pwd",  .handler = &eval_pwd },
    .{ .name = "cd",   .handler = &eval_cd },
};
