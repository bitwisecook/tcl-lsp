// WASI I/O helpers: itoa, fd_write_all, puts.

const std = @import("std");
const obj = @import("tcl_obj.zig");
const read_i32 = obj.read_i32;
const read_i64 = obj.read_i64;
const obj_ensure_string = obj.obj_ensure_string;
const TYPE_STRING = obj.TYPE_STRING;
const OBJ_TYPE_TAG = obj.OBJ_TYPE_TAG;
const OBJ_STR_PTR = obj.OBJ_STR_PTR;
const OBJ_STR_LEN = obj.OBJ_STR_LEN;
const OBJ_INT_CACHE = obj.OBJ_INT_CACHE;

// Re-export ``itoa`` from tcl_obj (it lives there to avoid circular deps).
// The canonical implementation renders an integer *without* a trailing
// newline; callers that want one (``tcl_cmd_puts``) append it
// explicitly via ``fd_write_all(1, "\n", 1)`` after writing the digits.
pub const itoa = obj.itoa;

pub fn fd_write_all(fd: i32, data: [*]const u8, len: u32) void {
    const iov = [_]std.os.wasi.ciovec_t{.{
        .base = data,
        .len = len,
    }};
    var written: usize = 0;
    _ = std.os.wasi.fd_write(@intCast(fd), &iov, 1, &written);
}

// Internal: emit the rendered string of *value* to stdout, with or
// without a trailing newline.  ``tcl_cmd_puts`` / ``tcl_cmd_puts_
// nonewline`` share this helper.  ``itoa`` renders digits without a
// newline so we can append (or skip) it uniformly at the end,
// matching Tcl's ``puts`` / ``puts -nonewline`` contract.
fn puts_raw(value: i32, want_newline: bool) void {
    if (value == 0) {
        if (want_newline) fd_write_all(1, "\n", 1);
        return;
    }
    const addr: u32 = @intCast(value);
    const tag = read_i32(addr + OBJ_TYPE_TAG);
    if (tag == TYPE_STRING) {
        const sptr = read_i32(addr + OBJ_STR_PTR);
        const slen = read_i32(addr + OBJ_STR_LEN);
        if (slen > 0) {
            fd_write_all(1, @ptrFromInt(@as(u32, @intCast(sptr))), @intCast(slen));
        }
    } else {
        const int_val = read_i64(addr + OBJ_INT_CACHE);
        const result = itoa(int_val);
        fd_write_all(1, result.ptr, result.len);
    }
    if (want_newline) fd_write_all(1, "\n", 1);
}

// Exported: puts — write value to stdout via WASI fd_write.
pub export fn tcl_cmd_puts(value: i32) i32 {
    puts_raw(value, true);
    return 0;
}

// Exported: puts -nonewline — write value without appending a newline.
pub export fn tcl_cmd_puts_nonewline(value: i32) i32 {
    puts_raw(value, false);
    return 0;
}
