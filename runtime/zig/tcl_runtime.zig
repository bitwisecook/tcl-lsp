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
// Export names must match the import names declared in _RUNTIME_IMPORTS
// in wasm.py (the second element of each tuple).  Until the value
// representation transition, the compiled Tcl code passes raw i64
// values (integers or data-segment offsets) and the runtime interprets
// them accordingly.

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
export fn obj_get_int(obj: i32) i64 {
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

// Exported: variable set (identity — pass-through TclObj pointer)
export fn tcl_var_set(value: i32) i32 {
    return value;
}

// Exported: variable get (identity — pass-through TclObj pointer)
export fn tcl_var_get(value: i32) i32 {
    return value;
}

// Exported: increment an integer TclObj, returning a new TclObj
export fn tcl_incr(obj: i32, amount: i32) i32 {
    const val = obj_get_int(obj);
    const amt = obj_get_int(amount);
    return obj_new_int(val + amt);
}

// -- Command runtime stubs --
// All parameters and results are i32 TclObj pointers.
// In freestanding mode these are mostly stubs; the real logic will
// be implemented when switching to wasm32-wasi.

// Exported: puts — write value to stdout.
export fn puts(value: i32) i32 {
    _ = value;
    // Stub: no I/O in freestanding mode.  Return null TclObj.
    return 0;
}

// Exported: append two strings
export fn append(current: i32, addition: i32) i32 {
    // Stub: return the addition value as placeholder.
    _ = current;
    return addition;
}

// Exported: string compare (returns TclObj wrapping -1/0/1)
export fn string_compare(a: i32, b: i32) i32 {
    const va = obj_get_int(a);
    const vb = obj_get_int(b);
    if (va < vb) return obj_new_int(-1);
    if (va > vb) return obj_new_int(1);
    return obj_new_int(0);
}

// Exported: list length — returns TclObj wrapping integer count
export fn list_length(list: i32) i32 {
    // Stub: treat the integer value of the TclObj as the count
    return list;
}

// Exported: list append
export fn lappend(current: i32, value: i32) i32 {
    _ = value;
    // Stub: return a new TclObj with incremented count
    const n = obj_get_int(current);
    return obj_new_int(n + 1);
}

// Exported: string length
export fn string_length(value: i32) i32 {
    _ = value;
    return obj_new_int(0);
}

// Exported: string index
export fn string_index(value: i32, idx: i32) i32 {
    _ = value;
    _ = idx;
    return obj_new_int(0);
}

// Exported: string range
export fn string_range(value: i32, first: i32, last: i32) i32 {
    _ = value;
    _ = first;
    _ = last;
    return obj_new_int(0);
}

// Exported: string map
export fn string_map(mapping: i32, value: i32) i32 {
    _ = mapping;
    return value;
}

// Exported: string match
export fn string_match(pattern: i32, value: i32) i32 {
    const vp = obj_get_int(pattern);
    const vv = obj_get_int(value);
    return obj_new_int(if (vp == vv) @as(i64, 1) else @as(i64, 0));
}

// Exported: string trim
export fn string_trim(value: i32) i32 {
    return value;
}

// Exported: concat
export fn concat(a: i32, b: i32) i32 {
    _ = a;
    return b;
}

// Exported: list index
export fn list_index(list: i32, idx: i32) i32 {
    _ = list;
    _ = idx;
    return obj_new_int(0);
}

// Exported: list range
export fn list_range(list: i32, first: i32, last: i32) i32 {
    _ = list;
    _ = first;
    _ = last;
    return obj_new_int(0);
}

// Exported: list sort
export fn list_sort(list: i32) i32 {
    return list;
}

// Exported: list search
export fn list_search(list: i32, value: i32) i32 {
    _ = list;
    _ = value;
    return obj_new_int(-1);
}

// Exported: error
export fn @"error"(msg: i32) void {
    _ = msg;
    // In freestanding mode, errors are no-ops
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
