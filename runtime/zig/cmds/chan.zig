// ``encoding``, ``fconfigure``, ``chan`` — channel/encoding commands.
//
// The ``chan`` ensemble dispatches each subcommand back to the
// corresponding bare-form handler in :file:`cmds/io.zig` /
// :func:`eval_fconfigure`, so the two surfaces share one
// implementation.  Sub-commands without a runtime impl
// (``create`` / ``event`` / ``pipe`` / ``pop`` / ``postevent`` /
// ``push`` / ``truncate`` / ``isbinary`` / ``pending``) trap with
// ``unsupported command: chan <sub>`` rather than silently
// returning empty — matches the convention in
// :file:`tcl_stub_fallback.zig`.

const rt = @import("../tcl_runtime.zig");
const result_mod = @import("../interp/tcl_result.zig");
const enc = @import("../valtypes/tcl_encoding.zig");
const chan = @import("../io/tcl_chan.zig");
const io_cmd = @import("io.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");
const stubs = @import("../stubs/tcl_stubs.zig");
const obj = @import("../valtypes/tcl_obj.zig");
const list_quote = @import("../valtypes/tcl_list_quote.zig");
const tcl_string = @import("../valtypes/tcl_string.zig");

const alloc = rt.alloc;
const obj_new_string = rt.obj_new_string;
const obj_ensure_string = obj.obj_ensure_string;

fn eval_encoding(words: []const i32) result_mod.InterpResult {
    const sub = if (words.len >= 2) words[1] else 0;
    const arg1 = if (words.len >= 3) words[2] else 0;
    const arg2 = if (words.len >= 4) words[3] else 0;
    return result_mod.from_globals(enc.tcl_cmd_encoding(sub, arg1, arg2));
}

pub fn eval_fconfigure(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) return result_mod.from_globals(chan.tcl_cmd_fconfigure(0, 0));
    const fd = words[1];
    if (words.len < 3) return result_mod.from_globals(chan.tcl_cmd_fconfigure(fd, 0));

    // Build a properly-quoted Tcl list of the option words.  Plain
    // ``tcl_cmd_concat`` with " " separators would collapse empty
    // strings — ``fconfigure $fd -eofchar {}`` would arrive at the
    // runtime as a bare ``-eofchar`` and silently flip from setter
    // to query.  ``list_elem_quote_nth`` emits canonical list
    // elements (empty → ``{}``, whitespace / brace values get
    // brace-wrapped or backslash-escaped) so the consumer's
    // ``list_parse`` walk recovers the original bytes.
    var total_cap: u32 = 0;
    var i: u32 = 2;
    while (i < words.len) : (i += 1) {
        const s = obj.obj_ensure_string(words[i]);
        // Worst-case expansion per element: 2*len + 2 bytes (every
        // byte gets a backslash + outer braces) plus a separator.
        total_cap += s.len * 2 + 3;
    }
    if (total_cap == 0) total_cap = 1;
    const buf = alloc(total_cap);
    if (buf == 0) return result_mod.from_globals(chan.tcl_cmd_fconfigure(fd, 0));
    var off: u32 = 0;
    i = 2;
    while (i < words.len) : (i += 1) {
        if (i > 2) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        const s = obj.obj_ensure_string(words[i]);
        off = list_quote.list_elem_quote_nth(buf, off, s.ptr, s.len);
    }
    const args_obj = rt.obj_new_string_take(buf, off, total_cap);
    return result_mod.from_globals(chan.tcl_cmd_fconfigure(fd, args_obj));
}

// -- chan ensemble --
//
// ``chan <sub> <args…>`` routes to the bare-command handler for the
// matching subcommand.  Each handler in ``io_cmd`` reads
// ``words[0]`` purely as the command name (it never inspects the
// string), so we can pass ``words[1..]`` verbatim — slot 0 is the
// subcommand name (e.g. ``configure``), which is ignored by the
// downstream handler the same way ``puts`` ignores ``words[0]``.

fn name_eq(w: i32, literal: []const u8) bool {
    if (w == 0) return literal.len == 0;
    const s = obj_ensure_string(w);
    if (s.len != literal.len) return false;
    const p: [*]const u8 = @ptrFromInt(s.ptr);
    for (0..literal.len) |i| if (p[i] != literal[i]) return false;
    return true;
}

fn eval_chan_names(words: []const i32) i32 {
    // Distinguish "no pattern arg" (``chan names``) from "empty
    // pattern arg" (``chan names ""``).  Tcl semantics: an empty
    // pattern is a real glob that only matches the empty string,
    // so the latter must filter out every (non-empty) channel name.
    const has_pattern = words.len >= 2;
    var pattern_ptr: u32 = 0;
    var pattern_len: u32 = 0;
    if (has_pattern) {
        const ps = obj_ensure_string(words[1]);
        pattern_ptr = ps.ptr;
        pattern_len = ps.len;
    }
    var result: i32 = obj_new_string(0, 0);
    var slot: u32 = 0;
    while (slot < chan.channel_count) : (slot += 1) {
        if (!chan.slot_in_use(slot)) continue;
        const name = chan.channel_name(slot);
        if (has_pattern) {
            const ns = obj_ensure_string(name);
            if (!tcl_string.glob_match(pattern_ptr, pattern_len, ns.ptr, ns.len)) continue;
        }
        result = rt.tcl_cmd_lappend(result, name);
    }
    return result;
}

/// ``chan close $ch ?direction?`` — the optional direction word
/// requests a half-close (``read`` or ``write``), which the WASI
/// channel layer doesn't model.  Trap on the directional form
/// rather than silently dropping the arg, matching the
/// "no silent stubs" convention in :file:`tcl_stub_fallback.zig`.
/// Plain ``chan close $ch`` (no direction) routes through the
/// regular full-close path.
fn eval_chan_close(words: []const i32) i32 {
    if (words.len >= 3) {
        stubs.unsupported("chan close (half-close direction not implemented)");
        return 0;
    }
    return io_cmd.eval_close(words).value;
}

fn eval_chan(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) {
        stubs.raise("chan: missing subcommand");
        return result_mod.from_globals(0);
    }
    const sub = words[1];
    // Forward the slice ``words[1..]`` to the bare-command handler:
    // its ``words[0]`` becomes the subcommand name (ignored by the
    // delegate, which never reads ``words[0]``).
    const tail = words[1..];
    // Sub-handlers from io_cmd are themselves ``HandlerFn``-typed
    // (returning ``InterpResult``); forward the typed result directly
    // instead of double-wrapping.  Local helpers (eval_chan_close,
    // eval_fconfigure, eval_chan_names) still return i32 and need
    // ``from_globals`` to lift into a typed result.
    if (name_eq(sub, "blocked")) return io_cmd.eval_fblocked(tail);
    if (name_eq(sub, "close")) return result_mod.from_globals(eval_chan_close(tail));
    if (name_eq(sub, "configure")) return eval_fconfigure(tail);
    if (name_eq(sub, "copy")) return io_cmd.eval_fcopy(tail);
    if (name_eq(sub, "eof")) return io_cmd.eval_eof(tail);
    if (name_eq(sub, "flush")) return io_cmd.eval_flush(tail);
    if (name_eq(sub, "gets")) return io_cmd.eval_gets(tail);
    if (name_eq(sub, "names")) return result_mod.from_globals(eval_chan_names(words[1..]));
    if (name_eq(sub, "puts")) return io_cmd.eval_puts(tail);
    if (name_eq(sub, "read")) return io_cmd.eval_read(tail);
    if (name_eq(sub, "seek")) return io_cmd.eval_seek(tail);
    if (name_eq(sub, "tell")) return io_cmd.eval_tell(tail);
    // Out-of-scope subcommands (issue #270 explicitly defers these):
    // create / event / pipe / pop / postevent / push / truncate need
    // transformation channels, an event loop, or fd resizing that the
    // WASM runtime doesn't model.  ``isbinary`` and ``pending`` are
    // additional subcommands the Python registry advertises but that
    // we don't yet implement.  Trap with ``unsupported command`` to
    // match the convention in :file:`tcl_stub_fallback.zig`.
    if (name_eq(sub, "create") or name_eq(sub, "event") or
        name_eq(sub, "isbinary") or name_eq(sub, "pending") or
        name_eq(sub, "pipe") or name_eq(sub, "pop") or
        name_eq(sub, "postevent") or name_eq(sub, "push") or
        name_eq(sub, "truncate"))
    {
        stubs.unsupported("chan (subcommand not implemented)");
        return result_mod.from_globals(0);
    }
    stubs.raise("chan: unknown subcommand");
    return result_mod.from_globals(0);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "encoding", .arity_min = 1, .arity_max = null, .handler = &eval_encoding },
    .{ .name = "fconfigure", .arity_min = 1, .arity_max = null, .handler = &eval_fconfigure },
    .{ .name = "chan", .arity_min = 1, .arity_max = null, .handler = &eval_chan },
};

// Per-command sub-tables.  Arities mirror
// ``core/commands/registry/tcl/chan.py``; the parity check enforces
// every entry here matches the Python ``SubCommand.arity`` declaration.
// The handler pointer is metadata-only — runtime dispatch happens
// inside :func:`eval_chan` (and similarly inside the bare-command
// handlers for fconfigure / encoding sub-forms).
pub const chan_subcommands: []const reg.SubEntry = &.{
    .{ .name = "blocked", .arity_min = 1, .arity_max = 1, .handler = &eval_chan },
    .{ .name = "close", .arity_min = 1, .arity_max = 2, .handler = &eval_chan },
    .{ .name = "configure", .arity_min = 1, .arity_max = null, .handler = &eval_chan },
    .{ .name = "copy", .arity_min = 2, .arity_max = null, .handler = &eval_chan },
    .{ .name = "create", .arity_min = 2, .arity_max = 2, .handler = &eval_chan },
    .{ .name = "eof", .arity_min = 1, .arity_max = 1, .handler = &eval_chan },
    .{ .name = "event", .arity_min = 2, .arity_max = 3, .handler = &eval_chan },
    .{ .name = "flush", .arity_min = 1, .arity_max = 1, .handler = &eval_chan },
    .{ .name = "gets", .arity_min = 1, .arity_max = 2, .handler = &eval_chan },
    .{ .name = "isbinary", .arity_min = 1, .arity_max = 1, .handler = &eval_chan },
    .{ .name = "names", .arity_min = 0, .arity_max = 1, .handler = &eval_chan },
    .{ .name = "pending", .arity_min = 2, .arity_max = 2, .handler = &eval_chan },
    .{ .name = "pipe", .arity_min = 0, .arity_max = 0, .handler = &eval_chan },
    .{ .name = "pop", .arity_min = 1, .arity_max = 1, .handler = &eval_chan },
    .{ .name = "postevent", .arity_min = 2, .arity_max = 2, .handler = &eval_chan },
    .{ .name = "push", .arity_min = 2, .arity_max = 2, .handler = &eval_chan },
    .{ .name = "puts", .arity_min = 1, .arity_max = 3, .handler = &eval_chan },
    .{ .name = "read", .arity_min = 1, .arity_max = 2, .handler = &eval_chan },
    .{ .name = "seek", .arity_min = 2, .arity_max = 3, .handler = &eval_chan },
    .{ .name = "tell", .arity_min = 1, .arity_max = 1, .handler = &eval_chan },
    .{ .name = "truncate", .arity_min = 1, .arity_max = 2, .handler = &eval_chan },
};

// ``encoding`` sub-table — kept after ``chan_subcommands`` so the
// generic-vs-named lookup in ``check_wasm_command_parity.py`` resolves
// cleanly.  ``fconfigure`` is variadic ``-option value`` pairs, not
// an ensemble.
pub const encoding_subcommands: []const reg.SubEntry = &.{
    .{ .name = "convertfrom", .arity_min = 1, .arity_max = null, .handler = &eval_encoding },
    .{ .name = "convertto", .arity_min = 1, .arity_max = null, .handler = &eval_encoding },
    .{ .name = "dirs", .arity_min = 0, .arity_max = 1, .handler = &eval_encoding },
    .{ .name = "names", .arity_min = 0, .arity_max = 0, .handler = &eval_encoding },
    .{ .name = "profiles", .arity_min = 0, .arity_max = 0, .handler = &eval_encoding },
    .{ .name = "system", .arity_min = 0, .arity_max = 1, .handler = &eval_encoding },
    .{ .name = "user", .arity_min = 0, .arity_max = 0, .handler = &eval_encoding },
};
