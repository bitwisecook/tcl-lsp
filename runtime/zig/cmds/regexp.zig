// ``regexp``, ``regsub`` — regular expression commands.

const rt        = @import("../tcl_runtime.zig");
const regex_mod = @import("../valtypes/tcl_regex.zig");
const frames    = @import("../interp/tcl_frames.zig");
const reg       = @import("../dispatch/tcl_cmd_registry.zig");
const stubs     = @import("../stubs/tcl_stubs.zig");

const obj_ensure_string = rt.obj_ensure_string;
const obj_new_int       = rt.obj_new_int;
const obj_new_string    = rt.obj_new_string;

fn eval_regexp(words: []const i32) i32 {
    return regex_mod.eval_regexp_cmd(words);
}

// ``regsub ?-nocase? ?-all? ?--? pattern string subSpec ?varName?``
fn eval_regsub(words: []const i32) i32 {
    if (words.len < 4) {
        stubs.raise("wrong # args: regsub ?-nocase? ?-all? ?--? pattern string subSpec ?varName?");
        return obj_new_int(0);
    }
    var nocase = false;
    var all    = false;
    var i: usize = 1;
    // Parse switches
    while (i < words.len) : (i += 1) {
        const w = obj_ensure_string(words[i]);
        if (w.len == 0) break;
        const p: [*]const u8 = @ptrFromInt(w.ptr);
        if (p[0] != '-') break;
        if (w.len == 2 and p[1] == '-') { i += 1; break; }
        if (w.len == 7 and p[1]=='n' and p[2]=='o' and p[3]=='c' and
            p[4]=='a' and p[5]=='s' and p[6]=='e')
        {
            nocase = true; continue;
        }
        if (w.len == 4 and p[1]=='a' and p[2]=='l' and p[3]=='l') {
            all = true; continue;
        }
        stubs.raise("regsub: unsupported or unknown option");
        return obj_new_int(0);
    }
    if (i + 2 >= words.len) {
        stubs.raise("wrong # args: regsub ?-nocase? ?-all? ?--? pattern string subSpec ?varName?");
        return obj_new_int(0);
    }
    const pattern = words[i];     i += 1;
    const string  = words[i];     i += 1;
    const subspec = words[i];     i += 1;
    const has_var = (i < words.len);
    const var_name = if (has_var) words[i] else 0;
    i += if (has_var) 1 else 0;
    if (i < words.len) {
        stubs.raise("wrong # args: regsub ?-nocase? ?-all? ?--? pattern string subSpec ?varName?");
        return obj_new_int(0);
    }

    var n_subs: i32 = 0;
    const result = regex_mod.do_regsub(pattern, string, subspec, nocase, all, &n_subs);
    if (has_var) {
        _ = frames.var_set(var_name, result);
        return obj_new_int(n_subs);
    }
    return result;
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "regexp", .handler = &eval_regexp },
    .{ .name = "regsub", .handler = &eval_regsub },
};
