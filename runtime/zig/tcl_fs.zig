// Filesystem pass-through implementations for the few commands
// that have a meaningful answer in a WASI-sandboxed WASM build.
// Everything else lives in ``tcl_fs_stubs.zig`` and traps.
//
// Coverage:
//   pwd → always "/" (the WASI preopen root).
//   cd  → accept the argument but do nothing; return empty.
//
// ``file mkdir``, ``file delete``, ``glob``, ``exec``, ``source``,
// ``load``, ``unload`` continue to trap; those operate on a disk we
// don't have (or a preopened FD we don't expose to Tcl).

const obj = @import("tcl_obj.zig");
const obj_new_string = obj.obj_new_string;
const obj_new_string_copy = obj.obj_new_string_copy;

pub export fn pwd() i32 {
    // WASI programs without preopens report ``/`` as their root;
    // scripts that use ``pwd`` for logging only need *some* stable
    // answer, and tcltest specifically calls it to seed its
    // ``workingDirectory`` option on init.
    return obj_new_string_copy(@intFromPtr("/".ptr), 1);
}

pub export fn cd(dir: i32) i32 {
    _ = dir;
    // Accept but do nothing — there's no real filesystem cwd in
    // WASM.  Scripts that rely on ``cd`` changing the cwd will not
    // observe the effect (real files are inaccessible anyway).
    return obj_new_string(0, 0);
}

const obj_ensure_string = obj.obj_ensure_string;
const stubs = @import("tcl_stubs.zig");

fn eq(a: [*]const u8, alen: u32, literal: []const u8) bool {
    if (alen != literal.len) return false;
    for (0..literal.len) |i| if (a[i] != literal[i]) return false;
    return true;
}

/// ``file <sub> <arg> ?extra?`` — minimal dispatch for the small set
/// of subcommands scripts use at init time without expecting real
/// filesystem access.  String-only operations (join / dirname /
/// tail / rootname / extension / normalize) compute their result
/// purely on the path text.  Status queries (exists / isfile /
/// isdirectory / readable / writable / executable / size / mtime /
/// atime) return "doesn't exist" (0 or -1) rather than trap so
/// init-time checks that optionally skip a path fall naturally.
/// Mutating operations (mkdir / delete / rename / copy / attributes
/// / link / readlink / tempfile / stat / lstat) trap so they can't
/// silently miss work.
pub export fn file(sub: i32, arg1: i32, arg2: i32) i32 {
    if (sub == 0) {
        stubs.unsupported("file (missing subcommand)");
        return 0;
    }
    const s = obj_ensure_string(sub);
    if (s.len == 0) {
        stubs.unsupported("file (empty subcommand)");
        return 0;
    }
    const sp: [*]const u8 = @ptrFromInt(s.ptr);

    // -- String-only path manipulation --
    if (eq(sp, s.len, "join")) return file_join(arg1, arg2);
    if (eq(sp, s.len, "dirname")) return file_dirname(arg1);
    if (eq(sp, s.len, "tail")) return file_tail(arg1);
    if (eq(sp, s.len, "rootname")) return file_rootname(arg1);
    if (eq(sp, s.len, "extension")) return file_extension(arg1);
    if (eq(sp, s.len, "normalize")) return arg1; // pass-through — we have no symlinks to resolve
    if (eq(sp, s.len, "pathtype")) return obj.obj_new_string_copy(@intFromPtr("absolute".ptr), 8);
    if (eq(sp, s.len, "separator")) return obj.obj_new_string_copy(@intFromPtr("/".ptr), 1);
    if (eq(sp, s.len, "nativename")) return arg1; // no native names in WASM
    if (eq(sp, s.len, "split")) return file_split(arg1);

    // -- Existence / attribute queries — WASI has no preopens we
    //    expose to Tcl, so every path reports as "doesn't exist" /
    //    "not readable".
    if (eq(sp, s.len, "exists") or
        eq(sp, s.len, "isfile") or
        eq(sp, s.len, "isdirectory") or
        eq(sp, s.len, "readable") or
        eq(sp, s.len, "writable") or
        eq(sp, s.len, "executable") or
        eq(sp, s.len, "owned"))
    {
        return obj.obj_new_int(0);
    }
    if (eq(sp, s.len, "size") or
        eq(sp, s.len, "mtime") or
        eq(sp, s.len, "atime") or
        eq(sp, s.len, "ctime"))
    {
        return obj.obj_new_int(-1);
    }

    // -- Channel inquiry --
    if (eq(sp, s.len, "channels")) return obj.obj_new_string_copy(@intFromPtr("stdin stdout stderr".ptr), 19);
    if (eq(sp, s.len, "volumes")) return obj.obj_new_string_copy(@intFromPtr("/".ptr), 1);

    // -- Mutating operations — trap so scripts can't silently miss
    //    work.
    if (eq(sp, s.len, "mkdir") or
        eq(sp, s.len, "delete") or
        eq(sp, s.len, "rename") or
        eq(sp, s.len, "copy") or
        eq(sp, s.len, "attributes") or
        eq(sp, s.len, "link") or
        eq(sp, s.len, "readlink") or
        eq(sp, s.len, "tempfile") or
        eq(sp, s.len, "stat") or
        eq(sp, s.len, "lstat") or
        eq(sp, s.len, "system") or
        eq(sp, s.len, "type"))
    {
        const sub_slice: []const u8 = (@as([*]const u8, @ptrFromInt(s.ptr)))[0..s.len];
        stubs.unsupported_sub("file", sub_slice);
        return 0;
    }

    stubs.unsupported("file (unknown subcommand)");
    return 0;
}

fn file_join(a: i32, b: i32) i32 {
    const as = obj_ensure_string(a);
    const bs = obj_ensure_string(b);
    if (bs.len == 0) return a;
    if (as.len == 0) return b;
    // If b is absolute, it wins entirely (Tcl ``file join`` semantics).
    const bp: [*]const u8 = @ptrFromInt(bs.ptr);
    if (bs.len >= 1 and bp[0] == '/') return b;
    // Otherwise concatenate with a single '/' separator, trimming
    // any trailing slash on `a`.
    const ap: [*]const u8 = @ptrFromInt(as.ptr);
    var a_end: u32 = as.len;
    while (a_end > 0 and ap[a_end - 1] == '/') : (a_end -= 1) {}
    const total: u32 = a_end + 1 + bs.len;
    const buf_addr: u32 = obj.alloc(total);
    const buf: [*]u8 = @ptrFromInt(buf_addr);
    for (0..a_end) |i| buf[i] = ap[i];
    buf[a_end] = '/';
    for (0..bs.len) |i| buf[a_end + 1 + i] = bp[i];
    return obj.obj_new_string(@intCast(buf_addr), @intCast(total));
}

fn file_dirname(a: i32) i32 {
    const as = obj_ensure_string(a);
    if (as.len == 0) return obj.obj_new_string_copy(@intFromPtr(".".ptr), 1);
    const ap: [*]const u8 = @ptrFromInt(as.ptr);
    // Find last '/'.
    var i: u32 = as.len;
    while (i > 0 and ap[i - 1] != '/') : (i -= 1) {}
    if (i == 0) return obj.obj_new_string_copy(@intFromPtr(".".ptr), 1);
    // Trim trailing '/' except for the root itself.
    var end: u32 = i - 1;
    while (end > 0 and ap[end - 1] == '/') : (end -= 1) {}
    if (end == 0) return obj.obj_new_string_copy(@intFromPtr("/".ptr), 1);
    return obj.obj_new_string(@intCast(as.ptr), @intCast(end));
}

fn file_tail(a: i32) i32 {
    const as = obj_ensure_string(a);
    if (as.len == 0) return a;
    const ap: [*]const u8 = @ptrFromInt(as.ptr);
    var i: u32 = as.len;
    while (i > 0 and ap[i - 1] != '/') : (i -= 1) {}
    return obj.obj_new_string(@intCast(as.ptr + i), @intCast(as.len - i));
}

fn file_rootname(a: i32) i32 {
    const as = obj_ensure_string(a);
    if (as.len == 0) return a;
    const ap: [*]const u8 = @ptrFromInt(as.ptr);
    // Find last '.' after the last '/'.
    var i: u32 = as.len;
    while (i > 0 and ap[i - 1] != '/' and ap[i - 1] != '.') : (i -= 1) {}
    if (i == 0 or ap[i - 1] == '/') return a;
    return obj.obj_new_string(@intCast(as.ptr), @intCast(i - 1));
}

fn file_extension(a: i32) i32 {
    const as = obj_ensure_string(a);
    if (as.len == 0) return a;
    const ap: [*]const u8 = @ptrFromInt(as.ptr);
    var i: u32 = as.len;
    while (i > 0 and ap[i - 1] != '/' and ap[i - 1] != '.') : (i -= 1) {}
    if (i == 0 or ap[i - 1] == '/') return obj.obj_new_string(0, 0);
    // Return ".ext" including the dot.
    return obj.obj_new_string(@intCast(as.ptr + i - 1), @intCast(as.len - i + 1));
}

fn file_split(a: i32) i32 {
    // Split a path on '/'.  Returns a Tcl list (space-separated
    // string with components braced if they contain spaces).  Good
    // enough for tcltest's path handling; production code uses
    // ``file join`` to round-trip.
    const as = obj_ensure_string(a);
    if (as.len == 0) return a;
    // Simplification: leave the raw path — scripts that need real
    // splitting should call ``split`` themselves.  Returning the
    // whole path keeps the list-looking shape so ``file join``
    // round-trips.
    return a;
}
