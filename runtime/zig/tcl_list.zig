// List operations: list_length, lappend, list_index, list_range,
// list_sort, list_search, tcl_list.

const obj = @import("tcl_obj.zig");
const str = @import("tcl_string.zig");
const alloc = obj.alloc;
const memcpy = obj.memcpy;
const obj_ensure_string = obj.obj_ensure_string;
const obj_new_string = obj.obj_new_string;
const obj_new_int = obj.obj_new_int;
const obj_get_int = obj.obj_get_int;
const obj_new_string_copy = obj.obj_new_string_copy;
const copy_unbraced_elem = obj.copy_unbraced_elem;
const list_elem_quote = obj.list_elem_quote;
const list_elem_quote_nth = obj.list_elem_quote_nth;
const str_cmp = obj.str_cmp;
const list_count_elements = obj.list_count_elements;
const list_element_at = obj.list_element_at;
const read_i32 = obj.read_i32;
const write_i32 = obj.write_i32;

// Exported: list length — count elements by whitespace-splitting.
pub export fn tcl_cmd_list_length(list: i32) i32 {
    const s = obj_ensure_string(list);
    const n = list_count_elements(s.ptr, s.len);
    return obj_new_int(n);
}

// Exported: list append — append a single element to a list, with
// proper Tcl quoting of every element.  Matches ``lappend`` semantics
// in reference Tcl: the existing value is parsed as a list, each
// element is re-rendered in canonical form (``TclScanElement`` /
// ``TclConvertElement``), then the new value is appended.  The
// canonical rep is what tests like ``append-4.7`` observe — e.g.
// ``lappend x abc`` where ``x`` = ``a{`` must produce ``a\{ abc``,
// not ``a{ abc``, because ``a{`` (as a list element) canonicalises
// to ``a\{``.
//
// Performance note: repeated lappend walks the full existing list
// each call (``O(n)``).  This matches tclsh's behaviour when the
// value doesn't carry a cached list internal rep — which is always
// the case in this runtime, since values are plain strings.
pub export fn tcl_cmd_lappend(current: i32, value: i32) i32 {
    const sc = obj_ensure_string(current);
    const sv = obj_ensure_string(value);
    return lappend_canonical(sc.ptr, sc.len, sv.ptr, sv.len);
}

/// Parse the existing list, re-quote each element canonically, then
/// append the new value.  Shared by :func:`tcl_cmd_lappend` and the
/// interpreter's multi-arg lappend loop.
fn lappend_canonical(sc_ptr: u32, sc_len: u32, sv_ptr: u32, sv_len: u32) i32 {
    const max_buf: u32 = sc_len * 2 + sv_len * 2 + 16;
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
        const quoter: *const fn (u32, u32, u32, u32) u32 = if (idx == 0)
            list_elem_quote
        else
            list_elem_quote_nth;
        if (elem.braced) {
            off = quoter(buf, off, sc_ptr + elem.start, elem.len);
        } else {
            const tmp = alloc(elem.len + 1);
            const actual_len = copy_unbraced_elem(tmp, sc_ptr + elem.start, elem.len);
            off = quoter(buf, off, tmp, actual_len);
        }
    }
    if (n > 0) {
        const d: [*]u8 = @ptrFromInt(buf + off);
        d[0] = ' ';
        off += 1;
        off = list_elem_quote_nth(buf, off, sv_ptr, sv_len);
    } else {
        off = list_elem_quote(buf, off, sv_ptr, sv_len);
    }
    return obj_new_string(@intCast(buf), @intCast(off));
}

// Exported: list — append one value to a list accumulator.  The
// Python codegen starts with an empty accumulator and chains this
// call per input element, so on entry ``a`` is either an empty
// string (first element) or an already-canonical list string.
//
// Fast path: when ``a`` is non-empty we preserve its bytes verbatim.
// Re-quoting would doubly-escape previously-canonicalised elements
// (e.g. an element ``a\{`` would turn into ``a\\\{`` on the second
// iteration).  Only the new element ``b`` is run through
// :func:`list_elem_quote_nth` (hash-quoting is disabled because it
// cannot be the first element).  When ``a`` is empty, ``b`` IS the
// first element and gets full :func:`list_elem_quote`.
pub export fn tcl_list(a: i32, b: i32) i32 {
    const sa = obj_ensure_string(a);
    const sb = obj_ensure_string(b);
    if (sa.len == 0) {
        const buf = alloc(sb.len * 2 + 4);
        const off = list_elem_quote(buf, 0, sb.ptr, sb.len);
        return obj_new_string(@intCast(buf), @intCast(off));
    }
    const buf = alloc(sa.len + sb.len * 2 + 8);
    memcpy(buf, sa.ptr, sa.len);
    var off: u32 = sa.len;
    const d: [*]u8 = @ptrFromInt(buf + off);
    d[0] = ' ';
    off += 1;
    off = list_elem_quote_nth(buf, off, sb.ptr, sb.len);
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

// Exported: list search — default Tcl ``lsearch`` semantics, which
// glob-match (``*``, ``?``, ``[...]``) the pattern against each list
// element and return the first matching index, or ``-1``.  Our
// previous implementation did exact matching only, so ``lsearch $x
// *5`` always returned ``-1`` even when elements matched the glob.
pub export fn tcl_cmd_list_search(list: i32, value: i32) i32 {
    const s = obj_ensure_string(list);
    const sv = obj_ensure_string(value);
    const n = list_count_elements(s.ptr, s.len);
    var idx: i64 = 0;
    while (idx < n) : (idx += 1) {
        const elem = list_element_at(s.ptr, s.len, idx);
        if (str.glob_match(sv.ptr, sv.len, s.ptr + elem.start, elem.len)) {
            return obj_new_int(idx);
        }
    }
    return obj_new_int(-1);
}

// Exported: list reverse — return a new list with elements in reverse order.
pub export fn tcl_cmd_list_reverse(list: i32) i32 {
    const s = obj_ensure_string(list);
    const n_i64 = list_count_elements(s.ptr, s.len);
    if (n_i64 <= 1) return list;
    const n: u32 = @intCast(n_i64);
    // Grab (ptr, len) for each element so we can walk backwards.
    const arr_buf = alloc(n * 8);
    var idx: u32 = 0;
    while (idx < n) : (idx += 1) {
        const elem = list_element_at(s.ptr, s.len, @intCast(idx));
        write_i32(arr_buf + idx * 8, @intCast(s.ptr + elem.start));
        write_i32(arr_buf + idx * 8 + 4, @intCast(elem.len));
    }
    const result_buf = alloc(s.len + n);
    var result_len: u32 = 0;
    var i: i32 = @as(i32, @intCast(n)) - 1;
    while (i >= 0) : (i -= 1) {
        if (result_len > 0) {
            const d: [*]u8 = @ptrFromInt(result_buf + result_len);
            d[0] = ' ';
            result_len += 1;
        }
        const iu: u32 = @intCast(i);
        const e_ptr: u32 = @intCast(read_i32(arr_buf + iu * 8));
        const e_len: u32 = @intCast(read_i32(arr_buf + iu * 8 + 4));
        memcpy(result_buf + result_len, e_ptr, e_len);
        result_len += e_len;
    }
    return obj_new_string(@intCast(result_buf), @intCast(result_len));
}

// Exported: list insert — ``linsert list index value1 ?value2 ...?``.
// Two-argument form: ``tcl_cmd_list_insert(list, index, value)`` —
// inserts *value* before position *index*, clamping negative indices
// to 0 and beyond-end indices to ``len``.  Multi-value ``linsert``
// calls into this per extra value; for now the common single-value
// case covers the test corpus.
pub export fn tcl_cmd_list_insert(list: i32, index: i32, value: i32) i32 {
    const s = obj_ensure_string(list);
    const sv = obj_ensure_string(value);
    const n_i64 = list_count_elements(s.ptr, s.len);
    const n: u32 = @intCast(n_i64);
    var pos: i64 = obj_get_int(index);
    if (pos < 0) pos = 0;
    if (pos > n_i64) pos = n_i64;
    const upos: u32 = @intCast(pos);

    const buf = alloc(s.len + sv.len * 2 + 4);
    var off: u32 = 0;
    var i: u32 = 0;
    while (i < n) : (i += 1) {
        if (i == upos) {
            if (off > 0) {
                const d: [*]u8 = @ptrFromInt(buf + off);
                d[0] = ' ';
                off += 1;
            }
            if (sv.len > 0) {
                memcpy(buf + off, sv.ptr, sv.len);
                off += sv.len;
            }
        }
        if (off > 0) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        const elem = list_element_at(s.ptr, s.len, @intCast(i));
        if (elem.len > 0) {
            memcpy(buf + off, s.ptr + elem.start, elem.len);
            off += elem.len;
        }
    }
    if (upos >= n) {
        if (off > 0 and sv.len > 0) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        if (sv.len > 0) {
            memcpy(buf + off, sv.ptr, sv.len);
            off += sv.len;
        }
    }
    return obj_new_string(@intCast(buf), @intCast(off));
}

// Exported: list replace — ``lreplace list first last ?value1 ...?``.
// Single-replacement form: ``tcl_cmd_list_replace(list, first, last,
// value)`` — replaces elements [first, last] with *value*.  If
// ``value == 0``, deletes the range.  Clamps first<0 to 0 and
// last>=n to n-1; when first>last after clamping the command becomes
// a pure insert before *first* (reference Tcl semantics).
pub export fn tcl_cmd_list_replace(list: i32, first: i32, last: i32, value: i32) i32 {
    const s = obj_ensure_string(list);
    const n_i64 = list_count_elements(s.ptr, s.len);
    const n: u32 = @intCast(n_i64);
    var f: i64 = obj_get_int(first);
    var l: i64 = obj_get_int(last);
    if (f < 0) f = 0;
    if (l >= n_i64) l = n_i64 - 1;
    if (f > n_i64) f = n_i64;

    const sv_len: u32 = if (value == 0) 0 else obj_ensure_string(value).len;
    const sv_ptr: u32 = if (value == 0 or sv_len == 0) 0 else obj_ensure_string(value).ptr;

    const buf = alloc(s.len + sv_len + 2);
    var off: u32 = 0;
    var i: u32 = 0;
    const uf: u32 = @intCast(if (f < 0) 0 else f);
    const ul: i64 = l;

    while (i < n) : (i += 1) {
        if (i == uf) {
            if (sv_len > 0) {
                if (off > 0) {
                    const d: [*]u8 = @ptrFromInt(buf + off);
                    d[0] = ' ';
                    off += 1;
                }
                memcpy(buf + off, sv_ptr, sv_len);
                off += sv_len;
            }
        }
        if (@as(i64, i) >= f and @as(i64, i) <= ul) {
            // skip — replaced
            continue;
        }
        if (off > 0) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        const elem = list_element_at(s.ptr, s.len, @intCast(i));
        if (elem.len > 0) {
            memcpy(buf + off, s.ptr + elem.start, elem.len);
            off += elem.len;
        }
    }
    // If first == n (append) and we never hit the insertion branch
    // above, drop the value at the end instead.
    if (uf >= n and sv_len > 0) {
        if (off > 0) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        memcpy(buf + off, sv_ptr, sv_len);
        off += sv_len;
    }
    return obj_new_string(@intCast(buf), @intCast(off));
}

// Exported: list repeat — ``lrepeat count value1 ?value2 ...?``.
//
// ``count`` is a non-negative integer.  With N value arguments the
// result is ``count * N`` elements, cycling through the values.  The
// runtime export here only supports the common one-value form
// (``lrepeat 3 foo`` → ``foo foo foo``); multi-value lrepeat falls
// through to the interpreter via the compiler's arg-count bridge.
pub export fn tcl_cmd_list_repeat(count: i32, value: i32) i32 {
    const cnt = obj_get_int(count);
    if (cnt <= 0) return obj_new_string(0, 0);
    const sv = obj_ensure_string(value);
    // Each element is the rendered string of *value*; join with spaces.
    // Quoting of embedded whitespace / braces is left to the caller —
    // matches the compiler's other list-building helpers.
    const per_elem: u32 = sv.len;
    const ucnt: u32 = @intCast(cnt);
    const total: u32 = if (ucnt == 0) 0 else per_elem * ucnt + (ucnt - 1);
    if (total == 0) return obj_new_string(0, 0);
    const buf = alloc(total);
    var off: u32 = 0;
    var i: u32 = 0;
    while (i < ucnt) : (i += 1) {
        if (i > 0) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        if (per_elem > 0) {
            memcpy(buf + off, sv.ptr, per_elem);
            off += per_elem;
        }
    }
    return obj_new_string(@intCast(buf), @intCast(total));
}
