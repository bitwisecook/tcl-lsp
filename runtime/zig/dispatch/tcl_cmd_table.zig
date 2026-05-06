// Central command dispatch table.
//
// Assembles the BUILTINS slice from all per-command modules and
// exposes a single ``lookup`` entry point consumed by
// ``tcl_interp.zig:eval_command``.
//
// To add a new command:
//   1. Create ``cmds/<name>.zig`` exporting a ``registrations``
//      constant of type ``[N]tcl_cmd_registry.CmdEntry``.
//   2. Add a ``const <name>_cmd = @import("cmds/<name>.zig")`` line.
//   3. Append ``++ <name>_cmd.registrations`` to the BUILTINS slice.
//
// No other file needs to change — eval_command probes this table
// before the legacy if-else chain, so new commands take effect
// automatically.

const reg = @import("tcl_cmd_registry.zig");

const string_cmd = @import("../cmds/string.zig");
const array_cmd = @import("../cmds/array.zig");
const dict_cmd = @import("../cmds/dict.zig");
const var_cmd = @import("../cmds/var.zig");
const scope_cmd = @import("../cmds/scope.zig");
const flow_cmd = @import("../cmds/flow.zig");
const loop_cmd = @import("../cmds/loop.zig");
const eval_cmd = @import("../cmds/eval.zig");
const proc_cmd = @import("../cmds/proc.zig");
const list_cmd = @import("../cmds/list.zig");
const io_cmd = @import("../cmds/io.zig");
const chan_cmd = @import("../cmds/chan.zig");
const fs_cmd = @import("../cmds/fs.zig");
const subst_cmd = @import("../cmds/subst.zig");
const regexp_cmd = @import("../cmds/regexp.zig");
const regsub_cmd = @import("../cmds/regsub.zig");
const inspect_cmd = @import("../cmds/inspect.zig");
const namespace_cmd = @import("../cmds/namespace.zig");
const interp_cmd = @import("../cmds/interp.zig");
const stubs_cmd = @import("../cmds/stubs.zig");
const scan_cmd = @import("../cmds/scan.zig");
const binary_cmd = @import("../cmds/binary.zig");
const oo_cmd = @import("../cmds/oo.zig");
const after_cmd = @import("../cmds/after.zig");
const event_cmd = @import("../cmds/event.zig");
const coroutine_cmd = @import("../cmds/coroutine.zig");
const fileevent_cmd = @import("../cmds/fileevent.zig");
const exec_cmd = @import("../cmds/exec.zig");
const exit_cmd = @import("../cmds/exit.zig");
const mathop_cmd = @import("../cmds/tcl_mathop.zig");

const BUILTINS: []const reg.CmdEntry = &([_]reg.CmdEntry{string_cmd.registration} ++
    [_]reg.CmdEntry{array_cmd.registration} ++
    [_]reg.CmdEntry{dict_cmd.registration} ++
    var_cmd.registrations ++
    scope_cmd.registrations ++
    flow_cmd.registrations ++
    loop_cmd.registrations ++
    eval_cmd.registrations ++
    proc_cmd.registrations ++
    list_cmd.registrations ++
    io_cmd.registrations ++
    chan_cmd.registrations ++
    fs_cmd.registrations ++
    subst_cmd.registrations ++
    regexp_cmd.registrations ++
    regsub_cmd.registrations ++
    inspect_cmd.registrations ++
    namespace_cmd.registrations ++
    interp_cmd.registrations ++
    stubs_cmd.registrations ++
    scan_cmd.registrations ++
    binary_cmd.registrations ++
    oo_cmd.registrations ++
    after_cmd.registrations ++
    event_cmd.registrations ++
    coroutine_cmd.registrations ++
    fileevent_cmd.registrations ++
    exec_cmd.registrations ++
    exit_cmd.registrations ++
    mathop_cmd.registrations);

/// Look up a command by name.  Returns the handler function pointer on
/// hit, null on miss.  Called from ``tcl_interp.zig:eval_command``
/// after the proc-registry fast path.
pub fn lookup(name_ptr: u32, name_len: u32) ?reg.HandlerFn {
    return reg.lookup(BUILTINS, name_ptr, name_len);
}

/// Read access to the BUILTINS slice.  ``info commands`` walks every
/// registered builtin so an ``info commands t*`` glob can find
/// ``try`` / ``time`` / ``trace`` etc. — these aren't in any
/// namespace's cmd_table.
pub fn entries() []const reg.CmdEntry {
    return BUILTINS;
}
