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

pub export fn package_cmd(sub: i32, arg: i32) i32 {
    _ = sub;
    _ = arg;
    stubs.unsupported("package");
    return 0;
}

// ``trace_cmd`` moved to tcl_trace.zig — accepts ``trace add`` /
// ``trace remove`` as benign pass-throughs, traps on info queries.

pub export fn interp_cmd(sub: i32, arg: i32) i32 {
    _ = sub;
    _ = arg;
    stubs.unsupported("interp");
    return 0;
}

pub export fn apply(lambda: i32, args: i32) i32 {
    _ = lambda;
    _ = args;
    stubs.unsupported("apply");
    return 0;
}
