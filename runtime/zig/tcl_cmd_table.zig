// Central command dispatch table.
//
// Assembles the BUILTINS slice from all per-command modules and
// exposes a single ``lookup`` entry point consumed by
// ``tcl_interp.zig:eval_command``.
//
// To add a new command:
//   1. Create ``cmds/<name>.zig`` exporting a ``registration``
//      constant of type ``tcl_cmd_registry.CmdEntry``.
//   2. Add a ``const <name>_cmd = @import("cmds/<name>.zig")`` line.
//   3. Append ``<name>_cmd.registration`` to the BUILTINS slice.
//
// No other file needs to change — eval_command probes this table
// before the legacy if-else chain, so new commands take effect
// automatically.

const reg = @import("tcl_cmd_registry.zig");

const string_cmd = @import("cmds/string.zig");
const array_cmd  = @import("cmds/array.zig");
const dict_cmd   = @import("cmds/dict.zig");

const BUILTINS: []const reg.CmdEntry = &.{
    string_cmd.registration,
    array_cmd.registration,
    dict_cmd.registration,
};

/// Look up a command by name.  Returns the handler function pointer on
/// hit, null on miss.  Called from ``tcl_interp.zig:eval_command``
/// after the proc-registry fast path.
pub fn lookup(name_ptr: u32, name_len: u32) ?reg.HandlerFn {
    return reg.lookup(BUILTINS, name_ptr, name_len);
}
