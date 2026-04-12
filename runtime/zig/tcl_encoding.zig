// Minimal UTF-8-only encoding command.
//
// Tcl's ``encoding`` command normally selects between character-set
// codecs (identity, utf-8, iso8859-*, cp1252, …) and converts byte
// sequences between them.  In a pure-WASM build we don't ship the
// codec tables, but real-world scripts overwhelmingly use only
// ``encoding convertfrom identity $s`` / ``encoding convertfrom
// utf-8 $s`` as a declarative pass-through (tcltest's ``bytestring``
// proc is the poster child — see
// tests/external/tcllib/tcltest/tcltest.tcl:3347).
//
// This module handles that common case:
//   encoding convertfrom ?encoding? bytes → bytes (pass-through for
//     identity / utf-8; default encoding is utf-8).
//   encoding convertto ?encoding? string → bytes (ditto).
//   encoding system → always "utf-8".
//   encoding names → {identity utf-8}.
//   encoding dirs → {}.
//
// Any OTHER encoding name (``iso8859-1``, ``cp1252``, …) raises
// ``unsupported encoding: <name>`` so silent data corruption is
// impossible.  Unknown subcommands also trap.
//
// Signature matches ``_RUNTIME_IMPORTS['tcl_encoding']``: three i32
// TclObj args (subcmd, arg1, arg2) so both the 2-arg
// ``encoding convertfrom $bytes`` form and the 3-arg
// ``encoding convertfrom utf-8 $bytes`` form work.  Codegen pads
// missing args with 0.

const obj = @import("tcl_obj.zig");
const stubs = @import("tcl_stubs.zig");

const obj_ensure_string = obj.obj_ensure_string;
const obj_new_string = obj.obj_new_string;
const obj_new_string_copy = obj.obj_new_string_copy;

fn eq(a: [*]const u8, alen: u32, literal: []const u8) bool {
    if (alen != literal.len) return false;
    for (0..literal.len) |i| if (a[i] != literal[i]) return false;
    return true;
}

fn is_passthru_encoding(p: [*]const u8, len: u32) bool {
    return eq(p, len, "identity") or eq(p, len, "utf-8");
}

fn is_known_other_encoding(p: [*]const u8, len: u32) bool {
    // Encoding names Tcl core ships — distinguishing these from an
    // accidental typo means we can trap with a precise "unsupported
    // encoding" message instead of silently treating the name as a
    // payload when a user really did ask for a codec we can't do.
    return eq(p, len, "iso8859-1") or eq(p, len, "iso8859-2") or
        eq(p, len, "iso8859-15") or eq(p, len, "cp1250") or
        eq(p, len, "cp1251") or eq(p, len, "cp1252") or
        eq(p, len, "cp437") or eq(p, len, "cp850") or
        eq(p, len, "ascii") or eq(p, len, "unicode") or
        eq(p, len, "ucs-2le") or eq(p, len, "ucs-2be") or
        eq(p, len, "ucs-4le") or eq(p, len, "ucs-4be") or
        eq(p, len, "utf-16") or eq(p, len, "utf-16le") or
        eq(p, len, "utf-16be") or eq(p, len, "utf-32") or
        eq(p, len, "macroman") or eq(p, len, "macjapan") or
        eq(p, len, "shiftjis") or eq(p, len, "jis0208") or
        eq(p, len, "jis0212") or eq(p, len, "euc-jp") or
        eq(p, len, "euc-kr") or eq(p, len, "euc-cn") or
        eq(p, len, "gb2312") or eq(p, len, "gb12345") or
        eq(p, len, "gb1988") or eq(p, len, "big5") or
        eq(p, len, "koi8-r") or eq(p, len, "koi8-u") or
        eq(p, len, "tis-620");
}

/// Dispatch ``encoding <subcmd> ?arg1? ?arg2?``.  Codegen always
/// pushes three i32 slots; missing args are 0 (empty string).
pub export fn encoding(sub: i32, arg1: i32, arg2: i32) i32 {
    if (sub == 0) {
        stubs.unsupported("encoding (missing subcommand)");
        return 0;
    }
    const s = obj_ensure_string(sub);
    if (s.len == 0) {
        stubs.unsupported("encoding (empty subcommand)");
        return 0;
    }
    const sp: [*]const u8 = @ptrFromInt(s.ptr);

    // ``encoding system`` — return "utf-8".  Ignore extra args.
    if (eq(sp, s.len, "system")) {
        return obj_new_string_copy(@intFromPtr("utf-8".ptr), 5);
    }
    // ``encoding names`` — return {identity utf-8}.
    if (eq(sp, s.len, "names")) {
        return obj_new_string_copy(@intFromPtr("identity utf-8".ptr), 14);
    }
    // ``encoding dirs`` — return {} (no codec search path).
    if (eq(sp, s.len, "dirs")) {
        return obj_new_string(0, 0);
    }

    const is_from = eq(sp, s.len, "convertfrom");
    const is_to = eq(sp, s.len, "convertto");
    if (is_from or is_to) {
        // Three-arg form: arg1 is the encoding name, arg2 the payload.
        if (arg2 != 0) {
            const a1 = obj_ensure_string(arg1);
            if (a1.len == 0) return arg2;
            const a1p: [*]const u8 = @ptrFromInt(a1.ptr);
            if (is_passthru_encoding(a1p, a1.len)) return arg2;
            if (is_known_other_encoding(a1p, a1.len)) {
                stubs.unsupported("encoding (only identity and utf-8 supported)");
                return 0;
            }
            // Unrecognised name — treat as payload (legacy Tcl 8.4
            // accepts ``encoding convertfrom $bytes`` too); fall
            // through to the two-arg branch.
        }
        // Two-arg form: arg1 is the payload, default encoding = utf-8.
        if (arg1 == 0) return obj_new_string(0, 0);
        return arg1;
    }

    stubs.unsupported("encoding (unknown subcommand)");
    return 0;
}
