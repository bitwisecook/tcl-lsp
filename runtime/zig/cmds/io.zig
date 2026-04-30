// ``puts``, ``append``, ``format``, plus the file-channel command
// surface (``open``, ``close``, ``read``, ``gets``, ``seek``,
// ``tell``, ``eof``, ``fblocked``, ``fcopy``).  ``scan`` lives in
// cmds/scan.zig.
//
// The channel commands here are thin shims over the runtime
// implementations in :file:`io/tcl_chan.zig`.  Each shim parses the
// argument shape (``-nonewline`` / ``-size`` / origin keyword) the
// codegen fast path can't express and routes the call to the
// corresponding ``tcl_cmd_*`` export.

const rt       = @import("../tcl_runtime.zig");
const frames   = @import("../interp/tcl_frames.zig");
const fmt_mod  = @import("../valtypes/tcl_format.zig");
const reg      = @import("../dispatch/tcl_cmd_registry.zig");
const chan     = @import("../io/tcl_chan.zig");
const obj      = @import("../valtypes/tcl_obj.zig");
const stubs    = @import("../stubs/tcl_stubs.zig");

const obj_ensure_string = obj.obj_ensure_string;
const obj_new_string    = obj.obj_new_string;

fn word_eq(w: i32, literal: []const u8) bool {
    if (w == 0) return literal.len == 0;
    const s = obj_ensure_string(w);
    if (s.len != literal.len) return false;
    const p: [*]const u8 = @ptrFromInt(s.ptr);
    for (0..literal.len) |i| if (p[i] != literal[i]) return false;
    return true;
}

/// ``puts ?-nonewline? ?channelId? string`` — handles the four
/// observed call shapes.  The compiler fast path covers the
/// stdout-and-newline / stdout-no-newline forms; the channel forms
/// fall through here via :file:`codegen/wasm/_emitter/cmds/puts_.py`'s
/// returning ``False`` so this handler can route through
/// :func:`tcl_chan.tcl_cmd_puts_chan`.
fn eval_puts(words: []const i32) i32 {
    // words[0] = "puts"
    const argc = words.len - 1;
    if (argc == 0) return 0;
    if (argc == 1) {
        // puts msg → stdout, newline.
        return rt.tcl_cmd_puts(words[1]);
    }
    if (argc == 2) {
        // puts -nonewline msg | puts $chan msg
        if (word_eq(words[1], "-nonewline")) {
            return rt.tcl_cmd_puts_nonewline(words[2]);
        }
        return chan.tcl_cmd_puts_chan(words[1], words[2], 0);
    }
    // argc >= 3 → puts -nonewline $chan msg
    if (word_eq(words[1], "-nonewline")) {
        return chan.tcl_cmd_puts_chan(words[2], words[3], 1);
    }
    stubs.unsupported("puts (unexpected arg shape)");
    return 0;
}

fn eval_open(words: []const i32) i32 {
    // open fileName ?access? ?permissions?
    const path = if (words.len >= 2) words[1] else 0;
    const access = if (words.len >= 3) words[2] else 0;
    // Permissions arg ignored — we use 0o644 unconditionally; the
    // wasi-libc layer doesn't implement umask differently anyway.
    return chan.tcl_cmd_open(path, access);
}

fn eval_close(words: []const i32) i32 {
    if (words.len < 2) {
        stubs.raise("close: missing channelId");
        return 0;
    }
    return chan.tcl_cmd_close(words[1]);
}

fn eval_read(words: []const i32) i32 {
    // read ?-nonewline? channel | read channel ?numChars?
    var nonewline = false;
    var arg_idx: usize = 1;
    if (words.len >= 2 and word_eq(words[1], "-nonewline")) {
        nonewline = true;
        arg_idx = 2;
    }
    if (arg_idx >= words.len) {
        stubs.raise("read: missing channelId");
        return 0;
    }
    const ch = words[arg_idx];
    const num_obj: i32 = if (arg_idx + 1 < words.len) words[arg_idx + 1] else 0;
    const result = chan.tcl_cmd_read(ch, num_obj);
    if (nonewline and result != 0) {
        // Strip a single trailing newline if present.
        const s = obj_ensure_string(result);
        if (s.len > 0) {
            const p: [*]const u8 = @ptrFromInt(s.ptr);
            if (p[s.len - 1] == '\n') {
                return obj_new_string(@intCast(s.ptr), @intCast(s.len - 1));
            }
        }
    }
    return result;
}

fn eval_gets(words: []const i32) i32 {
    if (words.len < 2) {
        stubs.raise("gets: missing channelId");
        return 0;
    }
    const ch = words[1];
    const var_name = if (words.len >= 3) words[2] else 0;
    return chan.tcl_cmd_gets(ch, var_name);
}

fn eval_seek(words: []const i32) i32 {
    if (words.len < 3) {
        stubs.raise("seek: missing offset");
        return 0;
    }
    const ch = words[1];
    const off_obj = words[2];
    const origin_obj: i32 = if (words.len >= 4) words[3] else 0;
    return chan.tcl_cmd_seek(ch, off_obj, origin_obj);
}

fn eval_tell(words: []const i32) i32 {
    if (words.len < 2) {
        stubs.raise("tell: missing channelId");
        return 0;
    }
    return chan.tcl_cmd_tell(words[1]);
}

fn eval_eof(words: []const i32) i32 {
    if (words.len < 2) {
        stubs.raise("eof: missing channelId");
        return 0;
    }
    return chan.tcl_cmd_eof(words[1]);
}

fn eval_fblocked(words: []const i32) i32 {
    if (words.len < 2) {
        stubs.raise("fblocked: missing channelId");
        return 0;
    }
    return chan.tcl_cmd_fblocked(words[1]);
}

fn eval_fcopy(words: []const i32) i32 {
    if (words.len < 3) {
        stubs.raise("fcopy: missing inputChan or outputChan");
        return 0;
    }
    var size_limit: i64 = -1;
    var i: usize = 3;
    while (i + 1 < words.len) : (i += 2) {
        if (word_eq(words[i], "-size")) {
            const s = obj_ensure_string(words[i + 1]);
            if (obj.try_parse_int(s.ptr, s.len)) |v| size_limit = v;
        } else if (word_eq(words[i], "-command")) {
            // ``-command`` requires an event loop we don't model.
            stubs.unsupported("fcopy -command (no event loop)");
            return 0;
        }
    }
    return chan.fcopy_limited(words[1], words[2], size_limit);
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
    .{ .name = "flush", .arity_min = 0, .arity_max = 1, .handler = &eval_flush },
    .{ .name = "append", .arity_min = 1, .arity_max = null, .handler = &eval_append },
    .{ .name = "format", .arity_min = 1, .arity_max = null, .handler = &eval_format },
    .{ .name = "open", .arity_min = 1, .arity_max = 3, .handler = &eval_open },
    .{ .name = "close", .arity_min = 1, .arity_max = 2, .handler = &eval_close },
    .{ .name = "read", .arity_min = 1, .arity_max = 2, .handler = &eval_read },
    .{ .name = "gets", .arity_min = 1, .arity_max = 2, .handler = &eval_gets },
    .{ .name = "seek", .arity_min = 2, .arity_max = 3, .handler = &eval_seek },
    .{ .name = "tell", .arity_min = 1, .arity_max = 1, .handler = &eval_tell },
    .{ .name = "eof", .arity_min = 1, .arity_max = 1, .handler = &eval_eof },
    .{ .name = "fblocked", .arity_min = 1, .arity_max = 1, .handler = &eval_fblocked },
    .{ .name = "fcopy", .arity_min = 2, .arity_max = 6, .handler = &eval_fcopy },
};
