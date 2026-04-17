// List operations: list_length, lappend, list_index, list_range,
// list_sort, list_search, tcl_list.

const obj = @import("tcl_obj.zig");
const alloc = obj.alloc;
const memcpy = obj.memcpy;
const obj_ensure_string = obj.obj_ensure_string;
const obj_new_string = obj.obj_new_string;
const obj_new_int = obj.obj_new_int;
const obj_get_int = obj.obj_get_int;
const obj_new_string_copy = obj.obj_new_string_copy;
const is_space = obj.is_space;
const str_cmp = obj.str_cmp;
const list_count_elements = obj.list_count_elements;
const list_element_at = obj.list_element_at;
const dict_needs_braces = obj.dict_needs_braces;
const dict_append_elem = obj.dict_append_elem;
const read_i32 = obj.read_i32;
const write_i32 = obj.write_i32;

// Exported: list length — count elements by whitespace-splitting.
pub export fn tcl_cmd_list_length(list: i32) i32 {
    const s = obj_ensure_string(list);
    const n = list_count_elements(s.ptr, s.len);
    return obj_new_int(n);
}

// Exported: list append — append element to list (space-separated).
pub export fn tcl_cmd_lappend(current: i32, value: i32) i32 {
    const sc = obj_ensure_string(current);
    const sv = obj_ensure_string(value);
    var needs_braces = false;
    if (sv.len > 0) {
        const vsrc: [*]const u8 = @ptrFromInt(sv.ptr);
        for (0..sv.len) |i| {
            if (is_space(vsrc[i])) {
                needs_braces = true;
                break;
            }
        }
    }
    if (sc.len == 0) {
        if (needs_braces) {
            const buf = alloc(sv.len + 2);
            const dst: [*]u8 = @ptrFromInt(buf);
            dst[0] = '{';
            memcpy(buf + 1, sv.ptr, sv.len);
            dst[sv.len + 1] = '}';
            return obj_new_string(@intCast(buf), @intCast(sv.len + 2));
        }
        return value;
    }
    const extra: u32 = if (needs_braces) sv.len + 3 else sv.len + 1;
    const total = sc.len + extra;
    const buf = alloc(total);
    memcpy(buf, sc.ptr, sc.len);
    const dst: [*]u8 = @ptrFromInt(buf + sc.len);
    dst[0] = ' ';
    if (needs_braces) {
        dst[1] = '{';
        memcpy(buf + sc.len + 2, sv.ptr, sv.len);
        const d2: [*]u8 = @ptrFromInt(buf + sc.len + 2 + sv.len);
        d2[0] = '}';
    } else {
        memcpy(buf + sc.len + 1, sv.ptr, sv.len);
    }
    return obj_new_string(@intCast(buf), @intCast(total));
}

// Exported: list — create a list from individual elements.
pub export fn tcl_list(a: i32, b: i32) i32 {
    const sa = obj_ensure_string(a);
    const sb = obj_ensure_string(b);
    if (sa.len == 0 and sb.len == 0) return obj_new_string(0, 0);
    if (sa.len == 0) return b;
    if (sb.len == 0) return a;
    const a_braces = dict_needs_braces(sa.ptr, sa.len);
    const b_braces = dict_needs_braces(sb.ptr, sb.len);
    const a_extra: u32 = if (a_braces) sa.len + 2 else sa.len;
    const b_extra: u32 = if (b_braces) sb.len + 2 else sb.len;
    const total = a_extra + 1 + b_extra;
    const buf = alloc(total);
    var off: u32 = 0;
    off = dict_append_elem(buf, off, sa.ptr, sa.len);
    const d: [*]u8 = @ptrFromInt(buf + off);
    d[0] = ' ';
    off += 1;
    off = dict_append_elem(buf, off, sb.ptr, sb.len);
    return obj_new_string(@intCast(buf), @intCast(off));
}

// Exported: list index — extract the nth element (0-based).
pub export fn tcl_cmd_list_index(list: i32, idx: i32) i32 {
    const s = obj_ensure_string(list);
    const i_val = obj_get_int(idx);
    if (i_val < 0) return obj_new_string(0, 0);
    const n = list_count_elements(s.ptr, s.len);
    if (i_val >= n) return obj_new_string(0, 0);
    const elem = list_element_at(s.ptr, s.len, i_val);
    return obj_new_string_copy(s.ptr + elem.start, elem.len);
}

// Exported: list range — extract elements [first..last] (inclusive).
pub export fn tcl_cmd_list_range(list: i32, first: i32, last: i32) i32 {
    const s = obj_ensure_string(list);
    var f = obj_get_int(first);
    var l = obj_get_int(last);
    const total = list_count_elements(s.ptr, s.len);
    if (f < 0) f = 0;
    if (l >= total) l = total - 1;
    if (f > l or f >= total) return obj_new_string(0, 0);
    var result_len: u32 = 0;
    const result_buf: u32 = alloc(s.len);
    var idx: i64 = f;
    while (idx <= l) : (idx += 1) {
        if (idx > f) {
            const d: [*]u8 = @ptrFromInt(result_buf + result_len);
            d[0] = ' ';
            result_len += 1;
        }
        const elem = list_element_at(s.ptr, s.len, idx);
        if (elem.braced) {
            const d: [*]u8 = @ptrFromInt(result_buf + result_len);
            d[0] = '{';
            result_len += 1;
            memcpy(result_buf + result_len, s.ptr + elem.start, elem.len);
            result_len += elem.len;
            const d2: [*]u8 = @ptrFromInt(result_buf + result_len);
            d2[0] = '}';
            result_len += 1;
        } else {
            memcpy(result_buf + result_len, s.ptr + elem.start, elem.len);
            result_len += elem.len;
        }
    }
    return obj_new_string(@intCast(result_buf), @intCast(result_len));
}

// Exported: tail of a list — elements from *start* onwards.  Used by
// ``lassign`` to return the leftover elements after variable binding.
// An empty or out-of-range start yields an empty list.
pub export fn list_tail(list: i32, start: i32) i32 {
    const s = obj_ensure_string(list);
    const start_val = obj_get_int(start);
    const total = list_count_elements(s.ptr, s.len);
    if (start_val >= total or start_val < 0) return obj_new_string(0, 0);
    // Re-use list_range with last = total - 1 (inclusive).  list_range's
    // ``first``/``last`` arguments are TclObj pointers, so box them.
    const start_obj = obj_new_int(start_val);
    const last_obj = obj_new_int(total - 1);
    return tcl_cmd_list_range(list, start_obj, last_obj);
}

// Exported: list sort — simple insertion sort on string comparison.
pub export fn tcl_cmd_list_sort(list: i32) i32 {
    const s = obj_ensure_string(list);
    const n_i64 = list_count_elements(s.ptr, s.len);
    if (n_i64 <= 1) return list;
    const n: u32 = @intCast(n_i64);
    const arr_buf = alloc(n * 8);
    var idx: u32 = 0;
    while (idx < n) : (idx += 1) {
        const elem = list_element_at(s.ptr, s.len, @intCast(idx));
        write_i32(arr_buf + idx * 8, @intCast(s.ptr + elem.start));
        write_i32(arr_buf + idx * 8 + 4, @intCast(elem.len));
    }
    var i: u32 = 1;
    while (i < n) : (i += 1) {
        const key_ptr: u32 = @intCast(read_i32(arr_buf + i * 8));
        const key_len: u32 = @intCast(read_i32(arr_buf + i * 8 + 4));
        var j: i32 = @as(i32, @intCast(i)) - 1;
        while (j >= 0) {
            const j_u: u32 = @intCast(j);
            const cmp_ptr: u32 = @intCast(read_i32(arr_buf + j_u * 8));
            const cmp_len: u32 = @intCast(read_i32(arr_buf + j_u * 8 + 4));
            if (str_cmp(cmp_ptr, cmp_len, key_ptr, key_len) > 0) {
                write_i32(arr_buf + (j_u + 1) * 8, @intCast(cmp_ptr));
                write_i32(arr_buf + (j_u + 1) * 8 + 4, @intCast(cmp_len));
                j -= 1;
            } else break;
        }
        const ins: u32 = @intCast(j + 1);
        write_i32(arr_buf + ins * 8, @intCast(key_ptr));
        write_i32(arr_buf + ins * 8 + 4, @intCast(key_len));
    }
    var result_len: u32 = 0;
    const result_buf = alloc(s.len + n);
    idx = 0;
    while (idx < n) : (idx += 1) {
        if (idx > 0) {
            const d: [*]u8 = @ptrFromInt(result_buf + result_len);
            d[0] = ' ';
            result_len += 1;
        }
        const e_ptr: u32 = @intCast(read_i32(arr_buf + idx * 8));
        const e_len: u32 = @intCast(read_i32(arr_buf + idx * 8 + 4));
        memcpy(result_buf + result_len, e_ptr, e_len);
        result_len += e_len;
    }
    return obj_new_string(@intCast(result_buf), @intCast(result_len));
}

// Exported: list search — linear search for exact match, returns index or -1.
pub export fn tcl_cmd_list_search(list: i32, value: i32) i32 {
    const s = obj_ensure_string(list);
    const sv = obj_ensure_string(value);
    const n = list_count_elements(s.ptr, s.len);
    var idx: i64 = 0;
    while (idx < n) : (idx += 1) {
        const elem = list_element_at(s.ptr, s.len, idx);
        if (str_cmp(s.ptr + elem.start, elem.len, sv.ptr, sv.len) == 0) {
            return obj_new_int(idx);
        }
    }
    return obj_new_int(-1);
}
