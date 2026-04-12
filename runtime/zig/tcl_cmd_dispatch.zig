// Interpreter-side stub dispatch.
//
// When compiled code falls back to ``tcl_eval`` — typically because
// a ``namespace eval { ... }`` body is too dynamic to compile — the
// interpreter then walks commands one at a time through
// ``eval_command`` / ``eval_proc_call``.  For user-defined procs the
// existing proc-registry lookup wins; for *core* Tcl 8.4–9.0 commands
// we haven't implemented (``encoding``, ``fconfigure``, ``regexp``,
// ``trace``, ``file``, ``glob``, …) we want the same clean trap that
// direct compiled dispatch produces via the ``_CMD_RUNTIME`` path —
// ``unsupported command: <name>`` with the current diag site.
//
// This file owns that mapping.  Keeping the command-name → stub
// table in one place means adding a new stub is a one-line change
// here in addition to the area-specific ``tcl_*_stubs.zig`` file.
//
// Only *unsupported* commands are listed.  Implemented commands
// (``puts``, ``lappend``, ``list_index``, ``string *``, ``dict *``,
// ``array *``, ``clock_seconds`` / ``clock_clicks`` /
// ``clock_milliseconds``, ``info exists`` etc.) continue to be
// handled by their dedicated dispatch in ``eval_command`` —
// ``try_stub`` is consulted only when the proc-registry lookup
// misses.

const stubs = @import("tcl_stubs.zig");

/// Consult the stub dispatch table.  Returns true if the command was
/// a known stub (and therefore an ``unsupported command: <name>``
/// error has been raised through :func:`tcl_stubs.unsupported`);
/// returns false if the caller should continue to the unknown-command
/// error path.
pub fn try_stub(cmd: [*]const u8, cmd_len: u32) bool {
    return match_stub(cmd[0..cmd_len]);
}

fn match_stub(c: []const u8) bool {
    // Helper — Zig 0.13's std.mem.eql requires a slice pair but
    // keeping the match explicit saves on std dependency here.
    const eql = struct {
        fn f(a: []const u8, b: []const u8) bool {
            if (a.len != b.len) return false;
            for (a, b) |x, y| if (x != y) return false;
            return true;
        }
    }.f;

    // I/O + channel stubs (tcl_io_stubs.zig).
    if (eql(c, "open")) {
        stubs.unsupported("open");
        return true;
    }
    if (eql(c, "close")) {
        stubs.unsupported("close");
        return true;
    }
    if (eql(c, "read")) {
        stubs.unsupported("read");
        return true;
    }
    if (eql(c, "gets")) {
        stubs.unsupported("gets");
        return true;
    }
    if (eql(c, "eof")) {
        stubs.unsupported("eof");
        return true;
    }
    if (eql(c, "flush")) {
        stubs.unsupported("flush");
        return true;
    }
    if (eql(c, "fblocked")) {
        stubs.unsupported("fblocked");
        return true;
    }
    // ``fconfigure`` has a real (NOP) implementation in tcl_chan.zig
    // so a trap inside an eval-fallback that references it doesn't
    // derail; the interpreter still goes through the proc-registry
    // miss → this dispatch, so we need to call the real function.
    if (eql(c, "fconfigure")) {
        const chan = @import("tcl_chan.zig");
        _ = chan.fconfigure(0, 0);
        return true;
    }
    if (eql(c, "tell")) {
        stubs.unsupported("tell");
        return true;
    }
    if (eql(c, "seek")) {
        stubs.unsupported("seek");
        return true;
    }
    if (eql(c, "chan")) {
        stubs.unsupported("chan");
        return true;
    }
    if (eql(c, "fcopy")) {
        stubs.unsupported("fcopy");
        return true;
    }
    if (eql(c, "fileevent")) {
        stubs.unsupported("fileevent");
        return true;
    }
    if (eql(c, "socket")) {
        stubs.unsupported("socket");
        return true;
    }

    // Filesystem / process stubs (tcl_fs_stubs.zig).
    if (eql(c, "file")) {
        stubs.unsupported("file");
        return true;
    }
    if (eql(c, "glob")) {
        stubs.unsupported("glob");
        return true;
    }
    if (eql(c, "pwd")) {
        stubs.unsupported("pwd");
        return true;
    }
    if (eql(c, "cd")) {
        stubs.unsupported("cd");
        return true;
    }
    if (eql(c, "exec")) {
        stubs.unsupported("exec");
        return true;
    }
    if (eql(c, "source")) {
        stubs.unsupported("source");
        return true;
    }
    if (eql(c, "load")) {
        stubs.unsupported("load");
        return true;
    }
    if (eql(c, "unload")) {
        stubs.unsupported("unload");
        return true;
    }

    // Format / regex / encoding stubs (tcl_fmt_stubs.zig).
    if (eql(c, "format")) {
        stubs.unsupported("format");
        return true;
    }
    if (eql(c, "scan")) {
        stubs.unsupported("scan");
        return true;
    }
    if (eql(c, "binary")) {
        stubs.unsupported("binary");
        return true;
    }
    if (eql(c, "regexp")) {
        stubs.unsupported("regexp");
        return true;
    }
    if (eql(c, "regsub")) {
        stubs.unsupported("regsub");
        return true;
    }
    // ``encoding`` has a real (UTF-8 pass-through) implementation
    // in tcl_encoding.zig.  When the interpreter walks a fallback
    // script and hits an unknown ``encoding`` command, we'd want
    // that real implementation to run — but here we don't have the
    // ``words`` array, so we can't marshal subcommand + arg into
    // its signature.  Fall through to the unknown-command path;
    // the interpreter's ``eval_proc_call`` will still trap with
    // ``unknown command: encoding`` if ``words`` is malformed.
    // Direct dispatch from compiled code (the common case) is
    // still served by ``_CMD_RUNTIME`` → real ``encoding`` fn.
    // NOTE: reaching encoding through this path is an interpreter
    // edge case; add a sub-dispatcher here if real-world scripts
    // prove to need it.

    // Event-loop / coroutine stubs (tcl_time_stubs.zig).  ``clock``
    // is NOT in this table because ``clock seconds`` / ``clock
    // clicks`` / ``clock milliseconds`` are real runtime functions;
    // the formatting variants (``clock format`` / ``clock scan`` /
    // ``clock add``) would need a sub-dispatcher we haven't wired.
    // See wasm.py::_CMD_RUNTIME note.
    if (eql(c, "after")) {
        stubs.unsupported("after");
        return true;
    }
    if (eql(c, "vwait")) {
        stubs.unsupported("vwait");
        return true;
    }
    if (eql(c, "update")) {
        stubs.unsupported("update");
        return true;
    }
    if (eql(c, "coroutine")) {
        stubs.unsupported("coroutine");
        return true;
    }
    if (eql(c, "yield")) {
        stubs.unsupported("yield");
        return true;
    }
    if (eql(c, "yieldto")) {
        stubs.unsupported("yieldto");
        return true;
    }

    // Environment / metadata stubs (tcl_env_stubs.zig).  ``namespace``
    // eval is evaluated specially through the interpreter's own
    // namespace handling (coming in a follow-up); for now the stub
    // catches ``namespace current`` / ``namespace which`` / etc.
    // However the interpreter still NOPs ``namespace`` because it
    // can't know whether the caller meant ``namespace eval``.  Skip
    // the namespace stub here — the user will see the inner traps
    // once we grow real ``namespace eval`` support in the interpreter.
    if (eql(c, "package")) {
        stubs.unsupported("package");
        return true;
    }
    if (eql(c, "trace")) {
        stubs.unsupported("trace");
        return true;
    }
    if (eql(c, "interp")) {
        stubs.unsupported("interp");
        return true;
    }
    if (eql(c, "apply")) {
        stubs.unsupported("apply");
        return true;
    }

    return false;
}
