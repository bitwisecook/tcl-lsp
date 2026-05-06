// ``info``, ``trace`` — introspection commands.

const rt = @import("../tcl_runtime.zig");
const result_mod = @import("../interp/tcl_result.zig");
const info = @import("./tcl_cmd_info.zig");
const trace_mod = @import("../interp/tcl_trace.zig");
const var_trace = @import("../interp/tcl_var_trace.zig");
const frames = @import("../interp/tcl_frames.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");

/// Phase 6 follow-up: classify ``NAME`` for ``trace add/remove/info
/// variable``.  Proc-local traces go on the per-frame chain (so two
/// distinct procs with same-named locals don't share traces) iff:
///   * a frame is active,
///   * the name has no ``::`` qualifier (which would force the
///     global / namespace path),
///   * the name has no ``(`` array-element notation (array-element
///     traces flow through the array directory unchanged).
/// Returns true when the per-frame chain owns the trace.
fn is_local_trace_target(name: i32) bool {
    if (frames.frame_depth == 0) return false;
    const sn = rt.obj_ensure_string(name);
    if (sn.len == 0 or sn.ptr == 0) return false;
    const sp: [*]const u8 = @ptrFromInt(sn.ptr);
    if (sn.len >= 2 and sp[0] == ':' and sp[1] == ':') return false;
    var k: u32 = 0;
    while (k < sn.len) : (k += 1) {
        if (sp[k] == ':' and k + 1 < sn.len and sp[k + 1] == ':') return false;
        if (sp[k] == '(') return false;
    }
    return true;
}

const str_eq = @import("../valtypes/tcl_chars.zig").str_eq;
const obj_new_string = rt.obj_new_string;
const obj_new_string_take = rt.obj_new_string_take;
const tcl_obj_release = @import("../valtypes/tcl_obj.zig").tcl_obj_release;
const obj_ensure_string = rt.obj_ensure_string;

fn eval_info(words: []const i32) result_mod.InterpResult {
    if (words.len >= 2) {
        const sub_s = obj_ensure_string(words[1]);
        const sub_p: [*]const u8 = @ptrFromInt(sub_s.ptr);
        if (str_eq(sub_p, sub_s.len, "default")) {
            if (words.len < 5) return result_mod.from_globals(obj_new_string(0, 0));
            return result_mod.from_globals(info.info_default(words[2], words[3], words[4]));
        }
        if (str_eq(sub_p, sub_s.len, "commands") and words.len == 2) {
            return result_mod.from_globals(info.info_commands(0));
        }
        if (str_eq(sub_p, sub_s.len, "procs") and words.len == 2) {
            return result_mod.from_globals(info.info_procs(0));
        }
        if (words.len == 2) return result_mod.from_globals(info.info_dispatch(words[1], 0));
    }
    if (words.len >= 3) return result_mod.from_globals(info.info_dispatch(words[1], words[2]));
    return result_mod.from_globals(obj_new_string(0, 0));
}

fn eval_trace(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) {
        return result_mod.from_globals(obj_new_string(0, 0));
    }
    const sub_s = obj_ensure_string(words[1]);
    const sub_p: [*]const u8 = @ptrFromInt(sub_s.ptr);
    // ``trace add variable NAME OPS CMD`` — phase 6 implementation.
    if (str_eq(sub_p, sub_s.len, "add") and words.len >= 6) {
        const kind_s = obj_ensure_string(words[2]);
        const kind_p: [*]const u8 = @ptrFromInt(kind_s.ptr);
        if (str_eq(kind_p, kind_s.len, "variable")) {
            const name = words[3];
            const ops = parse_ops(words[4]);
            install_var_trace(name, ops, words[5]);
            return result_mod.from_globals(obj_new_string(0, 0));
        }
        // execution / command tracing — pass-through NOP.
        return result_mod.from_globals(obj_new_string(0, 0));
    }
    // ``trace remove variable NAME OPS CMD``.
    if (str_eq(sub_p, sub_s.len, "remove") and words.len >= 6) {
        const kind_s = obj_ensure_string(words[2]);
        const kind_p: [*]const u8 = @ptrFromInt(kind_s.ptr);
        if (str_eq(kind_p, kind_s.len, "variable")) {
            const name = words[3];
            const ops = parse_ops(words[4]);
            _ = uninstall_var_trace(name, ops, words[5]);
            return result_mod.from_globals(obj_new_string(0, 0));
        }
        return result_mod.from_globals(obj_new_string(0, 0));
    }
    // ``trace info variable NAME``.
    if (str_eq(sub_p, sub_s.len, "info") and words.len >= 4) {
        const kind_s = obj_ensure_string(words[2]);
        const kind_p: [*]const u8 = @ptrFromInt(kind_s.ptr);
        if (str_eq(kind_p, kind_s.len, "variable")) {
            return result_mod.from_globals(query_var_trace(words[3]));
        }
        return result_mod.from_globals(obj_new_string(0, 0));
    }
    // Legacy forms: ``trace variable NAME OPS CMD`` /
    // ``trace vdelete NAME OPS CMD`` / ``trace vinfo NAME``.
    if (str_eq(sub_p, sub_s.len, "variable") and words.len >= 5) {
        install_var_trace(words[2], parse_ops(words[3]), words[4]);
        return result_mod.from_globals(obj_new_string(0, 0));
    }
    if (str_eq(sub_p, sub_s.len, "vdelete") and words.len >= 5) {
        _ = uninstall_var_trace(words[2], parse_ops(words[3]), words[4]);
        return result_mod.from_globals(obj_new_string(0, 0));
    }
    if (str_eq(sub_p, sub_s.len, "vinfo") and words.len >= 3) {
        return result_mod.from_globals(query_var_trace(words[2]));
    }
    // Anything else: defer to the legacy NOP module so we don't trap
    // tests that rely on pass-through behaviour.
    const sub = if (words.len >= 2) words[1] else 0;
    const arg_obj = if (words.len >= 3) words[2] else 0;
    return result_mod.from_globals(trace_mod.tcl_cmd_trace_cmd(sub, arg_obj));
}

/// Resolve a user-supplied variable name to the canonical FQ form
/// the trace directory keys against.  Mirrors what reference Tcl
/// does when ``trace add variable NAME …`` runs inside a
/// ``namespace eval`` body: an unqualified ``NAME`` resolves
/// against the active namespace, so a later read via
/// ``$::ns::NAME`` (or via a proc-local ``variable NAME`` alias)
/// fires the trace.  Without this, ``trace add variable a …``
/// inside ``namespace eval ::ns { … }`` registered against a bare
/// ``a`` while the array module looked up traces under
/// ``::ns::a`` — so ``tcltest::SafeFetch`` never fired and
/// ``$testConstraints(valgrind)`` returned ``""`` instead of
/// triggering the lazy initialiser, which our PR #341 strict-
/// boolean ``!`` then rejected (regression that emptied
/// ``string.test`` and friends).
///
/// Rules (in order):
///   1. Names starting with ``::`` pass through unchanged.
///   2. Names containing ``::`` but not starting with it get a
///      leading ``::`` so ``ns::var`` becomes ``::ns::var``;
///      mirrors :func:`tcl_array.normalize_ns_name`.
///   3. Bare names at top level inside a non-root namespace
///      (``frame_depth == 0`` and ``current_ns_full_len > 2``)
///      get prefixed with ``<current_ns>::``.
///   4. Anything else passes through unchanged.
fn canonical_var_name(name: i32) i32 {
    const sn = obj_ensure_string(name);
    if (sn.ptr == 0 or sn.len == 0) return name;
    const sp: [*]const u8 = @ptrFromInt(sn.ptr);
    if (sn.len >= 2 and sp[0] == ':' and sp[1] == ':') return name;
    // Probe for an internal ``::`` — if present, ``ns::var`` form;
    // prepend ``::`` to FQ-ify per :func:`tcl_array.normalize_ns_name`.
    var i: u32 = 0;
    while (i + 1 < sn.len) : (i += 1) {
        if (sp[i] == ':' and sp[i + 1] == ':') {
            const total: u32 = sn.len + 2;
            const buf = rt.alloc(total);
            if (buf == 0) return name;
            const dst: [*]u8 = @ptrFromInt(buf);
            dst[0] = ':';
            dst[1] = ':';
            for (0..sn.len) |k| dst[2 + k] = sp[k];
            // ``obj_new_string_take`` so the freshly-allocated buffer
            // is owned by the returned TclObj — releasing the obj
            // reclaims the bytes.  The earlier ``obj_new_string``
            // form left ``OBJ_STR_CAP=0`` and leaked the buffer
            // (Copilot review on PR #343).
            return obj_new_string_take(buf, total, total);
        }
    }
    // Bare unqualified name — only top-level (frame_depth == 0)
    // calls reach here, since ``is_local_trace_target`` already
    // routed proc-local bare-name traces to the per-frame chain.
    // When a ``namespace eval`` body is active, qualify against
    // the running namespace so the canonical-name dir matches the
    // ``::ns::name`` form used by the array fire path and by
    // ``variable``-aliased proc-local reads.
    if (frames.frame_depth != 0) return name;
    const tcl_ns = @import("../interp/tcl_ns.zig");
    const ns_ptr = tcl_ns.current_ns_full_ptr();
    const ns_len = tcl_ns.current_ns_full_len();
    if (ns_len <= 2) return name; // root namespace ``::`` — no qualification
    const total: u32 = ns_len + 2 + sn.len;
    const buf = rt.alloc(total);
    if (buf == 0) return name;
    const dst: [*]u8 = @ptrFromInt(buf);
    const ns_p: [*]const u8 = @ptrFromInt(ns_ptr);
    for (0..ns_len) |k| dst[k] = ns_p[k];
    dst[ns_len] = ':';
    dst[ns_len + 1] = ':';
    for (0..sn.len) |k| dst[ns_len + 2 + k] = sp[k];
    return obj_new_string_take(buf, total, total);
}

/// Phase 6 follow-up: route a trace install to the per-frame chain
/// when the target is a proc-local, otherwise to the canonical-name
/// directory.  Centralised so all ``trace add variable`` /
/// ``trace variable`` entry points stay in sync.
///
/// :func:`canonical_var_name` may return either the original
/// *name* TclObj (no qualification needed) or a freshly-allocated
/// owning TclObj.  Release the canonical handle when it's a fresh
/// alloc (i.e., differs from the input handle) so the
/// canonicalisation buffer doesn't leak — ``var_trace.add`` /
/// ``remove`` / ``info`` only read the bytes through the handle
/// and don't retain it (Copilot review on PR #343).
fn install_var_trace(name: i32, ops: u32, cmd_prefix: i32) void {
    if (is_local_trace_target(name)) {
        var_trace.add_to_list(&frames.frame_trace_heads[frames.frame_depth - 1], name, ops, cmd_prefix);
        return;
    }
    const canon = canonical_var_name(name);
    var_trace.add(canon, ops, cmd_prefix);
    if (canon != name and canon != 0) tcl_obj_release(canon);
}

/// Mirror of :func:`install_var_trace` for the remove path.
fn uninstall_var_trace(name: i32, ops: u32, cmd_prefix: i32) bool {
    if (is_local_trace_target(name)) {
        return var_trace.remove_from_list(&frames.frame_trace_heads[frames.frame_depth - 1], name, ops, cmd_prefix);
    }
    const canon = canonical_var_name(name);
    const result = var_trace.remove(canon, ops, cmd_prefix);
    if (canon != name and canon != 0) tcl_obj_release(canon);
    return result;
}

/// Mirror of :func:`install_var_trace` for the ``trace info`` query.
fn query_var_trace(name: i32) i32 {
    if (is_local_trace_target(name)) {
        return var_trace.info_for_list(frames.frame_trace_heads[frames.frame_depth - 1], name);
    }
    const canon = canonical_var_name(name);
    const result = var_trace.info(canon);
    if (canon != name and canon != 0) tcl_obj_release(canon);
    return result;
}

/// Parse the ops list ``{read write unset array}`` (any subset, in
/// any order — Tcl is lenient) into the OP_* bitmask.
fn parse_ops(ops_obj: i32) u32 {
    if (ops_obj == 0) return 0;
    const so = obj_ensure_string(ops_obj);
    const n = rt.list_count_elements(so.ptr, so.len);
    if (n <= 0) return 0;
    var mask: u32 = 0;
    var i: i64 = 0;
    while (i < n) : (i += 1) {
        const e = rt.list_element_at(so.ptr, so.len, i);
        const ep: [*]const u8 = @ptrFromInt(so.ptr + e.start);
        if (str_eq(ep, e.len, "read")) mask |= var_trace.OP_READ;
        if (str_eq(ep, e.len, "write")) mask |= var_trace.OP_WRITE;
        if (str_eq(ep, e.len, "unset")) mask |= var_trace.OP_UNSET;
        if (str_eq(ep, e.len, "array")) mask |= var_trace.OP_ARRAY;
    }
    return mask;
}

fn eval_pid(words: []const i32) result_mod.InterpResult {
    _ = words;
    return result_mod.from_globals(rt.obj_new_int(12345));
}

/// Stub for ``tcl::build-info`` — returns a hard-coded string for the
/// known string queries (``patchlevel``, ``version``, ``commit``,
/// ``compiler``).  Reference Tcl 9 (``BuildInfoObjCmd2`` in
/// ``generic/tclBasic.c``) treats any other identifier as a boolean
/// presence check against the build-info string and returns
/// ``Tcl_NewBooleanObj(0)`` when the flag is absent — so callers like
/// ``tcltests.tcl`` can write ``expr {![tcl::build-info no-deprecate]}``
/// without tripping strict-boolean rules.  Our WASM build claims none of
/// the optional flags (``no-deprecate``, ``debug``, ``purify``,
/// ``memdebug``, …), so unknown keys must return ``0``, not ``""``.
fn eval_tcl_build_info(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) {
        return result_mod.from_globals(obj_new_string_lit("9.0.3"));
    }
    const sub = obj_ensure_string(words[1]);
    const sp: [*]const u8 = @ptrFromInt(sub.ptr);
    if (str_eq(sp, sub.len, "patchlevel")) return result_mod.from_globals(obj_new_string_lit("9.0.3"));
    if (str_eq(sp, sub.len, "version")) return result_mod.from_globals(obj_new_string_lit("9.0"));
    if (str_eq(sp, sub.len, "commit")) return result_mod.from_globals(obj_new_string_lit("0000000000000000000000000000000000000000"));
    if (str_eq(sp, sub.len, "compiler")) return result_mod.from_globals(obj_new_string_lit("zig-wasm32"));
    // Unknown sub-key: boolean presence check.  None of the optional
    // build flags are set in our runtime, so always answer ``0``.
    return result_mod.from_globals(obj_new_string_lit("0"));
}

/// Helper: allocate a TclObj wrapping a Zig string literal.  The
/// literal lives in the wasm data segment, so its bytes are stable
/// for the lifetime of the module — we point ``OBJ_STR_PTR`` at
/// them with ``OBJ_STR_CAP == 0`` (not owned, not freeable).
fn obj_new_string_lit(comptime s: []const u8) i32 {
    return obj_new_string(@bitCast(@intFromPtr(s.ptr)), @bitCast(s.len));
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "info", .arity_min = 1, .arity_max = null, .handler = &eval_info },
    .{ .name = "trace", .arity_min = 1, .arity_max = null, .handler = &eval_trace },
    .{ .name = "pid", .arity_min = 0, .arity_max = 1, .handler = &eval_pid },
    .{ .name = "tcl::build-info", .arity_min = 0, .arity_max = 1, .handler = &eval_tcl_build_info },
    // Also register without the leading ``tcl::`` — some callers
    // import it bare.
    .{ .name = "::tcl::build-info", .arity_min = 0, .arity_max = 1, .handler = &eval_tcl_build_info },
};

// ``info <sub>`` sub-commands — mirrors
// ``core/commands/registry/tcl/info.py``.  Cross-checked against
// ``generic/tclCmdIL.c`` (most handlers enforce ``objc != N`` or
// ``objc < A || objc > B`` directly; the ``TclCompileBasic*ArgCmd``
// entries in ``infoImplMap`` imply the arity for the remainder).
pub const info_subcommands: []const reg.SubEntry = &.{
    .{ .name = "args", .arity_min = 1, .arity_max = 1, .handler = &eval_info },
    .{ .name = "body", .arity_min = 1, .arity_max = 1, .handler = &eval_info },
    .{ .name = "class", .arity_min = 2, .arity_max = null, .handler = &eval_info },
    .{ .name = "cmdcount", .arity_min = 0, .arity_max = 0, .handler = &eval_info },
    .{ .name = "cmdtype", .arity_min = 1, .arity_max = 1, .handler = &eval_info },
    .{ .name = "commands", .arity_min = 0, .arity_max = 1, .handler = &eval_info },
    .{ .name = "complete", .arity_min = 1, .arity_max = 1, .handler = &eval_info },
    .{ .name = "constant", .arity_min = 1, .arity_max = 1, .handler = &eval_info },
    .{ .name = "consts", .arity_min = 0, .arity_max = 1, .handler = &eval_info },
    .{ .name = "coroutine", .arity_min = 0, .arity_max = 0, .handler = &eval_info },
    .{ .name = "default", .arity_min = 3, .arity_max = 3, .handler = &eval_info },
    .{ .name = "errorstack", .arity_min = 0, .arity_max = 1, .handler = &eval_info },
    .{ .name = "exists", .arity_min = 1, .arity_max = 1, .handler = &eval_info },
    .{ .name = "frame", .arity_min = 0, .arity_max = 1, .handler = &eval_info },
    .{ .name = "functions", .arity_min = 0, .arity_max = 1, .handler = &eval_info },
    .{ .name = "globals", .arity_min = 0, .arity_max = 1, .handler = &eval_info },
    .{ .name = "hostname", .arity_min = 0, .arity_max = 0, .handler = &eval_info },
    .{ .name = "level", .arity_min = 0, .arity_max = 1, .handler = &eval_info },
    .{ .name = "library", .arity_min = 0, .arity_max = 0, .handler = &eval_info },
    .{ .name = "loaded", .arity_min = 0, .arity_max = 2, .handler = &eval_info },
    .{ .name = "locals", .arity_min = 0, .arity_max = 1, .handler = &eval_info },
    .{ .name = "nameofexecutable", .arity_min = 0, .arity_max = 0, .handler = &eval_info },
    .{ .name = "object", .arity_min = 2, .arity_max = null, .handler = &eval_info },
    .{ .name = "patchlevel", .arity_min = 0, .arity_max = 0, .handler = &eval_info },
    .{ .name = "procs", .arity_min = 0, .arity_max = 1, .handler = &eval_info },
    .{ .name = "script", .arity_min = 0, .arity_max = 1, .handler = &eval_info },
    .{ .name = "sharedlibextension", .arity_min = 0, .arity_max = 0, .handler = &eval_info },
    .{ .name = "tclversion", .arity_min = 0, .arity_max = 0, .handler = &eval_info },
    .{ .name = "vars", .arity_min = 0, .arity_max = 1, .handler = &eval_info },
};

// ``trace <sub>`` sub-commands — mirrors
// ``core/commands/registry/tcl/trace.py``.
pub const trace_subcommands: []const reg.SubEntry = &.{
    .{ .name = "add", .arity_min = 4, .arity_max = 4, .handler = &eval_trace },
    .{ .name = "info", .arity_min = 2, .arity_max = 2, .handler = &eval_trace },
    .{ .name = "remove", .arity_min = 4, .arity_max = 4, .handler = &eval_trace },
    .{ .name = "variable", .arity_min = 3, .arity_max = 3, .handler = &eval_trace },
    .{ .name = "vdelete", .arity_min = 3, .arity_max = 3, .handler = &eval_trace },
    .{ .name = "vinfo", .arity_min = 1, .arity_max = 1, .handler = &eval_trace },
};
