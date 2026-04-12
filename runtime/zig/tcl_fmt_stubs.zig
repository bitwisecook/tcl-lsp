// Formatting + pattern-matching stubs.  These cover the Tcl 8.4–9.0
// commands that deal with string-to-bytes conversion, regular
// expressions, and character-set recoding.  All raise
// ``unsupported command: <name>`` via :func:`tcl_stubs.unsupported`.
//
// Coverage:
//   - format, scan, binary (format / scan / encode / decode)
//   - regexp, regsub
//   - encoding (multiplexed — the subcommand variance is in the
//     sidecar map's args slot)

const stubs = @import("tcl_stubs.zig");

pub export fn format(fmt: i32, value: i32) i32 {
    _ = fmt;
    _ = value;
    stubs.unsupported("format");
    return 0;
}

pub export fn scan(str: i32, fmt: i32) i32 {
    _ = str;
    _ = fmt;
    stubs.unsupported("scan");
    return 0;
}

pub export fn binary(sub: i32, arg: i32) i32 {
    _ = sub;
    _ = arg;
    stubs.unsupported("binary");
    return 0;
}

pub export fn regexp(pattern: i32, str: i32) i32 {
    _ = pattern;
    _ = str;
    stubs.unsupported("regexp");
    return 0;
}

pub export fn regsub(pattern: i32, str: i32) i32 {
    _ = pattern;
    _ = str;
    stubs.unsupported("regsub");
    return 0;
}

// ``encoding`` moved to tcl_encoding.zig — has a real (UTF-8 only)
// implementation for convertfrom / convertto / system / names /
// dirs.  Unknown subcommands / unsupported codecs still trap.
