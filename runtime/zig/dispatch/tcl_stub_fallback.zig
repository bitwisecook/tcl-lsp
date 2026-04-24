// Fallback stub dispatch for Tcl 8.4–9.0 core commands that aren't
// wired up in the interpreter's main BUILTINS table and aren't
// shadowed by a user-defined proc.  Consulted last by
// :func:`tcl_interp.eval_proc_call` when both proc-lookup and the
// BUILTINS table miss.
//
// Philosophy: the interpreter must produce a meaningful result (or a
// clear trap) for *any* Tcl core command, without ever reaching
// ``unknown command: <name>``.  A user-defined proc that shadows a
// core command is handled by the proc registry (consulted in
// ``eval_proc_call`` *before* this dispatch table), so shadowing
// still works.  Commands fall into three groups:
//
//   (a) **Real** — the command has a runtime implementation and we
//       dispatch to it (``encoding``, ``fconfigure``, …).  Those
//       live in BUILTINS (``cmds/*.zig``) and never reach this file.
//
//   (b) **Stub** — the command has no WASM-available implementation
//       (``regexp``, ``file``, ``exec``, …); :func:`try_stub` traps
//       with ``unsupported command: X`` via :func:`tcl_stubs.unsupported`
//       so the caller sees a precise diagnostic.  Names live in
//       :data:`STUB_TRAP`.
//
//   (c) **Already handled in eval_command** — the hot-path builtins
//       (``set``, ``puts``, ``expr``, ``if``, ``while``, ``proc``,
//       ``string``, ``dict``, ``info``, ``array``, …) are checked by
//       BUILTINS before we're ever called.  A handful of commands
//       (``format``, ``unknown``) appear in :data:`STUB_FALLTHROUGH`
//       so this file documents that their absence from a trap is
//       deliberate — they're served elsewhere or should fall through
//       to the generic "unknown command" error path.

const stubs = @import("../stubs/tcl_stubs.zig");

/// Command names that trap with ``unsupported command: X`` when
/// reached.  Kept as a data table instead of an if-chain so adding a
/// new stub is one line.  Order doesn't matter — the lookup is a
/// linear scan.
const STUB_TRAP: []const []const u8 = &.{
    // I/O + channel — fconfigure has a real impl dispatched by BUILTINS.
    "open", "close", "read",     "gets",      "eof",     "flush",
    "fblocked", "tell", "seek",  "chan",      "fcopy",   "fileevent",
    "socket",
    // Filesystem / process — file/pwd/cd have real impls in BUILTINS.
    "glob", "exec", "source", "load", "unload",
    // Format / pattern matching — scan/binary are in BUILTINS.
    "regexp",
    // Time / event loop.  ``after``, ``vwait``, ``update``, ``clock``
    // are all handled by BUILTINS (``cmds/stubs.zig``) before we
    // reach this table, so they are intentionally absent.
    "coroutine", "yield", "yieldto",
    // Environment / metadata — ``package`` is in BUILTINS
    // (cmds/stubs.zig); ``trace`` / ``namespace`` / ``subst`` /
    // ``auto_*`` are in BUILTINS with real implementations.
    "interp", "rename",
    // Object system (TclOO, 8.6+).
    "oo::class", "oo::define", "oo::object", "oo::copy",
    "oo::abstract", "oo::configurable", "oo::objdefine", "oo::singleton",
    "self", "my", "next", "nextto", "classvariable",
    // Misc that tcltest specifically references.
    "zlib", "registry",
    // Core commands without WASM impl yet.
    // ``switch`` is compiled inline by the code generator (IRSwitch),
    // so this entry only fires when the interpreter evaluates a
    // dynamic ``switch`` string — acceptable to trap until a runtime
    // evaluator lands.
    "switch",
    // ``exit`` terminates the interpreter; in the sandbox we trap
    // rather than silently exit (callers can catch).
    "exit",
    // Debug-only / host-machine commands.
    "memory", "bgerror",
    // Tcl 9.0 additions missing in the WASM runtime.
    "lremove",
    // Standard-library procs Tcl ships in ``library/*.tcl``.
    // Without the library source bundled we trap rather than silently
    // return empty — callers relying on these can install shims.
    "auto_mkindex_old", "parray", "pkg_mkindex", "pkg::create",
    "tcl_findLibrary",
    // Regexp-quoting helpers — several aliases cover historical
    // spellings.
    "re_quote", "regex_quote", "regex::quote", "regexp::quote",
    // Package namespaces — ``http`` is an extension package.  Call
    // sites like ``http::geturl`` would already dispatch by the full
    // qualified name; the bare ``http`` trap catches misuse.
    "http",
    // ``filename`` is a documentation page, not a real command.  A
    // trap matches the invalid-command-name error Tcl would raise.
    "filename",
    // ``unknown`` is the user-overridable miss hook.  Stock Tcl
    // synthesises an ``unknown command`` error when no override is
    // installed; our sandbox has no override mechanism, so trap.
    "unknown",
};

// Commands deliberately absent from STUB_TRAP (documentation only, no
// code references this list — it's here for a reader grep'ing for a
// command name):
//
//   * ``format`` — real runtime impl in tcl_format.zig, dispatched
//     through BUILTINS before we're reached.

fn eql(a: []const u8, b: []const u8) bool {
    if (a.len != b.len) return false;
    for (a, b) |x, y| if (x != y) return false;
    return true;
}

/// Emit ``unsupported command: <name>`` and return true so the
/// dispatch caller treats the command as handled.
fn trap(name: []const u8) bool {
    stubs.unsupported(name);
    return true;
}

/// Consult the fallback dispatch table.  Returns true when the command
/// name matched :data:`STUB_TRAP` (trap fired) — the caller should
/// treat the command as handled.  Returns false for any name not in
/// the list, letting the caller emit the generic ``unknown command``
/// error.
pub fn try_stub(cmd: [*]const u8, cmd_len: u32) bool {
    const name = cmd[0..cmd_len];
    for (STUB_TRAP) |n| {
        if (eql(name, n)) return trap(n);
    }
    return false;
}
