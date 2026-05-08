// Per-extension TclObj slot table.
//
// The upstream tcltest C commands (``testintobj``, ``teststringobj``,
// …) thread a private ``Tcl_Obj **varPtr`` array indexed by integer
// (``GetVarPtr`` / ``SetVarToObj`` in ``generic/tclTestObj.c``) so the
// tests can probe object lifecycle without name-table interference.
// The Zig port mirrors that: a fixed-size array of ``i32`` TclObj
// handles, plus retain / release through the runtime's refcount API
// so freed slabs can't leak back into the tests via stale handles.
//
// Slots are zero-initialised; an ``ismax`` / ``ismin`` / ``get``
// against an unset slot must raise via :fn:`raise_var_unset` before
// dereferencing — the C tests rely on that error to validate
// shared-rep behaviour.
//
// Linked into ``tcl_runtime_with_tcltest.wasm`` only — ``cmd_table``
// gates the import on ``build_options.with_tcltest``.

const std = @import("std");
const obj = @import("../valtypes/tcl_obj.zig");
const catch_mod = @import("../interp/tcl_catch.zig");

/// Number of slot entries.  Upstream's ``NUMBER_OF_OBJECT_VARS`` is
/// 20; we round up to 256 so future test commands that push more
/// slots (the ``testlistobj`` suite uses 8 slots; we keep room for a
/// few more) don't run into the ceiling.
pub const NUM_SLOTS: u32 = 256;

var slots: [NUM_SLOTS]i32 = [_]i32{0} ** NUM_SLOTS;

/// Resolve an integer slot index from a TclObj.  ``handle`` must
/// stringify to a syntactically-valid base-10 (or 0x.../0o.../0b...)
/// integer in ``[0, NUM_SLOTS)``; everything else (a non-numeric
/// string, an out-of-range integer, an empty obj) returns ``null``.
///
/// Why strict: :fn:`obj.obj_get_int` is permissive — it returns 0
/// for inputs it can't parse — which let malformed indices silently
/// alias slot 0 across every ``test*obj`` command, diverging from
/// upstream's "bad index" error semantics (Codex review feedback,
/// PR #363).  We route through :fn:`obj.try_parse_int` here so a
/// non-integer slot argument turns into a clear error rather than
/// a stealth slot-0 mutation.
pub fn parse_slot_index(handle: i32) ?u32 {
    const s = obj.obj_ensure_string(handle);
    const v = obj.try_parse_int(s.ptr, s.len) orelse return null;
    if (v < 0 or v >= NUM_SLOTS) return null;
    return @intCast(v);
}

/// Read the obj at slot *idx*, or 0 (TclObj handle for "no obj") if
/// the slot is unset.  Does NOT retain — callers that store the
/// result must retain explicitly.
pub fn get(idx: u32) i32 {
    if (idx >= NUM_SLOTS) return 0;
    return slots[idx];
}

/// Store *handle* in slot *idx*, retaining the new value and
/// releasing the prior occupant.  ``handle == 0`` clears the slot.
pub fn set(idx: u32, handle: i32) void {
    if (idx >= NUM_SLOTS) return;
    if (handle != 0) obj.tcl_obj_retain(handle);
    const old = slots[idx];
    slots[idx] = handle;
    if (old != 0) obj.tcl_obj_release(old);
}

/// True iff slot *idx* is currently empty.
pub fn is_unset(idx: u32) bool {
    if (idx >= NUM_SLOTS) return true;
    return slots[idx] == 0;
}

fn build_msg(buf: []const u8) i32 {
    const dst = obj.alloc(@intCast(buf.len));
    if (dst == 0) return 0;
    const dst_p: [*]u8 = @ptrFromInt(dst);
    for (buf, 0..) |c, i| dst_p[i] = c;
    return obj.obj_new_string_take(dst, @intCast(buf.len), @intCast(buf.len));
}

/// Raise ``"bad slot index"`` via the runtime's error path.  Used by
/// every command that takes a slot argument so the test suite gets a
/// Tcl-shaped error.
pub fn raise_bad_index(idx_handle: i32) void {
    _ = idx_handle;
    const msg = build_msg("bad slot index: must be 0..255");
    catch_mod.tcl_cmd_error(msg);
}

/// Raise ``"variable <slot> is unset"`` for unset-slot reads.
pub fn raise_var_unset(idx: u32) void {
    var buf: [64]u8 = undefined;
    const slice = std.fmt.bufPrint(&buf, "variable {d} is unset", .{idx}) catch "variable is unset";
    const msg = build_msg(slice);
    catch_mod.tcl_cmd_error(msg);
}

/// Raise ``wrong # args`` for the calling sub-command.  *usage* is
/// the short hint string upstream emits.
pub fn raise_wrong_args(usage: []const u8) void {
    const msg = build_msg(usage);
    catch_mod.tcl_cmd_error(msg);
}

/// Test-only: clear every slot.  Production WASM never calls this —
/// extension state lives for the lifetime of the merged module.
pub fn reset_for_tests() void {
    for (0..NUM_SLOTS) |i| {
        if (slots[i] != 0) obj.tcl_obj_release(slots[i]);
        slots[i] = 0;
    }
}
