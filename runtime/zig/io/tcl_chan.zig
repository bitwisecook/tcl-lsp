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

const Channel = struct {
    in_use: bool,
    fd: c_int,
    mode: u32,
    eof: bool,
    blocked: bool,
    translation: u32,
    encoding_binary: bool,
    buffering: u32,
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
            .encoding_binary = false,
            .buffering = BUF_FULL,
            .buf_addr = 0,
            .buf_pos = 0,
            .buf_end = 0,
            .out_buf_addr = 0,
            .out_buf_size = WRITE_BUF_SIZE,
            .out_buf_pos = 0,
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
        .encoding_binary = false,
        .buffering = BUF_NONE,
        .buf_addr = 0,
        .buf_pos = 0,
        .buf_end = 0,
        .out_buf_addr = 0,
        .out_buf_size = WRITE_BUF_SIZE,
        .out_buf_pos = 0,
    };
    arr[1] = .{
        .in_use = true,
        .fd = 1,
        .mode = MODE_WRITE,
        .eof = false,
        .blocked = false,
        .translation = TR_AUTO,
        .encoding_binary = false,
        .buffering = BUF_LINE,
        .buf_addr = 0,
        .buf_pos = 0,
        .buf_end = 0,
        .out_buf_addr = 0,
        .out_buf_size = WRITE_BUF_SIZE,
        .out_buf_pos = 0,
    };
    arr[2] = .{
        .in_use = true,
        .fd = 2,
        .mode = MODE_WRITE,
        .eof = false,
        .blocked = false,
        .translation = TR_AUTO,
        .encoding_binary = false,
        .buffering = BUF_NONE,
        .buf_addr = 0,
        .buf_pos = 0,
        .buf_end = 0,
        .out_buf_addr = 0,
        .out_buf_size = WRITE_BUF_SIZE,
        .out_buf_pos = 0,
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
        stubs.raise("open: unrecognised access mode (use r/r+/w/w+/a/a+)");
        return 0;
    };
    const fd = open_fn(path_cstr(path), parsed.flags, 0o644);
    if (fd < 0) {
        stubs.raise("open: could not open file");
        return 0;
    }
    const slot = alloc_slot() orelse {
        _ = close_fn(fd);
        stubs.raise("open: too many channels");
        return 0;
    };
    channels[slot] = .{
        .in_use = true,
        .fd = fd,
        .mode = parsed.mode,
        .eof = false,
        .blocked = false,
        .translation = TR_AUTO,
        .encoding_binary = false,
        .buffering = BUF_FULL,
        .buf_addr = 0,
        .buf_pos = 0,
        .buf_end = 0,
        .out_buf_addr = 0,
        .out_buf_size = WRITE_BUF_SIZE,
        .out_buf_pos = 0,
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
        stubs.raise("close: unknown channel");
        return 0;
    };
    if (slot < 3) {
        // Per Tcl, closing stdin/stdout/stderr is permitted but
        // would shut the host stream — surprising for a sandbox.
        // We still drain the per-channel write buffer so anything
        // queued via ``-buffering full`` lands on the host stream.
        _ = flush_chan(&channels[slot]);
        return obj_new_string(0, 0);
    }
    const c = &channels[slot];
    _ = flush_chan(c);
    _ = close_fn(c.fd);
    if (c.buf_addr != 0) {
        obj.free_sized(c.buf_addr, READ_BUF_SIZE);
    }
    if (c.out_buf_addr != 0) {
        obj.free_sized(c.out_buf_addr, c.out_buf_size);
    }
    c.* = .{
        .in_use = false,
        .fd = -1,
        .mode = 0,
        .eof = false,
        .blocked = false,
        .translation = TR_AUTO,
        .encoding_binary = false,
        .buffering = BUF_FULL,
        .buf_addr = 0,
        .buf_pos = 0,
        .buf_end = 0,
        .out_buf_addr = 0,
        .out_buf_size = WRITE_BUF_SIZE,
        .out_buf_pos = 0,
    };
    return obj_new_string(0, 0);
}

// Refill the channel's pull-side buffer from the underlying fd.
// Sets ``c.eof`` on a zero-byte read.  Returns the number of bytes
// available after the refill, or 0 on EOF / error.
fn refill(c: *Channel) u32 {
    if (c.buf_addr == 0) {
        c.buf_addr = obj.alloc(READ_BUF_SIZE);
    }
    c.buf_pos = 0;
    c.buf_end = 0;
    const n = read_fn(c.fd, @ptrFromInt(c.buf_addr), READ_BUF_SIZE);
    if (n == 0) {
        // Genuine end-of-file: stamp the sticky EOF flag so
        // ``eof $chan`` reports 1 and signal "no bytes available".
        c.eof = true;
        return 0;
    }
    if (n < 0) {
        // I/O error from wasi-libc.  Surface as a real error
        // instead of a silent short read; otherwise EOF semantics
        // become indistinguishable from a transport failure and
        // scripts loop forever waiting for a refill that's
        // already broken.
        stubs.raise("read: I/O error");
        return 0;
    }
    c.buf_end = @intCast(n);
    return c.buf_end;
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
        stubs.raise("read: unknown channel");
        return 0;
    };
    const c = &channels[slot];
    if ((c.mode & MODE_READ) == 0) {
        stubs.raise("read: channel not open for reading");
        return 0;
    }
    var want: i64 = -1;
    if (num_chars != 0) {
        const ns = obj_ensure_string(num_chars);
        if (ns.len > 0) {
            const parsed = obj.try_parse_int(ns.ptr, ns.len) orelse {
                // ``read $chan abc`` — Tcl reports
                // ``expected integer but got "abc"``.  We surface a
                // shorter message here; the upstream-wording sweep
                // (#272) folds it into the canonical phrasing.
                stubs.raise("read: expected integer but got non-integer numChars");
                return 0;
            };
            if (parsed < 0) {
                stubs.raise("read: numChars must be non-negative");
                return 0;
            }
            want = parsed;
        }
    }
    var buf = buf_init(if (want > 0 and want < 4096) @intCast(want) else 256);
    var prev_cr = false;
    while (want != 0) {
        if (c.buf_pos >= c.buf_end) {
            if (refill(c) == 0) break;
        }
        const src: [*]const u8 = @ptrFromInt(c.buf_addr);
        while (c.buf_pos < c.buf_end and want != 0) {
            const r = translate_input(c, src[c.buf_pos], &prev_cr);
            c.buf_pos += 1;
            if (r.emit) {
                buf_push(&buf, r.value);
                if (want > 0) want -= 1;
            }
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
        stubs.raise("gets: unknown channel");
        return 0;
    };
    const c = &channels[slot];
    if ((c.mode & MODE_READ) == 0) {
        stubs.raise("gets: channel not open for reading");
        return 0;
    }
    var line = buf_init(128);
    var saw_newline = false;
    var any_byte = false;
    var prev_cr = false;
    outer: while (true) {
        if (c.buf_pos >= c.buf_end) {
            if (refill(c) == 0) break;
        }
        const src: [*]const u8 = @ptrFromInt(c.buf_addr);
        while (c.buf_pos < c.buf_end) {
            const r = translate_input(c, src[c.buf_pos], &prev_cr);
            c.buf_pos += 1;
            if (!r.emit) continue;
            any_byte = true;
            if (r.value == '\n') {
                saw_newline = true;
                break :outer;
            }
            buf_push(&line, r.value);
        }
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
        stubs.raise("seek: unknown channel");
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
        stubs.raise("seek: write failed");
        return 0;
    }
    if (do_seek(c.fd, off_val, whence) == null) {
        stubs.raise("seek: fd_seek failed");
        return 0;
    }
    c.buf_pos = 0;
    c.buf_end = 0;
    c.eof = false;
    return obj_new_string(0, 0);
}

/// ``tell channelId`` — current byte offset, or -1 if unseekable.
/// Subtracts any pre-read bytes still sitting in the pull buffer.
pub export fn tcl_cmd_tell(chan: i32) i32 {
    const slot = resolve(chan) orelse {
        stubs.raise("tell: unknown channel");
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
        stubs.raise("eof: unknown channel");
        return 0;
    };
    return obj_new_int(if (channels[slot].eof) 1 else 0);
}

/// ``fblocked channelId`` — under WASI's synchronous I/O model
/// reads either complete or hit EOF; "would-block" is never
/// observed.  Always returns 0.
pub export fn tcl_cmd_fblocked(chan: i32) i32 {
    const slot = resolve(chan) orelse {
        stubs.raise("fblocked: unknown channel");
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
        stubs.raise("fcopy: unknown input channel");
        return 0;
    };
    const out_slot = resolve(out_chan) orelse {
        stubs.raise("fcopy: unknown output channel");
        return 0;
    };
    const in_c = &channels[in_slot];
    const out_c = &channels[out_slot];
    if ((in_c.mode & MODE_READ) == 0) {
        stubs.raise("fcopy: input channel not open for reading");
        return 0;
    }
    if ((out_c.mode & MODE_WRITE) == 0) {
        stubs.raise("fcopy: output channel not open for writing");
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
        stubs.raise("fcopy: write failed");
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
                    stubs.raise("fcopy: write failed");
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
            stubs.raise("fcopy: read failed");
            return 0;
        }
        var off: usize = 0;
        var rem: usize = @intCast(n);
        while (rem > 0) {
            const w = write_fn(out_c.fd, @ptrCast(&tmp[off]), rem);
            if (w <= 0) {
                stubs.raise("fcopy: write failed");
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

// Output translation — append ``len`` bytes starting at ``ptr`` to
// the channel's write buffer.  TR_CRLF expands LF → CRLF; TR_CR
// rewrites LF → CR; everything else is byte-for-byte.  When the
// channel is unbuffered (``BUF_NONE``) the call ends with a flush so
// each ``puts`` lands on the fd immediately.  Returns false on a
// write failure so the caller can raise.
fn write_translated(c: *Channel, ptr: [*]const u8, len: u32) bool {
    var i: u32 = 0;
    while (i < len) : (i += 1) {
        const b = ptr[i];
        if (c.translation == TR_CRLF and b == '\n') {
            if (!emit_byte(c, '\r')) return false;
            if (!emit_byte(c, '\n')) return false;
        } else if (c.translation == TR_CR and b == '\n') {
            if (!emit_byte(c, '\r')) return false;
        } else {
            if (!emit_byte(c, b)) return false;
        }
    }
    if (c.buffering == BUF_NONE) {
        return flush_chan(c);
    }
    return true;
}

/// Resolve a channel id and flush its write buffer.  Used by
/// ``tcl_cmd_flush`` (the runtime entry point) and indirectly by
/// the ``flush`` eval shim.  Returns the empty string on success
/// or 0 with a raised error on failure.
pub fn flush_chan_id(chan_id: i32) i32 {
    if (chan_id == 0) return obj_new_string(0, 0);
    const slot = resolve(chan_id) orelse {
        stubs.raise("flush: unknown channel");
        return 0;
    };
    if (!flush_chan(&channels[slot])) {
        stubs.raise("flush: write failed");
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
        stubs.raise("puts: unknown channel");
        return 0;
    };
    const c = &channels[slot];
    if ((c.mode & MODE_WRITE) == 0) {
        stubs.raise("puts: channel not open for writing");
        return 0;
    }
    if (msg != 0) {
        const s = obj_ensure_string(msg);
        if (s.len > 0) {
            if (!write_translated(c, @ptrFromInt(s.ptr), s.len)) {
                stubs.raise("puts: write failed");
                return 0;
            }
        }
    }
    if (nonewline == 0) {
        const nl: [1]u8 = .{'\n'};
        if (!write_translated(c, &nl, 1)) {
            stubs.raise("puts: write failed");
            return 0;
        }
    }
    return obj_new_string(0, 0);
}

// -- fconfigure --
//
// Applies translation / buffering / encoding settings to a real
// channel.  Unknown options trap via :func:`stubs.unsupported` so
// scripts that rely on (e.g.) ``-handshake`` get a clear error
// instead of a silent no-op.
//
// Supported options:
//   -translation auto|lf|cr|crlf|binary
//   -buffering   full|line|none
//   -encoding    utf-8|identity|binary
//   -blocking    0|1            (always accepted; WASI is sync)
//   -buffersize  N              (accepted, no effect)
//   -eofchar     ...            (accepted, no effect)
//   -profile     tcl8|...       (accepted, no effect)

fn is_accepted_option(p: [*]const u8, len: u32) bool {
    return eq(p, len, "-blocking") or
        eq(p, len, "-buffering") or
        eq(p, len, "-buffersize") or
        eq(p, len, "-encoding") or
        eq(p, len, "-eofchar") or
        eq(p, len, "-profile") or
        eq(p, len, "-translation");
}

fn apply_option(c: *Channel, name_p: [*]const u8, name_len: u32, val_p: [*]const u8, val_len: u32) void {
    if (eq(name_p, name_len, "-translation")) {
        if (eq(val_p, val_len, "auto")) c.translation = TR_AUTO;
        if (eq(val_p, val_len, "lf")) c.translation = TR_LF;
        if (eq(val_p, val_len, "cr")) c.translation = TR_CR;
        if (eq(val_p, val_len, "crlf")) c.translation = TR_CRLF;
        if (eq(val_p, val_len, "binary")) c.translation = TR_BINARY;
        return;
    }
    if (eq(name_p, name_len, "-buffering")) {
        if (eq(val_p, val_len, "full")) c.buffering = BUF_FULL;
        if (eq(val_p, val_len, "line")) c.buffering = BUF_LINE;
        if (eq(val_p, val_len, "none")) c.buffering = BUF_NONE;
        return;
    }
    if (eq(name_p, name_len, "-encoding")) {
        c.encoding_binary = eq(val_p, val_len, "binary");
        return;
    }
    if (eq(name_p, name_len, "-buffersize")) {
        const n = parse_uint(val_p, val_len) orelse return;
        // Tcl clamps -buffersize to ``[1, 1_000_000]``; mirror that
        // so a hostile script can't request a 4 GiB allocation.
        if (n == 0) return;
        const clamped: u32 = if (n > 1_000_000) 1_000_000 else n;
        if (clamped == c.out_buf_size) return;
        // Drain any pending bytes at the old size before swapping
        // the buffer; otherwise a partial write straddles the
        // resize and the second flush would point at freed memory.
        _ = flush_chan(c);
        if (c.out_buf_addr != 0) {
            obj.free_sized(c.out_buf_addr, c.out_buf_size);
            c.out_buf_addr = 0;
        }
        c.out_buf_size = clamped;
        c.out_buf_pos = 0;
        return;
    }
}

/// ``fconfigure $fd ?option value …?`` — channel-option setter.
/// The args list is a flat space-separated stream of ``-opt val``
/// pairs assembled by :file:`cmds/chan.zig:eval_fconfigure`.  Each
/// option name is validated against the allowlist; unknown options
/// trap with ``unsupported command: fconfigure`` so the caller can
/// either drop the option or extend this allowlist.
pub export fn tcl_cmd_fconfigure(fd: i32, args: i32) i32 {
    const slot = resolve(fd) orelse {
        // Match the other channel commands' behaviour: an unknown
        // channel id is a hard error, not a silent no-op.  Without
        // this guard, ``fconfigure $bogusFd ...`` returned empty
        // and any per-channel state mutation in the option list
        // was discarded — masking caller bugs and breaking the
        // script's expectation that unknown ids surface here.
        stubs.raise("fconfigure: unknown channel");
        return 0;
    };
    const c: *Channel = &channels[slot];
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
    var pos: u32 = 0;
    var name_start: u32 = 0;
    var name_len: u32 = 0;
    var parity: u32 = 0;
    while (pos < a.len) {
        while (pos < a.len and (ap[pos] == ' ' or ap[pos] == '\t' or ap[pos] == '\n')) : (pos += 1) {}
        if (pos >= a.len) break;
        const start = pos;
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
        const word_len = end - start;
        if (parity == 0) {
            if (word_len < 2 or ap[start] != '-') {
                stubs.unsupported("fconfigure (expected -option)");
                return 0;
            }
            if (!is_accepted_option(ap + start, word_len)) {
                stubs.unsupported("fconfigure (unsupported option)");
                return 0;
            }
            name_start = start;
            name_len = word_len;
        } else {
            apply_option(c, ap + name_start, name_len, ap + start, word_len);
        }
        parity = 1 - parity;
    }
    if (parity == 1) {
        stubs.unsupported("fconfigure (odd number of option/value args)");
        return 0;
    }
    return obj_new_string(0, 0);
}
