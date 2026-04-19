// Dict operations: dict_create, dict_get, dict_set, dict_exists,
// dict_keys, dict_values, dict_size.

const obj = @import("tcl_obj.zig");
const alloc = obj.alloc;
const memcpy = obj.memcpy;
const obj_ensure_string = obj.obj_ensure_string;
const obj_new_string = obj.obj_new_string;
const obj_new_int = obj.obj_new_int;
const obj_new_string_copy = obj.obj_new_string_copy;
const str_cmp = obj.str_cmp;
const list_count_elements = obj.list_count_elements;
const list_element_at = obj.list_element_at;
const list_elem_quote = obj.list_elem_quote;
const list_elem_quote_nth = obj.list_elem_quote_nth;

// Exported: dict create — create an empty dict.
pub export fn dict_create() i32 {
    return obj_new_string(0, 0);
}

// Exported: dict get — look up a key in a dict, return its value.
pub export fn dict_get(dict: i32, key: i32) i32 {
    const sd = obj_ensure_string(dict);
    const sk = obj_ensure_string(key);
    const n = list_count_elements(sd.ptr, sd.len);
    var idx: i64 = 0;
    while (idx < n - 1) : (idx += 2) {
        const k = list_element_at(sd.ptr, sd.len, idx);
        if (str_cmp(sd.ptr + k.start, k.len, sk.ptr, sk.len) == 0) {
            const v = list_element_at(sd.ptr, sd.len, idx + 1);
            return obj_new_string_copy(sd.ptr + v.start, v.len);
        }
    }
    return obj_new_string(0, 0);
}

// Exported: dict set — set a key in a dict, return the updated dict.
pub export fn dict_set(dict: i32, key: i32, value: i32) i32 {
    const sd = obj_ensure_string(dict);
    const sk = obj_ensure_string(key);
    const sv = obj_ensure_string(value);
    const n = list_count_elements(sd.ptr, sd.len);
    var idx: i64 = 0;
    while (idx < n - 1) : (idx += 2) {
        const k = list_element_at(sd.ptr, sd.len, idx);
        if (str_cmp(sd.ptr + k.start, k.len, sk.ptr, sk.len) == 0) {
            return dict_rebuild_with_value(sd.ptr, sd.len, n, idx, sv.ptr, sv.len);
        }
    }
    return dict_append_pair(sd.ptr, sd.len, sk.ptr, sk.len, sv.ptr, sv.len);
}

fn dict_rebuild_with_value(sd_ptr: u32, sd_len: u32, n: i64, target_idx: i64, vp: u32, vl: u32) i32 {
    // Worst-case: each existing byte could double (backslash-escape) plus
    // braces, plus the new value doubled, plus separator spaces.
    const buf = alloc(sd_len * 2 + vl * 2 + 16);
    var off: u32 = 0;
    var idx: i64 = 0;
    while (idx < n) : (idx += 1) {
        if (off > 0) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        if (idx == target_idx + 1) {
            // A dict's value slots live at odd indices (1, 3, 5, …)
            // — they are never element 0 — so ``DONT_QUOTE_HASH`` is
            // always the right choice here.
            off = list_elem_quote_nth(buf, off, vp, vl);
        } else {
            const elem = list_element_at(sd_ptr, sd_len, idx);
            if (elem.braced) {
                const d: [*]u8 = @ptrFromInt(buf + off);
                d[0] = '{';
                off += 1;
                memcpy(buf + off, sd_ptr + elem.start, elem.len);
                off += elem.len;
                const d2: [*]u8 = @ptrFromInt(buf + off);
                d2[0] = '}';
                off += 1;
            } else {
                memcpy(buf + off, sd_ptr + elem.start, elem.len);
                off += elem.len;
            }
        }
    }
    return obj_new_string(@intCast(buf), @intCast(off));
}

fn dict_append_pair(sd_ptr: u32, sd_len: u32, kp: u32, kl: u32, vp: u32, vl: u32) i32 {
    const buf = alloc(sd_len + kl * 2 + vl * 2 + 16);
    var off: u32 = 0;
    if (sd_len > 0) {
        memcpy(buf, sd_ptr, sd_len);
        off = sd_len;
        const d: [*]u8 = @ptrFromInt(buf + off);
        d[0] = ' ';
        off += 1;
        // Key / value follow at least one prior element → hash-safe.
        off = list_elem_quote_nth(buf, off, kp, kl);
    } else {
        off = list_elem_quote(buf, off, kp, kl);
    }
    const d: [*]u8 = @ptrFromInt(buf + off);
    d[0] = ' ';
    off += 1;
    off = list_elem_quote_nth(buf, off, vp, vl);
    return obj_new_string(@intCast(buf), @intCast(off));
}

// Exported: dict merge — merge *source* into *target*; for duplicate
// keys the source value wins.  Variadic ``dict merge d1 d2 d3`` is
// implemented at the compiler level by chaining pair-merges.
pub export fn dict_merge_pair(target: i32, source: i32) i32 {
    const ss = obj_ensure_string(source);
    if (ss.len == 0) return target;
    const n = list_count_elements(ss.ptr, ss.len);
    if (n <= 0) return target;
    var current = target;
    var idx: i64 = 0;
    while (idx + 1 < n) : (idx += 2) {
        const k = list_element_at(ss.ptr, ss.len, idx);
        const v = list_element_at(ss.ptr, ss.len, idx + 1);
        current = dict_set_pair(current, ss.ptr + k.start, k.len, ss.ptr + v.start, v.len);
    }
    return current;
}

fn dict_set_pair(d: i32, kp: u32, kl: u32, vp: u32, vl: u32) i32 {
    const sd = obj_ensure_string(d);
    const n = list_count_elements(sd.ptr, sd.len);
    var idx: i64 = 0;
    while (idx + 1 < n) : (idx += 2) {
        const k = list_element_at(sd.ptr, sd.len, idx);
        if (str_cmp(sd.ptr + k.start, k.len, kp, kl) == 0) {
            return dict_rebuild_with_value(sd.ptr, sd.len, n, idx, vp, vl);
        }
    }
    return dict_append_pair(sd.ptr, sd.len, kp, kl, vp, vl);
}

// Exported: dict exists — check if key exists in dict, return 1 or 0.
pub export fn dict_exists(dict: i32, key: i32) i32 {
    const sd = obj_ensure_string(dict);
    const sk = obj_ensure_string(key);
    const n = list_count_elements(sd.ptr, sd.len);
    var idx: i64 = 0;
    while (idx < n - 1) : (idx += 2) {
        const k = list_element_at(sd.ptr, sd.len, idx);
        if (str_cmp(sd.ptr + k.start, k.len, sk.ptr, sk.len) == 0) {
            return obj_new_int(1);
        }
    }
    return obj_new_int(0);
}

// Exported: dict keys — return a list of all keys in the dict.
pub export fn dict_keys(dict: i32) i32 {
    const sd = obj_ensure_string(dict);
    const n = list_count_elements(sd.ptr, sd.len);
    if (n == 0) return obj_new_string(0, 0);
    const buf = alloc(sd.len);
    var off: u32 = 0;
    var idx: i64 = 0;
    while (idx < n) : (idx += 2) {
        if (off > 0) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        const elem = list_element_at(sd.ptr, sd.len, idx);
        if (elem.braced) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = '{';
            off += 1;
            memcpy(buf + off, sd.ptr + elem.start, elem.len);
            off += elem.len;
            const d2: [*]u8 = @ptrFromInt(buf + off);
            d2[0] = '}';
            off += 1;
        } else {
            memcpy(buf + off, sd.ptr + elem.start, elem.len);
            off += elem.len;
        }
    }
    return obj_new_string(@intCast(buf), @intCast(off));
}

// Exported: dict values — return a list of all values in the dict.
pub export fn dict_values(dict: i32) i32 {
    const sd = obj_ensure_string(dict);
    const n = list_count_elements(sd.ptr, sd.len);
    if (n == 0) return obj_new_string(0, 0);
    const buf = alloc(sd.len);
    var off: u32 = 0;
    var idx: i64 = 1;
    while (idx < n) : (idx += 2) {
        if (off > 0) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        const elem = list_element_at(sd.ptr, sd.len, idx);
        if (elem.braced) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = '{';
            off += 1;
            memcpy(buf + off, sd.ptr + elem.start, elem.len);
            off += elem.len;
            const d2: [*]u8 = @ptrFromInt(buf + off);
            d2[0] = '}';
            off += 1;
        } else {
            memcpy(buf + off, sd.ptr + elem.start, elem.len);
            off += elem.len;
        }
    }
    return obj_new_string(@intCast(buf), @intCast(off));
}

// Exported: dict size — number of key-value pairs.
pub export fn dict_size(dict: i32) i32 {
    const sd = obj_ensure_string(dict);
    const n = list_count_elements(sd.ptr, sd.len);
    return obj_new_int(@divTrunc(n, 2));
}
