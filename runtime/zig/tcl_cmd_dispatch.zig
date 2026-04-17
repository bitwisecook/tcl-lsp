// Interpreter-side pre-wired dispatch for every Tcl 8.4–9.0 core command.
//
// Philosophy: the interpreter must produce a meaningful result (or a
// clear trap) for *any* Tcl core command, without ever reaching
// ``unknown command: <name>``.  A user-defined proc that shadows a
// core command is handled by the proc registry (consulted in
// ``eval_proc_call`` *before* this dispatch table), so shadowing
// still works.  Commands in this file fall into three groups:
//
//   (a) **Real** — the command has a runtime implementation and we
//       dispatch to it (``encoding``, ``fconfigure``, …).
//
//   (b) **Stub** — the command has no WASM-available implementation
//       (``regexp``, ``file``, ``exec``, …); we trap with
//       ``unsupported command: X`` so the caller sees a precise
//       diagnostic.
//
//   (c) **Already handled in eval_command** — the hot-path builtins
//       (``set``, ``puts``, ``expr``, ``if``, ``while``, ``proc``,
//       ``string``, ``dict``, ``info``, ``array``, …) are not listed
//       here because they're checked with ``str_eq`` in the
//       interpreter's main switch before we're ever called.  They
//       still count as "pre-wired" — this file only covers the
//       remaining core surface.
//
// ``try_stub`` is called from ``eval_proc_call`` when proc_lookup
// misses.  It returns true if it handled the command (real impl ran
// *or* stub trap fired); false to let the caller emit the generic
// ``unknown command`` error.

const stubs = @import("tcl_stubs.zig");

/// Consult the dispatch table.  Returns true when the command name
/// matched a Tcl 8.4–9.0 core command — either executing it or
/// trapping through :func:`tcl_stubs.unsupported`.
pub fn try_stub(cmd: [*]const u8, cmd_len: u32) bool {
    return match_stub(cmd[0..cmd_len]);
}

fn eql(a: []const u8, b: []const u8) bool {
    if (a.len != b.len) return false;
    for (a, b) |x, y| if (x != y) return false;
    return true;
}

fn match_stub(c: []const u8) bool {
    // I/O + channel — ``fconfigure`` has a real impl in tcl_chan.zig;
    // it's dispatched directly from ``eval_command`` before we're
    // reached.  The rest are trapping stubs.
    if (eql(c, "open")) return trap("open");
    if (eql(c, "close")) return trap("close");
    if (eql(c, "read")) return trap("read");
    if (eql(c, "gets")) return trap("gets");
    if (eql(c, "eof")) return trap("eof");
    if (eql(c, "flush")) return trap("flush");
    if (eql(c, "fblocked")) return trap("fblocked");
    if (eql(c, "tell")) return trap("tell");
    if (eql(c, "seek")) return trap("seek");
    if (eql(c, "chan")) return trap("chan");
    if (eql(c, "fcopy")) return trap("fcopy");
    if (eql(c, "fileevent")) return trap("fileevent");
    if (eql(c, "socket")) return trap("socket");

    // Filesystem / process.
    // ``file`` has a real impl in tcl_fs.zig (string-path manip,
    // always-false existence, trapping mutations); dispatched in
    // eval_command directly.
    if (eql(c, "glob")) return trap("glob");
    // pwd / cd have real impls in tcl_fs.zig; they're wired directly
    // in eval_command so the interpreter never reaches this table
    // for them.
    if (eql(c, "exec")) return trap("exec");
    if (eql(c, "source")) return trap("source");
    if (eql(c, "load")) return trap("load");
    if (eql(c, "unload")) return trap("unload");

    // Format / pattern matching.  ``encoding`` lives in
    // ``eval_command`` (real impl) and isn't listed here.
    // ``format`` has a real runtime impl in tcl_format.zig.  It's
    // dispatched through eval_command for interpreter-path calls,
    // so we decline here — returning false lets the normal
    // proc-lookup / error path run (but eval_command should have
    // caught it before getting here).
    if (eql(c, "format")) return false;
    if (eql(c, "scan")) return trap("scan");
    if (eql(c, "binary")) return trap("binary");
    if (eql(c, "regexp")) return trap("regexp");
    if (eql(c, "regsub")) return trap("regsub");

    // Time / event loop.  ``clock seconds`` / ``clock clicks`` /
    // ``clock milliseconds`` are hot-path and handled in
    // ``eval_command``; the formatting subcommands need per-subcmd
    // discrimination and surface as ``unsupported command: clock
    // <sub>`` from tcl_time_stubs.
    if (eql(c, "after")) return trap("after");
    if (eql(c, "vwait")) return trap("vwait");
    if (eql(c, "update")) return trap("update");
    if (eql(c, "coroutine")) return trap("coroutine");
    if (eql(c, "yield")) return trap("yield");
    if (eql(c, "yieldto")) return trap("yieldto");
    if (eql(c, "yieldmeta")) return trap("yieldmeta");
    if (eql(c, "tailcall")) return trap("tailcall");

    // Environment / metadata.  ``namespace eval`` is handled by the
    // compiler (barrier fallback); this entry catches every other
    // ``namespace`` subcommand (current, qualifiers, which, tail,
    // code, delete, import, export, exists, parent, children,
    // inscope, origin, forget, path, ensemble).
    if (eql(c, "namespace")) return trap("namespace");
    if (eql(c, "package")) return trap("package");
    // ``trace`` is handled directly in eval_command via tcl_trace.zig
    // (pass-through add/remove, trap on info).
    if (eql(c, "interp")) return trap("interp");
    if (eql(c, "apply")) return trap("apply");
    if (eql(c, "rename")) return trap("rename");
    // ``subst`` is handled directly in eval_command with support for
    // -nobackslashes / -nocommands / -novariables flags.
    if (eql(c, "time")) return trap("time");
    // ``auto_load`` and friends are stdlib loaders — without the Tcl
    // stdlib there's nothing to load.  The correct Tcl return is 0
    // ("not auto-loaded") / empty string, but this bool-only
    // dispatcher can't communicate a TclObj result — returning true
    // here would drop the intended value and eval_proc_call would
    // emit null.  Return false instead so eval_proc_call falls
    // through to the unknown-command error, which in turn surfaces
    // as a clean ``unknown command: auto_load`` trap.  A future
    // refactor could have ``try_stub`` return ``(handled: bool,
    // result: i32)`` so stdlib stubs can supply their proper value.
    if (eql(c, "auto_load") or eql(c, "auto_reset") or eql(c, "auto_mkindex") or
        eql(c, "auto_import") or eql(c, "auto_execok") or eql(c, "auto_qualify") or
        eql(c, "unknown"))
    {
        return false;
    }
    if (eql(c, "try")) return trap("try");
    if (eql(c, "throw")) return trap("throw");
    // ``unknown`` is handled below as a pass-through no-op; a missing
    // ``unknown`` handler just means the stock "unknown command" error
    // triggers, which is exactly what we want here.

    // List commands that aren't in eval_command's hot path.
    if (eql(c, "lreplace")) return trap("lreplace");
    if (eql(c, "linsert")) return trap("linsert");
    if (eql(c, "lset")) return trap("lset");
    if (eql(c, "lreverse")) return trap("lreverse");
    if (eql(c, "lrepeat")) return trap("lrepeat");
    if (eql(c, "lmap")) return trap("lmap");
    if (eql(c, "lassign")) return trap("lassign");
    if (eql(c, "lseq")) return trap("lseq");

    // Object system (TclOO, 8.6+).
    if (eql(c, "oo::class")) return trap("oo::class");
    if (eql(c, "oo::define")) return trap("oo::define");
    if (eql(c, "oo::object")) return trap("oo::object");
    if (eql(c, "oo::copy")) return trap("oo::copy");
    if (eql(c, "self")) return trap("self");
    if (eql(c, "my")) return trap("my");
    if (eql(c, "next")) return trap("next");

    // Misc that tcltest specifically references.
    if (eql(c, "zlib")) return trap("zlib");
    if (eql(c, "registry")) return trap("registry");
    if (eql(c, "pid")) return trap("pid");

    return false;
}

/// Emit ``unsupported command: <name>`` and return true so the
/// dispatch caller treats the command as handled.
fn trap(comptime name: []const u8) bool {
    stubs.unsupported(name);
    return true;
}
