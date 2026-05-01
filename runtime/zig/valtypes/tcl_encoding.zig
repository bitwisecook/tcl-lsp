// Real encoding subsystem for the WASM runtime.
//
// Tcl's ``encoding`` command selects between character-set codecs
// (identity, utf-8, iso8859-*, cp1252, utf-16, shiftjis, …) and
// converts between byte sequences.  Earlier this module only handled
// ``identity`` / ``utf-8`` (both pass-through); anything else trapped.
// That gap surfaces in ``fconfigure $fd -encoding ...`` too — the
// option was on the allowlist but the runtime never transformed the
// bytes, so e.g. ``puts -nonewline $f "café"`` with
// ``-encoding iso8859-1`` wrote raw utf-8 instead of latin-1.
//
// This module now implements:
//
//   * ``identity`` / ``utf-8``      — pass-through.
//   * ``binary``                    — pass-through alias on
//                                     ``fconfigure -encoding`` and on
//                                     the ``encoding`` command.  Tcl's
//                                     ``encoding names`` does not list
//                                     ``binary`` (it isn't a real codec
//                                     — it's the no-translation /
//                                     identity-bytes mode), and we
//                                     match that.
//   * ``ascii``                     — strict 7-bit.  Codepoints > 0x7F
//                                     are emitted as the default char
//                                     ``?`` on the write side; bytes
//                                     > 0x7F surfaced as U+FFFD on
//                                     the read side.
//   * ``iso8859-1``                 — 1:1 codepoint↔byte for U+0000–
//                                     U+00FF.
//   * ``cp1252``                    — superset of latin-1 with the
//                                     0x80–0x9F window populated from
//                                     ``library/encoding/cp1252.enc``
//                                     in tcl 9.0.3.
//   * ``utf-16le`` / ``utf-16be``   — 16-bit codepoint round-trips.
//                                     Surrogate pairs handled on both
//                                     sides; lone surrogates emit U+FFFD
//                                     on read and trap on write.
//   * ``utf-16``                    — alias for utf-16le on this build
//                                     (no BOM emit/sniff — out of scope
//                                     per the parent issue).
//
// Multi-byte codecs (shiftjis, eucjp, gb2312, big5, …) and BOM-sniffing
// remain unsupported and trap with a clear ``unsupported encoding``
// message; they will land incrementally once the JIS / GB / Big5 tables
// are vendored.
//
// The codec layer is shared between the ``encoding convertfrom /
// convertto`` commands (this file) and channel I/O (``tcl_chan.zig``)
// so the on-disk bytes and the encoded-bytes returned by the encoding
// command stay in lock-step.

const obj = @import("tcl_obj.zig");
const stubs = @import("../stubs/tcl_stubs.zig");

const obj_ensure_string = obj.obj_ensure_string;
const obj_new_string = obj.obj_new_string;
const obj_new_string_copy = obj.obj_new_string_copy;

// -- Public encoding identity ------------------------------------

/// Tag for one of the codecs handled by this module.  ``pass`` is the
/// utf-8 / identity / binary group — no transformation in either
/// direction.  Channel state stores this tag so reads/writes share
/// the same dispatch table as the encoding command.
pub const Enc = enum(u8) {
    pass = 0,
    iso8859_1,
    ascii,
    cp1252,
    utf16le,
    utf16be,
    utf16, // alias for utf16le on this build
    koi8_r,
};

fn eq(a: [*]const u8, alen: u32, literal: []const u8) bool {
    if (alen != literal.len) return false;
    for (0..literal.len) |i| if (a[i] != literal[i]) return false;
    return true;
}

/// Resolve a Tcl encoding name to an ``Enc`` tag, or null if the name
/// is not a recognised Tcl codec at all (caller may then choose to
/// treat it as a payload — Tcl 8.4's two-arg ``encoding convertfrom``
/// shape).  ``known_unsupported`` is set when the name *is* a known
/// Tcl encoding the runtime doesn't yet implement (shiftjis, big5, …)
/// so the caller can raise a precise ``unsupported encoding`` error.
pub const Resolved = struct {
    enc: ?Enc,
    known_unsupported: bool,
};

pub fn resolve_name(p: [*]const u8, len: u32) Resolved {
    if (len == 0) return .{ .enc = .pass, .known_unsupported = false };
    if (eq(p, len, "identity") or eq(p, len, "utf-8") or eq(p, len, "binary")) {
        return .{ .enc = .pass, .known_unsupported = false };
    }
    if (eq(p, len, "ascii")) return .{ .enc = .ascii, .known_unsupported = false };
    if (eq(p, len, "iso8859-1")) return .{ .enc = .iso8859_1, .known_unsupported = false };
    if (eq(p, len, "cp1252")) return .{ .enc = .cp1252, .known_unsupported = false };
    if (eq(p, len, "utf-16")) return .{ .enc = .utf16, .known_unsupported = false };
    if (eq(p, len, "utf-16le")) return .{ .enc = .utf16le, .known_unsupported = false };
    if (eq(p, len, "utf-16be")) return .{ .enc = .utf16be, .known_unsupported = false };
    if (eq(p, len, "koi8-r")) return .{ .enc = .koi8_r, .known_unsupported = false };

    // Tcl-core codecs we don't model yet.  Distinguishing these from
    // an accidental typo means we can trap with a precise
    // "unsupported encoding" message instead of treating the name as
    // a payload.
    if (eq(p, len, "iso8859-2") or eq(p, len, "iso8859-15") or
        eq(p, len, "cp1250") or eq(p, len, "cp1251") or
        eq(p, len, "cp437") or eq(p, len, "cp850") or
        eq(p, len, "unicode") or eq(p, len, "ucs-2le") or
        eq(p, len, "ucs-2be") or eq(p, len, "ucs-4le") or
        eq(p, len, "ucs-4be") or eq(p, len, "utf-32") or
        eq(p, len, "macroman") or eq(p, len, "macjapan") or
        eq(p, len, "shiftjis") or eq(p, len, "jis0208") or
        eq(p, len, "jis0212") or eq(p, len, "euc-jp") or
        eq(p, len, "euc-kr") or eq(p, len, "euc-cn") or
        eq(p, len, "gb2312") or eq(p, len, "gb12345") or
        eq(p, len, "gb1988") or eq(p, len, "big5") or
        eq(p, len, "koi8-u") or
        eq(p, len, "tis-620"))
    {
        return .{ .enc = null, .known_unsupported = true };
    }

    return .{ .enc = null, .known_unsupported = false };
}

// -- cp1252 lookup tables ----------------------------------------
//
// 0x80–0x9F is the Windows extension window: characters that would
// otherwise be C1 controls in iso8859-1.  Outside this range cp1252
// agrees with iso8859-1 byte-for-byte, so we only table the window.
// Source: ``tmp/tcl9.0.3/library/encoding/cp1252.enc``.

const cp1252_byte_to_cp = [_]u16{
    0x20AC, 0x0000, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021,
    0x02C6, 0x2030, 0x0160, 0x2039, 0x0152, 0x0000, 0x017D, 0x0000,
    0x0000, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014,
    0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0x0000, 0x017E, 0x0178,
};

fn cp1252_encode_extra(cp: u32) ?u8 {
    // Reverse of cp1252_byte_to_cp for the 0x80–0x9F window.  Linear
    // scan — table is 32 entries so this beats a hash in code size
    // and runs in microbenchmark territory anyway.
    var i: u8 = 0;
    while (i < 32) : (i += 1) {
        const v = cp1252_byte_to_cp[i];
        if (v != 0 and @as(u32, v) == cp) return 0x80 + i;
    }
    return null;
}

// -- koi8-r lookup table ----------------------------------------
//
// Single-byte cyrillic encoding.  Bytes 0x00–0x7F are ASCII; 0x80–0xFF
// map to the codepoints in :data:`koi8r_byte_to_cp` (offset 0x80).
// Source: ``tmp/tcl9.0.3/library/encoding/koi8-r.enc``.

const koi8r_byte_to_cp = [_]u16{
    0x2500, 0x2502, 0x250C, 0x2510, 0x2514, 0x2518, 0x251C, 0x2524,
    0x252C, 0x2534, 0x253C, 0x2580, 0x2584, 0x2588, 0x258C, 0x2590,
    0x2591, 0x2592, 0x2593, 0x2320, 0x25A0, 0x2219, 0x221A, 0x2248,
    0x2264, 0x2265, 0x00A0, 0x2321, 0x00B0, 0x00B2, 0x00B7, 0x00F7,
    0x2550, 0x2551, 0x2552, 0x0451, 0x2553, 0x2554, 0x2555, 0x2556,
    0x2557, 0x2558, 0x2559, 0x255A, 0x255B, 0x255C, 0x255D, 0x255E,
    0x255F, 0x2560, 0x2561, 0x0401, 0x2562, 0x2563, 0x2564, 0x2565,
    0x2566, 0x2567, 0x2568, 0x2569, 0x256A, 0x256B, 0x256C, 0x00A9,
    0x044E, 0x0430, 0x0431, 0x0446, 0x0434, 0x0435, 0x0444, 0x0433,
    0x0445, 0x0438, 0x0439, 0x043A, 0x043B, 0x043C, 0x043D, 0x043E,
    0x043F, 0x044F, 0x0440, 0x0441, 0x0442, 0x0443, 0x0436, 0x0432,
    0x044C, 0x044B, 0x0437, 0x0448, 0x044D, 0x0449, 0x0447, 0x044A,
    0x042E, 0x0410, 0x0411, 0x0426, 0x0414, 0x0415, 0x0424, 0x0413,
    0x0425, 0x0418, 0x0419, 0x041A, 0x041B, 0x041C, 0x041D, 0x041E,
    0x041F, 0x042F, 0x0420, 0x0421, 0x0422, 0x0423, 0x0416, 0x0412,
    0x042C, 0x042B, 0x0417, 0x0428, 0x042D, 0x0429, 0x0427, 0x042A,
};

fn koi8r_encode_extra(cp: u32) ?u8 {
    // Reverse lookup over the 128-entry table.  Linear scan — koi8-r
    // is rarely exercised on the write side, so the table size /
    // simplicity wins out over a hash.
    var i: u8 = 0;
    while (i < 128) : (i += 1) {
        if (@as(u32, koi8r_byte_to_cp[i]) == cp) return 0x80 + i;
    }
    return null;
}

// -- ByteBuf -----------------------------------------------------
//
// Reusable growable byte buffer.  We allocate via ``obj.alloc`` so
// the eventual ``obj_new_string`` can claim ownership of the buffer
// (write OBJ_STR_CAP) and the recycler reclaims it via
// ``free_sized`` at end-of-life.

pub const ByteBuf = struct {
    addr: u32,
    cap: u32,
    len: u32,
};

pub fn buf_init(initial: u32) ByteBuf {
    const cap = if (initial < 64) 64 else initial;
    return .{
        .addr = obj.alloc(cap),
        .cap = cap,
        .len = 0,
    };
}

pub fn buf_reserve(b: *ByteBuf, want: u32) void {
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

pub fn buf_push(b: *ByteBuf, byte: u8) void {
    buf_reserve(b, 1);
    const dst: [*]u8 = @ptrFromInt(b.addr);
    dst[b.len] = byte;
    b.len += 1;
}

pub fn buf_push2(b: *ByteBuf, b0: u8, b1: u8) void {
    buf_reserve(b, 2);
    const dst: [*]u8 = @ptrFromInt(b.addr);
    dst[b.len] = b0;
    dst[b.len + 1] = b1;
    b.len += 2;
}

pub fn buf_release(b: ByteBuf) void {
    obj.free_sized(b.addr, b.cap);
}

/// Wrap a finished ByteBuf in a TclObj that owns the buffer.
pub fn buf_to_obj(b: ByteBuf) i32 {
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

// -- UTF-8 iteration --------------------------------------------
//
// Decode one codepoint from utf-8; on a malformed sequence emit
// U+FFFD and advance one byte.  The encoders below feed the
// codepoint into a per-codec mapper that may further transform it
// or reject (e.g. iso8859-1 codepoints > 0xFF).

const Decode = struct {
    cp: u32,
    consumed: u32,
};

fn utf8_decode(p: [*]const u8, len: u32, pos: u32) Decode {
    if (pos >= len) return .{ .cp = 0xFFFD, .consumed = 0 };
    const b0 = p[pos];
    if (b0 < 0x80) return .{ .cp = b0, .consumed = 1 };
    if (b0 < 0xC2) return .{ .cp = 0xFFFD, .consumed = 1 };
    if (b0 < 0xE0) {
        if (pos + 1 >= len) return .{ .cp = 0xFFFD, .consumed = 1 };
        const b1 = p[pos + 1];
        if ((b1 & 0xC0) != 0x80) return .{ .cp = 0xFFFD, .consumed = 1 };
        return .{
            .cp = (@as(u32, b0 & 0x1F) << 6) | (b1 & 0x3F),
            .consumed = 2,
        };
    }
    if (b0 < 0xF0) {
        if (pos + 2 >= len) return .{ .cp = 0xFFFD, .consumed = 1 };
        const b1 = p[pos + 1];
        const b2 = p[pos + 2];
        if ((b1 & 0xC0) != 0x80 or (b2 & 0xC0) != 0x80) {
            return .{ .cp = 0xFFFD, .consumed = 1 };
        }
        return .{
            .cp = (@as(u32, b0 & 0x0F) << 12) |
                (@as(u32, b1 & 0x3F) << 6) |
                (b2 & 0x3F),
            .consumed = 3,
        };
    }
    if (b0 < 0xF8) {
        if (pos + 3 >= len) return .{ .cp = 0xFFFD, .consumed = 1 };
        const b1 = p[pos + 1];
        const b2 = p[pos + 2];
        const b3 = p[pos + 3];
        if ((b1 & 0xC0) != 0x80 or (b2 & 0xC0) != 0x80 or (b3 & 0xC0) != 0x80) {
            return .{ .cp = 0xFFFD, .consumed = 1 };
        }
        return .{
            .cp = (@as(u32, b0 & 0x07) << 18) |
                (@as(u32, b1 & 0x3F) << 12) |
                (@as(u32, b2 & 0x3F) << 6) |
                (b3 & 0x3F),
            .consumed = 4,
        };
    }
    return .{ .cp = 0xFFFD, .consumed = 1 };
}

fn utf8_emit(b: *ByteBuf, cp: u32) void {
    if (cp < 0x80) {
        buf_push(b, @intCast(cp));
        return;
    }
    if (cp < 0x800) {
        buf_push2(b, @intCast(0xC0 | (cp >> 6)), @intCast(0x80 | (cp & 0x3F)));
        return;
    }
    if (cp < 0x10000) {
        buf_reserve(b, 3);
        const dst: [*]u8 = @ptrFromInt(b.addr);
        dst[b.len] = @intCast(0xE0 | (cp >> 12));
        dst[b.len + 1] = @intCast(0x80 | ((cp >> 6) & 0x3F));
        dst[b.len + 2] = @intCast(0x80 | (cp & 0x3F));
        b.len += 3;
        return;
    }
    buf_reserve(b, 4);
    const dst: [*]u8 = @ptrFromInt(b.addr);
    dst[b.len] = @intCast(0xF0 | (cp >> 18));
    dst[b.len + 1] = @intCast(0x80 | ((cp >> 12) & 0x3F));
    dst[b.len + 2] = @intCast(0x80 | ((cp >> 6) & 0x3F));
    dst[b.len + 3] = @intCast(0x80 | (cp & 0x3F));
    b.len += 4;
}

// -- Streaming decode -------------------------------------------
//
// Channel I/O reads bytes from a fixed-size raw pull buffer and a
// surrogate pair / latin-1 codepoint may cross a chunk boundary if
// :func:`read_fn` returns mid-unit.  ``decode_step`` deliberately
// only consumes one codepoint at a time so the channel layer can:
//
//   * advance ``buf_pos`` exactly by the raw bytes belonging to that
//     codepoint (so ``tell`` / ``fcopy`` stay aligned with the OS
//     fd position), and
//   * leave any incomplete trailing bytes in place (``needs_more =
//     true``, ``consumed = 0``) for compaction + re-read on the next
//     refill.

pub const StepResult = struct {
    /// Raw bytes consumed by this step.  Zero with ``needs_more =
    /// true`` means the caller must read more raw data and retry.
    consumed: u32,
    /// Decoded codepoint (only valid when ``needs_more = false``).
    /// Malformed units (lone surrogate, ascii > 0x7F …) decode to
    /// U+FFFD with ``consumed`` set to the byte count of the
    /// offending unit.
    cp: u32,
    /// Caller must refill the source before another step succeeds.
    needs_more: bool,
};

/// Decode one codepoint of *enc*.  ``Enc.pass`` is invalid here —
/// callers short-circuit pass-through before calling.
pub fn decode_step(enc: Enc, src: u32, len: u32) StepResult {
    if (len == 0) return .{ .consumed = 0, .cp = 0, .needs_more = true };
    const p: [*]const u8 = @ptrFromInt(src);
    switch (enc) {
        .pass => unreachable,
        .ascii => {
            const c = p[0];
            return .{ .consumed = 1, .cp = if (c > 0x7F) 0xFFFD else c, .needs_more = false };
        },
        .iso8859_1 => return .{ .consumed = 1, .cp = p[0], .needs_more = false },
        .cp1252 => {
            const c = p[0];
            if (c < 0x80 or c >= 0xA0) {
                return .{ .consumed = 1, .cp = c, .needs_more = false };
            }
            const m = cp1252_byte_to_cp[c - 0x80];
            return .{ .consumed = 1, .cp = if (m == 0) 0xFFFD else @as(u32, m), .needs_more = false };
        },
        .koi8_r => {
            const c = p[0];
            if (c < 0x80) {
                return .{ .consumed = 1, .cp = c, .needs_more = false };
            }
            return .{ .consumed = 1, .cp = @as(u32, koi8r_byte_to_cp[c - 0x80]), .needs_more = false };
        },
        .utf16le, .utf16, .utf16be => {
            if (len < 2) return .{ .consumed = 0, .cp = 0, .needs_more = true };
            const be = (enc == .utf16be);
            const lo = p[if (be) 1 else 0];
            const hi = p[if (be) 0 else 1];
            const unit: u32 = (@as(u32, hi) << 8) | lo;
            if (unit >= 0xD800 and unit <= 0xDBFF) {
                if (len < 4) return .{ .consumed = 0, .cp = 0, .needs_more = true };
                const lo2 = p[if (be) 3 else 2];
                const hi2 = p[if (be) 2 else 3];
                const unit2: u32 = (@as(u32, hi2) << 8) | lo2;
                if (unit2 < 0xDC00 or unit2 > 0xDFFF) {
                    return .{ .consumed = 2, .cp = 0xFFFD, .needs_more = false };
                }
                const cp: u32 = 0x10000 + ((unit - 0xD800) << 10) + (unit2 - 0xDC00);
                return .{ .consumed = 4, .cp = cp, .needs_more = false };
            }
            if (unit >= 0xDC00 and unit <= 0xDFFF) {
                return .{ .consumed = 2, .cp = 0xFFFD, .needs_more = false };
            }
            return .{ .consumed = 2, .cp = unit, .needs_more = false };
        },
    }
}

/// Encode *cp* as utf-8 into ``dst``; returns the byte count (1-4).
/// Used by the channel layer's per-codepoint decoder so the dec
/// buffer holds exactly one codepoint's expansion at a time.
pub fn utf8_encode(dst: u32, cp: u32) u32 {
    const out: [*]u8 = @ptrFromInt(dst);
    if (cp < 0x80) {
        out[0] = @intCast(cp);
        return 1;
    }
    if (cp < 0x800) {
        out[0] = @intCast(0xC0 | (cp >> 6));
        out[1] = @intCast(0x80 | (cp & 0x3F));
        return 2;
    }
    if (cp < 0x10000) {
        out[0] = @intCast(0xE0 | (cp >> 12));
        out[1] = @intCast(0x80 | ((cp >> 6) & 0x3F));
        out[2] = @intCast(0x80 | (cp & 0x3F));
        return 3;
    }
    out[0] = @intCast(0xF0 | (cp >> 18));
    out[1] = @intCast(0x80 | ((cp >> 12) & 0x3F));
    out[2] = @intCast(0x80 | ((cp >> 6) & 0x3F));
    out[3] = @intCast(0x80 | (cp & 0x3F));
    return 4;
}

// -- Codec dispatch ---------------------------------------------

/// Encode a utf-8 string into the byte form of *enc*.  ``src`` /
/// ``len`` describe a utf-8 byte slice; the returned ByteBuf owns a
/// freshly allocated buffer the caller must either wrap with
/// :func:`buf_to_obj` or release with :func:`buf_release`.  Returns
/// null on a hard codec error (codepoint not representable, etc.) —
/// the caller raises with an appropriate message.
pub fn encode(enc: Enc, src: u32, len: u32) ?ByteBuf {
    if (enc == .pass) {
        var b = buf_init(if (len > 0) len else 1);
        if (len > 0) {
            const sp: [*]const u8 = @ptrFromInt(src);
            buf_reserve(&b, len);
            const dst: [*]u8 = @ptrFromInt(b.addr);
            for (0..len) |i| dst[i] = sp[i];
            b.len = len;
        }
        return b;
    }
    const sp: [*]const u8 = @ptrFromInt(src);
    var out = buf_init(if (len > 0) len else 1);
    var pos: u32 = 0;
    while (pos < len) {
        const d = utf8_decode(sp, len, pos);
        pos += d.consumed;
        switch (enc) {
            .pass => unreachable,
            .ascii => {
                if (d.cp > 0x7F) {
                    buf_push(&out, '?');
                } else {
                    buf_push(&out, @intCast(d.cp));
                }
            },
            .iso8859_1 => {
                if (d.cp > 0xFF) {
                    buf_release(out);
                    stubs.raise("encoding iso8859-1: codepoint not representable");
                    return null;
                }
                buf_push(&out, @intCast(d.cp));
            },
            .cp1252 => {
                if (d.cp <= 0x7F or (d.cp >= 0xA0 and d.cp <= 0xFF)) {
                    buf_push(&out, @intCast(d.cp));
                } else if (cp1252_encode_extra(d.cp)) |b| {
                    buf_push(&out, b);
                } else {
                    buf_release(out);
                    stubs.raise("encoding cp1252: codepoint not representable");
                    return null;
                }
            },
            .koi8_r => {
                if (d.cp <= 0x7F) {
                    buf_push(&out, @intCast(d.cp));
                } else if (koi8r_encode_extra(d.cp)) |b| {
                    buf_push(&out, b);
                } else {
                    buf_release(out);
                    stubs.raise("encoding koi8-r: codepoint not representable");
                    return null;
                }
            },
            .utf16le, .utf16, .utf16be => {
                const be = (enc == .utf16be);
                if (d.cp < 0x10000) {
                    if (d.cp >= 0xD800 and d.cp <= 0xDFFF) {
                        buf_release(out);
                        stubs.raise("encoding utf-16: lone surrogate");
                        return null;
                    }
                    const lo: u8 = @intCast(d.cp & 0xFF);
                    const hi: u8 = @intCast((d.cp >> 8) & 0xFF);
                    if (be) buf_push2(&out, hi, lo) else buf_push2(&out, lo, hi);
                } else if (d.cp <= 0x10FFFF) {
                    const v = d.cp - 0x10000;
                    const high: u32 = 0xD800 | (v >> 10);
                    const low: u32 = 0xDC00 | (v & 0x3FF);
                    const h_lo: u8 = @intCast(high & 0xFF);
                    const h_hi: u8 = @intCast((high >> 8) & 0xFF);
                    const l_lo: u8 = @intCast(low & 0xFF);
                    const l_hi: u8 = @intCast((low >> 8) & 0xFF);
                    if (be) {
                        buf_push2(&out, h_hi, h_lo);
                        buf_push2(&out, l_hi, l_lo);
                    } else {
                        buf_push2(&out, h_lo, h_hi);
                        buf_push2(&out, l_lo, l_hi);
                    }
                } else {
                    buf_release(out);
                    stubs.raise("encoding utf-16: codepoint out of range");
                    return null;
                }
            },
        }
    }
    return out;
}

/// Decode encoded bytes into a utf-8 string.  Mirror of :func:`encode`.
/// Malformed input (truncated utf-16 unit, byte > 0x7F under ascii,
/// …) is rendered as U+FFFD on the read side rather than trapping —
/// this matches Tcl 9's default ``-profile tcl8`` semantics.
pub fn decode(enc: Enc, src: u32, len: u32) ?ByteBuf {
    if (enc == .pass) {
        var b = buf_init(if (len > 0) len else 1);
        if (len > 0) {
            const sp: [*]const u8 = @ptrFromInt(src);
            buf_reserve(&b, len);
            const dst: [*]u8 = @ptrFromInt(b.addr);
            for (0..len) |i| dst[i] = sp[i];
            b.len = len;
        }
        return b;
    }
    const sp: [*]const u8 = @ptrFromInt(src);
    var out = buf_init(if (len > 0) len else 1);
    switch (enc) {
        .pass => unreachable,
        .ascii => {
            for (0..len) |i| {
                const c = sp[i];
                utf8_emit(&out, if (c > 0x7F) 0xFFFD else c);
            }
        },
        .iso8859_1 => {
            for (0..len) |i| utf8_emit(&out, sp[i]);
        },
        .koi8_r => {
            for (0..len) |i| {
                const c = sp[i];
                if (c < 0x80) {
                    utf8_emit(&out, c);
                } else {
                    utf8_emit(&out, koi8r_byte_to_cp[c - 0x80]);
                }
            }
        },
        .cp1252 => {
            for (0..len) |i| {
                const c = sp[i];
                if (c < 0x80 or c >= 0xA0) {
                    utf8_emit(&out, c);
                } else {
                    const m = cp1252_byte_to_cp[c - 0x80];
                    utf8_emit(&out, if (m == 0) 0xFFFD else m);
                }
            }
        },
        .utf16le, .utf16, .utf16be => {
            const be = (enc == .utf16be);
            var i: u32 = 0;
            while (i + 1 < len) {
                const lo = sp[if (be) i + 1 else i];
                const hi = sp[if (be) i else i + 1];
                const unit: u32 = (@as(u32, hi) << 8) | lo;
                i += 2;
                if (unit >= 0xD800 and unit <= 0xDBFF) {
                    if (i + 1 >= len) {
                        utf8_emit(&out, 0xFFFD);
                        break;
                    }
                    const lo2 = sp[if (be) i + 1 else i];
                    const hi2 = sp[if (be) i else i + 1];
                    const unit2: u32 = (@as(u32, hi2) << 8) | lo2;
                    if (unit2 < 0xDC00 or unit2 > 0xDFFF) {
                        utf8_emit(&out, 0xFFFD);
                        continue;
                    }
                    i += 2;
                    const cp: u32 = 0x10000 + ((unit - 0xD800) << 10) + (unit2 - 0xDC00);
                    utf8_emit(&out, cp);
                } else if (unit >= 0xDC00 and unit <= 0xDFFF) {
                    utf8_emit(&out, 0xFFFD);
                } else {
                    utf8_emit(&out, unit);
                }
            }
        },
    }
    return out;
}

// -- ``encoding`` command dispatch -------------------------------

const NAMES_LIST: []const u8 = "ascii cp1252 identity iso8859-1 koi8-r utf-8 utf-16 utf-16be utf-16le";

/// Dispatch ``encoding <subcmd> ?arg1? ?arg2?``.  Codegen always
/// pushes three i32 slots; missing args are 0 (empty string).
pub export fn tcl_cmd_encoding(sub: i32, arg1: i32, arg2: i32) i32 {
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

    if (eq(sp, s.len, "system")) {
        return obj_new_string_copy(@intFromPtr("utf-8".ptr), 5);
    }
    if (eq(sp, s.len, "names")) {
        return obj_new_string_copy(@intFromPtr(NAMES_LIST.ptr), @intCast(NAMES_LIST.len));
    }
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
            const r = resolve_name(a1p, a1.len);
            if (r.known_unsupported) {
                stubs.unsupported("encoding (codec not implemented)");
                return 0;
            }
            const enc = r.enc orelse return arg2; // unknown name → pass-through (legacy 8.4 shape)
            if (enc == .pass) return arg2;
            const a2 = obj_ensure_string(arg2);
            const buf = if (is_to)
                encode(enc, a2.ptr, a2.len) orelse return 0
            else
                decode(enc, a2.ptr, a2.len) orelse return 0;
            return buf_to_obj(buf);
        }
        // Two-arg form: arg1 is the payload, default encoding = utf-8.
        if (arg1 == 0) return obj_new_string(0, 0);
        return arg1;
    }

    stubs.unsupported("encoding (unknown subcommand)");
    return 0;
}
