// Tcl value runtime for WASM.
//
// Provides reference-counted TclObj values in linear memory, with
// exports that the compiled Tcl WASM modules import.  Built as
// wasm32-wasi for WASI I/O support (fd_write for puts).
//
// Memory layout of a TclObj:
//   offset 0: refcount  (i32)
//   offset 4: type_tag  (i32)  0=string, 1=int, 2=list
//   offset 8: int_cache (i64)  cached integer representation
//   offset 16: str_ptr  (i32)  pointer to UTF-8 data in linear memory
//   offset 20: str_len  (i32)  byte length of the string representation
//   Total: 24 bytes per TclObj
//
// All command runtime parameters and results are i32 TclObj pointers.
// The TclObj lifecycle exports (obj_new_int, obj_new_string, obj_get_int)
// bridge between raw i64 integers used in expr arithmetic and i32 object
// pointers.

const std = @import("std");

// Type tags
const TYPE_STRING: i32 = 0;
const TYPE_INT: i32 = 1;
const TYPE_LIST: i32 = 2;
const TYPE_DICT: i32 = 3;

// Simple bump allocator over WASM linear memory with free-list recycling.
// Starts at page 1 (64KiB offset) to avoid collisions with the
// data segment placed at offset 0 by the compiled module.
// Freed TclObjs are placed on a singly-linked free list and recycled
// before bumping the heap pointer.
var heap_ptr: u32 = 65536;
var free_list: u32 = 0; // head of free TclObj list (0 = empty)

fn alloc(size: u32) callconv(.C) u32 {
    const aligned = (size + 7) & ~@as(u32, 7); // 8-byte alignment
    // Recycle from free list if the request is for a TclObj (24 bytes)
    if (aligned == OBJ_SIZE and free_list != 0) {
        const ptr = free_list;
        // The first 4 bytes of a freed TclObj store the next-pointer
        free_list = @intCast(read_i32(ptr));
        return ptr;
    }
    const ptr = heap_ptr;
    heap_ptr += aligned;
    return ptr;
}

fn free_obj(addr: u32) void {
    // Push onto the free list — store the current head in the first 4 bytes
    write_i32(addr, @intCast(free_list));
    free_list = addr;
}

// Read an i32 from linear memory.
// Assigns to a local first so @bitCast can infer the result type.
fn read_i32(addr: u32) i32 {
    const ptr: [*]const u8 = @ptrFromInt(addr);
    const bytes = [4]u8{ ptr[0], ptr[1], ptr[2], ptr[3] };
    return @bitCast(bytes);
}

// Write an i32 to linear memory
fn write_i32(addr: u32, val: i32) void {
    const ptr: [*]u8 = @ptrFromInt(addr);
    const bytes: [4]u8 = @bitCast(val);
    ptr[0] = bytes[0];
    ptr[1] = bytes[1];
    ptr[2] = bytes[2];
    ptr[3] = bytes[3];
}

// Read an i64 from linear memory.
fn read_i64(addr: u32) i64 {
    const ptr: [*]const u8 = @ptrFromInt(addr);
    const bytes = [8]u8{
        ptr[0], ptr[1], ptr[2], ptr[3],
        ptr[4], ptr[5], ptr[6], ptr[7],
    };
    return @bitCast(bytes);
}

// Write an i64 to linear memory
fn write_i64(addr: u32, val: i64) void {
    const ptr: [*]u8 = @ptrFromInt(addr);
    const bytes: [8]u8 = @bitCast(val);
    inline for (0..8) |i| {
        ptr[i] = bytes[i];
    }
}

// TclObj field offsets
const OBJ_REFCOUNT: u32 = 0;
const OBJ_TYPE_TAG: u32 = 4;
const OBJ_INT_CACHE: u32 = 8;
const OBJ_STR_PTR: u32 = 16;
const OBJ_STR_LEN: u32 = 20;
const OBJ_SIZE: u32 = 24;

// Allocate a new TclObj with refcount 1
fn obj_alloc() u32 {
    const ptr = alloc(OBJ_SIZE);
    write_i32(ptr + OBJ_REFCOUNT, 1);
    write_i32(ptr + OBJ_TYPE_TAG, TYPE_STRING);
    write_i64(ptr + OBJ_INT_CACHE, 0);
    write_i32(ptr + OBJ_STR_PTR, 0);
    write_i32(ptr + OBJ_STR_LEN, 0);
    return ptr;
}

// Exported: create a new integer TclObj.
// Export name matches wasm.py _RUNTIME_IMPORTS["tcl_obj_new_int"] → "obj_new_int".
export fn obj_new_int(value: i64) i32 {
    const ptr = obj_alloc();
    write_i32(ptr + OBJ_TYPE_TAG, TYPE_INT);
    write_i64(ptr + OBJ_INT_CACHE, value);
    return @as(i32, @intCast(ptr));
}

// Exported: create a new string TclObj from a data-segment pointer + length.
// Export name matches wasm.py _RUNTIME_IMPORTS["tcl_obj_new_string"] → "obj_new_string".
export fn obj_new_string(data_ptr: i32, length: i32) i32 {
    const ptr = obj_alloc();
    write_i32(ptr + OBJ_TYPE_TAG, TYPE_STRING);
    write_i32(ptr + OBJ_STR_PTR, data_ptr);
    write_i32(ptr + OBJ_STR_LEN, length);
    return @as(i32, @intCast(ptr));
}

// Exported: get the integer value of a TclObj.
// Export name matches wasm.py _RUNTIME_IMPORTS["tcl_obj_get_int"] → "obj_get_int".
// Implements Tcl's shimmer semantics: if the object is a string, try to
// parse it as an integer and cache the result.
export fn obj_get_int(obj: i32) i64 {
    const addr: u32 = @intCast(obj);
    const tag = read_i32(addr + OBJ_TYPE_TAG);
    if (tag == TYPE_INT) return read_i64(addr + OBJ_INT_CACHE);
    // String → try to parse as integer (shimmer)
    if (tag == TYPE_STRING) {
        const sptr: u32 = @intCast(read_i32(addr + OBJ_STR_PTR));
        const slen: u32 = @intCast(read_i32(addr + OBJ_STR_LEN));
        if (try_parse_int(sptr, slen)) |val| {
            // Cache the parsed integer (shimmer)
            write_i32(addr + OBJ_TYPE_TAG, TYPE_INT);
            write_i64(addr + OBJ_INT_CACHE, val);
            return val;
        }
    }
    return read_i64(addr + OBJ_INT_CACHE);
}

// Try to parse a decimal integer from bytes in linear memory.
// Handles optional leading whitespace, sign, and trailing whitespace.
fn try_parse_int(ptr: u32, len: u32) ?i64 {
    if (len == 0) return null;
    const src: [*]const u8 = @ptrFromInt(ptr);
    var i: u32 = 0;
    // Skip leading whitespace
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
    // Must have at least one digit
    if (src[i] < '0' or src[i] > '9') return null;
    var val: i64 = 0;
    while (i < len and src[i] >= '0' and src[i] <= '9') {
        val = val * 10 + @as(i64, src[i] - '0');
        i += 1;
    }
    // Skip trailing whitespace
    while (i < len and is_space(src[i])) i += 1;
    if (i != len) return null; // trailing non-space chars
    return if (negative) -val else val;
}

// Exported: increment refcount
export fn tcl_obj_retain(obj: i32) void {
    const addr: u32 = @intCast(obj);
    const rc = read_i32(addr + OBJ_REFCOUNT);
    write_i32(addr + OBJ_REFCOUNT, rc + 1);
}

// Exported: decrement refcount — free TclObj when it hits zero.
export fn tcl_obj_release(obj: i32) void {
    if (obj == 0) return;
    const addr: u32 = @intCast(obj);
    const rc = read_i32(addr + OBJ_REFCOUNT);
    if (rc <= 1) {
        // Recycle the TclObj slot
        free_obj(addr);
    } else {
        write_i32(addr + OBJ_REFCOUNT, rc - 1);
    }
}

// Exported: variable set (identity — pass-through TclObj pointer)
export fn tcl_var_set(value: i32) i32 {
    return value;
}

// Exported: variable get (identity — pass-through TclObj pointer)
export fn tcl_var_get(value: i32) i32 {
    return value;
}

// -- Global variable table --
// Open-addressing hash table with linear probing.  Starts at 16 buckets,
// doubles when load factor exceeds 75%.  No hard limit — grows until
// memory runs out, matching C Tcl's Tcl_HashTable semantics.
//
// Each bucket is 16 bytes:
//   name_ptr (i32)  — 0 means empty slot
//   name_len (i32)
//   hash     (i32)  — cached FNV-1a hash of the name
//   value    (i32)  — TclObj pointer
const HTAB_BUCKET_SIZE: u32 = 16;
const HTAB_INITIAL_CAP: u32 = 16;

var htab_buf: u32 = 0;
var htab_cap: u32 = 0;
var htab_count: u32 = 0;

fn htab_init() void {
    if (htab_buf != 0) return;
    htab_cap = HTAB_INITIAL_CAP;
    htab_buf = alloc(htab_cap * HTAB_BUCKET_SIZE);
    // Zero all name_ptr fields to mark slots as empty
    var i: u32 = 0;
    while (i < htab_cap) : (i += 1) {
        write_i32(htab_buf + i * HTAB_BUCKET_SIZE, 0);
    }
}

fn fnv1a(ptr: u32, len: u32) u32 {
    const src: [*]const u8 = @ptrFromInt(ptr);
    var h: u32 = 2166136261;
    for (0..len) |i| {
        h ^= @as(u32, src[i]);
        h *%= 16777619;
    }
    return h;
}

fn htab_find(name_ptr: u32, name_len: u32, hash: u32) ?u32 {
    if (htab_buf == 0) return null;
    const mask = htab_cap - 1;
    var idx = hash & mask;
    var probes: u32 = 0;
    while (probes < htab_cap) : (probes += 1) {
        const base = htab_buf + idx * HTAB_BUCKET_SIZE;
        const ep: u32 = @intCast(read_i32(base));
        if (ep == 0) return null; // empty slot — not found
        const el: u32 = @intCast(read_i32(base + 4));
        const eh: u32 = @intCast(read_i32(base + 8));
        if (eh == hash and el == name_len) {
            const sp: [*]const u8 = @ptrFromInt(ep);
            const np: [*]const u8 = @ptrFromInt(name_ptr);
            var match = true;
            for (0..el) |k| {
                if (sp[k] != np[k]) {
                    match = false;
                    break;
                }
            }
            if (match) return base;
        }
        idx = (idx + 1) & mask;
    }
    return null;
}

fn htab_insert(name_ptr: u32, name_len: u32, hash: u32, value: i32) void {
    const mask = htab_cap - 1;
    var idx = hash & mask;
    while (true) {
        const base = htab_buf + idx * HTAB_BUCKET_SIZE;
        const ep: u32 = @intCast(read_i32(base));
        if (ep == 0) {
            // Empty slot — insert here
            const nbuf = alloc(name_len);
            memcpy(nbuf, name_ptr, name_len);
            write_i32(base, @intCast(nbuf));
            write_i32(base + 4, @intCast(name_len));
            write_i32(base + 8, @intCast(hash));
            write_i32(base + 12, value);
            htab_count += 1;
            return;
        }
        idx = (idx + 1) & mask;
    }
}

fn htab_grow() void {
    const old_buf = htab_buf;
    const old_cap = htab_cap;
    htab_cap = old_cap * 2;
    htab_buf = alloc(htab_cap * HTAB_BUCKET_SIZE);
    htab_count = 0;
    // Zero new table
    var i: u32 = 0;
    while (i < htab_cap) : (i += 1) {
        write_i32(htab_buf + i * HTAB_BUCKET_SIZE, 0);
    }
    // Re-insert all entries from old table
    i = 0;
    while (i < old_cap) : (i += 1) {
        const base = old_buf + i * HTAB_BUCKET_SIZE;
        const ep: u32 = @intCast(read_i32(base));
        if (ep != 0) {
            const el: u32 = @intCast(read_i32(base + 4));
            const eh: u32 = @intCast(read_i32(base + 8));
            const ev: i32 = read_i32(base + 12);
            // Insert directly using the existing heap-allocated name
            const mask = htab_cap - 1;
            var idx = eh & mask;
            while (true) {
                const nb = htab_buf + idx * HTAB_BUCKET_SIZE;
                if (@as(u32, @intCast(read_i32(nb))) == 0) {
                    write_i32(nb, @intCast(ep));
                    write_i32(nb + 4, @intCast(el));
                    write_i32(nb + 8, @intCast(eh));
                    write_i32(nb + 12, ev);
                    htab_count += 1;
                    break;
                }
                idx = (idx + 1) & mask;
            }
        }
    }
}

// Exported: set a global variable by name.
export fn global_set(name: i32, value: i32) i32 {
    const sn = obj_ensure_string(name);
    htab_init();
    const hash = fnv1a(sn.ptr, sn.len);
    if (htab_find(sn.ptr, sn.len, hash)) |base| {
        write_i32(base + 12, value);
        return value;
    }
    // Check load factor — grow at 75%
    if (htab_count * 4 >= htab_cap * 3) {
        htab_grow();
    }
    htab_insert(sn.ptr, sn.len, hash, value);
    return value;
}

// Exported: get a global variable by name. Returns 0 (null) if not found.
export fn global_get(name: i32) i32 {
    const sn = obj_ensure_string(name);
    if (htab_buf == 0) return 0;
    const hash = fnv1a(sn.ptr, sn.len);
    if (htab_find(sn.ptr, sn.len, hash)) |base| {
        return read_i32(base + 12);
    }
    return 0;
}

// Exported: check if a global variable exists. Returns 1 or 0.
export fn global_exists(name: i32) i32 {
    const sn = obj_ensure_string(name);
    if (htab_buf == 0) return obj_new_int(0);
    const hash = fnv1a(sn.ptr, sn.len);
    if (htab_find(sn.ptr, sn.len, hash) != null) {
        return obj_new_int(1);
    }
    return obj_new_int(0);
}

// Exported: increment an integer TclObj, returning a new TclObj
export fn tcl_incr(obj: i32, amount: i32) i32 {
    const val = obj_get_int(obj);
    const amt = obj_get_int(amount);
    return obj_new_int(val + amt);
}

// -- TclObj string helpers --

fn obj_str_ptr(obj: i32) u32 {
    const addr: u32 = @intCast(obj);
    return @intCast(read_i32(addr + OBJ_STR_PTR));
}

fn obj_str_len(obj: i32) u32 {
    const addr: u32 = @intCast(obj);
    return @intCast(read_i32(addr + OBJ_STR_LEN));
}

fn obj_type(obj: i32) i32 {
    const addr: u32 = @intCast(obj);
    return read_i32(addr + OBJ_TYPE_TAG);
}

/// Copy *len* bytes from src to dst in linear memory.
fn memcpy(dst: u32, src: u32, len: u32) void {
    const d: [*]u8 = @ptrFromInt(dst);
    const s: [*]const u8 = @ptrFromInt(src);
    for (0..len) |i| {
        d[i] = s[i];
    }
}

/// Create a new string TclObj by copying *len* bytes from *src*.
fn obj_new_string_copy(src: u32, len: u32) i32 {
    const buf = alloc(len);
    memcpy(buf, src, len);
    return obj_new_string(@intCast(buf), @intCast(len));
}

/// Render an integer TclObj to its string representation.
/// Returns (ptr, len) for the rendered decimal string.
fn obj_ensure_string(obj: i32) struct { ptr: u32, len: u32 } {
    if (obj == 0) return .{ .ptr = 0, .len = 0 };
    const addr: u32 = @intCast(obj);
    const tag = read_i32(addr + OBJ_TYPE_TAG);
    if (tag == TYPE_STRING) {
        return .{
            .ptr = @intCast(read_i32(addr + OBJ_STR_PTR)),
            .len = @intCast(read_i32(addr + OBJ_STR_LEN)),
        };
    }
    // Integer → render to string and cache a heap copy.
    // This avoids the scratch-buffer aliasing issue when two integer
    // TclObjs are compared (e.g. string equal).
    const sptr: u32 = @intCast(read_i32(addr + OBJ_STR_PTR));
    if (sptr != 0) {
        // Already rendered — return the cached string
        return .{
            .ptr = sptr,
            .len = @intCast(read_i32(addr + OBJ_STR_LEN)),
        };
    }
    const val = read_i64(addr + OBJ_INT_CACHE);
    const result = itoa_no_nl(val);
    // Copy into heap memory so the scratch buffer can be reused
    const buf = alloc(result.len);
    memcpy(buf, @intFromPtr(result.ptr), result.len);
    // Cache on the TclObj so future calls skip rendering
    write_i32(addr + OBJ_STR_PTR, @intCast(buf));
    write_i32(addr + OBJ_STR_LEN, @intCast(result.len));
    return .{ .ptr = buf, .len = result.len };
}

// -- WASI I/O helpers --

// Scratch buffer for integer-to-string conversion (max 20 digits + sign + newline)
var itoa_buf: [22]u8 = undefined;

fn itoa(value: i64) struct { ptr: [*]u8, len: u32 } {
    var v = value;
    var negative = false;
    if (v < 0) {
        negative = true;
        v = -v;
    }
    var i: u32 = itoa_buf.len - 1;
    // Add trailing newline (puts appends newline in Tcl)
    itoa_buf[i] = '\n';
    i -= 1;
    if (v == 0) {
        itoa_buf[i] = '0';
    } else {
        while (v > 0) {
            itoa_buf[i] = @as(u8, @intCast(@rem(v, 10))) + '0';
            v = @divTrunc(v, 10);
            if (v > 0) i -= 1;
        }
    }
    if (negative) {
        i -= 1;
        itoa_buf[i] = '-';
    }
    return .{ .ptr = @as([*]u8, &itoa_buf) + i, .len = itoa_buf.len - i };
}

// Second scratch buffer for itoa_no_nl (avoids clobbering itoa_buf)
var itoa_buf2: [21]u8 = undefined;

fn itoa_no_nl(value: i64) struct { ptr: [*]u8, len: u32 } {
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

fn fd_write_all(fd: i32, data: [*]const u8, len: u32) void {
    const iov = [_]std.os.wasi.ciovec_t{.{
        .base = data,
        .len = len,
    }};
    var written: usize = 0;
    _ = std.os.wasi.fd_write(@intCast(fd), &iov, 1, &written);
}

// -- Command runtime --
// All parameters and results are i32 TclObj pointers.

// Exported: puts — write value to stdout via WASI fd_write.
// For integer TclObj: renders the integer as decimal string + newline.
// For string TclObj: writes the string data + newline.
export fn puts(value: i32) i32 {
    if (value == 0) {
        // Null TclObj — write empty line
        fd_write_all(1, "\n", 1);
        return 0;
    }
    const addr: u32 = @intCast(value);
    const tag = read_i32(addr + OBJ_TYPE_TAG);
    if (tag == TYPE_STRING) {
        const sptr = read_i32(addr + OBJ_STR_PTR);
        const slen = read_i32(addr + OBJ_STR_LEN);
        if (slen > 0) {
            fd_write_all(1, @ptrFromInt(@as(u32, @intCast(sptr))), @intCast(slen));
        }
        fd_write_all(1, "\n", 1);
    } else {
        // Integer — render as decimal
        const int_val = read_i64(addr + OBJ_INT_CACHE);
        const result = itoa(int_val);
        fd_write_all(1, result.ptr, result.len);
    }
    return 0;
}

// Exported: append — concatenate two TclObj string representations.
export fn append(current: i32, addition: i32) i32 {
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
export fn string_compare(a: i32, b: i32) i32 {
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

// -- List operations --
// Lists are represented as whitespace-separated elements in string form.
// Braced elements {like this} are supported for elements containing spaces.

// Count elements in a Tcl list string.
fn list_count_elements(ptr: u32, len: u32) i64 {
    if (len == 0) return 0;
    const src: [*]const u8 = @ptrFromInt(ptr);
    var count: i64 = 0;
    var i: u32 = 0;
    while (i < len) {
        // Skip whitespace
        while (i < len and is_space(src[i])) i += 1;
        if (i >= len) break;
        count += 1;
        // Skip element
        if (src[i] == '{') {
            i += 1;
            var depth: u32 = 1;
            while (i < len and depth > 0) {
                if (src[i] == '{') {
                    depth += 1;
                } else if (src[i] == '}') {
                    depth -= 1;
                }
                i += 1;
            }
        } else {
            while (i < len and !is_space(src[i])) i += 1;
        }
    }
    return count;
}

// Get the start and length of the nth element (0-based) in a Tcl list.
// Returns (start, length, is_braced).
fn list_element_at(ptr: u32, len: u32, idx: i64) struct { start: u32, len: u32, braced: bool } {
    if (len == 0) return .{ .start = 0, .len = 0, .braced = false };
    const src: [*]const u8 = @ptrFromInt(ptr);
    var count: i64 = 0;
    var i: u32 = 0;
    while (i < len) {
        while (i < len and is_space(src[i])) i += 1;
        if (i >= len) break;
        const elem_start = i;
        if (src[i] == '{') {
            i += 1;
            const inner_start = i;
            var depth: u32 = 1;
            while (i < len and depth > 0) {
                if (src[i] == '{') {
                    depth += 1;
                } else if (src[i] == '}') {
                    depth -= 1;
                }
                i += 1;
            }
            if (count == idx) {
                // Return inner content (without braces)
                return .{ .start = inner_start, .len = i - 1 - inner_start, .braced = true };
            }
        } else {
            while (i < len and !is_space(src[i])) i += 1;
            if (count == idx) {
                return .{ .start = elem_start, .len = i - elem_start, .braced = false };
            }
        }
        count += 1;
    }
    return .{ .start = 0, .len = 0, .braced = false };
}

// Exported: list length — count elements by whitespace-splitting.
export fn list_length(list: i32) i32 {
    const s = obj_ensure_string(list);
    const n = list_count_elements(s.ptr, s.len);
    return obj_new_int(n);
}

// Exported: list append — append element to list (space-separated).
export fn lappend(current: i32, value: i32) i32 {
    const sc = obj_ensure_string(current);
    const sv = obj_ensure_string(value);
    // Check if value needs bracing (contains spaces)
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

// Exported: string length — byte length of the string representation.
export fn string_length(value: i32) i32 {
    const s = obj_ensure_string(value);
    return obj_new_int(@intCast(s.len));
}

// Exported: string index — extract the character at a byte index.
export fn string_index(value: i32, idx: i32) i32 {
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
export fn string_range(value: i32, first: i32, last: i32) i32 {
    const s = obj_ensure_string(value);
    var f = obj_get_int(first);
    var l = obj_get_int(last);
    const slen: i64 = @intCast(s.len);
    // Clamp to valid range
    if (f < 0) f = 0;
    if (l >= slen) l = slen - 1;
    if (f > l or f >= slen) return obj_new_string(0, 0);
    const start: u32 = @intCast(f);
    const count: u32 = @intCast(l - f + 1);
    return obj_new_string_copy(s.ptr + start, count);
}

// Exported: string map
// Exported: string map — apply a mapping list {from to from to ...} to a string.
export fn string_map(mapping: i32, value: i32) i32 {
    const sm = obj_ensure_string(mapping);
    const sv = obj_ensure_string(value);
    if (sv.len == 0) return value;
    // Count mapping pairs
    const n_elems = list_count_elements(sm.ptr, sm.len);
    if (n_elems < 2) return value;
    const n_pairs: u32 = @intCast(@divTrunc(n_elems, 2));
    // Allocate output buffer (worst case: same size as input)
    const buf = alloc(sv.len * 2 + 64);
    const src: [*]const u8 = @ptrFromInt(sv.ptr);
    var out_len: u32 = 0;
    var pos: u32 = 0;
    outer: while (pos < sv.len) {
        // Try each mapping pair
        var pair: u32 = 0;
        while (pair < n_pairs) : (pair += 1) {
            const from = list_element_at(sm.ptr, sm.len, @as(i64, pair) * 2);
            if (from.len == 0) {
                pair += 1;
                continue;
            }
            // Check if src[pos..] starts with the 'from' pattern
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
                    // Copy the 'to' replacement
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
        // No match — copy character as-is
        const dst: [*]u8 = @ptrFromInt(buf + out_len);
        dst[0] = src[pos];
        out_len += 1;
        pos += 1;
    }
    return obj_new_string(@intCast(buf), @intCast(out_len));
}

// Exported: string match — glob pattern matching (* and ? wildcards).
export fn string_match(pattern: i32, value: i32) i32 {
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
export fn string_trim(value: i32) i32 {
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
export fn string_trimleft(value: i32) i32 {
    const s = obj_ensure_string(value);
    if (s.len == 0) return value;
    const src: [*]const u8 = @ptrFromInt(s.ptr);
    var start: u32 = 0;
    while (start < s.len and is_space(src[start])) start += 1;
    if (start == 0) return value;
    return obj_new_string_copy(s.ptr + start, s.len - start);
}

// Exported: string trimright — strip trailing whitespace.
export fn string_trimright(value: i32) i32 {
    const s = obj_ensure_string(value);
    if (s.len == 0) return value;
    const src: [*]const u8 = @ptrFromInt(s.ptr);
    var end: u32 = s.len;
    while (end > 0 and is_space(src[end - 1])) end -= 1;
    if (end == s.len) return value;
    return obj_new_string_copy(s.ptr, end);
}

fn is_space(c: u8) bool {
    return c == ' ' or c == '\t' or c == '\n' or c == '\r';
}

// Exported: list — create a list from individual elements.
export fn tcl_list(a: i32, b: i32) i32 {
    // Two-element list creation. Each element is space-separated.
    // If either element contains spaces, wrap in braces.
    const sa = obj_ensure_string(a);
    const sb = obj_ensure_string(b);
    if (sa.len == 0 and sb.len == 0) return obj_new_string(0, 0);
    if (sa.len == 0) return b;
    if (sb.len == 0) return a;
    // Check if elements need bracing
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

// Exported: string equal — compare two strings for equality (returns 1 or 0).
export fn string_equal(a: i32, b: i32) i32 {
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
export fn string_first(needle: i32, haystack: i32) i32 {
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
export fn string_last(needle: i32, haystack: i32) i32 {
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
export fn string_repeat(value: i32, count: i32) i32 {
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
export fn string_reverse(value: i32) i32 {
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
export fn string_toupper(value: i32) i32 {
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
export fn string_tolower(value: i32) i32 {
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
export fn string_replace(value: i32, first: i32, last: i32, new_str: i32) i32 {
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
export fn string_is_integer(value: i32) i32 {
    const sv = obj_ensure_string(value);
    if (sv.len == 0) return obj_new_int(0);
    if (try_parse_int(sv.ptr, sv.len) != null) {
        return obj_new_int(1);
    }
    return obj_new_int(0);
}

// Exported: string is alpha — check if a string contains only letters.
export fn string_is_alpha(value: i32) i32 {
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
export fn string_is_digit(value: i32) i32 {
    const sv = obj_ensure_string(value);
    if (sv.len == 0) return obj_new_int(0);
    const src: [*]const u8 = @ptrFromInt(sv.ptr);
    for (0..sv.len) |i| {
        if (src[i] < '0' or src[i] > '9') return obj_new_int(0);
    }
    return obj_new_int(1);
}

// Exported: string is space — check if a string contains only whitespace.
export fn string_is_space(value: i32) i32 {
    const sv = obj_ensure_string(value);
    if (sv.len == 0) return obj_new_int(0);
    const src: [*]const u8 = @ptrFromInt(sv.ptr);
    for (0..sv.len) |i| {
        if (!is_space(src[i])) return obj_new_int(0);
    }
    return obj_new_int(1);
}

// Exported: concat — concatenate two TclObj string representations with space.
export fn concat(a: i32, b: i32) i32 {
    const sa = obj_ensure_string(a);
    const sb = obj_ensure_string(b);
    if (sa.len == 0) return b;
    if (sb.len == 0) return a;
    const total = sa.len + 1 + sb.len; // space separator
    const buf = alloc(total);
    memcpy(buf, sa.ptr, sa.len);
    const dst: [*]u8 = @ptrFromInt(buf + sa.len);
    dst[0] = ' ';
    memcpy(buf + sa.len + 1, sb.ptr, sb.len);
    return obj_new_string(@intCast(buf), @intCast(total));
}

// Exported: list index — extract the nth element (0-based).
export fn list_index(list: i32, idx: i32) i32 {
    const s = obj_ensure_string(list);
    const i_val = obj_get_int(idx);
    if (i_val < 0) return obj_new_string(0, 0);
    const n = list_count_elements(s.ptr, s.len);
    if (i_val >= n) return obj_new_string(0, 0);
    const elem = list_element_at(s.ptr, s.len, i_val);
    return obj_new_string_copy(s.ptr + elem.start, elem.len);
}

// Exported: list range — extract elements [first..last] (inclusive).
export fn list_range(list: i32, first: i32, last: i32) i32 {
    const s = obj_ensure_string(list);
    var f = obj_get_int(first);
    var l = obj_get_int(last);
    const total = list_count_elements(s.ptr, s.len);
    if (f < 0) f = 0;
    if (l >= total) l = total - 1;
    if (f > l or f >= total) return obj_new_string(0, 0);
    // Build result by extracting elements
    var result_len: u32 = 0;
    const result_buf: u32 = alloc(s.len); // upper bound
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

// Exported: list sort — simple insertion sort on string comparison.
export fn list_sort(list: i32) i32 {
    const s = obj_ensure_string(list);
    const n_i64 = list_count_elements(s.ptr, s.len);
    if (n_i64 <= 1) return list;
    const n: u32 = @intCast(n_i64);
    // Collect element pointers into a temporary array
    const arr_buf = alloc(n * 8); // 4 bytes start + 4 bytes len per element
    var idx: u32 = 0;
    while (idx < n) : (idx += 1) {
        const elem = list_element_at(s.ptr, s.len, @intCast(idx));
        write_i32(arr_buf + idx * 8, @intCast(s.ptr + elem.start));
        write_i32(arr_buf + idx * 8 + 4, @intCast(elem.len));
    }
    // Insertion sort
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
                // Shift element right
                write_i32(arr_buf + (j_u + 1) * 8, @intCast(cmp_ptr));
                write_i32(arr_buf + (j_u + 1) * 8 + 4, @intCast(cmp_len));
                j -= 1;
            } else break;
        }
        const ins: u32 = @intCast(j + 1);
        write_i32(arr_buf + ins * 8, @intCast(key_ptr));
        write_i32(arr_buf + ins * 8 + 4, @intCast(key_len));
    }
    // Build result string
    var result_len: u32 = 0;
    const result_buf = alloc(s.len + n); // upper bound
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

// Compare two byte spans lexicographically. Returns <0, 0, or >0.
fn str_cmp(a_ptr: u32, a_len: u32, b_ptr: u32, b_len: u32) i32 {
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

// Exported: list search — linear search for exact match, returns index or -1.
export fn list_search(list: i32, value: i32) i32 {
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

// -- Dict operations --
// Dicts are stored as lists of alternating key-value pairs in string form.
// "key1 val1 key2 val2 ..."

// Exported: dict create — create an empty dict.
export fn dict_create() i32 {
    return obj_new_string(0, 0);
}

// Exported: dict get — look up a key in a dict, return its value.
export fn dict_get(dict: i32, key: i32) i32 {
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
    // Key not found — return empty string
    return obj_new_string(0, 0);
}

// Exported: dict set — set a key in a dict, return the updated dict.
export fn dict_set(dict: i32, key: i32, value: i32) i32 {
    const sd = obj_ensure_string(dict);
    const sk = obj_ensure_string(key);
    const sv = obj_ensure_string(value);
    const n = list_count_elements(sd.ptr, sd.len);
    // Check if key exists — if so, rebuild with updated value
    var idx: i64 = 0;
    while (idx < n - 1) : (idx += 2) {
        const k = list_element_at(sd.ptr, sd.len, idx);
        if (str_cmp(sd.ptr + k.start, k.len, sk.ptr, sk.len) == 0) {
            // Found — rebuild dict with new value at this position
            return dict_rebuild_with_value(sd.ptr, sd.len, n, idx, sv.ptr, sv.len);
        }
    }
    // Key not found — append key value pair
    return dict_append_pair(sd.ptr, sd.len, sk.ptr, sk.len, sv.ptr, sv.len);
}

fn dict_needs_braces(ptr: u32, len: u32) bool {
    if (len == 0) return true; // empty value needs braces
    const src: [*]const u8 = @ptrFromInt(ptr);
    for (0..len) |i| {
        if (is_space(src[i])) return true;
    }
    return false;
}

fn dict_append_elem(buf: u32, offset: u32, ptr: u32, len: u32) u32 {
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

fn dict_rebuild_with_value(sd_ptr: u32, sd_len: u32, n: i64, target_idx: i64, vp: u32, vl: u32) i32 {
    // Upper bound: original length + new value + braces + spaces
    const buf = alloc(sd_len + vl + 10);
    var off: u32 = 0;
    var idx: i64 = 0;
    while (idx < n) : (idx += 1) {
        if (off > 0) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        if (idx == target_idx + 1) {
            // Replace value
            off = dict_append_elem(buf, off, vp, vl);
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
    // Estimate size: existing + space + key + space + value + braces
    const buf = alloc(sd_len + kl + vl + 10);
    var off: u32 = 0;
    if (sd_len > 0) {
        memcpy(buf, sd_ptr, sd_len);
        off = sd_len;
        const d: [*]u8 = @ptrFromInt(buf + off);
        d[0] = ' ';
        off += 1;
    }
    off = dict_append_elem(buf, off, kp, kl);
    const d: [*]u8 = @ptrFromInt(buf + off);
    d[0] = ' ';
    off += 1;
    off = dict_append_elem(buf, off, vp, vl);
    return obj_new_string(@intCast(buf), @intCast(off));
}

// Exported: dict exists — check if key exists in dict, return 1 or 0.
export fn dict_exists(dict: i32, key: i32) i32 {
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
export fn dict_keys(dict: i32) i32 {
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
export fn dict_values(dict: i32) i32 {
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
export fn dict_size(dict: i32) i32 {
    const sd = obj_ensure_string(dict);
    const n = list_count_elements(sd.ptr, sd.len);
    return obj_new_int(@divTrunc(n, 2));
}

// -- Error handling for catch --
// When catch_depth > 0, error() sets a flag instead of trapping.
// The compiled catch block checks the flag after the body executes.
var catch_depth: u32 = 0;
var error_flag: u32 = 0; // 0 = no error, 1 = error pending
var error_msg: i32 = 0; // TclObj with error message

// Exported: enter a catch scope.
export fn catch_enter() void {
    catch_depth += 1;
    error_flag = 0;
    error_msg = 0;
}

// Exported: leave a catch scope. Returns 0 (TCL_OK) or 1 (TCL_ERROR).
export fn catch_leave() i32 {
    if (catch_depth > 0) catch_depth -= 1;
    const had_error = error_flag;
    error_flag = 0;
    return obj_new_int(@intCast(had_error));
}

// Exported: get the error message (or last result) after catch.
export fn catch_result() i32 {
    return error_msg;
}

// Exported: check if an error is pending (for early exit from catch body).
export fn catch_has_error() i32 {
    return @as(i32, @intCast(error_flag));
}

// Exported: error — write message to stderr and trap, OR set error flag in catch.
export fn @"error"(msg: i32) void {
    if (catch_depth > 0) {
        // Inside catch — set flag, don't trap
        error_flag = 1;
        error_msg = msg;
        return;
    }
    // Not inside catch — hard trap
    const s = obj_ensure_string(msg);
    if (s.len > 0) {
        fd_write_all(2, @ptrFromInt(s.ptr), s.len);
        fd_write_all(2, "\n", 1);
    }
    @trap();
}

// Exported: format
export fn format(fmt: i32, value: i32) i32 {
    _ = fmt;
    return value;
}

// Exported: regexp
export fn regexp(pattern: i32, str: i32) i32 {
    _ = pattern;
    _ = str;
    return obj_new_int(0);
}

// Exported: open
export fn open(path: i32) i32 {
    _ = path;
    return obj_new_int(-1); // no file I/O in freestanding
}

// Exported: close
export fn close(fd: i32) i32 {
    _ = fd;
    return obj_new_int(0);
}

// Exported: read
export fn read(fd: i32) i32 {
    _ = fd;
    return obj_new_int(0);
}

// Exported: gets
export fn gets(fd: i32) i32 {
    _ = fd;
    return obj_new_int(0);
}

// ============================================================
// Minimal Tcl interpreter — tokeniser + eval loop.
//
// Provides tcl_eval(script) for the compiled WASM code to fall
// back to when it encounters constructs that can't be statically
// compiled.  Shares all existing runtime (TclObj, string ops,
// list ops, dict ops, globals).
//
// Design: parse one command at a time, split into words, look
// up the command in a static dispatch table, call the handler.
// Expressions are evaluated via a simple recursive-descent parser.
// ============================================================

// -- Tokeniser --
// Splits a Tcl script into commands, each command into words.
// Handles: braces {}, quotes "", $var substitution, [cmd] substitution,
// backslash escapes, semicolons, newlines.

const MAX_WORDS: u32 = 32;

fn skip_space(src: [*]const u8, pos: u32, len: u32) u32 {
    var p = pos;
    while (p < len and (src[p] == ' ' or src[p] == '\t')) p += 1;
    return p;
}

// Parse a braced word {....}. Returns end position (after closing brace).
fn parse_braced(src: [*]const u8, pos: u32, len: u32) struct { end: u32, start: u32, wlen: u32 } {
    // pos points at '{'
    var p = pos + 1;
    const start = p;
    var depth: u32 = 1;
    while (p < len and depth > 0) {
        if (src[p] == '{') depth += 1 else if (src[p] == '}') depth -= 1;
        if (depth > 0) p += 1 else p += 1;
    }
    return .{ .end = p, .start = start, .wlen = p - 1 - start };
}

// Parse a quoted word "....". Returns end position (after closing quote).
fn parse_quoted(src: [*]const u8, pos: u32, len: u32) struct { end: u32, start: u32, wlen: u32 } {
    var p = pos + 1;
    const start = p;
    while (p < len and src[p] != '"') {
        if (src[p] == '\\' and p + 1 < len) p += 2 else p += 1;
    }
    const wlen = p - start;
    if (p < len) p += 1; // skip closing quote
    return .{ .end = p, .start = start, .wlen = wlen };
}

// Parse a bare word. Returns end position.
fn parse_bare(src: [*]const u8, pos: u32, len: u32) struct { end: u32, start: u32, wlen: u32 } {
    const start = pos;
    var p = pos;
    while (p < len and src[p] != ' ' and src[p] != '\t' and
        src[p] != '\n' and src[p] != ';' and src[p] != '\r')
    {
        if (src[p] == '\\' and p + 1 < len) p += 2 else p += 1;
    }
    return .{ .end = p, .start = start, .wlen = p - start };
}

// Parse one command from a script. Returns (word_ptrs, word_lens, word_count, next_pos).
fn parse_command(
    src: [*]const u8,
    pos: u32,
    len: u32,
    word_ptrs: *[MAX_WORDS]u32,
    word_lens: *[MAX_WORDS]u32,
) struct { count: u32, next: u32 } {
    var p = pos;
    var count: u32 = 0;

    // Skip leading whitespace and empty lines/semicolons
    while (p < len and (src[p] == ' ' or src[p] == '\t' or src[p] == '\n' or src[p] == '\r' or src[p] == ';')) p += 1;

    // Skip comment lines
    if (p < len and src[p] == '#') {
        while (p < len and src[p] != '\n') p += 1;
        if (p < len) p += 1;
        return .{ .count = 0, .next = p };
    }

    while (p < len and count < MAX_WORDS) {
        p = skip_space(src, p, len);
        if (p >= len or src[p] == '\n' or src[p] == ';' or src[p] == '\r') {
            if (p < len) p += 1;
            break;
        }
        if (src[p] == '#' and count == 0) {
            while (p < len and src[p] != '\n') p += 1;
            if (p < len) p += 1;
            break;
        }

        if (src[p] == '{') {
            const r = parse_braced(src, p, len);
            word_ptrs[count] = @intFromPtr(src) + r.start;
            word_lens[count] = r.wlen;
            count += 1;
            p = r.end;
        } else if (src[p] == '"') {
            const r = parse_quoted(src, p, len);
            word_ptrs[count] = @intFromPtr(src) + r.start;
            word_lens[count] = r.wlen;
            count += 1;
            p = r.end;
        } else {
            const r = parse_bare(src, p, len);
            word_ptrs[count] = @intFromPtr(src) + r.start;
            word_lens[count] = r.wlen;
            count += 1;
            p = r.end;
        }
    }
    return .{ .count = count, .next = p };
}

// -- Variable substitution --
// Expand $var references in a word, returning a new TclObj.
fn subst_word(wptr: u32, wlen: u32) i32 {
    if (wlen == 0) return obj_new_string(0, 0);
    const src: [*]const u8 = @ptrFromInt(wptr);
    // Quick check — no substitution needed?
    var has_dollar = false;
    var has_bracket = false;
    for (0..wlen) |i| {
        if (src[i] == '$') has_dollar = true;
        if (src[i] == '[') has_bracket = true;
    }
    if (!has_dollar and !has_bracket) {
        return obj_new_string(@intCast(wptr), @intCast(wlen));
    }
    // Build result with substitution
    const buf = alloc(wlen * 4 + 64); // generous buffer
    var out: u32 = 0;
    var i: u32 = 0;
    while (i < wlen) {
        if (src[i] == '$' and i + 1 < wlen) {
            i += 1;
            // Read variable name
            const vstart = i;
            if (src[i] == '{') {
                // ${varname}
                i += 1;
                const vs = i;
                while (i < wlen and src[i] != '}') i += 1;
                const ve = i;
                if (i < wlen) i += 1;
                const name_obj = obj_new_string(@intCast(wptr + vs), @intCast(ve - vs));
                const val = global_get(name_obj);
                if (val != 0) {
                    const sv = obj_ensure_string(val);
                    if (sv.len > 0) {
                        memcpy(buf + out, sv.ptr, sv.len);
                        out += sv.len;
                    }
                }
            } else {
                // $varname — alphanumeric + underscore
                while (i < wlen and ((src[i] >= 'a' and src[i] <= 'z') or
                    (src[i] >= 'A' and src[i] <= 'Z') or
                    (src[i] >= '0' and src[i] <= '9') or src[i] == '_'))
                {
                    i += 1;
                }
                const name_obj = obj_new_string(@intCast(wptr + vstart), @intCast(i - vstart));
                const val = global_get(name_obj);
                if (val != 0) {
                    const sv = obj_ensure_string(val);
                    if (sv.len > 0) {
                        memcpy(buf + out, sv.ptr, sv.len);
                        out += sv.len;
                    }
                }
            }
        } else if (src[i] == '[') {
            // Command substitution [cmd args...]
            i += 1;
            const cs = i;
            var depth: u32 = 1;
            while (i < wlen and depth > 0) {
                if (src[i] == '[') depth += 1 else if (src[i] == ']') depth -= 1;
                if (depth > 0) i += 1 else i += 1;
            }
            const ce = i - 1;
            // Recursively evaluate
            const result = eval_script(wptr + cs, ce - cs);
            if (result != 0) {
                const sv = obj_ensure_string(result);
                if (sv.len > 0) {
                    memcpy(buf + out, sv.ptr, sv.len);
                    out += sv.len;
                }
            }
        } else if (src[i] == '\\' and i + 1 < wlen) {
            // Backslash substitution
            i += 1;
            const c = src[i];
            const esc: u8 = switch (c) {
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                else => c,
            };
            const d: [*]u8 = @ptrFromInt(buf + out);
            d[0] = esc;
            out += 1;
            i += 1;
        } else {
            const d: [*]u8 = @ptrFromInt(buf + out);
            d[0] = src[i];
            out += 1;
            i += 1;
        }
    }
    return obj_new_string(@intCast(buf), @intCast(out));
}

// -- Expression evaluator --
// Simple recursive-descent for integer expressions: +, -, *, /, %, ==, !=, <, >, <=, >=
fn eval_expr_str(ptr: u32, len: u32) i64 {
    var pos: u32 = 0;
    return expr_add(ptr, len, &pos);
}

fn expr_skip_ws(src: [*]const u8, len: u32, pos: *u32) void {
    while (pos.* < len and (src[pos.*] == ' ' or src[pos.*] == '\t')) pos.* += 1;
}

fn expr_add(ptr: u32, len: u32, pos: *u32) i64 {
    const src: [*]const u8 = @ptrFromInt(ptr);
    var left = expr_mul(ptr, len, pos);
    while (pos.* < len) {
        expr_skip_ws(src, len, pos);
        if (pos.* >= len) break;
        if (src[pos.*] == '+') {
            pos.* += 1;
            left = left + expr_mul(ptr, len, pos);
        } else if (src[pos.*] == '-') {
            pos.* += 1;
            left = left - expr_mul(ptr, len, pos);
        } else if (pos.* + 1 < len and src[pos.*] == '=' and src[pos.* + 1] == '=') {
            pos.* += 2;
            left = if (left == expr_mul(ptr, len, pos)) @as(i64, 1) else @as(i64, 0);
        } else if (pos.* + 1 < len and src[pos.*] == '!' and src[pos.* + 1] == '=') {
            pos.* += 2;
            left = if (left != expr_mul(ptr, len, pos)) @as(i64, 1) else @as(i64, 0);
        } else if (pos.* + 1 < len and src[pos.*] == '<' and src[pos.* + 1] == '=') {
            pos.* += 2;
            left = if (left <= expr_mul(ptr, len, pos)) @as(i64, 1) else @as(i64, 0);
        } else if (pos.* + 1 < len and src[pos.*] == '>' and src[pos.* + 1] == '=') {
            pos.* += 2;
            left = if (left >= expr_mul(ptr, len, pos)) @as(i64, 1) else @as(i64, 0);
        } else if (src[pos.*] == '<') {
            pos.* += 1;
            left = if (left < expr_mul(ptr, len, pos)) @as(i64, 1) else @as(i64, 0);
        } else if (src[pos.*] == '>') {
            pos.* += 1;
            left = if (left > expr_mul(ptr, len, pos)) @as(i64, 1) else @as(i64, 0);
        } else break;
    }
    return left;
}

fn expr_mul(ptr: u32, len: u32, pos: *u32) i64 {
    const src: [*]const u8 = @ptrFromInt(ptr);
    var left = expr_atom(ptr, len, pos);
    while (pos.* < len) {
        expr_skip_ws(src, len, pos);
        if (pos.* >= len) break;
        if (src[pos.*] == '*') {
            pos.* += 1;
            left = left * expr_atom(ptr, len, pos);
        } else if (src[pos.*] == '/') {
            pos.* += 1;
            const r = expr_atom(ptr, len, pos);
            left = if (r != 0) @divTrunc(left, r) else 0;
        } else if (src[pos.*] == '%') {
            pos.* += 1;
            const r = expr_atom(ptr, len, pos);
            left = if (r != 0) @rem(left, r) else 0;
        } else break;
    }
    return left;
}

fn expr_atom(ptr: u32, len: u32, pos: *u32) i64 {
    const src: [*]const u8 = @ptrFromInt(ptr);
    expr_skip_ws(src, len, pos);
    if (pos.* >= len) return 0;
    // Unary minus
    if (src[pos.*] == '-') {
        pos.* += 1;
        return -expr_atom(ptr, len, pos);
    }
    // Parenthesised expression
    if (src[pos.*] == '(') {
        pos.* += 1;
        const val = expr_add(ptr, len, pos);
        expr_skip_ws(src, len, pos);
        if (pos.* < len and src[pos.*] == ')') pos.* += 1;
        return val;
    }
    // $variable
    if (src[pos.*] == '$') {
        pos.* += 1;
        const vs = pos.*;
        while (pos.* < len and ((src[pos.*] >= 'a' and src[pos.*] <= 'z') or
            (src[pos.*] >= 'A' and src[pos.*] <= 'Z') or
            (src[pos.*] >= '0' and src[pos.*] <= '9') or src[pos.*] == '_'))
        {
            pos.* += 1;
        }
        const name = obj_new_string(@intCast(ptr + vs), @intCast(pos.* - vs));
        const val = global_get(name);
        if (val != 0) return obj_get_int(val);
        return 0;
    }
    // [command substitution]
    if (src[pos.*] == '[') {
        pos.* += 1;
        const cs = pos.*;
        var depth: u32 = 1;
        while (pos.* < len and depth > 0) {
            if (src[pos.*] == '[') depth += 1 else if (src[pos.*] == ']') depth -= 1;
            if (depth > 0) pos.* += 1 else pos.* += 1;
        }
        const result = eval_script(ptr + cs, pos.* - 1 - cs);
        if (result != 0) return obj_get_int(result);
        return 0;
    }
    // Number literal
    var negative = false;
    if (src[pos.*] == '+') pos.* += 1;
    if (pos.* < len and src[pos.*] == '-') {
        negative = true;
        pos.* += 1;
    }
    var val: i64 = 0;
    while (pos.* < len and src[pos.*] >= '0' and src[pos.*] <= '9') {
        val = val * 10 + @as(i64, src[pos.*] - '0');
        pos.* += 1;
    }
    return if (negative) -val else val;
}

// -- Command dispatch --
// Evaluate a single command given word TclObjs.
fn eval_command(words: []const i32) i32 {
    if (words.len == 0) return 0;
    const cmd_s = obj_ensure_string(words[0]);
    if (cmd_s.len == 0) return 0;
    const cmd: [*]const u8 = @ptrFromInt(cmd_s.ptr);

    // Dispatch based on command name
    if (str_eq(cmd, cmd_s.len, "set")) {
        if (words.len >= 3) {
            _ = global_set(words[1], words[2]);
            return words[2];
        } else if (words.len >= 2) {
            return global_get(words[1]);
        }
        return 0;
    }
    if (str_eq(cmd, cmd_s.len, "puts")) {
        if (words.len >= 2) return puts(words[words.len - 1]);
        return 0;
    }
    if (str_eq(cmd, cmd_s.len, "expr")) {
        if (words.len >= 2) {
            const es = obj_ensure_string(words[1]);
            return obj_new_int(eval_expr_str(es.ptr, es.len));
        }
        return 0;
    }
    if (str_eq(cmd, cmd_s.len, "return")) {
        if (words.len >= 2) return words[1];
        return 0;
    }
    if (str_eq(cmd, cmd_s.len, "incr")) {
        if (words.len >= 2) {
            const amt_obj = if (words.len >= 3) words[2] else obj_new_int(1);
            return tcl_incr(global_get(words[1]), amt_obj);
        }
        return 0;
    }
    if (str_eq(cmd, cmd_s.len, "if")) {
        return eval_if(words);
    }
    if (str_eq(cmd, cmd_s.len, "while")) {
        return eval_while(words);
    }
    if (str_eq(cmd, cmd_s.len, "for")) {
        return eval_for(words);
    }
    if (str_eq(cmd, cmd_s.len, "foreach")) {
        return eval_foreach(words);
    }
    if (str_eq(cmd, cmd_s.len, "proc")) {
        // proc definitions in interpreted mode — store body in globals
        // for now just ignore (procs should be compiled)
        return 0;
    }
    if (str_eq(cmd, cmd_s.len, "error")) {
        if (words.len >= 2) {
            @"error"(words[1]);
        }
        return 0;
    }
    if (str_eq(cmd, cmd_s.len, "catch")) {
        if (words.len >= 2) {
            catch_enter();
            const body_s = obj_ensure_string(words[1]);
            const result = eval_script(body_s.ptr, body_s.len);
            _ = result;
            return catch_leave();
        }
        return obj_new_int(0);
    }
    if (str_eq(cmd, cmd_s.len, "append")) {
        if (words.len >= 3) return append(global_get(words[1]), words[2]);
        return 0;
    }
    if (str_eq(cmd, cmd_s.len, "llength")) {
        if (words.len >= 2) return list_length(words[1]);
        return 0;
    }
    if (str_eq(cmd, cmd_s.len, "lindex")) {
        if (words.len >= 3) return list_index(words[1], words[2]);
        return 0;
    }
    if (str_eq(cmd, cmd_s.len, "lappend")) {
        if (words.len >= 3) {
            const cur = global_get(words[1]);
            const result = lappend(cur, words[2]);
            _ = global_set(words[1], result);
            return result;
        }
        return 0;
    }
    if (str_eq(cmd, cmd_s.len, "string")) {
        return eval_string_cmd(words);
    }
    if (str_eq(cmd, cmd_s.len, "dict")) {
        return eval_dict_cmd(words);
    }
    if (str_eq(cmd, cmd_s.len, "global")) {
        // In interpreter mode, all variables are already global
        return 0;
    }
    if (str_eq(cmd, cmd_s.len, "package") or str_eq(cmd, cmd_s.len, "namespace") or
        str_eq(cmd, cmd_s.len, "variable") or str_eq(cmd, cmd_s.len, "rename"))
    {
        return 0; // compile-time only
    }
    // Unknown command — set error
    @"error"(words[0]);
    return 0;
}

fn str_eq(a: [*]const u8, alen: u32, comptime b: []const u8) bool {
    if (alen != b.len) return false;
    inline for (0..b.len) |i| {
        if (a[i] != b[i]) return false;
    }
    return true;
}

// -- Control flow commands for interpreter --

fn eval_if(words: []const i32) i32 {
    // if {cond} {body} ?elseif {cond} {body}? ?else {body}?
    var i: u32 = 1;
    while (i < words.len) {
        // Skip "elseif" keyword
        const kw = obj_ensure_string(words[i]);
        const kp: [*]const u8 = @ptrFromInt(kw.ptr);
        if (str_eq(kp, kw.len, "else")) {
            if (i + 1 < words.len) {
                const body_s = obj_ensure_string(words[i + 1]);
                return eval_script(body_s.ptr, body_s.len);
            }
            return 0;
        }
        // Evaluate condition
        const cond_s = obj_ensure_string(words[i]);
        const cval = eval_expr_str(cond_s.ptr, cond_s.len);
        if (cval != 0) {
            // True — execute body
            if (i + 1 < words.len) {
                const body_s = obj_ensure_string(words[i + 1]);
                return eval_script(body_s.ptr, body_s.len);
            }
            return 0;
        }
        // Skip body, look for elseif/else
        i += 2;
        if (i < words.len) {
            const nk = obj_ensure_string(words[i]);
            const np: [*]const u8 = @ptrFromInt(nk.ptr);
            if (str_eq(np, nk.len, "elseif")) {
                i += 1; // skip keyword, loop continues with condition
            }
        }
    }
    return 0;
}

fn eval_while(words: []const i32) i32 {
    // while {cond} {body}
    if (words.len < 3) return 0;
    const cond_s = obj_ensure_string(words[1]);
    const body_s = obj_ensure_string(words[2]);
    var result: i32 = 0;
    while (true) {
        const cval = eval_expr_str(cond_s.ptr, cond_s.len);
        if (cval == 0) break;
        result = eval_script(body_s.ptr, body_s.len);
    }
    return result;
}

fn eval_for(words: []const i32) i32 {
    // for {init} {cond} {next} {body}
    if (words.len < 5) return 0;
    const init_s = obj_ensure_string(words[1]);
    const cond_s = obj_ensure_string(words[2]);
    const next_s = obj_ensure_string(words[3]);
    const body_s = obj_ensure_string(words[4]);
    _ = eval_script(init_s.ptr, init_s.len);
    var result: i32 = 0;
    while (true) {
        const cval = eval_expr_str(cond_s.ptr, cond_s.len);
        if (cval == 0) break;
        result = eval_script(body_s.ptr, body_s.len);
        _ = eval_script(next_s.ptr, next_s.len);
    }
    return result;
}

fn eval_foreach(words: []const i32) i32 {
    // foreach var list body
    if (words.len < 4) return 0;
    const var_name = words[1];
    const list_s = obj_ensure_string(words[2]);
    const body_s = obj_ensure_string(words[3]);
    const n = list_count_elements(list_s.ptr, list_s.len);
    var result: i32 = 0;
    var idx: i64 = 0;
    while (idx < n) : (idx += 1) {
        const elem = list_element_at(list_s.ptr, list_s.len, idx);
        const elem_obj = obj_new_string_copy(list_s.ptr + elem.start, elem.len);
        _ = global_set(var_name, elem_obj);
        result = eval_script(body_s.ptr, body_s.len);
    }
    return result;
}

fn eval_string_cmd(words: []const i32) i32 {
    if (words.len < 3) return 0;
    const sub = obj_ensure_string(words[1]);
    const sp: [*]const u8 = @ptrFromInt(sub.ptr);
    if (str_eq(sp, sub.len, "length")) return string_length(words[2]);
    if (str_eq(sp, sub.len, "index") and words.len >= 4) return string_index(words[2], words[3]);
    if (str_eq(sp, sub.len, "range") and words.len >= 5) return string_range(words[2], words[3], words[4]);
    if (str_eq(sp, sub.len, "compare") and words.len >= 4) return string_compare(words[2], words[3]);
    if (str_eq(sp, sub.len, "equal") and words.len >= 4) return string_equal(words[2], words[3]);
    if (str_eq(sp, sub.len, "match") and words.len >= 4) return string_match(words[2], words[3]);
    if (str_eq(sp, sub.len, "map") and words.len >= 4) return string_map(words[2], words[3]);
    if (str_eq(sp, sub.len, "trim") and words.len >= 3) return string_trim(words[2]);
    if (str_eq(sp, sub.len, "first") and words.len >= 4) return string_first(words[2], words[3]);
    if (str_eq(sp, sub.len, "last") and words.len >= 4) return string_last(words[2], words[3]);
    if (str_eq(sp, sub.len, "toupper")) return string_toupper(words[2]);
    if (str_eq(sp, sub.len, "tolower")) return string_tolower(words[2]);
    if (str_eq(sp, sub.len, "reverse")) return string_reverse(words[2]);
    if (str_eq(sp, sub.len, "repeat") and words.len >= 4) return string_repeat(words[2], words[3]);
    if (str_eq(sp, sub.len, "replace") and words.len >= 6) return string_replace(words[2], words[3], words[4], words[5]);
    return 0;
}

fn eval_dict_cmd(words: []const i32) i32 {
    if (words.len < 3) return 0;
    const sub = obj_ensure_string(words[1]);
    const sp: [*]const u8 = @ptrFromInt(sub.ptr);
    if (str_eq(sp, sub.len, "get") and words.len >= 4) return dict_get(words[2], words[3]);
    if (str_eq(sp, sub.len, "set") and words.len >= 5) {
        const cur = global_get(words[2]);
        const result = dict_set(cur, words[3], words[4]);
        _ = global_set(words[2], result);
        return result;
    }
    if (str_eq(sp, sub.len, "exists") and words.len >= 4) return dict_exists(words[2], words[3]);
    if (str_eq(sp, sub.len, "keys")) return dict_keys(words[2]);
    if (str_eq(sp, sub.len, "values")) return dict_values(words[2]);
    if (str_eq(sp, sub.len, "size")) return dict_size(words[2]);
    if (str_eq(sp, sub.len, "create")) return dict_create();
    return 0;
}

// -- Main eval entry point --

fn eval_script(script_ptr: u32, script_len: u32) i32 {
    if (script_len == 0) return 0;
    const src: [*]const u8 = @ptrFromInt(script_ptr);
    var pos: u32 = 0;
    var result: i32 = 0;
    var wp: [MAX_WORDS]u32 = undefined;
    var wl: [MAX_WORDS]u32 = undefined;

    while (pos < script_len) {
        const cmd = parse_command(src, pos, script_len, &wp, &wl);
        pos = cmd.next;
        if (cmd.count == 0) continue;

        // Substitute words and build TclObj array
        var word_objs: [MAX_WORDS]i32 = undefined;
        var i: u32 = 0;
        while (i < cmd.count) : (i += 1) {
            word_objs[i] = subst_word(wp[i], wl[i]);
        }

        // Dispatch
        result = eval_command(word_objs[0..cmd.count]);

        // Check error flag (for catch propagation)
        if (error_flag != 0) return result;
    }
    return result;
}

// Exported: evaluate a Tcl script string. This is the interpreter entry
// point called by compiled code for constructs it can't handle statically.
export fn tcl_eval(script: i32) i32 {
    const s = obj_ensure_string(script);
    if (s.len == 0) return 0;
    return eval_script(s.ptr, s.len);
}
