// String operations: append, string_compare, string_equal, string_match,
// string_trim, string_first, string_last, string_repeat, string_reverse,
// string_toupper, string_tolower, string_replace, string_length, string_index,
// string_range, string_map, string_is_integer, string_is_alpha, string_is_digit,
// string_is_space, string_trimleft, string_trimright, concat.

const obj = @import("tcl_obj.zig");
const alloc = obj.alloc;
const memcpy = obj.memcpy;
const obj_ensure_string = obj.obj_ensure_string;
const obj_new_string = obj.obj_new_string;
const obj_new_int = obj.obj_new_int;
const obj_get_int = obj.obj_get_int;
const obj_new_string_copy = obj.obj_new_string_copy;
const try_parse_int = obj.try_parse_int;
const is_space = obj.is_space;
const list_count_elements = obj.list_count_elements;
const list_element_at = obj.list_element_at;

// Exported: append — concatenate two TclObj string representations.
pub export fn append(current: i32, addition: i32) i32 {
    const a = obj_ensure_string(current);
    const b = obj_ensure_string(addition);
    const total = a.len + b.len;
    if (total == 0) return obj_new_string(0, 0);
    const buf = alloc(total);
    if (a.len > 0) memcpy(buf, a.ptr, a.len);
    if (b.len > 0) memcpy(buf + a.len, b.ptr, b.len);
    return obj_new_string(@intCast(buf), @intCast(total));
}

// Exported: string compare — lexicographic comparison of string representations.
pub export fn string_compare(a: i32, b: i32) i32 {
    const sa = obj_ensure_string(a);
    const sb = obj_ensure_string(b);
    const min_len = if (sa.len < sb.len) sa.len else sb.len;
    const pa: [*]const u8 = @ptrFromInt(sa.ptr);
    const pb: [*]const u8 = @ptrFromInt(sb.ptr);
    for (0..min_len) |i| {
        if (pa[i] < pb[i]) return obj_new_int(-1);
        if (pa[i] > pb[i]) return obj_new_int(1);
    }
    if (sa.len < sb.len) return obj_new_int(-1);
    if (sa.len > sb.len) return obj_new_int(1);
    return obj_new_int(0);
}

// Exported: string length — byte length of the string representation.
pub export fn string_length(value: i32) i32 {
    const s = obj_ensure_string(value);
    return obj_new_int(@intCast(s.len));
}

// Exported: string index — extract the character at a byte index.
pub export fn string_index(value: i32, idx: i32) i32 {
    const s = obj_ensure_string(value);
    const i_val = obj_get_int(idx);
    if (i_val < 0 or i_val >= @as(i64, s.len)) return obj_new_string(0, 0);
    const pos: u32 = @intCast(i_val);
    const src: [*]const u8 = @ptrFromInt(s.ptr);
    const buf = alloc(1);
    const dst: [*]u8 = @ptrFromInt(buf);
    dst[0] = src[pos];
    return obj_new_string(@intCast(buf), 1);
}

// Exported: string range — extract a substring [first..last] (inclusive).
pub export fn string_range(value: i32, first: i32, last: i32) i32 {
    const s = obj_ensure_string(value);
    var f = obj_get_int(first);
    var l = obj_get_int(last);
    const slen: i64 = @intCast(s.len);
    if (f < 0) f = 0;
    if (l >= slen) l = slen - 1;
    if (f > l or f >= slen) return obj_new_string(0, 0);
    const start: u32 = @intCast(f);
    const count: u32 = @intCast(l - f + 1);
    return obj_new_string_copy(s.ptr + start, count);
}

// Exported: string map — apply a mapping list {from to from to ...} to a string.
pub export fn string_map(mapping: i32, value: i32) i32 {
    const sm = obj_ensure_string(mapping);
    const sv = obj_ensure_string(value);
    if (sv.len == 0) return value;
    const n_elems = list_count_elements(sm.ptr, sm.len);
    if (n_elems < 2) return value;
    const n_pairs: u32 = @intCast(@divTrunc(n_elems, 2));
    const buf = alloc(sv.len * 2 + 64);
    const src: [*]const u8 = @ptrFromInt(sv.ptr);
    var out_len: u32 = 0;
    var pos: u32 = 0;
    outer: while (pos < sv.len) {
        var pair: u32 = 0;
        while (pair < n_pairs) : (pair += 1) {
            const from = list_element_at(sm.ptr, sm.len, @as(i64, pair) * 2);
            if (from.len == 0) {
                pair += 1;
                continue;
            }
            if (pos + from.len <= sv.len) {
                const fp: [*]const u8 = @ptrFromInt(sm.ptr + from.start);
                var match = true;
                for (0..from.len) |k| {
                    if (src[pos + k] != fp[k]) {
                        match = false;
                        break;
                    }
                }
                if (match) {
                    const to = list_element_at(sm.ptr, sm.len, @as(i64, pair) * 2 + 1);
                    if (to.len > 0) {
                        memcpy(buf + out_len, sm.ptr + to.start, to.len);
                        out_len += to.len;
                    }
                    pos += from.len;
                    continue :outer;
                }
            }
        }
        const dst: [*]u8 = @ptrFromInt(buf + out_len);
        dst[0] = src[pos];
        out_len += 1;
        pos += 1;
    }
    return obj_new_string(@intCast(buf), @intCast(out_len));
}

// Exported: string match — glob pattern matching (* and ? wildcards).
pub export fn string_match(pattern: i32, value: i32) i32 {
    const sp = obj_ensure_string(pattern);
    const sv = obj_ensure_string(value);
    const matched = glob_match(sp.ptr, sp.len, sv.ptr, sv.len);
    return obj_new_int(if (matched) @as(i64, 1) else @as(i64, 0));
}

fn glob_match(pp: u32, plen: u32, vp: u32, vlen: u32) bool {
    const pat: [*]const u8 = @ptrFromInt(pp);
    const val: [*]const u8 = @ptrFromInt(vp);
    var pi: u32 = 0;
    var vi: u32 = 0;
    var star_pi: u32 = plen;
    var star_vi: u32 = 0;
    while (vi < vlen or pi < plen) {
        if (pi < plen and pat[pi] == '*') {
            star_pi = pi;
            star_vi = vi;
            pi += 1;
        } else if (pi < plen and vi < vlen and (pat[pi] == '?' or pat[pi] == val[vi])) {
            pi += 1;
            vi += 1;
        } else if (star_pi < plen) {
            pi = star_pi + 1;
            star_vi += 1;
            vi = star_vi;
        } else {
            return false;
        }
    }
    return true;
}

// Exported: string trim — strip leading/trailing whitespace.
pub export fn string_trim(value: i32) i32 {
    const s = obj_ensure_string(value);
    if (s.len == 0) return value;
    const src: [*]const u8 = @ptrFromInt(s.ptr);
    var start: u32 = 0;
    while (start < s.len and is_space(src[start])) start += 1;
    var end: u32 = s.len;
    while (end > start and is_space(src[end - 1])) end -= 1;
    if (start == 0 and end == s.len) return value;
    return obj_new_string_copy(s.ptr + start, end - start);
}

// Exported: string trimleft — strip leading whitespace.
pub export fn string_trimleft(value: i32) i32 {
    const s = obj_ensure_string(value);
    if (s.len == 0) return value;
    const src: [*]const u8 = @ptrFromInt(s.ptr);
    var start: u32 = 0;
    while (start < s.len and is_space(src[start])) start += 1;
    if (start == 0) return value;
    return obj_new_string_copy(s.ptr + start, s.len - start);
}

// Exported: string trimright — strip trailing whitespace.
pub export fn string_trimright(value: i32) i32 {
    const s = obj_ensure_string(value);
    if (s.len == 0) return value;
    const src: [*]const u8 = @ptrFromInt(s.ptr);
    var end: u32 = s.len;
    while (end > 0 and is_space(src[end - 1])) end -= 1;
    if (end == s.len) return value;
    return obj_new_string_copy(s.ptr, end);
}

// Exported: string equal — compare two strings for equality (returns 1 or 0).
pub export fn string_equal(a: i32, b: i32) i32 {
    const sa = obj_ensure_string(a);
    const sb = obj_ensure_string(b);
    if (sa.len != sb.len) return obj_new_int(0);
    if (sa.len == 0) return obj_new_int(1);
    const pa: [*]const u8 = @ptrFromInt(sa.ptr);
    const pb: [*]const u8 = @ptrFromInt(sb.ptr);
    for (0..sa.len) |i| {
        if (pa[i] != pb[i]) return obj_new_int(0);
    }
    return obj_new_int(1);
}

// Exported: string first — find first occurrence of needle in haystack.
pub export fn string_first(needle: i32, haystack: i32) i32 {
    const sn = obj_ensure_string(needle);
    const sh = obj_ensure_string(haystack);
    if (sn.len == 0 or sn.len > sh.len) return obj_new_int(-1);
    const np: [*]const u8 = @ptrFromInt(sn.ptr);
    const hp: [*]const u8 = @ptrFromInt(sh.ptr);
    const limit = sh.len - sn.len + 1;
    var i: u32 = 0;
    while (i < limit) : (i += 1) {
        var match = true;
        for (0..sn.len) |k| {
            if (hp[i + k] != np[k]) {
                match = false;
                break;
            }
        }
        if (match) return obj_new_int(@intCast(i));
    }
    return obj_new_int(-1);
}

// Exported: string last — find last occurrence of needle in haystack.
pub export fn string_last(needle: i32, haystack: i32) i32 {
    const sn = obj_ensure_string(needle);
    const sh = obj_ensure_string(haystack);
    if (sn.len == 0 or sn.len > sh.len) return obj_new_int(-1);
    const np: [*]const u8 = @ptrFromInt(sn.ptr);
    const hp: [*]const u8 = @ptrFromInt(sh.ptr);
    var i: i32 = @as(i32, @intCast(sh.len - sn.len));
    while (i >= 0) : (i -= 1) {
        const ui: u32 = @intCast(i);
        var match = true;
        for (0..sn.len) |k| {
            if (hp[ui + k] != np[k]) {
                match = false;
                break;
            }
        }
        if (match) return obj_new_int(@intCast(ui));
    }
    return obj_new_int(-1);
}

// Exported: string repeat — repeat a string N times.
pub export fn string_repeat(value: i32, count: i32) i32 {
    const sv = obj_ensure_string(value);
    const n = obj_get_int(count);
    if (n <= 0 or sv.len == 0) return obj_new_string(0, 0);
    const cn: u32 = @intCast(n);
    const total = sv.len * cn;
    const buf = alloc(total);
    var off: u32 = 0;
    var i: u32 = 0;
    while (i < cn) : (i += 1) {
        memcpy(buf + off, sv.ptr, sv.len);
        off += sv.len;
    }
    return obj_new_string(@intCast(buf), @intCast(total));
}

// Exported: string reverse — reverse a string.
pub export fn string_reverse(value: i32) i32 {
    const sv = obj_ensure_string(value);
    if (sv.len <= 1) return value;
    const buf = alloc(sv.len);
    const src: [*]const u8 = @ptrFromInt(sv.ptr);
    const dst: [*]u8 = @ptrFromInt(buf);
    var i: u32 = 0;
    while (i < sv.len) : (i += 1) {
        dst[i] = src[sv.len - 1 - i];
    }
    return obj_new_string(@intCast(buf), @intCast(sv.len));
}

// Exported: string toupper — convert to uppercase.
pub export fn string_toupper(value: i32) i32 {
    const sv = obj_ensure_string(value);
    if (sv.len == 0) return value;
    const buf = alloc(sv.len);
    const src: [*]const u8 = @ptrFromInt(sv.ptr);
    const dst: [*]u8 = @ptrFromInt(buf);
    for (0..sv.len) |i| {
        dst[i] = if (src[i] >= 'a' and src[i] <= 'z') src[i] - 32 else src[i];
    }
    return obj_new_string(@intCast(buf), @intCast(sv.len));
}

// Exported: string tolower — convert to lowercase.
pub export fn string_tolower(value: i32) i32 {
    const sv = obj_ensure_string(value);
    if (sv.len == 0) return value;
    const buf = alloc(sv.len);
    const src: [*]const u8 = @ptrFromInt(sv.ptr);
    const dst: [*]u8 = @ptrFromInt(buf);
    for (0..sv.len) |i| {
        dst[i] = if (src[i] >= 'A' and src[i] <= 'Z') src[i] + 32 else src[i];
    }
    return obj_new_string(@intCast(buf), @intCast(sv.len));
}

// Exported: string replace — replace characters in range [first..last] with new string.
pub export fn string_replace(value: i32, first: i32, last: i32, new_str: i32) i32 {
    const sv = obj_ensure_string(value);
    var f = obj_get_int(first);
    var l = obj_get_int(last);
    const slen: i64 = @intCast(sv.len);
    if (f < 0) f = 0;
    if (l >= slen) l = slen - 1;
    if (f > l or f >= slen) return value;
    const sn = obj_ensure_string(new_str);
    const fst: u32 = @intCast(f);
    const lst: u32 = @intCast(l);
    const tail_start = lst + 1;
    const tail_len = sv.len - tail_start;
    const total = fst + sn.len + tail_len;
    const buf = alloc(total);
    if (fst > 0) memcpy(buf, sv.ptr, fst);
    if (sn.len > 0) memcpy(buf + fst, sn.ptr, sn.len);
    if (tail_len > 0) memcpy(buf + fst + sn.len, sv.ptr + tail_start, tail_len);
    return obj_new_string(@intCast(buf), @intCast(total));
}

// Exported: string is integer — check if a string is a valid integer.
pub export fn string_is_integer(value: i32) i32 {
    const sv = obj_ensure_string(value);
    if (sv.len == 0) return obj_new_int(0);
    if (try_parse_int(sv.ptr, sv.len) != null) {
        return obj_new_int(1);
    }
    return obj_new_int(0);
}

// Exported: string is alpha — check if a string contains only letters.
pub export fn string_is_alpha(value: i32) i32 {
    const sv = obj_ensure_string(value);
    if (sv.len == 0) return obj_new_int(0);
    const src: [*]const u8 = @ptrFromInt(sv.ptr);
    for (0..sv.len) |i| {
        if (!((src[i] >= 'a' and src[i] <= 'z') or (src[i] >= 'A' and src[i] <= 'Z'))) {
            return obj_new_int(0);
        }
    }
    return obj_new_int(1);
}

// Exported: string is digit — check if a string contains only digits.
pub export fn string_is_digit(value: i32) i32 {
    const sv = obj_ensure_string(value);
    if (sv.len == 0) return obj_new_int(0);
    const src: [*]const u8 = @ptrFromInt(sv.ptr);
    for (0..sv.len) |i| {
        if (src[i] < '0' or src[i] > '9') return obj_new_int(0);
    }
    return obj_new_int(1);
}

// Exported: string is space — check if a string contains only whitespace.
pub export fn string_is_space(value: i32) i32 {
    const sv = obj_ensure_string(value);
    if (sv.len == 0) return obj_new_int(0);
    const src: [*]const u8 = @ptrFromInt(sv.ptr);
    for (0..sv.len) |i| {
        if (!is_space(src[i])) return obj_new_int(0);
    }
    return obj_new_int(1);
}

// Exported: concat — concatenate two TclObj string representations with space.
pub export fn concat(a: i32, b: i32) i32 {
    const sa = obj_ensure_string(a);
    const sb = obj_ensure_string(b);
    if (sa.len == 0) return b;
    if (sb.len == 0) return a;
    const total = sa.len + 1 + sb.len;
    const buf = alloc(total);
    memcpy(buf, sa.ptr, sa.len);
    const dst: [*]u8 = @ptrFromInt(buf + sa.len);
    dst[0] = ' ';
    memcpy(buf + sa.len + 1, sb.ptr, sb.len);
    return obj_new_string(@intCast(buf), @intCast(total));
}
