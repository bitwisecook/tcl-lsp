// Channel-config pass-through for ``fconfigure``.
//
// Pure WASM has no user-opened channels (no ``open``); only
// ``stdin``/``stdout``/``stderr`` via WASI, whose buffering /
// blocking / translation modes are fixed by the host.  Scripts
// nonetheless call ``fconfigure $fd -profile tcl8 -encoding utf-8
// -translation binary ...`` as a declarative no-op on init.  The
// previous behaviour of this module was a silent NOP — the project
// style rules forbid that (``Don't silently NOP``), so this version
// accepts a small allowlist of benign option names and raises
// ``unsupported fconfigure option: <opt>`` on anything else.  The
// set-call return value is empty (matches Tcl).  Get-calls (odd
// number of option args) fall through to the unsupported path
// because we have nothing useful to return.
//
// Accepted options (all are accepted with any value — we don't
// validate the value because the real channel doesn't exist):
//   -blocking          -buffering
//   -buffersize        -encoding
//   -eofchar           -profile
//   -translation
//
// Anything else (``-polarity``, ``-strictencoding``, ``-handshake``,
// user extensions) traps through :func:`tcl_stubs.unsupported` so
// the caller sees a clear "we saw this option but don't implement
// it" message and can either remove the option or extend the
// allowlist.

const obj = @import("../value/tcl_obj.zig");
const stubs = @import("../stubs/tcl_stubs.zig");

const obj_ensure_string = obj.obj_ensure_string;
const obj_new_string = obj.obj_new_string;

fn eq(a: [*]const u8, alen: u32, literal: []const u8) bool {
    if (alen != literal.len) return false;
    for (0..literal.len) |i| if (a[i] != literal[i]) return false;
    return true;
}

fn is_accepted_option(p: [*]const u8, len: u32) bool {
    return eq(p, len, "-blocking") or
        eq(p, len, "-buffering") or
        eq(p, len, "-buffersize") or
        eq(p, len, "-encoding") or
        eq(p, len, "-eofchar") or
        eq(p, len, "-profile") or
        eq(p, len, "-translation");
}

/// ``fconfigure $fd ?option value …?`` — channel-option pass-through.
///
/// Two-arg compiled call:
///   fconfigure $fd $args_list
///
/// *args* is a Tcl list of alternating ``-option value`` pairs.  We
/// walk it and validate each option name against the allowlist.
/// Empty args means "query all options" — not implemented, traps.
pub export fn tcl_cmd_fconfigure(fd: i32, args: i32) i32 {
    _ = fd;
    if (args == 0) {
        stubs.unsupported("fconfigure (query all options)");
        return 0;
    }
    const a = obj_ensure_string(args);
    if (a.len == 0) {
        stubs.unsupported("fconfigure (query all options)");
        return 0;
    }
    const ap: [*]const u8 = @ptrFromInt(a.ptr);

    // Walk the args list one word at a time, checking each option
    // name.  Values are not validated.  A list here is plain space-
    // separated words (no braces) because the caller passes
    // _concat_args() output; that's sufficient for the init-time
    // ``fconfigure ... -profile tcl8 -encoding utf-8`` idiom.
    var pos: u32 = 0;
    var parity: u32 = 0; // 0 = expect option, 1 = expect value
    while (pos < a.len) {
        // Skip whitespace.
        while (pos < a.len and (ap[pos] == ' ' or ap[pos] == '\t' or ap[pos] == '\n')) : (pos += 1) {}
        if (pos >= a.len) break;
        const start = pos;
        // Consume a braced or bare word.  Braces matter when a value
        // contains whitespace (``-eofchar {\x1A {}}``).
        if (ap[pos] == '{') {
            pos += 1;
            var depth: u32 = 1;
            while (pos < a.len and depth > 0) : (pos += 1) {
                if (ap[pos] == '{') depth += 1;
                if (ap[pos] == '}') depth -= 1;
            }
        } else {
            while (pos < a.len and ap[pos] != ' ' and ap[pos] != '\t' and ap[pos] != '\n') : (pos += 1) {}
        }
        const end = pos;
        if (parity == 0) {
            // Option name.  Must start with '-' and be in the allowlist.
            const word_len = end - start;
            if (word_len < 2 or ap[start] != '-') {
                stubs.unsupported("fconfigure (expected -option)");
                return 0;
            }
            if (!is_accepted_option(ap + start, word_len)) {
                stubs.unsupported("fconfigure (unsupported option)");
                return 0;
            }
        }
        parity = 1 - parity;
    }
    if (parity == 1) {
        stubs.unsupported("fconfigure (odd number of option/value args)");
        return 0;
    }
    // Set form — return empty.
    return obj_new_string(0, 0);
}
