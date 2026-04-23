// ``return``, ``break``, ``continue``, ``error``, ``catch`` — control-flow signals.

const rt     = @import("../tcl_runtime.zig");
const frames = @import("../tcl_frames.zig");
const reg    = @import("../tcl_cmd_registry.zig");

const str_eq            = @import("../tcl_chars.zig").str_eq;
const obj_ensure_string = rt.obj_ensure_string;
const obj_new_int       = rt.obj_new_int;

fn eval_return(words: []const i32) i32 {
    var is_error = false;
    var result_obj: i32 = 0;
    var wi: u32 = 1;
    while (wi < words.len) : (wi += 1) {
        const w = obj_ensure_string(words[wi]);
        if (w.len >= 1) {
            const wp: [*]const u8 = @ptrFromInt(w.ptr);
            if (wp[0] == '-') {
                if (str_eq(wp, w.len, "-code") and wi + 1 < words.len) {
                    const code = obj_ensure_string(words[wi + 1]);
                    if (code.len >= 1) {
                        const cp: [*]const u8 = @ptrFromInt(code.ptr);
                        if (str_eq(cp, code.len, "error")) is_error = true;
                    }
                    wi += 1;
                    continue;
                }
                if ((str_eq(wp, w.len, "-level") or
                    str_eq(wp, w.len, "-errorinfo") or
                    str_eq(wp, w.len, "-errorcode") or
                    str_eq(wp, w.len, "-options")) and wi + 1 < words.len)
                {
                    wi += 1;
                    continue;
                }
            }
        }
        result_obj = words[wi];
    }
    if (is_error) {
        const catch_mod = @import("../tcl_catch.zig");
        catch_mod.tcl_cmd_error(result_obj);
        return 0;
    }
    rt.return_flag.* = 1;
    rt.return_val.* = result_obj;
    return result_obj;
}

fn eval_break(words: []const i32) i32 {
    _ = words;
    rt.break_flag.* = 1;
    return 0;
}

fn eval_continue(words: []const i32) i32 {
    _ = words;
    rt.continue_flag.* = 1;
    return 0;
}

fn eval_error(words: []const i32) i32 {
    if (words.len >= 2) rt.tcl_cmd_error(words[1]);
    return 0;
}

fn eval_catch(words: []const i32) i32 {
    if (words.len >= 2) {
        const interp = @import("../tcl_interp.zig");
        rt.catch_enter();
        const body_s = obj_ensure_string(words[1]);
        const body_result = interp.eval_script(body_s.ptr, body_s.len);
        rt.catch_set_ok_result(body_result);
        const catch_val = rt.catch_result();
        const code = rt.catch_leave();
        if (words.len >= 3) _ = frames.var_set(words[2], catch_val);
        return code;
    }
    return obj_new_int(0);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "return",   .handler = &eval_return },
    .{ .name = "break",    .handler = &eval_break },
    .{ .name = "continue", .handler = &eval_continue },
    .{ .name = "error",    .handler = &eval_error },
    .{ .name = "catch",    .handler = &eval_catch },
};
