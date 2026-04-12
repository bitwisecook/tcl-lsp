// I/O + channel stubs for Tcl 8.4–9.0 core commands we haven't
// implemented.  Each stub raises ``unsupported command: <name>``
// through :func:`tcl_stubs.unsupported`.
//
// These supersede the silent-0 stubs that used to live in
// ``tcl_catch.zig`` (format / regexp / open / close / read / gets).
// Splitting them out per area keeps ``tcl_catch.zig`` focused on
// catch/return signal machinery and lets us grow the stub surface
// without further cluttering that file.
//
// Coverage:
//   - open, close, read, gets, eof, flush, fblocked, fconfigure
//   - tell, seek, chan, fcopy
//   - fileevent, socket
//
// ``puts`` is *implemented* in tcl_io.zig — no stub here.

const stubs = @import("tcl_stubs.zig");

pub export fn open(path: i32) i32 {
    _ = path;
    stubs.unsupported("open");
    return 0;
}

pub export fn close(fd: i32) i32 {
    _ = fd;
    stubs.unsupported("close");
    return 0;
}

pub export fn read(fd: i32) i32 {
    _ = fd;
    stubs.unsupported("read");
    return 0;
}

pub export fn gets(fd: i32) i32 {
    _ = fd;
    stubs.unsupported("gets");
    return 0;
}

pub export fn eof(fd: i32) i32 {
    _ = fd;
    stubs.unsupported("eof");
    return 0;
}

pub export fn flush(fd: i32) i32 {
    _ = fd;
    stubs.unsupported("flush");
    return 0;
}

pub export fn fblocked(fd: i32) i32 {
    _ = fd;
    stubs.unsupported("fblocked");
    return 0;
}

pub export fn fconfigure(fd: i32, opts: i32) i32 {
    _ = fd;
    _ = opts;
    stubs.unsupported("fconfigure");
    return 0;
}

pub export fn tell(fd: i32) i32 {
    _ = fd;
    stubs.unsupported("tell");
    return 0;
}

pub export fn seek(fd: i32, offset: i32) i32 {
    _ = fd;
    _ = offset;
    stubs.unsupported("seek");
    return 0;
}

pub export fn chan(sub: i32, arg: i32) i32 {
    _ = sub;
    _ = arg;
    stubs.unsupported("chan");
    return 0;
}

pub export fn fcopy(in_fd: i32, out_fd: i32) i32 {
    _ = in_fd;
    _ = out_fd;
    stubs.unsupported("fcopy");
    return 0;
}

pub export fn fileevent(fd: i32, mode: i32) i32 {
    _ = fd;
    _ = mode;
    stubs.unsupported("fileevent");
    return 0;
}

pub export fn socket(host: i32, port: i32) i32 {
    _ = host;
    _ = port;
    stubs.unsupported("socket");
    return 0;
}
