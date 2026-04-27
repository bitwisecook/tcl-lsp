// Minimum-viable TclOO scaffold (Phase 6.1 / Step 3 of the
// trap-cluster sweep plan).
//
// The full TclOO machinery (~3500 LOC of C in tclOO.c) is out of
// scope for this cycle; the goal here is narrowly to remove the
// ``oo::*`` traps that abort ``oo.test`` (and any test file that
// brushes up against TclOO at init time) so the surrounding suite
// reaches a tcltest summary line.  Tests that exercise real
// object semantics still fail, but the failures are local rather
// than a hard ``unsupported command: oo::class`` trap that aborts
// the whole bundle.
//
// What's stubbed:
//   * ``oo::class create NAME ?BODY?`` — registers NAME as a Tcl
//     proc whose body is the no-op object handler.  The proc
//     accepts any args (``args`` tail) and returns NAME, so
//     ``[oo::class create C]`` flows are sane and the ``C new``
//     dispatch reaches the same handler.
//   * ``oo::object new`` / ``create`` — returns a fresh object
//     name (``::oo::Obj_<N>``) registered as a no-op proc.
//   * ``oo::define``, ``oo::objdefine``, ``oo::copy``,
//     ``oo::abstract``, ``oo::configurable``, ``oo::singleton`` —
//     return the empty string.  Tests that try to define real
//     methods through these get a no-op define and fail-but-run.
//   * ``my``, ``next``, ``nextto``, ``self``, ``classvariable`` —
//     no-op stubs returning the empty string; these only show up
//     inside method bodies which won't run anyway under the
//     scaffold.
//
// Real implementation lives in a future commit.  The contract
// here is: no traps, callers get a deterministic empty-ish
// answer, oo.test reaches its summary line.

const std = @import("std");
const obj = @import("../valtypes/tcl_obj.zig");
const procs = @import("../interp/tcl_procs.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");

const obj_new_string = obj.obj_new_string;
const obj_new_string_copy = obj.obj_new_string_copy;
const obj_ensure_string = obj.obj_ensure_string;

/// Counter that backs the ``::oo::Obj_<N>`` names produced by
/// ``oo::object new`` and ``oo::class new`` / ``create``.  Plain
/// monotonic — no recycling on destroy.
var oo_obj_counter: u32 = 0;

/// Register *name* as an interpreted proc whose body is empty.
/// Used by ``oo::class create FOO`` / ``oo::object new`` so the
/// resulting object's name resolves to a callable command in
/// ``proc_lookup``.  The proc body returns the empty string for
/// every method invocation — sufficient for the scaffold's
/// "no traps, fail-but-run" contract.
fn register_object_proc(name_obj: i32) void {
    if (name_obj == 0) return;
    const empty_params = obj_new_string_copy(@intFromPtr("args".ptr), 4);
    const empty_body = obj_new_string_copy(@intFromPtr("".ptr), 0);
    _ = procs.proc_register(name_obj, empty_params, empty_body);
}

/// Build ``::oo::Obj_<N>`` for the next object-counter slot.
fn synthesise_object_name() i32 {
    oo_obj_counter += 1;
    var buf: [32]u8 = undefined;
    const prefix = "::oo::Obj_";
    var off: u32 = 0;
    for (prefix) |c| {
        buf[off] = c;
        off += 1;
    }
    // Decimal encoding of oo_obj_counter into buf.
    var v = oo_obj_counter;
    if (v == 0) {
        buf[off] = '0';
        off += 1;
    } else {
        var tmp: [12]u8 = undefined;
        var tlen: u32 = 0;
        while (v > 0) {
            tmp[tlen] = @intCast(@as(u32, '0') + (v % 10));
            v /= 10;
            tlen += 1;
        }
        var i: u32 = 0;
        while (i < tlen) : (i += 1) {
            buf[off + i] = tmp[tlen - 1 - i];
        }
        off += tlen;
    }
    return obj_new_string_copy(@intFromPtr(&buf), off);
}

/// ``oo::class create NAME ?defscript?`` / ``oo::class new`` —
/// registers NAME (or an auto-generated name for ``new``) as a
/// proc.  Ignores the optional defscript.  Returns the proc name.
fn eval_oo_class(words: []const i32) i32 {
    if (words.len < 2) return obj_new_string(0, 0);
    const sub_obj = words[1];
    const sub = obj_ensure_string(sub_obj);
    if (sub.len == 0) return obj_new_string(0, 0);
    const sp: [*]const u8 = @ptrFromInt(sub.ptr);
    if (sub.len == 6 and sp[0] == 'c' and sp[1] == 'r' and sp[2] == 'e' and sp[3] == 'a' and sp[4] == 't' and sp[5] == 'e') {
        if (words.len < 3) return obj_new_string(0, 0);
        const name_obj = words[2];
        register_object_proc(name_obj);
        return name_obj;
    }
    if (sub.len == 3 and sp[0] == 'n' and sp[1] == 'e' and sp[2] == 'w') {
        const name_obj = synthesise_object_name();
        register_object_proc(name_obj);
        return name_obj;
    }
    if (sub.len == 7 and sp[0] == 'd' and sp[1] == 'e' and sp[2] == 's' and sp[3] == 't' and sp[4] == 'r' and sp[5] == 'o' and sp[6] == 'y') {
        return obj_new_string(0, 0);
    }
    return obj_new_string(0, 0);
}

/// ``oo::object create NAME`` / ``oo::object new`` — registers a
/// fresh named or auto-named object proc.  Ignores destroy.
fn eval_oo_object(words: []const i32) i32 {
    return eval_oo_class(words);
}

/// Catch-all no-op for the remaining oo:: commands.  Returns the
/// empty string for every call — enough to keep tests that reach
/// these commands from trapping while real behaviour is missing.
fn eval_oo_noop(words: []const i32) i32 {
    _ = words;
    return obj_new_string(0, 0);
}

// Arities mirror ``core/commands/registry/tcl/oo_*.py`` so the
// WASM-command-parity gate stays clean.  The Python registry is
// the single source of truth; tightening here keeps the
// scaffold's contract (no traps) without diverging from the
// stated arity contract.
pub const registrations = [_]reg.CmdEntry{
    .{ .name = "oo::class", .arity_min = 0, .arity_max = null, .handler = &eval_oo_class },
    .{ .name = "oo::object", .arity_min = 0, .arity_max = null, .handler = &eval_oo_object },
    .{ .name = "oo::define", .arity_min = 1, .arity_max = null, .handler = &eval_oo_noop },
    .{ .name = "oo::objdefine", .arity_min = 1, .arity_max = null, .handler = &eval_oo_noop },
    .{ .name = "oo::copy", .arity_min = 1, .arity_max = 3, .handler = &eval_oo_noop },
    .{ .name = "oo::abstract", .arity_min = 0, .arity_max = null, .handler = &eval_oo_noop },
    .{ .name = "oo::configurable", .arity_min = 0, .arity_max = null, .handler = &eval_oo_noop },
    .{ .name = "oo::singleton", .arity_min = 0, .arity_max = null, .handler = &eval_oo_noop },
    // Method-context helpers — no-ops at scaffold scope.
    .{ .name = "my", .arity_min = 1, .arity_max = null, .handler = &eval_oo_noop },
    .{ .name = "next", .arity_min = 0, .arity_max = null, .handler = &eval_oo_noop },
    .{ .name = "nextto", .arity_min = 1, .arity_max = null, .handler = &eval_oo_noop },
    .{ .name = "self", .arity_min = 0, .arity_max = 1, .handler = &eval_oo_noop },
    .{ .name = "classvariable", .arity_min = 1, .arity_max = null, .handler = &eval_oo_noop },
};
