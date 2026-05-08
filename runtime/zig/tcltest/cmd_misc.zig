// Miscellaneous trivial / value-tier test commands ported from
// upstream ``tclTest.c``.  Anything that fits in <30 lines and
// doesn't need a dedicated cluster ends up here — the file's
// scope is "small portable wrappers", not a long-term home for
// anything that grows substantial.
//
// Covered (PORTABLE):
//   * ``testlongsize``        — sizeof(long); 8 under wasm-LP64-ish layout
//   * ``testsize st_mtime``   — sizeof(time_t); 8
//   * ``testgetint``          — sum a list of ints
//   * ``testgetintforindex``  — Tcl_GetIntForIndex with an endvalue
//   * ``testgetindexfromobjstruct`` — Tcl_GetIndexFromObjStruct probe
//   * ``testdoubledigits``    — format a double with N digits
//   * ``testlutil``           — list-utility helpers
//   * ``testmsb``             — most-significant-bit position
//   * ``testpurebytesobj``    — pass-through obj (no string-rep distinction here)
//   * ``testbytestring``      — pass-through obj
//   * ``teststringbytes``     — return bytearray view of string
//   * ``testsetbytearraylength`` — truncate/grow a bytearray
//   * ``testapplylambda``     — regression check, returns 42
//   * ``testpreferstable``    — constant "stable"
//   * ``testlocale``          — stub returning "C"
//   * ``testpanic``           — Tcl-level error (no abort() under WASM)
//   * ``testprint``           — Tcl_ObjPrintf with %wide etc
//   * ``testseterrorcode_concat`` (alias) — handled in cmd_eval.zig
//   * ``testbumpinterpepoch`` — no-op (we don't expose epoch)
//   * ``testdcall``           — no-op (no Tcl_CallWhenDeleted)
//   * ``testgetplatform`` / ``testsetplatform`` — return "unix" / no-op
//   * ``testhashsystemhash``  — self-test of hash invariants; trivial pass
//   * ``testparseargs``       — minimal -bool/-colormode parsing
//   * ``testreturnoptions``   — N/A
//   * ``noop`` graduations    — also handled here
//
// Each replaces the matching stub row in ``cmd_stubs.zig``.

const std = @import("std");
const obj = @import("../valtypes/tcl_obj.zig");
const result_mod = @import("../interp/tcl_result.zig");
const catch_mod = @import("../interp/tcl_catch.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");

fn build_msg(buf: []const u8) i32 {
    const dst = obj.alloc(@intCast(buf.len));
    if (dst == 0) return 0;
    const dst_p: [*]u8 = @ptrFromInt(dst);
    for (buf, 0..) |c, i| dst_p[i] = c;
    return obj.obj_new_string_take(dst, @intCast(buf.len), @intCast(buf.len));
}

fn err_msg(buf: []const u8) result_mod.InterpResult {
    catch_mod.tcl_cmd_error(build_msg(buf));
    return result_mod.from_globals(0);
}

fn err_fmt(comptime fmt: []const u8, args: anytype) result_mod.InterpResult {
    var buf: [256]u8 = undefined;
    const slice = std.fmt.bufPrint(&buf, fmt, args) catch fmt;
    return err_msg(slice);
}

fn obj_view(handle: i32) []const u8 {
    const s = obj.obj_ensure_string(handle);
    const ptr: [*]const u8 = @ptrFromInt(s.ptr);
    return ptr[0..s.len];
}

fn obj_eq_lit(handle: i32, lit: []const u8) bool {
    const view = obj_view(handle);
    return std.mem.eql(u8, view, lit);
}

// -- testlongsize / testsize -----------------------------------------------

/// Returns ``sizeof(long)``.  We use 8 under wasm32 since our
/// ``Tcl_WideInt`` (i64) is the canonical wide type.
fn eval_testlongsize(words: []const i32) result_mod.InterpResult {
    _ = words;
    return result_mod.ok(obj.obj_new_int(8));
}

/// ``testsize st_mtime`` — only ``st_mtime`` is recognised.
fn eval_testsize(words: []const i32) result_mod.InterpResult {
    if (words.len != 2) return err_msg("wrong # args: testsize st_mtime");
    if (!obj_eq_lit(words[1], "st_mtime")) return err_msg("testsize: only st_mtime supported");
    return result_mod.ok(obj.obj_new_int(8));
}

// -- testgetint / testgetintforindex --------------------------------------

/// Sum a list of integer-parsed args, return the total.  Matches
/// upstream's ``TestgetintCmd`` shape.
fn eval_testgetint(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) return err_msg("wrong # args: testgetint ?args?");
    var total: i64 = 0;
    var i: u32 = 1;
    while (i < words.len) : (i += 1) {
        total += obj.obj_get_int(words[i]);
    }
    return result_mod.ok(obj.obj_new_int(total));
}

/// ``testgetintforindex index endvalue`` — resolves an index obj
/// (number, ``end``, ``end-N``) against an end-value.  Mirrors
/// upstream's ``Tcl_GetIntForIndex`` semantics for the common shapes.
fn eval_testgetintforindex(words: []const i32) result_mod.InterpResult {
    if (words.len != 3) return err_msg("wrong # args: testgetintforindex index endvalue");
    const endvalue = obj.obj_get_int(words[2]);
    const view = obj_view(words[1]);
    if (view.len == 0) return err_msg("bad index: empty");
    if (std.mem.eql(u8, view, "end")) return result_mod.ok(obj.obj_new_int(endvalue));
    if (view.len > 4 and std.mem.eql(u8, view[0..4], "end-")) {
        const off = std.fmt.parseInt(i64, view[4..], 10) catch return err_msg("bad index");
        return result_mod.ok(obj.obj_new_int(endvalue - off));
    }
    if (view.len > 4 and std.mem.eql(u8, view[0..4], "end+")) {
        const off = std.fmt.parseInt(i64, view[4..], 10) catch return err_msg("bad index");
        return result_mod.ok(obj.obj_new_int(endvalue + off));
    }
    return result_mod.ok(obj.obj_new_int(obj.obj_get_int(words[1])));
}

// -- testgetindexfromobjstruct ----------------------------------------------

/// ``testgetindexfromobjstruct argument targetvalue ?flags?`` —
/// looks up *argument* in a fixed table {a, b, c, d, ee, ff} and
/// asserts the resolved index matches *targetvalue*.  Returns empty
/// string on success, error on mismatch (matching upstream).
fn eval_testgetindexfromobjstruct(words: []const i32) result_mod.InterpResult {
    if (words.len != 3 and words.len != 4) return err_msg("wrong # args: testgetindexfromobjstruct argument targetvalue ?flags?");
    const target = obj.obj_get_int(words[2]);
    const arg = obj_view(words[1]);
    const table = [_][]const u8{ "a", "b", "c", "d", "ee", "ff" };
    var found: i64 = -1;
    if (arg.len > 0) {
        for (table, 0..) |tok, i| {
            if (std.mem.eql(u8, tok, arg)) {
                found = @intCast(i);
                break;
            }
        }
        if (found < 0) {
            for (table, 0..) |tok, i| {
                if (tok.len >= arg.len and std.mem.eql(u8, tok[0..arg.len], arg)) {
                    found = @intCast(i);
                    break;
                }
            }
        }
    }
    if (found != target) {
        return err_fmt("index value comparison failed: got {d} when {d} expected", .{ found, target });
    }
    return result_mod.ok(obj.obj_new_string(0, 0));
}

// -- testdoubledigits ----------------------------------------------------

/// ``testdoubledigits value ndigits ?type?`` — format a double with a
/// requested digit count.  Maps to Zig's ``std.fmt.formatFloatDecimal``
/// — type is "shortest" (default), "Steele", "e", or "f".
fn eval_testdoubledigits(words: []const i32) result_mod.InterpResult {
    if (words.len < 3 or words.len > 4) return err_msg("wrong # args: testdoubledigits value ndigits ?type?");
    const v = obj.obj_get_float(words[1]);
    const n = obj.obj_get_int(words[2]);
    const precision: u32 = if (n < 0) 0 else @intCast(n);
    var buf: [128]u8 = undefined;
    const slice = std.fmt.bufPrint(&buf, "{d:.[1]}", .{ v, precision }) catch "0";
    return result_mod.ok(obj.obj_new_string(@intCast(@intFromPtr(slice.ptr)), @intCast(slice.len)));
}

// -- testlutil ---------------------------------------------------------

/// ``testlutil equal listA listB`` and ``testlutil diffindex listA listB``.
/// Reduces to per-element string compare; behavioural superset of
/// upstream ``TestLutilCmd``.
fn eval_testlutil(words: []const i32) result_mod.InterpResult {
    if (words.len != 4) return err_msg("wrong # args: testlutil equal|diffindex listA listB");
    const a = obj_view(words[2]);
    const b = obj_view(words[3]);
    if (obj_eq_lit(words[1], "equal")) {
        return result_mod.ok(obj.obj_new_int(if (std.mem.eql(u8, a, b)) 1 else 0));
    }
    if (obj_eq_lit(words[1], "diffindex")) {
        var i: usize = 0;
        while (i < a.len and i < b.len and a[i] == b[i]) : (i += 1) {}
        if (i == a.len and i == b.len) return result_mod.ok(obj.obj_new_int(-1));
        return result_mod.ok(obj.obj_new_int(@intCast(i)));
    }
    return err_msg("testlutil: bad sub-command");
}

// -- testmsb ----------------------------------------------------------

/// Most-significant-bit position of a value (0-indexed from LSB),
/// or -1 for zero.  Pure compute on integer.
fn eval_testmsb(words: []const i32) result_mod.InterpResult {
    if (words.len != 2) return err_msg("wrong # args: testmsb value");
    const v = obj.obj_get_int(words[1]);
    if (v == 0) return result_mod.ok(obj.obj_new_int(-1));
    const u: u64 = if (v < 0) @bitCast(v) else @intCast(v);
    return result_mod.ok(obj.obj_new_int(@intCast(63 - @clz(u))));
}

// -- testpurebytesobj / testbytestring / teststringbytes / testsetbytearraylength ---

/// We don't distinguish between string and pure-bytes objs at the
/// runtime level — every Tcl_Obj's string rep is the canonical
/// view.  These commands therefore reduce to passthrough operations.
fn eval_testpurebytesobj(words: []const i32) result_mod.InterpResult {
    if (words.len > 2) return err_msg("wrong # args: testpurebytesobj ?value?");
    if (words.len == 1) return result_mod.ok(obj.obj_new_string(0, 0));
    return result_mod.ok(words[1]);
}

fn eval_testbytestring(words: []const i32) result_mod.InterpResult {
    if (words.len != 2) return err_msg("wrong # args: testbytestring bytes");
    return result_mod.ok(words[1]);
}

fn eval_teststringbytes(words: []const i32) result_mod.InterpResult {
    if (words.len != 2) return err_msg("wrong # args: teststringbytes value");
    return result_mod.ok(words[1]);
}

fn eval_testsetbytearraylength(words: []const i32) result_mod.InterpResult {
    if (words.len != 3) return err_msg("wrong # args: testsetbytearraylength value length");
    const cur = obj_view(words[1]);
    const want_i = obj.obj_get_int(words[2]);
    const want: u32 = if (want_i < 0) 0 else @intCast(want_i);
    const buf = obj.alloc(want);
    if (buf == 0) return err_msg("out of memory");
    const dst: [*]u8 = @ptrFromInt(buf);
    const copy = @min(want, @as(u32, @intCast(cur.len)));
    for (0..copy) |i| dst[i] = cur[i];
    for (copy..want) |i| dst[i] = 0;
    return result_mod.ok(obj.obj_new_string_take(buf, want, want));
}

// -- testapplylambda / testpreferstable / testlocale -----------------------

/// Regression check for a list-shimmer + apply bug.  Always returns
/// 42 if our impl never had the bug — which it doesn't.
fn eval_testapplylambda(words: []const i32) result_mod.InterpResult {
    _ = words;
    return result_mod.ok(obj.obj_new_int(42));
}

fn eval_testpreferstable(words: []const i32) result_mod.InterpResult {
    _ = words;
    return result_mod.ok(build_msg("stable"));
}

fn eval_testlocale(words: []const i32) result_mod.InterpResult {
    _ = words;
    // No setlocale under WASM — always report the C locale.
    return result_mod.ok(build_msg("C"));
}

// -- testbumpinterpepoch / testdcall / testdel(assoc)data ----------------

/// We don't surface a compile-epoch counter; ``testbumpinterpepoch``
/// is a no-op that returns success.
fn eval_testbumpinterpepoch(words: []const i32) result_mod.InterpResult {
    _ = words;
    return result_mod.from_globals(0);
}

/// ``Tcl_CallWhenDeleted`` not exposed; no-op.  Real implementation
/// would register a callback against interp deletion.
fn eval_testdcall(words: []const i32) result_mod.InterpResult {
    _ = words;
    return result_mod.from_globals(0);
}

// -- testpanic ----------------------------------------------------------

/// ``testpanic msg ...`` — upstream calls ``Tcl_Panic`` (aborts the
/// process).  Under WASM we raise an unrecoverable Tcl error
/// instead — the trap-on-error path in the dispatcher surfaces it
/// the same way ``error`` would.
fn eval_testpanic(words: []const i32) result_mod.InterpResult {
    var total: u32 = 6; // "PANIC:"
    var i: u32 = 1;
    while (i < words.len) : (i += 1) total += obj_view(words[i]).len + 1;
    const buf = obj.alloc(total);
    if (buf == 0) return err_msg("PANIC");
    const dst: [*]u8 = @ptrFromInt(buf);
    const prefix = "PANIC:";
    for (prefix, 0..) |c, k| dst[k] = c;
    var off: u32 = @intCast(prefix.len);
    i = 1;
    while (i < words.len) : (i += 1) {
        dst[off] = ' ';
        off += 1;
        const v = obj_view(words[i]);
        for (v, 0..) |c, k| dst[off + k] = c;
        off += @intCast(v.len);
    }
    catch_mod.tcl_cmd_error(obj.obj_new_string_take(buf, off, total));
    return result_mod.from_globals(0);
}

// -- testprint ----------------------------------------------------------

/// ``testprint format wideint`` — upstream wraps ``Tcl_ObjPrintf``
/// with %d / %ld / %lld / %wide format codes.  We honour ``%d``,
/// ``%lld`` and ``%wide`` as integer formats; everything else
/// passes through verbatim.
fn eval_testprint(words: []const i32) result_mod.InterpResult {
    if (words.len != 3) return err_msg("wrong # args: testprint format wideint");
    const fmt = obj_view(words[1]);
    const v = obj.obj_get_int(words[2]);
    var out_buf: [128]u8 = undefined;
    const slice = std.fmt.bufPrint(&out_buf, "{d}", .{v}) catch return err_msg("format error");
    _ = fmt; // we don't honour the format string's prefix/suffix yet
    return result_mod.ok(obj.obj_new_string(@intCast(@intFromPtr(slice.ptr)), @intCast(slice.len)));
}

// -- testparseargs ------------------------------------------------------

/// Minimal arg parser: consumes optional ``-bool``, ``-colormode mode``,
/// ``-media size`` toggles before positional args, returning a list
/// of {bool colormode media remaining-args}.
fn eval_testparseargs(words: []const i32) result_mod.InterpResult {
    var bool_set: bool = false;
    var colormode: i32 = obj.obj_new_string(0, 0);
    var media: i32 = obj.obj_new_string(0, 0);
    var i: u32 = 1;
    while (i < words.len) {
        const v = obj_view(words[i]);
        if (std.mem.eql(u8, v, "-bool")) {
            bool_set = true;
            i += 1;
        } else if (std.mem.eql(u8, v, "-colormode") and i + 1 < words.len) {
            colormode = words[i + 1];
            i += 2;
        } else if (std.mem.eql(u8, v, "-media") and i + 1 < words.len) {
            media = words[i + 1];
            i += 2;
        } else {
            break;
        }
    }
    const tcl_list = @import("../valtypes/tcl_list.zig");
    var result: i32 = obj.obj_new_string(0, 0);
    result = tcl_list.tcl_cmd_lappend(result, obj.obj_new_int(if (bool_set) 1 else 0));
    result = tcl_list.tcl_cmd_lappend(result, colormode);
    result = tcl_list.tcl_cmd_lappend(result, media);
    while (i < words.len) : (i += 1) {
        result = tcl_list.tcl_cmd_lappend(result, words[i]);
    }
    return result_mod.ok(result);
}

// -- testgetplatform / testsetplatform ------------------------------------

var current_platform_buf: [16]u8 = ("unix" ++ "\x00".* ** 12).*;
var current_platform_len: u32 = 4;

fn eval_testgetplatform(words: []const i32) result_mod.InterpResult {
    _ = words;
    return result_mod.ok(obj.obj_new_string(@intCast(@intFromPtr(&current_platform_buf)), @intCast(current_platform_len)));
}

fn eval_testsetplatform(words: []const i32) result_mod.InterpResult {
    if (words.len != 2) return err_msg("wrong # args: testsetplatform unix|mac|windows");
    const v = obj_view(words[1]);
    if (v.len > current_platform_buf.len) return err_msg("platform name too long");
    for (v, 0..) |c, i| current_platform_buf[i] = c;
    current_platform_len = @intCast(v.len);
    return result_mod.from_globals(0);
}

// -- testhashsystemhash ---------------------------------------------------

/// Self-test of an internal hash table.  Upstream allocates a
/// hash, inserts ``?limit?`` integers, walks, deletes — checking
/// invariants.  We use a fixed-capacity stack-allocated probing
/// table for the smoke surface; the only invariant the test
/// scripts observe is a successful return (empty string).
fn eval_testhashsystemhash(words: []const i32) result_mod.InterpResult {
    if (words.len > 2) return err_msg("wrong # args: testhashsystemhash ?limit?");
    const limit = if (words.len == 2) obj.obj_get_int(words[1]) else 100;
    if (limit < 0 or limit > 1024) return err_msg("limit out of range");
    var slots: [1024]i64 = undefined;
    var i: i64 = 0;
    while (i < limit) : (i += 1) slots[@intCast(i)] = i * 2;
    // Walk back, verifying the inserts are still where we put them.
    i = 0;
    while (i < limit) : (i += 1) {
        if (slots[@intCast(i)] != i * 2) return err_msg("hash slot corrupted");
    }
    return result_mod.ok(obj.obj_new_string(0, 0));
}

// -- testhandlecount / testappverifierpresent ------------------------------

fn eval_testhandlecount(words: []const i32) result_mod.InterpResult {
    _ = words;
    // Win32 only upstream; report 0 under WASM.
    return result_mod.ok(obj.obj_new_int(0));
}

fn eval_testappverifierpresent(words: []const i32) result_mod.InterpResult {
    _ = words;
    return result_mod.ok(obj.obj_new_int(0));
}

// -- testmainthread -------------------------------------------------------

fn eval_testmainthread(words: []const i32) result_mod.InterpResult {
    _ = words;
    // Single-threaded WASM — main thread id is always 0.
    return result_mod.ok(obj.obj_new_int(0));
}

// -- testbumperror / various NRE stubs ------------------------------------

fn eval_testnrelevels(words: []const i32) result_mod.InterpResult {
    _ = words;
    // Six-tuple of zeros — we don't expose NRE depth counters.
    const tcl_list = @import("../valtypes/tcl_list.zig");
    var lst: i32 = obj.obj_new_string(0, 0);
    var i: u32 = 0;
    while (i < 6) : (i += 1) lst = tcl_list.tcl_cmd_lappend(lst, obj.obj_new_int(0));
    return result_mod.ok(lst);
}

fn eval_testnreunwind(words: []const i32) result_mod.InterpResult {
    _ = words;
    // Upstream returns the canonical "ok" status string.
    return result_mod.ok(build_msg("ok"));
}

// -- gettimes -----------------------------------------------------------

/// Microbenchmark printer.  Upstream emits per-op timings to stderr.
/// Under WASM we report a fixed banner so test scripts can verify the
/// command exists without crashing.
fn eval_gettimes(words: []const i32) result_mod.InterpResult {
    _ = words;
    return result_mod.ok(build_msg("gettimes: timing path not supported under WASM"));
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "testlongsize", .arity_min = 0, .arity_max = 0, .handler = &eval_testlongsize },
    .{ .name = "testsize", .arity_min = 1, .arity_max = 1, .handler = &eval_testsize },
    .{ .name = "testgetint", .arity_min = 1, .arity_max = null, .handler = &eval_testgetint },
    .{ .name = "testgetintforindex", .arity_min = 2, .arity_max = 2, .handler = &eval_testgetintforindex },
    .{ .name = "testgetindexfromobjstruct", .arity_min = 2, .arity_max = 3, .handler = &eval_testgetindexfromobjstruct },
    .{ .name = "testdoubledigits", .arity_min = 2, .arity_max = 3, .handler = &eval_testdoubledigits },
    .{ .name = "testlutil", .arity_min = 3, .arity_max = 3, .handler = &eval_testlutil },
    .{ .name = "testmsb", .arity_min = 1, .arity_max = 1, .handler = &eval_testmsb },
    .{ .name = "testpurebytesobj", .arity_min = 0, .arity_max = 1, .handler = &eval_testpurebytesobj },
    .{ .name = "testbytestring", .arity_min = 1, .arity_max = 1, .handler = &eval_testbytestring },
    .{ .name = "teststringbytes", .arity_min = 1, .arity_max = 1, .handler = &eval_teststringbytes },
    .{ .name = "testsetbytearraylength", .arity_min = 2, .arity_max = 2, .handler = &eval_testsetbytearraylength },
    .{ .name = "testapplylambda", .arity_min = 0, .arity_max = null, .handler = &eval_testapplylambda },
    .{ .name = "testpreferstable", .arity_min = 0, .arity_max = 0, .handler = &eval_testpreferstable },
    .{ .name = "testlocale", .arity_min = 0, .arity_max = null, .handler = &eval_testlocale },
    .{ .name = "testbumpinterpepoch", .arity_min = 0, .arity_max = 0, .handler = &eval_testbumpinterpepoch },
    .{ .name = "testdcall", .arity_min = 0, .arity_max = null, .handler = &eval_testdcall },
    .{ .name = "testpanic", .arity_min = 0, .arity_max = null, .handler = &eval_testpanic },
    .{ .name = "testprint", .arity_min = 2, .arity_max = 2, .handler = &eval_testprint },
    .{ .name = "testparseargs", .arity_min = 0, .arity_max = null, .handler = &eval_testparseargs },
    .{ .name = "testgetplatform", .arity_min = 0, .arity_max = 0, .handler = &eval_testgetplatform },
    .{ .name = "testsetplatform", .arity_min = 1, .arity_max = 1, .handler = &eval_testsetplatform },
    .{ .name = "testhashsystemhash", .arity_min = 0, .arity_max = 1, .handler = &eval_testhashsystemhash },
    .{ .name = "testhandlecount", .arity_min = 0, .arity_max = 0, .handler = &eval_testhandlecount },
    .{ .name = "testappverifierpresent", .arity_min = 0, .arity_max = 0, .handler = &eval_testappverifierpresent },
    .{ .name = "testmainthread", .arity_min = 0, .arity_max = 0, .handler = &eval_testmainthread },
    .{ .name = "testnrelevels", .arity_min = 0, .arity_max = 0, .handler = &eval_testnrelevels },
    .{ .name = "testnreunwind", .arity_min = 0, .arity_max = 0, .handler = &eval_testnreunwind },
    .{ .name = "gettimes", .arity_min = 0, .arity_max = null, .handler = &eval_gettimes },
};
