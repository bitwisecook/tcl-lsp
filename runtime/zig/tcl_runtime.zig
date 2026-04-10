///! Tcl value runtime for WASM
///!
///! Implements Tcl string, list, and dict values as reference-counted
///! objects in linear WASM memory.  All values are represented as
///! 32-bit pointers (TclObj).  The runtime exports functions that the
///! compiled Tcl code calls via WASM imports.
///!
///! Object layout (in linear memory):
///!   [0..3]  refcount  (u32)
///!   [4..7]  type tag  (u32: 0=string, 1=int, 2=list, 3=dict)
///!   [8..15] int cache (i64, valid when type=1 or after successful parse)
///!   [16..19] str_len  (u32, byte length of UTF-8 string representation)
///!   [20..]  str_data  (UTF-8 bytes)
///!
///! This is intentionally simple: every value has a string
///! representation (Tcl's EIAS principle).  Integers are cached but
///! the string is always authoritative.

const std = @import("std");

// Object header layout
const HEADER_SIZE: u32 = 20;
const OFF_REFCOUNT: u32 = 0;
const OFF_TYPE: u32 = 4;
const OFF_INT_CACHE: u32 = 8;
const OFF_STR_LEN: u32 = 16;
const OFF_STR_DATA: u32 = 20;

// Type tags
const TYPE_STRING: u32 = 0;
const TYPE_INT: u32 = 1;
const TYPE_LIST: u32 = 2;
const TYPE_DICT: u32 = 3;

// Simple bump allocator for WASM linear memory
var heap_ptr: u32 = 1024; // Start after 1KB reserved for stack/globals

fn align_up(addr: u32, alignment: u32) -> u32 {
    return (addr + alignment - 1) & ~(alignment - 1);
}

fn alloc(size: u32) -> u32 {
    const aligned = align_up(heap_ptr, 8);
    const result = aligned;
    heap_ptr = aligned + size;
    // Grow memory if needed
    const pages_needed = (heap_ptr + 65535) / 65536;
    const current_pages: u32 = @intCast(@wasmMemorySize(0));
    if (pages_needed > current_pages) {
        _ = @wasmMemoryGrow(0, pages_needed - current_pages);
    }
    return result;
}

fn mem_ptr(offset: u32) -> [*]u8 {
    return @as([*]u8, @ptrFromInt(offset));
}

fn read_u32(offset: u32) -> u32 {
    const ptr = @as(*align(1) const u32, @ptrFromInt(offset));
    return ptr.*;
}

fn write_u32(offset: u32, value: u32) -> void {
    const ptr = @as(*align(1) u32, @ptrFromInt(offset));
    ptr.* = value;
}

fn read_i64(offset: u32) -> i64 {
    const ptr = @as(*align(1) const i64, @ptrFromInt(offset));
    return ptr.*;
}

fn write_i64(offset: u32, value: i64) -> void {
    const ptr = @as(*align(1) i64, @ptrFromInt(offset));
    ptr.* = value;
}

// Create a new TclObj from a string in linear memory
export fn tcl_obj_new_string(str_ptr: u32, str_len: u32) -> u32 {
    const obj = alloc(HEADER_SIZE + str_len);
    write_u32(obj + OFF_REFCOUNT, 1);
    write_u32(obj + OFF_TYPE, TYPE_STRING);
    write_i64(obj + OFF_INT_CACHE, 0);
    write_u32(obj + OFF_STR_LEN, str_len);
    // Copy string data
    const dst = mem_ptr(obj + OFF_STR_DATA);
    const src = mem_ptr(str_ptr);
    @memcpy(dst[0..str_len], src[0..str_len]);
    return obj;
}

// Create a new TclObj from an integer
export fn tcl_obj_new_int(value: i64) -> u32 {
    // Format integer as string
    var buf: [21]u8 = undefined;
    const str = std.fmt.bufPrint(&buf, "{d}", .{value}) catch unreachable;
    const str_len: u32 = @intCast(str.len);

    const obj = alloc(HEADER_SIZE + str_len);
    write_u32(obj + OFF_REFCOUNT, 1);
    write_u32(obj + OFF_TYPE, TYPE_INT);
    write_i64(obj + OFF_INT_CACHE, value);
    write_u32(obj + OFF_STR_LEN, str_len);
    const dst = mem_ptr(obj + OFF_STR_DATA);
    @memcpy(dst[0..str_len], str.ptr[0..str_len]);
    return obj;
}

// Create an empty string object
export fn tcl_obj_new_empty() -> u32 {
    const obj = alloc(HEADER_SIZE);
    write_u32(obj + OFF_REFCOUNT, 1);
    write_u32(obj + OFF_TYPE, TYPE_STRING);
    write_i64(obj + OFF_INT_CACHE, 0);
    write_u32(obj + OFF_STR_LEN, 0);
    return obj;
}

// Increment reference count
export fn tcl_obj_incr_ref(obj: u32) -> void {
    if (obj == 0) return;
    const rc = read_u32(obj + OFF_REFCOUNT);
    write_u32(obj + OFF_REFCOUNT, rc + 1);
}

// Decrement reference count (free when zero)
export fn tcl_obj_decr_ref(obj: u32) -> void {
    if (obj == 0) return;
    const rc = read_u32(obj + OFF_REFCOUNT);
    if (rc <= 1) {
        // Object is freed — in a real allocator we'd return memory.
        // With bump allocation, we just mark it as dead.
        write_u32(obj + OFF_REFCOUNT, 0);
        write_u32(obj + OFF_TYPE, 0xDEAD);
    } else {
        write_u32(obj + OFF_REFCOUNT, rc - 1);
    }
}

// Get the integer value of a TclObj (parse if needed)
export fn tcl_obj_get_int(obj: u32) -> i64 {
    if (obj == 0) return 0;
    const typ = read_u32(obj + OFF_TYPE);
    if (typ == TYPE_INT) {
        return read_i64(obj + OFF_INT_CACHE);
    }
    // Parse string as integer
    const str_len = read_u32(obj + OFF_STR_LEN);
    if (str_len == 0) return 0;
    const str_data = mem_ptr(obj + OFF_STR_DATA);
    const slice = str_data[0..str_len];
    const value = std.fmt.parseInt(i64, slice, 10) catch 0;
    // Cache the parsed value
    write_i64(obj + OFF_INT_CACHE, value);
    write_u32(obj + OFF_TYPE, TYPE_INT);
    return value;
}

// Get the string length
export fn tcl_obj_str_len(obj: u32) -> u32 {
    if (obj == 0) return 0;
    return read_u32(obj + OFF_STR_LEN);
}

// Get pointer to string data
export fn tcl_obj_str_ptr(obj: u32) -> u32 {
    if (obj == 0) return 0;
    return obj + OFF_STR_DATA;
}

// String concatenation: append str2 to str1
export fn tcl_string_append(obj1: u32, obj2: u32) -> u32 {
    const len1 = read_u32(obj1 + OFF_STR_LEN);
    const len2 = read_u32(obj2 + OFF_STR_LEN);
    const total = len1 + len2;

    const result = alloc(HEADER_SIZE + total);
    write_u32(result + OFF_REFCOUNT, 1);
    write_u32(result + OFF_TYPE, TYPE_STRING);
    write_i64(result + OFF_INT_CACHE, 0);
    write_u32(result + OFF_STR_LEN, total);

    const dst = mem_ptr(result + OFF_STR_DATA);
    if (len1 > 0) {
        const src1 = mem_ptr(obj1 + OFF_STR_DATA);
        @memcpy(dst[0..len1], src1[0..len1]);
    }
    if (len2 > 0) {
        const src2 = mem_ptr(obj2 + OFF_STR_DATA);
        @memcpy(dst[len1..total], src2[0..len2]);
    }
    return result;
}

// String comparison (returns -1, 0, or 1)
export fn tcl_string_compare(obj1: u32, obj2: u32) -> i32 {
    const len1 = read_u32(obj1 + OFF_STR_LEN);
    const len2 = read_u32(obj2 + OFF_STR_LEN);
    const min_len = if (len1 < len2) len1 else len2;

    const s1 = mem_ptr(obj1 + OFF_STR_DATA);
    const s2 = mem_ptr(obj2 + OFF_STR_DATA);

    var i: u32 = 0;
    while (i < min_len) : (i += 1) {
        if (s1[i] < s2[i]) return -1;
        if (s1[i] > s2[i]) return 1;
    }
    if (len1 < len2) return -1;
    if (len1 > len2) return 1;
    return 0;
}

// String equality check
export fn tcl_string_equal(obj1: u32, obj2: u32) -> i32 {
    const len1 = read_u32(obj1 + OFF_STR_LEN);
    const len2 = read_u32(obj2 + OFF_STR_LEN);
    if (len1 != len2) return 0;
    if (len1 == 0) return 1;

    const s1 = mem_ptr(obj1 + OFF_STR_DATA);
    const s2 = mem_ptr(obj2 + OFF_STR_DATA);

    var i: u32 = 0;
    while (i < len1) : (i += 1) {
        if (s1[i] != s2[i]) return 0;
    }
    return 1;
}

// Integer arithmetic
export fn tcl_int_add(a: i64, b: i64) -> u32 {
    return tcl_obj_new_int(a + b);
}

export fn tcl_int_sub(a: i64, b: i64) -> u32 {
    return tcl_obj_new_int(a - b);
}

export fn tcl_int_mul(a: i64, b: i64) -> u32 {
    return tcl_obj_new_int(a * b);
}

export fn tcl_int_div(a: i64, b: i64) -> u32 {
    if (b == 0) return tcl_obj_new_int(0); // Error in real Tcl
    return tcl_obj_new_int(@divTrunc(a, b));
}

export fn tcl_int_mod(a: i64, b: i64) -> u32 {
    if (b == 0) return tcl_obj_new_int(0);
    return tcl_obj_new_int(@rem(a, b));
}

// set command: store value in variable table
// Variables are stored as an array of (name_ptr, value_ptr) pairs
// starting at a fixed memory location.
var var_table_ptr: u32 = 0;
var var_table_count: u32 = 0;
const MAX_VARS: u32 = 256;

export fn tcl_var_set(name: u32, value: u32) -> u32 {
    if (var_table_ptr == 0) {
        var_table_ptr = alloc(MAX_VARS * 8); // 2 x u32 per entry
    }

    // Search for existing variable
    var i: u32 = 0;
    while (i < var_table_count) : (i += 1) {
        const entry_name = read_u32(var_table_ptr + i * 8);
        if (tcl_string_equal(entry_name, name) == 1) {
            // Update existing
            const old_value = read_u32(var_table_ptr + i * 8 + 4);
            tcl_obj_decr_ref(old_value);
            tcl_obj_incr_ref(value);
            write_u32(var_table_ptr + i * 8 + 4, value);
            return value;
        }
    }

    // Add new variable
    if (var_table_count < MAX_VARS) {
        tcl_obj_incr_ref(name);
        tcl_obj_incr_ref(value);
        write_u32(var_table_ptr + var_table_count * 8, name);
        write_u32(var_table_ptr + var_table_count * 8 + 4, value);
        var_table_count += 1;
    }
    return value;
}

export fn tcl_var_get(name: u32) -> u32 {
    if (var_table_ptr == 0) return 0;

    var i: u32 = 0;
    while (i < var_table_count) : (i += 1) {
        const entry_name = read_u32(var_table_ptr + i * 8);
        if (tcl_string_equal(entry_name, name) == 1) {
            return read_u32(var_table_ptr + i * 8 + 4);
        }
    }
    return 0; // Variable not found
}

// puts command — write string to stdout via WASI fd_write
export fn tcl_puts(obj: u32) -> u32 {
    const str_len = read_u32(obj + OFF_STR_LEN);
    const str_ptr = obj + OFF_STR_DATA;

    // Use WASI fd_write to write to stdout (fd=1)
    // Set up iovec: [ptr, len]
    const iov_base = alloc(8);
    write_u32(iov_base, str_ptr);
    write_u32(iov_base + 4, str_len);

    // Write newline
    const nl_buf = alloc(1);
    mem_ptr(nl_buf)[0] = '\n';
    const nl_iov = alloc(8);
    write_u32(nl_iov, nl_buf);
    write_u32(nl_iov + 4, 1);

    // For now, just return empty — actual fd_write needs WASI imports
    return tcl_obj_new_empty();
}

// List operations

// Split a string into a Tcl list (simplified: split on whitespace)
export fn tcl_list_length(obj: u32) -> i64 {
    const str_len = read_u32(obj + OFF_STR_LEN);
    if (str_len == 0) return 0;

    const data = mem_ptr(obj + OFF_STR_DATA);
    var count: i64 = 0;
    var in_word = false;
    var in_braces: u32 = 0;
    var in_quotes = false;

    var i: u32 = 0;
    while (i < str_len) : (i += 1) {
        const c = data[i];
        if (in_braces > 0) {
            if (c == '{') in_braces += 1;
            if (c == '}') in_braces -= 1;
            continue;
        }
        if (in_quotes) {
            if (c == '"') in_quotes = false;
            continue;
        }
        if (c == '{') {
            if (!in_word) count += 1;
            in_word = true;
            in_braces = 1;
        } else if (c == '"') {
            if (!in_word) count += 1;
            in_word = true;
            in_quotes = true;
        } else if (c == ' ' or c == '\t' or c == '\n') {
            in_word = false;
        } else {
            if (!in_word) count += 1;
            in_word = true;
        }
    }
    return count;
}

// Append element to list
export fn tcl_list_append(list_obj: u32, elem: u32) -> u32 {
    const list_len = read_u32(list_obj + OFF_STR_LEN);
    const elem_len = read_u32(elem + OFF_STR_LEN);

    // Simple: concatenate with space separator
    const need_space: u32 = if (list_len > 0) 1 else 0;
    const total = list_len + need_space + elem_len;

    const result = alloc(HEADER_SIZE + total);
    write_u32(result + OFF_REFCOUNT, 1);
    write_u32(result + OFF_TYPE, TYPE_LIST);
    write_i64(result + OFF_INT_CACHE, 0);
    write_u32(result + OFF_STR_LEN, total);

    const dst = mem_ptr(result + OFF_STR_DATA);
    if (list_len > 0) {
        const src1 = mem_ptr(list_obj + OFF_STR_DATA);
        @memcpy(dst[0..list_len], src1[0..list_len]);
    }
    if (need_space == 1) {
        dst[list_len] = ' ';
    }
    if (elem_len > 0) {
        const src2 = mem_ptr(elem + OFF_STR_DATA);
        @memcpy(dst[list_len + need_space .. total], src2[0..elem_len]);
    }
    return result;
}

// incr: increment integer variable
export fn tcl_incr(obj: u32, amount: i64) -> u32 {
    const current = tcl_obj_get_int(obj);
    return tcl_obj_new_int(current + amount);
}

// expr: boolean check (non-zero = true)
export fn tcl_obj_is_true(obj: u32) -> i32 {
    if (obj == 0) return 0;
    const typ = read_u32(obj + OFF_TYPE);
    if (typ == TYPE_INT) {
        return if (read_i64(obj + OFF_INT_CACHE) != 0) 1 else 0;
    }
    // Try parsing as integer
    const val = tcl_obj_get_int(obj);
    return if (val != 0) 1 else 0;
}
