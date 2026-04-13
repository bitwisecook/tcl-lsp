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

// ``format`` moved to tcl_format.zig — minimal %d / %s / %c /
// %x / %o / %f / %e / %g with width + precision support.

pub export fn tcl_cmd_scan(str: i32, fmt: i32) i32 {
    _ = str;
    _ = fmt;
    stubs.unsupported("scan");
    return 0;
}

pub export fn tcl_cmd_binary(sub: i32, arg: i32) i32 {
    _ = sub;
    _ = arg;
    stubs.unsupported("binary");
    return 0;
}

// ``regexp`` moved to tcl_regex.zig — real implementation backed
// by Tcl's Henry-Spencer engine (linked from
// ``runtime/zig/vendor/tcl-regex/``).  Only ``regsub`` remains a
// stub until we add the substitution path.

pub export fn tcl_cmd_regsub(pattern: i32, str: i32) i32 {
    _ = pattern;
    _ = str;
    stubs.unsupported("regsub");
    return 0;
}

// ``encoding`` moved to tcl_encoding.zig — has a real (UTF-8 only)
// implementation for convertfrom / convertto / system / names /
// dirs.  Unknown subcommands / unsupported codecs still trap.
