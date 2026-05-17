// ``namespace`` — namespace management command (eval, export, import, forget,
// which, current, path, qualifiers, tail, parent, exists, children subcommands).

const rt = @import("../tcl_runtime.zig");
const result_mod = @import("../interp/tcl_result.zig");
const tcl_ns = @import("../interp/tcl_ns.zig");
const procs = @import("../interp/tcl_procs.zig");
const interp_impl = @import("./tcl_cmd_interp.zig");
const obj_mod = @import("../valtypes/tcl_obj.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");
const tcl_string = @import("../valtypes/tcl_string.zig");

const str_eq = @import("../valtypes/tcl_chars.zig").str_eq;
const alloc = rt.alloc;
const memcpy = rt.memcpy;
const obj_new_string = rt.obj_new_string;
const obj_new_string_copy = rt.obj_new_string_copy;
const obj_ensure_string = rt.obj_ensure_string;
const obj_new_int = rt.obj_new_int;
const obj_get_int = rt.obj_get_int;

/// Walk *name* (a relative ``::``-separated namespace path) starting
/// from *parent* and create each missing component.  Used by
/// ``namespace eval`` for unqualified targets so ``namespace eval
/// foo {…}`` inside a ``::a::`` body creates ``::a::foo`` rather
/// than ``::foo`` under the root.
fn ns_create_relative(parent: u32, name_ptr: u32, name_len: u32) u32 {
    if (name_len == 0) return parent;
    // Defensive: when the current ns hasn't been initialised yet
    // (early bootstrap, or top-level eval without a frame),
    // fall back to the root so we don't dereference a null
    // ns pointer in ``ns_create``.
    var ns: u32 = if (parent == 0) tcl_ns.ns_root() else parent;
    const src: [*]const u8 = @ptrFromInt(name_ptr);
    var i: u32 = 0;
    while (i < name_len) {
        var j: u32 = i;
        while (j < name_len and src[j] != ':') : (j += 1) {}
        const comp_len: u32 = j - i;
        if (comp_len > 0) {
            ns = tcl_ns.ns_create(ns, name_ptr + i, comp_len);
        }
        i = j;
        while (i < name_len and src[i] == ':') : (i += 1) {}
    }
    return ns;
}

/// Raise a Tcl-level error with *msg* via the catch module.  Used by
/// the dispatch front-door to surface ``wrong # args`` and ``unknown
/// or ambiguous subcommand`` errors that the body branches don't
/// otherwise produce.
fn raise_ns_error(msg: []const u8) void {
    const catch_mod = @import("../interp/tcl_catch.zig");
    const buf = alloc(@intCast(msg.len));
    if (buf == 0) {
        catch_mod.tcl_cmd_error(obj_mod.obj_new_string(0, 0));
        return;
    }
    const dst: [*]u8 = @ptrFromInt(buf);
    for (msg, 0..) |b, i| dst[i] = b;
    const e = obj_mod.obj_new_string_take(buf, @intCast(msg.len), @intCast(msg.len));
    catch_mod.tcl_cmd_error(e);
}

/// Canonical Tcl 9 ``namespace`` arity / dispatch errors.
const NamespaceArity = struct {
    sub: []const u8,
    min_args: u32, // additional args beyond ``namespace SUB`` (i.e. words.len - 2)
    max_args: ?u32, // null = unbounded
    message: []const u8,
};

const ns_arity_table: []const NamespaceArity = &.{
    .{ .sub = "children", .min_args = 0, .max_args = 2, .message = "wrong # args: should be \"namespace children ?name? ?pattern?\"" },
    .{ .sub = "code", .min_args = 1, .max_args = 1, .message = "wrong # args: should be \"namespace code arg\"" },
    .{ .sub = "current", .min_args = 0, .max_args = 0, .message = "wrong # args: should be \"namespace current\"" },
    .{ .sub = "ensemble", .min_args = 1, .max_args = null, .message = "wrong # args: should be \"namespace ensemble subcommand ?arg ...?\"" },
    .{ .sub = "eval", .min_args = 2, .max_args = null, .message = "wrong # args: should be \"namespace eval name arg ?arg...?\"" },
    .{ .sub = "exists", .min_args = 1, .max_args = 1, .message = "wrong # args: should be \"namespace exists name\"" },
    .{ .sub = "inscope", .min_args = 2, .max_args = null, .message = "wrong # args: should be \"namespace inscope name arg ?arg...?\"" },
    .{ .sub = "origin", .min_args = 1, .max_args = 1, .message = "wrong # args: should be \"namespace origin name\"" },
    .{ .sub = "parent", .min_args = 0, .max_args = 1, .message = "wrong # args: should be \"namespace parent ?name?\"" },
    .{ .sub = "qualifiers", .min_args = 1, .max_args = 1, .message = "wrong # args: should be \"namespace qualifiers string\"" },
    .{ .sub = "tail", .min_args = 1, .max_args = 1, .message = "wrong # args: should be \"namespace tail string\"" },
    .{ .sub = "which", .min_args = 1, .max_args = 2, .message = "wrong # args: should be \"namespace which ?-command? ?-variable? name\"" },
};

inline fn slice_eq_ns(p: u32, plen: u32, lit: []const u8) bool {
    if (plen != lit.len) return false;
    const sp: [*]const u8 = @ptrFromInt(p);
    for (lit, 0..) |c, i| if (sp[i] != c) return false;
    return true;
}

/// Validate the call's arity against the per-subcommand rule.  Raises
/// ``wrong # args`` and returns false on a violation; returns true
/// otherwise.  Subcommands not in the table fall through to the
/// dispatch body's own validation.
fn check_ns_arity(words: []const i32) bool {
    if (words.len < 2) {
        raise_ns_error("wrong # args: should be \"namespace subcommand ?arg ...?\"");
        return false;
    }
    const sub = obj_ensure_string(words[1]);
    if (sub.len == 0) return true;
    const extra: u32 = @intCast(words.len - 2);
    for (ns_arity_table) |rule| {
        if (slice_eq_ns(sub.ptr, sub.len, rule.sub)) {
            if (extra < rule.min_args) {
                raise_ns_error(rule.message);
                return false;
            }
            if (rule.max_args) |mx| {
                if (extra > mx) {
                    raise_ns_error(rule.message);
                    return false;
                }
            }
            return true;
        }
    }
    return true;
}

fn eval_namespace(words: []const i32) result_mod.InterpResult {
    if (!check_ns_arity(words)) return result_mod.from_globals(0);
    if (words.len >= 2) {
        const interp = @import("../interp/tcl_interp.zig");
        const sub = obj_ensure_string(words[1]);
        if (sub.len == 4 and sub.ptr != 0) {
            const sp: [*]const u8 = @ptrFromInt(sub.ptr);
            if (sp[0] == 'e' and sp[1] == 'v' and sp[2] == 'a' and sp[3] == 'l') {
                if (words.len < 4) return result_mod.from_globals(0);
                const ns_obj_s = obj_ensure_string(words[2]);
                // Resolve the namespace name relative to the
                // *current* namespace when it isn't FQ-anchored.
                // Tcl 9 semantics: ``namespace eval foo`` inside a
                // ``::a::`` body creates ``::a::foo`` (a child of the
                // current ns).  ``namespace eval ::foo`` is absolute
                // and creates a child of the root.  The previous
                // wiring always anchored at root, so nested
                // ``namespace eval`` chains lost the parent ns when
                // dispatched through a multi-level uplevel chain.
                const target_ns = if (ns_obj_s.len >= 2 and @as([*]const u8, @ptrFromInt(ns_obj_s.ptr))[0] == ':' and @as([*]const u8, @ptrFromInt(ns_obj_s.ptr))[1] == ':')
                    tcl_ns.ns_create_from_fqn(ns_obj_s.ptr, ns_obj_s.len)
                else
                    ns_create_relative(tcl_ns.current_ns, ns_obj_s.ptr, ns_obj_s.len);
                const saved_ns = tcl_ns.current_ns;
                tcl_ns.current_ns = target_ns;
                defer tcl_ns.current_ns = saved_ns;
                if (words.len == 4) {
                    const bs = obj_ensure_string(words[3]);
                    if (bs.len > 0) return result_mod.from_globals(interp.eval_script(bs.ptr, bs.len));
                    return result_mod.from_globals(0);
                }
                var total: u32 = 0;
                var wi: u32 = 3;
                while (wi < words.len) : (wi += 1) {
                    const ws = obj_ensure_string(words[wi]);
                    total += ws.len;
                    if (wi + 1 < words.len) total += 1;
                }
                const buf = alloc(total);
                var off: u32 = 0;
                wi = 3;
                while (wi < words.len) : (wi += 1) {
                    const ws = obj_ensure_string(words[wi]);
                    memcpy(buf + off, ws.ptr, ws.len);
                    off += ws.len;
                    if (wi + 1 < words.len) {
                        const d: [*]u8 = @ptrFromInt(buf + off);
                        d[0] = ' ';
                        off += 1;
                    }
                }
                return result_mod.from_globals(interp.eval_script(buf, total));
            }
        }
        if (sub.len == 6 and sub.ptr != 0) {
            const sp6: [*]const u8 = @ptrFromInt(sub.ptr);
            if (sp6[0] == 'e' and sp6[1] == 'x' and sp6[2] == 'p' and sp6[3] == 'o' and sp6[4] == 'r' and sp6[5] == 't') {
                var pi: u32 = 2;
                while (pi < words.len) : (pi += 1) {
                    const ps = obj_ensure_string(words[pi]);
                    if (ps.len == 6 and ps.ptr != 0) {
                        const psp: [*]const u8 = @ptrFromInt(ps.ptr);
                        if (psp[0] == '-' and psp[1] == 'c' and psp[2] == 'l' and psp[3] == 'e' and psp[4] == 'a' and psp[5] == 'r') continue;
                    }
                    // Tcl 9 ``Tcl_ExportCmd`` rejects patterns that
                    // include namespace qualifiers — exports are
                    // always relative to the current namespace
                    // (namespace-26.3).
                    if (export_pattern_has_ns(ps.ptr, ps.len)) {
                        raise_invalid_export_pattern(ps.ptr, ps.len);
                        return result_mod.from_globals(0);
                    }
                    tcl_ns.ns_export(tcl_ns.ns_current(), ps.ptr, ps.len);
                }
                return result_mod.from_globals(0);
            }
            if (sp6[0] == 'i' and sp6[1] == 'm' and sp6[2] == 'p' and sp6[3] == 'o' and sp6[4] == 'r' and sp6[5] == 't') {
                const catch_mod = @import("../interp/tcl_catch.zig");
                var ii: u32 = 2;
                while (ii < words.len) : (ii += 1) {
                    const is = obj_ensure_string(words[ii]);
                    if (is.len == 6 and is.ptr != 0) {
                        const isp: [*]const u8 = @ptrFromInt(is.ptr);
                        if (isp[0] == '-' and isp[1] == 'f' and isp[2] == 'o' and isp[3] == 'r' and isp[4] == 'c' and isp[5] == 'e') continue;
                    }
                    // Validate the import pattern.  Reference Tcl 9
                    // raises specific diagnostics for two
                    // user-visible misuses (namespace-9.1 / 9.2):
                    //   * empty pattern → ``empty import pattern``
                    //   * pattern names a namespace that doesn't
                    //     exist → ``unknown namespace in import
                    //     pattern "<pat>"``.
                    if (is.len == 0) {
                        const msg_text: []const u8 = "empty import pattern";
                        const buf2 = alloc(@intCast(msg_text.len));
                        if (buf2 == 0) {
                            catch_mod.tcl_cmd_error(obj_mod.obj_new_string(0, 0));
                            return result_mod.from_globals(0);
                        }
                        const dst: [*]u8 = @ptrFromInt(buf2);
                        for (msg_text, 0..) |b, k| dst[k] = b;
                        const msg = obj_mod.obj_new_string_take(buf2, @intCast(msg_text.len), @intCast(msg_text.len));
                        catch_mod.tcl_cmd_error(msg);
                        return result_mod.from_globals(0);
                    }
                    // Locate the source namespace half of the
                    // pattern and verify it exists.  Bare names
                    // (no ``::``) are treated as patterns within
                    // the current namespace, which always exists,
                    // so the unknown-ns check only fires when the
                    // pattern is qualified.
                    const isp2: [*]const u8 = @ptrFromInt(is.ptr);
                    var last_sep: i32 = -1;
                    var k: u32 = 0;
                    while (k + 1 < is.len) : (k += 1) {
                        if (isp2[k] == ':' and isp2[k + 1] == ':') {
                            last_sep = @intCast(k);
                            k += 1; // skip second colon; loop adds the third increment
                        }
                    }
                    if (last_sep >= 0) {
                        const sep_at: u32 = @intCast(last_sep);
                        // Source ns is the bytes up to (and not
                        // including) the trailing ``::`` separator.
                        // ``::pat`` (last_sep == 0) means the
                        // pattern is anchored at root, which always
                        // exists.
                        if (sep_at > 0) {
                            const src_ns_ptr = is.ptr;
                            const src_ns_len = sep_at;
                            // Always run the BUILTINS materialiser
                            // before resolving the source ns.  Two
                            // shapes need it:
                            //   * The namespace doesn't exist yet
                            //     (``::tcl::mathop`` cold-start).
                            //   * The namespace was created by a bare
                            //     ``namespace eval ::tcl::mathop {}``
                            //     and so has an empty ``cmd_table``
                            //     even though the BUILTINS slice has
                            //     entries under that prefix.
                            // The materialiser is idempotent — it
                            // skips slots already populated as
                            // forwards and only stamps
                            // ``namespace export *`` once.  When the
                            // prefix doesn't match any BUILTINS
                            // (``::myns::foo`` etc.), it returns 0
                            // without touching the ns tree, and we
                            // fall through to ``resolve_ns``.
                            const builtin_ns = @import("../dispatch/tcl_builtin_ns.zig");
                            var target = builtin_ns.materialise(src_ns_ptr, src_ns_len);
                            if (target == 0) target = resolve_ns(src_ns_ptr, src_ns_len);
                            if (target == 0) {
                                const prefix: []const u8 = "unknown namespace in import pattern \"";
                                const suffix: []const u8 = "\"";
                                const total: u32 = @as(u32, @intCast(prefix.len)) + is.len + @as(u32, @intCast(suffix.len));
                                const buf2 = alloc(total);
                                if (buf2 == 0) {
                                    catch_mod.tcl_cmd_error(obj_mod.obj_new_string(0, 0));
                                    return result_mod.from_globals(0);
                                }
                                const dst: [*]u8 = @ptrFromInt(buf2);
                                var off2: u32 = 0;
                                for (prefix) |b| {
                                    dst[off2] = b;
                                    off2 += 1;
                                }
                                const ip: [*]const u8 = @ptrFromInt(is.ptr);
                                for (0..is.len) |kk| {
                                    dst[off2] = ip[kk];
                                    off2 += 1;
                                }
                                for (suffix) |b| {
                                    dst[off2] = b;
                                    off2 += 1;
                                }
                                const msg = obj_mod.obj_new_string_take(buf2, total, total);
                                catch_mod.tcl_cmd_error(msg);
                                return result_mod.from_globals(0);
                            }
                        }
                    }
                    // Self-import is a hard error in Tcl 9
                    // ``Tcl_Import`` (namespace-9.3): when the
                    // source namespace prefix resolves to the
                    // current namespace itself, raise ``import
                    // pattern ... tries to import from namespace
                    // ... into itself``.
                    if (last_sep >= 0) {
                        const sep_at: u32 = @intCast(last_sep);
                        const src_ns_ptr = is.ptr;
                        const src_ns_len = sep_at;
                        const target_h: u32 = if (src_ns_len == 0) tcl_ns.ns_root() else resolve_ns(src_ns_ptr, src_ns_len);
                        if (target_h != 0 and target_h == tcl_ns.ns_current()) {
                            raise_self_import(is.ptr, is.len, src_ns_ptr, src_ns_len);
                            return result_mod.from_globals(0);
                        }
                    }
                    const created = tcl_ns.ns_import(tcl_ns.ns_current(), is.ptr, is.len);
                    var bk: u32 = 0;
                    while (bk < created) : (bk += 1) procs.proc_count_bump();
                }
                return result_mod.from_globals(0);
            }
            if (sp6[0] == 'f' and sp6[1] == 'o' and sp6[2] == 'r' and sp6[3] == 'g' and sp6[4] == 'e' and sp6[5] == 't') {
                var fi: u32 = 2;
                var any_forgotten: u32 = 0;
                while (fi < words.len) : (fi += 1) {
                    const fs = obj_ensure_string(words[fi]);
                    // Tcl 9 ``Tcl_ForgetImport`` checks that the
                    // namespace prefix of each pattern names a real
                    // namespace and raises "unknown namespace in
                    // namespace forget pattern ..." otherwise
                    // (namespace-10.1 / 27.2).
                    if (!forget_pattern_ns_valid(fs.ptr, fs.len)) {
                        raise_unknown_ns_in_pattern(fs.ptr, fs.len, "namespace forget pattern");
                        return result_mod.from_globals(0);
                    }
                    any_forgotten += tcl_ns.ns_forget(tcl_ns.ns_current(), fs.ptr, fs.len);
                }
                if (any_forgotten > 0) procs.lru_invalidate_all();
                return result_mod.from_globals(0);
            }
        }
        if (str_eq(@ptrFromInt(sub.ptr), sub.len, "which")) {
            return result_mod.from_globals(interp_impl.eval_namespace_which(words));
        }
        if (str_eq(@ptrFromInt(sub.ptr), sub.len, "origin")) {
            // ``namespace origin CMD`` — return the FQN of the
            // original command CMD refers to.  For an imported
            // redirect (``CMD_IMPORTED``) walk the chain to its
            // ultimate source and return that command's stored FQN;
            // otherwise return CMD's own FQN unchanged.  Reference
            // Tcl 9 raises ``invalid command name "X"`` when the
            // lookup misses; tcltest's ``Eval`` proc relies on
            // that error rather than the silent-empty return —
            // ``[list namespace import [namespace origin
            // Replace::puts]]`` builds ``namespace import {}``
            // (empty pattern) and traps with ``empty import
            // pattern`` if the inner ``[namespace origin ...]``
            // returned an empty string.
            if (words.len < 3) return result_mod.from_globals(obj_new_string(0, 0));
            const name_obj = obj_ensure_string(words[2]);
            if (name_obj.len == 0) return result_mod.from_globals(obj_new_string(0, 0));
            const cxt = tcl_ns.ns_current();
            var cmd = tcl_ns.ns_find_command(cxt, name_obj.ptr, name_obj.len);
            if (cmd == 0) {
                // Mirror ``Tcl_NamespaceOriginCmd``: missing command
                // raises ``invalid command name "<name>"``.  Use the
                // catch_mod helper so an outer ``catch`` (e.g.
                // tcltest's per-test wrapper) absorbs it.
                const catch_mod = @import("../interp/tcl_catch.zig");
                const prefix: []const u8 = "invalid command name \"";
                const suffix: []const u8 = "\"";
                const total_len: u32 = @as(u32, @intCast(prefix.len)) + name_obj.len + @as(u32, @intCast(suffix.len));
                const buf2 = alloc(total_len);
                if (buf2 == 0) {
                    catch_mod.tcl_cmd_error(obj_mod.obj_new_string(0, 0));
                    return result_mod.from_globals(0);
                }
                const dst: [*]u8 = @ptrFromInt(buf2);
                var off: u32 = 0;
                for (prefix) |b| {
                    dst[off] = b;
                    off += 1;
                }
                const np: [*]const u8 = @ptrFromInt(name_obj.ptr);
                for (0..name_obj.len) |k| {
                    dst[off] = np[k];
                    off += 1;
                }
                for (suffix) |b| {
                    dst[off] = b;
                    off += 1;
                }
                const msg = obj_mod.obj_new_string_take(buf2, total_len, total_len);
                catch_mod.tcl_cmd_error(msg);
                return result_mod.from_globals(0);
            }
            // Walk the import chain.  Each redirect's
            // ``ImportedCmdData.real_cmd`` points at the next stage;
            // the final source has ``CMD_IMPORTED`` clear.  Bound
            // the walk to a sane depth to defang a malformed cycle
            // (shouldn't happen in well-formed state, but be
            // defensive — the walker is on the dispatch hot path
            // every time tcltest assembles a Replace::puts wrapper).
            const procs_const = tcl_ns.tcl_procs_constants;
            var depth: u32 = 0;
            while (depth < 64) : (depth += 1) {
                const flags: u32 = @bitCast(obj_mod.read_i32(cmd + procs_const.OFF_FLAGS));
                if ((flags & procs_const.CMD_IMPORTED) == 0) break;
                const desc: u32 = @bitCast(obj_mod.read_i32(cmd + procs_const.OFF_PARAMS_OBJ));
                if (desc == 0) break;
                const real_cmd: u32 = @bitCast(obj_mod.read_i32(desc));
                if (real_cmd == 0 or real_cmd == cmd) break;
                cmd = real_cmd;
            }
            return result_mod.from_globals(interp_impl.command_fqn_obj(cmd));
        }
        if (str_eq(@ptrFromInt(sub.ptr), sub.len, "current")) {
            const nf = tcl_ns.ns_full_name(tcl_ns.ns_current());
            return result_mod.from_globals(obj_new_string(@bitCast(nf.ptr), @bitCast(nf.len)));
        }
        if (str_eq(@ptrFromInt(sub.ptr), sub.len, "ensemble")) {
            return result_mod.from_globals(eval_ns_ensemble(words));
        }
        if (sub.len == 4 and sub.ptr != 0) {
            const sp4: [*]const u8 = @ptrFromInt(sub.ptr);
            if (sp4[0] == 'p' and sp4[1] == 'a' and sp4[2] == 't' and sp4[3] == 'h') {
                if (words.len < 3) return result_mod.from_globals(0);
                const ls = obj_ensure_string(words[2]);
                const count = obj_mod.list_count_elements(ls.ptr, ls.len);
                if (count == 0) {
                    tcl_ns.ns_set_path(tcl_ns.ns_current(), 0, 0);
                    return result_mod.from_globals(0);
                }
                const targets_buf = alloc(@intCast(count * 4));
                var li: i64 = 0;
                while (li < count) : (li += 1) {
                    const elt = obj_mod.list_element_at(ls.ptr, ls.len, li);
                    // ``element_at`` returns ``start`` as a byte
                    // *offset* into ``ls.ptr``, not an absolute
                    // address — same convention as the dict / inspect
                    // call sites.  Compute the absolute address before
                    // handing the bytes to anything that derefs.
                    const elt_ptr: u32 = ls.ptr + elt.start;
                    // Materialise BUILTIN-tier namespaces first so a
                    // bare ``namespace path ::tcl::mathop`` finds the
                    // populated ns regardless of whether anything
                    // else has poked it yet (idempotent for
                    // already-materialised entries; no-op for
                    // non-BUILTINS prefixes).  Mirrors the same hook
                    // in ``namespace import``.
                    const builtin_ns = @import("../dispatch/tcl_builtin_ns.zig");
                    var resolved: u32 = builtin_ns.materialise(elt_ptr, elt.len);
                    if (resolved == 0) {
                        const r = tcl_ns.ns_resolve_qualified(tcl_ns.ns_current(), elt_ptr, elt.len);
                        resolved = r.target_ns;
                        if (r.simple_len > 0 and r.target_ns != 0) {
                            const child = tcl_ns.ns_lookup(r.target_ns, r.simple_ptr, r.simple_len);
                            resolved = child;
                        }
                    }
                    obj_mod.write_i32(targets_buf + @as(u32, @intCast(li)) * 4, @bitCast(resolved));
                }
                tcl_ns.ns_set_path(tcl_ns.ns_current(), targets_buf, @intCast(count));
                procs.lru_invalidate_all();
                return result_mod.from_globals(0);
            }
            // ``namespace tail name`` — return the last simple component.
            if (sp4[0] == 't' and sp4[1] == 'a' and sp4[2] == 'i' and sp4[3] == 'l') {
                if (words.len < 3) return result_mod.from_globals(obj_new_string(0, 0));
                const ns_s = obj_ensure_string(words[2]);
                return result_mod.from_globals(ns_tail(ns_s.ptr, ns_s.len));
            }
        }
        // ``namespace qualifiers name``
        if (str_eq(@ptrFromInt(sub.ptr), sub.len, "qualifiers")) {
            if (words.len < 3) return result_mod.from_globals(obj_new_string(0, 0));
            const ns_s = obj_ensure_string(words[2]);
            return result_mod.from_globals(ns_qualifiers(ns_s.ptr, ns_s.len));
        }
        // ``namespace parent ?name?`` — return the FQN of NAME's
        // parent namespace.  Resolves NAME to a real namespace
        // first; an unknown namespace raises the canonical "namespace
        // not found" diagnostic (namespace-31.4).
        if (str_eq(@ptrFromInt(sub.ptr), sub.len, "parent")) {
            const target_h: u32 = if (words.len >= 3) blk: {
                const ns_s = obj_ensure_string(words[2]);
                const h = resolve_ns(ns_s.ptr, ns_s.len);
                if (h == 0) {
                    raise_ns_not_found_in_current(ns_s.ptr, ns_s.len);
                    return result_mod.from_globals(0);
                }
                break :blk h;
            } else tcl_ns.ns_current();
            const target_ns: *const tcl_ns.Namespace = @ptrFromInt(target_h);
            const parent_h = target_ns.parent;
            if (parent_h == 0) return result_mod.from_globals(obj_new_string(0, 0));
            const pf = tcl_ns.ns_full_name(parent_h);
            return result_mod.from_globals(obj_new_string(@bitCast(pf.ptr), @bitCast(pf.len)));
        }
        // ``namespace exists name``
        if (str_eq(@ptrFromInt(sub.ptr), sub.len, "exists")) {
            if (words.len < 3) return result_mod.from_globals(obj_new_int(0));
            const ns_s = obj_ensure_string(words[2]);
            return result_mod.from_globals(ns_exists(ns_s.ptr, ns_s.len));
        }
        // ``namespace children ?ns? ?pattern?``
        if (str_eq(@ptrFromInt(sub.ptr), sub.len, "children")) {
            const ctx_ns = if (words.len >= 3) blk: {
                const cs = obj_ensure_string(words[2]);
                // ``namespace children NS`` rejects an unknown ``NS``
                // using strict resolution — relative names are not
                // looked up from root (namespace-14.4 / 14.6).
                const h = resolve_ns_strict(cs.ptr, cs.len);
                if (h == 0) {
                    raise_ns_not_found_in_current(cs.ptr, cs.len);
                    return result_mod.from_globals(0);
                }
                break :blk h;
            } else tcl_ns.ns_current();
            var pat_ptr: u32 = 0;
            var pat_len: u32 = 0;
            if (words.len >= 4) {
                const pat_s = obj_ensure_string(words[3]);
                pat_ptr = pat_s.ptr;
                pat_len = pat_s.len;
            }
            return result_mod.from_globals(ns_children(ctx_ns, pat_ptr, pat_len));
        }
        // ``namespace delete ns ...`` — mark dead by zeroing child entries in
        // the parent table.  Our bump allocator cannot truly free, but clearing
        // the child-table slot prevents future ``ns_lookup`` hits.
        if (str_eq(@ptrFromInt(sub.ptr), sub.len, "delete")) {
            var di: u32 = 2;
            while (di < words.len) : (di += 1) {
                const ds = obj_ensure_string(words[di]);
                // Tcl 9 raises ``unknown namespace "X" in namespace
                // delete command`` for missing targets, but our test
                // bundles rely on top-level ``namespace delete X``
                // (no surrounding catch) being silent when X is
                // absent — propagating the error becomes a wasm trap
                // that aborts the entire bundle.  Keep the error
                // path conditional on an active catch scope so
                // catch'd callers (the tcltest cleanup wrapper) still
                // see the error, while bare top-level deletes stay
                // best-effort.
                const catch_depth = @import("../interp/tcl_catch.zig").catch_depth_get();
                if (!ns_delete_strict(ds.ptr, ds.len)) {
                    if (catch_depth > 0) {
                        raise_unknown_ns(ds.ptr, ds.len, "namespace delete command");
                        return result_mod.from_globals(0);
                    }
                    // No catch — best-effort silent fail (Tcl 9.0.3
                    // info.test pattern at line 5060).
                }
            }
            return result_mod.from_globals(0);
        }
        // ``namespace inscope ns body ?arg...?`` — like ``namespace eval`` but
        // evaluates with additional arguments appended.
        if (str_eq(@ptrFromInt(sub.ptr), sub.len, "inscope")) {
            if (words.len < 4) return result_mod.from_globals(0);
            const ns_obj_s = obj_ensure_string(words[2]);
            const target_ns = tcl_ns.ns_create_from_fqn(ns_obj_s.ptr, ns_obj_s.len);
            const saved_ns = tcl_ns.current_ns;
            tcl_ns.current_ns = target_ns;
            defer tcl_ns.current_ns = saved_ns;
            const bs = obj_ensure_string(words[3]);
            if (words.len == 4) {
                if (bs.len > 0) return result_mod.from_globals(interp.eval_script(bs.ptr, bs.len));
                return result_mod.from_globals(0);
            }
            // Build a properly-quoted args list via tcl_list, then append to body.
            // This ensures args containing spaces/braces are re-tokenized correctly.
            var args_obj: i32 = obj_new_string_copy(0, 0);
            var wi: u32 = 4;
            while (wi < words.len) : (wi += 1) {
                args_obj = rt.tcl_list(args_obj, words[wi]);
            }
            const as = obj_ensure_string(args_obj);
            const total = bs.len + (if (as.len > 0) 1 + as.len else 0);
            const buf = alloc(total);
            memcpy(buf, bs.ptr, bs.len);
            if (as.len > 0) {
                const d: [*]u8 = @ptrFromInt(buf + bs.len);
                d[0] = ' ';
                memcpy(buf + bs.len + 1, as.ptr, as.len);
            }
            return result_mod.from_globals(interp.eval_script(buf, total));
        }
        // ``namespace code script`` — return a script that evaluates
        // *script* in the current namespace.  Reference Tcl
        // (``NamespaceCodeCmd`` in ``generic/tclNamesp.c``) returns
        // ``::namespace inscope <currentNs> <script>`` (with *script*
        // properly list-quoted) so the resulting prefix is callable
        // from any context — e.g. ``trace add variable v read
        // [namespace code SafeFetch]`` registered against a callback
        // that lives in ``::tcltest`` but isn't ``namespace export``-ed,
        // so the trace fire path needs the inscope wrapping to find
        // it.  When *script* already begins with ``::namespace
        // inscope `` or ``namespace inscope `` it's returned
        // unchanged (idempotent).  The previous "return the script
        // unchanged (no-op approximation)" implementation registered
        // bare callback names that the trace dispatcher looked up in
        // root, so unexported ``::tcltest::SafeFetch`` traces silently
        // failed and ``$testConstraints(valgrind)`` never populated —
        // tcltests.tcl's ``expr {![testConstraint valgrind]}`` then
        // tripped PR #341's strict-boolean ``!`` (string.test
        // regression).
        if (str_eq(@ptrFromInt(sub.ptr), sub.len, "code")) {
            if (words.len < 3) return result_mod.from_globals(obj_new_string(0, 0));
            const ss = obj_ensure_string(words[2]);
            // Idempotency check — pass through scripts already
            // prefixed with the canonical ``::namespace inscope ``
            // (upstream ``NamespaceCodeCmd`` only checks the
            // ``::``-anchored spelling; ``namespace inscope ...``
            // without the leading ``::`` IS wrapped per Tcl 9 —
            // namespace-22.7 / Bug 3202171).
            if (ss.len >= 20) {
                const sp: [*]const u8 = @ptrFromInt(ss.ptr);
                const fq = "::namespace inscope ";
                var ok: bool = true;
                for (0..fq.len) |k| if (sp[k] != fq[k]) {
                    ok = false;
                    break;
                };
                if (ok) return result_mod.from_globals(words[2]);
            }
            const nf = tcl_ns.ns_full_name(tcl_ns.ns_current());
            // Build ``::namespace inscope <ns> <script>`` via
            // ``tcl_list`` so each element is properly list-quoted
            // (handles braces, spaces, backslashes in ``<script>``).
            // ``tcl_list(a, b)`` allocates a fresh accumulator each
            // call without consuming either input — release the
            // previous accumulator and the per-element TclObjs after
            // each append so the only live owner is the current
            // ``acc`` (Copilot review on PR #343).  ``words[2]`` is
            // owned by the caller; do NOT release it here.
            var acc: i32 = obj_mod.obj_new_string_copy(0, 0);
            const e1 = obj_mod.obj_new_string_copy(@bitCast(@intFromPtr("::namespace".ptr)), 11);
            const a1 = rt.tcl_list(acc, e1);
            obj_mod.tcl_obj_release(acc);
            obj_mod.tcl_obj_release(e1);
            acc = a1;
            const e2 = obj_mod.obj_new_string_copy(@bitCast(@intFromPtr("inscope".ptr)), 7);
            const a2 = rt.tcl_list(acc, e2);
            obj_mod.tcl_obj_release(acc);
            obj_mod.tcl_obj_release(e2);
            acc = a2;
            const e3 = obj_mod.obj_new_string_copy(@bitCast(nf.ptr), @bitCast(nf.len));
            const a3 = rt.tcl_list(acc, e3);
            obj_mod.tcl_obj_release(acc);
            obj_mod.tcl_obj_release(e3);
            acc = a3;
            const a4 = rt.tcl_list(acc, words[2]);
            obj_mod.tcl_obj_release(acc);
            return result_mod.from_globals(a4);
        }
    }
    return result_mod.from_globals(0);
}

/// Return the "tail" component of a namespace name: the part after the last
/// ``::`` separator (inclusive of multiple adjacent colons).
fn ns_tail(ptr: u32, len: u32) i32 {
    if (len == 0) return obj_new_string(0, 0);
    const sp: [*]const u8 = @ptrFromInt(ptr);
    var tail_start: u32 = 0;
    var i: u32 = 0;
    while (i + 1 < len) : (i += 1) {
        if (sp[i] == ':' and sp[i + 1] == ':') {
            var j: u32 = i + 2;
            while (j < len and sp[j] == ':') j += 1;
            tail_start = j;
        }
    }
    return obj_new_string(@bitCast(ptr + tail_start), @bitCast(len - tail_start));
}

/// Return the "qualifiers" of a namespace name: everything before the last
/// ``::`` separator.  Returns empty string when there is no qualifier.
fn ns_qualifiers(ptr: u32, len: u32) i32 {
    if (len == 0) return obj_new_string(0, 0);
    const sp: [*]const u8 = @ptrFromInt(ptr);
    // Walk backwards: skip tail chars then skip colons.
    var i: u32 = len;
    while (i > 0 and sp[i - 1] != ':') i -= 1;
    while (i > 0 and sp[i - 1] == ':') i -= 1;
    // ::foo → qualifiers are "::" (not empty string)
    if (i == 0 and len >= 3 and sp[0] == ':' and sp[1] == ':') {
        return obj_new_string(@bitCast(ptr), 2);
    }
    return obj_new_string(@bitCast(ptr), @bitCast(i));
}

/// ``namespace parent`` — return the qualifiers portion, but substitute `::`
/// for an empty result when the name was absolute (had at least one ``::``)
/// and was only one level deep.
fn ns_parent(ptr: u32, len: u32) i32 {
    if (len == 0) return obj_new_string(0, 0);
    const sp: [*]const u8 = @ptrFromInt(ptr);
    // Special-case root (``::``).
    if (len == 2 and sp[0] == ':' and sp[1] == ':') return obj_new_string(0, 0);

    var i: u32 = len;
    while (i > 0 and sp[i - 1] != ':') i -= 1;
    const has_sep = (i > 0);
    while (i > 0 and sp[i - 1] == ':') i -= 1;
    if (i == 0 and has_sep) {
        // Was ``::foo`` — parent is root ``::``.
        const buf = alloc(2);
        const d: [*]u8 = @ptrFromInt(buf);
        d[0] = ':';
        d[1] = ':';
        return obj_new_string(@bitCast(buf), 2);
    }
    return obj_new_string(@bitCast(ptr), @bitCast(i));
}

/// Resolve a (possibly qualified) name to a namespace handle, find-only.
fn resolve_ns(name_ptr: u32, name_len: u32) u32 {
    const r = tcl_ns.ns_resolve_qualified(tcl_ns.ns_current(), name_ptr, name_len);
    if (r.simple_len == 0) return if (r.target_ns != 0) r.target_ns else tcl_ns.ns_root();
    if (r.target_ns != 0) {
        const c = tcl_ns.ns_lookup(r.target_ns, r.simple_ptr, r.simple_len);
        if (c != 0) return c;
    }
    if (r.alt_ns != 0) {
        return tcl_ns.ns_lookup(r.alt_ns, r.simple_ptr, r.simple_len);
    }
    return 0;
}

/// Strict variant: ``namespace delete`` / ``namespace forget`` look
/// the target up *only* in the current namespace tree, without the
/// from-root alternate path that command / variable resolution
/// allows.  Per Tcl 9 ``TclGetNamespaceForQualName`` with
/// ``TCL_NAMESPACE_ONLY`` (namespace-14.4 / 14.5 / 14.6 / 27.2).
fn resolve_ns_strict(name_ptr: u32, name_len: u32) u32 {
    const r = tcl_ns.ns_resolve_qualified(tcl_ns.ns_current(), name_ptr, name_len);
    if (r.simple_len == 0) return if (r.target_ns != 0) r.target_ns else 0;
    if (r.target_ns != 0) {
        return tcl_ns.ns_lookup(r.target_ns, r.simple_ptr, r.simple_len);
    }
    return 0;
}

/// ``namespace exists name`` — 1 if the namespace exists, 0 otherwise.
fn ns_exists(name_ptr: u32, name_len: u32) i32 {
    const h = resolve_ns(name_ptr, name_len);
    return obj_new_int(if (h != 0) 1 else 0);
}

/// ``namespace children ?ns? ?pattern?`` — return a list of fully-qualified
/// child namespace names, optionally filtered by a glob pattern.
fn ns_children(ns_handle: u32, pat_ptr: u32, pat_len: u32) i32 {
    if (ns_handle == 0) return obj_new_string(0, 0);
    const ns: *const tcl_ns.Namespace = @ptrFromInt(ns_handle);
    if (ns.child_table.buf == 0) return obj_new_string(0, 0);

    // Tcl 9 ``namespace children`` matches the pattern against
    // each child's FULLY-QUALIFIED name.  When the supplied
    // pattern doesn't already start with ``::`` it is prefixed
    // with the queried namespace path: ``namespace children
    // ::a::b foo*`` matches against ``::a::b::foo*``.  Without
    // this, bare patterns like ``b*`` from inside a nested
    // namespace would always miss because the children's FQNs
    // start with ``::``.
    var eff_pat_ptr: u32 = pat_ptr;
    var eff_pat_len: u32 = pat_len;
    if (pat_len > 0) {
        const psp: [*]const u8 = @ptrFromInt(pat_ptr);
        const pat_starts_qual = pat_len >= 2 and psp[0] == ':' and psp[1] == ':';
        if (!pat_starts_qual) {
            const ns_full = tcl_ns.ns_full_name(ns_handle);
            const ns_root_only = ns_full.len == 2;
            const sep_len: u32 = if (ns_root_only) 0 else 2;
            const prefixed_total: u32 = ns_full.len + sep_len + pat_len;
            const prefixed_buf_addr = alloc(prefixed_total);
            if (prefixed_buf_addr != 0) {
                const pbuf: [*]u8 = @ptrFromInt(prefixed_buf_addr);
                const ns_p: [*]const u8 = @ptrFromInt(ns_full.ptr);
                var off2: u32 = 0;
                for (0..ns_full.len) |k| {
                    pbuf[off2] = ns_p[k];
                    off2 += 1;
                }
                if (!ns_root_only) {
                    pbuf[off2] = ':';
                    off2 += 1;
                    pbuf[off2] = ':';
                    off2 += 1;
                }
                for (0..pat_len) |k| {
                    pbuf[off2] = psp[k];
                    off2 += 1;
                }
                eff_pat_ptr = prefixed_buf_addr;
                eff_pat_len = prefixed_total;
            }
        }
    }
    // Free the prefixed pattern buffer (if we allocated one) once
    // both passes finish.  ``defer`` keeps the cleanup paired with
    // the alloc so accidental early returns don't leak.
    defer if (eff_pat_ptr != pat_ptr and eff_pat_ptr != 0) {
        obj_mod.free_sized(eff_pat_ptr, eff_pat_len);
    };

    const bucket_size: u32 = tcl_ns.NS_BUCKET_SIZE;
    // Two-pass: size then fill.
    var total_bytes: u32 = 0;
    var count: u32 = 0;
    var i: u32 = 0;
    while (i < ns.child_table.cap) : (i += 1) {
        const bucket = ns.child_table.buf + i * bucket_size;
        const name_ptr_v: u32 = @bitCast(obj_mod.read_i32(bucket));
        if (name_ptr_v == 0) continue;
        const child_handle: u32 = @bitCast(obj_mod.read_i32(bucket + tcl_ns.OFF_HANDLE));
        if (child_handle == 0) continue;
        const fqn = tcl_ns.ns_full_name(child_handle);
        if (eff_pat_len > 0 and !tcl_string.glob_match(eff_pat_ptr, eff_pat_len, fqn.ptr, fqn.len)) continue;
        if (total_bytes > 0) total_bytes += 1; // space
        total_bytes += fqn.len;
        count += 1;
    }
    if (count == 0) return obj_new_string(0, 0);
    const buf = alloc(total_bytes);
    var off: u32 = 0;
    var first: bool = true;
    i = 0;
    while (i < ns.child_table.cap) : (i += 1) {
        const bucket = ns.child_table.buf + i * bucket_size;
        const name_ptr_v: u32 = @bitCast(obj_mod.read_i32(bucket));
        if (name_ptr_v == 0) continue;
        const child_handle: u32 = @bitCast(obj_mod.read_i32(bucket + tcl_ns.OFF_HANDLE));
        if (child_handle == 0) continue;
        const fqn = tcl_ns.ns_full_name(child_handle);
        if (eff_pat_len > 0 and !tcl_string.glob_match(eff_pat_ptr, eff_pat_len, fqn.ptr, fqn.len)) continue;
        if (!first) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        first = false;
        memcpy(buf + off, fqn.ptr, fqn.len);
        off += fqn.len;
    }
    return obj_new_string(@bitCast(buf), @bitCast(total_bytes));
}

/// ``namespace delete name ...`` — mark namespace dead by removing it from its
/// parent's child_table.  Cannot truly free memory (bump allocator), but
/// prevents future lookups from finding it.
fn ns_delete(name_ptr: u32, name_len: u32) void {
    const h = resolve_ns(name_ptr, name_len);
    if (h == 0) return;
    do_ns_delete(h);
}

/// Strict-resolution version of :fn:`ns_delete` for ``namespace
/// delete``.  Returns true when the namespace was found (and
/// detached), false otherwise — caller surfaces the canonical
/// "unknown namespace" error.
fn ns_delete_strict(name_ptr: u32, name_len: u32) bool {
    const h = resolve_ns_strict(name_ptr, name_len);
    if (h == 0) return false;
    do_ns_delete(h);
    return true;
}

fn do_ns_delete(h: u32) void {
    const ns: *const tcl_ns.Namespace = @ptrFromInt(h);
    // Tcl 9 ``TclTeardownNamespace`` cascades the delete to every
    // command in the namespace's cmd_table, invalidating each
    // import redirect that pointed at the live command.  After
    // delete, ``info commands`` queries downstream stop returning
    // the orphaned imports (namespace-8.4).
    invalidate_ns_command_imports(h);
    const parent = ns.parent;
    if (parent == 0) return; // can't delete root
    const parent_ns: *tcl_ns.Namespace = @ptrFromInt(parent);
    if (parent_ns.child_table.buf == 0) return;
    const bucket_size: u32 = tcl_ns.NS_BUCKET_SIZE;
    var i: u32 = 0;
    while (i < parent_ns.child_table.cap) : (i += 1) {
        const bucket = parent_ns.child_table.buf + i * bucket_size;
        const child_handle: u32 = @bitCast(obj_mod.read_i32(bucket + tcl_ns.OFF_HANDLE));
        if (child_handle == h) {
            obj_mod.write_i32(bucket + tcl_ns.OFF_HANDLE, 0);
            return;
        }
    }
}

/// Walk *ns*'s ``cmd_table`` and zero the ``real_cmd`` slot of every
/// import redirect that points at one of these commands.  ``info
/// commands`` filters such "dead" redirects via ``entry_matches``,
/// matching Tcl 9 ``Tcl_DeleteNamespace`` semantics.
fn invalidate_ns_command_imports(ns_addr: u32) void {
    if (ns_addr == 0) return;
    const ns: *const tcl_ns.Namespace = @ptrFromInt(ns_addr);
    if (ns.cmd_table.buf == 0) return;
    const c = tcl_procs_constants;
    const bucket_size: u32 = 16;
    // Live linear-memory bound — any cmd handle past this points
    // into unmapped territory and the read_i32 would trap.
    const mem_pages: u32 = @intCast(@wasmMemorySize(0));
    const mem_bytes: u64 = @as(u64, mem_pages) * 65536;
    var i: u32 = 0;
    while (i < ns.cmd_table.cap) : (i += 1) {
        const bucket = ns.cmd_table.buf + i * bucket_size;
        // Only walk *occupied* buckets — a zero name_ptr indicates an
        // empty slot, and reading the handle there can produce
        // arbitrary noise the way some hash-table implementations
        // pack their headers (set-old-8.x regression on bare cmd
        // table walks).
        const name_ptr: u32 = @bitCast(obj_mod.read_i32(bucket));
        if (name_ptr == 0) continue;
        const cmd: u32 = @bitCast(obj_mod.read_i32(bucket + tcl_ns.OFF_HANDLE));
        if (cmd == 0) continue;
        // Builtin forwards / aliases / freshly-stamped command
        // entries can have a handle outside the heap range; guard
        // every dereference.
        if (@as(u64, cmd) + c.OFF_IMPORT_REF_HEAD + 4 > mem_bytes) continue;
        // Walk the linked list of redirects at OFF_IMPORT_REF_HEAD.
        var ref_head: u32 = @bitCast(obj_mod.read_i32(cmd + c.OFF_IMPORT_REF_HEAD));
        var hops: u32 = 0;
        while (ref_head != 0 and hops < 4096) : (hops += 1) {
            if (@as(u64, ref_head) + 8 > mem_bytes) break;
            const redirect_cmd: u32 = @bitCast(obj_mod.read_i32(ref_head));
            const next_ref: u32 = @bitCast(obj_mod.read_i32(ref_head + 4));
            if (redirect_cmd != 0 and @as(u64, redirect_cmd) + c.OFF_PARAMS_OBJ + 4 <= mem_bytes) {
                const desc: u32 = @bitCast(obj_mod.read_i32(redirect_cmd + c.OFF_PARAMS_OBJ));
                if (desc != 0 and @as(u64, desc) + 4 <= mem_bytes) {
                    obj_mod.write_i32(desc, 0);
                }
            }
            ref_head = next_ref;
        }
        obj_mod.write_i32(cmd + c.OFF_IMPORT_REF_HEAD, 0);
    }
}

const tcl_procs_constants = struct {
    pub const OFF_FLAGS: u32 = 8;
    pub const OFF_PARAMS_OBJ: u32 = 12;
    pub const OFF_IMPORT_REF_HEAD: u32 = 32;
    pub const CMD_IMPORTED: u32 = 0x02;
};

/// Return true when *pat*'s namespace prefix (everything before the
/// final ``::`` separator) resolves to a real namespace.  When the
/// pattern has no namespace prefix, the current namespace counts.
fn forget_pattern_ns_valid(pat_ptr: u32, pat_len: u32) bool {
    if (pat_len == 0) return true;
    const sp: [*]const u8 = @ptrFromInt(pat_ptr);
    var last_sep: i32 = -1;
    var k: u32 = 0;
    while (k + 1 < pat_len) : (k += 1) {
        if (sp[k] == ':' and sp[k + 1] == ':') {
            last_sep = @intCast(k);
            k += 1;
        }
    }
    if (last_sep < 0) return true; // no qualifier
    const sep_at: u32 = @intCast(last_sep);
    if (sep_at == 0) return true; // ``::pat`` — root prefix
    // Resolve the qualifier as a namespace.
    return resolve_ns(pat_ptr, sep_at) != 0;
}

/// Raise the canonical Tcl 9 ``unknown namespace in CONTEXT "PAT"``
/// diagnostic.  Used by ``namespace forget`` for patterns whose
/// qualifier doesn't name a real namespace.
fn raise_unknown_ns_in_pattern(pat_ptr: u32, pat_len: u32, context: []const u8) void {
    const catch_mod = @import("../interp/tcl_catch.zig");
    const prefix: []const u8 = "unknown namespace in ";
    const mid: []const u8 = " \"";
    const suffix: []const u8 = "\"";
    const total: u32 = @intCast(prefix.len + context.len + mid.len + pat_len + suffix.len);
    const buf = alloc(total);
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
    for (context) |c| {
        dst[off] = c;
        off += 1;
    }
    for (mid) |c| {
        dst[off] = c;
        off += 1;
    }
    const pp: [*]const u8 = @ptrFromInt(pat_ptr);
    for (0..pat_len) |k| {
        dst[off] = pp[k];
        off += 1;
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const e = obj_mod.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(e);
}

/// Handle ``namespace ensemble`` subcommands.  The arity check at
/// the front gate already validated ``words.len >= 3``; here we
/// dispatch on the second sub-keyword.  Ensemble dispatch / live-
/// command shape is still stubbed — we surface the canonical
/// errors and ``configure`` introspection only.
fn eval_ns_ensemble(words: []const i32) i32 {
    const sub2 = obj_ensure_string(words[2]);
    if (sub2.len == 0) {
        raise_bad_ensemble_subcmd(0, 0);
        return 0;
    }
    const sp2: [*]const u8 = @ptrFromInt(sub2.ptr);
    if (slice_eq_ns(sub2.ptr, sub2.len, "create")) {
        return eval_ns_ensemble_create(words);
    }
    if (slice_eq_ns(sub2.ptr, sub2.len, "exists")) {
        // Stub: we don't track ensembles, so always 0.
        return obj_new_int(0);
    }
    if (slice_eq_ns(sub2.ptr, sub2.len, "configure")) {
        // Stub: report empty default options.  Matches the option
        // names Tcl 9 ships even though we don't actually use them.
        return obj_new_string_copy_str("-map {} -namespace ::%s -parameters {} -prefixes 1 -subcommands {} -unknown {}");
    }
    _ = sp2;
    raise_bad_ensemble_subcmd(sub2.ptr, sub2.len);
    return 0;
}

fn eval_ns_ensemble_create(words: []const i32) i32 {
    // ``namespace ensemble create ?-OPT VAL?...`` — we just validate
    // the option shape to surface namespace-44.3 / 44.4 / 44.6 and
    // return an empty result.  Real dispatch isn't wired up.
    if (((words.len - 3) & 1) != 0) {
        // A bare ``namespace ensemble create gorp`` (one trailing
        // word) is wrong-args in Tcl 9 because options are
        // ``-key value`` pairs; an odd remainder means the user
        // gave a stray word like ``gorp`` (namespace-44.6).
        raise_ns_error("wrong # args: should be \"namespace ensemble create ?option value ...?\"");
        return 0;
    }
    var i: u32 = 3;
    while (i + 1 < words.len) : (i += 2) {
        const key = obj_ensure_string(words[i]);
        const val = obj_ensure_string(words[i + 1]);
        if (key.len == 0 or @as([*]const u8, @ptrFromInt(key.ptr))[0] != '-') {
            raise_ns_error("missing value to go with key");
            return 0;
        }
        // ``-map`` value must be a list of {key impl key impl ...}
        // with non-empty implementations (44.4).
        if (slice_eq_ns(key.ptr, key.len, "-map")) {
            if (val.len == 0) continue;
            const obj_h = @import("../valtypes/tcl_obj.zig");
            const n = obj_h.list_count_elements(val.ptr, val.len);
            if (@rem(n, 2) != 0) {
                raise_ns_error("missing value to go with key");
                return 0;
            }
            var k: i64 = 1;
            while (k < n) : (k += 2) {
                const impl = obj_h.list_element_at(val.ptr, val.len, k);
                if (impl.len == 0) {
                    raise_ns_error("ensemble subcommand implementations must be non-empty lists");
                    return 0;
                }
            }
        }
    }
    if (i < words.len) {
        // Trailing single token (e.g. ``-map`` with no value at the
        // end) — Tcl 9 says "missing value to go with key".
        raise_ns_error("missing value to go with key");
        return 0;
    }
    return obj_new_string(0, 0);
}

fn raise_bad_ensemble_subcmd(sub_ptr: u32, sub_len: u32) void {
    const catch_mod = @import("../interp/tcl_catch.zig");
    const prefix: []const u8 = "bad subcommand \"";
    const suffix: []const u8 = "\": must be configure, create, or exists";
    const total: u32 = @intCast(prefix.len + sub_len + suffix.len);
    const buf = alloc(total);
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
    if (sub_len > 0) {
        const sp: [*]const u8 = @ptrFromInt(sub_ptr);
        for (0..sub_len) |k| {
            dst[off] = sp[k];
            off += 1;
        }
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const e = obj_mod.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(e);
}

fn obj_new_string_copy_str(s: []const u8) i32 {
    // ``%s`` placeholder: drop it (we don't carry a real ns at
    // present).  This produces the literal Tcl 9 default
    // configure dict minus the live namespace name.
    var trimmed: [128]u8 = undefined;
    var off: u32 = 0;
    var i: u32 = 0;
    while (i < s.len and off < trimmed.len) : (i += 1) {
        if (i + 1 < s.len and s[i] == '%' and s[i + 1] == 's') {
            i += 1;
            continue;
        }
        trimmed[off] = s[i];
        off += 1;
    }
    return obj_mod.obj_new_string_copy(@bitCast(@intFromPtr(&trimmed)), off);
}

/// Raise the canonical ``import pattern "PAT" tries to import from
/// namespace "NS" into itself`` error (namespace-9.3).  The "NS"
/// is rendered using only the *simple* trailing component of the
/// source qualifier — matches the upstream Tcl wording.
fn raise_self_import(pat_ptr: u32, pat_len: u32, src_ptr: u32, src_len: u32) void {
    const catch_mod = @import("../interp/tcl_catch.zig");
    // Strip a leading ``::`` from the source for the diagnostic
    // (upstream emits the unqualified tail).
    var sp_ptr = src_ptr;
    var sp_len = src_len;
    if (sp_len >= 2) {
        const sp: [*]const u8 = @ptrFromInt(sp_ptr);
        if (sp[0] == ':' and sp[1] == ':') {
            sp_ptr += 2;
            sp_len -= 2;
        }
    }
    const prefix: []const u8 = "import pattern \"";
    const mid: []const u8 = "\" tries to import from namespace \"";
    const suffix: []const u8 = "\" into itself";
    const total: u32 = @intCast(prefix.len + pat_len + mid.len + sp_len + suffix.len);
    const buf = alloc(total);
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
    const pp: [*]const u8 = @ptrFromInt(pat_ptr);
    for (0..pat_len) |k| {
        dst[off] = pp[k];
        off += 1;
    }
    for (mid) |c| {
        dst[off] = c;
        off += 1;
    }
    const np: [*]const u8 = @ptrFromInt(sp_ptr);
    for (0..sp_len) |k| {
        dst[off] = np[k];
        off += 1;
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const e = obj_mod.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(e);
}

/// Return true when *pat* contains a ``::`` namespace separator —
/// used to reject qualified patterns passed to ``namespace export``
/// (namespace-26.3) and ``namespace import`` self-imports.
fn export_pattern_has_ns(ptr: u32, len: u32) bool {
    if (len < 2) return false;
    const sp: [*]const u8 = @ptrFromInt(ptr);
    var i: u32 = 0;
    while (i + 1 < len) : (i += 1) {
        if (sp[i] == ':' and sp[i + 1] == ':') return true;
    }
    return false;
}

/// Raise ``invalid export pattern "PAT": pattern can't specify a
/// namespace``.
fn raise_invalid_export_pattern(pat_ptr: u32, pat_len: u32) void {
    const catch_mod = @import("../interp/tcl_catch.zig");
    const prefix: []const u8 = "invalid export pattern \"";
    const suffix: []const u8 = "\": pattern can't specify a namespace";
    const total: u32 = @intCast(prefix.len + pat_len + suffix.len);
    const buf = alloc(total);
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
    const pp: [*]const u8 = @ptrFromInt(pat_ptr);
    for (0..pat_len) |k| {
        dst[off] = pp[k];
        off += 1;
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const e = obj_mod.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(e);
}

/// Raise the canonical Tcl 9 ``namespace "NAME" not found in
/// "<current_ns>"`` error used by relative-resolution failures
/// (namespace-14.4 / 14.6).
fn raise_ns_not_found_in_current(name_ptr: u32, name_len: u32) void {
    const catch_mod = @import("../interp/tcl_catch.zig");
    const cur = tcl_ns.ns_current();
    const cur_full = tcl_ns.ns_full_name(cur);
    const prefix: []const u8 = "namespace \"";
    const mid: []const u8 = "\" not found in \"";
    const suffix: []const u8 = "\"";
    const total: u32 = @intCast(prefix.len + name_len + mid.len + cur_full.len + suffix.len);
    const buf = alloc(total);
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
    const np: [*]const u8 = @ptrFromInt(name_ptr);
    for (0..name_len) |k| {
        dst[off] = np[k];
        off += 1;
    }
    for (mid) |c| {
        dst[off] = c;
        off += 1;
    }
    const cp: [*]const u8 = @ptrFromInt(cur_full.ptr);
    for (0..cur_full.len) |k| {
        dst[off] = cp[k];
        off += 1;
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const e = obj_mod.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(e);
}

/// Raise the canonical Tcl 9 ``unknown namespace "NAME" in CONTEXT``
/// error.  *context* is a short fragment such as ``"namespace delete
/// command"`` that the operation name appends to its diagnostic.
fn raise_unknown_ns(name_ptr: u32, name_len: u32, context: []const u8) void {
    const catch_mod = @import("../interp/tcl_catch.zig");
    const prefix: []const u8 = "unknown namespace \"";
    const mid: []const u8 = "\" in ";
    const total: u32 = @intCast(prefix.len + name_len + mid.len + context.len);
    const buf = alloc(total);
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
    const np: [*]const u8 = @ptrFromInt(name_ptr);
    for (0..name_len) |k| {
        dst[off] = np[k];
        off += 1;
    }
    for (mid) |c| {
        dst[off] = c;
        off += 1;
    }
    for (context) |c| {
        dst[off] = c;
        off += 1;
    }
    const e = obj_mod.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(e);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "namespace", .arity_min = 1, .arity_max = null, .handler = &eval_namespace },
};

// Sub-command arities — mirrors ``core/commands/registry/tcl/namespace.py``.
// Cross-checked against C Tcl 9.0 ``tclNamesp.c`` every ``Namespace*Cmd``.
pub const subcommands: []const reg.SubEntry = &.{
    .{ .name = "children", .arity_min = 0, .arity_max = 2, .handler = &eval_namespace },
    .{ .name = "code", .arity_min = 1, .arity_max = 1, .handler = &eval_namespace },
    .{ .name = "current", .arity_min = 0, .arity_max = 0, .handler = &eval_namespace },
    .{ .name = "delete", .arity_min = 0, .arity_max = null, .handler = &eval_namespace },
    .{ .name = "ensemble", .arity_min = 1, .arity_max = null, .handler = &eval_namespace },
    .{ .name = "eval", .arity_min = 2, .arity_max = null, .handler = &eval_namespace },
    .{ .name = "exists", .arity_min = 1, .arity_max = 1, .handler = &eval_namespace },
    .{ .name = "export", .arity_min = 0, .arity_max = null, .handler = &eval_namespace },
    .{ .name = "forget", .arity_min = 0, .arity_max = null, .handler = &eval_namespace },
    .{ .name = "import", .arity_min = 0, .arity_max = null, .handler = &eval_namespace },
    .{ .name = "inscope", .arity_min = 2, .arity_max = null, .handler = &eval_namespace },
    .{ .name = "origin", .arity_min = 1, .arity_max = 1, .handler = &eval_namespace },
    .{ .name = "parent", .arity_min = 0, .arity_max = 1, .handler = &eval_namespace },
    .{ .name = "path", .arity_min = 0, .arity_max = 1, .handler = &eval_namespace },
    .{ .name = "qualifiers", .arity_min = 1, .arity_max = 1, .handler = &eval_namespace },
    .{ .name = "tail", .arity_min = 1, .arity_max = 1, .handler = &eval_namespace },
    .{ .name = "unknown", .arity_min = 0, .arity_max = 1, .handler = &eval_namespace },
    .{ .name = "upvar", .arity_min = 1, .arity_max = null, .handler = &eval_namespace },
    .{ .name = "which", .arity_min = 1, .arity_max = 2, .handler = &eval_namespace },
};
