// Command registration type and linear-scan lookup.
//
// Provides the ``CmdEntry`` struct that every per-command module
// exports as its ``registration`` constant, the ``SubEntry`` struct
// that commands with sub-commands use to declare them, and the
// ``lookup`` helper used by ``tcl_cmd_table.zig``.
//
// This file has no imports from the rest of the runtime so it can be
// pulled in by any command module without creating circular deps.
//
// Design intent (Zig analogue of the Python ``@verb`` decorator):
//
//   Each command module declares:
//
//       pub const registration = reg.CmdEntry{
//           .name      = "string",
//           .arity_min = 1,
//           .arity_max = null,            // variadic
//           .handler   = &eval,
//       };
//
//   ``tcl_cmd_table.zig`` collects all of these into a comptime
//   ``BUILTINS`` slice and exposes ``lookup()``/``lookup_entry()``
//   entry points.  ``tcl_interp.zig:eval_command`` probes that table
//   after the proc-registry fast path and before the legacy if-else
//   chain.
//
// The Python ``CommandSpec`` / ``SubCommand`` registry is the
// authoritative source for ``arity_min`` / ``arity_max`` (Phase F of
// the refactor).  ``scripts/check_wasm_command_parity.py`` enforces
// that both sides agree on every (command, arity) and (command,
// sub-command, arity) triple.

/// Handler function type shared by every registered builtin command.
/// words[0] is the command name; words[1..] are the arguments.
/// Returns a TclObj (i32 handle into WASM linear memory).
pub const HandlerFn = *const fn (words: []const i32) i32;

/// One registered builtin command.
///
/// ``arity_min`` and ``arity_max`` count *arguments*, not including
/// the command name itself.  ``set x 42`` has 2 arguments; ``set x``
/// has 1; ``set`` has 0.  ``arity_max == null`` means variadic
/// (accepts any number of trailing args).
///
/// Defaults are ``arity_min = 0`` and ``arity_max = null`` so old
/// registrations that haven't been annotated yet compile as
/// variadic.  Phase F.2 fills in explicit values for every entry;
/// the parity check in ``scripts/check_wasm_command_parity.py``
/// cross-verifies against the Python ``CommandSpec.validation.arity``
/// on every build.
pub const CmdEntry = struct {
    /// Canonical command name as a comptime string literal (e.g. "string").
    name: []const u8,
    /// Minimum number of arguments (not counting the command name).
    arity_min: u32 = 0,
    /// Maximum number of arguments, or ``null`` for variadic.
    arity_max: ?u32 = null,
    /// Command implementation.
    handler: HandlerFn,
};

/// One registered sub-command (e.g. ``string length``, ``dict get``).
///
/// Mirrors ``CmdEntry`` for commands with sub-dispatch.  Each command
/// that takes sub-commands exports a ``pub const subcommands: []const
/// SubEntry`` slice; the parity tool cross-checks against the Python
/// ``SubCommand`` entries on the parent ``CommandSpec``.
///
/// Arity here counts arguments after the sub-command name:
/// ``string length foo`` has a sub-arity of 1 (``foo``), not 2.
pub const SubEntry = struct {
    /// Sub-command name (e.g. "length").
    name: []const u8,
    /// Minimum number of arguments after the sub-command name.
    arity_min: u32 = 0,
    /// Maximum number of arguments, or ``null`` for variadic.
    arity_max: ?u32 = null,
    /// Sub-command implementation — takes the full ``words`` slice
    /// (words[0] = parent command, words[1] = sub-command name,
    /// words[2..] = sub-arguments).
    handler: HandlerFn,
};

/// Linear scan over ``entries`` matching ``name_ptr``/``name_len``.
/// Returns the handler, or null on miss.
///
/// For the current builtin count (~50) a linear scan is faster than
/// a hash probe at realistic working-set sizes: the name bytes are
/// in the data segment (hot in L1), branching on length first skips
/// most entries in one compare, and the actual command execution
/// dominates time.  Switch to a perfect-hash if benchmarks show
/// otherwise.
pub fn lookup(
    entries: []const CmdEntry,
    name_ptr: u32,
    name_len: u32,
) ?HandlerFn {
    if (name_len == 0) return null;
    const name: [*]const u8 = @ptrFromInt(name_ptr);
    for (entries) |e| {
        if (e.name.len != name_len) continue;
        var match = true;
        for (0..name_len) |i| {
            if (name[i] != e.name[i]) {
                match = false;
                break;
            }
        }
        if (match) return e.handler;
    }
    return null;
}

/// Like :fn:`lookup` but returns the full ``CmdEntry`` so callers can
/// validate arity as well as dispatch the handler.  Returns ``null``
/// on miss.
pub fn lookup_entry(
    entries: []const CmdEntry,
    name_ptr: u32,
    name_len: u32,
) ?CmdEntry {
    if (name_len == 0) return null;
    const name: [*]const u8 = @ptrFromInt(name_ptr);
    for (entries) |e| {
        if (e.name.len != name_len) continue;
        var match = true;
        for (0..name_len) |i| {
            if (name[i] != e.name[i]) {
                match = false;
                break;
            }
        }
        if (match) return e;
    }
    return null;
}

/// Look up a sub-command by name within an array of ``SubEntry``
/// entries.  Returns the matched entry or ``null`` on miss.
///
/// Used by command modules that have dispatched on a sub-command
/// name — typically the second-word sub-command pattern seen in
/// ``string``, ``dict``, ``clock``, ``info``, etc.
pub fn lookup_sub(
    subs: []const SubEntry,
    name_ptr: u32,
    name_len: u32,
) ?SubEntry {
    if (name_len == 0) return null;
    const name: [*]const u8 = @ptrFromInt(name_ptr);
    for (subs) |s| {
        if (s.name.len != name_len) continue;
        var match = true;
        for (0..name_len) |i| {
            if (name[i] != s.name[i]) {
                match = false;
                break;
            }
        }
        if (match) return s;
    }
    return null;
}
