// String operations: append, string_compare, string_equal, string_match,
// string_trim, string_first, string_last, string_repeat, string_reverse,
// string_toupper, string_tolower, string_replace, string_length, string_index,
// string_range, string_map, string_is_integer, string_is_alpha, string_is_digit,
// string_is_space, string_trimleft, string_trimright, concat.

const obj = @import("tcl_obj.zig");
const bignum = @import("tcl_bignum.zig");
const list_mod = @import("tcl_list.zig");
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
//
// Capacity-aware in-place growth path:
//   - When the current TclObj has refcount == 1 and owns its byte
//     buffer (OBJ_STR_CAP > 0), append in place.  If the existing
//     capacity holds the new total length, the cost is just a
//     memcpy of the addition; otherwise reallocate to the next
//     size class (geometric, doubling) and copy.
//   - When refcount > 1 (shared object) or the buffer is interned
//     (cap == 0, points into a literal data segment), fall through
//     to the today's allocate-new-buffer path so the existing
//     references stay valid.
pub export fn tcl_cmd_append(current: i32, addition: i32) i32 {
    if (current == 0) {
        // First append into a null sentinel — make a brand-new owned
        // buffer so subsequent appends benefit from in-place growth.
        const b = obj_ensure_string(addition);
        if (b.len == 0) return obj_new_string(0, 0);
        return obj.obj_new_string_copy(@bitCast(b.ptr), @bitCast(b.len));
    }
    const a = obj_ensure_string(current);
    const b = obj_ensure_string(addition);
    const total: u32 = a.len + b.len;
    if (total == 0) return obj_new_string(0, 0);

    // ``is_immediate`` guard (PR #237 review): with S6.4, every small
    // integer is a tagged handle whose ``@intCast`` value is a low
    // integer — rc/tag/cap reads would dereference wasm data-segment
    // bytes.  Skip the in-place paths on immediates and fall through
    // to the new-buffer fallback below.
    if (!obj.is_immediate(current)) {
        const addr: u32 = @intCast(current);
        const rc = obj.read_i32(addr + obj.OBJ_REFCOUNT);
        const tag = obj.read_i32(addr + obj.OBJ_TYPE_TAG);
        const cap: u32 = @bitCast(obj.read_i32(addr + obj.OBJ_STR_CAP));

        // In-place fast path: we own the buffer and have spare capacity.
        if (rc == 1 and tag == obj.TYPE_STRING and cap >= total and cap > 0) {
            if (b.len > 0) memcpy(a.ptr + a.len, b.ptr, b.len);
            obj.write_i32(addr + obj.OBJ_STR_LEN, @bitCast(total));
            return current;
        }

        // In-place grow path: we own the buffer but need more capacity.
        // Pick the smallest size class >= total, doubling past the
        // current cap to amortise grow cost across a long append loop.
        if (rc == 1 and tag == obj.TYPE_STRING and cap > 0) {
            var new_cap_g: u32 = if (cap == 0) 16 else cap * 2;
            while (new_cap_g < total) new_cap_g *= 2;
            const new_buf = alloc(new_cap_g);
            if (new_buf == 0) return current; // OOM — alloc raised the flag
            if (a.len > 0) memcpy(new_buf, a.ptr, a.len);
            if (b.len > 0) memcpy(new_buf + a.len, b.ptr, b.len);
            // Recycle the old buffer.
            obj.free_sized(a.ptr, cap);
            obj.write_i32(addr + obj.OBJ_STR_PTR, @bitCast(new_buf));
            obj.write_i32(addr + obj.OBJ_STR_LEN, @bitCast(total));
            obj.write_i32(addr + obj.OBJ_STR_CAP, @bitCast(new_cap_g));
            return current;
        }
    }

    // Fallback: shared (rc > 1) or non-owning (cap == 0, literal /
    // interned).  Allocate a new owned buffer, copy both halves,
    // wrap in a new TclObj with cap set so future appends through
    // the new TclObj benefit from in-place growth.
    var new_cap: u32 = total;
    new_cap = (new_cap + 7) & ~@as(u32, 7);
    new_cap = obj.round_up_to_class(new_cap);
    const buf = alloc(new_cap);
    if (buf == 0) return obj_new_string(0, 0);
    if (a.len > 0) memcpy(buf, a.ptr, a.len);
    if (b.len > 0) memcpy(buf + a.len, b.ptr, b.len);
    const new_obj = obj_new_string(@bitCast(buf), @bitCast(total));
    if (new_obj != 0) {
        obj.write_i32(@as(u32, @bitCast(new_obj)) + obj.OBJ_STR_CAP, @bitCast(new_cap));
    }
    return new_obj;
}

// Exported: string compare — lexicographic comparison of string representations.
pub export fn string_compare(a: i32, b: i32) i32 {
    return string_compare_full(a, b, 0, -1);
}

/// ``string compare ?-nocase? ?-length N? a b`` — full-arity helper.
/// ``nocase != 0`` switches to case-insensitive ASCII comparison;
/// ``len_limit < 0`` means no length cap, ``len_limit >= 0`` clamps
/// the comparison to the first ``len_limit`` bytes.  Returns a
/// TclObj wrapping ``-1`` / ``0`` / ``1``.
pub export fn string_compare_full(a: i32, b: i32, nocase: i32, len_limit: i32) i32 {
    const sa = obj_ensure_string(a);
    const sb = obj_ensure_string(b);
    var ea: u32 = sa.len;
    var eb: u32 = sb.len;
    if (len_limit >= 0) {
        const lim: u32 = @intCast(len_limit);
        if (lim < ea) ea = lim;
        if (lim < eb) eb = lim;
    }
    if (ea == 0 and eb == 0) return obj_new_int(0);
    if (ea == 0) return obj_new_int(-1);
    if (eb == 0) return obj_new_int(1);
    const min_len = if (ea < eb) ea else eb;
    const pa: [*]const u8 = @ptrFromInt(sa.ptr);
    const pb: [*]const u8 = @ptrFromInt(sb.ptr);
    for (0..min_len) |i| {
        const ca: u8 = if (nocase != 0) ascii_lower(pa[i]) else pa[i];
        const cb: u8 = if (nocase != 0) ascii_lower(pb[i]) else pb[i];
        if (ca < cb) return obj_new_int(-1);
        if (ca > cb) return obj_new_int(1);
    }
    if (ea < eb) return obj_new_int(-1);
    if (ea > eb) return obj_new_int(1);
    return obj_new_int(0);
}

// Exported: expr ordering comparison — try numeric first, fall back to string.
// Returns TclObj wrapping -1, 0, or 1.
// Used for expr's < > <= >= operators (Tcl 9 semantics: numeric when both
// operands parse as integers, otherwise bytewise string comparison).
//
// Bignum-aware: when one or both operands exceed the i64 range, the
// comparison routes through ``std.math.big.int.Managed.order`` so that
// e.g. ``expr {99 < (1 << 70)}`` returns 1 rather than the lexicographic
// ``"99" > "1180591620717411303424"`` answer.  Match the i128-or-bignum
// promotion discipline the arithmetic helpers use.
pub export fn tcl_expr_order_cmp(a: i32, b: i32) i32 {
    // Numeric type-tag fast path: when both operands are already
    // numeric TclObjs, compare via the typed values directly without
    // forcing a string rendering.  Without this, ``expr {(2**100000)
    // < 0}`` would call ``obj_ensure_string`` on a 30000-digit bignum
    // each time — ``alloc_format`` is O(n²) in digit count, which
    // hangs expr.test.
    const ta = obj.obj_type(a);
    const tb = obj.obj_type(b);
    if ((ta == obj.TYPE_INT or ta == obj.TYPE_BIGNUM) and
        (tb == obj.TYPE_INT or tb == obj.TYPE_BIGNUM))
    {
        if (ta == obj.TYPE_INT and tb == obj.TYPE_INT) {
            const av = obj.obj_get_int(a);
            const bv = obj.obj_get_int(b);
            if (av < bv) return obj_new_int(-1);
            if (av > bv) return obj_new_int(1);
            return obj_new_int(0);
        }
        // At least one bignum — promote both and compare numerically.
        const ap = obj.obj_promote_to_bignum(a);
        defer if (ap.owned) bignum.destroy(ap.m);
        const bp = obj.obj_promote_to_bignum(b);
        defer if (bp.owned) bignum.destroy(bp.m);
        if (ap.m != null and bp.m != null) {
            return switch (ap.m.?.order(bp.m.?.*)) {
                .lt => obj_new_int(-1),
                .eq => obj_new_int(0),
                .gt => obj_new_int(1),
            };
        }
    }
    // Fast path: both operands fit i64 — preserves the zero-allocation
    // numeric compare for the common case.
    const sa = obj_ensure_string(a);
    const sb = obj_ensure_string(b);
    const ai = try_parse_int(sa.ptr, sa.len);
    const bi = try_parse_int(sb.ptr, sb.len);
    if (ai != null and bi != null) {
        const av = ai.?;
        const bv = bi.?;
        if (av < bv) return obj_new_int(-1);
        if (av > bv) return obj_new_int(1);
        return obj_new_int(0);
    }
    // Bignum path: if either operand is a TYPE_BIGNUM TclObj, or its
    // string representation is a valid integer literal too wide for i64,
    // promote both to ``*BigInt`` and compare via ``Managed.order``.
    // We promote unconditionally when either side has a non-i64 numeric
    // form — the fallback to string-compare below still triggers for
    // genuinely non-numeric operands (lists, named values, …).
    //
    // ``parse_i128`` covers the i64 < |x| <= i128 range cheaply;
    // ``string_needs_bignum`` (Managed parse) catches integer-shaped
    // literals beyond i128 like ``"1`` + 100 zeros (Copilot review
    // #326).  Without that fallback, ``expr {(1<<200) > 99}`` and
    // similar comparisons of huge string literals fell through to
    // bytewise compare and produced the wrong sign.
    const a_can_bignum = obj.obj_type(a) == obj.TYPE_BIGNUM or
        bignum.parse_i128(sa.ptr, sa.len) != null or
        bignum.string_needs_bignum(sa.ptr, sa.len);
    const b_can_bignum = obj.obj_type(b) == obj.TYPE_BIGNUM or
        bignum.parse_i128(sb.ptr, sb.len) != null or
        bignum.string_needs_bignum(sb.ptr, sb.len);
    if (a_can_bignum and b_can_bignum) {
        const ap = obj.obj_promote_to_bignum(a);
        defer if (ap.owned) bignum.destroy(ap.m);
        const bp = obj.obj_promote_to_bignum(b);
        defer if (bp.owned) bignum.destroy(bp.m);
        if (ap.m == null or bp.m == null) return obj_new_int(0);
        return switch (ap.m.?.order(bp.m.?.*)) {
            .lt => obj_new_int(-1),
            .eq => obj_new_int(0),
            .gt => obj_new_int(1),
        };
    }
    // Numeric float compare path: when at least one operand is a
    // TYPE_FLOAT (or a string like ``"1.5"`` / ``"1e2"``), promote
    // both to f64 and compare.  Without this branch ``expr {1 ==
    // 1.0}`` falls through to the bytewise compare below and reports
    // unequal — Tcl 9 says those should compare equal.
    const a_can_float = obj.obj_type(a) == obj.TYPE_FLOAT or obj.try_parse_float(sa.ptr, sa.len) != null;
    const b_can_float = obj.obj_type(b) == obj.TYPE_FLOAT or obj.try_parse_float(sb.ptr, sb.len) != null;
    if ((a_can_float and (b_can_float or bi != null or a_can_bignum or b_can_bignum)) or
        (b_can_float and (a_can_float or ai != null or a_can_bignum or b_can_bignum)))
    {
        const af = obj.obj_get_float(a);
        const bf = obj.obj_get_float(b);
        if (af < bf) return obj_new_int(-1);
        if (af > bf) return obj_new_int(1);
        return obj_new_int(0);
    }
    // Fall back to bytewise string comparison (Unicode code-point order for
    // single-character values, which is what Asciify and similar procs need).
    const min_len = if (sa.len < sb.len) sa.len else sb.len;
    if (min_len > 0) {
        const pa: [*]const u8 = @ptrFromInt(sa.ptr);
        const pb: [*]const u8 = @ptrFromInt(sb.ptr);
        for (0..min_len) |i| {
            if (pa[i] < pb[i]) return obj_new_int(-1);
            if (pa[i] > pb[i]) return obj_new_int(1);
        }
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
    // Accept ``end`` / ``end-N`` / ``end+N`` as well as plain integers —
    // the same arithmetic list indexing uses.  Previously only plain
    // integers parsed, so ``string index foo end`` resolved to 0.
    const slen_i64: i64 = @intCast(s.len);
    const i_val = list_mod.resolve_list_index(idx, slen_i64);
    if (i_val < 0 or i_val >= slen_i64) return obj_new_string(0, 0);
    const pos: u32 = @intCast(i_val);
    const src: [*]const u8 = @ptrFromInt(s.ptr);
    const buf = alloc(1);
    const dst: [*]u8 = @ptrFromInt(buf);
    dst[0] = src[pos];
    return obj_new_string(@bitCast(buf), 1);
}

// Exported: string range — extract a substring [first..last] (inclusive).
pub export fn string_range(value: i32, first: i32, last: i32) i32 {
    const s = obj_ensure_string(value);
    const slen: i64 = @intCast(s.len);
    var f = list_mod.resolve_list_index(first, slen);
    var l = list_mod.resolve_list_index(last, slen);
    if (f < 0) f = 0;
    if (l >= slen) l = slen - 1;
    if (f > l or f >= slen) return obj_new_string(0, 0);
    const start: u32 = @intCast(f);
    const count: u32 = @intCast(l - f + 1);
    return obj_new_string_copy(s.ptr + start, count);
}

// Exported: string map — apply a mapping list {from to from to ...} to a string.
pub export fn string_map(mapping: i32, value: i32) i32 {
    return string_map_impl(mapping, value, false);
}

/// ``string map -nocase MAP STRING`` — case-insensitive variant.
/// Splits the same MAP/STRING handling as ``string_map`` but
/// compares each MAP key against STRING with ASCII case folded.
/// The replacement bytes are inserted as-is (case preserved).
pub export fn string_map_nocase(mapping: i32, value: i32) i32 {
    return string_map_impl(mapping, value, true);
}

fn ascii_lower(c: u8) u8 {
    if (c >= 'A' and c <= 'Z') return c + 32;
    return c;
}

fn string_map_impl(mapping: i32, value: i32, nocase: bool) i32 {
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
                    const a = src[pos + k];
                    const b = fp[k];
                    const eq = if (nocase) ascii_lower(a) == ascii_lower(b) else a == b;
                    if (!eq) {
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
    return obj_new_string(@bitCast(buf), @bitCast(out_len));
}

// Exported: string match — glob pattern matching (* and ? wildcards).
pub export fn string_match(pattern: i32, value: i32) i32 {
    const sp = obj_ensure_string(pattern);
    const sv = obj_ensure_string(value);
    const matched = glob_match(sp.ptr, sp.len, sv.ptr, sv.len);
    return obj_new_int(if (matched) @as(i64, 1) else @as(i64, 0));
}

pub fn glob_match(pp: u32, plen: u32, vp: u32, vlen: u32) bool {
    // Guard against null/zero pointers produced by obj_ensure_string(0).
    if (plen == 0) return vlen == 0;
    const pat: [*]const u8 = @ptrFromInt(pp);
    // vp may be 0 when vlen == 0; defer the cast until we know vlen > 0.
    var pi: u32 = 0;
    var vi: u32 = 0;
    var star_pi: u32 = plen;
    var star_vi: u32 = 0;
    if (vlen == 0) {
        // Non-empty pattern vs empty value — only matches if every
        // remaining pattern char is ``*`` (skipping over any
        // backslash escapes that don't reduce to ``*``).
        while (pi < plen) {
            if (pat[pi] != '*') return false;
            pi += 1;
        }
        return true;
    }
    const val: [*]const u8 = @ptrFromInt(vp);
    while (vi < vlen or pi < plen) {
        if (pi < plen and pat[pi] == '*') {
            star_pi = pi;
            star_vi = vi;
            pi += 1;
        } else if (pi < plen and pat[pi] == '\\' and pi + 1 < plen) {
            // Backslash escape: ``\?`` ``\*`` ``\[`` ``\\`` etc. all
            // turn into the LITERAL char that follows.  Without this,
            // ``string match {\?*} cmd`` parses ``\`` as a literal
            // and tries to match ``c`` against ``\``, returning 0
            // for the empty case but 1 for any value starting with
            // a ``?`` (because ``?`` then matches anything as a
            // wildcard) — root cause of opt-10.x silently passing
            // ``cmd`` as if it were optional.
            if (vi < vlen and pat[pi + 1] == val[vi]) {
                pi += 2;
                vi += 1;
            } else if (star_pi < plen) {
                pi = star_pi + 1;
                if (star_vi >= vlen) return false;
                star_vi += 1;
                vi = star_vi;
            } else {
                return false;
            }
        } else if (pi < plen and vi < vlen and (pat[pi] == '?' or pat[pi] == val[vi])) {
            pi += 1;
            vi += 1;
        } else if (star_pi < plen) {
            pi = star_pi + 1;
            // No more value positions to try — value exhausted, no match.
            if (star_vi >= vlen) return false;
            star_vi += 1;
            vi = star_vi;
        } else {
            return false;
        }
    }
    return true;
}

/// Return true if *c* is in *chars* (a u8 slice).
inline fn in_chars(c: u8, chars: [*]const u8, chars_len: u32) bool {
    for (0..chars_len) |i| {
        if (chars[i] == c) return true;
    }
    return false;
}

/// Test whether *c* is a "trim" character for ``string trim ?chars?``.
/// When *chars_obj* is 0 the default is Tcl whitespace (space, tab, LF,
/// CR, VT, FF).  Otherwise the caller's trim set wins verbatim.
inline fn is_trim_char(c: u8, chars_ptr: u32, chars_len: u32) bool {
    if (chars_len == 0) return is_space(c);
    const p: [*]const u8 = @ptrFromInt(chars_ptr);
    return in_chars(c, p, chars_len);
}

// Exported: string trim — strip leading/trailing *chars* (default whitespace).
// ``chars`` is a TclObj whose string value enumerates the bytes to trim;
// pass 0 to use the default whitespace set.
pub export fn string_trim(value: i32, chars: i32) i32 {
    const s = obj_ensure_string(value);
    if (s.len == 0) return value;
    var cp: u32 = 0;
    var cl: u32 = 0;
    if (chars != 0) {
        const cs = obj_ensure_string(chars);
        cp = cs.ptr;
        cl = cs.len;
    }
    const src: [*]const u8 = @ptrFromInt(s.ptr);
    var start: u32 = 0;
    while (start < s.len and is_trim_char(src[start], cp, cl)) start += 1;
    var end: u32 = s.len;
    while (end > start and is_trim_char(src[end - 1], cp, cl)) end -= 1;
    if (start == 0 and end == s.len) return value;
    return obj_new_string_copy(s.ptr + start, end - start);
}

// Exported: string trimleft — strip leading *chars* (default whitespace).
pub export fn string_trimleft(value: i32, chars: i32) i32 {
    const s = obj_ensure_string(value);
    if (s.len == 0) return value;
    var cp: u32 = 0;
    var cl: u32 = 0;
    if (chars != 0) {
        const cs = obj_ensure_string(chars);
        cp = cs.ptr;
        cl = cs.len;
    }
    const src: [*]const u8 = @ptrFromInt(s.ptr);
    var start: u32 = 0;
    while (start < s.len and is_trim_char(src[start], cp, cl)) start += 1;
    if (start == 0) return value;
    return obj_new_string_copy(s.ptr + start, s.len - start);
}

// Exported: string trimright — strip trailing *chars* (default whitespace).
pub export fn string_trimright(value: i32, chars: i32) i32 {
    const s = obj_ensure_string(value);
    if (s.len == 0) return value;
    var cp: u32 = 0;
    var cl: u32 = 0;
    if (chars != 0) {
        const cs = obj_ensure_string(chars);
        cp = cs.ptr;
        cl = cs.len;
    }
    const src: [*]const u8 = @ptrFromInt(s.ptr);
    var end: u32 = s.len;
    while (end > 0 and is_trim_char(src[end - 1], cp, cl)) end -= 1;
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

/// ``string equal ?-nocase? ?-length N? a b`` — full-arity helper.
/// Same flag semantics as :func:`string_compare_full`.
pub export fn string_equal_full(a: i32, b: i32, nocase: i32, len_limit: i32) i32 {
    if (nocase == 0 and len_limit < 0) {
        // Fast path: bytewise equality without flag interpretation —
        // matches the historical ``string_equal`` exactly.
        return string_equal(a, b);
    }
    const r = string_compare_full(a, b, nocase, len_limit);
    const v = obj.obj_get_int(r);
    obj.tcl_obj_release(r);
    return obj_new_int(if (v == 0) @as(i64, 1) else @as(i64, 0));
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
    return obj_new_string(@bitCast(buf), @bitCast(total));
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
    return obj_new_string(@bitCast(buf), @bitCast(sv.len));
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
    return obj_new_string(@bitCast(buf), @bitCast(sv.len));
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
    return obj_new_string(@bitCast(buf), @bitCast(sv.len));
}

// Exported: string totitle — uppercase the first alphabetic byte and
// lowercase every other byte.  Tcl's reference totitle walks the
// whole string by Unicode codepoint; this ASCII-only approximation
// covers the tcltest / tcllib patterns we see in the 9.0 corpus and
// keeps parity with ``string toupper`` / ``string tolower`` above.
pub export fn string_totitle(value: i32) i32 {
    const sv = obj_ensure_string(value);
    if (sv.len == 0) return value;
    const buf = alloc(sv.len);
    const src: [*]const u8 = @ptrFromInt(sv.ptr);
    const dst: [*]u8 = @ptrFromInt(buf);
    var first_alpha_seen = false;
    for (0..sv.len) |i| {
        const c = src[i];
        if (!first_alpha_seen and ((c >= 'a' and c <= 'z') or (c >= 'A' and c <= 'Z'))) {
            dst[i] = if (c >= 'a' and c <= 'z') c - 32 else c;
            first_alpha_seen = true;
        } else if (c >= 'A' and c <= 'Z') {
            dst[i] = c + 32;
        } else {
            dst[i] = c;
        }
    }
    return obj_new_string(@bitCast(buf), @bitCast(sv.len));
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
    return obj_new_string(@bitCast(buf), @bitCast(total));
}

// Exported: string is integer — check if a string is a valid integer.
//
// Tcl 9 ``string is integer`` accepts any decimal / hex / octal /
// binary literal that fits the runtime's arbitrary-precision integer
// type — i.e. with libtommath / our Managed-backed bignum, *any*
// integer string regardless of magnitude.  Stage 1's i64-only check
// rejected literals exceeding i64 (``string is integer
// 99999999999999999999999`` returned 0); Stage 2 lets the bignum
// parse path accept them via ``alloc_from_string``.
pub export fn string_is_integer(value: i32) i32 {
    const sv = obj_ensure_string(value);
    if (sv.len == 0) return obj_new_int(0);
    // ``string is integer`` accepts a TYPE_BIGNUM directly without
    // going through the string parse — saves the parse cost when the
    // operand is already known-integer (``string is integer [expr {1
    // << 200}]``).
    if (obj.obj_type(value) == obj.TYPE_BIGNUM or obj.obj_type(value) == obj.TYPE_INT) {
        return obj_new_int(1);
    }
    if (try_parse_int(sv.ptr, sv.len) != null) {
        return obj_new_int(1);
    }
    // Bignum-shaped literal — ``9223372036854775808`` etc.  The
    // module-level ``bignum`` import is used for the parse.
    if (bignum.parse_i128(sv.ptr, sv.len) != null) {
        return obj_new_int(1);
    }
    const m = bignum.alloc_from_string(sv.ptr, sv.len) orelse return obj_new_int(0);
    bignum.destroy(m);
    return obj_new_int(1);
}

// Exported: string is wideinteger — same as ``string is integer``
// for the bignum path (both accept arbitrary precision), but kept
// as a separate symbol so the WASM emitter / Python registry can
// route the ``string is wideinteger ...`` form to it.  In Tcl 9 the
// distinction is mostly historical — ``wideinteger`` was the
// 64-bit-or-fits class before bignum landed.
pub export fn string_is_wideinteger(value: i32) i32 {
    return string_is_integer(value);
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

// ``list_quote_elem`` is now a thin wrapper around the canonical
// :func:`obj.list_elem_quote` in tcl_obj.zig.  Kept as a file-local
// alias so the split-by-char path below can stay a single-line call;
// any future change to the quoting rules only happens in one place.
const list_quote_elem = obj.list_elem_quote;

// Exported: split — split a string by a separator into a Tcl list.
// If splitChars is empty, splits into individual characters.
pub export fn tcl_cmd_split(value: i32, split_chars: i32) i32 {
    const sv = obj_ensure_string(value);
    const sd = obj_ensure_string(split_chars);
    if (sv.len == 0) return obj_new_string(0, 0);
    const src: [*]const u8 = @ptrFromInt(sv.ptr);

    // Empty splitChars: split into individual characters
    if (sd.len == 0) {
        // Each char becomes a properly-quoted list element.
        // Allocate generously: each char can expand to at most 4 bytes
        // (backslash + char + possible braces) plus a space separator.
        const buf = alloc(sv.len * 5 + 4);
        var out: u32 = 0;
        for (0..sv.len) |i| {
            if (i > 0) {
                const d: [*]u8 = @ptrFromInt(buf + out);
                d[0] = ' ';
                out += 1;
            }
            out = list_quote_elem(buf, out, sv.ptr + @as(u32, @intCast(i)), 1);
        }
        return obj_new_string(@bitCast(buf), @bitCast(out));
    }

    // Single-char separator (common case)
    const sep: [*]const u8 = @ptrFromInt(sd.ptr);
    if (sd.len == 1) {
        const sc = sep[0];
        // Allocate generously — backslash-escape path can double each byte.
        const buf = alloc(sv.len * 3 + 4);
        var out: u32 = 0;
        var start: u32 = 0;
        var i: u32 = 0;
        var first = true;
        while (i <= sv.len) : (i += 1) {
            if (i == sv.len or src[i] == sc) {
                if (!first) {
                    const d: [*]u8 = @ptrFromInt(buf + out);
                    d[0] = ' ';
                    out += 1;
                }
                first = false;
                out = list_quote_elem(buf, out, sv.ptr + start, i - start);
                start = i + 1;
            }
        }
        return obj_new_string(@bitCast(buf), @bitCast(out));
    }

    // Multi-char separator: split on any char in splitChars
    const buf = alloc(sv.len * 3 + 4);
    var out: u32 = 0;
    var start: u32 = 0;
    var i: u32 = 0;
    var first = true;
    while (i <= sv.len) : (i += 1) {
        var is_sep = (i == sv.len);
        if (!is_sep) {
            for (0..sd.len) |k| {
                if (src[i] == sep[k]) {
                    is_sep = true;
                    break;
                }
            }
        }
        if (is_sep) {
            if (!first) {
                const d: [*]u8 = @ptrFromInt(buf + out);
                d[0] = ' ';
                out += 1;
            }
            first = false;
            out = list_quote_elem(buf, out, sv.ptr + start, i - start);
            start = i + 1;
            continue;
        }
    }
    return obj_new_string(@bitCast(buf), @bitCast(out));
}

// Exported: join — join a Tcl list with a separator string.
// ``separator == 0`` means the caller omitted the optional argument
// (``join list``), in which case Tcl defaults to a single space.  The
// compiler fills missing runtime-call args with 0 so we have to
// recover the default here rather than at the call site.
pub export fn tcl_cmd_join(list: i32, separator: i32) i32 {
    const sl = obj_ensure_string(list);
    const default_sep = " ";
    const ss_len: u32 = if (separator == 0) @intCast(default_sep.len) else obj_ensure_string(separator).len;
    const ss_ptr: u32 = if (separator == 0) @intCast(@intFromPtr(default_sep.ptr)) else obj_ensure_string(separator).ptr;
    if (sl.len == 0) return obj_new_string(0, 0);
    const n = list_count_elements(sl.ptr, sl.len);
    if (n <= 0) return obj_new_string(0, 0);
    if (n == 1) {
        const elem = list_element_at(sl.ptr, sl.len, 0);
        return obj_new_string_copy(sl.ptr + elem.start, elem.len);
    }
    // Estimate output: sum of element lengths + (n-1) * sep_len
    const buf = alloc(sl.len + @as(u32, @intCast(n)) * ss_len + 1);
    var out: u32 = 0;
    var idx: i64 = 0;
    while (idx < n) : (idx += 1) {
        if (idx > 0 and ss_len > 0) {
            memcpy(buf + out, ss_ptr, ss_len);
            out += ss_len;
        }
        const elem = list_element_at(sl.ptr, sl.len, idx);
        if (elem.len > 0) {
            memcpy(buf + out, sl.ptr + elem.start, elem.len);
            out += elem.len;
        }
    }
    return obj_new_string(@bitCast(buf), @bitCast(out));
}

// Exported: concat — concatenate two TclObj string representations with space.
// Each argument has leading/trailing whitespace trimmed before joining.
// If both are empty after trimming, returns an empty string object.
pub export fn tcl_cmd_concat(a: i32, b: i32) i32 {
    const sa = obj_ensure_string(a);
    const sb = obj_ensure_string(b);
    // Trim leading/trailing whitespace from each argument.
    var a_start: u32 = 0;
    var a_end: u32 = sa.len;
    if (sa.len > 0) {
        const pa: [*]const u8 = @ptrFromInt(sa.ptr);
        while (a_start < a_end and is_space(pa[a_start])) a_start += 1;
        while (a_end > a_start and is_space(pa[a_end - 1])) a_end -= 1;
    }
    var b_start: u32 = 0;
    var b_end: u32 = sb.len;
    if (sb.len > 0) {
        const pb: [*]const u8 = @ptrFromInt(sb.ptr);
        while (b_start < b_end and is_space(pb[b_start])) b_start += 1;
        while (b_end > b_start and is_space(pb[b_end - 1])) b_end -= 1;
    }
    const ta_len = a_end - a_start;
    const tb_len = b_end - b_start;
    if (ta_len == 0 and tb_len == 0) return obj_new_string(0, 0);
    if (ta_len == 0) return obj_new_string_copy(sb.ptr + b_start, tb_len);
    if (tb_len == 0) return obj_new_string_copy(sa.ptr + a_start, ta_len);
    const total = ta_len + 1 + tb_len;
    const buf = alloc(total);
    memcpy(buf, sa.ptr + a_start, ta_len);
    const dst: [*]u8 = @ptrFromInt(buf + ta_len);
    dst[0] = ' ';
    memcpy(buf + ta_len + 1, sb.ptr + b_start, tb_len);
    return obj_new_string(@bitCast(buf), @bitCast(total));
}
