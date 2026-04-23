// List manipulation commands: list, lappend, llength, lindex, lset,
// linsert, lreplace, lsort, lsearch, lrange, concat, join, split.

const rt      = @import("../tcl_runtime.zig");
const frames  = @import("../tcl_frames.zig");
const obj_mod = @import("../tcl_obj.zig");
const reg     = @import("../tcl_cmd_registry.zig");

const alloc             = rt.alloc;
const obj_new_string    = rt.obj_new_string;
const obj_new_int       = rt.obj_new_int;
const obj_ensure_string = rt.obj_ensure_string;

fn eval_list(words: []const i32) i32 {
    if (words.len <= 1) return obj_new_string(0, 0);
    var max_total: u32 = 0;
    var ei: u32 = 1;
    while (ei < words.len) : (ei += 1) {
        const s = obj_ensure_string(words[ei]);
        max_total += s.len * 2 + 2;
        if (ei > 1) max_total += 1;
    }
    const buf = obj_mod.alloc(max_total + 4);
    var off: u32 = 0;
    ei = 1;
    while (ei < words.len) : (ei += 1) {
        if (ei > 1) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        const s = obj_ensure_string(words[ei]);
        if (ei == 1) {
            off = obj_mod.list_elem_quote(buf, off, s.ptr, s.len);
        } else {
            off = obj_mod.list_elem_quote_nth(buf, off, s.ptr, s.len);
        }
    }
    return obj_new_string(@bitCast(buf), @bitCast(off));
}

fn eval_lappend(words: []const i32) i32 {
    if (words.len >= 2) {
        var result = frames.var_resolve(words[1]);
        var wi: u32 = 2;
        while (wi < words.len) : (wi += 1) {
            result = rt.tcl_cmd_lappend(result, words[wi]);
        }
        _ = frames.var_set(words[1], result);
        return result;
    }
    return 0;
}

fn eval_llength(words: []const i32) i32 {
    if (words.len >= 2) return rt.tcl_cmd_list_length(words[1]);
    return 0;
}

fn eval_lindex(words: []const i32) i32 {
    if (words.len >= 3) return rt.tcl_cmd_list_index(words[1], words[2]);
    return 0;
}

fn eval_lset(words: []const i32) i32 {
    if (words.len >= 3) {
        const current = frames.var_resolve(words[1]);
        const newval = words[words.len - 1];
        const indices: i32 = if (words.len == 3)
            obj_new_string(0, 0)
        else if (words.len == 4)
            words[2]
        else blk: {
            var acc: i32 = rt.tcl_list(words[2], words[3]);
            var wi: u32 = 4;
            while (wi + 1 < words.len) : (wi += 1) {
                acc = rt.tcl_list(acc, words[wi]);
            }
            break :blk acc;
        };
        const result = rt.tcl_cmd_list_set(current, indices, newval);
        _ = frames.var_set(words[1], result);
        return result;
    }
    return 0;
}

fn eval_linsert(words: []const i32) i32 {
    if (words.len >= 4) {
        const list_arg = words[1];
        const idx_arg = words[2];
        const idx_s = obj_ensure_string(idx_arg);
        var forward = false;
        if (idx_s.len >= 3) {
            const p: [*]const u8 = @ptrFromInt(idx_s.ptr);
            if (p[0] == 'e' and p[1] == 'n' and p[2] == 'd') forward = true;
        }
        var result: i32 = list_arg;
        if (forward) {
            var wi: u32 = 3;
            while (wi < words.len) : (wi += 1) {
                result = rt.tcl_cmd_list_insert(result, idx_arg, words[wi]);
            }
        } else {
            var wi: u32 = words.len;
            while (wi > 3) {
                wi -= 1;
                result = rt.tcl_cmd_list_insert(result, idx_arg, words[wi]);
            }
        }
        return result;
    }
    return 0;
}

fn eval_lreplace(words: []const i32) i32 {
    if (words.len >= 4) {
        const list_arg  = words[1];
        const first_arg = words[2];
        const last_arg  = words[3];
        if (words.len == 4) {
            return rt.tcl_cmd_list_replace(list_arg, first_arg, last_arg, 0);
        }
        const idx_s = obj_ensure_string(first_arg);
        var forward = false;
        if (idx_s.len >= 3) {
            const p: [*]const u8 = @ptrFromInt(idx_s.ptr);
            if (p[0] == 'e' and p[1] == 'n' and p[2] == 'd') forward = true;
        }
        if (forward) {
            var result = rt.tcl_cmd_list_replace(list_arg, first_arg, last_arg, words[4]);
            var wi: u32 = 5;
            while (wi < words.len) : (wi += 1) {
                result = rt.tcl_cmd_list_insert(result, first_arg, words[wi]);
            }
            return result;
        } else {
            var result = rt.tcl_cmd_list_replace(list_arg, first_arg, last_arg, words[words.len - 1]);
            var wi: u32 = words.len - 1;
            while (wi > 4) {
                wi -= 1;
                result = rt.tcl_cmd_list_insert(result, first_arg, words[wi]);
            }
            return result;
        }
    }
    return 0;
}

fn eval_lsort(words: []const i32) i32 {
    if (words.len >= 2) return rt.tcl_cmd_list_sort(words[words.len - 1]);
    return obj_new_string(0, 0);
}

fn eval_lsearch(words: []const i32) i32 {
    if (words.len >= 3) return rt.tcl_cmd_list_search(words[1], words[2]);
    return obj_new_int(-1);
}

fn eval_lrange(words: []const i32) i32 {
    if (words.len >= 4) return rt.tcl_cmd_list_range(words[1], words[2], words[3]);
    return obj_new_string(0, 0);
}

fn eval_concat(words: []const i32) i32 {
    if (words.len <= 1) return obj_new_string(0, 0);
    var acc = words[1];
    var ci: usize = 2;
    while (ci < words.len) : (ci += 1) {
        acc = rt.tcl_cmd_concat(acc, words[ci]);
    }
    return acc;
}

fn eval_join(words: []const i32) i32 {
    if (words.len >= 3) return rt.tcl_cmd_join(words[1], words[2]);
    if (words.len >= 2) {
        const sp = alloc(1);
        const d: [*]u8 = @ptrFromInt(sp);
        d[0] = ' ';
        return rt.tcl_cmd_join(words[1], obj_new_string(@intCast(sp), 1));
    }
    return obj_new_string(0, 0);
}

fn eval_split(words: []const i32) i32 {
    if (words.len >= 3) return rt.tcl_cmd_split(words[1], words[2]);
    if (words.len >= 2) return rt.tcl_cmd_split(words[1], obj_new_string(0, 0));
    return obj_new_string(0, 0);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "list",     .handler = &eval_list },
    .{ .name = "lappend",  .handler = &eval_lappend },
    .{ .name = "llength",  .handler = &eval_llength },
    .{ .name = "lindex",   .handler = &eval_lindex },
    .{ .name = "lset",     .handler = &eval_lset },
    .{ .name = "linsert",  .handler = &eval_linsert },
    .{ .name = "lreplace", .handler = &eval_lreplace },
    .{ .name = "lsort",    .handler = &eval_lsort },
    .{ .name = "lsearch",  .handler = &eval_lsearch },
    .{ .name = "lrange",   .handler = &eval_lrange },
    .{ .name = "concat",   .handler = &eval_concat },
    .{ .name = "join",     .handler = &eval_join },
    .{ .name = "split",    .handler = &eval_split },
};
