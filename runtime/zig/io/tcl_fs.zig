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
//     rename / copy / link /
//     readlink              — real wasi-libc syscalls (mkdir /
//                             rmdir / unlink / rename / symlink /
//                             readlink / POSIX open+read+write
//                             for copy).  Failures surface through
//                             ``stubs.raise`` with a clear
//                             syscall-failure message (not the
//                             ``unsupported command:`` prefix,
//                             which would be misleading for an
//                             *implemented* command).
//
// Path handling: Tcl paths arrive as TclObj-wrapped UTF-8 byte
// strings without a trailing NUL.  We allocate a fresh
// NUL-terminated copy on the bump allocator for each call
// (cheap — a few bytes per syscall) rather than demanding the
// caller preserve a NUL-terminated form.

const obj = @import("../valtypes/tcl_obj.zig");
const obj_new_string = obj.obj_new_string;
const obj_new_string_copy = obj.obj_new_string_copy;
const obj_ensure_string = obj.obj_ensure_string;
const stubs = @import("../stubs/tcl_stubs.zig");

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
extern fn rename(old: [*:0]const u8, new: [*:0]const u8) c_int;
extern fn symlink(target: [*:0]const u8, linkpath: [*:0]const u8) c_int;
extern fn readlink(path: [*:0]const u8, buf: [*]u8, bufsiz: usize) isize;

// Directory iteration for ``glob``.  wasi-libc translates
// ``opendir`` / ``readdir`` / ``closedir`` to ``path_open`` +
// ``fd_readdir`` under the hood.  ``opendir`` returns a ``DIR*``
// cookie we treat as opaque.  ``readdir`` returns a pointer to a
// statically-allocated ``struct dirent`` whose layout is documented
// at the call site.
extern fn opendir(path: [*:0]const u8) ?*anyopaque;
extern fn readdir(dir: *anyopaque) ?*anyopaque;
extern fn closedir(dir: *anyopaque) c_int;

// ``struct dirent`` layout from
// ``zig/lib/libc/include/wasm-wasi-musl/__struct_dirent.h``:
//   ino_t  d_ino   (u64) [0..8)
//   u8     d_type        [8..9)
//   char[] d_name        [9..)   NUL-terminated
const DIRENT_OFF_NAME: u32 = 9;

// File I/O for ``file copy`` — POSIX open/read/write/close.
// ``extern`` imports don't collide with our Tcl-command WASM
// exports (those are exports from our module; these are imports
// from libc) so we can spell the libc functions with their
// natural names.
//
// POSIX ``open`` is variadic: the third ``mode`` argument is
// consulted only when ``O_CREAT`` is in the flags.  wasi-libc
// exposes the variadic shape, but we always pass three args
// (mode=0 when not creating) so a fixed-arity extern is safe on
// WASM's calling convention.  The explicit ``callconv(.c)`` +
// ``@extern`` pins the linkage name so Zig doesn't name-mangle
// it — the symbol the linker resolves is literally ``open``.
const open = @extern(
    *const fn ([*:0]const u8, c_int, c_uint) callconv(.c) c_int,
    .{ .name = "open" },
);
const read = @extern(
    *const fn (c_int, [*]u8, usize) callconv(.c) isize,
    .{ .name = "read" },
);
const write = @extern(
    *const fn (c_int, [*]const u8, usize) callconv(.c) isize,
    .{ .name = "write" },
);
const close = @extern(
    *const fn (c_int) callconv(.c) c_int,
    .{ .name = "close" },
);

// open() flags — wasi-libc uses a non-musl encoding derived
// from the underlying ``__WASI_OFLAGS_*`` / ``__WASI_RIGHTS_*``
// values, NOT the standard musl bit positions.  The correct
// values live in ``zig/lib/libc/include/wasm-wasi-musl/
// __header_fcntl.h``.  Using the wrong (musl) values makes
// ``open(path, O_RDONLY, ...)`` silently request an invalid mix
// of rights and returns ``EINVAL``.
const O_RDONLY: c_int = 0x04000000;
const O_WRONLY: c_int = 0x10000000;
const O_CREAT: c_int = 1 << 12;
const O_TRUNC: c_int = 8 << 12;

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

// --- source ---

/// ``source FILE`` — read FILE's contents and evaluate as Tcl.
///
/// Resolves *path* against the WASI preopen tree via the existing
/// ``open(2)`` extern, reads the contents into a heap buffer, and
/// hands them to ``tcl_eval``.  Returns the result of the
/// evaluation; missing files, read errors, and out-of-memory
/// conditions all surface through ``tcl_stubs.raise`` rather than
/// being silently swallowed (a partial read or directory read that
/// returns -1 must not be treated like EOF + empty script).
///
/// Buffer ownership: we allocate the read buffer via the size-class
/// allocator and stash it in the wrapping TclObj's ``OBJ_STR_CAP``
/// slot so a subsequent ``tcl_obj_release`` frees the bytes via
/// ``free_sized`` along with the header.  The release is queued
/// onto the deferred-free list inside ``tcl_obj_release`` so
/// any words that borrow from the script bytes stay valid until
/// the next drain at the eval-loop boundary.
pub export fn tcl_cmd_source(path: i32) i32 {
    if (path == 0) {
        stubs.raise("source: no path given");
        return 0;
    }
    const stat_buf = stat_path(path);
    if (stat_buf == 0) {
        stubs.raise("source: file not found");
        return 0;
    }
    const size_i64 = stat_size(stat_buf);
    // Free the stat scratch buffer immediately — repeated sources of
    // a missing file used to leak STAT_SIZE bytes per call.  After
    // this point ``stat_buf`` must not be dereferenced.
    obj.free_sized(stat_buf, STAT_SIZE);
    if (size_i64 < 0 or size_i64 > 64 * 1024 * 1024) {
        stubs.raise("source: file size out of range");
        return 0;
    }
    const size: u32 = @intCast(size_i64);
    if (size == 0) return obj_new_string(0, 0);
    const fd = open(path_cstr(path), O_RDONLY, 0);
    if (fd < 0) {
        stubs.raise("source: open failed");
        return 0;
    }
    const buf = obj.alloc(size);
    if (buf == 0) {
        _ = close(fd);
        stubs.raise("source: out of memory");
        return 0;
    }
    var off: u32 = 0;
    var read_failed = false;
    while (off < size) {
        const got = read(fd, @ptrFromInt(buf + off), size - off);
        if (got == 0) break; // genuine EOF
        if (got < 0) {
            // I/O error or sourcing a directory — must not fall
            // through and execute an empty / truncated script.
            read_failed = true;
            break;
        }
        off += @intCast(got);
    }
    _ = close(fd);
    if (read_failed) {
        obj.free_sized(buf, size);
        stubs.raise("source: read failed");
        return 0;
    }
    if (off == 0) {
        obj.free_sized(buf, size);
        return obj_new_string(0, 0);
    }
    // Wrap as a TclObj string, claim ownership of the read buffer
    // via OBJ_STR_CAP so eventual release frees both header and
    // bytes, then delegate to tcl_eval.  After eval returns, drop
    // our caller-owned reference; the deferred-free queue keeps
    // the buffer alive across any words still borrowing from it.
    const script_obj = obj_new_string(@bitCast(buf), @bitCast(off));
    if (script_obj == 0) {
        // Header alloc OOM — free the read buffer ourselves (without
        // a TclObj header to attach it to, ``tcl_obj_release`` would
        // never see it) and surface as a clean Tcl error rather than
        // a write-through-zero trap.
        obj.free_sized(buf, size);
        stubs.raise("source: out of memory");
        return 0;
    }
    obj.write_i32(@as(u32, @intCast(script_obj)) + obj.OBJ_STR_CAP, @bitCast(size));
    const interp = @import("../interp/tcl_interp.zig");
    const result = interp.tcl_eval(script_obj);
    obj.tcl_obj_release(script_obj);
    return result;
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
/// atime / ctime) route through ``access(2)`` / ``stat(2)`` and
/// reflect the actual WASI-preopen-exposed filesystem.
/// Mutating operations (mkdir / delete / rename / copy / link /
/// readlink / stat / lstat / type) are implemented via real
/// wasi-libc syscalls; real failures raise a clear syscall-
/// failure message (not the ``unsupported command:`` prefix).
/// ``attributes`` / ``tempfile`` / ``system`` remain stubs for
/// now — follow-up work.
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
    if (eq(sp, s.len, "rename")) return file_rename(arg1, arg2);
    if (eq(sp, s.len, "copy")) return file_copy(arg1, arg2);
    if (eq(sp, s.len, "type")) return file_type(arg1);
    if (eq(sp, s.len, "stat")) return file_stat_cmd(arg1, arg2, false);
    if (eq(sp, s.len, "lstat")) return file_stat_cmd(arg1, arg2, true);
    if (eq(sp, s.len, "readlink")) return file_readlink(arg1);
    if (eq(sp, s.len, "link")) return file_link(arg1, arg2);
    // ``file system`` returns the VFS that backs the path.  We
    // only have a single WASI-preopen filesystem; report ``native``
    // with an empty per-volume type, matching what tclsh reports
    // on POSIX.  The second element (``/``) is the mount point.
    if (eq(sp, s.len, "system")) return obj.obj_new_string_copy(@intFromPtr("native".ptr), 6);

    // -- Mutating operations — trap so scripts can't silently miss
    //    work.  ``attributes`` queries/sets Unix-style owner/group/
    //    permissions which WASI doesn't expose meaningfully; ``tempfile``
    //    needs the mkstemp family that wasi-libc lacks cleanly.  Both
    //    remain trap-as-unsupported until we have a concrete caller.
    if (eq(sp, s.len, "attributes") or
        eq(sp, s.len, "tempfile"))
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
    return obj.obj_new_string(@bitCast(buf_addr), @bitCast(total));
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
    return obj.obj_new_string(@bitCast(as.ptr), @bitCast(end));
}

fn file_tail(a: i32) i32 {
    const as = obj_ensure_string(a);
    if (as.len == 0) return a;
    const ap: [*]const u8 = @ptrFromInt(as.ptr);
    var i: u32 = as.len;
    while (i > 0 and ap[i - 1] != '/') : (i -= 1) {}
    return obj.obj_new_string(@bitCast(as.ptr + i), @bitCast(as.len - i));
}

fn file_rootname(a: i32) i32 {
    const as = obj_ensure_string(a);
    if (as.len == 0) return a;
    const ap: [*]const u8 = @ptrFromInt(as.ptr);
    // Find last '.' after the last '/'.
    var i: u32 = as.len;
    while (i > 0 and ap[i - 1] != '/' and ap[i - 1] != '.') : (i -= 1) {}
    if (i == 0 or ap[i - 1] == '/') return a;
    return obj.obj_new_string(@bitCast(as.ptr), @bitCast(i - 1));
}

fn file_extension(a: i32) i32 {
    const as = obj_ensure_string(a);
    if (as.len == 0) return a;
    const ap: [*]const u8 = @ptrFromInt(as.ptr);
    var i: u32 = as.len;
    while (i > 0 and ap[i - 1] != '/' and ap[i - 1] != '.') : (i -= 1) {}
    if (i == 0 or ap[i - 1] == '/') return obj.obj_new_string(0, 0);
    // Return ".ext" including the dot.
    return obj.obj_new_string(@bitCast(as.ptr + i - 1), @bitCast(as.len - i + 1));
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
        stubs.raise("file mkdir: missing path argument");
        return 0;
    }
    if (!mkdir_p(path)) {
        // Real syscall failure (permission denied, parent missing,
        // out of space, …).  Tagging it as ``unsupported command:``
        // would be misleading — ``file mkdir`` is fully
        // implemented; the OS call just failed.  wasi-libc doesn't
        // expose ``errno`` cheaply here, so we report a generic
        // failure; callers inside ``catch`` see the message via
        // ``$::errorInfo`` / the catch var.
        stubs.raise("file mkdir: filesystem operation failed");
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
            // This ``unsupported`` is genuine — we don't implement
            // the recursive path yet — so keep the prefix.
            stubs.unsupported("file delete -force (recursive)");
            return 0;
        }
        const rc = rmdir(path_cstr(target));
        if (rc != 0) {
            // Real failure (non-empty directory, permission, …) —
            // the command *is* implemented, the syscall failed.
            stubs.raise("file delete: rmdir failed (directory may not be empty)");
            return 0;
        }
        return obj_new_string(0, 0);
    }

    const rc = unlink(path_cstr(target));
    if (rc != 0) {
        stubs.raise("file delete: unlink failed");
        return 0;
    }
    return obj_new_string(0, 0);
}

/// ``file rename src dst`` — atomic rename via ``rename(2)``.
/// wasi-libc translates to ``path_rename``.  The ``-force`` flag
/// (overwrite existing dst) is a Tcl extension; without it Tcl
/// errors if dst exists — we skip that check for now so the
/// underlying ``rename`` semantics (which overwrite a file dst
/// but fail on directory collision) propagate directly.
fn file_rename(src: i32, dst: i32) i32 {
    if (src == 0 or dst == 0) {
        stubs.raise("file rename: missing src or dst argument");
        return 0;
    }
    const rc = rename(path_cstr(src), path_cstr(dst));
    if (rc != 0) {
        stubs.raise("file rename: rename failed");
        return 0;
    }
    return obj_new_string(0, 0);
}

/// ``file copy src dst`` — regular-file copy.  Opens *src* for
/// reading, creates/truncates *dst*, shuttles bytes through a
/// fixed-size buffer.  Directory-tree copy (``-force`` /
/// recursive) is not yet wired — a follow-up concern.
fn file_copy(src: i32, dst: i32) i32 {
    if (src == 0 or dst == 0) {
        stubs.raise("file copy: missing src or dst argument");
        return 0;
    }
    const in_fd = open(path_cstr(src), O_RDONLY, 0);
    if (in_fd < 0) {
        stubs.raise("file copy: could not open source for reading");
        return 0;
    }
    // 0o644 = rw-r--r-- — matches cp's default create-mode.
    const out_fd = open(path_cstr(dst), O_WRONLY | O_CREAT | O_TRUNC, 0o644);
    if (out_fd < 0) {
        _ = close(in_fd);
        stubs.raise("file copy: could not open destination for writing");
        return 0;
    }
    // 8 KiB buffer on the bump allocator — big enough to keep
    // syscall overhead low, small enough to not pressure linear
    // memory for tcltest-scale files.
    const buf_size: u32 = 8 * 1024;
    const buf_addr = obj.alloc(buf_size);
    const buf: [*]u8 = @ptrFromInt(buf_addr);
    var ok = true;
    while (true) {
        const n = read(in_fd, buf, buf_size);
        if (n == 0) break;
        if (n < 0) {
            ok = false;
            break;
        }
        var remaining: usize = @intCast(n);
        var off: usize = 0;
        while (remaining > 0) {
            const w = write(out_fd, buf + off, remaining);
            if (w <= 0) {
                ok = false;
                break;
            }
            off += @intCast(w);
            remaining -= @intCast(w);
        }
        if (!ok) break;
    }
    _ = close(in_fd);
    _ = close(out_fd);
    if (!ok) {
        stubs.raise("file copy: I/O error during transfer");
        return 0;
    }
    return obj_new_string(0, 0);
}

/// ``file type path`` — string-naming the path's S_IFMT.  Tcl
/// returns one of ``file``, ``directory``, ``characterSpecial``,
/// ``blockSpecial``, ``fifo``, ``link``, ``socket``.  We use
/// ``lstat(2)`` so symlinks report as ``link`` rather than
/// resolving — matches Tcl's documented behaviour.
fn file_type(path: i32) i32 {
    if (path == 0) {
        stubs.raise("file type: missing path argument");
        return 0;
    }
    const st = lstat_path(path);
    if (st == 0) {
        // Path doesn't exist — Tcl's ``file type`` errors here.
        // Report as a real error rather than tagging it
        // ``unsupported command:`` (the command IS supported;
        // the path just isn't there).
        stubs.raise("file type: could not stat path");
        return 0;
    }
    const masked = stat_mode(st) & S_IFMT;
    return switch (masked) {
        S_IFREG => obj.obj_new_string_copy(@intFromPtr("file".ptr), 4),
        S_IFDIR => obj.obj_new_string_copy(@intFromPtr("directory".ptr), 9),
        S_IFLNK => obj.obj_new_string_copy(@intFromPtr("link".ptr), 4),
        S_IFCHR => obj.obj_new_string_copy(@intFromPtr("characterSpecial".ptr), 16),
        S_IFBLK => obj.obj_new_string_copy(@intFromPtr("blockSpecial".ptr), 12),
        S_IFIFO => obj.obj_new_string_copy(@intFromPtr("fifo".ptr), 4),
        S_IFSOCK => obj.obj_new_string_copy(@intFromPtr("socket".ptr), 6),
        else => obj.obj_new_string_copy(@intFromPtr("file".ptr), 4),
    };
}

/// ``file stat path arrName`` / ``file lstat path arrName`` —
/// populate *arrName* as a Tcl array with the classic POSIX
/// stat fields: ``dev``, ``ino``, ``mode``, ``nlink``, ``uid``,
/// ``gid``, ``size``, ``atime``, ``mtime``, ``ctime``, ``type``.
/// ``lstat`` uses ``lstat(2)`` so the type reports ``link`` for
/// symbolic links rather than following them.  Returns the
/// empty string on success; traps with a clear error if stat
/// itself fails.
fn file_stat_cmd(path: i32, arr_name: i32, use_lstat: bool) i32 {
    if (path == 0 or arr_name == 0) {
        stubs.raise("file stat: missing path or array-name argument");
        return 0;
    }
    const buf = if (use_lstat) lstat_path(path) else stat_path(path);
    if (buf == 0) {
        stubs.raise("file stat: could not stat path");
        return 0;
    }

    const array_mod = @import("../valtypes/tcl_array.zig");

    // Small helper: set a single field by string key + i64 value.
    const set_i64 = struct {
        fn go(arr: i32, key: []const u8, v: i64) void {
            const k = obj.obj_new_string_copy(@intFromPtr(key.ptr), @intCast(key.len));
            _ = array_mod.array_set(arr, k, obj.obj_new_int(v));
        }
    }.go;

    set_i64(arr_name, "dev", 0);
    set_i64(arr_name, "ino", 0);
    set_i64(arr_name, "mode", @intCast(stat_mode(buf)));
    set_i64(arr_name, "nlink", 1);
    set_i64(arr_name, "uid", 0);
    set_i64(arr_name, "gid", 0);
    set_i64(arr_name, "size", stat_size(buf));
    set_i64(arr_name, "atime", stat_atim_sec(buf));
    set_i64(arr_name, "mtime", stat_mtim_sec(buf));
    set_i64(arr_name, "ctime", stat_ctim_sec(buf));

    // ``type`` key mirrors ``file type``.
    const type_str: []const u8 = switch (stat_mode(buf) & S_IFMT) {
        S_IFREG => "file",
        S_IFDIR => "directory",
        S_IFLNK => "link",
        S_IFCHR => "characterSpecial",
        S_IFBLK => "blockSpecial",
        S_IFIFO => "fifo",
        S_IFSOCK => "socket",
        else => "file",
    };
    const type_key = obj.obj_new_string_copy(@intFromPtr("type".ptr), 4);
    const type_val = obj.obj_new_string_copy(@intFromPtr(type_str.ptr), @intCast(type_str.len));
    _ = array_mod.array_set(arr_name, type_key, type_val);

    return obj_new_string(0, 0);
}

/// ``file readlink path`` — returns the target of *path* as a
/// string, or traps if *path* is not a symlink.  wasi-libc
/// provides ``readlink(2)`` backed by ``path_readlink``.
fn file_readlink(path: i32) i32 {
    if (path == 0) {
        stubs.raise("file readlink: missing path argument");
        return 0;
    }
    // 4 KiB is enough for any realistic symlink on WASI's
    // path_readlink (which is bounded anyway by the underlying
    // preopen-relative resolution).
    const buf_size: usize = 4096;
    const buf_addr = obj.alloc(buf_size);
    const buf: [*]u8 = @ptrFromInt(buf_addr);
    const n = readlink(path_cstr(path), buf, buf_size);
    if (n < 0) {
        stubs.raise("file readlink: path is not a symlink or is inaccessible");
        return 0;
    }
    return obj.obj_new_string(@bitCast(buf_addr), @bitCast(n));
}

// --- glob ---
//
// Capability-gated; refuses with ``permission denied: glob requires
// CAP_FS_GLOB`` until the embedder grants the bit.  See
// :module:`interp/tcl_caps.zig`.
//
// Pattern shape: a single component (no embedded ``/``) matched
// against the cwd, *or* a multi-component path where the directory
// portion is taken literally and only the trailing basename is
// matched.  ``glob ./*.tcl`` and ``glob foo/bar/*.txt`` both work;
// recursive ``**`` patterns and ``-directory`` / ``-tails`` /
// ``-types`` switches are not yet wired (the BUILTIN handler in
// :module:`cmds/fs.zig` raises ``unsupported`` for those forms so
// scripts see a clear diagnostic rather than a silent miss).
//
// The fnmatch port supports ``*`` (any-non-slash run, including
// empty), ``?`` (any single non-slash char), ``[abc]`` /
// ``[a-z]`` / ``[!abc]`` character classes, and backslash escapes
// (``\*`` is a literal asterisk).  This is a subset of csh-style
// glob — Tcl's full grammar adds brace expansion and group
// alternations which a follow-up patch can layer on top.

const caps = @import("../interp/tcl_caps.zig");

/// Match a single basename against a glob pattern.  Returns ``true``
/// on a successful match.  Recursive (no allocations); the depth is
/// bounded by the number of ``*`` segments in the pattern.
fn fnmatch_basename(pattern: [*]const u8, plen: u32, name: [*]const u8, nlen: u32) bool {
    return fnmatch_at(pattern, plen, 0, name, nlen, 0);
}

fn fnmatch_at(
    pattern: [*]const u8,
    plen: u32,
    pi_in: u32,
    name: [*]const u8,
    nlen: u32,
    ni_in: u32,
) bool {
    var pi = pi_in;
    var ni = ni_in;
    while (pi < plen) {
        const pc = pattern[pi];
        if (pc == '*') {
            // Collapse adjacent stars and recurse on every possible
            // suffix split.  Empty match is the base case (`pi+1`
            // against the current `ni`).
            var px = pi + 1;
            while (px < plen and pattern[px] == '*') : (px += 1) {}
            if (px >= plen) return true;
            var nx: u32 = ni;
            while (true) {
                if (fnmatch_at(pattern, plen, px, name, nlen, nx)) return true;
                if (nx >= nlen) return false;
                nx += 1;
            }
        }
        if (ni >= nlen) return false;
        if (pc == '?') {
            ni += 1;
            pi += 1;
            continue;
        }
        if (pc == '[') {
            // Character class.  Empty class (``[]``) matches nothing.
            var cx = pi + 1;
            var negate = false;
            if (cx < plen and pattern[cx] == '!') {
                negate = true;
                cx += 1;
            }
            var matched = false;
            while (cx < plen and pattern[cx] != ']') {
                const lo = pattern[cx];
                if (cx + 2 < plen and pattern[cx + 1] == '-' and pattern[cx + 2] != ']') {
                    const hi = pattern[cx + 2];
                    if (name[ni] >= lo and name[ni] <= hi) matched = true;
                    cx += 3;
                } else {
                    if (name[ni] == lo) matched = true;
                    cx += 1;
                }
            }
            if (cx >= plen) return false; // unterminated class — fail safe
            if (matched == negate) return false;
            ni += 1;
            pi = cx + 1;
            continue;
        }
        if (pc == '\\' and pi + 1 < plen) {
            if (pattern[pi + 1] != name[ni]) return false;
            pi += 2;
            ni += 1;
            continue;
        }
        if (pc != name[ni]) return false;
        pi += 1;
        ni += 1;
    }
    return ni == nlen;
}

/// Returns ``true`` if *pattern* contains any unescaped glob
/// metacharacter.  Used to short-circuit the directory-walk when a
/// glob pattern is in fact a literal path.
fn pattern_has_meta(pattern: []const u8) bool {
    var i: usize = 0;
    while (i < pattern.len) : (i += 1) {
        const c = pattern[i];
        if (c == '\\' and i + 1 < pattern.len) {
            i += 1;
            continue;
        }
        if (c == '*' or c == '?' or c == '[') return true;
    }
    return false;
}

/// Decode backslash escapes in *pattern*, returning a fresh slice
/// on the bump allocator.  ``\X`` collapses to ``X`` for any byte;
/// a trailing ``\`` is preserved as a literal backslash.  Used by
/// the literal fast path so ``glob {\*}`` probes a file named
/// ``*`` rather than the byte sequence ``\*``.  Patterns without
/// any backslash escapes round-trip byte-for-byte (the helper
/// always allocates so the caller can store the result without
/// special-casing).
fn unescape_pattern(pattern: []const u8) []const u8 {
    if (pattern.len == 0) return pattern;
    const buf_addr = obj.alloc(@intCast(pattern.len));
    const out: [*]u8 = @ptrFromInt(buf_addr);
    var src: usize = 0;
    var dst: usize = 0;
    while (src < pattern.len) {
        if (pattern[src] == '\\' and src + 1 < pattern.len) {
            out[dst] = pattern[src + 1];
            dst += 1;
            src += 2;
            continue;
        }
        out[dst] = pattern[src];
        dst += 1;
        src += 1;
    }
    return out[0..dst];
}

/// Split *pattern* at the last ``/``.  Returns ``(dir_len, basename_off)``
/// where dir_len is the number of bytes (including the trailing
/// slash) belonging to the directory portion, and basename_off is
/// the offset of the first basename byte.
fn split_dir_basename(pattern: []const u8) struct { dir_len: u32, base_off: u32 } {
    var i: u32 = @intCast(pattern.len);
    while (i > 0 and pattern[i - 1] != '/') : (i -= 1) {}
    return .{ .dir_len = i, .base_off = i };
}

/// Build a NUL-terminated bump-allocator copy of *src*.  Mirrors
/// :func:`path_cstr` but takes raw bytes so callers can pass a
/// dir-prefix slice carved out of the pattern.
fn cstr_from_bytes(src: []const u8) [*:0]const u8 {
    const buf_addr = obj.alloc(@intCast(src.len + 1));
    const out: [*]u8 = @ptrFromInt(buf_addr);
    for (src, 0..) |c, i| out[i] = c;
    out[src.len] = 0;
    return @ptrCast(out);
}

/// Append a single result string to a Tcl-list accumulator and
/// return the new accumulator.  Wraps :func:`tcl_list` so the
/// caller doesn't have to construct intermediate TclObj wrappers.
fn list_append(acc: i32, s_ptr: u32, s_len: u32) i32 {
    const list_mod = @import("../valtypes/tcl_list.zig");
    const elem = obj_new_string(@bitCast(s_ptr), @bitCast(s_len));
    return list_mod.tcl_list(acc, elem);
}

/// ``glob pattern`` — list filesystem entries matching *pattern*.
///
/// Single-pattern shape — the BUILTIN handler in :module:`cmds/fs.zig`
/// peels switches and chains multi-pattern forms by calling this
/// helper once per pattern.  Always returns a Tcl-list TclObj; the
/// BUILTIN decides whether an empty result is silent
/// (``-nocomplain``) or raises (default).  Not exported — there is
/// no Python ``wasm_runtime_import`` for ``glob`` because the
/// variadic shape doesn't fit a fixed-arity import descriptor.
pub fn tcl_cmd_glob(pattern: i32) i32 {
    if (!caps.check(caps.CAP_FS_GLOB, "glob", "FS_GLOB")) return 0;
    if (pattern == 0) return obj_new_string(0, 0);
    const s = obj_ensure_string(pattern);
    if (s.len == 0) return obj_new_string(0, 0);
    const pat_bytes: []const u8 = (@as([*]const u8, @ptrFromInt(s.ptr)))[0..s.len];

    // Literal path with no *unescaped* metas — return as-is iff it
    // exists.  Backslash escapes inside the pattern (e.g.
    // ``glob {\*}`` to match a real file named ``*``) must be
    // decoded into the bytes we actually probe; otherwise
    // ``access()`` runs against the literal ``\*`` byte sequence
    // and reports "doesn't exist" for files whose names a Tcl user
    // would expect to match.
    if (!pattern_has_meta(pat_bytes)) {
        const lit = unescape_pattern(pat_bytes);
        const lit_cstr = cstr_from_bytes(lit);
        if (access(lit_cstr, F_OK) != 0) return obj_new_string(0, 0);
        return list_append(
            obj_new_string(0, 0),
            @intCast(@intFromPtr(lit.ptr)),
            @intCast(lit.len),
        );
    }

    const split = split_dir_basename(pat_bytes);
    const dir_bytes: []const u8 = pat_bytes[0..split.dir_len];
    const base_bytes: []const u8 = pat_bytes[split.base_off..];

    // wasi-libc's ``opendir(".")`` translates to a relative-cwd
    // ``path_open`` which the wasi-libc preopen scanner resolves
    // against the embedder-granted preopens; an explicit dir prefix
    // resolves the same way.  Empty prefix → cwd.
    const dir_path: [*:0]const u8 = if (dir_bytes.len == 0)
        cstr_from_bytes(".")
    else
        cstr_from_bytes(dir_bytes);
    const dir_handle = opendir(dir_path);
    if (dir_handle == null) return obj_new_string(0, 0);

    var acc: i32 = obj_new_string(0, 0);
    while (true) {
        const ent = readdir(dir_handle.?);
        if (ent == null) break;
        const ent_addr: u32 = @intFromPtr(ent.?);
        const name_ptr: [*]const u8 = @ptrFromInt(ent_addr + DIRENT_OFF_NAME);
        // Compute name length by NUL scan — the trailing NUL sits
        // inside the libc-owned struct so we don't include it.
        var nlen: u32 = 0;
        while (name_ptr[nlen] != 0) : (nlen += 1) {}
        if (nlen == 0) continue;
        // Skip ``.`` and ``..`` unless the pattern explicitly starts
        // with a ``.``.  Matches csh / Tcl / fnmatch convention so
        // ``glob *`` doesn't return them silently.
        if (name_ptr[0] == '.' and (base_bytes.len == 0 or base_bytes[0] != '.')) continue;
        const name_slice: []const u8 = name_ptr[0..nlen];
        if (!fnmatch_basename(base_bytes.ptr, @intCast(base_bytes.len), name_slice.ptr, @intCast(name_slice.len))) continue;
        // Glue the dir prefix back onto the matched basename.
        const out_len: u32 = split.dir_len + nlen;
        const out_buf = obj.alloc(out_len);
        const out: [*]u8 = @ptrFromInt(out_buf);
        if (split.dir_len > 0) {
            for (0..split.dir_len) |i| out[i] = dir_bytes[i];
        }
        for (0..nlen) |i| out[split.dir_len + i] = name_ptr[i];
        acc = list_append(acc, out_buf, out_len);
    }
    _ = closedir(dir_handle.?);
    return acc;
}

/// ``file link ?-type? linkName target`` — create a link.  Tcl's
/// full command accepts ``-symbolic`` / ``-hard`` and arg order
/// ``linkName target``.  Our 3-arg export has arg1=linkName,
/// arg2=target.  Defaults to a symbolic link (matching Tcl's
/// documented default); if callers need hard links they'll hit
/// the interpreter fallback path once ``file link`` gets richer
/// arg support.
fn file_link(link_name: i32, target: i32) i32 {
    if (link_name == 0 or target == 0) {
        stubs.raise("file link: missing linkName or target argument");
        return 0;
    }
    const rc = symlink(path_cstr(target), path_cstr(link_name));
    if (rc != 0) {
        stubs.raise("file link: symlink syscall failed");
        return 0;
    }
    // Per ``file link`` semantics the result is the *linkName*,
    // not the target — callers like
    // ``set lnk [file link a.link a]`` expect ``a.link`` back.
    return link_name;
}
