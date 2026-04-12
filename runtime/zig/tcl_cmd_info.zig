// Command: info — introspection into the interpreter state.
//
// Subcommands implemented:
//   info exists varName  — check if variable is defined (local or global)
//   info body procName   — return the body of a registered proc
//   info args procName   — return the parameter list of a registered proc
//
// Unimplemented subcommands (future work):
//   info commands ?pattern?   — list built-in + registered commands
//   info procs    ?pattern?   — list registered procedures
//   info level                — current frame depth
//   info vars / info locals / info globals
// info_dispatch() returns an empty string for any subcommand not in the
// list above; this is an explicit NOP rather than an error so code using
// unsupported introspection degrades gracefully in the WASM sandbox.
//
// Operates on frames (tcl_frames.zig) and proc registry (tcl_procs.zig).
// Callable from both the interpreter dispatch and WASM codegen imports.

const obj = @import("tcl_obj.zig");
const obj_ensure_string = obj.obj_ensure_string;
const obj_new_int = obj.obj_new_int;
const obj_new_string = obj.obj_new_string;

const frames = @import("tcl_frames.zig");
const procs = @import("tcl_procs.zig");

fn str_eq(a: [*]const u8, alen: u32, comptime b: []const u8) bool {
    if (alen != b.len) return false;
    inline for (0..b.len) |i| {
        if (a[i] != b[i]) return false;
    }
    return true;
}

/// info exists varName — returns 1 if variable is defined, 0 otherwise.
/// Checks current frame locals first, then globals.
pub export fn info_exists(name: i32) i32 {
    return frames.var_exists(name);
}

/// info body procName — returns the body of an interpreted proc, or empty string.
pub export fn info_body(name: i32) i32 {
    const bucket = procs.proc_lookup(name);
    if (bucket == 0) return obj_new_string(0, 0);
    return procs.proc_get_body(bucket);
}

/// info args procName — returns the parameter list of a proc, or empty string.
pub export fn info_args(name: i32) i32 {
    const bucket = procs.proc_lookup(name);
    if (bucket == 0) return obj_new_string(0, 0);
    return procs.proc_get_params(bucket);
}

/// Dispatch for 'info' command. words[0] = "info", words[1] = subcommand, ...
/// Called by the interpreter's eval_command.
pub export fn info_dispatch(subcmd: i32, arg: i32) i32 {
    const sub = obj_ensure_string(subcmd);
    if (sub.len == 0) return obj_new_string(0, 0);
    const sp: [*]const u8 = @ptrFromInt(sub.ptr);

    if (str_eq(sp, sub.len, "exists")) {
        return info_exists(arg);
    }
    if (str_eq(sp, sub.len, "body")) {
        return info_body(arg);
    }
    if (str_eq(sp, sub.len, "args")) {
        return info_args(arg);
    }
    // Unimplemented subcommands return empty string
    return obj_new_string(0, 0);
}
