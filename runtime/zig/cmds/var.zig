// ``set``, ``incr``, ``unset`` — variable read/write commands.

const rt = @import("../tcl_runtime.zig");
const result_mod = @import("../interp/tcl_result.zig");
const frames = @import("../interp/tcl_frames.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");
const tcl_array = @import("../valtypes/tcl_array.zig");
const tcl_ns = @import("../interp/tcl_ns.zig");

const obj_ensure_string = rt.obj_ensure_string;
const obj_new_string = rt.obj_new_string;

fn eval_set(words: []const i32) result_mod.InterpResult {
    // Reference Tcl: ``set varName ?newValue?`` takes 1 or 2 args.
    // Out-of-range arities raise ``wrong # args`` (parse-9.2).
    if (words.len < 2 or words.len > 3) {
        const catch_mod = @import("../interp/tcl_catch.zig");
        const msg_text: []const u8 = "wrong # args: should be \"set varName ?newValue?\"";
        const buf = rt.alloc(@intCast(msg_text.len));
        if (buf == 0) {
            catch_mod.tcl_cmd_error(0);
            return result_mod.from_globals(0);
        }
        const dst: [*]u8 = @ptrFromInt(buf);
        for (msg_text, 0..) |b, k| dst[k] = b;
        const msg = rt.obj_new_string_take(buf, @intCast(msg_text.len), @intCast(msg_text.len));
        catch_mod.tcl_cmd_error(msg);
        return result_mod.from_globals(0);
    }
    if (words.len == 3) {
        // Tcl 9 ``const`` semantics: a write to a constant variable
        // raises ``can't set "<name>": variable is a constant``.
        if (name_is_constant(words[1])) {
            raise_const_set_error(words[1], "set");
            return result_mod.from_globals(0);
        }
        _ = frames.var_set(words[1], words[2]);
        return result_mod.from_globals(words[2]);
    }
    // Read form: ``set varName``.  When the variable doesn't exist,
    // raise ``can't read "<name>": no such variable`` per Tcl 9
    // semantics (set-1.13).  ``var_resolve`` returns 0 silently for
    // missing slots — we re-raise via ``var_unset_error``.
    const v = frames.var_resolve(words[1]);
    if (v == 0) {
        const catch_mod = @import("../interp/tcl_catch.zig");
        catch_mod.var_unset_error(words[1]);
        return result_mod.from_globals(0);
    }
    return result_mod.from_globals(v);
}

fn eval_incr(words: []const i32) result_mod.InterpResult {
    if (words.len < 2 or words.len > 3) {
        const stubs = @import("../stubs/tcl_stubs.zig");
        stubs.raise("wrong # args: should be \"incr varName ?increment?\"");
        return result_mod.from_globals(0);
    }
    if (name_is_constant(words[1])) {
        raise_const_set_error(words[1], "incr");
        return result_mod.from_globals(0);
    }
    const amt_obj = if (words.len >= 3) words[2] else rt.obj_new_int(1);
    // Inside a proc frame, ``incr x`` (unqualified) must NOT see an
    // outer ``x`` — unqualified names refer to local variables.
    // BUT qualified names (``incr ::x`` or ``incr ns::x``) always
    // refer to the global / namespace variable, exactly like
    // ``set ::x``.  Skip the local-only restriction in that case;
    // ``var_resolve`` already handles the qualifier routing
    // (interp-29.3.x ``incr ::i`` inside ``proc p`` relies on this).
    const name_s = obj_ensure_string(words[1]);
    var is_qualified = false;
    if (name_s.len >= 2) {
        const np: [*]const u8 = @ptrFromInt(name_s.ptr);
        var qi: u32 = 0;
        while (qi + 1 < name_s.len) : (qi += 1) {
            if (np[qi] == ':' and np[qi + 1] == ':') {
                is_qualified = true;
                break;
            }
        }
    }
    const in_proc = frames.frame_depth > 0;
    const cur: i32 = if (in_proc and !is_qualified)
        frames.local_get_silent(words[1])
    else
        frames.var_resolve(words[1]);
    const result = rt.tcl_incr(cur, amt_obj);
    _ = frames.var_set(words[1], result);
    return result_mod.from_globals(result);
}

fn eval_unset(words: []const i32) result_mod.InterpResult {
    var i: u32 = 1;
    var nocomplain = false;
    // Consume option arguments first (``-nocomplain``, ``--``).
    while (i < words.len) : (i += 1) {
        const w = obj_ensure_string(words[i]);
        if (w.len == 0) continue;
        const wp: [*]const u8 = @ptrFromInt(w.ptr);
        if (wp[0] != '-') break;
        if (w.len == 2 and wp[1] == '-') {
            i += 1;
            break;
        }
        if (w.len == 11) {
            const lit = "-nocomplain";
            var matches = true;
            for (lit, 0..) |c, k2| {
                if (wp[k2] != c) {
                    matches = false;
                    break;
                }
            }
            if (matches) {
                nocomplain = true;
                continue;
            }
        }
        // Unknown option — Tcl 9 ``unset`` is permissive and treats
        // anything else (or any past-option name) as a variable name.
        break;
    }
    while (i < words.len) : (i += 1) {
        const w = obj_ensure_string(words[i]);
        if (w.len == 0) continue;
        // Tcl 9: ``unset X`` raises ``can't unset "X": no such
        // variable`` when X doesn't exist (unless ``-nocomplain``
        // was passed).  For ``arr(key)`` forms, additional
        // discrimination — ``variable isn't array`` when ``arr``
        // is a scalar, ``no such element in array`` when ``arr``
        // is an array but the element is missing.
        if (!nocomplain and !var_exists_for_unset(words[i])) {
            raise_unset_error(words[i]);
            return result_mod.from_globals(0);
        }
        // Tcl 9 ``const``: ``unset`` of a constant variable raises
        // even with ``-nocomplain``.  Match reference Tcl's wording
        // (``can't unset "<name>": variable is a constant``).
        if (name_is_constant(words[i])) {
            raise_const_set_error(words[i], "unset");
            return result_mod.from_globals(0);
        }
        const wp: [*]const u8 = @ptrFromInt(w.ptr);
        // Clear the array table before nulling the variable so
        // ``info exists arr`` returns 0 after ``unset arr``.
        const resolved_arr = frames.frame_resolve_array_name(words[i]);
        const tcl_obj_mod = @import("../valtypes/tcl_obj.zig");
        defer if (resolved_arr != words[i]) tcl_obj_mod.tcl_obj_release(resolved_arr);
        _ = tcl_array.array_unset(resolved_arr);
        var is_global: bool = (w.len >= 2 and wp[0] == ':' and wp[1] == ':');
        if (!is_global) {
            for (0..w.len - 1) |k| {
                if (wp[k] == ':' and wp[k + 1] == ':') {
                    is_global = true;
                    break;
                }
            }
        }
        if (is_global) {
            _ = tcl_ns.global_set(words[i], 0);
        } else {
            _ = frames.var_set(words[i], 0);
        }
    }
    return result_mod.from_globals(obj_new_string(0, 0));
}

/// Does *name* refer to a live variable (scalar or array element)?
/// Used by ``unset`` to decide whether to raise the canonical
/// "can't unset X: no such variable" diagnostic.
fn var_exists_for_unset(name: i32) bool {
    const obj_mod = @import("../valtypes/tcl_obj.zig");
    const r = frames.var_exists(name);
    return obj_mod.obj_get_int(r) != 0;
}

fn raise_unset_error(name: i32) void {
    const obj_mod = @import("../valtypes/tcl_obj.zig");
    const catch_mod = @import("../interp/tcl_catch.zig");
    const sn = obj_mod.obj_ensure_string(name);
    const prefix: []const u8 = "can't unset \"";
    // Pick the suffix the same way var_unset_error (read path)
    // does — array-element vs whole-array vs missing-scalar.
    const suffix = pick_unset_suffix(sn);
    const total: u32 = @intCast(prefix.len + sn.len + suffix.len);
    const buf = obj_mod.alloc(total);
    if (buf == 0) {
        catch_mod.tcl_cmd_error(obj_mod.obj_new_string(0, 0));
        return;
    }
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    const sp: [*]const u8 = @ptrFromInt(sn.ptr);
    for (0..sn.len) |k| {
        dst[off] = sp[k];
        off += 1;
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const msg = obj_mod.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(msg);
}

fn pick_unset_suffix(s: anytype) []const u8 {
    if (s.len == 0 or s.ptr == 0) return "\": no such variable";
    const sp: [*]const u8 = @ptrFromInt(s.ptr);
    var paren: u32 = 0;
    var has_paren = false;
    var k: u32 = 0;
    while (k < s.len) : (k += 1) {
        if (sp[k] == '(') {
            paren = k;
            has_paren = true;
            break;
        }
    }
    if (has_paren and paren > 0 and sp[s.len - 1] == ')') {
        const arr_len = paren;
        var probe_ptr: u32 = s.ptr;
        var probe_len: u32 = arr_len;
        if (arr_len >= 2 and sp[0] == ':' and sp[1] == ':') {
            probe_ptr += 2;
            probe_len -= 2;
        }
        if (tcl_array.array_exists_raw(probe_ptr, probe_len)) {
            return "\": no such element in array";
        }
        const obj_helpers = @import("../valtypes/tcl_obj.zig");
        const arr_obj = obj_helpers.obj_new_string(@bitCast(s.ptr), @bitCast(arr_len));
        const exists = obj_helpers.obj_get_int(tcl_ns.global_exists(arr_obj)) != 0;
        obj_helpers.tcl_obj_release(arr_obj);
        if (exists) {
            return "\": variable isn't array";
        }
        return "\": no such variable";
    }
    return "\": no such variable";
}

/// Predicate: is *name* currently flagged as a Tcl 9 constant?
/// Checks the per-frame const set first (proc-local consts) then
/// the VAR_CONSTANT bit on a namespace-resolved ``Var`` record.
/// Array-element forms (``arr(key)``) are never constants — TIP 590
/// only allows whole scalars.
fn name_is_constant(name: i32) bool {
    const tcl_obj_mod = @import("../valtypes/tcl_obj.zig");
    const sn = tcl_obj_mod.obj_ensure_string(name);
    if (sn.len == 0 or sn.ptr == 0) return false;
    const sp: [*]const u8 = @ptrFromInt(sn.ptr);
    // ``arr(key)`` shapes are never const-flagged — bail.
    var k: u32 = 0;
    while (k < sn.len) : (k += 1) {
        if (sp[k] == '(') return false;
    }
    // Proc-local lookup: check the per-frame const set when a frame
    // is active and the name has no namespace qualifier (qualified
    // names always resolve through the ns variable table).
    if (frames.frame_depth != 0) {
        var has_colons = false;
        var i: u32 = 0;
        while (i + 1 < sn.len) : (i += 1) {
            if (sp[i] == ':' and sp[i + 1] == ':') {
                has_colons = true;
                break;
            }
        }
        if (!has_colons) {
            const ht_mod = @import("../valtypes/hash_table.zig");
            const hash = ht_mod.fnv1a(sn.ptr, sn.len);
            if (frames.frame_const_check(sn.ptr, sn.len, hash)) return true;
            // A frame-local non-const slot shadows the namespace.
            const exists_obj = frames.var_exists(name);
            if (tcl_obj_mod.obj_get_int(exists_obj) != 0) return false;
        }
    }
    // Namespace probe: reach the ``Var`` and inspect its flags.
    // Qualify against current_ns the same way the read / write
    // paths in ``var_resolve`` / ``var_set`` do.
    var key_ptr = sn.ptr;
    var key_len = sn.len;
    if (sn.len >= 2 and sp[0] == ':' and sp[1] == ':') {
        key_ptr = sn.ptr + 2;
        key_len = sn.len - 2;
    } else if (frames.frame_depth == 0 and tcl_ns.current_ns != 0 and tcl_ns.current_ns != tcl_ns.ns_root()) {
        const ns_full_ptr = tcl_ns.current_ns_full_ptr();
        const ns_full_len = tcl_ns.current_ns_full_len();
        var nfp = ns_full_ptr;
        var nfl = ns_full_len;
        if (nfl >= 2) {
            const nsp: [*]const u8 = @ptrFromInt(nfp);
            if (nsp[0] == ':' and nsp[1] == ':') {
                nfp += 2;
                nfl -= 2;
            }
        }
        if (nfl > 0) {
            const total: u32 = nfl + 2 + sn.len;
            const buf = tcl_obj_mod.alloc(total);
            if (buf == 0) return false;
            const dst: [*]u8 = @ptrFromInt(buf);
            const nsp: [*]const u8 = @ptrFromInt(nfp);
            for (0..nfl) |i| dst[i] = nsp[i];
            dst[nfl] = ':';
            dst[nfl + 1] = ':';
            for (0..sn.len) |i| dst[nfl + 2 + i] = sp[i];
            const v_addr = tcl_ns.ns_var_find(tcl_ns.ns_root(), buf, total);
            tcl_obj_mod.free_sized(buf, total);
            if (v_addr == 0) return false;
            const v: *const tcl_ns.Var = @ptrFromInt(v_addr);
            return (v.flags & tcl_ns.VAR_CONSTANT) != 0;
        }
    }
    const v_addr = tcl_ns.ns_var_find(tcl_ns.ns_root(), key_ptr, key_len);
    if (v_addr == 0) return false;
    const v: *const tcl_ns.Var = @ptrFromInt(v_addr);
    return (v.flags & tcl_ns.VAR_CONSTANT) != 0;
}

/// Emit the canonical ``can't <op> "<name>": variable is a constant``
/// diagnostic for an attempted write to a constant variable.  *op* is
/// the user-visible verb (``"set"``, ``"incr"``, ``"unset"``) the
/// dispatcher was running.
fn raise_const_set_error(name: i32, op: []const u8) void {
    const tcl_obj_mod = @import("../valtypes/tcl_obj.zig");
    const catch_mod = @import("../interp/tcl_catch.zig");
    const sn = tcl_obj_mod.obj_ensure_string(name);
    const prefix1: []const u8 = "can't ";
    const prefix2: []const u8 = " \"";
    const suffix: []const u8 = "\": variable is a constant";
    const total: u32 = @as(u32, @intCast(prefix1.len + op.len + prefix2.len)) + sn.len + @as(u32, @intCast(suffix.len));
    const buf = tcl_obj_mod.alloc(total);
    if (buf == 0) {
        catch_mod.tcl_cmd_error(tcl_obj_mod.obj_new_string(0, 0));
        return;
    }
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix1) |c| {
        dst[off] = c;
        off += 1;
    }
    for (op) |c| {
        dst[off] = c;
        off += 1;
    }
    for (prefix2) |c| {
        dst[off] = c;
        off += 1;
    }
    if (sn.len > 0) {
        const np: [*]const u8 = @ptrFromInt(sn.ptr);
        for (0..sn.len) |k| dst[off + k] = np[k];
        off += sn.len;
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const msg = tcl_obj_mod.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(msg);
}

/// Tcl 9 ``const VARNAME VALUE`` — set a variable and flag it as a
/// read-only constant.  Per :cite:`tip.590`:
///   * Re-running ``const`` on an already-constant name is a silent
///     success (the existing value is preserved).
///   * Running ``const`` on an existing *non-constant* variable
///     raises ``can't make constant "<name>": variable already exists``.
///   * Array names and array elements cannot be made constant.
///   * In a proc frame, the constness lives in the per-frame
///     ``frame_const_*`` side list (frame buckets have no flag bits).
///     At namespace scope, the namespace ``Var`` carries the
///     ``VAR_CONSTANT`` bit directly.
fn eval_const(words: []const i32) result_mod.InterpResult {
    const catch_mod = @import("../interp/tcl_catch.zig");
    const tcl_obj_mod = @import("../valtypes/tcl_obj.zig");
    if (words.len != 3) {
        const msg_text: []const u8 = "wrong # args: should be \"const varName value\"";
        const buf = rt.alloc(@intCast(msg_text.len));
        if (buf != 0) {
            const dst: [*]u8 = @ptrFromInt(buf);
            for (msg_text, 0..) |b, k| dst[k] = b;
            const msg = rt.obj_new_string_take(buf, @intCast(msg_text.len), @intCast(msg_text.len));
            catch_mod.tcl_cmd_error(msg);
        }
        return result_mod.from_globals(0);
    }
    const name = words[1];
    const value = words[2];
    const sn = obj_ensure_string(name);
    if (sn.len == 0) return result_mod.from_globals(0);
    const sp: [*]const u8 = @ptrFromInt(sn.ptr);

    // Reject array-element form ``arr(key)`` — TIP 590 only flags
    // whole scalars.  Matches the C-Tcl ISARRAYELEMENT diagnostic.
    var i: u32 = 0;
    while (i < sn.len) : (i += 1) {
        if (sp[i] == '(') {
            raise_const_array_error(sn.ptr, sn.len);
            return result_mod.from_globals(0);
        }
    }

    if (frames.frame_depth != 0) {
        // Proc-local: check the frame's const list first so a repeat
        // ``const X V`` on an already-constant name silently succeeds.
        const ht_mod = @import("../valtypes/hash_table.zig");
        const hash = ht_mod.fnv1a(sn.ptr, sn.len);
        if (frames.frame_const_check(sn.ptr, sn.len, hash)) {
            return result_mod.from_globals(value);
        }
        // If the local already exists *and* isn't a const, raise the
        // ``variable already exists`` diagnostic.  Tcl's ``info exists``
        // is the right discriminator: it gates on actual storage so an
        // ``unset``-cleared slot doesn't count.
        const exists_obj = frames.var_exists(name);
        if (rt.obj_get_int(exists_obj) != 0) {
            raise_const_exists_error(sn.ptr, sn.len);
            return result_mod.from_globals(0);
        }
        _ = frames.var_set(name, value);
        frames.frame_const_mark(sn.ptr, sn.len, hash);
        return result_mod.from_globals(value);
    }

    // No frame: namespace scope.  Resolve the storage key the way
    // ``global_set`` will and reach the ``Var`` record so the
    // VAR_CONSTANT flag can be set / checked.
    const qname = qualify_for_ns(name, sn);
    const qs = obj_ensure_string(qname);
    const stripped = strip_double_colon(qs.ptr, qs.len);
    // Reject ``const X V`` when ``X`` already exists as an array
    // at the resolved storage key.  Arrays live in the parallel
    // ``tcl_array`` directory, not the ``ns_var_find`` table, so
    // the ``Var`` probe below would miss them.
    if (tcl_array.array_exists_raw(stripped.ptr, stripped.len)) {
        raise_const_array_var_error(sn.ptr, sn.len);
        if (qname != name) tcl_obj_mod.tcl_obj_release(qname);
        return result_mod.from_globals(0);
    }
    const v_addr = tcl_ns.ns_var_find(tcl_ns.ns_root(), stripped.ptr, stripped.len);
    if (v_addr != 0) {
        const v: *const tcl_ns.Var = @ptrFromInt(v_addr);
        if ((v.flags & tcl_ns.VAR_CONSTANT) != 0) {
            if (qname != name) tcl_obj_mod.tcl_obj_release(qname);
            return result_mod.from_globals(value);
        }
        // Belt-and-braces: a ``Var`` record carrying ``VAR_ARRAY``
        // (e.g. a future direct ns-var array shape) is also a
        // non-scalar — reject just like the array-directory probe
        // above.  ``var_get_scalar`` returns 0 for arrays, which
        // would otherwise let the slot look like an undefined
        // scalar and slip into the ``promote-to-const`` branch.
        if ((v.flags & tcl_ns.VAR_ARRAY) != 0) {
            raise_const_array_var_error(sn.ptr, sn.len);
            if (qname != name) tcl_obj_mod.tcl_obj_release(qname);
            return result_mod.from_globals(0);
        }
        // Existing non-const var: error iff it has a value.  An
        // undefined slot (created by ``variable X`` with no value)
        // should be allowed to become a constant.
        if (tcl_ns.var_get_scalar(v_addr) != 0) {
            raise_const_exists_error(sn.ptr, sn.len);
            if (qname != name) tcl_obj_mod.tcl_obj_release(qname);
            return result_mod.from_globals(0);
        }
    }
    _ = tcl_ns.global_set(qname, value);
    const v2 = tcl_ns.ns_var_find(tcl_ns.ns_root(), stripped.ptr, stripped.len);
    if (v2 != 0) {
        const vp: *tcl_ns.Var = @ptrFromInt(v2);
        vp.flags |= tcl_ns.VAR_CONSTANT;
    }
    if (qname != name) tcl_obj_mod.tcl_obj_release(qname);
    return result_mod.from_globals(value);
}

/// Build the fully-qualified storage key for a bare ``name`` when no
/// frame is active.  Mirrors :func:`var_set`'s namespace-write branch:
///   * ``::``-anchored names pass through unchanged.
///   * Inside ``namespace eval ::A { ... }`` (``current_ns != root``)
///     an unqualified ``X`` becomes ``::A::X``.
///   * Root-scope unqualified names also pass through.
/// Returns either the original handle or a freshly-allocated owning
/// TclObj — caller releases the latter (``qname != name``).
fn qualify_for_ns(name: i32, sn: anytype) i32 {
    if (sn.len >= 2) {
        const sp: [*]const u8 = @ptrFromInt(sn.ptr);
        if (sp[0] == ':' and sp[1] == ':') return name;
    }
    if (tcl_ns.current_ns == 0 or tcl_ns.current_ns == tcl_ns.ns_root()) return name;
    const ns_full_ptr = tcl_ns.current_ns_full_ptr();
    const ns_full_len = tcl_ns.current_ns_full_len();
    if (ns_full_len <= 2) return name;
    const total: u32 = ns_full_len + 2 + sn.len;
    const buf = rt.alloc(total);
    if (buf == 0) return name;
    const dst: [*]u8 = @ptrFromInt(buf);
    const ns_p: [*]const u8 = @ptrFromInt(ns_full_ptr);
    for (0..ns_full_len) |k| dst[k] = ns_p[k];
    dst[ns_full_len] = ':';
    dst[ns_full_len + 1] = ':';
    const name_p: [*]const u8 = @ptrFromInt(sn.ptr);
    for (0..sn.len) |k| dst[ns_full_len + 2 + k] = name_p[k];
    return rt.obj_new_string_take(buf, total, total);
}

/// Strip a leading ``::`` so the result matches the flat-key form the
/// namespace ``var_table`` uses.  Pure span manipulation.
const Span = struct { ptr: u32, len: u32 };
fn strip_double_colon(ptr: u32, len: u32) Span {
    if (len >= 2) {
        const sp: [*]const u8 = @ptrFromInt(ptr);
        if (sp[0] == ':' and sp[1] == ':') return .{ .ptr = ptr + 2, .len = len - 2 };
    }
    return .{ .ptr = ptr, .len = len };
}

fn raise_const_exists_error(name_ptr: u32, name_len: u32) void {
    const catch_mod = @import("../interp/tcl_catch.zig");
    const prefix: []const u8 = "can't make constant \"";
    const suffix: []const u8 = "\": variable already exists";
    const total: u32 = @as(u32, @intCast(prefix.len)) + name_len + @as(u32, @intCast(suffix.len));
    const buf = rt.alloc(total);
    if (buf == 0) return;
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    const np: [*]const u8 = @ptrFromInt(name_ptr);
    for (0..name_len) |k| {
        dst[off] = np[k];
        off += 1;
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const msg = rt.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(msg);
}

fn raise_const_array_var_error(name_ptr: u32, name_len: u32) void {
    const catch_mod = @import("../interp/tcl_catch.zig");
    const prefix: []const u8 = "can't make constant \"";
    const suffix: []const u8 = "\": variable is array";
    const total: u32 = @as(u32, @intCast(prefix.len)) + name_len + @as(u32, @intCast(suffix.len));
    const buf = rt.alloc(total);
    if (buf == 0) return;
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    const np: [*]const u8 = @ptrFromInt(name_ptr);
    for (0..name_len) |k| {
        dst[off] = np[k];
        off += 1;
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const msg = rt.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(msg);
}

fn raise_const_array_error(name_ptr: u32, name_len: u32) void {
    const catch_mod = @import("../interp/tcl_catch.zig");
    const prefix: []const u8 = "can't make constant \"";
    const suffix: []const u8 = "\": name refers to an element in an array";
    const total: u32 = @as(u32, @intCast(prefix.len)) + name_len + @as(u32, @intCast(suffix.len));
    const buf = rt.alloc(total);
    if (buf == 0) return;
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    const np: [*]const u8 = @ptrFromInt(name_ptr);
    for (0..name_len) |k| {
        dst[off] = np[k];
        off += 1;
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const msg = rt.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(msg);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "set", .arity_min = 1, .arity_max = 2, .handler = &eval_set },
    .{ .name = "incr", .arity_min = 1, .arity_max = 2, .handler = &eval_incr },
    .{ .name = "unset", .arity_min = 1, .arity_max = null, .handler = &eval_unset },
    .{ .name = "const", .arity_min = 2, .arity_max = 2, .handler = &eval_const },
};
