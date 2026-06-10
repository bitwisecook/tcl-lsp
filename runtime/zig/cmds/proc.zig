// ``proc`` — procedure definition command.

const procs = @import("../interp/tcl_procs.zig");
const result_mod = @import("../interp/tcl_result.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");
const rt = @import("../tcl_runtime.zig");
const obj_mod = @import("../valtypes/tcl_obj.zig");
const stubs = @import("../stubs/tcl_stubs.zig");
const list_parse = @import("../valtypes/tcl_list_parse.zig");

const obj_ensure_string = obj_mod.obj_ensure_string;
const list_count_elements = rt.list_count_elements;
const list_element_at = rt.list_element_at;

/// Validate ``proc``'s argument-specifier list, raising the same
/// definition-time diagnostics as C Tcl (``TclCreateProc``).  Returns
/// true when an error was raised so the caller can abort before
/// registering the (now-malformed) command — leaving it undefined,
/// which proc-old-5.11 relies on.  Mirrors the call-time validation
/// ``eval_apply`` performs for lambdas in ``tcl_interp.zig``.
fn validate_arg_spec(args_obj: i32) bool {
    const args_spec = obj_ensure_string(args_obj);
    if (args_spec.len == 0) return false;

    // The spec must be a well-formed list — ``check_list_syntax`` raises
    // ``unmatched open brace in list`` (proc-old-5.4) and returns -1.
    if (list_parse.check_list_syntax(args_spec.ptr, args_spec.len) != 0) {
        return true;
    }

    const n_params: u32 = @intCast(list_count_elements(args_spec.ptr, args_spec.len));
    var pi: u32 = 0;
    while (pi < n_params) : (pi += 1) {
        const param_elem = list_element_at(args_spec.ptr, args_spec.len, @intCast(pi));
        const spec_ptr = args_spec.ptr + param_elem.start;
        const spec_len = param_elem.len;
        const field_count = list_count_elements(spec_ptr, spec_len);
        if (field_count == 0) {
            // ``{}`` / empty argument specifier — proc-old-5.5 / 5.6.
            stubs.raise("argument with no name");
            return true;
        }
        if (field_count > 2) {
            // ``{x 1 2}`` — more than ``{name ?default?}`` — proc-old-5.7.
            var mbuf: [256]u8 = undefined;
            const pfx = "too many fields in argument specifier \"";
            var mo: u32 = 0;
            for (pfx) |c| {
                mbuf[mo] = c;
                mo += 1;
            }
            const sp_bytes: [*]const u8 = @ptrFromInt(spec_ptr);
            var si: u32 = 0;
            while (si < spec_len and mo < mbuf.len - 1) : (si += 1) {
                mbuf[mo] = sp_bytes[si];
                mo += 1;
            }
            mbuf[mo] = '"';
            mo += 1;
            stubs.raise(mbuf[0..mo]);
            return true;
        }
    }
    return false;
}

fn eval_proc(words: []const i32) result_mod.InterpResult {
    // ``proc name args body`` takes exactly three arguments.  The runtime
    // dispatch doesn't enforce the registry arity bounds, so the handler
    // raises the canonical wording itself (proc-old-5.1 / 5.2 / 5.3).
    if (words.len != 4) {
        stubs.raise("wrong # args: should be \"proc name args body\"");
        return result_mod.from_globals(0);
    }

    // Validate the argument spec up front so a malformed proc leaves the
    // command undefined rather than registering a broken body.
    if (validate_arg_spec(words[2])) {
        return result_mod.from_globals(0);
    }

    const interp = @import("../interp/tcl_interp.zig");
    const qname = interp.qualify_name(words[1]);
    _ = procs.proc_register(qname, words[2], words[3]);
    return result_mod.from_globals(0);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "proc", .arity_min = 3, .arity_max = 3, .handler = &eval_proc },
};
