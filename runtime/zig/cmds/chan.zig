// ``encoding``, ``fconfigure`` — channel/encoding commands.

const rt  = @import("../tcl_runtime.zig");
const enc = @import("../valtypes/tcl_encoding.zig");
const chan = @import("../io/tcl_chan.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");
const obj = @import("../valtypes/tcl_obj.zig");
const list_quote = @import("../valtypes/tcl_list_quote.zig");

const alloc          = rt.alloc;
const obj_new_string = rt.obj_new_string;

fn eval_encoding(words: []const i32) i32 {
    const sub  = if (words.len >= 2) words[1] else 0;
    const arg1 = if (words.len >= 3) words[2] else 0;
    const arg2 = if (words.len >= 4) words[3] else 0;
    return enc.tcl_cmd_encoding(sub, arg1, arg2);
}

fn eval_fconfigure(words: []const i32) i32 {
    if (words.len < 2) return chan.tcl_cmd_fconfigure(0, 0);
    const fd = words[1];
    if (words.len < 3) return chan.tcl_cmd_fconfigure(fd, 0);

    // Build a properly-quoted Tcl list of the option words.  Plain
    // ``tcl_cmd_concat`` with " " separators would collapse empty
    // strings — ``fconfigure $fd -eofchar {}`` would arrive at the
    // runtime as a bare ``-eofchar`` and silently flip from setter
    // to query.  ``list_elem_quote_nth`` emits canonical list
    // elements (empty → ``{}``, whitespace / brace values get
    // brace-wrapped or backslash-escaped) so the consumer's
    // ``list_parse`` walk recovers the original bytes.
    var total_cap: u32 = 0;
    var i: u32 = 2;
    while (i < words.len) : (i += 1) {
        const s = obj.obj_ensure_string(words[i]);
        // Worst-case expansion per element: 2*len + 2 bytes (every
        // byte gets a backslash + outer braces) plus a separator.
        total_cap += s.len * 2 + 3;
    }
    if (total_cap == 0) total_cap = 1;
    const buf = alloc(total_cap);
    var off: u32 = 0;
    i = 2;
    while (i < words.len) : (i += 1) {
        if (i > 2) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        const s = obj.obj_ensure_string(words[i]);
        off = list_quote.list_elem_quote_nth(buf, off, s.ptr, s.len);
    }
    const args_obj = obj_new_string(@intCast(buf), @intCast(off));
    return chan.tcl_cmd_fconfigure(fd, args_obj);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "encoding", .arity_min = 1, .arity_max = null, .handler = &eval_encoding },
    .{ .name = "fconfigure", .arity_min = 1, .arity_max = null, .handler = &eval_fconfigure },
};

// Per-command sub-table — only ``encoding`` has sub-commands here;
// ``fconfigure`` is variadic ``-option value`` pairs, not an ensemble.
pub const encoding_subcommands: []const reg.SubEntry = &.{
    .{ .name = "convertfrom", .arity_min = 1, .arity_max = null, .handler = &eval_encoding },
    .{ .name = "convertto", .arity_min = 1, .arity_max = null, .handler = &eval_encoding },
    .{ .name = "dirs", .arity_min = 0, .arity_max = 1, .handler = &eval_encoding },
    .{ .name = "names", .arity_min = 0, .arity_max = 0, .handler = &eval_encoding },
    .{ .name = "profiles", .arity_min = 0, .arity_max = 0, .handler = &eval_encoding },
    .{ .name = "system", .arity_min = 0, .arity_max = 1, .handler = &eval_encoding },
    .{ .name = "user", .arity_min = 0, .arity_max = 0, .handler = &eval_encoding },
};
