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

// List-element conversion flags — mirror ``tcl.h`` and ``tclUtil.c``
// ``enum ConvertFlags``.  Returned by :func:`scan_element` and consumed
// by :func:`convert_element`.
pub const FLAG_CONVERT_NONE: u8 = 0;
pub const FLAG_DONT_USE_BRACES: u8 = 1; // TCL_DONT_USE_BRACES
pub const FLAG_CONVERT_BRACE: u8 = 2;
pub const FLAG_CONVERT_ESCAPE: u8 = 4;
pub const FLAG_DONT_QUOTE_HASH: u8 = 8; // TCL_DONT_QUOTE_HASH
pub const FLAG_CONVERT_MASK: u8 = FLAG_CONVERT_BRACE | FLAG_CONVERT_ESCAPE;

/// Port of ``TclScanElement`` (tclUtil.c, Tcl 9.0, COMPAT=1).
///
/// Classifies *src* and returns the conversion mode needed for
/// :func:`convert_element` to emit a valid list element.  The caller
/// may pass :data:`FLAG_DONT_QUOTE_HASH` to suppress the leading-``#``
/// quoting rule (used for all but the first element when a list object
/// re-renders its string representation).
///
/// Returns only the chosen ``CONVERT_*`` flag bits OR'd with any of the
/// ``DONT_*`` bits the caller supplied.  The byte-count tracking used
/// by the C version for buffer sizing is omitted: callers in this
/// runtime allocate worst-case buffers.
pub fn scan_element(src_ptr: u32, len: u32, flag_in: u8) u8 {
    if (len == 0) {
        return (flag_in & FLAG_DONT_QUOTE_HASH) | FLAG_CONVERT_BRACE;
    }
    const src: [*]const u8 = @ptrFromInt(src_ptr);
    var forbid_none: bool = false;
    var require_escape: bool = false;
    // COMPAT preferences:
    var prefer_escape: bool = false;
    var prefer_brace: bool = false;
    var nesting: i32 = 0;

    // Leading-{ or leading-" forces some form of quoting.
    if (src[0] == '{' or src[0] == '"') {
        forbid_none = true;
        prefer_brace = true;
    }
    // Leading-# forces brace-preference unless the caller opts out.
    if (src[0] == '#' and (flag_in & FLAG_DONT_QUOTE_HASH) == 0) {
        prefer_brace = true;
    }

    var i: u32 = 0;
    while (i < len) : (i += 1) {
        const ch = src[i];
        switch (ch) {
            '{' => {
                nesting += 1;
            },
            '}' => {
                nesting -= 1;
                if (nesting < 0) require_escape = true;
            },
            ']', '"' => {
                forbid_none = true;
                prefer_escape = true;
            },
            '[', '$', ';' => {
                forbid_none = true;
                prefer_brace = true;
            },
            '\\' => {
                if (i + 1 >= len) {
                    // Trailing ``\`` — cannot brace-quote, would escape the close.
                    require_escape = true;
                } else if (src[i + 1] == '\n') {
                    // ``\<newline>`` collapses to space via subst; brace form
                    // is forbidden (would be re-parsed as literal).
                    require_escape = true;
                    i += 1;
                } else if (src[i + 1] == '{' or src[i + 1] == '}' or src[i + 1] == '\\') {
                    // ``\{`` / ``\}`` / ``\\`` — consume as a pair, do NOT
                    // credit the inner brace toward nesting.
                    i += 1;
                }
                forbid_none = true;
                prefer_brace = true;
            },
            else => {
                if (is_scan_space(ch)) {
                    forbid_none = true;
                    prefer_brace = true;
                }
            },
        }
    }
    if (nesting > 0) require_escape = true;

    const out_hash = flag_in & FLAG_DONT_QUOTE_HASH;
    if (require_escape) return out_hash | FLAG_CONVERT_ESCAPE;
    if (forbid_none) {
        if (prefer_escape and !prefer_brace) {
            // COMPAT "mask" mode — escape every special char EXCEPT braces.
            return out_hash | FLAG_CONVERT_MASK;
        }
        return out_hash | FLAG_CONVERT_BRACE;
    }
    return out_hash | FLAG_CONVERT_NONE;
}

/// Port of ``TclConvertElement`` (tclUtil.c, Tcl 9.0, COMPAT=1).  Writes
/// the list-element representation of ``src[0..len]`` to ``dst``.  The
/// ``flags`` argument must come from :func:`scan_element` (possibly with
/// ``FLAG_DONT_USE_BRACES`` / ``FLAG_DONT_QUOTE_HASH`` added by the caller).
///
/// Returns the number of bytes written.  ``dst`` must have capacity for
/// the worst case — callers should size for ``2 * len + 2``.
pub fn convert_element(src_ptr: u32, len_in: u32, dst_base: u32, flags_in: u8) u32 {
    const flags = flags_in;
    var conversion = flags & FLAG_CONVERT_MASK;
    // DONT_USE_BRACES + any BRACE bit → downgrade to ESCAPE.
    if ((flags & FLAG_DONT_USE_BRACES) != 0 and (conversion & FLAG_CONVERT_BRACE) != 0) {
        conversion = FLAG_CONVERT_ESCAPE;
    }

    // Empty string is always ``{}``.
    if (len_in == 0) {
        const d: [*]u8 = @ptrFromInt(dst_base);
        d[0] = '{'; d[1] = '}';
        return 2;
    }

    const src: [*]const u8 = @ptrFromInt(src_ptr);
    var p: u32 = 0;
    var s: u32 = 0;
    var len: u32 = len_in;

    // Leading-# handling: either escape ``\#`` or switch to brace mode.
    if (src[0] == '#' and (flags & FLAG_DONT_QUOTE_HASH) == 0) {
        if (conversion == FLAG_CONVERT_ESCAPE) {
            const d: [*]u8 = @ptrFromInt(dst_base + p);
            d[0] = '\\'; d[1] = '#';
            p += 2;
            s += 1;
            len -= 1;
        } else {
            conversion = FLAG_CONVERT_BRACE;
        }
    }

    if (conversion == FLAG_CONVERT_NONE) {
        memcpy(dst_base + p, src_ptr + s, len);
        return p + len;
    }

    if (conversion == FLAG_CONVERT_BRACE) {
        var d: [*]u8 = @ptrFromInt(dst_base + p);
        d[0] = '{';
        p += 1;
        memcpy(dst_base + p, src_ptr + s, len);
        p += len;
        d = @ptrFromInt(dst_base + p);
        d[0] = '}';
        return p + 1;
    }

    // CONVERT_ESCAPE or CONVERT_MASK.
    var k: u32 = 0;
    while (k < len) : (k += 1) {
        const ch = src[s + k];
        switch (ch) {
            ']', '[', '$', ';', ' ', '\\', '"' => {
                const d: [*]u8 = @ptrFromInt(dst_base + p);
                d[0] = '\\';
                p += 1;
            },
            '{', '}' => {
                // In CONVERT_MASK, braces are NOT escaped.
                if (conversion == FLAG_CONVERT_ESCAPE) {
                    const d: [*]u8 = @ptrFromInt(dst_base + p);
                    d[0] = '\\';
                    p += 1;
                }
            },
            '\n' => {
                const d: [*]u8 = @ptrFromInt(dst_base + p);
                d[0] = '\\'; d[1] = 'n';
                p += 2;
                continue;
            },
            '\t' => {
                const d: [*]u8 = @ptrFromInt(dst_base + p);
                d[0] = '\\'; d[1] = 't';
                p += 2;
                continue;
            },
            '\r' => {
                const d: [*]u8 = @ptrFromInt(dst_base + p);
                d[0] = '\\'; d[1] = 'r';
                p += 2;
                continue;
            },
            0x0B => { // \v
                const d: [*]u8 = @ptrFromInt(dst_base + p);
                d[0] = '\\'; d[1] = 'v';
                p += 2;
                continue;
            },
            0x0C => { // \f
                const d: [*]u8 = @ptrFromInt(dst_base + p);
                d[0] = '\\'; d[1] = 'f';
                p += 2;
                continue;
            },
            else => {},
        }
        const d: [*]u8 = @ptrFromInt(dst_base + p);
        d[0] = ch;
        p += 1;
    }
    return p;
}

/// Append *src* to *buf* at *off* as a canonical list element (flag=0 —
/// first-element mode).  Returns the new offset.  Worst-case expansion is
/// ``2 * len + 2`` bytes; callers must size their buffer accordingly.
pub fn list_elem_quote(buf: u32, off: u32, ptr: u32, len: u32) u32 {
    const flags = scan_element(ptr, len, 0);
    return off + convert_element(ptr, len, buf + off, flags);
}

/// Non-first-element variant: ``FLAG_DONT_QUOTE_HASH`` — a leading ``#``
/// is NOT braced / escaped.  Used by list-builders for every element
/// after index 0, matching ``UpdateStringOfList``.
pub fn list_elem_quote_nth(buf: u32, off: u32, ptr: u32, len: u32) u32 {
    const flags = scan_element(ptr, len, FLAG_DONT_QUOTE_HASH);
    return off + convert_element(ptr, len, buf + off, flags);
}
