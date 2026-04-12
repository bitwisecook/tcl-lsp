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

// Scratch buffer for integer-to-string conversion (max 20 digits + sign + newline)
var itoa_buf: [22]u8 = undefined;

pub fn itoa(value: i64) struct { ptr: [*]u8, len: u32 } {
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

// Re-export itoa_no_nl from tcl_obj (it lives there to avoid circular deps)
pub const itoa_no_nl = obj.itoa_no_nl;

pub fn fd_write_all(fd: i32, data: [*]const u8, len: u32) void {
    const iov = [_]std.os.wasi.ciovec_t{.{
        .base = data,
        .len = len,
    }};
    var written: usize = 0;
    _ = std.os.wasi.fd_write(@intCast(fd), &iov, 1, &written);
}

// Exported: puts — write value to stdout via WASI fd_write.
pub export fn puts(value: i32) i32 {
    if (value == 0) {
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
        const int_val = read_i64(addr + OBJ_INT_CACHE);
        const result = itoa(int_val);
        fd_write_all(1, result.ptr, result.len);
    }
    return 0;
}
