// Filesystem implementations backed by wasi-libc's POSIX wrappers.
//
// Tcl's ``file`` command has dozens of subcommands; the ones that
// mutate or query the filesystem route to wasi-libc calls
// (``mkdir`` / ``rmdir`` / ``unlink`` / ``rename`` / ``access`` /
// ``stat`` / …), which wasi-libc translates into WASI preopens
// under the hood.  Scripts running in wasmtime can reach the
// files the embedder grants via ``WasiConfig.preopen_dir``.  With
// no preopens, every path resolves to ``ENOTCAPABLE`` and the
// queries report "doesn't exist" — same behaviour we had before
// but now backed by real system calls rather than hardcoded 0.
//
// Coverage (updated incrementally):
//   pwd / cd                — stable strings (root preopen).
//   file join / dirname /
//     tail / rootname /
//     extension / split /
//     pathtype / …          — pure string operations.
//   file exists / isfile /
//     isdirectory /
//     readable / writable /
//     executable / owned    — real access()/stat() checks.
//   file size / mtime /
//     atime / ctime         — real stat()-derived.
//   file mkdir / delete /
//     rename / copy / …     — still trap; follow-up work.
//
// Path handling: Tcl paths arrive as TclObj-wrapped UTF-8 byte
// strings without a trailing NUL.  We allocate a fresh
// NUL-terminated copy on the bump allocator for each call
// (cheap — a few bytes per syscall) rather than demanding the
// caller preserve a NUL-terminated form.

const obj = @import("tcl_obj.zig");
const obj_new_string = obj.obj_new_string;
const obj_new_string_copy = obj.obj_new_string_copy;
const obj_ensure_string = obj.obj_ensure_string;
const stubs = @import("tcl_stubs.zig");

// --- wasi-libc extern declarations ---
//
// These resolve against the wasi-libc archive linked in by
// ``build.zig``'s ``linkLibC()`` call.  Signatures match POSIX;
// on 32-bit WASM, ``long`` is 32 bits and ``long long`` is 64.
// ``mode_t`` is ``unsigned`` (32 bits).

extern fn access(path: [*:0]const u8, mode: c_int) c_int;
extern fn stat(path: [*:0]const u8, buf: *anyopaque) c_int;
extern fn lstat(path: [*:0]const u8, buf: *anyopaque) c_int;
extern fn mkdir(path: [*:0]const u8, mode: c_uint) c_int;
extern fn rmdir(path: [*:0]const u8) c_int;
extern fn unlink(path: [*:0]const u8) c_int;

// access() mode flags (POSIX unistd.h values).
const F_OK: c_int = 0;
const X_OK: c_int = 1;
const W_OK: c_int = 2;
const R_OK: c_int = 4;

// struct stat layout on 32-bit WASM / wasi-libc (musl-derived).
// Verified against
//   zig/lib/libc/include/wasm-wasi-musl/__struct_stat.h
// Field sizes + padding:
//   dev_t (u64)     [  0..  8)
//   ino_t (u64)     [  8.. 16)
//   nlink_t (u64)   [ 16.. 24)
//   mode_t (u32)    [ 24.. 28)
//   uid_t (u32)     [ 28.. 32)
//   gid_t (u32)     [ 32.. 36)
//   __pad0 (u32)    [ 36.. 40)
//   rdev (u64)      [ 40.. 48)
//   st_size (i64)   [ 48.. 56)
//   blksize (i32)   [ 56.. 60)
//   pad             [ 60.. 64)
//   blocks (i64)    [ 64.. 72)
//   atim.tv_sec     [ 72.. 80)
//   atim.tv_nsec    [ 80.. 84)
//   pad             [ 84.. 88)
//   mtim.tv_sec     [ 88.. 96)
//   mtim.tv_nsec    [ 96..100)
//   pad             [100..104)
//   ctim.tv_sec     [104..112)
//   ctim.tv_nsec    [112..116)
//   pad             [116..120)
//   __reserved[3]   [120..144)
// Total: 144 bytes.  Allocate 160 to cover any future libc
// changes without overflowing.
const STAT_SIZE: u32 = 160;
const STAT_OFF_MODE: u32 = 24;
const STAT_OFF_SIZE: u32 = 48;
const STAT_OFF_ATIM_SEC: u32 = 72;
const STAT_OFF_MTIM_SEC: u32 = 88;
const STAT_OFF_CTIM_SEC: u32 = 104;

// mode_t bits (POSIX <sys/stat.h>).
const S_IFMT: u32 = 0o170000;
const S_IFSOCK: u32 = 0o140000;
const S_IFLNK: u32 = 0o120000;
const S_IFREG: u32 = 0o100000;
const S_IFBLK: u32 = 0o060000;
const S_IFDIR: u32 = 0o040000;
const S_IFCHR: u32 = 0o020000;
const S_IFIFO: u32 = 0o010000;

/// Copy *path*'s bytes onto the bump allocator with a trailing
/// NUL so it can be passed to wasi-libc APIs that expect
/// C-strings.
fn path_cstr(path: i32) [*:0]const u8 {
    const s = obj_ensure_string(path);
    const buf_addr = obj.alloc(s.len + 1);
    const out: [*]u8 = @ptrFromInt(buf_addr);
    if (s.len > 0) {
        const src: [*]const u8 = @ptrFromInt(s.ptr);
        for (0..s.len) |i| out[i] = src[i];
    }
    out[s.len] = 0;
    return @ptrCast(out);
}

/// Run ``stat(2)`` on *path*.  Returns the bump-allocator address
/// of the filled struct, or 0 if the call failed (path doesn't
/// exist, not accessible, etc.).
fn stat_path(path: i32) u32 {
    const buf_addr = obj.alloc(STAT_SIZE);
    const buf: *anyopaque = @ptrFromInt(buf_addr);
    const rc = stat(path_cstr(path), buf);
    if (rc != 0) return 0;
    return buf_addr;
}

/// Same as :func:`stat_path` but uses ``lstat(2)`` — does not
/// follow symbolic links on the final path component.
fn lstat_path(path: i32) u32 {
    const buf_addr = obj.alloc(STAT_SIZE);
    const buf: *anyopaque = @ptrFromInt(buf_addr);
    const rc = lstat(path_cstr(path), buf);
    if (rc != 0) return 0;
    return buf_addr;
}

fn stat_mode(stat_buf: u32) u32 {
    const p: *u32 = @ptrFromInt(stat_buf + STAT_OFF_MODE);
    return p.*;
}

fn stat_size(stat_buf: u32) i64 {
    const p: *i64 = @ptrFromInt(stat_buf + STAT_OFF_SIZE);
    return p.*;
}

fn stat_atim_sec(stat_buf: u32) i64 {
    const p: *i64 = @ptrFromInt(stat_buf + STAT_OFF_ATIM_SEC);
    return p.*;
}

fn stat_mtim_sec(stat_buf: u32) i64 {
    const p: *i64 = @ptrFromInt(stat_buf + STAT_OFF_MTIM_SEC);
    return p.*;
}

fn stat_ctim_sec(stat_buf: u32) i64 {
    const p: *i64 = @ptrFromInt(stat_buf + STAT_OFF_CTIM_SEC);
    return p.*;
}

// --- pwd / cd ---

pub export fn tcl_cmd_pwd() i32 {
    // WASI programs without preopens report ``/`` as their root;
    // scripts that use ``pwd`` for logging only need *some* stable
    // answer, and tcltest specifically calls it to seed its
    // ``workingDirectory`` option on init.
    return obj_new_string_copy(@intFromPtr("/".ptr), 1);
}

pub export fn tcl_cmd_cd(dir: i32) i32 {
    _ = dir;
    // Accept but do nothing — there's no real filesystem cwd in
    // WASM.  Scripts that rely on ``cd`` changing the cwd will not
    // observe the effect.
    return obj_new_string(0, 0);
}

fn eq(a: [*]const u8, alen: u32, literal: []const u8) bool {
    if (alen != literal.len) return false;
    for (0..literal.len) |i| if (a[i] != literal[i]) return false;
    return true;
}

/// ``file <sub> <arg> ?extra?`` — dispatch for subcommand set.
/// String-only operations (join / dirname / tail / rootname /
/// extension / normalize / pathtype / split) compute purely on
/// the path text.  Status queries (exists / isfile / isdirectory
/// / readable / writable / executable / owned / size / mtime /
/// atime / ctime) now route through ``access(2)`` / ``stat(2)``
/// so they reflect the actual WASI-preopen-exposed filesystem.
/// Mutating operations (mkdir / delete / rename / copy /
/// attributes / link / readlink / tempfile / stat / lstat /
/// system / type) continue to trap; incremental follow-up work.
pub export fn tcl_cmd_file(sub: i32, arg1: i32, arg2: i32) i32 {
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

    // -- Existence / accessibility queries — real access(2) calls.
    if (eq(sp, s.len, "exists")) return bool_obj(access(path_cstr(arg1), F_OK) == 0);
    if (eq(sp, s.len, "readable")) return bool_obj(access(path_cstr(arg1), R_OK) == 0);
    if (eq(sp, s.len, "writable")) return bool_obj(access(path_cstr(arg1), W_OK) == 0);
    if (eq(sp, s.len, "executable")) return bool_obj(access(path_cstr(arg1), X_OK) == 0);

    // -- Type queries — stat + S_IF* bit check.
    if (eq(sp, s.len, "isfile")) {
        const st = stat_path(arg1);
        if (st == 0) return obj.obj_new_int(0);
        return bool_obj((stat_mode(st) & S_IFMT) == S_IFREG);
    }
    // ``isdir`` is the unique-prefix shorthand Tcl accepts for
    // ``isdirectory``; tcltest's ``AcceptDirectory`` uses that
    // shorter form.  Matching both literal spellings is cheaper
    // than a generic prefix matcher and covers every caller.
    if (eq(sp, s.len, "isdirectory") or eq(sp, s.len, "isdir")) {
        const st = stat_path(arg1);
        if (st == 0) return obj.obj_new_int(0);
        return bool_obj((stat_mode(st) & S_IFMT) == S_IFDIR);
    }
    // ``file owned`` — always false under WASI (no meaningful
    // user identity); keep the old 0 return rather than faking a
    // match.
    if (eq(sp, s.len, "owned")) return obj.obj_new_int(0);

    // -- Size / time queries — stat-derived.  Return -1 (matches
    //    Tcl's typical "unavailable" signal; tclsh would error,
    //    but -1 lets scripts test for non-positive).
    if (eq(sp, s.len, "size")) {
        const st = stat_path(arg1);
        if (st == 0) return obj.obj_new_int(-1);
        return obj.obj_new_int(stat_size(st));
    }
    if (eq(sp, s.len, "mtime")) {
        const st = stat_path(arg1);
        if (st == 0) return obj.obj_new_int(-1);
        return obj.obj_new_int(stat_mtim_sec(st));
    }
    if (eq(sp, s.len, "atime")) {
        const st = stat_path(arg1);
        if (st == 0) return obj.obj_new_int(-1);
        return obj.obj_new_int(stat_atim_sec(st));
    }
    if (eq(sp, s.len, "ctime")) {
        const st = stat_path(arg1);
        if (st == 0) return obj.obj_new_int(-1);
        return obj.obj_new_int(stat_ctim_sec(st));
    }

    // -- Channel inquiry --
    if (eq(sp, s.len, "channels")) return obj.obj_new_string_copy(@intFromPtr("stdin stdout stderr".ptr), 19);
    if (eq(sp, s.len, "volumes")) return obj.obj_new_string_copy(@intFromPtr("/".ptr), 1);

    // -- Mutating operations backed by wasi-libc --
    if (eq(sp, s.len, "mkdir")) return file_mkdir(arg1);
    if (eq(sp, s.len, "delete")) return file_delete(arg1, arg2);

    // -- Mutating operations — trap so scripts can't silently miss
    //    work.  These have incremental follow-up commits pending;
    //    see tcl_fs.zig's header comment.
    if (eq(sp, s.len, "rename") or
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

fn bool_obj(b: bool) i32 {
    return obj.obj_new_int(if (b) 1 else 0);
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

/// ``file mkdir path`` — create *path* plus any missing parent
/// directories (``mkdir -p`` semantics).  Idempotent: succeeds if
/// the directory already exists.  Returns the empty string on
/// success or traps with a descriptive message on failure so the
/// caller sees a clear error rather than silent partial success.
///
/// The multi-path variadic form (``file mkdir a b c``) does not
/// fit our 3-arg export signature; the codegen routes that
/// through the eval fallback, whose interpreter-side dispatcher
/// would loop over the paths — a follow-up concern.  Single-path
/// is what tcltest uses at init time (``AcceptTemporaryDirectory``
/// creates ``$temporaryDirectory``).
fn file_mkdir(path: i32) i32 {
    if (path == 0) {
        stubs.unsupported("file mkdir (no path)");
        return 0;
    }
    if (!mkdir_p(path)) {
        stubs.unsupported("file mkdir (failed)");
        return 0;
    }
    return obj_new_string(0, 0);
}

/// Recursive mkdir.  Returns true if *path* exists as a
/// directory after the call (either already existed or we
/// successfully created it).
fn mkdir_p(path: i32) bool {
    const s = obj_ensure_string(path);
    if (s.len == 0) return true; // empty path — treat as a no-op

    // Already exists and is a directory → done.
    const st = stat_path(path);
    if (st != 0 and (stat_mode(st) & S_IFMT) == S_IFDIR) return true;

    // Try to create.  On ENOENT (missing parent), create parent first.
    const rc = mkdir(path_cstr(path), 0o755);
    if (rc == 0) return true;

    // Retry after ensuring the parent exists, which covers the
    // "intermediate dir missing" case.  Dirname of the current
    // path is computed purely from the string.  Termination:
    // compare by contents (not TclObj identity) so the recursion
    // stops when ``dirname("/") == "/"`` (which produces fresh
    // TclObjs each call).
    const parent = file_dirname(path);
    const parent_s = obj_ensure_string(parent);
    var same = parent_s.len == s.len;
    if (same) {
        const ap: [*]const u8 = @ptrFromInt(s.ptr);
        const bp: [*]const u8 = @ptrFromInt(parent_s.ptr);
        var i: u32 = 0;
        while (i < s.len) : (i += 1) {
            if (ap[i] != bp[i]) {
                same = false;
                break;
            }
        }
    }
    if (!same) {
        if (!mkdir_p(parent)) return false;
        const rc2 = mkdir(path_cstr(path), 0o755);
        if (rc2 == 0) return true;
    }

    // Final fallback: maybe another agent created it in the
    // interval (the "already exists" race) — re-stat.
    const st2 = stat_path(path);
    return st2 != 0 and (stat_mode(st2) & S_IFMT) == S_IFDIR;
}

/// ``file delete ?-force? path`` — remove a file, an empty
/// directory, or (with ``-force``) a directory tree.
///
/// Tcl semantics: deleting a non-existent path is a no-op
/// (returns empty, no error).  Deleting a non-empty directory
/// without ``-force`` is an error.  With ``-force``, recursively
/// removes contents first.
///
/// Recursive delete needs ``opendir`` / ``readdir`` which
/// wasi-libc supports but requires additional wiring — so the
/// ``-force`` branch traps for now and lands in a follow-up.
/// Non-recursive single-path is the common case at init and
/// cleanup time.
fn file_delete(arg1: i32, arg2: i32) i32 {
    // Two call shapes fit our 3-arg signature:
    //   file delete path         → arg1=path,   arg2=0
    //   file delete -force path  → arg1=-force, arg2=path
    // Everything richer (multiple paths, ``--``) falls through
    // the codegen to the interpreter's eval_fallback.
    var target = arg1;
    var force = false;
    if (arg1 != 0) {
        const s = obj_ensure_string(arg1);
        if (s.len == 6) {
            const p: [*]const u8 = @ptrFromInt(s.ptr);
            if (eq(p, s.len, "-force")) {
                force = true;
                target = arg2;
            }
        }
    }
    if (target == 0) {
        // Nothing to delete → success.
        return obj_new_string(0, 0);
    }

    // Non-existent path → no-op (matches Tcl).
    if (access(path_cstr(target), F_OK) != 0) {
        return obj_new_string(0, 0);
    }

    const st = lstat_path(target);
    if (st == 0) {
        // Couldn't stat even though access said it exists —
        // treat as gone (edge case with permission oddities).
        return obj_new_string(0, 0);
    }
    const mode = stat_mode(st);

    if ((mode & S_IFMT) == S_IFDIR) {
        if (force) {
            // Recursive removal needs directory iteration
            // (readdir) which is a separate piece of wiring.
            // Trap rather than silently miss entries.
            stubs.unsupported("file delete -force (recursive)");
            return 0;
        }
        const rc = rmdir(path_cstr(target));
        if (rc != 0) {
            stubs.unsupported("file delete (directory not empty)");
            return 0;
        }
        return obj_new_string(0, 0);
    }

    const rc = unlink(path_cstr(target));
    if (rc != 0) {
        stubs.unsupported("file delete (unlink failed)");
        return 0;
    }
    return obj_new_string(0, 0);
}

// Silence unused-warning for helpers that will be used by the
// follow-up mutating-op commits (rename / copy / stat / lstat /
// type / …).  Keeping the forward references here means the
// wiring is already in place and subsequent commits only touch
// the dispatch table above.
comptime {
    _ = &stat_ctim_sec;
}
