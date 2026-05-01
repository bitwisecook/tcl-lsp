// Channel registry + WASI-backed file I/O.
//
// Tcl scripts call ``open``, ``read``, ``gets``, ``puts $fd ...``,
// ``seek``, ``tell``, ``eof``, ``close``, ``fblocked``, ``fcopy``;
// each command resolves the channel name (``stdin`` / ``stdout`` /
// ``stderr`` / ``fileN``) to a slot in :data:`channels`, then routes
// the operation through wasi-libc's POSIX wrappers (open / read /
// write / lseek / close).  Without this layering each command would
// have to duplicate the path-cstr / fd / read-buffer dance.
//
// Layout — one ``Channel`` per slot:
//
//   * Slots 0/1/2 are pre-populated for ``stdin`` / ``stdout`` /
//     ``stderr`` so ``puts stderr "..."`` and ``gets stdin``
//     work without an explicit ``open``.
//   * Slots 3..MAX_CHANNELS-1 are user channels named ``fileN``
//     where N is the slot index.  Allocated on ``open`` and freed
//     on ``close``.
//
// Reference: ``tmp/tcl9.0.3/generic/tclIO.c`` (``Tcl_OpenFileChannel``,
// ``Tcl_ReadChars``, ``Tcl_GetsObj``, ``Tcl_Seek``, ``Tcl_Tell``).
// We don't model Tcl's ChannelType vtable; the operations are direct
// wasi-libc calls because every channel here is a real file fd.

const std = @import("std");
const obj = @import("../valtypes/tcl_obj.zig");
const stubs = @import("../stubs/tcl_stubs.zig");
const list_quote = @import("../valtypes/tcl_list_quote.zig");
const list_parse = @import("../valtypes/tcl_list_parse.zig");
const encoding = @import("../valtypes/tcl_encoding.zig");

const obj_new_string = obj.obj_new_string;
const obj_new_string_copy = obj.obj_new_string_copy;
const obj_new_int = obj.obj_new_int;
const obj_ensure_string = obj.obj_ensure_string;

// -- libc imports (resolve against wasi-libc) --
//
// Same shape as ``tcl_fs.zig``'s libc externs.  Variadic ``open``
// is pinned to a fixed three-arg form: the third ``mode`` arg is
// only consulted when ``O_CREAT`` is set in the flags; passing 0
// when not creating is harmless on wasi-libc's calling convention.

const open_fn = @extern(
    *const fn ([*:0]const u8, c_int, c_uint) callconv(.c) c_int,
    .{ .name = "open" },
);
const read_fn = @extern(
    *const fn (c_int, [*]u8, usize) callconv(.c) isize,
    .{ .name = "read" },
);
const write_fn = @extern(
    *const fn (c_int, [*]const u8, usize) callconv(.c) isize,
    .{ .name = "write" },
);
const close_fn = @extern(
    *const fn (c_int) callconv(.c) c_int,
    .{ .name = "close" },
);
// ``lseek`` is implemented via direct WASI ``fd_seek`` / ``fd_tell``
// calls below — wasi-libc's ``lseek`` shim adds a calling-convention
// wrapper that doesn't bitcast cleanly to a Zig function-pointer
// extern.  ``fd_seek`` takes the new offset by out-pointer instead
// of returning it, so :func:`do_seek` and :func:`do_tell` adapt.

// (Zig provides built-in ``c_int`` / ``c_uint`` types; we use them
// directly so the wasi-libc extern signatures match the headers.)

// open() flags — wasi-libc encoding (see tcl_fs.zig for the
// rationale; these are not the standard musl bit positions).
const O_RDONLY: c_int = 0x04000000;
const O_WRONLY: c_int = 0x10000000;
const O_RDWR: c_int = O_RDONLY | O_WRONLY;
const O_CREAT: c_int = 1 << 12;
const O_TRUNC: c_int = 8 << 12;
const O_APPEND: c_int = 1 << 0;

// fd_seek whence — these match :data:`std.os.wasi.whence_t` ordinals.
const SEEK_SET: u8 = 0;
const SEEK_CUR: u8 = 1;
const SEEK_END: u8 = 2;

fn do_seek(fd: c_int, offset: i64, whence: u8) ?i64 {
    var new_off: u64 = 0;
    const wh: std.os.wasi.whence_t = @enumFromInt(whence);
    const rc = std.os.wasi.fd_seek(@intCast(fd), offset, wh, &new_off);
    if (rc != .SUCCESS) return null;
    return @intCast(new_off);
}

fn do_tell(fd: c_int) ?i64 {
    var off: u64 = 0;
    const rc = std.os.wasi.fd_tell(@intCast(fd), &off);
    if (rc != .SUCCESS) return null;
    return @intCast(off);
}

// Translation modes — values stored on the channel; consulted by
// ``read``/``gets`` (input side) and ``puts`` (output side).
pub const TR_AUTO: u32 = 0;
pub const TR_LF: u32 = 1;
pub const TR_CR: u32 = 2;
pub const TR_CRLF: u32 = 3;
pub const TR_BINARY: u32 = 4;

// Buffering / encoding — declarative only today; the real wiring
// is "synchronous unbuffered" because wasi-libc's read/write are
// already line-of-sight to the host filesystem.
pub const BUF_FULL: u32 = 0;
pub const BUF_LINE: u32 = 1;
pub const BUF_NONE: u32 = 2;

pub const MODE_READ: u32 = 1;
pub const MODE_WRITE: u32 = 2;

const READ_BUF_SIZE: u32 = 4096;
const WRITE_BUF_SIZE: u32 = 4096;
const MAX_CHANNELS: u32 = 64;

// Default values for option-state TclObj slots — when a slot is
// 0, the renderer falls back to these constants.  This keeps the
// initial state heap-free (no allocation at module load) while
// still letting the setter store arbitrary-length values.
const DEFAULT_ENCODING_STR: []const u8 = "utf-8";
const DEFAULT_EOFCHAR_STR: []const u8 = "";
const DEFAULT_PROFILE_STR: []const u8 = "strict";

const Channel = struct {
    in_use: bool,
    fd: c_int,
    mode: u32,
    eof: bool,
    blocked: bool,
    translation: u32,
    // Codec applied on read (bytes → utf-8) and write (utf-8 →
    // bytes).  ``Enc.pass`` covers identity / utf-8 / binary — no
    // transformation, the legacy code path.  Non-pass codecs route
    // through ``encoding.encode`` / ``encoding.decode_step`` and a
    // single-codepoint decoded utf-8 pull-side buffer (``dec_*``).
    enc: encoding.Enc,
    buffering: u32,
    // Per-option state retained for fconfigure query forms.  Set
    // accepts any value; query renders these back as their
    // canonical Tcl strings.  String-valued options are stored as
    // refcounted TclObj handles (0 → use the default constant), so
    // long values round-trip without truncation and the channel
    // struct stays a fixed size.
    blocking: bool,
    buffersize: u32,
    encoding_obj: i32,
    eofchar_obj: i32,
    profile_obj: i32,
    // Pull-side buffer for gets/read.  Allocated lazily on first
    // read so unused channels don't reserve 4 KiB.
    buf_addr: u32,
    buf_pos: u32,
    buf_end: u32,
    // Push-side buffer for puts/flush.  Allocated lazily on first
    // write; sized by ``-buffersize`` (default 4096).  ``out_buf_pos``
    // counts post-translation bytes accumulated so far.  CRLF/CR
    // translation is applied byte-by-byte on the way in, so a ``\n``
    // arriving when only one slot remains is split across two
    // buffers (the ``savedLF > 0`` case from upstream io-2.2).
    out_buf_addr: u32,
    out_buf_size: u32,
    out_buf_pos: u32,
    // Decoded utf-8 buffer.  Holds the utf-8 expansion of a SINGLE
    // codepoint (≤ 4 bytes) so ``buf_pos`` stays aligned with the
    // raw bytes whose codepoints have been fully delivered.  Only
    // populated when ``enc != .pass``; ``dec_raw`` records how many
    // raw bytes the codepoint cost so we know how far to bump
    // ``buf_pos`` once the user has drained dec_*.
    dec_addr: u32,
    dec_cap: u32,
    dec_pos: u32,
    dec_end: u32,
    dec_raw: u32,
};

var channels: [MAX_CHANNELS]Channel = init: {
    var arr: [MAX_CHANNELS]Channel = undefined;
    var i: usize = 0;
    while (i < MAX_CHANNELS) : (i += 1) {
        arr[i] = .{
            .in_use = false,
            .fd = -1,
            .mode = 0,
            .eof = false,
            .blocked = false,
            .translation = TR_AUTO,
            .enc = .pass,
            .buffering = BUF_FULL,
            .blocking = true,
            .buffersize = 4096,
            .encoding_obj = 0,
            .eofchar_obj = 0,
            .profile_obj = 0,
            .buf_addr = 0,
            .buf_pos = 0,
            .buf_end = 0,
            .out_buf_addr = 0,
            .out_buf_size = WRITE_BUF_SIZE,
            .out_buf_pos = 0,
            .dec_addr = 0,
            .dec_cap = 0,
            .dec_pos = 0,
            .dec_end = 0,
            .dec_raw = 0,
        };
    }
    // Slots 0/1/2 are pre-populated for stdin/stdout/stderr so
    // ``puts stderr ...`` / ``gets stdin`` work without ``open``.
    arr[0] = .{
        .in_use = true,
        .fd = 0,
        .mode = MODE_READ,
        .eof = false,
        .blocked = false,
        .translation = TR_AUTO,
        .enc = .pass,
        .buffering = BUF_NONE,
        .blocking = true,
        .buffersize = 4096,
        .encoding_obj = 0,
        .eofchar_obj = 0,
        .profile_obj = 0,
        .buf_addr = 0,
        .buf_pos = 0,
        .buf_end = 0,
        .out_buf_addr = 0,
        .out_buf_size = WRITE_BUF_SIZE,
        .out_buf_pos = 0,
        .dec_addr = 0,
        .dec_cap = 0,
        .dec_pos = 0,
        .dec_end = 0,
        .dec_raw = 0,
    };
    arr[1] = .{
        .in_use = true,
        .fd = 1,
        .mode = MODE_WRITE,
        .eof = false,
        .blocked = false,
        .translation = TR_AUTO,
        .enc = .pass,
        .buffering = BUF_LINE,
        .blocking = true,
        .buffersize = 4096,
        .encoding_obj = 0,
        .eofchar_obj = 0,
        .profile_obj = 0,
        .buf_addr = 0,
        .buf_pos = 0,
        .buf_end = 0,
        .out_buf_addr = 0,
        .out_buf_size = WRITE_BUF_SIZE,
        .out_buf_pos = 0,
        .dec_addr = 0,
        .dec_cap = 0,
        .dec_pos = 0,
        .dec_end = 0,
        .dec_raw = 0,
    };
    arr[2] = .{
        .in_use = true,
        .fd = 2,
        .mode = MODE_WRITE,
        .eof = false,
        .blocked = false,
        .translation = TR_AUTO,
        .enc = .pass,
        .buffering = BUF_NONE,
        .blocking = true,
        .buffersize = 4096,
        .encoding_obj = 0,
        .eofchar_obj = 0,
        .profile_obj = 0,
        .buf_addr = 0,
        .buf_pos = 0,
        .buf_end = 0,
        .out_buf_addr = 0,
        .out_buf_size = WRITE_BUF_SIZE,
        .out_buf_pos = 0,
        .dec_addr = 0,
        .dec_cap = 0,
        .dec_pos = 0,
        .dec_end = 0,
        .dec_raw = 0,
    };
    break :init arr;
};

fn eq(a: [*]const u8, alen: u32, literal: []const u8) bool {
    if (alen != literal.len) return false;
    for (0..literal.len) |i| if (a[i] != literal[i]) return false;
    return true;
}

fn parse_uint(p: [*]const u8, len: u32) ?u32 {
    if (len == 0) return null;
    var n: u32 = 0;
    for (0..len) |i| {
        const c = p[i];
        if (c < '0' or c > '9') return null;
        n = n * 10 + (c - '0');
    }
    return n;
}

// Channel-error wording helpers — issue #272.
//
// Each helper builds the upstream Tcl error message (matching
// ``tmp/tcl9.0.3/generic/tclIO.c`` / ``tclIOCmd.c``) into a heap
// buffer and routes it through ``stubs.raise`` so the per-suite
// ``exact matching`` assertions in upstream ioCmd.test pass without
// having to special-case our wording.

/// Concatenate a list of byte slices into a single heap buffer and
/// raise it through the standard error path.
fn raise_parts(parts: []const []const u8) void {
    var total: u32 = 0;
    for (parts) |p| total += @intCast(p.len);
    const buf_addr: u32 = obj.alloc(total);
    const buf: [*]u8 = @ptrFromInt(buf_addr);
    var off: u32 = 0;
    for (parts) |p| {
        for (p) |b| {
            buf[off] = b;
            off += 1;
        }
    }
    const slice = @as([*]const u8, @ptrFromInt(buf_addr))[0..total];
    stubs.raise(slice);
}

/// Render the bytes of a channel-name TclObj as an unowned slice.
/// Returns an empty slice when the obj is null or empty so callers
/// can still produce a well-formed quoted message.
fn name_slice(name_obj: i32) []const u8 {
    if (name_obj == 0) return "";
    const s = obj_ensure_string(name_obj);
    if (s.len == 0) return "";
    const p: [*]const u8 = @ptrFromInt(s.ptr);
    return p[0..s.len];
}

/// Recover the channel slot index from a ``*Channel`` pointer.
/// Used by helper functions (``refill``, ``flush_chan``) that don't
/// see the original name TclObj but still need to interpolate a
/// channel name into an error message.
fn slot_of(c: *const Channel) u32 {
    const base = @intFromPtr(&channels[0]);
    const off = @intFromPtr(c) - base;
    return @intCast(off / @sizeOf(Channel));
}

/// Write the canonical name for ``slot`` (``stdin`` / ``stdout`` /
/// ``stderr`` / ``fileN``) into ``dst`` and return how many bytes
/// were written.  ``dst`` must be at least 16 bytes.
fn write_slot_name(slot: u32, dst: []u8) u32 {
    if (slot == 0) {
        const s = "stdin";
        for (s, 0..) |b, i| dst[i] = b;
        return s.len;
    }
    if (slot == 1) {
        const s = "stdout";
        for (s, 0..) |b, i| dst[i] = b;
        return s.len;
    }
    if (slot == 2) {
        const s = "stderr";
        for (s, 0..) |b, i| dst[i] = b;
        return s.len;
    }
    dst[0] = 'f';
    dst[1] = 'i';
    dst[2] = 'l';
    dst[3] = 'e';
    var digits: [10]u8 = undefined;
    var dlen: u32 = 0;
    var v: u32 = slot;
    if (v == 0) {
        digits[0] = '0';
        dlen = 1;
    } else {
        while (v > 0) : (v /= 10) {
            digits[dlen] = '0' + @as(u8, @intCast(v % 10));
            dlen += 1;
        }
    }
    var i: u32 = 0;
    while (i < dlen) : (i += 1) dst[4 + i] = digits[dlen - 1 - i];
    return 4 + dlen;
}

/// ``can not find channel named "<name>"`` — matches
/// ``tclIO.c::Tcl_GetChannel`` (``tmp/tcl9.0.3/generic/tclIO.c:1466``).
fn raise_unknown_channel(name_obj: i32) void {
    raise_parts(&.{ "can not find channel named \"", name_slice(name_obj), "\"" });
}

/// ``channel "<name>" wasn't opened for reading`` — matches
/// ``tclIOCmd.c::Tcl_ReadObjCmd`` (``tclIOCmd.c:300``).
fn raise_not_readable(name_obj: i32) void {
    raise_parts(&.{ "channel \"", name_slice(name_obj), "\" wasn't opened for reading" });
}

/// ``channel "<name>" wasn't opened for writing`` — matches
/// ``tclIOCmd.c::Tcl_PutsObjCmd`` (``tclIOCmd.c:161``).
fn raise_not_writable(name_obj: i32) void {
    raise_parts(&.{ "channel \"", name_slice(name_obj), "\" wasn't opened for writing" });
}

/// Build the ``can not find...`` / ``... wasn't opened...`` /
/// ``error reading...`` family by slot when the caller doesn't have
/// the original name TclObj (helpers like ``refill`` / ``flush_chan``
/// see only ``*Channel``).
fn raise_io_error_by_slot(c: *const Channel, prefix: []const u8, syscall: []const u8) void {
    var name_buf: [16]u8 = undefined;
    const nlen = write_slot_name(slot_of(c), &name_buf);
    const name = name_buf[0..nlen];
    raise_parts(&.{ prefix, "\"", name, "\": ", syscall });
}

fn raise_read_io_error(c: *const Channel, syscall: []const u8) void {
    raise_io_error_by_slot(c, "error reading ", syscall);
}

fn raise_write_io_error(c: *const Channel, syscall: []const u8) void {
    raise_io_error_by_slot(c, "error writing ", syscall);
}

fn raise_seek_io_error(name_obj: i32, syscall: []const u8) void {
    raise_parts(&.{ "error during seek on \"", name_slice(name_obj), "\": ", syscall });
}

/// ``couldn't open "<path>": <syscall>`` — matches
/// ``tclIOUtil.c::Tcl_FSOpenFileChannel`` (``tclIOUtil.c:2255``).
fn raise_open_failed(path_obj: i32, syscall: []const u8) void {
    raise_parts(&.{ "couldn't open \"", name_slice(path_obj), "\": ", syscall });
}

/// ``illegal access mode "<mode>"`` — matches
/// ``tclIOUtil.c::TclGetOpenMode`` (``tclIOUtil.c:1518``).
fn raise_illegal_access_mode(mode_obj: i32) void {
    raise_parts(&.{ "illegal access mode \"", name_slice(mode_obj), "\"" });
}

/// Map a Tcl channel name to a slot index.  Accepts ``stdin`` /
/// ``stdout`` / ``stderr`` (slots 0/1/2) and ``fileN`` (slot N if
/// allocated).  Returns null on miss; callers raise via stubs.raise.
pub fn resolve(name_obj: i32) ?u32 {
    if (name_obj == 0) return null;
    const s = obj_ensure_string(name_obj);
    if (s.len == 0) return null;
    const p: [*]const u8 = @ptrFromInt(s.ptr);
    if (eq(p, s.len, "stdin")) return 0;
    if (eq(p, s.len, "stdout")) return 1;
    if (eq(p, s.len, "stderr")) return 2;
    if (s.len <= 4 or p[0] != 'f' or p[1] != 'i' or p[2] != 'l' or p[3] != 'e') {
        return null;
    }
    const n = parse_uint(p + 4, s.len - 4) orelse return null;
    if (n >= MAX_CHANNELS) return null;
    if (!channels[n].in_use) return null;
    return n;
}

fn alloc_slot() ?u32 {
    var i: u32 = 3;
    while (i < MAX_CHANNELS) : (i += 1) {
        if (!channels[i].in_use) return i;
    }
    return null;
}

// Two static slots for ``path_cstr`` so call sites that thread two
// paths simultaneously (``rename src dst``, ``file copy``, …) don't
// clobber each other's buffer.  WASM is single-threaded so a static
// global is safe.  Cap matches POSIX ``PATH_MAX`` on Linux; longer
// names get truncated, which mirrors what wasi-libc's open would
// reject anyway.
const PATH_MAX: u32 = 4096;
var path_buf_a: [PATH_MAX]u8 = undefined;
var path_buf_b: [PATH_MAX]u8 = undefined;
var path_buf_toggle: u32 = 0;

fn path_cstr(path: i32) [*:0]const u8 {
    const s = obj_ensure_string(path);
    const buf_ptr: [*]u8 = if (path_buf_toggle == 0)
        @ptrCast(&path_buf_a[0])
    else
        @ptrCast(&path_buf_b[0]);
    path_buf_toggle = 1 - path_buf_toggle;
    var n: u32 = s.len;
    if (n >= PATH_MAX) n = PATH_MAX - 1;
    if (n > 0) {
        const src: [*]const u8 = @ptrFromInt(s.ptr);
        for (0..n) |i| buf_ptr[i] = src[i];
    }
    buf_ptr[n] = 0;
    return @ptrCast(buf_ptr);
}

/// Render a channel name for slot ``n`` ("stdin" / "stdout" /
/// "stderr" / "fileN").  The standard streams' interned names live
/// in the data segment; ``fileN`` is built fresh on the bump
/// allocator.
fn slot_name(n: u32) i32 {
    if (n == 0) return obj_new_string_copy(@intFromPtr("stdin".ptr), 5);
    if (n == 1) return obj_new_string_copy(@intFromPtr("stdout".ptr), 6);
    if (n == 2) return obj_new_string_copy(@intFromPtr("stderr".ptr), 6);
    // Render ``fileN`` into a tiny stack buffer and let
    // ``obj_new_string_copy`` mint a TclObj that owns its own
    // payload (the obj's ``OBJ_STR_CAP`` slot is set, so
    // ``release_now`` frees the bytes alongside the header).  The
    // earlier shape used a fresh ``obj.alloc`` whose pointer was
    // wrapped in ``obj_new_string`` *without* the cap slot, so
    // every ``open`` leaked one heap allocation when the channel
    // id obj was eventually released.
    var stack: [16]u8 = undefined;
    stack[0] = 'f';
    stack[1] = 'i';
    stack[2] = 'l';
    stack[3] = 'e';
    var dlen: u32 = 0;
    {
        var digits: [10]u8 = undefined;
        var v: u32 = n;
        if (v == 0) {
            digits[0] = '0';
            dlen = 1;
        } else {
            while (v > 0) : (v /= 10) {
                digits[dlen] = '0' + @as(u8, @intCast(v % 10));
                dlen += 1;
            }
        }
        var i: u32 = 0;
        while (i < dlen) : (i += 1) stack[4 + i] = digits[dlen - 1 - i];
    }
    const total: u32 = 4 + dlen;
    return obj_new_string_copy(@intFromPtr(&stack[0]), @intCast(total));
}

// -- Mode parsing for ``open`` --
//
// Tcl's ``open`` accepts both simple-string modes ("r", "w", "a",
// "r+", "w+", "a+") and POSIX flag-list forms ("RDONLY CREAT").
// We support the simple-string forms here — covers ``open file w``
// / ``open file r`` / ``open file a`` which is what tcltest and
// tcllib reach for.  The flag-list forms route through the eval
// fallback in :file:`cmds/io.zig` until somebody needs them.

const ParsedMode = struct {
    flags: c_int,
    mode: u32, // MODE_READ / MODE_WRITE bits
};

fn parse_access(access: i32) ?ParsedMode {
    if (access == 0) {
        return .{ .flags = O_RDONLY, .mode = MODE_READ };
    }
    const s = obj_ensure_string(access);
    if (s.len == 0) {
        return .{ .flags = O_RDONLY, .mode = MODE_READ };
    }
    const p: [*]const u8 = @ptrFromInt(s.ptr);
    if (eq(p, s.len, "r")) return .{ .flags = O_RDONLY, .mode = MODE_READ };
    if (eq(p, s.len, "r+")) return .{ .flags = O_RDWR, .mode = MODE_READ | MODE_WRITE };
    if (eq(p, s.len, "w")) return .{ .flags = O_WRONLY | O_CREAT | O_TRUNC, .mode = MODE_WRITE };
    if (eq(p, s.len, "w+")) return .{ .flags = O_RDWR | O_CREAT | O_TRUNC, .mode = MODE_READ | MODE_WRITE };
    if (eq(p, s.len, "a")) return .{ .flags = O_WRONLY | O_CREAT | O_APPEND, .mode = MODE_WRITE };
    if (eq(p, s.len, "a+")) return .{ .flags = O_RDWR | O_CREAT | O_APPEND, .mode = MODE_READ | MODE_WRITE };
    return null;
}

/// ``open fileName ?access?`` — two-arg shape (path + mode).  The
/// optional permissions third argument is consumed by the eval-
/// fallback handler in :file:`cmds/io.zig`; the codegen path always
/// passes 0o644 implicitly.  Returns the channel-id TclObj
/// (``fileN``) on success or 0 with a raised error on failure.
pub export fn tcl_cmd_open(path: i32, access: i32) i32 {
    if (path == 0) {
        stubs.raise("open: missing fileName argument");
        return 0;
    }
    const parsed = parse_access(access) orelse {
        raise_illegal_access_mode(access);
        return 0;
    };
    const fd = open_fn(path_cstr(path), parsed.flags, 0o644);
    if (fd < 0) {
        raise_open_failed(path, "i/o error");
        return 0;
    }
    const slot = alloc_slot() orelse {
        _ = close_fn(fd);
        stubs.raise("couldn't allocate channel");
        return 0;
    };
    channels[slot] = .{
        .in_use = true,
        .fd = fd,
        .mode = parsed.mode,
        .eof = false,
        .blocked = false,
        .translation = TR_AUTO,
        .enc = .pass,
        .buffering = BUF_FULL,
        .blocking = true,
        .buffersize = 4096,
        .encoding_obj = 0,
        .eofchar_obj = 0,
        .profile_obj = 0,
        .buf_addr = 0,
        .buf_pos = 0,
        .buf_end = 0,
        .out_buf_addr = 0,
        .out_buf_size = WRITE_BUF_SIZE,
        .out_buf_pos = 0,
        .dec_addr = 0,
        .dec_cap = 0,
        .dec_pos = 0,
        .dec_end = 0,
        .dec_raw = 0,
    };
    return slot_name(slot);
}

/// ``close channelId`` — flush any buffered output, close the
/// underlying fd, and free the slot.  Returns the empty string on
/// success.  Closing stdin/stdout/stderr is silently treated as a
/// flush-only no-op: the WASI host owns those fds, and shutting
/// them mid-script would surprise the embedder.  This diverges
/// from real tclsh (which would close the host stream) in the
/// conservative direction.
pub export fn tcl_cmd_close(chan: i32) i32 {
    const slot = resolve(chan) orelse {
        raise_unknown_channel(chan);
        return 0;
    };
    if (slot < 3) {
        // Per Tcl, closing stdin/stdout/stderr is permitted but
        // would shut the host stream — surprising for a sandbox.
        // We still drain the per-channel write buffer so anything
        // queued via ``-buffering full`` lands on the host stream;
        // a flush failure here is reported as an error so callers
        // notice silent data loss (broken pipe, disk full, …).
        if (!flush_chan(&channels[slot])) {
            raise_write_io_error(&channels[slot], "i/o error");
            return 0;
        }
        return obj_new_string(0, 0);
    }
    const c = &channels[slot];
    // Drain pending writes before tearing the slot down.  If the
    // flush fails the bytes are lost regardless — the fd has to be
    // closed to reclaim the slot — but we still surface the error
    // to the caller rather than masking it as a clean close.
    const flush_ok = flush_chan(c);
    _ = close_fn(c.fd);
    if (c.buf_addr != 0) {
        obj.free_sized(c.buf_addr, READ_BUF_SIZE);
    }
    if (c.out_buf_addr != 0) {
        obj.free_sized(c.out_buf_addr, c.out_buf_size);
    }
    if (c.encoding_obj != 0) obj.tcl_obj_release(c.encoding_obj);
    if (c.eofchar_obj != 0) obj.tcl_obj_release(c.eofchar_obj);
    if (c.profile_obj != 0) obj.tcl_obj_release(c.profile_obj);
    if (c.dec_addr != 0) {
        obj.free_sized(c.dec_addr, c.dec_cap);
    }
    c.* = .{
        .in_use = false,
        .fd = -1,
        .mode = 0,
        .eof = false,
        .blocked = false,
        .translation = TR_AUTO,
        .enc = .pass,
        .buffering = BUF_FULL,
        .blocking = true,
        .buffersize = 4096,
        .encoding_obj = 0,
        .eofchar_obj = 0,
        .profile_obj = 0,
        .buf_addr = 0,
        .buf_pos = 0,
        .buf_end = 0,
        .out_buf_addr = 0,
        .out_buf_size = WRITE_BUF_SIZE,
        .out_buf_pos = 0,
        .dec_addr = 0,
        .dec_cap = 0,
        .dec_pos = 0,
        .dec_end = 0,
        .dec_raw = 0,
    };
    if (!flush_ok) {
        raise_parts(&.{ "error writing \"", name_slice(chan), "\": i/o error" });
        return 0;
    }
    return obj_new_string(0, 0);
}

// Refill the channel's pull-side buffer from the underlying fd.
// Compacts any carry bytes (``buf[buf_pos..buf_end]``) to the front
// of the buffer before reading more — the per-codepoint decoder
// leaves a 1- or 3-byte residue at the tail of the chunk when a
// utf-16 unit straddles the 4 KiB boundary, and that residue must
// survive into the next read.  Sets ``c.eof`` on a zero-byte read.
// Returns the number of bytes available after the refill, or 0 on
// EOF with no carry / error.
fn refill(c: *Channel) u32 {
    if (c.buf_addr == 0) {
        c.buf_addr = obj.alloc(READ_BUF_SIZE);
    }
    var carry: u32 = 0;
    if (c.buf_pos < c.buf_end) {
        carry = c.buf_end - c.buf_pos;
        if (c.buf_pos > 0) {
            const buf: [*]u8 = @ptrFromInt(c.buf_addr);
            for (0..carry) |i| buf[i] = buf[c.buf_pos + i];
        }
    }
    c.buf_pos = 0;
    c.buf_end = carry;
    if (carry >= READ_BUF_SIZE) {
        // Pathological: carry filled the entire buffer.  Nothing to
        // read; caller must consume something first.  Shouldn't
        // happen with the 4-byte max-codepoint codecs we ship.
        return carry;
    }
    const n = read_fn(c.fd, @ptrFromInt(c.buf_addr + carry), READ_BUF_SIZE - carry);
    if (n == 0) {
        // Genuine end-of-file: stamp the sticky EOF flag so
        // ``eof $chan`` reports 1.  Carry bytes (if any) stay in
        // the buffer — the streaming decoder converts them to a
        // U+FFFD on its next pass, then declares EOF.
        c.eof = true;
        return carry;
    }
    if (n < 0) {
        // I/O error from wasi-libc.  Surface as a real error
        // instead of a silent short read; otherwise EOF semantics
        // become indistinguishable from a transport failure and
        // scripts loop forever waiting for a refill that's
        // already broken.
        raise_read_io_error(c, "i/o error");
        return 0;
    }
    c.buf_end = carry + @as(u32, @intCast(n));
    return c.buf_end;
}

// Decode the next codepoint from the raw pull buffer into ``c.dec_*``.
// Stages the codepoint's utf-8 expansion (≤ 4 bytes) into the small
// dec buffer and records the raw bytes the codepoint cost in
// ``c.dec_raw``.  ``c.buf_pos`` is *not* advanced here — it's bumped
// only after the user drains the dec buffer in :func:`next_byte`, so
// position-dependent operations (``tell`` / ``fcopy``) see a raw
// chunk that still includes the in-flight codepoint as "pending".
//
// On EOF mid-unit (utf-16 with one trailing byte) the residue is
// flushed as U+FFFD and ``dec_raw`` reflects the discarded count, so
// the channel reports a single replacement char and then EOF.
fn refill_decoded(c: *Channel) u32 {
    if (c.dec_addr == 0) {
        // Lazy alloc — utf-8 expansion of one codepoint never exceeds
        // 4 bytes, but we round up to the smallest size class via the
        // recycler so the slab can be reused without realloc.
        c.dec_cap = 8;
        c.dec_addr = obj.alloc(c.dec_cap);
    }
    while (true) {
        if (c.buf_pos >= c.buf_end) {
            if (refill(c) == 0) return 0;
        }
        const raw_p: u32 = c.buf_addr + c.buf_pos;
        const raw_len: u32 = c.buf_end - c.buf_pos;
        const step = encoding.decode_step(c.enc, raw_p, raw_len);
        if (step.needs_more) {
            // Try to pull more raw data so the partial unit can be
            // completed.  ``refill`` compacts the carry to the front
            // of the buffer first.
            const before = c.buf_end - c.buf_pos;
            if (refill(c) == 0 and (c.buf_end - c.buf_pos) == before) {
                // Hit EOF with residue still in the buffer — flush
                // it as a single U+FFFD so the read returns the
                // replacement char before declaring EOF.
                c.dec_raw = c.buf_end - c.buf_pos;
                c.dec_pos = 0;
                c.dec_end = encoding.utf8_encode(c.dec_addr, 0xFFFD);
                return c.dec_end;
            }
            continue;
        }
        c.dec_raw = step.consumed;
        c.dec_pos = 0;
        c.dec_end = encoding.utf8_encode(c.dec_addr, step.cp);
        return c.dec_end;
    }
}

// Pull the next utf-8 byte from the channel.  ``c.enc == .pass``
// short-circuits to the raw buffer; otherwise the per-codepoint
// streaming decoder feeds dec_*, advancing ``buf_pos`` only after a
// codepoint is fully delivered so ``tell`` / ``fcopy`` stay aligned
// with the OS fd offset.  Returns null on EOF / error.
fn next_byte(c: *Channel) ?u8 {
    if (c.enc == .pass) {
        if (c.buf_pos >= c.buf_end) {
            if (refill(c) == 0) return null;
        }
        const src: [*]const u8 = @ptrFromInt(c.buf_addr);
        const v = src[c.buf_pos];
        c.buf_pos += 1;
        return v;
    }
    if (c.dec_pos >= c.dec_end) {
        if (refill_decoded(c) == 0) return null;
    }
    const src: [*]const u8 = @ptrFromInt(c.dec_addr);
    const v = src[c.dec_pos];
    c.dec_pos += 1;
    if (c.dec_pos >= c.dec_end) {
        // Codepoint fully delivered — advance raw past its bytes so
        // ``tell``'s ``pos - (buf_end - buf_pos)`` formula stays
        // consistent with what Tcl 9 reports between codepoints.
        c.buf_pos += c.dec_raw;
        c.dec_pos = 0;
        c.dec_end = 0;
        c.dec_raw = 0;
    }
    return v;
}

// Append ``len`` bytes from ``src`` to a growable byte buffer.
// Used by read/gets to assemble the result before wrapping in a
// TclObj.
const ByteBuf = struct {
    addr: u32,
    cap: u32,
    len: u32,
};

fn buf_init(initial: u32) ByteBuf {
    const cap = if (initial < 64) 64 else initial;
    return .{
        .addr = obj.alloc(cap),
        .cap = cap,
        .len = 0,
    };
}

fn buf_grow(b: *ByteBuf, want: u32) void {
    if (b.len + want <= b.cap) return;
    var new_cap = b.cap * 2;
    while (new_cap < b.len + want) new_cap *= 2;
    const new_addr = obj.alloc(new_cap);
    const dst: [*]u8 = @ptrFromInt(new_addr);
    const src: [*]const u8 = @ptrFromInt(b.addr);
    for (0..b.len) |i| dst[i] = src[i];
    obj.free_sized(b.addr, b.cap);
    b.addr = new_addr;
    b.cap = new_cap;
}

fn buf_push(b: *ByteBuf, byte: u8) void {
    buf_grow(b, 1);
    const dst: [*]u8 = @ptrFromInt(b.addr);
    dst[b.len] = byte;
    b.len += 1;
}

fn buf_finish(b: ByteBuf) i32 {
    if (b.len == 0) {
        obj.free_sized(b.addr, b.cap);
        return obj_new_string(0, 0);
    }
    const out = obj_new_string(@intCast(b.addr), @intCast(b.len));
    if (out != 0) {
        obj.write_i32(@as(u32, @intCast(out)) + obj.OBJ_STR_CAP, @bitCast(b.cap));
    }
    return out;
}

// Translate one input byte: with auto/cr/crlf translation, fold
// CR / CRLF to LF.  Returns true if the byte should be emitted,
// false if the caller should skip it (the LF half of a CRLF that
// already emitted an LF for the CR).
fn translate_input(c: *Channel, byte: u8, prev_cr: *bool) struct { emit: bool, value: u8 } {
    if (c.translation == TR_LF or c.translation == TR_BINARY) {
        prev_cr.* = false;
        return .{ .emit = true, .value = byte };
    }
    // auto / cr / crlf — collapse CR / CRLF to LF.
    if (byte == '\r') {
        prev_cr.* = true;
        return .{ .emit = true, .value = '\n' };
    }
    if (byte == '\n' and prev_cr.*) {
        prev_cr.* = false;
        return .{ .emit = false, .value = byte };
    }
    prev_cr.* = false;
    return .{ .emit = true, .value = byte };
}

/// ``read channelId ?numChars?`` — read up to ``numChars`` bytes
/// from the channel, or to EOF when *num_chars* is 0 or absent.
/// Both arguments are TclObj handles so the codegen and eval
/// paths can call this with the same shape.  Returns a TclObj
/// string holding the bytes read (after CR/CRLF translation).
pub export fn tcl_cmd_read(chan: i32, num_chars: i32) i32 {
    const slot = resolve(chan) orelse {
        raise_unknown_channel(chan);
        return 0;
    };
    const c = &channels[slot];
    if ((c.mode & MODE_READ) == 0) {
        raise_not_readable(chan);
        return 0;
    }
    var want: i64 = -1;
    if (num_chars != 0) {
        const ns = obj_ensure_string(num_chars);
        if (ns.len > 0) {
            const parsed = obj.try_parse_int(ns.ptr, ns.len) orelse {
                // Upstream wording: ``expected integer but got "<ns>"``.
                raise_parts(&.{ "expected integer but got \"", name_slice(num_chars), "\"" });
                return 0;
            };
            if (parsed < 0) {
                raise_parts(&.{ "expected non-negative integer but got \"", name_slice(num_chars), "\"" });
                return 0;
            }
            want = parsed;
        }
    }
    var buf = buf_init(if (want > 0 and want < 4096) @intCast(want) else 256);
    var prev_cr = false;
    while (want != 0) {
        const b = next_byte(c) orelse break;
        const r = translate_input(c, b, &prev_cr);
        if (r.emit) {
            buf_push(&buf, r.value);
            if (want > 0) want -= 1;
        }
    }
    return buf_finish(buf);
}

/// ``gets channelId ?varName?`` — read up to and including the
/// next newline.  When ``var_name`` is 0 (one-arg form), returns
/// the line as a TclObj string (newline stripped).  When
/// ``var_name`` is non-zero, the line is stored in the variable
/// and the return value is the byte length (or -1 on EOF).
pub export fn tcl_cmd_gets(chan: i32, var_name: i32) i32 {
    const slot = resolve(chan) orelse {
        raise_unknown_channel(chan);
        return 0;
    };
    const c = &channels[slot];
    if ((c.mode & MODE_READ) == 0) {
        raise_not_readable(chan);
        return 0;
    }
    var line = buf_init(128);
    var saw_newline = false;
    var any_byte = false;
    var prev_cr = false;
    while (true) {
        const b = next_byte(c) orelse break;
        const r = translate_input(c, b, &prev_cr);
        if (!r.emit) continue;
        any_byte = true;
        if (r.value == '\n') {
            saw_newline = true;
            break;
        }
        buf_push(&line, r.value);
    }

    const interp = @import("../interp/tcl_frames.zig");
    if (var_name != 0) {
        if (!any_byte and !saw_newline) {
            // EOF before any byte → -1, var unchanged.
            obj.free_sized(line.addr, line.cap);
            return obj_new_int(-1);
        }
        const len_i64: i64 = @intCast(line.len);
        const line_obj = buf_finish(line);
        _ = interp.var_set(var_name, line_obj);
        return obj_new_int(len_i64);
    }
    return buf_finish(line);
}

/// ``seek channelId offset ?origin?`` — origin word is one of
/// ``start`` / ``current`` / ``end`` (default ``start``).  All
/// arguments arrive as TclObj handles; we parse the offset as a
/// signed decimal and the origin as the keyword.  Drops any
/// pull-side buffer because its contents predate the new fd
/// position.
pub export fn tcl_cmd_seek(chan: i32, offset: i32, origin: i32) i32 {
    const slot = resolve(chan) orelse {
        raise_unknown_channel(chan);
        return 0;
    };
    const c = &channels[slot];
    var off_val: i64 = 0;
    if (offset != 0) {
        const os = obj_ensure_string(offset);
        if (obj.try_parse_int(os.ptr, os.len)) |v| {
            off_val = v;
        }
    }
    var whence: u8 = SEEK_SET;
    if (origin != 0) {
        const rs = obj_ensure_string(origin);
        if (rs.len > 0) {
            const rp: [*]const u8 = @ptrFromInt(rs.ptr);
            if (eq(rp, rs.len, "current")) whence = SEEK_CUR;
            if (eq(rp, rs.len, "end")) whence = SEEK_END;
            if (eq(rp, rs.len, "start")) whence = SEEK_SET;
        }
    }
    // Drain pending writes before moving the fd offset; otherwise
    // the buffered bytes would land at the new position rather than
    // the position they were intended for.
    if (!flush_chan(c)) {
        raise_write_io_error(c, "i/o error");
        return 0;
    }
    if (do_seek(c.fd, off_val, whence) == null) {
        raise_seek_io_error(chan, "i/o error");
        return 0;
    }
    c.buf_pos = 0;
    c.buf_end = 0;
    // Drop any pre-decoded utf-8 sitting in ``dec_*`` for the same
    // reason we drop the raw buffer: its contents predate the new
    // logical position.
    c.dec_pos = 0;
    c.dec_end = 0;
    c.dec_raw = 0;
    c.eof = false;
    return obj_new_string(0, 0);
}

/// ``tell channelId`` — current byte offset, or -1 if unseekable.
/// Subtracts any pre-read bytes still sitting in the pull buffer.
pub export fn tcl_cmd_tell(chan: i32) i32 {
    const slot = resolve(chan) orelse {
        raise_unknown_channel(chan);
        return 0;
    };
    const c = &channels[slot];
    const pos = do_tell(c.fd) orelse return obj_new_int(-1);
    const pending: i64 = @intCast(c.buf_end - c.buf_pos);
    return obj_new_int(pos - pending);
}

/// ``eof channelId`` — 1 if the last input op hit end-of-file, 0
/// otherwise.  Sticky: cleared by ``seek`` / ``close``.
pub export fn tcl_cmd_eof(chan: i32) i32 {
    const slot = resolve(chan) orelse {
        raise_unknown_channel(chan);
        return 0;
    };
    return obj_new_int(if (channels[slot].eof) 1 else 0);
}

/// ``fblocked channelId`` — under WASI's synchronous I/O model
/// reads either complete or hit EOF; "would-block" is never
/// observed.  Always returns 0.
pub export fn tcl_cmd_fblocked(chan: i32) i32 {
    const slot = resolve(chan) orelse {
        raise_unknown_channel(chan);
        return 0;
    };
    _ = slot;
    return obj_new_int(0);
}

/// ``fcopy inputChan outputChan`` — bulk byte copy, blocking.
/// Returns the number of bytes transferred.  ``-size`` /
/// ``-command`` option pairs are not modelled here; the eval
/// handler in :file:`cmds/io.zig` parses ``-size N`` and passes a
/// limit through, but the async ``-command`` form is rejected
/// because the WASM runtime has no event loop.
pub export fn tcl_cmd_fcopy(in_chan: i32, out_chan: i32) i32 {
    return fcopy_limited(in_chan, out_chan, -1);
}

pub fn fcopy_limited(in_chan: i32, out_chan: i32, max_bytes: i64) i32 {
    const in_slot = resolve(in_chan) orelse {
        raise_unknown_channel(in_chan);
        return 0;
    };
    const out_slot = resolve(out_chan) orelse {
        raise_unknown_channel(out_chan);
        return 0;
    };
    const in_c = &channels[in_slot];
    const out_c = &channels[out_slot];
    if ((in_c.mode & MODE_READ) == 0) {
        raise_not_readable(in_chan);
        return 0;
    }
    if ((out_c.mode & MODE_WRITE) == 0) {
        raise_not_writable(out_chan);
        return 0;
    }
    var copied: i64 = 0;

    // Drain any bytes still sitting in the input channel's pull-side
    // buffer before reading more from the fd.  ``read``/``gets``
    // prefetch into ``in_c.buf_*`` via :func:`refill`, which
    // advances the OS file offset past the logical channel
    // position; an unguarded ``read_fn`` call here would skip those
    // bytes (or return them out of order).  We forward the buffer's
    // pending span verbatim — translation already happened on the
    // way into ``in_c.buf_*``.
    // Drain anything still queued on the destination's write
    // buffer first.  fcopy writes raw bytes straight to ``out_c.fd``
    // (no per-byte translation here — that already happened on the
    // input pull side), so any pending buffered output has to land
    // first or it would appear *after* the freshly copied bytes.
    if (!flush_chan(out_c)) {
        raise_write_io_error(out_c, "i/o error");
        return 0;
    }
    if (in_c.buf_pos < in_c.buf_end) {
        const pending: u32 = in_c.buf_end - in_c.buf_pos;
        var take: u32 = pending;
        if (max_bytes >= 0 and @as(i64, take) > max_bytes - copied) {
            take = @intCast(max_bytes - copied);
        }
        if (take > 0) {
            const src: [*]const u8 = @ptrFromInt(in_c.buf_addr + in_c.buf_pos);
            var off: usize = 0;
            var rem: usize = take;
            while (rem > 0) {
                const w = write_fn(out_c.fd, src + off, rem);
                if (w <= 0) {
                    raise_write_io_error(out_c, "i/o error");
                    return 0;
                }
                off += @intCast(w);
                rem -= @intCast(w);
            }
            in_c.buf_pos += take;
            copied += take;
        }
    }

    var tmp: [4096]u8 = undefined;
    while (true) {
        var want: usize = tmp.len;
        if (max_bytes >= 0) {
            const remaining = max_bytes - copied;
            if (remaining <= 0) break;
            if (@as(i64, @intCast(want)) > remaining) want = @intCast(remaining);
        }
        const n = read_fn(in_c.fd, &tmp, want);
        if (n == 0) {
            in_c.eof = true;
            break;
        }
        if (n < 0) {
            raise_read_io_error(in_c, "i/o error");
            return 0;
        }
        var off: usize = 0;
        var rem: usize = @intCast(n);
        while (rem > 0) {
            const w = write_fn(out_c.fd, @ptrCast(&tmp[off]), rem);
            if (w <= 0) {
                raise_write_io_error(out_c, "i/o error");
                return 0;
            }
            off += @intCast(w);
            rem -= @intCast(w);
        }
        copied += @intCast(n);
    }
    return obj_new_int(copied);
}

// Drain the per-channel write buffer to the underlying fd.  Returns
// false on a write failure (caller raises).  Idempotent and safe to
// call when no buffer has been allocated.
pub fn flush_chan(c: *Channel) bool {
    if (c.out_buf_addr == 0 or c.out_buf_pos == 0) {
        c.out_buf_pos = 0;
        return true;
    }
    const ptr: [*]const u8 = @ptrFromInt(c.out_buf_addr);
    var off: usize = 0;
    var rem: usize = c.out_buf_pos;
    while (rem > 0) {
        const w = write_fn(c.fd, ptr + off, rem);
        if (w <= 0) return false;
        off += @intCast(w);
        rem -= @intCast(w);
    }
    c.out_buf_pos = 0;
    return true;
}

// Append a single post-translation byte to the channel's write
// buffer, flushing on a full buffer or when line-buffering hits a
// newline.  Returns false on a write failure.  Allocates the buffer
// lazily on first use.
fn emit_byte(c: *Channel, byte: u8) bool {
    if (c.out_buf_addr == 0) {
        if (c.out_buf_size == 0) c.out_buf_size = WRITE_BUF_SIZE;
        c.out_buf_addr = obj.alloc(c.out_buf_size);
        c.out_buf_pos = 0;
    }
    const dst: [*]u8 = @ptrFromInt(c.out_buf_addr);
    dst[c.out_buf_pos] = byte;
    c.out_buf_pos += 1;
    if (c.out_buf_pos >= c.out_buf_size) {
        if (!flush_chan(c)) return false;
    } else if (c.buffering == BUF_LINE and byte == '\n') {
        if (!flush_chan(c)) return false;
    }
    return true;
}

// Distinguishes a codec-level failure (``encoding.encode`` already
// raised a precise ``codepoint not representable`` / ``lone surrogate``
// message — caller must NOT overwrite that with a generic
// ``write failed``) from an I/O failure on ``write_fn`` / a flush.
const WriteResult = enum { ok, codec_err, io_err };

// Output translation — push ``len`` bytes starting at ``ptr`` (a
// utf-8 byte stream) onto the channel's write buffer with TR_CRLF /
// TR_CR translation applied in utf-8 source space.  When
// ``c.enc != .pass`` the translated utf-8 is then encoded through
// the channel codec and the *encoded* bytes hit ``emit_byte``; for
// ``.pass`` it's a straight byte-by-byte path.  Unbuffered channels
// (``BUF_NONE``) flush at the end so each ``puts`` lands on the fd
// immediately.
fn write_translated(c: *Channel, ptr: [*]const u8, len: u32) WriteResult {
    if (c.enc == .pass) {
        var i: u32 = 0;
        while (i < len) : (i += 1) {
            const b = ptr[i];
            if (c.translation == TR_CRLF and b == '\n') {
                if (!emit_byte(c, '\r')) return .io_err;
                if (!emit_byte(c, '\n')) return .io_err;
            } else if (c.translation == TR_CR and b == '\n') {
                if (!emit_byte(c, '\r')) return .io_err;
            } else {
                if (!emit_byte(c, b)) return .io_err;
            }
        }
        if (c.buffering == BUF_NONE) {
            return if (flush_chan(c)) .ok else .io_err;
        }
        return .ok;
    }
    // Non-pass codec — translation must run in utf-8 source space so
    // a multi-byte codepoint isn't split mid-byte by an LF→CRLF
    // expansion.  Build the translated stream into a temp buffer,
    // encode it, then funnel the encoded bytes through ``emit_byte``
    // so the per-channel write buffer / line-flush rules still apply.
    var staged = encoding.buf_init(if (len > 0) len + 2 else 4);
    defer encoding.buf_release(staged);
    var i: u32 = 0;
    while (i < len) : (i += 1) {
        const b = ptr[i];
        if (c.translation == TR_CRLF and b == '\n') {
            encoding.buf_push(&staged, '\r');
            encoding.buf_push(&staged, '\n');
        } else if (c.translation == TR_CR and b == '\n') {
            encoding.buf_push(&staged, '\r');
        } else {
            encoding.buf_push(&staged, b);
        }
    }
    const enc_buf = encoding.encode(c.enc, staged.addr, staged.len) orelse return .codec_err;
    defer encoding.buf_release(enc_buf);
    const ep: [*]const u8 = @ptrFromInt(enc_buf.addr);
    var j: u32 = 0;
    while (j < enc_buf.len) : (j += 1) {
        if (!emit_byte(c, ep[j])) return .io_err;
    }
    if (c.buffering == BUF_NONE) {
        return if (flush_chan(c)) .ok else .io_err;
    }
    return .ok;
}

/// Resolve a channel id and flush its write buffer.  Used by
/// ``tcl_cmd_flush`` (the runtime entry point) and indirectly by
/// the ``flush`` eval shim.  Returns the empty string on success
/// or 0 with a raised error on failure.
pub fn flush_chan_id(chan_id: i32) i32 {
    if (chan_id == 0) return obj_new_string(0, 0);
    const slot = resolve(chan_id) orelse {
        raise_unknown_channel(chan_id);
        return 0;
    };
    if (!flush_chan(&channels[slot])) {
        raise_write_io_error(&channels[slot], "i/o error");
        return 0;
    }
    return obj_new_string(0, 0);
}

/// Channel-aware puts.  ``chan`` is a TclObj channel id;
/// ``msg`` is the string to write; ``nonewline != 0`` suppresses
/// the trailing newline.  Used by :func:`cmds/io.zig:eval_puts`
/// when it sees a 2-arg form (``puts $fd "..."``).
pub export fn tcl_cmd_puts_chan(chan: i32, msg: i32, nonewline: i32) i32 {
    const slot = resolve(chan) orelse {
        raise_unknown_channel(chan);
        return 0;
    };
    const c = &channels[slot];
    if ((c.mode & MODE_WRITE) == 0) {
        raise_not_writable(chan);
        return 0;
    }
    if (msg != 0) {
        const s = obj_ensure_string(msg);
        if (s.len > 0) {
            switch (write_translated(c, @ptrFromInt(s.ptr), s.len)) {
                .ok => {},
                .codec_err => return 0, // codec already raised a precise message
                .io_err => {
                    raise_write_io_error(c, "i/o error");
                    return 0;
                },
            }
        }
    }
    if (nonewline == 0) {
        const nl: [1]u8 = .{'\n'};
        switch (write_translated(c, &nl, 1)) {
            .ok => {},
            .codec_err => return 0,
            .io_err => {
                raise_write_io_error(c, "i/o error");
                return 0;
            },
        }
    }
    return obj_new_string(0, 0);
}

// -- fconfigure --
//
// Three shapes, mirroring real Tcl:
//
//   1. ``fconfigure $fd``                       — query all options
//      and return ``-name value`` pairs as a Tcl list.
//   2. ``fconfigure $fd -opt``                  — query one option,
//      return its current value (raw, not list-element-quoted).
//   3. ``fconfigure $fd -opt val ?-opt val …?`` — set; returns "".
//
// Settable options:
//   -translation auto|lf|cr|crlf|binary
//   -buffering   full|line|none
//   -encoding    arbitrary string (utf-8 / utf-16 / binary / …)
//   -blocking    boolean (WASI is synchronous; tracked for query
//                only)
//   -buffersize  N (decimal integer)
//   -eofchar     character or 2-element list
//   -profile     tcl8|replace|strict
//
// Queryable-only options (file fd):
//   -error -mode -readable -writable
//
// Unknown options trap via :func:`stubs.unsupported` so scripts
// that rely on (e.g.) ``-handshake`` get a clear error instead of a
// silent no-op.
//
// The args TclObj arrives as a Tcl list assembled by either
// :file:`cmds/chan.zig:eval_fconfigure` (eval-fallback path) or
// :file:`core/compiler/codegen/wasm/_emitter/cmds/fconfigure_.py`
// (codegen literal path).  Both producers emit canonical Tcl list
// elements via ``list_quote.list_elem_quote_nth`` so empty values
// (``{}``) and values containing braces / whitespace round-trip
// through the parser intact — :mod:`tcl_list_parse` handles
// backslash and brace escapes.

fn is_settable_option(p: [*]const u8, len: u32) bool {
    return eq(p, len, "-blocking") or
        eq(p, len, "-buffering") or
        eq(p, len, "-buffersize") or
        eq(p, len, "-encoding") or
        eq(p, len, "-eofchar") or
        eq(p, len, "-profile") or
        eq(p, len, "-translation");
}

fn is_queryable_option(p: [*]const u8, len: u32) bool {
    return is_settable_option(p, len) or
        eq(p, len, "-error") or
        eq(p, len, "-mode") or
        eq(p, len, "-readable") or
        eq(p, len, "-writable");
}

// Replace a TclObj-ref slot with a fresh copy of the bytes at
// *src_p* / *src_len*.  Releases the previous reference (if any)
// and retains the new one so the slot owns exactly one count.  An
// empty value clears the slot back to 0 — render_* falls back to
// the per-option default constant.
fn set_obj_option(slot: *i32, src_p: [*]const u8, src_len: u32) void {
    const old = slot.*;
    if (src_len == 0) {
        slot.* = 0;
    } else {
        const new_obj = obj_new_string_copy(@intFromPtr(src_p), src_len);
        if (new_obj == 0) {
            // Allocation failed; leave slot untouched (don't drop
            // the old value).
            return;
        }
        obj.tcl_obj_retain(new_obj);
        slot.* = new_obj;
    }
    if (old != 0) obj.tcl_obj_release(old);
}

// Read a TclObj-ref option slot as a byte slice.  When the slot is
// 0, fall back to *default_str*.  The returned slice is valid as
// long as the obj or constant lives — both are stable until the
// channel is closed (slot release) or a new value is set.
fn get_obj_option(slot: i32, default_str: []const u8) []const u8 {
    if (slot == 0) return default_str;
    const s = obj_ensure_string(slot);
    if (s.len == 0) return default_str;
    const p: [*]const u8 = @ptrFromInt(s.ptr);
    return p[0..s.len];
}

// Apply a single ``-name value`` pair to ``c``.  Returns false on a
// validation error (raised via ``stubs.raise``) or a resize-time
// flush failure so the caller can stop walking the remaining
// option list and surface a real error.  Success returns true.
fn apply_option(c: *Channel, name_p: [*]const u8, name_len: u32, val_p: [*]const u8, val_len: u32) bool {
    if (eq(name_p, name_len, "-translation")) {
        if (eq(val_p, val_len, "auto")) {
            c.translation = TR_AUTO;
        } else if (eq(val_p, val_len, "lf")) {
            c.translation = TR_LF;
        } else if (eq(val_p, val_len, "cr")) {
            c.translation = TR_CR;
        } else if (eq(val_p, val_len, "crlf")) {
            c.translation = TR_CRLF;
        } else if (eq(val_p, val_len, "binary")) {
            c.translation = TR_BINARY;
        } else {
            stubs.raise("fconfigure: bad value for -translation: must be auto, binary, cr, crlf, or lf");
            return false;
        }
        return true;
    }
    if (eq(name_p, name_len, "-buffering")) {
        if (eq(val_p, val_len, "full")) {
            c.buffering = BUF_FULL;
        } else if (eq(val_p, val_len, "line")) {
            c.buffering = BUF_LINE;
        } else if (eq(val_p, val_len, "none")) {
            c.buffering = BUF_NONE;
        } else {
            stubs.raise("fconfigure: bad value for -buffering: must be full, line, or none");
            return false;
        }
        return true;
    }
    if (eq(name_p, name_len, "-encoding")) {
        const r = encoding.resolve_name(val_p, val_len);
        if (r.enc) |new_enc| {
            // Switching encodings invalidates the decoded utf-8 cache;
            // drop it so the next read re-decodes through the new
            // codec.  (The raw-side buffer stays — its bytes haven't
            // changed.)
            if (c.enc != new_enc) {
                c.enc = new_enc;
                c.dec_pos = 0;
                c.dec_end = 0;
                c.dec_raw = 0;
            }
        } else if (r.known_unsupported) {
            // A real Tcl codec we don't model yet — surface a clear
            // error rather than silently passing bytes through.
            stubs.unsupported("fconfigure -encoding (codec not implemented)");
            return false;
        }
        // Unrecognised free-form names fall through: the runtime
        // codec stays as it was, and ``set_obj_option`` records the
        // string so the query form returns what the caller set.
        // (Real Tcl errors on unknown codecs; main's fconfigure
        // query tests rely on lenient string storage, and that's
        // the established direction here — see PR #271 / #292.)
        set_obj_option(&c.encoding_obj, val_p, val_len);
        return true;
    }
    if (eq(name_p, name_len, "-blocking")) {
        // Treat "0"/"false"/"no"/"off" as false, "1"/"true"/"yes"/
        // "on" as true; anything else is rejected.  WASI I/O is
        // synchronous so the value is only consulted by query.
        if (eq(val_p, val_len, "0") or
            eq(val_p, val_len, "false") or
            eq(val_p, val_len, "no") or
            eq(val_p, val_len, "off"))
        {
            c.blocking = false;
        } else if (eq(val_p, val_len, "1") or
            eq(val_p, val_len, "true") or
            eq(val_p, val_len, "yes") or
            eq(val_p, val_len, "on"))
        {
            c.blocking = true;
        } else {
            stubs.raise("fconfigure: expected boolean value for -blocking");
            return false;
        }
        return true;
    }
    if (eq(name_p, name_len, "-buffersize")) {
        const n = parse_uint(val_p, val_len) orelse {
            stubs.raise("fconfigure: expected integer value for -buffersize");
            return false;
        };
        // Tcl clamps -buffersize to ``[1, 1_000_000]``; mirror that
        // so a hostile script can't request a 4 GiB allocation.
        // ``c.buffersize`` mirrors what the query form returns; the
        // push-side allocation is held separately in ``out_buf_size``
        // so we can drain at the old size before swapping.
        if (n == 0) return true;
        const clamped: u32 = if (n > 1_000_000) 1_000_000 else n;
        c.buffersize = clamped;
        if (clamped == c.out_buf_size) return true;
        // Drain any pending bytes at the old size before swapping
        // the buffer; otherwise a partial write straddles the
        // resize and the second flush would point at freed memory.
        // If the drain fails (broken pipe, ENOSPC, …) we must keep
        // the old buffer intact so the queued bytes aren't silently
        // discarded.  Raise here so the caller doesn't have to
        // distinguish flush failures from validation errors — both
        // arrive as a ``return false`` with a message already on
        // the error channel.
        if (!flush_chan(c)) {
            raise_write_io_error(c, "i/o error");
            return false;
        }
        if (c.out_buf_addr != 0) {
            obj.free_sized(c.out_buf_addr, c.out_buf_size);
            c.out_buf_addr = 0;
        }
        c.out_buf_size = clamped;
        c.out_buf_pos = 0;
        return true;
    }
    if (eq(name_p, name_len, "-eofchar")) {
        set_obj_option(&c.eofchar_obj, val_p, val_len);
        return true;
    }
    if (eq(name_p, name_len, "-profile")) {
        set_obj_option(&c.profile_obj, val_p, val_len);
        return true;
    }
    return true;
}

fn translation_str(t: u32) []const u8 {
    return switch (t) {
        TR_LF => "lf",
        TR_CR => "cr",
        TR_CRLF => "crlf",
        TR_BINARY => "binary",
        else => "auto",
    };
}

fn buffering_str(b: u32) []const u8 {
    return switch (b) {
        BUF_LINE => "line",
        BUF_NONE => "none",
        else => "full",
    };
}

fn append_str_buf(buf: *ByteBuf, s: []const u8) void {
    for (s) |b| buf_push(buf, b);
}

fn append_decimal(buf: *ByteBuf, n: u32) void {
    if (n == 0) {
        buf_push(buf, '0');
        return;
    }
    var digits: [10]u8 = undefined;
    var dlen: u32 = 0;
    var v: u32 = n;
    while (v > 0) : (v /= 10) {
        digits[dlen] = '0' + @as(u8, @intCast(v % 10));
        dlen += 1;
    }
    var i: u32 = dlen;
    while (i > 0) {
        i -= 1;
        buf_push(buf, digits[i]);
    }
}

// Append *s* as a canonical Tcl list element via the shared
// ``tcl_list_quote`` encoder.  Worst-case expansion is ``2*len + 2``
// bytes (every byte gets a backslash + outer braces); pre-grow
// the byte buffer to that ceiling so ``convert_element`` can write
// directly into the slab without re-checking capacity.
fn append_list_element(buf: *ByteBuf, s: []const u8) void {
    const worst: u32 = if (s.len == 0) 2 else @as(u32, @intCast(s.len)) * 2 + 2;
    buf_grow(buf, worst);
    const src_u32: u32 = if (s.len == 0) 0 else @intCast(@intFromPtr(s.ptr));
    const flags = list_quote.scan_element(src_u32, @intCast(s.len), list_quote.FLAG_DONT_QUOTE_HASH);
    const written = list_quote.convert_element(src_u32, @intCast(s.len), buf.addr + buf.len, flags);
    buf.len += written;
}

// Render an option's value into *buf*.  When *as_list_element* is
// true (used by the all-options query), string-valued options are
// list-quoted so the result is a well-formed Tcl list/dict.  When
// false (single-option query), the raw value is emitted so
// ``fconfigure $fd -encoding`` returns the canonical encoding name
// rather than an outer-list-quoted form.
fn render_option_value(
    buf: *ByteBuf,
    c: *const Channel,
    name_p: [*]const u8,
    name_len: u32,
    as_list_element: bool,
) void {
    if (eq(name_p, name_len, "-blocking")) {
        buf_push(buf, if (c.blocking) '1' else '0');
    } else if (eq(name_p, name_len, "-buffering")) {
        append_str_buf(buf, buffering_str(c.buffering));
    } else if (eq(name_p, name_len, "-buffersize")) {
        append_decimal(buf, c.buffersize);
    } else if (eq(name_p, name_len, "-encoding")) {
        const v = get_obj_option(c.encoding_obj, DEFAULT_ENCODING_STR);
        if (as_list_element) {
            append_list_element(buf, v);
        } else {
            append_str_buf(buf, v);
        }
    } else if (eq(name_p, name_len, "-eofchar")) {
        const v = get_obj_option(c.eofchar_obj, DEFAULT_EOFCHAR_STR);
        if (as_list_element) {
            append_list_element(buf, v);
        } else {
            append_str_buf(buf, v);
        }
    } else if (eq(name_p, name_len, "-profile")) {
        const v = get_obj_option(c.profile_obj, DEFAULT_PROFILE_STR);
        if (as_list_element) {
            append_list_element(buf, v);
        } else {
            append_str_buf(buf, v);
        }
    } else if (eq(name_p, name_len, "-translation")) {
        append_str_buf(buf, translation_str(c.translation));
    } else if (eq(name_p, name_len, "-mode")) {
        const r = (c.mode & MODE_READ) != 0;
        const w = (c.mode & MODE_WRITE) != 0;
        if (r and w) {
            append_str_buf(buf, "RDWR");
        } else if (r) {
            append_str_buf(buf, "RDONLY");
        } else if (w) {
            append_str_buf(buf, "WRONLY");
        }
    } else if (eq(name_p, name_len, "-readable")) {
        buf_push(buf, if ((c.mode & MODE_READ) != 0) '1' else '0');
    } else if (eq(name_p, name_len, "-writable")) {
        buf_push(buf, if ((c.mode & MODE_WRITE) != 0) '1' else '0');
    }
    // -error renders empty: we don't track per-channel I/O errors.
}

// All-options query — order matches real tclsh's
// ``Tcl_FconfigureCmd`` output for file channels so ``dict get``
// over the result behaves identically.
const QUERY_ALL_OPTIONS = [_][]const u8{
    "-blocking",
    "-buffering",
    "-buffersize",
    "-encoding",
    "-eofchar",
    "-profile",
    "-translation",
};

fn query_all(c: *const Channel) i32 {
    var buf = buf_init(128);
    inline for (QUERY_ALL_OPTIONS, 0..) |name, idx| {
        if (idx > 0) buf_push(&buf, ' ');
        append_str_buf(&buf, name);
        buf_push(&buf, ' ');
        render_option_value(&buf, c, name.ptr, @intCast(name.len), true);
    }
    return buf_finish(buf);
}

fn query_one(c: *const Channel, name_p: [*]const u8, name_len: u32) i32 {
    if (!is_queryable_option(name_p, name_len)) {
        stubs.unsupported("fconfigure (unsupported option)");
        return 0;
    }
    var buf = buf_init(32);
    render_option_value(&buf, c, name_p, name_len, false);
    return buf_finish(buf);
}

const MAX_FCONFIGURE_WORDS: u32 = 32;

/// ``fconfigure $fd ?option ?value? …?`` — query / set channel
/// options.  *args* is a Tcl list TclObj built by
/// :file:`cmds/chan.zig:eval_fconfigure` (eval path) or
/// :file:`fconfigure_.py` (codegen path); both producers go
/// through ``list_quote.list_elem_quote_nth`` so empty / brace /
/// whitespace values round-trip.
///
/// Three shapes:
///   * empty args      → return all options as ``-name value`` list.
///   * single ``-opt`` → return that option's value.
///   * ``-opt val`` …  → set; returns the empty string.
/// Unknown options trap with ``unsupported command: fconfigure``.
pub export fn tcl_cmd_fconfigure(fd: i32, args: i32) i32 {
    const slot = resolve(fd) orelse {
        // Match the other channel commands' behaviour: an unknown
        // channel id is a hard error, not a silent no-op.
        raise_unknown_channel(fd);
        return 0;
    };
    const c: *Channel = &channels[slot];
    if (args == 0) return query_all(c);
    const a = obj_ensure_string(args);
    if (a.len == 0) return query_all(c);

    const n_i64: i64 = list_parse.count_elements(a.ptr, a.len);
    if (n_i64 <= 0) return query_all(c);
    const n_elems: u32 = @intCast(n_i64);
    if (n_elems > MAX_FCONFIGURE_WORDS) {
        stubs.raise("fconfigure: too many arguments");
        return 0;
    }

    // Decode all elements into one working buffer, recording an
    // (offset, len) pair per element.  Backslash decoding can only
    // shrink the input, so ``a.len`` (plus a sentinel byte for the
    // empty-everything case) is a safe upper bound.  ``defer``
    // returns the buffer to the recycler before any of the early
    // exits below.
    const work_cap: u32 = if (a.len == 0) 1 else a.len;
    const work_buf = obj.alloc(work_cap);
    defer obj.free_sized(work_buf, work_cap);

    var elem_off: [MAX_FCONFIGURE_WORDS]u32 = undefined;
    var elem_len: [MAX_FCONFIGURE_WORDS]u32 = undefined;
    var write_off: u32 = 0;

    var k: u32 = 0;
    while (k < n_elems) : (k += 1) {
        const e = list_parse.element_at(a.ptr, a.len, @intCast(k));
        elem_off[k] = write_off;
        if (e.braced) {
            // Already-literal — copy bytes verbatim.
            const dst: [*]u8 = @ptrFromInt(work_buf + write_off);
            const src: [*]const u8 = @ptrFromInt(a.ptr + e.start);
            for (0..e.len) |bi| dst[bi] = src[bi];
            elem_len[k] = e.len;
            write_off += e.len;
        } else {
            const decoded = list_parse.copy_unbraced_elem(work_buf + write_off, a.ptr + e.start, e.len);
            elem_len[k] = decoded;
            write_off += decoded;
        }
    }

    if (n_elems == 1) {
        const np: [*]const u8 = @ptrFromInt(work_buf + elem_off[0]);
        const nl = elem_len[0];
        if (nl < 2 or np[0] != '-') {
            stubs.unsupported("fconfigure (expected -option)");
            return 0;
        }
        return query_one(c, np, nl);
    }

    if (n_elems & 1 != 0) {
        stubs.raise("fconfigure: value for \"option\" missing");
        return 0;
    }

    // Validate every option name first so a partial-set on a typo
    // doesn't leave the channel half-configured.
    var i: u32 = 0;
    while (i < n_elems) : (i += 2) {
        const np: [*]const u8 = @ptrFromInt(work_buf + elem_off[i]);
        const nl = elem_len[i];
        if (nl < 2 or np[0] != '-') {
            stubs.unsupported("fconfigure (expected -option)");
            return 0;
        }
        if (!is_settable_option(np, nl)) {
            stubs.unsupported("fconfigure (unsupported option)");
            return 0;
        }
    }
    i = 0;
    while (i < n_elems) : (i += 2) {
        const np: [*]const u8 = @ptrFromInt(work_buf + elem_off[i]);
        const nl = elem_len[i];
        const vp: [*]const u8 = @ptrFromInt(work_buf + elem_off[i + 1]);
        const vl = elem_len[i + 1];
        if (!apply_option(c, np, nl, vp, vl)) return 0;
    }
    return obj_new_string(0, 0);
}

/// Total number of channel slots — exposed so ``chan names`` can walk
/// the slot array.
pub const channel_count: u32 = MAX_CHANNELS;

/// Whether slot ``n`` is currently allocated (bound to a real fd).
pub fn slot_in_use(n: u32) bool {
    if (n >= MAX_CHANNELS) return false;
    return channels[n].in_use;
}

/// Render a channel-name TclObj for slot ``n``.  Public wrapper
/// around the private :func:`slot_name` so the ``chan names`` ensemble
/// (in :file:`cmds/chan.zig`) can mint each name without duplicating
/// the ``stdin`` / ``stdout`` / ``stderr`` / ``fileN`` rendering.
pub fn channel_name(n: u32) i32 {
    return slot_name(n);
}
