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
//   - file (mkdir, delete, exists, isfile, isdirectory, dirname,
//     tail, normalize, rootname, extension, join, split, pathtype,
//     readable, writable, executable, owned, size, mtime, atime,
//     stat, lstat, link, readlink, rename, copy, attributes,
//     tempfile, system, channels, volumes, separator, nativename)
//     is multiplexed through a single ``file`` stub — the subcommand
//     variance is captured in the site's ``args`` in the sidecar map.
//   - glob, pwd, cd
//   - exec, source, load, unload

const stubs = @import("tcl_stubs.zig");
const obj = @import("../valtypes/tcl_obj.zig");

// ``file`` moved to tcl_fs.zig — has pass-through implementations
// for string-only path manipulation (join / dirname / tail /
// rootname / extension / normalize / split / pathtype / separator
// / nativename), always-false answers for existence queries, and
// trapping behaviour for mutating ops (mkdir / delete / rename / …).

pub export fn tcl_cmd_glob(pattern: i32) i32 {
    _ = pattern;
    // In the WASM sandbox there is no real filesystem to glob.  Always return
    // an empty list — this matches ``glob -nocomplain`` behaviour and lets
    // callers like tcltest's cleanupTests proceed without trapping.
    return obj.obj_new_string(0, 0);
}

// ``pwd`` and ``cd`` moved to tcl_fs.zig — pwd returns "/" and cd
// silently accepts its arg.  Scripts that use them for logging /
// path-seed purposes (tcltest's ``workingDirectory`` option is the
// poster child) now load without tripping.

pub export fn tcl_cmd_exec(cmd: i32) i32 {
    _ = cmd;
    stubs.unsupported("exec");
    return 0;
}

pub export fn tcl_cmd_source(path: i32) i32 {
    _ = path;
    stubs.unsupported("source");
    return 0;
}

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
