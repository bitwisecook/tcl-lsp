// Object-tier ``test*`` commands (``testintobj``, ``testbooleanobj``,
// ``testdoubleobj``, …) ported from upstream ``generic/tclTestObj.c``.
//
// Linked into ``tcl_runtime_with_tcltest.wasm`` only.
// ``dispatch/tcl_cmd_table.zig`` ``inline if``s the
// :data:`registrations` slice in based on
// ``build_options.with_tcltest``; on the lean ``tcl_runtime`` build
// the module is replaced by an empty stub via the ``else`` branch.
//
// PR-1 ports a small but real surface end-to-end (set / get / mult10
// / div10 / set2 over ``testintobj`` plus matching minimums on
// ``testbooleanobj`` and ``testdoubleobj``); subsequent PRs extend
// the surface command-by-command per
// ``docs/design/compiler/wasm-extensions-tcltest.md``.

const obj = @import("../valtypes/tcl_obj.zig");
const result_mod = @import("../interp/tcl_result.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");

const slots = @import("slots.zig");

/// Compare a TclObj's string view against a literal ASCII bareword.
/// Used for sub-command dispatch (``set`` / ``get`` / …).
fn obj_eq_lit(handle: i32, lit: []const u8) bool {
    const s = obj.obj_ensure_string(handle);
    if (s.len != lit.len) return false;
    const ptr: [*]const u8 = @ptrFromInt(s.ptr);
    for (0..lit.len) |i| {
        if (ptr[i] != lit[i]) return false;
    }
    return true;
}

// -- testintobj -------------------------------------------------------------

/// ``testintobj`` — int-typed Tcl_Obj test command.
///
/// Sub-commands implemented in the PR-1 cut:
///   * ``testintobj set <slot> <value>`` — store integer in slot;
///     return the new obj (matches upstream's ``set`` setting the
///     interp result to the new value).
///   * ``testintobj set2 <slot> <value>`` — same store but does NOT
///     set the interp result (matches upstream's ``set2`` which is
///     used to test the unshared-rep mutation path).
///   * ``testintobj get <slot>`` — return the slot value, error on
///     unset.
///   * ``testintobj mult10 <slot>`` — multiply slot value by 10.
///   * ``testintobj div10 <slot>``  — integer-divide slot value by 10.
fn eval_testintobj(words: []const i32) result_mod.InterpResult {
    if (words.len < 3) {
        slots.raise_wrong_args("testintobj option arg ?arg ...?");
        return result_mod.from_globals(0);
    }
    const slot = slots.parse_slot_index(words[2]) orelse {
        slots.raise_bad_index(words[2]);
        return result_mod.from_globals(0);
    };

    if (obj_eq_lit(words[1], "set")) {
        if (words.len != 4) {
            slots.raise_wrong_args("testintobj set varIndex value");
            return result_mod.from_globals(0);
        }
        const v = obj.obj_new_int(obj.obj_get_int(words[3]));
        slots.set(slot, v);
        return result_mod.ok(v);
    }
    if (obj_eq_lit(words[1], "set2")) {
        if (words.len != 4) {
            slots.raise_wrong_args("testintobj set2 varIndex value");
            return result_mod.from_globals(0);
        }
        const v = obj.obj_new_int(obj.obj_get_int(words[3]));
        slots.set(slot, v);
        return result_mod.ok(obj.obj_new_string(0, 0));
    }
    if (obj_eq_lit(words[1], "get")) {
        if (words.len != 3) {
            slots.raise_wrong_args("testintobj get varIndex");
            return result_mod.from_globals(0);
        }
        if (slots.is_unset(slot)) {
            slots.raise_var_unset(slot);
            return result_mod.from_globals(0);
        }
        return result_mod.ok(slots.get(slot));
    }
    if (obj_eq_lit(words[1], "mult10")) {
        if (words.len != 3) {
            slots.raise_wrong_args("testintobj mult10 varIndex");
            return result_mod.from_globals(0);
        }
        if (slots.is_unset(slot)) {
            slots.raise_var_unset(slot);
            return result_mod.from_globals(0);
        }
        const cur = obj.obj_get_int(slots.get(slot));
        const next = obj.obj_new_int(cur * 10);
        slots.set(slot, next);
        return result_mod.ok(next);
    }
    if (obj_eq_lit(words[1], "div10")) {
        if (words.len != 3) {
            slots.raise_wrong_args("testintobj div10 varIndex");
            return result_mod.from_globals(0);
        }
        if (slots.is_unset(slot)) {
            slots.raise_var_unset(slot);
            return result_mod.from_globals(0);
        }
        const cur = obj.obj_get_int(slots.get(slot));
        const next = obj.obj_new_int(@divTrunc(cur, 10));
        slots.set(slot, next);
        return result_mod.ok(next);
    }
    slots.raise_wrong_args("testintobj: unknown sub-command (set|set2|get|mult10|div10)");
    return result_mod.from_globals(0);
}

// -- testbooleanobj ---------------------------------------------------------

/// ``testbooleanobj`` — boolean-typed Tcl_Obj test command.
///
/// Sub-commands implemented in the PR-1 cut:
///   * ``testbooleanobj set <slot> <value>`` — store boolean.
///   * ``testbooleanobj get <slot>`` — return slot value.
///   * ``testbooleanobj not <slot>`` — logically negate slot value
///     (also stores back).
fn eval_testbooleanobj(words: []const i32) result_mod.InterpResult {
    if (words.len < 3) {
        slots.raise_wrong_args("testbooleanobj option arg ?arg ...?");
        return result_mod.from_globals(0);
    }
    const slot = slots.parse_slot_index(words[2]) orelse {
        slots.raise_bad_index(words[2]);
        return result_mod.from_globals(0);
    };

    if (obj_eq_lit(words[1], "set")) {
        if (words.len != 4) {
            slots.raise_wrong_args("testbooleanobj set varIndex value");
            return result_mod.from_globals(0);
        }
        const v: i64 = if (obj.obj_get_int(words[3]) != 0) 1 else 0;
        const out = obj.obj_new_int(v);
        slots.set(slot, out);
        return result_mod.ok(out);
    }
    if (obj_eq_lit(words[1], "get")) {
        if (words.len != 3) {
            slots.raise_wrong_args("testbooleanobj get varIndex");
            return result_mod.from_globals(0);
        }
        if (slots.is_unset(slot)) {
            slots.raise_var_unset(slot);
            return result_mod.from_globals(0);
        }
        return result_mod.ok(slots.get(slot));
    }
    if (obj_eq_lit(words[1], "not")) {
        if (words.len != 3) {
            slots.raise_wrong_args("testbooleanobj not varIndex");
            return result_mod.from_globals(0);
        }
        if (slots.is_unset(slot)) {
            slots.raise_var_unset(slot);
            return result_mod.from_globals(0);
        }
        const cur = obj.obj_get_int(slots.get(slot));
        const out = obj.obj_new_int(if (cur == 0) 1 else 0);
        slots.set(slot, out);
        return result_mod.ok(out);
    }
    slots.raise_wrong_args("testbooleanobj: unknown sub-command (set|get|not)");
    return result_mod.from_globals(0);
}

// -- testdoubleobj ----------------------------------------------------------

/// ``testdoubleobj`` — double-typed Tcl_Obj test command.
///
/// Sub-commands implemented in the PR-1 cut:
///   * ``testdoubleobj set <slot> <value>``
///   * ``testdoubleobj get <slot>``
///   * ``testdoubleobj mult10 <slot>``
fn eval_testdoubleobj(words: []const i32) result_mod.InterpResult {
    if (words.len < 3) {
        slots.raise_wrong_args("testdoubleobj option arg ?arg ...?");
        return result_mod.from_globals(0);
    }
    const slot = slots.parse_slot_index(words[2]) orelse {
        slots.raise_bad_index(words[2]);
        return result_mod.from_globals(0);
    };

    if (obj_eq_lit(words[1], "set")) {
        if (words.len != 4) {
            slots.raise_wrong_args("testdoubleobj set varIndex value");
            return result_mod.from_globals(0);
        }
        const v = obj.obj_new_float(obj.obj_get_float(words[3]));
        slots.set(slot, v);
        return result_mod.ok(v);
    }
    if (obj_eq_lit(words[1], "get")) {
        if (words.len != 3) {
            slots.raise_wrong_args("testdoubleobj get varIndex");
            return result_mod.from_globals(0);
        }
        if (slots.is_unset(slot)) {
            slots.raise_var_unset(slot);
            return result_mod.from_globals(0);
        }
        return result_mod.ok(slots.get(slot));
    }
    if (obj_eq_lit(words[1], "mult10")) {
        if (words.len != 3) {
            slots.raise_wrong_args("testdoubleobj mult10 varIndex");
            return result_mod.from_globals(0);
        }
        if (slots.is_unset(slot)) {
            slots.raise_var_unset(slot);
            return result_mod.from_globals(0);
        }
        const cur = obj.obj_get_float(slots.get(slot));
        const out = obj.obj_new_float(cur * 10.0);
        slots.set(slot, out);
        return result_mod.ok(out);
    }
    slots.raise_wrong_args("testdoubleobj: unknown sub-command (set|get|mult10)");
    return result_mod.from_globals(0);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "testintobj", .arity_min = 2, .arity_max = 3, .handler = &eval_testintobj },
    .{ .name = "testbooleanobj", .arity_min = 2, .arity_max = 3, .handler = &eval_testbooleanobj },
    .{ .name = "testdoubleobj", .arity_min = 2, .arity_max = 3, .handler = &eval_testdoubleobj },
};
