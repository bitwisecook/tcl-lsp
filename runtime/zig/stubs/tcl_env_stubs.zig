// Environment / metadata stubs — namespace (non-eval),
// package, trace, interp, apply.  These are Tcl 8.4–9.0 commands
// that reshape the interpreter's global state; each raises
// ``unsupported command: <name>``.
//
// ``namespace eval`` is handled by the compiler directly (see
// core/compiler/lowering_hooks) so it is not stubbed here — only
// the other ``namespace`` subcommands route through this file.
// Similarly ``info exists`` and a handful of ``info`` subcommands
// are real runtime functions in ``tcl_cmd_info.zig``; the
// remaining ``info`` variants fall through to a generic
// ``info_dispatch`` that raises ``unsupported``.
//
// Coverage:
//   - namespace (current, which, code, qualifiers, tail, delete,
//     import, export, forget, origin, inscope, parent, children,
//     path, exists, ensemble)
//   - package (provide, require, vsatisfies, ifneeded, names,
//     versions, unknown, forget, prefer)
//   - trace (add, remove, info)
//   - interp (create, alias, eval, delete, slaves, target,
//     issafe, exists, limit, marktrusted, hide, expose,
//     invokehidden, children, bgerror)
//   - apply (anonymous proc invocation)

const stubs = @import("tcl_stubs.zig");

pub export fn @"namespace"(sub: i32, arg: i32) i32 {
    _ = sub;
    _ = arg;
    stubs.unsupported("namespace");
    return 0;
}

/// ``package`` — permissive stub for the common metadata
/// subcommands.  A compiled WASM bundle links all its modules into
/// one unit, so ``package require``/``provide``/``vsatisfies`` are
/// informational rather than load-triggering: the module is
/// already present if it was compiled in.
///
/// Returns:
///   - ``vsatisfies`` → "1" (treat every version query as "yes,
///     required version is available").  tcltest uses
///     ``[package vsatisfies $version 8.7-]`` to gate
///     UTF-8-related behaviour; answering ``1`` keeps tcltest on
///     its modern-Tcl code path.
///   - every other subcommand → empty string.  ``package require``
///     in Tcl returns the loaded version; we return empty, which
///     tcltest stores in ``::tcltest::version`` and then feeds to
///     ``vsatisfies`` — the vsatisfies stub ignores its args so
///     the empty version doesn't matter.
///
/// This replaces the previous trapping stub — with namespace-
/// eval variables actually persisting, tcltest's top-level
/// ``variable version [package require Tcl 8.5-]`` now evaluates
/// its ``[…]`` substitution and would trap here.
pub export fn tcl_cmd_package_cmd(sub: i32, arg: i32) i32 {
    _ = arg;
    const obj = @import("../valtypes/tcl_obj.zig");
    const rt = @import("../tcl_runtime.zig");
    const s = rt.obj_ensure_string(sub);
    if (s.len == 10) {
        const p: [*]const u8 = @ptrFromInt(s.ptr);
        // ``vsatisfies`` has length 10.  Compare byte-wise to
        // avoid pulling in ``std.mem.eql`` here.
        const target = "vsatisfies";
        var match = true;
        var i: u32 = 0;
        while (i < 10) : (i += 1) {
            if (p[i] != target[i]) {
                match = false;
                break;
            }
        }
        if (match) {
            const buf = obj.alloc(1);
            const d: [*]u8 = @ptrFromInt(buf);
            d[0] = '1';
            return obj.obj_new_string(@bitCast(buf), 1);
        }
    }
    return obj.obj_new_string(0, 0);
}

// ``trace_cmd`` moved to tcl_trace.zig — accepts ``trace add`` /
// ``trace remove`` as benign pass-throughs, traps on info queries.

pub export fn tcl_cmd_interp_cmd(sub: i32, arg: i32) i32 {
    _ = sub;
    _ = arg;
    stubs.unsupported("interp");
    return 0;
}

pub export fn tcl_cmd_apply(lambda: i32, args: i32) i32 {
    const rt     = @import("../tcl_runtime.zig");
    const interp = @import("../interp/tcl_interp.zig");

    // Unpack the args list into individual TclObj handles.
    const args_s = rt.obj_ensure_string(args);
    const n_args: u32 = if (args_s.len > 0)
        @intCast(rt.list_count_elements(args_s.ptr, args_s.len))
    else
        0;

    // Build words slice: words[0]=dummy, words[1]=lambda, words[2..]=args.
    const n_words: u32 = 2 + n_args;
    const words_buf: u32 = rt.alloc(n_words * 4);
    const words_ptr: [*]i32 = @ptrFromInt(words_buf);
    words_ptr[0] = rt.obj_new_string(0, 0); // placeholder for "apply" name
    words_ptr[1] = lambda;
    var ai: u32 = 0;
    while (ai < n_args) : (ai += 1) {
        const elem = rt.list_element_at(args_s.ptr, args_s.len, @intCast(ai));
        const elem_obj = if (elem.braced)
            rt.obj_new_string_copy(args_s.ptr + elem.start, elem.len)
        else blk: {
            const buf = rt.alloc(elem.len + 4);
            const out_len = rt.copy_unbraced_elem(buf, args_s.ptr + elem.start, elem.len);
            break :blk rt.obj_new_string(@bitCast(buf), @bitCast(out_len));
        };
        words_ptr[2 + ai] = elem_obj;
    }
    return interp.eval_apply(words_ptr[0..n_words]);
}
