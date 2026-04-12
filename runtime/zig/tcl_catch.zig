// Error handling for catch, plus file I/O stubs (format, regexp, open, close, read, gets).

const obj = @import("tcl_obj.zig");
const io = @import("tcl_io.zig");
const obj_ensure_string = obj.obj_ensure_string;
const obj_new_int = obj.obj_new_int;
const fd_write_all = io.fd_write_all;

// Control flow signals — picol-style return codes as mutable flags.
// Each flag is checked by eval_script after every command.
// Loops catch break/continue; proc dispatch catches return; catch catches error.
pub var catch_depth: u32 = 0;
pub var error_flag: u32 = 0; // 0 = no error, 1 = error pending
pub var error_msg: i32 = 0; // TclObj with error message
pub var return_flag: u32 = 0; // 1 = return pending (absorbed by proc dispatch)
pub var return_val: i32 = 0; // TclObj return value
pub var break_flag: u32 = 0; // 1 = break pending (absorbed by loops)
pub var continue_flag: u32 = 0; // 1 = continue pending (absorbed by loops)

// Exported: enter a catch scope.
pub export fn catch_enter() void {
    catch_depth += 1;
    error_flag = 0;
    error_msg = 0;
}

// Exported: leave a catch scope. Returns 0 (TCL_OK) or 1 (TCL_ERROR).
pub export fn catch_leave() i32 {
    if (catch_depth > 0) catch_depth -= 1;
    const had_error = error_flag;
    error_flag = 0;
    return obj_new_int(@intCast(had_error));
}

// Exported: get the error message (or last result) after catch.
pub export fn catch_result() i32 {
    return error_msg;
}

// Exported: check if an error is pending (for early exit from catch body).
pub export fn catch_has_error() i32 {
    return @as(i32, @intCast(error_flag));
}

// Exported: error — write message to stderr and trap, OR set error flag in catch.
pub export fn @"error"(msg: i32) void {
    if (catch_depth > 0) {
        error_flag = 1;
        error_msg = msg;
        return;
    }
    const s = obj_ensure_string(msg);
    if (s.len > 0) {
        fd_write_all(2, @ptrFromInt(s.ptr), s.len);
        fd_write_all(2, "\n", 1);
    }
    @trap();
}

// Exported: format
pub export fn format(fmt: i32, value: i32) i32 {
    _ = fmt;
    return value;
}

// Exported: regexp
pub export fn regexp(pattern: i32, str: i32) i32 {
    _ = pattern;
    _ = str;
    return obj_new_int(0);
}

// Exported: open
pub export fn open(path: i32) i32 {
    _ = path;
    return obj_new_int(-1);
}

// Exported: close
pub export fn close(fd: i32) i32 {
    _ = fd;
    return obj_new_int(0);
}

// Exported: read
pub export fn read(fd: i32) i32 {
    _ = fd;
    return obj_new_int(0);
}

// Exported: gets
pub export fn gets(fd: i32) i32 {
    _ = fd;
    return obj_new_int(0);
}
