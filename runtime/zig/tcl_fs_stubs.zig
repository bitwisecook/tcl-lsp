// Filesystem + process stubs — Tcl 8.4–9.0 commands that operate
// on the host filesystem or launch subprocesses.  Pure WASM has no
// real filesystem (WASI preopens a narrow slice) so these all raise
// ``unsupported command: <name>``.
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

// ``file`` moved to tcl_fs.zig — has pass-through implementations
// for string-only path manipulation (join / dirname / tail /
// rootname / extension / normalize / split / pathtype / separator
// / nativename), always-false answers for existence queries, and
// trapping behaviour for mutating ops (mkdir / delete / rename / …).

pub export fn glob(pattern: i32) i32 {
    _ = pattern;
    stubs.unsupported("glob");
    return 0;
}

// ``pwd`` and ``cd`` moved to tcl_fs.zig — pwd returns "/" and cd
// silently accepts its arg.  Scripts that use them for logging /
// path-seed purposes (tcltest's ``workingDirectory`` option is the
// poster child) now load without tripping.

pub export fn exec(cmd: i32) i32 {
    _ = cmd;
    stubs.unsupported("exec");
    return 0;
}

pub export fn source(path: i32) i32 {
    _ = path;
    stubs.unsupported("source");
    return 0;
}

pub export fn load(path: i32) i32 {
    _ = path;
    stubs.unsupported("load");
    return 0;
}

pub export fn unload(path: i32) i32 {
    _ = path;
    stubs.unsupported("unload");
    return 0;
}
