// ``puts``, ``append``, ``format`` — I/O and string output commands.
// ``scan`` moved to cmds/scan.zig for full multi-varname support.

const rt       = @import("../tcl_runtime.zig");
const frames   = @import("../interp/tcl_frames.zig");
const fmt_mod  = @import("../valtypes/tcl_format.zig");
const reg      = @import("../dispatch/tcl_cmd_registry.zig");

fn eval_puts(words: []const i32) i32 {
    if (words.len >= 2) return rt.tcl_cmd_puts(words[words.len - 1]);
    return 0;
}

/// ``flush ?channelId?`` — under WASI our writes are synchronous /
/// unbuffered so flush is a no-op that returns the empty string,
/// matching tclsh's contract.  Without this entry the interpreter
/// route (e.g. ``flush`` issued from inside a tcltest body
/// uplevel'd by ``test``/``Eval``/``RunTest``) hits ``STUB_TRAP``
/// and fails with ``unsupported command: flush``, breaking any
/// script that explicitly flushes between writes.  The compiled
/// fast path in the codegen calls ``tcl_cmd_flush`` directly and
/// has always worked.
fn eval_flush(words: []const i32) i32 {
    _ = words;
    return 0;
}

fn eval_append(words: []const i32) i32 {
    if (words.len >= 2) {
        var result = frames.var_resolve(words[1]);
        var wi: u32 = 2;
        while (wi < words.len) : (wi += 1) {
            result = rt.tcl_cmd_append(result, words[wi]);
        }
        _ = frames.var_set(words[1], result);
        return result;
    }
    return 0;
}

fn eval_format(words: []const i32) i32 {
    const fmt  = if (words.len >= 2) words[1] else 0;
    const a1   = if (words.len >= 3) words[2] else 0;
    const a2   = if (words.len >= 4) words[3] else 0;
    const a3   = if (words.len >= 5) words[4] else 0;
    return fmt_mod.tcl_cmd_format(fmt, a1, a2, a3);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "puts", .arity_min = 1, .arity_max = 2, .handler = &eval_puts },
    // ``flush ?channelId?`` — strict tclsh requires a channel, but
    // tcltest's ``Eval`` / ``RunTest`` interleave bare ``flush`` calls
    // between writes (see eval_flush docstring), so accept both
    // shapes as no-ops under WASI's synchronous-write semantics.
    .{ .name = "flush", .arity_min = 0, .arity_max = 1, .handler = &eval_flush },
    .{ .name = "append", .arity_min = 1, .arity_max = null, .handler = &eval_append },
    .{ .name = "format", .arity_min = 1, .arity_max = null, .handler = &eval_format },
};
