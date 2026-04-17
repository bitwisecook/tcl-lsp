// TclObj value type, memory management, and string helpers for WASM.
//
// Memory layout of a TclObj:
//   offset 0: refcount  (i32)
//   offset 4: type_tag  (i32)  0=string, 1=int, 2=list
//   offset 8: int_cache (i64)  cached integer representation
//   offset 16: str_ptr  (i32)  pointer to UTF-8 data in linear memory
//   offset 20: str_len  (i32)  byte length of the string representation
//   Total: 24 bytes per TclObj

// Type tags
pub const TYPE_STRING: i32 = 0;
pub const TYPE_INT: i32 = 1;
pub const TYPE_LIST: i32 = 2;
pub const TYPE_DICT: i32 = 3;

// TclObj field offsets
pub const OBJ_REFCOUNT: u32 = 0;
pub const OBJ_TYPE_TAG: u32 = 4;
pub const OBJ_INT_CACHE: u32 = 8;
pub const OBJ_STR_PTR: u32 = 16;
pub const OBJ_STR_LEN: u32 = 20;
pub const OBJ_SIZE: u32 = 24;

// Simple bump allocator over WASM linear memory with free-list recycling.
var heap_ptr: u32 = 65536;
var free_list: u32 = 0;

pub fn alloc(size: u32) callconv(.C) u32 {
    const aligned = (size + 7) & ~@as(u32, 7);
    if (aligned == OBJ_SIZE and free_list != 0) {
        const ptr = free_list;
        free_list = @intCast(read_i32(ptr));
        return ptr;
    }
    const ptr = heap_ptr;
    heap_ptr += aligned;
    return ptr;
}

fn free_obj(addr: u32) void {
    write_i32(addr, @intCast(free_list));
    free_list = addr;
}

pub fn read_i32(addr: u32) i32 {
    const ptr: [*]const u8 = @ptrFromInt(addr);
    const bytes = [4]u8{ ptr[0], ptr[1], ptr[2], ptr[3] };
    return @bitCast(bytes);
}

pub fn write_i32(addr: u32, val: i32) void {
    const ptr: [*]u8 = @ptrFromInt(addr);
    const bytes: [4]u8 = @bitCast(val);
    ptr[0] = bytes[0];
    ptr[1] = bytes[1];
    ptr[2] = bytes[2];
    ptr[3] = bytes[3];
}

pub fn read_i64(addr: u32) i64 {
    const ptr: [*]const u8 = @ptrFromInt(addr);
    const bytes = [8]u8{
        ptr[0], ptr[1], ptr[2], ptr[3],
        ptr[4], ptr[5], ptr[6], ptr[7],
    };
    return @bitCast(bytes);
}

pub fn write_i64(addr: u32, val: i64) void {
    const ptr: [*]u8 = @ptrFromInt(addr);
    const bytes: [8]u8 = @bitCast(val);
    inline for (0..8) |i| {
        ptr[i] = bytes[i];
    }
}

// Allocate a new TclObj with refcount 1
pub fn obj_alloc() u32 {
    const ptr = alloc(OBJ_SIZE);
    write_i32(ptr + OBJ_REFCOUNT, 1);
    write_i32(ptr + OBJ_TYPE_TAG, TYPE_STRING);
    write_i64(ptr + OBJ_INT_CACHE, 0);
    write_i32(ptr + OBJ_STR_PTR, 0);
    write_i32(ptr + OBJ_STR_LEN, 0);
    return ptr;
}

pub export fn obj_new_int(value: i64) i32 {
    const ptr = obj_alloc();
    write_i32(ptr + OBJ_TYPE_TAG, TYPE_INT);
    write_i64(ptr + OBJ_INT_CACHE, value);
    return @as(i32, @intCast(ptr));
}

pub export fn obj_new_string(data_ptr: i32, length: i32) i32 {
    const ptr = obj_alloc();
    write_i32(ptr + OBJ_TYPE_TAG, TYPE_STRING);
    write_i32(ptr + OBJ_STR_PTR, data_ptr);
    write_i32(ptr + OBJ_STR_LEN, length);
    return @as(i32, @intCast(ptr));
}

pub export fn obj_get_int(obj: i32) i64 {
    // Null/zero pointer sentinel — return 0 rather than reading
    // arbitrary bytes from address 0 (the pre-heap WASM stack area).
    // ``global_get`` returns 0 for unset variables; callers that pass
    // that result here expect to get the empty-string / zero integer.
    if (obj == 0) return 0;
    const addr: u32 = @intCast(obj);
    const tag = read_i32(addr + OBJ_TYPE_TAG);
    if (tag == TYPE_INT) return read_i64(addr + OBJ_INT_CACHE);
    if (tag == TYPE_STRING) {
        const sptr: u32 = @intCast(read_i32(addr + OBJ_STR_PTR));
        const slen: u32 = @intCast(read_i32(addr + OBJ_STR_LEN));
        if (try_parse_int(sptr, slen)) |val| {
            write_i32(addr + OBJ_TYPE_TAG, TYPE_INT);
            write_i64(addr + OBJ_INT_CACHE, val);
            return val;
        }
        // Tcl boolean literals — ``true`` / ``yes`` / ``on`` → 1,
        // ``false`` / ``no`` / ``off`` → 0 (case-insensitive, and
        // Tcl also accepts unique prefixes).  Used by ``expr`` in
        // boolean contexts (``$x && $x`` with ``$x = "true"``) and
        // by tcltest's ``AcceptBoolean``.
        if (try_parse_bool(sptr, slen)) |val| {
            write_i32(addr + OBJ_TYPE_TAG, TYPE_INT);
            write_i64(addr + OBJ_INT_CACHE, val);
            return val;
        }
    }
    return read_i64(addr + OBJ_INT_CACHE);
}

/// Recognise Tcl boolean keywords (case-insensitive) — ``true`` /
/// ``yes`` / ``on`` → 1, ``false`` / ``no`` / ``off`` → 0.  Returns
/// ``null`` if the string is not a recognised boolean.  Matches the
/// set accepted by ``Tcl_GetBooleanFromObj``.
pub fn try_parse_bool(ptr: u32, len: u32) ?i64 {
    if (len == 0 or len > 5) return null;
    const src: [*]const u8 = @ptrFromInt(ptr);
    var buf: [5]u8 = undefined;
    for (0..len) |i| {
        const c = src[i];
        buf[i] = if (c >= 'A' and c <= 'Z') c + 32 else c;
    }
    const lc = buf[0..len];
    // True-valued keywords.
    if (std_eq(lc, "true") or std_eq(lc, "yes") or std_eq(lc, "on") or
        std_eq(lc, "tru") or std_eq(lc, "tr") or std_eq(lc, "t") or
        std_eq(lc, "ye") or std_eq(lc, "y"))
        return 1;
    // False-valued keywords (``off`` / ``of`` need the longer match
    // before ``of`` so ``on``/``of`` don't collide; handled by the
    // exact-match comparisons above already).
    if (std_eq(lc, "false") or std_eq(lc, "fals") or std_eq(lc, "fal") or
        std_eq(lc, "fa") or std_eq(lc, "f") or
        std_eq(lc, "no") or std_eq(lc, "n") or
        std_eq(lc, "off") or std_eq(lc, "of"))
        return 0;
    return null;
}

fn std_eq(a: []const u8, b: []const u8) bool {
    if (a.len != b.len) return false;
    for (a, b) |ca, cb| {
        if (ca != cb) return false;
    }
    return true;
}

pub fn try_parse_int(ptr: u32, len: u32) ?i64 {
    if (len == 0) return null;
    const src: [*]const u8 = @ptrFromInt(ptr);
    var i: u32 = 0;
    while (i < len and is_space(src[i])) i += 1;
    if (i >= len) return null;
    var negative = false;
    if (src[i] == '-') {
        negative = true;
        i += 1;
    } else if (src[i] == '+') {
        i += 1;
    }
    if (i >= len) return null;
    if (src[i] < '0' or src[i] > '9') return null;
    var val: i64 = 0;
    while (i < len and src[i] >= '0' and src[i] <= '9') {
        val = val * 10 + @as(i64, src[i] - '0');
        i += 1;
    }
    while (i < len and is_space(src[i])) i += 1;
    if (i != len) return null;
    return if (negative) -val else val;
}

pub export fn tcl_obj_retain(obj: i32) void {
    const addr: u32 = @intCast(obj);
    const rc = read_i32(addr + OBJ_REFCOUNT);
    write_i32(addr + OBJ_REFCOUNT, rc + 1);
}

pub export fn tcl_obj_release(obj: i32) void {
    if (obj == 0) return;
    const addr: u32 = @intCast(obj);
    const rc = read_i32(addr + OBJ_REFCOUNT);
    if (rc <= 1) {
        free_obj(addr);
    } else {
        write_i32(addr + OBJ_REFCOUNT, rc - 1);
    }
}

pub export fn tcl_var_set(value: i32) i32 {
    return value;
}

pub export fn tcl_var_get(value: i32) i32 {
    return value;
}

// -- TclObj string helpers --

pub fn obj_str_ptr(obj: i32) u32 {
    const addr: u32 = @intCast(obj);
    return @intCast(read_i32(addr + OBJ_STR_PTR));
}

pub fn obj_str_len(obj: i32) u32 {
    const addr: u32 = @intCast(obj);
    return @intCast(read_i32(addr + OBJ_STR_LEN));
}

pub fn obj_type(obj: i32) i32 {
    const addr: u32 = @intCast(obj);
    return read_i32(addr + OBJ_TYPE_TAG);
}

/// Copy *len* bytes from src to dst in linear memory.
pub fn memcpy(dst: u32, src: u32, len: u32) void {
    const d: [*]u8 = @ptrFromInt(dst);
    const s: [*]const u8 = @ptrFromInt(src);
    for (0..len) |i| {
        d[i] = s[i];
    }
}

/// Create a new string TclObj by copying *len* bytes from *src*.
pub fn obj_new_string_copy(src: u32, len: u32) i32 {
    const buf = alloc(len);
    memcpy(buf, src, len);
    return obj_new_string(@intCast(buf), @intCast(len));
}

// Scratch buffer for integer-to-string conversion (no newline)
var itoa_buf2: [21]u8 = undefined;

pub fn itoa_no_nl(value: i64) struct { ptr: [*]u8, len: u32 } {
    var v = value;
    var negative = false;
    if (v < 0) {
        negative = true;
        v = -v;
    }
    var i: u32 = itoa_buf2.len - 1;
    if (v == 0) {
        itoa_buf2[i] = '0';
    } else {
        while (v > 0) {
            itoa_buf2[i] = @as(u8, @intCast(@rem(v, 10))) + '0';
            v = @divTrunc(v, 10);
            if (v > 0) i -= 1;
        }
    }
    if (negative) {
        i -= 1;
        itoa_buf2[i] = '-';
    }
    return .{ .ptr = @as([*]u8, &itoa_buf2) + i, .len = itoa_buf2.len - i };
}

/// Render an integer TclObj to its string representation.
pub fn obj_ensure_string(obj: i32) struct { ptr: u32, len: u32 } {
    if (obj == 0) return .{ .ptr = 0, .len = 0 };
    const addr: u32 = @intCast(obj);
    const tag = read_i32(addr + OBJ_TYPE_TAG);
    if (tag == TYPE_STRING) {
        return .{
            .ptr = @intCast(read_i32(addr + OBJ_STR_PTR)),
            .len = @intCast(read_i32(addr + OBJ_STR_LEN)),
        };
    }
    const sptr: u32 = @intCast(read_i32(addr + OBJ_STR_PTR));
    if (sptr != 0) {
        return .{
            .ptr = sptr,
            .len = @intCast(read_i32(addr + OBJ_STR_LEN)),
        };
    }
    const val = read_i64(addr + OBJ_INT_CACHE);
    const result = itoa_no_nl(val);
    const buf = alloc(result.len);
    memcpy(buf, @intFromPtr(result.ptr), result.len);
    write_i32(addr + OBJ_STR_PTR, @intCast(buf));
    write_i32(addr + OBJ_STR_LEN, @intCast(result.len));
    return .{ .ptr = buf, .len = result.len };
}

pub fn is_space(c: u8) bool {
    return c == ' ' or c == '\t' or c == '\n' or c == '\r';
}

// Compare two byte spans lexicographically. Returns <0, 0, or >0.
pub fn str_cmp(a_ptr: u32, a_len: u32, b_ptr: u32, b_len: u32) i32 {
    const pa: [*]const u8 = @ptrFromInt(a_ptr);
    const pb: [*]const u8 = @ptrFromInt(b_ptr);
    const min_len = if (a_len < b_len) a_len else b_len;
    for (0..min_len) |k| {
        if (pa[k] < pb[k]) return -1;
        if (pa[k] > pb[k]) return 1;
    }
    if (a_len < b_len) return -1;
    if (a_len > b_len) return 1;
    return 0;
}

// Count elements in a Tcl list string.
pub fn list_count_elements(ptr: u32, len: u32) i64 {
    if (len == 0) return 0;
    const src: [*]const u8 = @ptrFromInt(ptr);
    var count: i64 = 0;
    var i: u32 = 0;
    while (i < len) {
        while (i < len and is_space(src[i])) i += 1;
        if (i >= len) break;
        count += 1;
        if (src[i] == '{') {
            i += 1;
            var depth: u32 = 1;
            while (i < len and depth > 0) {
                // Tcl list parsing: a backslash escapes the next
                // character so ``\{`` / ``\}`` do NOT affect the
                // brace-nesting depth.  Consume both bytes and
                // continue without touching depth.
                if (src[i] == '\\' and i + 1 < len) {
                    i += 2;
                    continue;
                }
                if (src[i] == '{') {
                    depth += 1;
                } else if (src[i] == '}') {
                    depth -= 1;
                }
                i += 1;
            }
        } else {
            while (i < len and !is_space(src[i])) {
                if (src[i] == '\\' and i + 1 < len) {
                    i += 2;
                } else {
                    i += 1;
                }
            }
        }
    }
    return count;
}

// Get the start and length of the nth element (0-based) in a Tcl list.
pub fn list_element_at(ptr: u32, len: u32, idx: i64) struct { start: u32, len: u32, braced: bool } {
    if (len == 0) return .{ .start = 0, .len = 0, .braced = false };
    const src: [*]const u8 = @ptrFromInt(ptr);
    var count: i64 = 0;
    var i: u32 = 0;
    while (i < len) {
        while (i < len and is_space(src[i])) i += 1;
        if (i >= len) break;
        if (src[i] == '{') {
            i += 1;
            const inner_start = i;
            var depth: u32 = 1;
            while (i < len and depth > 0) {
                // Backslash escapes the next char — ``\{`` / ``\}``
                // inside a braced list element are NOT depth-changing.
                if (src[i] == '\\' and i + 1 < len) {
                    i += 2;
                    continue;
                }
                if (src[i] == '{') {
                    depth += 1;
                } else if (src[i] == '}') {
                    depth -= 1;
                }
                i += 1;
            }
            if (count == idx) {
                return .{ .start = inner_start, .len = i - 1 - inner_start, .braced = true };
            }
        } else {
            const elem_start = i;
            while (i < len and !is_space(src[i])) {
                if (src[i] == '\\' and i + 1 < len) {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            if (count == idx) {
                return .{ .start = elem_start, .len = i - elem_start, .braced = false };
            }
        }
        count += 1;
    }
    return .{ .start = 0, .len = 0, .braced = false };
}

// Copy an unbraced list-element's bytes, expanding backslash sequences.
// Returns number of output bytes written to dst.
pub fn copy_unbraced_elem(dst: u32, src_ptr: u32, src_len: u32) u32 {
    const src: [*]const u8 = @ptrFromInt(src_ptr);
    const out: [*]u8 = @ptrFromInt(dst);
    var si: u32 = 0;
    var di: u32 = 0;
    while (si < src_len) {
        if (src[si] == '\\' and si + 1 < src_len) {
            si += 1;
            const ch = src[si];
            switch (ch) {
                'n' => { out[di] = '\n'; di += 1; },
                't' => { out[di] = '\t'; di += 1; },
                'r' => { out[di] = '\r'; di += 1; },
                else => { out[di] = ch; di += 1; },
            }
            si += 1;
        } else {
            out[di] = src[si];
            di += 1;
            si += 1;
        }
    }
    return di;
}

// Check if a string value needs braces when used as a list/dict element.
pub fn dict_needs_braces(ptr: u32, len: u32) bool {
    if (len == 0) return true;
    const src: [*]const u8 = @ptrFromInt(ptr);
    for (0..len) |i| {
        if (is_space(src[i])) return true;
    }
    return false;
}

// Append a string as a properly-quoted list element.  Returns new offset.
// Handles all cases: empty, spaces, special chars, unbalanced braces.
pub fn list_elem_quote(buf: u32, off: u32, ptr: u32, len: u32) u32 {
    if (len == 0) {
        const d: [*]u8 = @ptrFromInt(buf + off);
        d[0] = '{'; d[1] = '}';
        return off + 2;
    }
    const src: [*]const u8 = @ptrFromInt(ptr);
    var has_special = false;
    var brace_balance: i32 = 0;
    var min_balance: i32 = 0;
    for (0..len) |k| {
        const ch = src[k];
        if (is_space(ch) or ch == '"') has_special = true;
        if (ch == '{') brace_balance += 1;
        if (ch == '}') {
            brace_balance -= 1;
            if (brace_balance < min_balance) min_balance = brace_balance;
        }
    }
    const starts_with_brace = src[0] == '{';
    const balanced = (brace_balance == 0) and (min_balance >= 0);
    // Plain word: no special chars, balanced braces, doesn't start with `{`
    if (!has_special and !starts_with_brace and brace_balance == 0) {
        memcpy(buf + off, ptr, len);
        return off + len;
    }
    // Safely braceable: balanced inner braces → wrap in {}
    if (balanced and !starts_with_brace) {
        const d0: [*]u8 = @ptrFromInt(buf + off);
        d0[0] = '{';
        memcpy(buf + off + 1, ptr, len);
        const d1: [*]u8 = @ptrFromInt(buf + off + 1 + len);
        d1[0] = '}';
        return off + len + 2;
    }
    // Starts with { and balanced braces → wrap in outer {} giving {{...}}
    if (starts_with_brace and balanced) {
        const d0: [*]u8 = @ptrFromInt(buf + off);
        d0[0] = '{';
        memcpy(buf + off + 1, ptr, len);
        const d1: [*]u8 = @ptrFromInt(buf + off + 1 + len);
        d1[0] = '}';
        return off + len + 2;
    }
    // Unbalanced braces or other problematic chars → backslash-escape
    var o = off;
    for (0..len) |k| {
        const ch = src[k];
        const needs_esc = is_space(ch) or ch == '{' or ch == '}' or
            ch == '\\' or ch == '"' or ch == '$' or ch == '[' or ch == ';';
        if (needs_esc) {
            const d: [*]u8 = @ptrFromInt(buf + o);
            d[0] = '\\';
            o += 1;
        }
        const d2: [*]u8 = @ptrFromInt(buf + o);
        d2[0] = ch;
        o += 1;
    }
    return o;
}

// Append an element to a buffer, adding braces if needed. Returns new offset.
pub fn dict_append_elem(buf: u32, offset: u32, ptr: u32, len: u32) u32 {
    var off = offset;
    if (dict_needs_braces(ptr, len)) {
        const d: [*]u8 = @ptrFromInt(buf + off);
        d[0] = '{';
        off += 1;
        memcpy(buf + off, ptr, len);
        off += len;
        const d2: [*]u8 = @ptrFromInt(buf + off);
        d2[0] = '}';
        off += 1;
    } else {
        memcpy(buf + off, ptr, len);
        off += len;
    }
    return off;
}
