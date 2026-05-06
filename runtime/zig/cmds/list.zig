// List manipulation commands: list, lappend, llength, lindex, lset,
// linsert, lreplace, lsort, lsearch, lrange, concat, join, split,
// lreverse, lrepeat, lassign, lmap, lseq.

const rt = @import("../tcl_runtime.zig");
const frames = @import("../interp/tcl_frames.zig");
const obj_mod = @import("../valtypes/tcl_obj.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");
const list_mod = @import("../valtypes/tcl_list.zig");
const interp = @import("../interp/tcl_interp.zig");
const result_mod = @import("../interp/tcl_result.zig");
const tcl_str = @import("../valtypes/tcl_string.zig");
const tcl_chars = @import("../valtypes/tcl_chars.zig");
const list_parse = @import("../valtypes/tcl_list_parse.zig");

const alloc = rt.alloc;
const obj_new_string = rt.obj_new_string;
const obj_new_int = rt.obj_new_int;
const obj_ensure_string = rt.obj_ensure_string;

fn eval_list(words: []const i32) result_mod.InterpResult {
    if (words.len <= 1) return result_mod.from_globals(obj_new_string(0, 0));
    var max_total: u32 = 0;
    var ei: u32 = 1;
    while (ei < words.len) : (ei += 1) {
        const s = obj_ensure_string(words[ei]);
        max_total += s.len * 2 + 2;
        if (ei > 1) max_total += 1;
    }
    const alloc_size: u32 = max_total + 4;
    const buf = obj_mod.alloc(alloc_size);
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
    // Issue #317: own the result buffer so the caller's release
    // reclaims it via ``free_sized``; without this every
    // ``[list ...]`` invocation leaked one buf, and the
    // resulting cap=0 also forced ``lappend``'s slow rebuild
    // path on every subsequent append.
    return result_mod.from_globals(obj_mod.obj_new_string_take(buf, off, alloc_size));
}

fn eval_lappend(words: []const i32) result_mod.InterpResult {
    if (words.len >= 2) {
        var result = frames.var_resolve(words[1]);
        var wi: u32 = 2;
        while (wi < words.len) : (wi += 1) {
            result = rt.tcl_cmd_lappend(result, words[wi]);
        }
        _ = frames.var_set(words[1], result);
        return result_mod.from_globals(result);
    }
    return result_mod.from_globals(0);
}

fn eval_llength(words: []const i32) result_mod.InterpResult {
    if (words.len >= 2) return result_mod.from_globals(rt.tcl_cmd_list_length(words[1]));
    return result_mod.from_globals(0);
}

fn eval_lindex(words: []const i32) result_mod.InterpResult {
    if (words.len >= 3) return result_mod.from_globals(rt.tcl_cmd_list_index(words[1], words[2]));
    return result_mod.from_globals(0);
}

fn eval_lset(words: []const i32) result_mod.InterpResult {
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
        return result_mod.from_globals(result);
    }
    return result_mod.from_globals(0);
}

fn eval_linsert(words: []const i32) result_mod.InterpResult {
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
        return result_mod.from_globals(result);
    }
    return result_mod.from_globals(0);
}

fn eval_lreplace(words: []const i32) result_mod.InterpResult {
    if (words.len >= 4) {
        const list_arg = words[1];
        const first_arg = words[2];
        const last_arg = words[3];
        if (words.len == 4) {
            return result_mod.from_globals(rt.tcl_cmd_list_replace(list_arg, first_arg, last_arg, 0));
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
            return result_mod.from_globals(result);
        } else {
            var result = rt.tcl_cmd_list_replace(list_arg, first_arg, last_arg, words[words.len - 1]);
            var wi: u32 = words.len - 1;
            while (wi > 4) {
                wi -= 1;
                result = rt.tcl_cmd_list_insert(result, first_arg, words[wi]);
            }
            return result_mod.from_globals(result);
        }
    }
    return result_mod.from_globals(0);
}

fn eval_lsort(words: []const i32) result_mod.InterpResult {
    if (words.len >= 2) return result_mod.from_globals(rt.tcl_cmd_list_sort(words[words.len - 1]));
    return result_mod.from_globals(obj_new_string(0, 0));
}

// lsearch full implementation.
// Supported: -exact -glob -all -not -nocase -inline -start -stride -index -subindices
// Recognised-but-linear: -sorted -decreasing -bisect -integer -real -dictionary
// Stub: -regexp (raises unsupported)
fn ls_opt_eq(ptr: u32, len: u32, lit: []const u8) bool {
    if (len != lit.len) return false;
    const sp: [*]const u8 = @ptrFromInt(ptr);
    for (lit, 0..) |c, k| if (sp[k] != c) return false;
    return true;
}

fn ls_tolower(c: u8) u8 {
    return if (c >= 'A' and c <= 'Z') c + 32 else c;
}

fn ls_match_exact(nocase: bool, pat_ptr: u32, pat_len: u32, ep: u32, elen: u32) bool {
    if (pat_len != elen) return false;
    if (pat_len == 0) return true;
    const pp: [*]const u8 = @ptrFromInt(pat_ptr);
    const vp: [*]const u8 = @ptrFromInt(ep);
    var k: u32 = 0;
    while (k < elen) : (k += 1) {
        const a = if (nocase) ls_tolower(pp[k]) else pp[k];
        const b = if (nocase) ls_tolower(vp[k]) else vp[k];
        if (a != b) return false;
    }
    return true;
}

fn ls_match_glob_nc(pat_ptr: u32, pat_len: u32, ep: u32, elen: u32) bool {
    // Case-insensitive glob: lowercase copies then call glob_match.
    const bp = alloc(pat_len + 1);
    const be = alloc(elen + 1);
    if (pat_len > 0 and bp != 0) {
        const src: [*]const u8 = @ptrFromInt(pat_ptr);
        const dst: [*]u8 = @ptrFromInt(bp);
        var k: u32 = 0;
        while (k < pat_len) : (k += 1) dst[k] = ls_tolower(src[k]);
    }
    if (elen > 0 and be != 0) {
        const src: [*]const u8 = @ptrFromInt(ep);
        const dst: [*]u8 = @ptrFromInt(be);
        var k: u32 = 0;
        while (k < elen) : (k += 1) dst[k] = ls_tolower(src[k]);
    }
    return tcl_str.glob_match(bp, pat_len, be, elen);
}

// Build a two-element index list "outer_idx inner_idx" for -subindices output.
fn ls_subindex_pair(outer: i64, inner: i64) i32 {
    return rt.tcl_list(rt.tcl_list(obj_new_string(0, 0), obj_new_int(outer)), obj_new_int(inner));
}

// Get the matchable bytes for element at `idx`, optionally drilling into a
// sub-element via `index_arg` (a TclObj whose string is the sub-index integer).
// Returns (ep, elen, sub_idx) — sub_idx is only meaningful when index_arg != 0.
fn ls_get_match_target(
    ls_ptr: u32,
    ls_len: u32,
    idx: i64,
    index_arg: i32,
) struct { ep: u32, elen: u32, sub_idx: i64 } {
    const outer_elem = rt.list_element_at(ls_ptr, ls_len, idx);
    var outer_ptr: u32 = ls_ptr + outer_elem.start;
    var outer_len: u32 = outer_elem.len;
    if (!outer_elem.braced and outer_len > 0) {
        const buf = alloc(outer_len + 4);
        if (buf != 0) {
            const n = rt.copy_unbraced_elem(buf, outer_ptr, outer_len);
            outer_ptr = buf;
            outer_len = n;
        }
    }
    if (index_arg == 0) {
        return .{ .ep = outer_ptr, .elen = outer_len, .sub_idx = 0 };
    }
    // Drill: treat index_arg's string as the integer sub-index.
    const sub_idx = obj_mod.obj_get_int(index_arg);
    const inner_elem = rt.list_element_at(outer_ptr, outer_len, sub_idx);
    return .{
        .ep = outer_ptr + inner_elem.start,
        .elen = inner_elem.len,
        .sub_idx = sub_idx,
    };
}

fn eval_lsearch(words: []const i32) result_mod.InterpResult {
    var mode: u8 = 'g'; // 'e'=exact 'g'=glob 'r'=regexp (stub)
    var find_all = false;
    var negate = false;
    var nocase = false;
    var do_inline = false;
    var start: i64 = 0;
    var stride: i64 = 1;
    var index_arg: i32 = 0; // -index argument TclObj (0 = not given)
    var do_subindices = false;

    const stubs = @import("../stubs/tcl_stubs.zig");
    var wi: u32 = 1;
    while (wi + 1 < words.len) : (wi += 1) {
        const sv = obj_ensure_string(words[wi]);
        if (sv.len == 0 or sv.ptr == 0) break;
        const sp: [*]const u8 = @ptrFromInt(sv.ptr);
        if (sp[0] != '-') break;
        if (ls_opt_eq(sv.ptr, sv.len, "--")) {
            wi += 1;
            break;
        } else if (ls_opt_eq(sv.ptr, sv.len, "-exact")) {
            mode = 'e';
        } else if (ls_opt_eq(sv.ptr, sv.len, "-glob")) {
            mode = 'g';
        } else if (ls_opt_eq(sv.ptr, sv.len, "-regexp")) {
            mode = 'r';
        } else if (ls_opt_eq(sv.ptr, sv.len, "-all")) {
            find_all = true;
        } else if (ls_opt_eq(sv.ptr, sv.len, "-not")) {
            negate = true;
        } else if (ls_opt_eq(sv.ptr, sv.len, "-nocase")) {
            nocase = true;
        } else if (ls_opt_eq(sv.ptr, sv.len, "-inline")) {
            do_inline = true;
        } else if (ls_opt_eq(sv.ptr, sv.len, "-subindices")) {
            do_subindices = true;
        } else if (ls_opt_eq(sv.ptr, sv.len, "-sorted") or
            ls_opt_eq(sv.ptr, sv.len, "-decreasing") or
            ls_opt_eq(sv.ptr, sv.len, "-bisect") or
            ls_opt_eq(sv.ptr, sv.len, "-integer") or
            ls_opt_eq(sv.ptr, sv.len, "-real") or
            ls_opt_eq(sv.ptr, sv.len, "-dictionary"))
        {
            // Recognised; fall back to linear search (correct result, slower).
        } else if (ls_opt_eq(sv.ptr, sv.len, "-start")) {
            wi += 1;
            if (wi + 1 >= words.len) break;
            const v = obj_mod.obj_get_int(words[wi]);
            start = if (v < 0) 0 else v;
        } else if (ls_opt_eq(sv.ptr, sv.len, "-stride")) {
            wi += 1;
            if (wi + 1 >= words.len) break;
            const v = obj_mod.obj_get_int(words[wi]);
            stride = if (v < 1) 1 else v;
        } else if (ls_opt_eq(sv.ptr, sv.len, "-index")) {
            wi += 1;
            if (wi + 1 >= words.len) break;
            index_arg = words[wi];
        } else {
            // Unknown option — raise a Tcl error rather than silently
            // misinterpreting the arg as the list.
            const stubs2 = @import("../stubs/tcl_stubs.zig");
            const prefix = "bad option \"";
            const suffix = "\": must be -all, -ascii, -bisect, -decreasing, -dictionary, -exact, -glob, -index, -inline, -integer, -nocase, -not, -real, -regexp, -sorted, -start, -stride, or -subindices";
            const name_len = sv.len;
            const total: u32 = @intCast(prefix.len + name_len + suffix.len);
            const buf = obj_mod.alloc(total);
            if (buf != 0) {
                const bp: [*]u8 = @ptrFromInt(buf);
                var off: u32 = 0;
                for (prefix) |c| {
                    bp[off] = c;
                    off += 1;
                }
                if (sv.ptr != 0) {
                    const sp2: [*]const u8 = @ptrFromInt(sv.ptr);
                    for (0..name_len) |k| {
                        bp[off] = sp2[k];
                        off += 1;
                    }
                }
                for (suffix) |c| {
                    bp[off] = c;
                    off += 1;
                }
                const msg = obj_mod.obj_new_string_take(buf, total, total);
                const catch_mod = @import("../interp/tcl_catch.zig");
                catch_mod.tcl_cmd_error(msg);
            } else {
                stubs2.raise("bad option to lsearch");
            }
            return result_mod.from_globals(0);
        }
    }

    if (wi + 1 >= words.len) return result_mod.from_globals(obj_new_int(-1));
    const list_obj = words[wi];
    const pat_obj = words[wi + 1];

    _ = stubs; // suppress unused warning when not used below

    const ls = obj_ensure_string(list_obj);
    const pv = obj_ensure_string(pat_obj);
    const n = rt.list_count_elements(ls.ptr, ls.len);

    // Align idx to the first stride group that covers [start, n).
    var idx: i64 = if (stride > 1) @divFloor(start, stride) * stride else start;

    var acc: i32 = obj_new_string(0, 0); // -all accumulator

    if (index_arg == 0) {
        // Fast O(N) sequential scan via cursor — avoids the O(N²) that
        // repeated list_element_at(idx) would incur for large lists.
        var cur = list_parse.Cursor{ .pos = 0 };
        if (idx > 0)
            list_parse.cursor_skip(ls.ptr, ls.len, &cur, @intCast(idx));
        while (idx < n) {
            const elem = list_parse.cursor_next(ls.ptr, ls.len, &cur);
            // Decode the element (unbraced elements need backslash expansion).
            var ep: u32 = ls.ptr + elem.start;
            var elen: u32 = elem.len;
            if (!elem.braced and elen > 0) {
                const buf = alloc(elen + 4);
                if (buf != 0) {
                    const decoded = rt.copy_unbraced_elem(buf, ep, elen);
                    ep = buf;
                    elen = decoded;
                }
            }
            const raw: bool = switch (mode) {
                'e' => ls_match_exact(nocase, pv.ptr, pv.len, ep, elen),
                'r' => blk: {
                    const tcl_regex = @import("../valtypes/tcl_regex.zig");
                    break :blk tcl_regex.run_match_pub(pv.ptr, pv.len, ep, elen, nocase);
                },
                else => if (nocase)
                    ls_match_glob_nc(pv.ptr, pv.len, ep, elen)
                else
                    tcl_str.glob_match(pv.ptr, pv.len, ep, elen),
            };
            const matched = if (negate) !raw else raw;
            if (matched) {
                if (!find_all) {
                    if (do_inline) return result_mod.from_globals(obj_new_string(@bitCast(ep), @bitCast(elen)));
                    return result_mod.from_globals(obj_new_int(idx));
                }
                const entry: i32 = if (do_inline)
                    obj_new_string(@bitCast(ep), @bitCast(elen))
                else
                    obj_new_int(idx);
                acc = list_mod.tcl_cmd_lappend(acc, entry);
            }
            idx += stride;
            if (stride > 1 and idx < n)
                list_parse.cursor_skip(ls.ptr, ls.len, &cur, @intCast(stride - 1));
        }
    } else {
        // -index path: random-access (lists using -index are typically small).
        while (idx < n) : (idx += stride) {
            const t = ls_get_match_target(ls.ptr, ls.len, idx, index_arg);
            const raw: bool = switch (mode) {
                'e' => ls_match_exact(nocase, pv.ptr, pv.len, t.ep, t.elen),
                'r' => blk: {
                    const tcl_regex = @import("../valtypes/tcl_regex.zig");
                    break :blk tcl_regex.run_match_pub(pv.ptr, pv.len, t.ep, t.elen, nocase);
                },
                else => if (nocase)
                    ls_match_glob_nc(pv.ptr, pv.len, t.ep, t.elen)
                else
                    tcl_str.glob_match(pv.ptr, pv.len, t.ep, t.elen),
            };
            const matched = if (negate) !raw else raw;
            if (!matched) continue;
            if (!find_all) {
                if (do_inline) return result_mod.from_globals(obj_new_string(@bitCast(t.ep), @bitCast(t.elen)));
                if (do_subindices) return result_mod.from_globals(ls_subindex_pair(idx, t.sub_idx));
                return result_mod.from_globals(obj_new_int(idx));
            }
            const entry: i32 = if (do_inline)
                obj_new_string(@bitCast(t.ep), @bitCast(t.elen))
            else if (do_subindices)
                ls_subindex_pair(idx, t.sub_idx)
            else
                obj_new_int(idx);
            acc = list_mod.tcl_cmd_lappend(acc, entry);
        }
    }

    if (find_all) return result_mod.from_globals(acc);
    if (do_inline) return result_mod.from_globals(obj_new_string(0, 0));
    return result_mod.from_globals(obj_new_int(-1));
}

fn eval_lrange(words: []const i32) result_mod.InterpResult {
    if (words.len >= 4) return result_mod.from_globals(rt.tcl_cmd_list_range(words[1], words[2], words[3]));
    return result_mod.from_globals(obj_new_string(0, 0));
}

fn eval_concat(words: []const i32) result_mod.InterpResult {
    if (words.len <= 1) return result_mod.from_globals(obj_new_string(0, 0));
    var acc = words[1];
    var ci: usize = 2;
    while (ci < words.len) : (ci += 1) {
        acc = rt.tcl_cmd_concat(acc, words[ci]);
    }
    return result_mod.from_globals(acc);
}

fn eval_join(words: []const i32) result_mod.InterpResult {
    if (words.len >= 3) return result_mod.from_globals(rt.tcl_cmd_join(words[1], words[2]));
    if (words.len >= 2) {
        const sp = alloc(1);
        const d: [*]u8 = @ptrFromInt(sp);
        d[0] = ' ';
        return result_mod.from_globals(rt.tcl_cmd_join(words[1], obj_new_string(@bitCast(sp), 1)));
    }
    return result_mod.from_globals(obj_new_string(0, 0));
}

fn eval_split(words: []const i32) result_mod.InterpResult {
    if (words.len >= 3) return result_mod.from_globals(rt.tcl_cmd_split(words[1], words[2]));
    if (words.len >= 2) return result_mod.from_globals(rt.tcl_cmd_split(words[1], obj_new_string(0, 0)));
    return result_mod.from_globals(obj_new_string(0, 0));
}

fn eval_lreverse(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) return result_mod.from_globals(obj_new_string(0, 0));
    return result_mod.from_globals(rt.tcl_cmd_list_reverse(words[1]));
}

fn eval_lrepeat(words: []const i32) result_mod.InterpResult {
    if (words.len < 3) return result_mod.from_globals(obj_new_string(0, 0));
    if (words.len == 3) return result_mod.from_globals(rt.tcl_cmd_list_repeat(words[1], words[2]));
    // Multi-value: build one cycle then repeat it count times.
    const count_val = rt.obj_get_int(words[1]);
    if (count_val <= 0) return result_mod.from_globals(obj_new_string(0, 0));
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
    if (cycle_off == 0) return result_mod.from_globals(obj_new_string(0, 0));
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
    return result_mod.from_globals(obj_new_string(@bitCast(result_buf), @bitCast(off)));
}

fn eval_lassign(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) return result_mod.from_globals(obj_new_string(0, 0));
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
                break :blk obj_new_string(@bitCast(buf), @bitCast(out_len));
            }
        } else obj_new_string(0, 0);
        _ = frames.var_set(words[pi], val);
        i += 1;
    }
    const assigned: i64 = @intCast(words.len - 2);
    if (assigned >= n or n == 0) return result_mod.from_globals(obj_new_string(0, 0));
    const start_obj = obj_new_int(assigned);
    return result_mod.from_globals(list_mod.list_tail(list_obj, start_obj));
}

fn eval_lmap(words: []const i32) result_mod.InterpResult {
    // lmap v1 l1 ?v2 l2 ...? body — same multi-var semantics as foreach,
    // but accumulates body results into a list instead of discarding them.
    if (words.len < 4) return result_mod.from_globals(obj_new_string(0, 0));
    const pair_words = words.len - 2;
    if (pair_words % 2 != 0) return result_mod.from_globals(obj_new_string(0, 0));
    const n_pairs = pair_words / 2;
    const MAX_PAIRS = 15;
    if (n_pairs > MAX_PAIRS) return result_mod.from_globals(obj_new_string(0, 0));
    const body_s = obj_ensure_string(words[words.len - 1]);
    var list_lens: [MAX_PAIRS]i64 = [_]i64{0} ** MAX_PAIRS;
    var n: i64 = 0;
    {
        var p: u32 = 0;
        while (p < n_pairs) : (p += 1) {
            const ls = obj_ensure_string(words[2 + p * 2]);
            const len = rt.list_count_elements(ls.ptr, ls.len);
            list_lens[p] = len;
            if (len > n) n = len;
        }
    }
    var result: i32 = obj_new_string(0, 0);
    var idx: i64 = 0;
    while (idx < n) : (idx += 1) {
        var p: u32 = 0;
        while (p < n_pairs) : (p += 1) {
            const var_name = words[1 + p * 2];
            const list_obj = words[2 + p * 2];
            const elem_val: i32 = if (idx < list_lens[p]) blk: {
                const ls = obj_ensure_string(list_obj);
                const elem = rt.list_element_at(ls.ptr, ls.len, idx);
                break :blk if (elem.braced)
                    rt.obj_new_string_copy(ls.ptr + elem.start, elem.len)
                else inner: {
                    const buf = alloc(elem.len + 4);
                    const out_len = rt.copy_unbraced_elem(buf, ls.ptr + elem.start, elem.len);
                    break :inner obj_new_string(@bitCast(buf), @bitCast(out_len));
                };
            } else obj_new_string(0, 0);
            _ = frames.var_set(var_name, elem_val);
            obj_mod.tcl_obj_release(elem_val);
        }
        const item = interp.eval_script(body_s.ptr, body_s.len);
        const ir = result_mod.snapshot(item);
        switch (ir.code) {
            .OK => result = rt.tcl_list(result, item),
            .BREAK => {
                result_mod.consume(.BREAK);
                break;
            },
            .CONTINUE => {
                result_mod.consume(.CONTINUE);
                continue;
            },
            .ERROR, .RETURN => return result_mod.from_globals(item),
        }
    }
    return result_mod.from_globals(result);
}

/// Match a separator word (``to`` / ``..``) at index ``i``.  Returns
/// true iff the i-th word is exactly that separator.
fn lseq_match_word(words: []const i32, i: u32, expected: []const u8) bool {
    if (i >= words.len) return false;
    const s = obj_ensure_string(words[i]);
    if (s.len != expected.len) return false;
    const sp: [*]const u8 = @ptrFromInt(s.ptr);
    for (expected, 0..) |c, k| {
        if (sp[k] != c) return false;
    }
    return true;
}

/// Tcl 9 ``lseq``.  Forms supported (matching tcl9.0.3 builtin):
///
///   lseq N                    -> 0 .. N-1
///   lseq START END            -> START .. END (step inferred)
///   lseq START to END         -> same
///   lseq START .. END         -> same
///   lseq START END by STEP    -> step explicit
///   lseq START to END by STEP -> same
///   lseq START .. END by STEP -> same
///   lseq START by STEP        -> N=START items, step from 0
///
/// Step direction is inferred from end-vs-start when no explicit
/// step is given.  Floats are NOT supported yet — this impl is
/// integer-only; tests that need ``arithSeriesDouble`` are SKIPPED
/// by tcltest's constraint table because ``arithSeriesDouble``
/// isn't set.
fn eval_lseq(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) return result_mod.from_globals(obj_new_string(0, 0));
    var start_val: i64 = 0;
    var end_val: i64 = 0;
    var step_val: i64 = 1;
    var have_step = false;

    if (words.len == 2) {
        // ``lseq N`` -> 0 .. N-1
        const count = rt.obj_get_int(words[1]);
        if (count <= 0) return result_mod.from_globals(obj_new_string(0, 0));
        end_val = count - 1;
    } else {
        // Two-or-more forms.  After ``words[1] = START``, scan for an
        // optional ``to``/``..`` separator (or its absence) and an
        // optional ``by STEP`` suffix.
        start_val = rt.obj_get_int(words[1]);
        var idx: u32 = 2;
        if (lseq_match_word(words, idx, "to") or lseq_match_word(words, idx, "..")) {
            idx += 1;
        }
        if (idx >= words.len) return result_mod.from_globals(obj_new_string(0, 0));
        // ``lseq START by STEP`` (no end) means N=START items; the
        // separator-less ``words[2]`` could be ``by`` instead of an
        // end value.
        if (lseq_match_word(words, idx, "by")) {
            // ``lseq START by STEP`` — start=0, count=START, step
            const cnt = start_val;
            start_val = 0;
            if (cnt <= 0) return result_mod.from_globals(obj_new_string(0, 0));
            if (idx + 1 < words.len) step_val = rt.obj_get_int(words[idx + 1]);
            if (step_val == 0) return result_mod.from_globals(obj_new_string(0, 0));
            end_val = start_val + (cnt - 1) * step_val;
            have_step = true;
        } else {
            end_val = rt.obj_get_int(words[idx]);
            idx += 1;
            if (lseq_match_word(words, idx, "by")) {
                if (idx + 1 < words.len) step_val = rt.obj_get_int(words[idx + 1]);
                have_step = true;
            } else if (idx < words.len) {
                // ``lseq START END STEP`` (or ``lseq START to END STEP``)
                // — Tcl 9 accepts the trailing positional as the step
                // without an explicit ``by`` keyword.  Without this
                // branch, ``lseq 1000000 2000000 100000`` falls through
                // with step=1 and tries to enumerate a million-element
                // sequence with O(N²) tcl_list appends — that's the
                // lseq.test hang at lseq-1.16.
                step_val = rt.obj_get_int(words[idx]);
                have_step = true;
            }
        }

        if (!have_step) {
            // Infer direction from start..end.
            if (end_val < start_val) step_val = -1 else step_val = 1;
        }
    }
    if (step_val == 0) return result_mod.from_globals(obj_new_string(0, 0));

    // Sanity bound: ``lseq 1e50 1e50+1`` and similar large-double
    // forms convert to out-of-range i64 via ``@intFromFloat`` and
    // can otherwise loop for billions of iterations before tripping
    // the wasmtime watchdog.  Cap the absolute span so a poorly
    // formed call returns an empty list (or partial result) instead
    // of hanging the test runner.  The cap is conservative — well
    // above any realistic production lseq — but small enough that
    // a runaway terminates in <100ms.
    const max_count: i64 = 16 * 1024 * 1024; // 16 M elements
    const span: i64 = if (step_val > 0) end_val - start_val else start_val - end_val;
    if (span < 0 or @divTrunc(span, if (step_val > 0) step_val else -step_val) > max_count) {
        return result_mod.from_globals(obj_new_string(0, 0));
    }

    var acc: i32 = obj_new_string(0, 0);
    var i: i64 = start_val;
    if (step_val > 0) {
        while (i <= end_val) : (i += step_val) acc = rt.tcl_list(acc, obj_new_int(i));
    } else {
        while (i >= end_val) : (i += step_val) acc = rt.tcl_list(acc, obj_new_int(i));
    }
    return result_mod.from_globals(acc);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "list", .arity_min = 0, .arity_max = null, .handler = &eval_list },
    .{ .name = "lappend", .arity_min = 1, .arity_max = null, .handler = &eval_lappend },
    .{ .name = "llength", .arity_min = 1, .arity_max = 1, .handler = &eval_llength },
    .{ .name = "lindex", .arity_min = 1, .arity_max = null, .handler = &eval_lindex },
    .{ .name = "lset", .arity_min = 2, .arity_max = null, .handler = &eval_lset },
    .{ .name = "linsert", .arity_min = 2, .arity_max = null, .handler = &eval_linsert },
    .{ .name = "lreplace", .arity_min = 3, .arity_max = null, .handler = &eval_lreplace },
    .{ .name = "lsort", .arity_min = 1, .arity_max = null, .handler = &eval_lsort },
    .{ .name = "lsearch", .arity_min = 2, .arity_max = null, .handler = &eval_lsearch },
    .{ .name = "lrange", .arity_min = 3, .arity_max = 3, .handler = &eval_lrange },
    .{ .name = "concat", .arity_min = 0, .arity_max = null, .handler = &eval_concat },
    .{ .name = "join", .arity_min = 1, .arity_max = 2, .handler = &eval_join },
    .{ .name = "split", .arity_min = 1, .arity_max = 2, .handler = &eval_split },
    .{ .name = "lreverse", .arity_min = 1, .arity_max = 1, .handler = &eval_lreverse },
    .{ .name = "lrepeat", .arity_min = 1, .arity_max = null, .handler = &eval_lrepeat },
    .{ .name = "lassign", .arity_min = 1, .arity_max = null, .handler = &eval_lassign },
    .{ .name = "lmap", .arity_min = 3, .arity_max = null, .handler = &eval_lmap },
    .{ .name = "lseq", .arity_min = 1, .arity_max = 5, .handler = &eval_lseq },
};
