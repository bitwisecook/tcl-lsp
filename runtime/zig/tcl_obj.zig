// TclObj value type, memory management, and string helpers for WASM.
//
// Memory layout of a TclObj:
//   offset 0: refcount  (i32)
//   offset 4: type_tag  (i32)  0=string, 1=int, 2=list
//   offset 8: int_cache (i64)  cached integer representation
//   offset 16: str_ptr  (i32)  pointer to UTF-8 data in linear memory
//   offset 20: str_len  (i32)  byte length of the string representation
//   Total: 24 bytes per TclObj
//
// Layering: character classification and byte-span comparison are in
// ``tcl_chars.zig``; list-quoting / list-parsing / backslash decoding
// live in their own modules.  This file owns the TclObj memory model
// and integer / boolean scalar parsers.  ``pub const`` re-exports of
// ``is_space`` / ``is_scan_space`` / ``str_cmp`` are kept as
// compatibility shims so existing ``obj.is_space`` callers work —
// new code should import ``tcl_chars.zig`` directly.

const chars = @import("tcl_chars.zig");

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

const std_eq = chars.slice_eq;

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

// Character classification re-exports: callers that already say
// ``obj.is_space`` / ``obj.str_cmp`` keep working, but new code should
// import ``tcl_chars.zig`` directly.
pub const is_space = chars.is_space;
const is_scan_space = chars.is_scan_space;
pub const str_cmp = chars.str_cmp;

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

// Backslash-decoding helpers live in ``tcl_bs.zig``; ``copy_unbraced_elem``
// here becomes a thin wrapper around :func:`tcl_bs.decode_into` so the
// list-parse path and the script-word ``subst_flagged`` path share one
// canonical decoder.  Re-exports keep existing ``obj.encode_utf8`` /
// ``obj.consume_bs_escape`` / ``obj.copy_unbraced_elem`` callers working
// during migration.
const bs = @import("tcl_bs.zig");
pub const encode_utf8 = bs.encode_utf8;
pub const consume_bs_escape = bs.consume_bs_escape;

/// Copy an unbraced list-element's bytes, expanding backslash sequences
/// via :func:`tcl_bs.decode_into`.  Returns number of output bytes
/// written to ``dst``.
pub fn copy_unbraced_elem(dst: u32, src_ptr: u32, src_len: u32) u32 {
    return bs.decode_into(src_ptr, src_len, dst);
}

// List-element scan / convert / quote helpers live in
// ``tcl_list_quote.zig``.  Re-export surface area so existing callers
// that say ``obj.scan_element`` / ``obj.list_elem_quote`` etc. keep
// working — new code should import ``tcl_list_quote.zig`` directly.
const list_quote = @import("tcl_list_quote.zig");
pub const FLAG_CONVERT_NONE = list_quote.FLAG_CONVERT_NONE;
pub const FLAG_DONT_USE_BRACES = list_quote.FLAG_DONT_USE_BRACES;
pub const FLAG_CONVERT_BRACE = list_quote.FLAG_CONVERT_BRACE;
pub const FLAG_CONVERT_ESCAPE = list_quote.FLAG_CONVERT_ESCAPE;
pub const FLAG_DONT_QUOTE_HASH = list_quote.FLAG_DONT_QUOTE_HASH;
pub const FLAG_CONVERT_MASK = list_quote.FLAG_CONVERT_MASK;
pub const scan_element = list_quote.scan_element;
pub const convert_element = list_quote.convert_element;
pub const list_elem_quote = list_quote.list_elem_quote;
pub const list_elem_quote_nth = list_quote.list_elem_quote_nth;
