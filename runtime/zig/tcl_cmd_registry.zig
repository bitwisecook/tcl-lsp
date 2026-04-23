// Command registration type and linear-scan lookup.
//
// Provides the ``CmdEntry`` struct that every per-command module
// exports as its ``registration`` constant, and the ``lookup``
// helper used by ``tcl_cmd_table.zig``.
//
// This file has no imports from the rest of the runtime so it can be
// pulled in by any command module without creating circular deps.
//
// Design intent (Zig analogue of the Python ``@verb`` decorator):
//
//   Each command module declares:
//
//       pub const registration = reg.CmdEntry{
//           .name    = "string",
//           .handler = &eval,
//       };
//
//   ``tcl_cmd_table.zig`` collects all of these into a comptime
//   ``BUILTINS`` slice and exposes a single ``lookup`` entry point.
//   ``tcl_interp.zig:eval_command`` probes that table after the
//   proc-registry fast path and before the legacy if-else chain.

/// Handler function type shared by every registered builtin command.
/// words[0] is the command name; words[1..] are the arguments.
/// Returns a TclObj (i32 handle into WASM linear memory).
pub const HandlerFn = *const fn (words: []const i32) i32;

/// One registered builtin command.
pub const CmdEntry = struct {
    /// Canonical command name as a comptime string literal (e.g. "string").
    name: []const u8,
    /// Command implementation.
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
