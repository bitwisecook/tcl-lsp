// Global variable table — thin forwarding shim onto the root
// namespace's ``var_table`` (P3.2 onwards).  All storage lives in
// ``tcl_ns.zig`` now; this module just preserves the long-standing
// ``global_set`` / ``global_get`` / ``global_exists`` ABI so
// existing callers (the compiler-emitted code, ``tcl_frames.zig``'s
// ALIAS_GLOBAL fallback, ``tcl_array.zig``) keep working without
// changes.
//
// P3.4 deletes this file once ``tcl_frames.zig`` and
// ``tcl_array.zig`` migrate to call through ``tcl_ns`` directly.

const obj = @import("tcl_obj.zig");
const obj_ensure_string = obj.obj_ensure_string;
const obj_new_int = obj.obj_new_int;
const obj_get_int = obj.obj_get_int;

const tcl_ns = @import("tcl_ns.zig");

pub export fn global_set(name: i32, value: i32) i32 {
    const sn = obj_ensure_string(name);
    const root = tcl_ns.ns_root();
    const v = tcl_ns.ns_var_create(root, sn.ptr, sn.len);
    tcl_ns.var_set_scalar(v, @bitCast(value));
    return value;
}

pub export fn global_get(name: i32) i32 {
    const sn = obj_ensure_string(name);
    const root = tcl_ns.ns_root();
    const v = tcl_ns.ns_var_find(root, sn.ptr, sn.len);
    if (v == 0) return 0;
    return @bitCast(tcl_ns.var_get_scalar(v));
}

pub export fn global_exists(name: i32) i32 {
    const sn = obj_ensure_string(name);
    const root = tcl_ns.ns_root();
    const v = tcl_ns.ns_var_find(root, sn.ptr, sn.len);
    if (v == 0) return obj_new_int(0);
    // Match the prior semantics: an entry exists iff someone has
    // ever called ``global_set``, even if the value was set back to
    // 0.  ``ns_var_find`` returns the ``*Var``; existence == ``v != 0``.
    // Don't follow ``VAR_LINK`` here — ``info exists`` semantics are
    // about the local entry, not the terminal storage.
    return obj_new_int(1);
}

pub export fn tcl_incr(o: i32, amount: i32) i32 {
    const val = obj_get_int(o);
    const amt = obj_get_int(amount);
    return obj_new_int(val + amt);
}
