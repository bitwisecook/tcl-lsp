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
    const obj = @import("tcl_obj.zig");
    const rt = @import("tcl_runtime.zig");
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
            return obj.obj_new_string(@intCast(buf), 1);
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
    _ = lambda;
    _ = args;
    stubs.unsupported("apply");
    return 0;
}
