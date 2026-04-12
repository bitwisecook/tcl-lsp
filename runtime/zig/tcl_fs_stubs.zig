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

pub export fn file(sub: i32, arg: i32) i32 {
    _ = sub;
    _ = arg;
    stubs.unsupported("file");
    return 0;
}

pub export fn glob(pattern: i32) i32 {
    _ = pattern;
    stubs.unsupported("glob");
    return 0;
}

pub export fn pwd() i32 {
    stubs.unsupported("pwd");
    return 0;
}

pub export fn cd(dir: i32) i32 {
    _ = dir;
    stubs.unsupported("cd");
    return 0;
}

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
