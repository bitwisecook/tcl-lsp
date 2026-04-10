// Tcl value runtime for WASM.
//
// Provides reference-counted TclObj values in linear memory, with
// exports that the compiled Tcl WASM modules import.  Built as
// wasm32-freestanding (no OS, no libc).
//
// Memory layout of a TclObj:
//   offset 0: refcount  (i32)
//   offset 4: type_tag  (i32)  0=string, 1=int, 2=list
//   offset 8: int_cache (i64)  cached integer representation
//   offset 16: str_ptr  (i32)  pointer to UTF-8 data in linear memory
//   offset 20: str_len  (i32)  byte length of the string representation
//   Total: 24 bytes per TclObj
//
// The runtime exports functions matching the signatures declared in
// _RUNTIME_IMPORTS in wasm.py.  Until the value representation
// transition, the compiled Tcl code passes raw i64 values (integers
// or data-segment offsets) and the runtime interprets them accordingly.

const std = @import("std");

// Type tags
const TYPE_STRING: i32 = 0;
const TYPE_INT: i32 = 1;
const TYPE_LIST: i32 = 2;

// Simple bump allocator over WASM linear memory.
// Starts at page 1 (64KiB offset) to avoid collisions with the
// data segment placed at offset 0 by the compiled module.
var heap_ptr: u32 = 65536;

fn alloc(size: u32) callconv(.C) u32 {
    const aligned = (size + 7) & ~@as(u32, 7); // 8-byte alignment
    const ptr = heap_ptr;
    heap_ptr += aligned;
    return ptr;
}

// Read an i32 from linear memory
fn read_i32(addr: u32) i32 {
    const ptr: [*]const u8 = @ptrFromInt(addr);
    return @as(i32, @bitCast([4]u8{ ptr[0], ptr[1], ptr[2], ptr[3] }));
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

// Read an i64 from linear memory
fn read_i64(addr: u32) i64 {
    const ptr: [*]const u8 = @ptrFromInt(addr);
    return @as(i64, @bitCast([8]u8{
        ptr[0], ptr[1], ptr[2], ptr[3],
        ptr[4], ptr[5], ptr[6], ptr[7],
    }));
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

// Exported: create a new integer TclObj
export fn tcl_obj_new_int(value: i64) i32 {
    const ptr = obj_alloc();
    write_i32(ptr + OBJ_TYPE_TAG, TYPE_INT);
    write_i64(ptr + OBJ_INT_CACHE, value);
    return @as(i32, @intCast(ptr));
}

// Exported: create a new string TclObj from a data-segment pointer + length
export fn tcl_obj_new_string(data_ptr: i32, length: i32) i32 {
    const ptr = obj_alloc();
    write_i32(ptr + OBJ_TYPE_TAG, TYPE_STRING);
    write_i32(ptr + OBJ_STR_PTR, data_ptr);
    write_i32(ptr + OBJ_STR_LEN, length);
    return @as(i32, @intCast(ptr));
}

// Exported: get the integer value of a TclObj
export fn tcl_obj_get_int(obj: i32) i64 {
    const addr: u32 = @intCast(obj);
    return read_i64(addr + OBJ_INT_CACHE);
}

// Exported: increment refcount
export fn tcl_obj_retain(obj: i32) void {
    const addr: u32 = @intCast(obj);
    const rc = read_i32(addr + OBJ_REFCOUNT);
    write_i32(addr + OBJ_REFCOUNT, rc + 1);
}

// Exported: decrement refcount (no-op free for now — no GC)
export fn tcl_obj_release(obj: i32) void {
    const addr: u32 = @intCast(obj);
    const rc = read_i32(addr + OBJ_REFCOUNT);
    write_i32(addr + OBJ_REFCOUNT, rc - 1);
    // TODO: free when rc reaches 0
}

// Exported: variable set (identity in the i64 model)
export fn tcl_var_set(value: i64) i64 {
    return value;
}

// Exported: variable get (identity in the i64 model)
export fn tcl_var_get(value: i64) i64 {
    return value;
}

// Exported: increment an integer value
export fn tcl_incr(value: i64, amount: i64) i64 {
    return value + amount;
}

// Exported: puts — write value to stdout.
// In freestanding mode this is a stub; wasm32-wasi target would use fd_write.
export fn puts(value: i64) i64 {
    _ = value;
    // Stub: no I/O in freestanding mode
    return 0;
}

// Exported: string append
export fn string_append(current: i64, addition: i64) i64 {
    // In the i64 model, string append is not meaningful.
    // Return the addition value as a placeholder.
    _ = current;
    return addition;
}

// Exported: string compare
export fn string_compare(a: i64, b: i64) i64 {
    if (a < b) return -1;
    if (a > b) return 1;
    return 0;
}

// Exported: list length
// In the i64 model, the "list" is just an integer count.
export fn list_length(list: i64) i64 {
    return list;
}

// Exported: list append
export fn lappend(current: i64, value: i64) i64 {
    _ = value;
    return current + 1;
}

// Exported: string length
// In the i64 model, returns 0 for non-string values.
export fn string_length(value: i64) i64 {
    _ = value;
    return 0;
}

// Exported: string index
export fn string_index(value: i64, idx: i64) i64 {
    _ = value;
    _ = idx;
    return 0;
}

// Exported: string range
export fn string_range(value: i64, first: i64, last: i64) i64 {
    _ = value;
    _ = first;
    _ = last;
    return 0;
}

// Exported: string map
export fn string_map(mapping: i64, value: i64) i64 {
    _ = mapping;
    return value;
}

// Exported: string match
export fn string_match(pattern: i64, value: i64) i64 {
    return if (pattern == value) @as(i64, 1) else @as(i64, 0);
}

// Exported: string trim
export fn string_trim(value: i64) i64 {
    return value;
}

// Exported: concat
export fn concat(a: i64, b: i64) i64 {
    _ = a;
    return b;
}

// Exported: list index
export fn list_index(list: i64, idx: i64) i64 {
    _ = list;
    _ = idx;
    return 0;
}

// Exported: list range
export fn list_range(list: i64, first: i64, last: i64) i64 {
    _ = list;
    _ = first;
    _ = last;
    return 0;
}

// Exported: list sort
export fn list_sort(list: i64) i64 {
    return list;
}

// Exported: list search
export fn list_search(list: i64, value: i64) i64 {
    _ = list;
    _ = value;
    return -1;
}

// Exported: error
export fn @"error"(msg: i64) void {
    _ = msg;
    // In freestanding mode, errors are no-ops
}

// Exported: format
export fn format(fmt: i64, value: i64) i64 {
    _ = fmt;
    return value;
}

// Exported: regexp
export fn regexp(pattern: i64, str: i64) i64 {
    _ = pattern;
    _ = str;
    return 0;
}

// Exported: open
export fn open(path: i64) i64 {
    _ = path;
    return -1; // no file I/O in freestanding
}

// Exported: close
export fn close(fd: i64) i64 {
    _ = fd;
    return 0;
}

// Exported: read
export fn read(fd: i64) i64 {
    _ = fd;
    return 0;
}

// Exported: gets
export fn gets(fd: i64) i64 {
    _ = fd;
    return 0;
}
