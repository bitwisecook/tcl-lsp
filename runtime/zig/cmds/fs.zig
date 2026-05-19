// ``file``, ``pwd``, ``cd``, ``glob`` — filesystem commands.

const fs_mod = @import("../io/tcl_fs.zig");
const result_mod = @import("../interp/tcl_result.zig");
const obj = @import("../valtypes/tcl_obj.zig");
const stubs = @import("../stubs/tcl_stubs.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");

fn eval_file(words: []const i32) result_mod.InterpResult {
    const sub = if (words.len >= 2) words[1] else 0;
    const a1 = if (words.len >= 3) words[2] else 0;
    const a2 = if (words.len >= 4) words[3] else 0;
    return result_mod.from_globals(fs_mod.tcl_cmd_file(sub, a1, a2));
}

fn eval_pwd(words: []const i32) result_mod.InterpResult {
    _ = words;
    return result_mod.from_globals(fs_mod.tcl_cmd_pwd());
}

fn eval_cd(words: []const i32) result_mod.InterpResult {
    return result_mod.from_globals(fs_mod.tcl_cmd_cd(if (words.len >= 2) words[1] else 0));
}

/// Tcl 9 spelling: ``source ?-encoding name? fileName``.
///
/// We accept either the bare ``source FILE`` form or the
/// ``source -encoding NAME FILE`` form; the encoding *value* is
/// parsed and discarded because the WASI runtime always reads raw
/// bytes which we already treat as UTF-8.  Anything else (extra
/// trailing args, ``-encoding`` without a following name, an
/// unknown leading option) raises so scripts don't silently lose
/// the trailing word — diverging from tclsh on those shapes would
/// hide a real bug at the call site.
fn eval_source(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) {
        stubs.raise("source: missing fileName");
        return result_mod.from_globals(0);
    }
    if (words.len == 2) return result_mod.from_globals(fs_mod.tcl_cmd_source(words[1]));
    if (words.len == 4 and is_dash_encoding(words[1])) {
        return result_mod.from_globals(fs_mod.tcl_cmd_source(words[3]));
    }
    stubs.raise("source: expected ?-encoding name? fileName");
    return result_mod.from_globals(0);
}

fn is_dash_encoding(o: i32) bool {
    if (o == 0) return false;
    const s = obj.obj_ensure_string(o);
    if (s.len != 9) return false;
    const p: [*]const u8 = @ptrFromInt(s.ptr);
    const lit = "-encoding";
    inline for (0..9) |i| {
        if (p[i] != lit[i]) return false;
    }
    return true;
}

/// ``glob ?-nocomplain? ?--? pattern ?pattern ...?``
///
/// Capability-gated through :func:`fs_mod.tcl_cmd_glob`.  Walks each
/// pattern in turn, concatenating the per-pattern result lists into
/// one combined Tcl list.  Without ``-nocomplain`` an empty result
/// raises (per the man page); with the switch we silently return
/// the empty list.
///
/// Switches not yet wired (``-directory`` / ``-tails`` / ``-types``
/// / ``-path`` / ``-join``) raise ``unsupported`` so scripts get
/// a clear diagnostic — adding them is purely a parsing job; the
/// underlying readdir machinery already supports the work.
fn eval_glob(words: []const i32) result_mod.InterpResult {
    var idx: usize = 1;
    var nocomplain = false;
    while (idx < words.len) : (idx += 1) {
        const w = words[idx];
        const s = obj.obj_ensure_string(w);
        if (s.len == 0 or s.len > 32) break;
        const sp: [*]const u8 = @ptrFromInt(s.ptr);
        if (sp[0] != '-') break;
        if (s.len == 2 and sp[1] == '-') {
            idx += 1;
            break;
        }
        if (eq_lit(sp, s.len, "-nocomplain")) {
            nocomplain = true;
            continue;
        }
        if (eq_lit(sp, s.len, "-directory") or
            eq_lit(sp, s.len, "-tails") or
            eq_lit(sp, s.len, "-types") or
            eq_lit(sp, s.len, "-path") or
            eq_lit(sp, s.len, "-join"))
        {
            stubs.unsupported_sub("glob", sp[0..s.len]);
            return result_mod.from_globals(0);
        }
        // Unknown switch — match stock Tcl's behaviour and raise
        // ``bad option "<switch>"`` instead of silently demoting to
        // a literal pattern.  Catches typos like ``glob -typs f *``
        // before they hide behind an empty result.
        const prefix: []const u8 = "bad option \"";
        const suffix: []const u8 = "\": must be -directory, -join, -nocomplain, -path, -tails, -types, or --";
        const total: u32 = @intCast(prefix.len + s.len + suffix.len);
        const buf_addr = obj.alloc(total);
        const buf: [*]u8 = @ptrFromInt(buf_addr);
        var off: usize = 0;
        for (prefix) |c| {
            buf[off] = c;
            off += 1;
        }
        for (0..s.len) |i| {
            buf[off] = sp[i];
            off += 1;
        }
        for (suffix) |c| {
            buf[off] = c;
            off += 1;
        }
        const msg_slice = (@as([*]const u8, @ptrFromInt(buf_addr)))[0..total];
        stubs.raise(msg_slice);
        return result_mod.from_globals(0);
    }
    if (idx >= words.len) {
        if (nocomplain) return result_mod.from_globals(obj.obj_new_string(0, 0));
        stubs.raise("wrong # args: should be \"glob ?switches? pattern ?pattern ...?\"");
        return result_mod.from_globals(0);
    }
    var acc: i32 = obj.obj_new_string(0, 0);
    var any_matched = false;
    while (idx < words.len) : (idx += 1) {
        const result = fs_mod.tcl_cmd_glob(words[idx]);
        if (result == 0) return result_mod.from_globals(0); // capability-denied or fatal — error already raised
        const r = obj.obj_ensure_string(result);
        if (r.len == 0) continue;
        any_matched = true;
        // Concatenate result lists by pasting raw bytes with a single
        // separator — both sides are already canonical list strings.
        if (obj.obj_ensure_string(acc).len == 0) {
            acc = result;
        } else {
            acc = list_concat(acc, result);
        }
    }
    if (!any_matched and !nocomplain) {
        stubs.raise("no files matched glob pattern");
        return result_mod.from_globals(0);
    }
    return result_mod.from_globals(acc);
}

fn eq_lit(p: [*]const u8, len: u32, lit: []const u8) bool {
    if (len != lit.len) return false;
    for (0..lit.len) |i| if (p[i] != lit[i]) return false;
    return true;
}

/// Glue two canonical list strings together, separated by a single
/// space.  Both inputs are assumed to be the result of a glob walk
/// (so already-quoted) — we don't re-quote elements.  This sidesteps
/// the per-element TclObj allocation that ``list_mod.tcl_list``
/// would do for each element across the second input.
fn list_concat(a: i32, b: i32) i32 {
    const sa = obj.obj_ensure_string(a);
    const sb = obj.obj_ensure_string(b);
    if (sa.len == 0) return b;
    if (sb.len == 0) return a;
    const total: u32 = sa.len + 1 + sb.len;
    const buf = obj.alloc(total);
    if (buf == 0) return obj.obj_new_string(0, 0);
    const out: [*]u8 = @ptrFromInt(buf);
    const ap: [*]const u8 = @ptrFromInt(sa.ptr);
    const bp: [*]const u8 = @ptrFromInt(sb.ptr);
    for (0..sa.len) |i| out[i] = ap[i];
    out[sa.len] = ' ';
    for (0..sb.len) |i| out[sa.len + 1 + i] = bp[i];
    return obj.obj_new_string_take(buf, total, total);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "file", .arity_min = 1, .arity_max = null, .handler = &eval_file },
    .{ .name = "pwd", .arity_min = 0, .arity_max = 0, .handler = &eval_pwd },
    .{ .name = "cd", .arity_min = 0, .arity_max = 1, .handler = &eval_cd },
    .{ .name = "source", .arity_min = 1, .arity_max = 3, .handler = &eval_source },
    .{ .name = "glob", .arity_min = 1, .arity_max = null, .handler = &eval_glob },
};

// ``file <sub>`` sub-commands — mirrors
// ``core/commands/registry/tcl/file.py``.  Cross-checked against
// ``generic/tclFCmd.c`` + ``generic/tclFileName.c`` in C Tcl 9.0 —
// each sub-command's handler enforces its arity via
// ``Tcl_WrongNumArgs``.  ``pwd`` and ``cd`` have no sub-commands.
pub const file_subcommands: []const reg.SubEntry = &.{
    .{ .name = "atime", .arity_min = 1, .arity_max = 2, .handler = &eval_file },
    .{ .name = "attributes", .arity_min = 1, .arity_max = null, .handler = &eval_file },
    .{ .name = "channels", .arity_min = 0, .arity_max = 1, .handler = &eval_file },
    .{ .name = "copy", .arity_min = 2, .arity_max = null, .handler = &eval_file },
    .{ .name = "delete", .arity_min = 1, .arity_max = null, .handler = &eval_file },
    .{ .name = "dirname", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "executable", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "exists", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "extension", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "home", .arity_min = 0, .arity_max = 1, .handler = &eval_file },
    .{ .name = "isdirectory", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "isfile", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "join", .arity_min = 1, .arity_max = null, .handler = &eval_file },
    .{ .name = "link", .arity_min = 1, .arity_max = 2, .handler = &eval_file },
    .{ .name = "lstat", .arity_min = 1, .arity_max = 2, .handler = &eval_file },
    .{ .name = "mkdir", .arity_min = 1, .arity_max = null, .handler = &eval_file },
    .{ .name = "mtime", .arity_min = 1, .arity_max = 2, .handler = &eval_file },
    .{ .name = "nativename", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "normalize", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "owned", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "pathtype", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "readable", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "readlink", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "rename", .arity_min = 2, .arity_max = null, .handler = &eval_file },
    .{ .name = "rootname", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "separator", .arity_min = 0, .arity_max = 1, .handler = &eval_file },
    .{ .name = "size", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "split", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "stat", .arity_min = 1, .arity_max = 2, .handler = &eval_file },
    .{ .name = "system", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "tail", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "tempdir", .arity_min = 0, .arity_max = 1, .handler = &eval_file },
    .{ .name = "tempfile", .arity_min = 0, .arity_max = 2, .handler = &eval_file },
    .{ .name = "tildeexpand", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "type", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
    .{ .name = "volumes", .arity_min = 0, .arity_max = 0, .handler = &eval_file },
    .{ .name = "writable", .arity_min = 1, .arity_max = 1, .handler = &eval_file },
};
