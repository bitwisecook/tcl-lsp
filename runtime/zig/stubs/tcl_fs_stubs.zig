// Filesystem + process stubs — Tcl 8.4–9.0 commands that operate
// on the host filesystem or launch subprocesses.  Pure WASM has no
// real filesystem (WASI preopens a narrow slice) so these all raise
// ``unsupported command: <name>``.
//
// **Reachability** — same as ``tcl_io_stubs.zig``: each
// ``pub export fn tcl_cmd_X`` is a WASM import declared by a
// ``CommandSpec.wasm_runtime_import`` under
// ``core/commands/registry/tcl/`` (e.g. ``file.py``, ``glob_.py``,
// ``exec_.py``).  ``_imports.py:import_signature`` resolves those
// specs and only falls back to ``_INFRASTRUCTURE_IMPORTS`` for
// helpers with no command owner.  Do not delete an export without
// also dropping the corresponding ``wasm_runtime_import`` declaration
// — the parity gate will block the merge otherwise.
//
// Coverage:
//   - ``file`` — pass-through impl in io/tcl_fs.zig (string-only
//     subcommands, real access(2)/stat(2), recursive mkdir, …).
//   - ``glob`` — real impl in io/tcl_fs.zig gated on CAP_FS_GLOB.
//   - ``pwd`` / ``cd`` — pass-through in io/tcl_fs.zig.
//   - ``exec`` — real impl in cmds/exec.zig gated on CAP_EXEC,
//     dispatching to the host-imported ``env.host_spawn``.
//   - ``source`` — real impl in io/tcl_fs.zig.
//   - ``load`` / ``unload`` — trapping stubs (below).

const stubs = @import("tcl_stubs.zig");

// ``file`` moved to tcl_fs.zig — has pass-through implementations
// for string-only path manipulation (join / dirname / tail /
// rootname / extension / normalize / split / pathtype / separator
// / nativename), always-false answers for existence queries, and
// trapping behaviour for mutating ops (mkdir / delete / rename / …).

// ``glob`` moved to io/tcl_fs.zig with a real ``opendir`` /
// ``readdir`` / fnmatch implementation gated on ``CAP_FS_GLOB``
// (see :module:`interp/tcl_caps.zig`).  Scripts that hit this
// command without the capability granted get a Tcl-catchable
// ``permission denied`` error.

// ``pwd`` and ``cd`` moved to tcl_fs.zig — pwd returns "/" and cd
// silently accepts its arg.  Scripts that use them for logging /
// path-seed purposes (tcltest's ``workingDirectory`` option is the
// poster child) now load without tripping.

// ``exec`` moved to cmds/exec.zig with a host-imported ``host_spawn``
// primitive gated on ``CAP_EXEC``; the BUILTIN handler there owns
// argv assembly and capability enforcement.  Scripts that call
// ``exec`` without the capability see a Tcl-catchable
// ``permission denied`` error.

// ``tcl_cmd_source`` moved to io/tcl_fs.zig with a real WASI-fd
// resolution + read + tcl_eval implementation.  Kept this comment
// stub so a grep for ``tcl_cmd_source`` still lands somewhere
// useful when reading the stubs file.

pub export fn tcl_cmd_load(path: i32) i32 {
    _ = path;
    stubs.unsupported("load");
    return 0;
}

pub export fn tcl_cmd_unload(path: i32) i32 {
    _ = path;
    stubs.unsupported("unload");
    return 0;
}
