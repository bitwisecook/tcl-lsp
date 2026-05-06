// ``global``, ``variable``, ``upvar`` — scope/namespace linkage commands.

const rt = @import("../tcl_runtime.zig");
const result_mod = @import("../interp/tcl_result.zig");
const frames = @import("../interp/tcl_frames.zig");
const tcl_ns = @import("../interp/tcl_ns.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");

const obj_new_string = rt.obj_new_string;
const obj_ensure_string = rt.obj_ensure_string;

fn eval_global(words: []const i32) result_mod.InterpResult {
    var gi: u32 = 1;
    while (gi < words.len) : (gi += 1) {
        frames.frame_alias_global(words[gi]);
    }
    return result_mod.from_globals(0);
}

fn eval_variable(words: []const i32) result_mod.InterpResult {
    var i: u32 = 1;
    while (i < words.len) : (i += 1) {
        const name_obj = words[i];
        const sn = obj_ensure_string(name_obj);
        const r = tcl_ns.ns_resolve_qualified_creating(
            tcl_ns.ns_current(),
            sn.ptr,
            sn.len,
        );
        if (r.target_ns == 0 or r.simple_len == 0) continue;
        const var_ptr = tcl_ns.ns_var_create(r.target_ns, r.simple_ptr, r.simple_len);
        const local_name = obj_new_string(@bitCast(r.simple_ptr), @bitCast(r.simple_len));
        // Pass the target namespace + simple-name span through to
        // ``frame_alias_ns_var`` so :func:`frame_resolve_array_name`
        // can synthesise a fresh ``<ns_full>::<simple>`` TclObj on
        // demand (caller releases) — without the alias, ``set
        // arr(k) v`` from inside the proc body would land in a
        // synthetic ``::__local::<depth>::<simple>`` array and
        // disconnect writes from the trace dispatch keyed on the
        // FQ form (string.test SafeFetch path).  The earlier shape
        // here built a TclObj for the FQ name and stashed its
        // handle in the alias descriptor, but the namespace var's
        // simple-name span — owned by the namespace's own
        // ``var_table`` bucket — is stable for the namespace's
        // lifetime, so we can defer the FQ-name materialisation
        // and avoid descriptor-side TclObj lifetime questions
        // (Copilot review on PR #343).
        frames.frame_alias_ns_var(local_name, var_ptr, r.target_ns, r.simple_ptr, r.simple_len);
        if (i + 1 < words.len) {
            tcl_ns.var_set_scalar(var_ptr, @bitCast(words[i + 1]));
            i += 1;
        }
    }
    return result_mod.from_globals(obj_new_string(0, 0));
}

fn eval_upvar(words: []const i32) result_mod.InterpResult {
    const interp = @import("../interp/tcl_interp.zig");
    return result_mod.from_globals(interp.eval_upvar(words));
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "global", .arity_min = 1, .arity_max = null, .handler = &eval_global },
    .{ .name = "variable", .arity_min = 1, .arity_max = null, .handler = &eval_variable },
    .{ .name = "upvar", .arity_min = 2, .arity_max = null, .handler = &eval_upvar },
};
