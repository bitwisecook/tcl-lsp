// List manipulation commands: list, lappend, llength, lindex, lset,
// linsert, lreplace, lsort, lsearch, lrange, concat, join, split,
// lreverse, lrepeat, lassign, lmap, lseq.

const rt       = @import("../tcl_runtime.zig");
const frames   = @import("../interp/tcl_frames.zig");
const obj_mod  = @import("../value/tcl_obj.zig");
const reg      = @import("../dispatch/tcl_cmd_registry.zig");
const list_mod = @import("../value/tcl_list.zig");
const interp   = @import("../interp/tcl_interp.zig");

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

fn eval_lreverse(words: []const i32) i32 {
    if (words.len < 2) return obj_new_string(0, 0);
    return rt.tcl_cmd_list_reverse(words[1]);
}

fn eval_lrepeat(words: []const i32) i32 {
    if (words.len < 3) return obj_new_string(0, 0);
    if (words.len == 3) return rt.tcl_cmd_list_repeat(words[1], words[2]);
    // Multi-value: build one cycle then repeat it count times.
    const count_val = rt.obj_get_int(words[1]);
    if (count_val <= 0) return obj_new_string(0, 0);
    const count: u32 = @intCast(count_val);
    var cycle_max: u32 = 0;
    var vi: u32 = 2;
    while (vi < words.len) : (vi += 1) {
        const s = obj_ensure_string(words[vi]);
        cycle_max += s.len * 2 + 2;
        if (vi > 2) cycle_max += 1;
    }
    const cycle_buf = alloc(cycle_max + 4);
    var cycle_off: u32 = 0;
    vi = 2;
    while (vi < words.len) : (vi += 1) {
        if (vi > 2) {
            const d: [*]u8 = @ptrFromInt(cycle_buf + cycle_off);
            d[0] = ' ';
            cycle_off += 1;
        }
        const s = obj_ensure_string(words[vi]);
        if (vi == 2) {
            cycle_off = obj_mod.list_elem_quote(cycle_buf, cycle_off, s.ptr, s.len);
        } else {
            cycle_off = obj_mod.list_elem_quote_nth(cycle_buf, cycle_off, s.ptr, s.len);
        }
    }
    if (cycle_off == 0) return obj_new_string(0, 0);
    const total: u32 = count * (cycle_off + 1);
    const result_buf = alloc(total);
    var off: u32 = 0;
    var ci: u32 = 0;
    while (ci < count) : (ci += 1) {
        if (ci > 0) {
            const d: [*]u8 = @ptrFromInt(result_buf + off);
            d[0] = ' ';
            off += 1;
        }
        rt.memcpy(result_buf + off, cycle_buf, cycle_off);
        off += cycle_off;
    }
    return obj_new_string(@intCast(result_buf), @intCast(off));
}

fn eval_lassign(words: []const i32) i32 {
    if (words.len < 2) return obj_new_string(0, 0);
    const list_obj = words[1];
    const list_s = obj_ensure_string(list_obj);
    const n = rt.list_count_elements(list_s.ptr, list_s.len);
    var pi: u32 = 2;
    var i: i64 = 0;
    while (pi < words.len) : (pi += 1) {
        const val: i32 = if (i < n) blk: {
            const elem = rt.list_element_at(list_s.ptr, list_s.len, i);
            if (elem.braced) {
                break :blk rt.obj_new_string_copy(list_s.ptr + elem.start, elem.len);
            } else {
                const buf = alloc(elem.len + 4);
                const out_len = rt.copy_unbraced_elem(buf, list_s.ptr + elem.start, elem.len);
                break :blk obj_new_string(@intCast(buf), @intCast(out_len));
            }
        } else obj_new_string(0, 0);
        _ = frames.var_set(words[pi], val);
        i += 1;
    }
    const assigned: i64 = @intCast(words.len - 2);
    if (assigned >= n or n == 0) return obj_new_string(0, 0);
    const start_obj = obj_new_int(assigned);
    return list_mod.list_tail(list_obj, start_obj);
}

fn eval_lmap(words: []const i32) i32 {
    if (words.len < 4) return obj_new_string(0, 0);
    const var_name = words[1];
    const list_s = obj_ensure_string(words[2]);
    const body_s = obj_ensure_string(words[3]);
    const n = rt.list_count_elements(list_s.ptr, list_s.len);
    var result: i32 = obj_new_string(0, 0);
    var idx: i64 = 0;
    while (idx < n) : (idx += 1) {
        const elem = rt.list_element_at(list_s.ptr, list_s.len, idx);
        const elem_val: i32 = if (elem.braced)
            rt.obj_new_string_copy(list_s.ptr + elem.start, elem.len)
        else blk: {
            const buf = alloc(elem.len + 4);
            const out_len = rt.copy_unbraced_elem(buf, list_s.ptr + elem.start, elem.len);
            break :blk obj_new_string(@intCast(buf), @intCast(out_len));
        };
        _ = frames.var_set(var_name, elem_val);
        const item = interp.eval_script(body_s.ptr, body_s.len);
        if (rt.break_flag.* != 0) { rt.break_flag.* = 0; break; }
        if (rt.continue_flag.* != 0) { rt.continue_flag.* = 0; continue; }
        if (rt.error_flag.* != 0 or rt.return_flag.* != 0) return item;
        result = rt.tcl_list(result, item);
    }
    return result;
}

fn eval_lseq(words: []const i32) i32 {
    if (words.len < 2) return obj_new_string(0, 0);
    var start_val: i64 = 0;
    var end_val: i64 = 0;
    var step_val: i64 = 1;
    if (words.len == 2) {
        const count = rt.obj_get_int(words[1]);
        if (count <= 0) return obj_new_string(0, 0);
        end_val = count - 1;
    } else {
        start_val = rt.obj_get_int(words[1]);
        end_val   = rt.obj_get_int(words[2]);
        if (end_val < start_val) step_val = -1;
        if (words.len >= 5) {
            const by_s = obj_ensure_string(words[3]);
            if (by_s.len == 2) {
                const bp: [*]const u8 = @ptrFromInt(by_s.ptr);
                if (bp[0] == 'b' and bp[1] == 'y') {
                    step_val = rt.obj_get_int(words[4]);
                }
            }
        }
    }
    if (step_val == 0) return obj_new_string(0, 0);
    var acc: i32 = obj_new_string(0, 0);
    var i: i64 = start_val;
    if (step_val > 0) {
        while (i <= end_val) : (i += step_val) acc = rt.tcl_list(acc, obj_new_int(i));
    } else {
        while (i >= end_val) : (i += step_val) acc = rt.tcl_list(acc, obj_new_int(i));
    }
    return acc;
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
    .{ .name = "lreverse", .handler = &eval_lreverse },
    .{ .name = "lrepeat",  .handler = &eval_lrepeat },
    .{ .name = "lassign",  .handler = &eval_lassign },
    .{ .name = "lmap",     .handler = &eval_lmap },
    .{ .name = "lseq",     .handler = &eval_lseq },
};
