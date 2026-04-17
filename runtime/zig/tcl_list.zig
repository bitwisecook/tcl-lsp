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
const copy_unbraced_elem = obj.copy_unbraced_elem;
const list_elem_quote = obj.list_elem_quote;
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

// Exported: list append — append element to list with proper quoting.
//
// Fast path (the common case): the existing list representation is kept
// verbatim — we only trim trailing whitespace, append a single space,
// and append the quoted new element.  Existing elements retain
// whatever canonical form they already had (braced / backslash-escaped),
// so repeated lappend is O(n) per call instead of the O(existing_elems)
// re-parse + re-quote the two-pass approach used before.
//
// Slow path: if the existing list's last non-whitespace byte is an
// unpaired backslash, the simple concat would turn the appended space
// into a literal character (``\ ``) and merge our new element into
// the last existing one.  In that rare case we fall back to the
// parse / re-quote path that guarantees correct element boundaries.
pub export fn tcl_cmd_lappend(current: i32, value: i32) i32 {
    const sc = obj_ensure_string(current);
    const sv = obj_ensure_string(value);
    // Worst-case buffer: existing content verbatim + separator + new element (2x+2).
    const max_buf: u32 = sc.len + sv.len * 2 + 8;
    const buf = alloc(max_buf);
    var off: u32 = 0;

    if (sc.len > 0) {
        // Strip trailing whitespace from the existing representation.
        const cp: [*]const u8 = @ptrFromInt(sc.ptr);
        var end: u32 = sc.len;
        while (end > 0 and is_space(cp[end - 1])) end -= 1;
        if (end > 0) {
            // Count trailing backslashes — odd means the next space
            // would be eaten as a ``\<space>`` escape.  Fall back to
            // re-parse if so.
            var bs_count: u32 = 0;
            var ei: u32 = end;
            while (ei > 0 and cp[ei - 1] == '\\') : (ei -= 1) {
                bs_count += 1;
            }
            if ((bs_count & 1) == 1) {
                return lappend_reparse(sc.ptr, sc.len, sv.ptr, sv.len);
            }
            memcpy(buf, sc.ptr, end);
            off = end;
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
    }

    off = list_elem_quote(buf, off, sv.ptr, sv.len);
    return obj_new_string(@intCast(buf), @intCast(off));
}

/// Fallback for :func:`tcl_cmd_lappend`: parse the existing list into
/// elements, re-quote each, then append the new element.  Only invoked
/// when the fast concat path would misplace the element boundary
/// (existing rep ends with an unpaired ``\``).
fn lappend_reparse(sc_ptr: u32, sc_len: u32, sv_ptr: u32, sv_len: u32) i32 {
    const max_buf: u32 = sc_len * 3 + sv_len * 2 + 8;
    const buf = alloc(max_buf);
    var off: u32 = 0;
    const n = list_count_elements(sc_ptr, sc_len);
    var idx: i64 = 0;
    while (idx < n) : (idx += 1) {
        if (idx > 0) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        const elem = list_element_at(sc_ptr, sc_len, idx);
        if (elem.braced) {
            off = list_elem_quote(buf, off, sc_ptr + elem.start, elem.len);
        } else {
            const tmp = alloc(elem.len + 1);
            const actual_len = copy_unbraced_elem(tmp, sc_ptr + elem.start, elem.len);
            off = list_elem_quote(buf, off, tmp, actual_len);
        }
    }
    if (n > 0) {
        const d: [*]u8 = @ptrFromInt(buf + off);
        d[0] = ' ';
        off += 1;
    }
    off = list_elem_quote(buf, off, sv_ptr, sv_len);
    return obj_new_string(@intCast(buf), @intCast(off));
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

// Parse a list index that may be "end", "end-N", or a plain integer.
// Returns the resolved 0-based index, or -1 for out-of-range.
fn resolve_list_index(idx: i32, n: i64) i64 {
    const sv = obj_ensure_string(idx);
    if (sv.len >= 3) {
        const sp: [*]const u8 = @ptrFromInt(sv.ptr);
        if (sp[0] == 'e' and sp[1] == 'n' and sp[2] == 'd') {
            if (sv.len == 3) return n - 1;  // "end"
            if (sv.len >= 5 and sp[3] == '-') {
                // "end-N"
                var offset: i64 = 0;
                var i: u32 = 4;
                while (i < sv.len and sp[i] >= '0' and sp[i] <= '9') : (i += 1) {
                    offset = offset * 10 + @as(i64, sp[i] - '0');
                }
                return n - 1 - offset;
            }
            if (sv.len >= 5 and sp[3] == '+') {
                // "end+N"
                var offset: i64 = 0;
                var i: u32 = 4;
                while (i < sv.len and sp[i] >= '0' and sp[i] <= '9') : (i += 1) {
                    offset = offset * 10 + @as(i64, sp[i] - '0');
                }
                return n - 1 + offset;
            }
        }
    }
    return obj_get_int(idx);
}

// Exported: list index — extract the nth element (0-based).
pub export fn tcl_cmd_list_index(list: i32, idx: i32) i32 {
    const s = obj_ensure_string(list);
    const n = list_count_elements(s.ptr, s.len);
    const i_val = resolve_list_index(idx, n);
    if (i_val < 0 or i_val >= n) return obj_new_string(0, 0);
    const elem = list_element_at(s.ptr, s.len, i_val);
    if (elem.braced) return obj_new_string_copy(s.ptr + elem.start, elem.len);
    // Unbraced element: process backslash escapes.
    const buf = alloc(elem.len);
    const out_len = copy_unbraced_elem(buf, s.ptr + elem.start, elem.len);
    return obj_new_string(@intCast(buf), @intCast(out_len));
}

// Exported: list range — extract elements [first..last] (inclusive).
pub export fn tcl_cmd_list_range(list: i32, first: i32, last: i32) i32 {
    const s = obj_ensure_string(list);
    const total = list_count_elements(s.ptr, s.len);
    var f = resolve_list_index(first, total);
    var l = resolve_list_index(last, total);
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
