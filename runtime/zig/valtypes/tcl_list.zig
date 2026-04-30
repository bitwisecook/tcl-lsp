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
//
// Validates brace balance and raises ``unmatched open brace in
// list`` on failure (S2.7).  Internal callers (linsert / lrepeat /
// lseq / list_index) call ``list_count_elements`` directly and
// stay permissive — only ``llength`` reports the error to user
// scripts.
pub export fn tcl_cmd_list_length(list: i32) i32 {
    const s = obj_ensure_string(list);
    if (!obj.list_validate_braces(s.ptr, s.len)) {
        const stubs = @import("../stubs/tcl_stubs.zig");
        stubs.raise("unmatched open brace in list");
        return 0;
    }
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
    // Fast path: when ``current`` is non-empty AND we own its byte
    // buffer with refcount == 1, append in place — preserves the
    // existing bytes verbatim (they are already canonical) and just
    // appends ``" " + quote(value)`` at the end.  Mirrors the
    // capacity-aware append in ``tcl_string.tcl_cmd_append``.
    //
    // Re-tokenising the whole list (the canonical-rebuild path) is
    // O(N) per call, which makes any ``for { ... { lappend L $i } }``
    // loop O(N²).  The in-place path is O(1) per call when the
    // buffer has room, O(N) only on grow.
    // ``is_immediate`` guard (PR #237 review): tagged immediates would
    // ``@intCast`` to a low integer and the rc/tag/cap reads would
    // dereference wasm data-segment bytes — junk values or a trap.
    if (sc.len > 0 and current != 0 and !obj.is_immediate(current)) {
        const addr: u32 = @intCast(current);
        const rc = obj.read_i32(addr + obj.OBJ_REFCOUNT);
        const tag = obj.read_i32(addr + obj.OBJ_TYPE_TAG);
        const cap: u32 = @bitCast(obj.read_i32(addr + obj.OBJ_STR_CAP));
        if (rc == 1 and tag == obj.TYPE_STRING and cap > 0) {
            // Worst case for the new tail: a single space + sv.len*2 + 2
            // (for surrounding braces if the value needs them).
            const tail_max: u32 = 1 + sv.len * 2 + 4;
            const want: u32 = sc.len + tail_max;
            var buf: u32 = sc.ptr;
            var new_cap: u32 = cap;
            if (cap < want) {
                new_cap = if (cap == 0) 16 else cap * 2;
                while (new_cap < want) new_cap *= 2;
                const nbuf = obj.alloc(new_cap);
                if (nbuf == 0) return current;
                memcpy(nbuf, sc.ptr, sc.len);
                obj.free_sized(sc.ptr, cap);
                buf = nbuf;
                obj.write_i32(addr + obj.OBJ_STR_PTR, @bitCast(nbuf));
                obj.write_i32(addr + obj.OBJ_STR_CAP, @bitCast(new_cap));
            }
            const d: [*]u8 = @ptrFromInt(buf + sc.len);
            d[0] = ' ';
            const off = list_elem_quote_nth(buf, sc.len + 1, sv.ptr, sv.len);
            obj.write_i32(addr + obj.OBJ_STR_LEN, @bitCast(off));
            return current;
        }
    }
    return lappend_canonical(sc.ptr, sc.len, sv.ptr, sv.len);
}

/// Parse the existing list, re-quote each element canonically, then
/// append the new value.  Shared by :func:`tcl_cmd_lappend` and the
/// interpreter's multi-arg lappend loop.
///
/// The returned TclObj records the actual size-class capacity in
/// ``OBJ_STR_CAP`` so the next ``lappend`` against this obj can
/// take the in-place fast path.
fn lappend_canonical(sc_ptr: u32, sc_len: u32, sv_ptr: u32, sv_len: u32) i32 {
    const max_buf: u32 = sc_len * 2 + sv_len * 2 + 16;
    const cap = obj.round_up_to_class((max_buf + 7) & ~@as(u32, 7));
    const buf = alloc(cap);
    if (buf == 0) return obj_new_string(0, 0);
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
    const new_obj = obj_new_string(@intCast(buf), @intCast(off));
    if (new_obj != 0) {
        obj.write_i32(@as(u32, @intCast(new_obj)) + obj.OBJ_STR_CAP, @bitCast(cap));
    }
    return new_obj;
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

// Copy one list element into *buf* at offset *off*, re-adding the
// surrounding braces if the element came from a braced source word
// (``{a b}``).  Without this, building a result list by raw
// ``memcpy`` of the element span flattens nested lists — e.g.
// ``lreverse {{a b} c}`` would emit ``c a b`` instead of
// ``c {a b}``, silently promoting sub-list content to siblings.
fn append_list_element(buf: u32, off_in: u32, sd_ptr: u32, elem: anytype) u32 {
    var off = off_in;
    if (elem.braced) {
        const d: [*]u8 = @ptrFromInt(buf + off);
        d[0] = '{';
        off += 1;
        if (elem.len > 0) {
            memcpy(buf + off, sd_ptr + elem.start, elem.len);
            off += elem.len;
        }
        const d2: [*]u8 = @ptrFromInt(buf + off);
        d2[0] = '}';
        off += 1;
    } else if (elem.len > 0) {
        memcpy(buf + off, sd_ptr + elem.start, elem.len);
        off += elem.len;
    }
    return off;
}

// Parse an index that may be "end", "end-N", "end+N", or a plain
// integer.  Returns the resolved 0-based index (may be negative for
// under-range or >= n for over-range; callers clamp as they see fit).
// Exported as ``pub`` so string-index helpers in ``tcl_string.zig``
// reuse the same "end" arithmetic.
pub fn resolve_list_index(idx: i32, n: i64) i64 {
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

// Exported: ``expr {x in list}`` / ``ni`` support — return 1 when
// *value* is string-equal to any element of *list*, 0 otherwise.
// Separate from ``tcl_cmd_list_search`` because ``in`` and ``ni`` use
// exact string match, not glob matching.
pub export fn tcl_cmd_list_contains(list: i32, value: i32) i32 {
    const s = obj_ensure_string(list);
    const sv = obj_ensure_string(value);
    const n = list_count_elements(s.ptr, s.len);
    var idx: i64 = 0;
    while (idx < n) : (idx += 1) {
        const elem = list_element_at(s.ptr, s.len, idx);
        if (str_cmp(s.ptr + elem.start, elem.len, sv.ptr, sv.len) == 0) {
            return obj_new_int(1);
        }
    }
    return obj_new_int(0);
}

// Exported: list reverse — return a new list with elements in reverse order.
pub export fn tcl_cmd_list_reverse(list: i32) i32 {
    const s = obj_ensure_string(list);
    const n_i64 = list_count_elements(s.ptr, s.len);
    if (n_i64 <= 1) return list;
    const n: u32 = @intCast(n_i64);
    // Over-allocate to fit braces around every element (2 extra bytes
    // each) in the worst case; the bump allocator makes over-alloc
    // free.
    const result_buf = alloc(s.len + n * 3 + 4);
    var result_len: u32 = 0;
    var i: i32 = @as(i32, @intCast(n)) - 1;
    while (i >= 0) : (i -= 1) {
        if (result_len > 0) {
            const d: [*]u8 = @ptrFromInt(result_buf + result_len);
            d[0] = ' ';
            result_len += 1;
        }
        const elem = list_element_at(s.ptr, s.len, @intCast(i));
        // ``append_list_element`` re-adds ``{...}`` when the source
        // word was braced so nested lists (e.g. ``{a b}`` inside
        // ``{{a b} c}``) keep their grouping instead of flattening
        // into siblings.
        result_len = append_list_element(result_buf, result_len, s.ptr, elem);
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
    // ``linsert`` uses a different ``end`` semantic from ``lindex`` /
    // ``string index`` / ``lrange``: the ``end`` marker denotes the
    // position *after* the last element (n), not n-1.  So
    // ``linsert L end x`` appends x rather than inserting before the
    // last element.  Resolve as if the list had n+1 positions so
    // ``end`` → n, ``end-1`` → n-1, ``end-N`` → n-N, matching the
    // Tcl ``linsert`` manual page.
    var pos: i64 = resolve_list_index(index, n_i64 + 1);
    if (pos < 0) pos = 0;
    if (pos > n_i64) pos = n_i64;
    const upos: u32 = @intCast(pos);

    // Over-allocate to fit worst-case brace-wrapping on every
    // existing element plus the inserted value.  Inserted-value
    // worst-case is ``2 * len + 2`` because list_elem_quote_*
    // may brace-wrap.
    const buf = alloc(s.len + n * 3 + sv.len * 2 + 8);
    var off: u32 = 0;
    var i: u32 = 0;
    while (i < n) : (i += 1) {
        if (i == upos) {
            if (off > 0) {
                const d: [*]u8 = @ptrFromInt(buf + off);
                d[0] = ' ';
                off += 1;
            }
            // Quote the inserted value canonically so values
            // containing whitespace, braces, or backslashes still
            // form a single list element instead of splitting into
            // multiple elements when the result is reparsed.
            const quoter: *const fn (u32, u32, u32, u32) u32 = if (off == 0) &list_elem_quote else &list_elem_quote_nth;
            off = quoter(buf, off, sv.ptr, sv.len);
        }
        if (off > 0) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        const elem = list_element_at(s.ptr, s.len, @intCast(i));
        // Preserve braces so nested lists keep their grouping: without
        // this ``linsert {{a b} c} 1 X`` flattens to ``a b X c``
        // instead of ``{a b} X c``.
        off = append_list_element(buf, off, s.ptr, elem);
    }
    if (upos >= n) {
        if (off > 0) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        const quoter: *const fn (u32, u32, u32, u32) u32 = if (off == 0) &list_elem_quote else &list_elem_quote_nth;
        off = quoter(buf, off, sv.ptr, sv.len);
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
    // Accept ``end`` / ``end-N`` / ``end+N`` for both bounds.  Bare
    // ``obj_get_int`` would treat ``end`` as 0 and replace the wrong
    // slice.
    var f: i64 = resolve_list_index(first, n_i64);
    var l: i64 = resolve_list_index(last, n_i64);
    if (f < 0) f = 0;
    if (l >= n_i64) l = n_i64 - 1;
    if (f > n_i64) f = n_i64;

    const sv_len: u32 = if (value == 0) 0 else obj_ensure_string(value).len;
    const sv_ptr: u32 = if (value == 0 or sv_len == 0) 0 else obj_ensure_string(value).ptr;

    // Over-allocate to fit worst-case brace-wrapping on every
    // preserved element plus the canonically-quoted value.
    const buf = alloc(s.len + n * 3 + sv_len * 2 + 8);
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
                // Quote canonically — see linsert above.
                const quoter: *const fn (u32, u32, u32, u32) u32 = if (off == 0) &list_elem_quote else &list_elem_quote_nth;
                off = quoter(buf, off, sv_ptr, sv_len);
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
        // Preserve source braces so nested lists keep their
        // grouping through ``lreplace``.
        off = append_list_element(buf, off, s.ptr, elem);
    }
    // If first == n (append) and we never hit the insertion branch
    // above, drop the value at the end instead.
    if (uf >= n and sv_len > 0) {
        if (off > 0) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        const quoter: *const fn (u32, u32, u32, u32) u32 = if (off == 0) &list_elem_quote else &list_elem_quote_nth;
        off = quoter(buf, off, sv_ptr, sv_len);
    }
    return obj_new_string(@intCast(buf), @intCast(off));
}

// Exported: list set — ``lset varName ?index ...? newValue``.
//
// Takes a source list, an *indices* TclObj whose string form is a
// list of index words (a single integer / ``end[-N]`` / ``end+N``,
// or multiple space-separated index words for nested access), and
// a new *value*.  Returns a freshly-rendered list with the element
// at the nested index replaced.  Does not mutate the source.
//
// The emitter always passes *indices* as a TclObj list; a call of
// the shape ``lset v i newval`` uses a single-element indices list,
// ``lset v {i j} newval`` passes the braced list verbatim, and
// ``lset v i j k newval`` builds a three-element indices list
// via ``tcl_list``.  Unifying the shapes at runtime keeps the
// caller simple and matches the C Tcl ``Tcl_LsetFlat`` semantics.
//
// On out-of-range index or malformed indices list, returns a copy
// of the source unchanged.  This function does not itself stamp
// catch / error state; a caller that needs observable failure
// (the ``lset`` Tcl command normally raises an error for
// out-of-range indices) must wire that separately.  Deferred
// until the broader error-context work lands — the current
// corpus does not distinguish the two.
pub export fn tcl_cmd_list_set(list: i32, indices: i32, value: i32) i32 {
    const s_list = obj_ensure_string(list);
    const s_indices = obj_ensure_string(indices);

    const n_indices = list_count_elements(s_indices.ptr, s_indices.len);
    if (n_indices == 0) {
        // ``lset v newval`` — the indices list is empty, so the
        // new value replaces the whole variable's value.  The
        // compiler-side ``_emit_cmd_lset`` handles the zero-index
        // arity shape directly (simple ``set``) so this branch is
        // a safety net.
        const sv = obj_ensure_string(value);
        return obj_new_string_copy(sv.ptr, sv.len);
    }

    return lset_recurse(s_list.ptr, s_list.len, s_indices.ptr, s_indices.len, 0, n_indices, value);
}

/// Rebuild a list with its ``depth``-th nested element replaced by
/// the recursive result of setting at the remaining indices.  At
/// the deepest level (``depth == n_indices - 1``), the element is
/// replaced by the raw *value* itself; at shallower levels we
/// recurse into the element (treated as a sublist).
fn lset_recurse(
    src_ptr: u32,
    src_len: u32,
    indices_ptr: u32,
    indices_len: u32,
    depth: i64,
    n_indices: i64,
    value: i32,
) i32 {
    const n_src_i64 = list_count_elements(src_ptr, src_len);
    const n_src: u32 = @intCast(n_src_i64);

    // Resolve this depth's index word against the source list's length.
    const idx_elem = list_element_at(indices_ptr, indices_len, depth);
    // ``list_element_at`` returns a slice whose ``start``/``len`` are
    // relative to the *indices* text.  Build a TclObj wrapping the
    // single index word so ``resolve_list_index`` (which takes a
    // TclObj) can parse ``end[-N]`` / ``end+N`` / integer forms.
    const idx_obj = obj_new_string_copy(indices_ptr + idx_elem.start, idx_elem.len);
    const idx = resolve_list_index(idx_obj, n_src_i64);
    if (idx < 0 or idx >= n_src_i64) {
        // Out of range — return a copy of the source unchanged.
        // The compiler-level caller is expected to treat the result
        // as a no-op on error; a future change can thread an error
        // message through ``tcl_cmd_error`` for closer Tcl parity.
        return obj_new_string_copy(src_ptr, src_len);
    }
    const uidx: u32 = @intCast(idx);

    // Determine the replacement for the element at this depth:
    //   - At the deepest level, the replacement is *value* itself.
    //   - Otherwise, recurse into the element treated as a sublist.
    var replacement: i32 = value;
    if (depth + 1 < n_indices) {
        const elem = list_element_at(src_ptr, src_len, idx);
        replacement = lset_recurse(
            src_ptr + elem.start,
            elem.len,
            indices_ptr,
            indices_len,
            depth + 1,
            n_indices,
            value,
        );
    }
    const s_rep = obj_ensure_string(replacement);

    // Build the result list by copying elements, substituting the
    // replacement at ``uidx``.  Over-allocate to fit worst-case
    // brace-wrapping on every element.
    const buf = alloc(src_len + n_src * 3 + s_rep.len * 2 + 8);
    var off: u32 = 0;
    var i: u32 = 0;
    while (i < n_src) : (i += 1) {
        if (off > 0) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        if (i == uidx) {
            // Quote the replacement canonically so values containing
            // whitespace, braces, or backslashes form a single list
            // element when the result is reparsed.
            const quoter: *const fn (u32, u32, u32, u32) u32 =
                if (off == 0) &list_elem_quote else &list_elem_quote_nth;
            off = quoter(buf, off, s_rep.ptr, s_rep.len);
        } else {
            const elem = list_element_at(src_ptr, src_len, @intCast(i));
            off = append_list_element(buf, off, src_ptr, elem);
        }
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
    const ucnt: u32 = @intCast(cnt);

    // Quote *value* canonically once for the first-element form and
    // once for the nth-element form (the only difference is leading-
    // ``#`` handling per ``UpdateStringOfList``).  Values containing
    // whitespace, braces or backslashes must be brace-wrapped so the
    // result parses back as ``count`` elements rather than splitting
    // on the literal whitespace.  Worst-case quoted length is
    // ``2 * sv.len + 2``.
    const first_buf = alloc(sv.len * 2 + 4);
    const first_len = list_elem_quote(first_buf, 0, sv.ptr, sv.len);
    const next_buf = alloc(sv.len * 2 + 4);
    const next_len = list_elem_quote_nth(next_buf, 0, sv.ptr, sv.len);

    const sep_count: u32 = if (ucnt == 0) 0 else ucnt - 1;
    const total: u32 = first_len + sep_count * (1 + next_len);
    if (total == 0) return obj_new_string(0, 0);
    const buf = alloc(total);
    var off: u32 = 0;
    if (first_len > 0) {
        memcpy(buf + off, first_buf, first_len);
        off += first_len;
    }
    var i: u32 = 1;
    while (i < ucnt) : (i += 1) {
        const sep: [*]u8 = @ptrFromInt(buf + off);
        sep[0] = ' ';
        off += 1;
        if (next_len > 0) {
            memcpy(buf + off, next_buf, next_len);
            off += next_len;
        }
    }
    return obj_new_string(@intCast(buf), @intCast(off));
}
