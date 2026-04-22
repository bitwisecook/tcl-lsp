// ``regexp`` — regular expression matching command.

const regex_mod = @import("../tcl_regex.zig");
const reg       = @import("../tcl_cmd_registry.zig");

fn eval_regexp(words: []const i32) i32 {
    return regex_mod.eval_regexp_cmd(words);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "regexp", .handler = &eval_regexp },
};
