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
                // ``namespace import`` (no args) returns the simple
                // names of every command currently imported into
                // the calling namespace (namespace-old-9.5).
                if (words.len == 2) {
                    return result_mod.from_globals(list_imported_simple_names(tcl_ns.ns_current()));
                }
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
                // BUILTIN fallback — core commands (``set``, ``string``,
                // ``puts``, ...) live in the static BUILTINS slice
                // rather than any namespace's cmd_table.  Reference
                // Tcl 9 considers them rooted at ``::`` for the
                // purposes of :func:`Tcl_GetCommandFullName`, so
                // ``namespace origin set`` returns ``::set``.  Probe
                // the BUILTINS lookup first; on hit, fabricate the
                // canonical FQN ``::<name>``.  Also tolerate an input
                // already prefixed with ``::`` (e.g. ``namespace origin
                // ::set``) — strip the prefix before the BUILTINS
                // probe so the lookup matches.
                const cmd_table = @import("../dispatch/tcl_cmd_table.zig");
                var probe_ptr: u32 = name_obj.ptr;
                var probe_len: u32 = name_obj.len;
                if (name_obj.len >= 2) {
                    const pp: [*]const u8 = @ptrFromInt(name_obj.ptr);
                    if (pp[0] == ':' and pp[1] == ':') {
                        probe_ptr = name_obj.ptr + 2;
                        probe_len = name_obj.len - 2;
                    }
                }
                if (cmd_table.lookup(probe_ptr, probe_len) != null) {
                    const fqn_total: u32 = probe_len + 2;
                    const fqn_buf = alloc(fqn_total);
                    if (fqn_buf == 0) return result_mod.from_globals(obj_new_string(0, 0));
                    const fqn_dst: [*]u8 = @ptrFromInt(fqn_buf);
                    fqn_dst[0] = ':';
                    fqn_dst[1] = ':';
                    const sp: [*]const u8 = @ptrFromInt(probe_ptr);
                    for (0..probe_len) |k| fqn_dst[2 + k] = sp[k];
                    return result_mod.from_globals(obj_mod.obj_new_string_take(fqn_buf, fqn_total, fqn_total));
                }
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
    // Tcl 9 ``Tcl_NamespaceQualifiersCmd`` (tclNamesp.c) returns
    // the prefix up to (and not including) the trailing ``::``
    // separator, then trims any trailing colons.  Inputs without
    // any namespace separator return the empty string — including
    // ``::x`` (which has the root ``::`` prefix but produces ``""``
    // per namespace-old-3.2).
    var i: u32 = len;
    while (i > 0 and sp[i - 1] != ':') i -= 1;
    // Now i is at the byte AFTER the last ``::`` (or 0 if none).
    // Strip trailing colons to leave a clean qualifier.
    while (i > 0 and sp[i - 1] == ':') i -= 1;
    // ``::foo`` and ``::`` both have all-colon prefixes and return
    // ``""``.  ``::foo::bar`` returns ``::foo``.  The disambiguation
    // is: the final non-stripped prefix must contain a non-colon
    // byte.  If everything left is colons we yield the empty
    // string.
    if (i == 0) return obj_new_string(0, 0);
    var saw_nonsep: bool = false;
    var j: u32 = 0;
    while (j < i) : (j += 1) {
        if (sp[j] != ':') {
            saw_nonsep = true;
            break;
        }
    }
    if (!saw_nonsep) return obj_new_string(0, 0);
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
        if (buf == 0) return obj_new_string(0, 0);
        const d: [*]u8 = @ptrFromInt(buf);
        d[0] = ':';
        d[1] = ':';
        return rt.obj_new_string_take(buf, 2, 2);
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
    if (buf == 0) return obj_new_string(0, 0);
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
    return rt.obj_new_string_take(buf, total_bytes, total_bytes);
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
    //
    // ``invalidate_ns_command_imports`` already tombstones every
    // non-root cmd_table bucket's OFF_HANDLE to 0, so future
    // ``ns_cmd_find`` calls on those tables miss.  The proc-lookup
    // LRU, however, caches resolved bucket addresses directly —
    // post-delete LRU hits would still hand back the orphaned
    // Command pointer and ``test_ns_1::q`` would keep running its
    // body (namespace-old-7.1, namespace-35.2).  Drop the LRU
    // unconditionally on every namespace teardown.
    invalidate_ns_command_imports(h);
    procs.lru_invalidate_all();
    const parent = ns.parent;
    if (parent == 0) return; // can't delete root
    const parent_ns: *tcl_ns.Namespace = @ptrFromInt(parent);
    // Drop ensemble commands whose target_ns matches the namespace
    // being deleted.  ``namespace eval ns { namespace ensemble create
    // }`` registers the dispatch command in the parent's cmd_table;
    // when the ns goes away, the matching ensemble bucket should
    // disappear too (namespace-42.1 / 42.2).
    drop_parent_ensembles_for(parent_ns, h);
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

/// Walk *parent_ns*'s ``cmd_table`` and clear every ensemble Command
/// whose ``EnsembleRec.target_ns`` is *deleted_ns*.  Mirrors C Tcl's
/// ``TclDeleteNamespace`` → ``TclMakeEnsemble`` teardown: an
/// ensemble dispatcher tied to a deleted namespace must stop being
/// reachable by name.
fn drop_parent_ensembles_for(parent_ns: *tcl_ns.Namespace, deleted_ns: u32) void {
    if (parent_ns.cmd_table.buf == 0) return;
    const bucket_size: u32 = tcl_ns.NS_BUCKET_SIZE;
    // Capture the parent address up-front so we can route the
    // tombstone through ``ns_cmd_clear`` (which bumps the namespace's
    // ``cmd_ref_epoch`` to invalidate stale command-resolution
    // caches).  Direct ``write_i32`` of OFF_HANDLE to zero leaves
    // the epoch unchanged and stale ``::path`` lookups keep
    // resolving to the dead bucket.
    const parent_addr: u32 = @intFromPtr(parent_ns);
    var i: u32 = 0;
    while (i < parent_ns.cmd_table.cap) : (i += 1) {
        const bucket = parent_ns.cmd_table.buf + i * bucket_size;
        const np: u32 = @bitCast(obj_mod.read_i32(bucket));
        if (np == 0) continue;
        const cmd: u32 = @bitCast(obj_mod.read_i32(bucket + tcl_ns.OFF_HANDLE));
        if (cmd == 0) continue;
        const flags: u32 = @bitCast(obj_mod.read_i32(cmd + procs.OFF_FLAGS));
        if ((flags & procs.CMD_ENSEMBLE) == 0) continue;
        const rec_addr: u32 = @bitCast(obj_mod.read_i32(cmd + procs.OFF_PARAMS_OBJ));
        if (rec_addr == 0) continue;
        const rec: *const EnsembleRec = @ptrFromInt(rec_addr);
        if (rec.target_ns != deleted_ns) continue;
        ensemble_rec_release(rec_addr);
        obj_mod.write_i32(cmd + procs.OFF_PARAMS_OBJ, 0);
        obj_mod.write_i32(cmd + procs.OFF_FLAGS, 0);
        const nl: u32 = @bitCast(obj_mod.read_i32(bucket + 4));
        _ = tcl_ns.ns_cmd_clear(parent_addr, np, nl);
    }
}

/// ``namespace import`` (no args) handler — returns a space-
/// joined Tcl list of the simple names of every command in
/// *ns_addr*'s cmd_table that carries the ``CMD_IMPORTED`` flag
/// (and whose redirect hasn't been forgotten).  Order is bucket-
/// table order; callers can ``lsort`` for stability (namespace-
/// old-9.5).
fn list_imported_simple_names(ns_addr: u32) i32 {
    if (ns_addr == 0) return obj_new_string(0, 0);
    const ns: *const tcl_ns.Namespace = @ptrFromInt(ns_addr);
    if (ns.cmd_table.buf == 0) return obj_new_string(0, 0);
    const bucket_size: u32 = 16;
    const mem_pages: u32 = @intCast(@wasmMemorySize(0));
    const mem_bytes: u64 = @as(u64, mem_pages) * 65536;

    // Predicate: is the Command at *cmd* a live import redirect?
    // CMD_IMPORTED bit set AND its ImportedCmdData.real_cmd is
    // non-zero (a forgotten redirect zeroes ``real_cmd`` to
    // tombstone the alias).
    const Pred = struct {
        fn is_live_import(cmd: u32, mem_b: u64) bool {
            if (cmd == 0) return false;
            if (@as(u64, cmd) + tcl_procs_constants.OFF_FLAGS + 4 > mem_b) return false;
            const flags: u32 = @bitCast(obj_mod.read_i32(cmd + tcl_procs_constants.OFF_FLAGS));
            if ((flags & tcl_procs_constants.CMD_IMPORTED) == 0) return false;
            if (@as(u64, cmd) + tcl_procs_constants.OFF_PARAMS_OBJ + 4 > mem_b) return false;
            const desc: u32 = @bitCast(obj_mod.read_i32(cmd + tcl_procs_constants.OFF_PARAMS_OBJ));
            if (desc == 0) return false;
            if (@as(u64, desc) + 4 > mem_b) return false;
            const real_cmd: u32 = @bitCast(obj_mod.read_i32(desc));
            return real_cmd != 0;
        }
    };

    // Two-pass sizing + emit.
    var total: u32 = 0;
    var count: u32 = 0;
    var i: u32 = 0;
    while (i < ns.cmd_table.cap) : (i += 1) {
        const bucket = ns.cmd_table.buf + i * bucket_size;
        const name_ptr: u32 = @bitCast(obj_mod.read_i32(bucket));
        if (name_ptr == 0) continue;
        const cmd: u32 = @bitCast(obj_mod.read_i32(bucket + tcl_ns.OFF_HANDLE));
        if (!Pred.is_live_import(cmd, mem_bytes)) continue;
        const name_len: u32 = @bitCast(obj_mod.read_i32(bucket + 4));
        if (count > 0) total += 1;
        total += name_len;
        count += 1;
    }
    if (count == 0) return obj_new_string(0, 0);
    const buf = alloc(total);
    if (buf == 0) return obj_new_string(0, 0);
    var off: u32 = 0;
    var written: u32 = 0;
    i = 0;
    while (i < ns.cmd_table.cap) : (i += 1) {
        const bucket = ns.cmd_table.buf + i * bucket_size;
        const name_ptr: u32 = @bitCast(obj_mod.read_i32(bucket));
        if (name_ptr == 0) continue;
        const cmd: u32 = @bitCast(obj_mod.read_i32(bucket + tcl_ns.OFF_HANDLE));
        if (!Pred.is_live_import(cmd, mem_bytes)) continue;
        const name_len: u32 = @bitCast(obj_mod.read_i32(bucket + 4));
        if (written > 0) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        const dst: [*]u8 = @ptrFromInt(buf + off);
        const np: [*]const u8 = @ptrFromInt(name_ptr);
        for (0..name_len) |k| dst[k] = np[k];
        off += name_len;
        written += 1;
    }
    return obj_mod.obj_new_string_take(buf, off, total);
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
    // Bump the cmd-ref epoch on this namespace so any cached
    // bucket lookups in the dispatcher re-resolve and notice the
    // tombstoned entries.
    if (ns_addr != tcl_ns.ns_root()) tcl_ns.bump_cmd_ref_epoch(ns_addr);
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
        if (@as(u64, cmd) + c.OFF_IMPORT_REF_HEAD + 4 > mem_bytes) {
            // Still tombstone non-root buckets so subsequent
            // dispatch fails (Tcl 9 TclTeardownNamespace).
            if (ns_addr != tcl_ns.ns_root()) {
                obj_mod.write_i32(bucket + tcl_ns.OFF_HANDLE, 0);
            }
            continue;
        }
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
        // Tombstone *only* non-root buckets so ``namespace delete
        // ::ns`` leaves the proc dispatchable as "invalid command
        // name" (namespace-7.1 / 35.2).  Root must remain intact
        // because almost every test runs commands at root.
        if (ns_addr != tcl_ns.ns_root()) {
            obj_mod.write_i32(bucket + tcl_ns.OFF_HANDLE, 0);
        }
    }
}

const tcl_procs_constants = struct {
    pub const OFF_FLAGS: u32 = 8;
    pub const OFF_PARAMS_OBJ: u32 = 12;
    pub const OFF_IMPORT_REF_HEAD: u32 = 32;
    // Keep in sync with :data:`tcl_procs.CMD_IMPORTED` /
    // :data:`tcl_ns.tcl_procs_constants.CMD_IMPORTED`.
    pub const CMD_IMPORTED: u32 = 0x80;
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
    if (slice_eq_ns(sub2.ptr, sub2.len, "create")) {
        return eval_ns_ensemble_create(words);
    }
    if (slice_eq_ns(sub2.ptr, sub2.len, "exists")) {
        if (words.len != 4) {
            raise_ns_error("wrong # args: should be \"namespace ensemble exists cmdname\"");
            return 0;
        }
        const bucket = ensemble_find_cmd(words[3]);
        if (bucket == 0) return obj_new_int(0);
        const flags: u32 = @bitCast(obj_mod.read_i32(bucket + procs.OFF_FLAGS));
        return obj_new_int(if ((flags & procs.CMD_ENSEMBLE) != 0) 1 else 0);
    }
    if (slice_eq_ns(sub2.ptr, sub2.len, "configure") or
        slice_eq_ns(sub2.ptr, sub2.len, "config"))
    {
        if (words.len < 4) {
            raise_ns_error("wrong # args: should be \"namespace ensemble configure cmdname ?-option value ...?\"");
            return 0;
        }
        return eval_ns_ensemble_configure(words);
    }
    raise_bad_ensemble_subcmd(sub2.ptr, sub2.len);
    return 0;
}

/// ``namespace ensemble configure CMD ?option? ?value option value ...?``
/// — read or write one or more ensemble options.
///   * 4-arg form (``namespace ensemble configure CMD``): return the
///     full option list.
///   * 5-arg form (``... CMD -OPT``): return that option's value.
///   * 6+-arg form (``... CMD -OPT VAL ...``): set option(s); return
///     empty.
fn eval_ns_ensemble_configure(words: []const i32) i32 {
    const bucket = ensemble_find_cmd(words[3]);
    if (bucket == 0) {
        raise_not_an_ensemble(words[3]);
        return 0;
    }
    const flags: u32 = @bitCast(obj_mod.read_i32(bucket + procs.OFF_FLAGS));
    if ((flags & procs.CMD_ENSEMBLE) == 0) {
        raise_not_an_ensemble(words[3]);
        return 0;
    }
    const rec_addr: u32 = @bitCast(obj_mod.read_i32(bucket + procs.OFF_PARAMS_OBJ));
    if (rec_addr == 0) {
        raise_not_an_ensemble(words[3]);
        return 0;
    }
    const rec: *EnsembleRec = @ptrFromInt(rec_addr);
    if (words.len == 4) {
        return render_ensemble_config(rec);
    }
    if (words.len == 5) {
        return read_ensemble_option(rec, words[4]);
    }
    // Setter form: pairs from words[4..].  Reject odd-length tails
    // (each option needs a value) up front so the loop below can't
    // silently drop the last option / value.
    if (((words.len - 4) & 1) != 0) {
        raise_ns_error("missing value to go with key");
        return 0;
    }
    var i: u32 = 4;
    while (i + 1 < words.len) : (i += 2) {
        const key = obj_ensure_string(words[i]);
        const val_obj = words[i + 1];
        const val = obj_ensure_string(val_obj);
        if (slice_eq_ns(key.ptr, key.len, "-map")) {
            if (val.len == 0) {
                if (rec.map_obj != 0) {
                    obj_mod.tcl_obj_release(rec.map_obj);
                    rec.map_obj = 0;
                }
            } else {
                const n = obj_mod.list_count_elements(val.ptr, val.len);
                if (@rem(n, 2) != 0) {
                    raise_ns_error("missing value to go with key");
                    return 0;
                }
                // Validate non-empty impls and auto-qualify the
                // first impl element against the ensemble's target
                // namespace — mirrors the ``create`` path so a
                // subsequent ``configure -map`` read returns the
                // FQ form (namespace-46.1 semantics).
                var k: i64 = 1;
                while (k < n) : (k += 2) {
                    const impl = obj_mod.list_element_at(val.ptr, val.len, k);
                    if (impl.len == 0) {
                        raise_ns_error("ensemble subcommand implementations must be non-empty lists");
                        return 0;
                    }
                }
                const qualified = qualify_map_impls(val_obj, rec.target_ns);
                ensemble_set_obj_slot(&rec.map_obj, qualified);
                if (qualified != val_obj) obj_mod.tcl_obj_release(qualified);
            }
        } else if (slice_eq_ns(key.ptr, key.len, "-subcommands") or slice_eq_ns(key.ptr, key.len, "-subcommand")) {
            if (val.len == 0) {
                if (rec.sub_obj != 0) {
                    obj_mod.tcl_obj_release(rec.sub_obj);
                    rec.sub_obj = 0;
                }
            } else {
                ensemble_set_obj_slot(&rec.sub_obj, val_obj);
            }
        } else if (slice_eq_ns(key.ptr, key.len, "-unknown")) {
            if (val.len == 0) {
                if (rec.unknown_obj != 0) {
                    obj_mod.tcl_obj_release(rec.unknown_obj);
                    rec.unknown_obj = 0;
                }
            } else {
                ensemble_set_obj_slot(&rec.unknown_obj, val_obj);
            }
        } else if (slice_eq_ns(key.ptr, key.len, "-parameters")) {
            if (val.len == 0) {
                if (rec.params_obj != 0) {
                    obj_mod.tcl_obj_release(rec.params_obj);
                    rec.params_obj = 0;
                }
            } else {
                ensemble_set_obj_slot(&rec.params_obj, val_obj);
            }
        } else if (slice_eq_ns(key.ptr, key.len, "-prefixes")) {
            const b = obj_get_int(val_obj);
            rec.prefixes = if (b != 0) 1 else 0;
        } else {
            // Unknown option — reference Tcl errors rather than
            // silently dropping.  Match its ``bad option`` shape.
            raise_unknown_ensemble_option(key.ptr, key.len);
            return 0;
        }
    }
    return obj_new_string(0, 0);
}

fn render_ensemble_config(rec: *EnsembleRec) i32 {
    // Build ``-map {…} -namespace ::ns -parameters {…} -prefixes N
    // -subcommands {…} -unknown {…}``.
    var bufs: [6]i32 = .{ 0, 0, 0, 0, 0, 0 };
    const keys: [6][]const u8 = .{ "-map", "-namespace", "-parameters", "-prefixes", "-subcommands", "-unknown" };
    bufs[0] = if (rec.map_obj != 0) brace_wrap(rec.map_obj) else obj_new_string_copy_str("{}");
    {
        const ns_full = tcl_ns.ns_full_name(rec.target_ns);
        bufs[1] = obj_mod.obj_new_string(@bitCast(ns_full.ptr), @bitCast(ns_full.len));
    }
    bufs[2] = if (rec.params_obj != 0) brace_wrap(rec.params_obj) else obj_new_string_copy_str("{}");
    bufs[3] = if (rec.prefixes != 0) obj_new_string_copy_str("1") else obj_new_string_copy_str("0");
    bufs[4] = if (rec.sub_obj != 0) brace_wrap(rec.sub_obj) else obj_new_string_copy_str("{}");
    bufs[5] = if (rec.unknown_obj != 0) brace_wrap(rec.unknown_obj) else obj_new_string_copy_str("{}");
    // Compute total.
    var total: u32 = 0;
    var i: u32 = 0;
    while (i < 6) : (i += 1) {
        if (i > 0) total += 1;
        total += @as(u32, @intCast(keys[i].len)) + 1; // key + space
        const s = obj_ensure_string(bufs[i]);
        total += s.len;
    }
    const buf = alloc(total);
    if (buf == 0) return obj_new_string(0, 0);
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    i = 0;
    while (i < 6) : (i += 1) {
        if (i > 0) {
            dst[off] = ' ';
            off += 1;
        }
        for (keys[i]) |c| {
            dst[off] = c;
            off += 1;
        }
        dst[off] = ' ';
        off += 1;
        const s = obj_ensure_string(bufs[i]);
        if (s.len > 0) {
            const sp: [*]const u8 = @ptrFromInt(s.ptr);
            for (0..s.len) |k| dst[off + k] = sp[k];
            off += s.len;
        }
    }
    // Release the temporary value-formatted objs.
    i = 0;
    while (i < 6) : (i += 1) if (bufs[i] != 0) obj_mod.tcl_obj_release(bufs[i]);
    return obj_mod.obj_new_string_take(buf, off, total);
}

fn retained_or_empty(slot: i32) i32 {
    if (slot == 0) return obj_new_string(0, 0);
    obj_mod.tcl_obj_retain(slot);
    return slot;
}

fn read_ensemble_option(rec: *EnsembleRec, key_obj: i32) i32 {
    const key = obj_ensure_string(key_obj);
    if (slice_eq_ns(key.ptr, key.len, "-map")) return retained_or_empty(rec.map_obj);
    if (slice_eq_ns(key.ptr, key.len, "-namespace")) {
        const ns_full = tcl_ns.ns_full_name(rec.target_ns);
        return obj_mod.obj_new_string(@bitCast(ns_full.ptr), @bitCast(ns_full.len));
    }
    if (slice_eq_ns(key.ptr, key.len, "-parameters")) return retained_or_empty(rec.params_obj);
    if (slice_eq_ns(key.ptr, key.len, "-prefixes")) {
        return if (rec.prefixes != 0) obj_new_string_copy_str("1") else obj_new_string_copy_str("0");
    }
    if (slice_eq_ns(key.ptr, key.len, "-subcommands")) return retained_or_empty(rec.sub_obj);
    if (slice_eq_ns(key.ptr, key.len, "-unknown")) return retained_or_empty(rec.unknown_obj);
    return obj_new_string(0, 0);
}

/// Wrap a TclObj's string in ``{…}`` braces for the ``configure``
/// emit.  Empty becomes ``{}``.  Caller releases.
fn brace_wrap(value: i32) i32 {
    const s = obj_ensure_string(value);
    if (s.len == 0) return obj_new_string_copy_str("{}");
    const total: u32 = s.len + 2;
    const buf = alloc(total);
    if (buf == 0) return obj_new_string(0, 0);
    const dst: [*]u8 = @ptrFromInt(buf);
    dst[0] = '{';
    const sp: [*]const u8 = @ptrFromInt(s.ptr);
    for (0..s.len) |i| dst[1 + i] = sp[i];
    dst[total - 1] = '}';
    return obj_mod.obj_new_string_take(buf, total, total);
}

fn raise_unknown_ensemble_option(name_ptr: u32, name_len: u32) void {
    const catch_mod = @import("../interp/tcl_catch.zig");
    const prefix: []const u8 = "bad option \"";
    const suffix: []const u8 = "\": must be -map, -namespace, -parameters, -prefixes, -subcommands, or -unknown";
    const total: u32 = @as(u32, @intCast(prefix.len)) + name_len + @as(u32, @intCast(suffix.len));
    const buf = alloc(total);
    if (buf == 0) return;
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    if (name_len > 0) {
        const np: [*]const u8 = @ptrFromInt(name_ptr);
        for (0..name_len) |i| dst[off + i] = np[i];
        off += name_len;
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    catch_mod.tcl_cmd_error(obj_mod.obj_new_string_take(buf, total, total));
}

fn raise_not_an_ensemble(name: i32) void {
    const catch_mod = @import("../interp/tcl_catch.zig");
    const sn = obj_ensure_string(name);
    const prefix: []const u8 = "\"";
    const suffix: []const u8 = "\" is not an ensemble command";
    const total: u32 = @intCast(prefix.len + sn.len + suffix.len);
    const buf = alloc(total);
    if (buf == 0) return;
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    if (sn.len > 0) {
        const np: [*]const u8 = @ptrFromInt(sn.ptr);
        for (0..sn.len) |i| dst[off + i] = np[i];
        off += sn.len;
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    catch_mod.tcl_cmd_error(obj_mod.obj_new_string_take(buf, total, total));
}

// -- Ensemble support (Tcl 9 ``namespace ensemble``) -----------------------
//
// Each ensemble is a Command bucket whose ``CMD_ENSEMBLE`` flag is set
// and whose ``OFF_PARAMS_OBJ`` slot holds a pointer to an
// :type:`EnsembleRec`.  The proc dispatch fast path in
// ``eval_proc_call_bucket`` detects the flag and routes invocations
// through :func:`dispatch_ensemble`.

fn std_fmt_u32(buf: *[16]u8, v: u32) u32 {
    if (v == 0) {
        buf[0] = '0';
        return 1;
    }
    var i: u32 = 0;
    var x = v;
    while (x > 0) : (i += 1) {
        buf[i] = '0' + @as(u8, @intCast(x % 10));
        x /= 10;
    }
    var a: u32 = 0;
    var b: u32 = i - 1;
    while (a < b) {
        const t = buf[a];
        buf[a] = buf[b];
        buf[b] = t;
        a += 1;
        b -= 1;
    }
    return i;
}

/// Heap record describing a single ensemble command.  Owned by the
/// Command bucket via ``OFF_PARAMS_OBJ``; lifetime ends when the
/// owning Command is deleted (``rename ENS ""``) or its namespace
/// torn down.  Retains the TclObj fields it holds — the create /
/// configure paths balance retain/release around mutations.
const EnsembleRec = extern struct {
    target_ns: u32, // *Namespace
    map_obj: i32, // {key impl key impl ...} or 0
    sub_obj: i32, // {sub sub ...} or 0
    unknown_obj: i32, // unknown handler cmd-prefix or 0
    params_obj: i32, // formal parameter list or 0
    prefixes: u32, // boolean: allow prefix matching (default 1)
};

const ENS_REC_SIZE: u32 = @sizeOf(EnsembleRec);

fn ensemble_rec_alloc() u32 {
    const addr = alloc(ENS_REC_SIZE);
    const r: *EnsembleRec = @ptrFromInt(addr);
    r.target_ns = 0;
    r.map_obj = 0;
    r.sub_obj = 0;
    r.unknown_obj = 0;
    r.params_obj = 0;
    r.prefixes = 1;
    return addr;
}

fn ensemble_rec_release(addr: u32) void {
    if (addr == 0) return;
    const r: *EnsembleRec = @ptrFromInt(addr);
    if (r.map_obj != 0) obj_mod.tcl_obj_release(r.map_obj);
    if (r.sub_obj != 0) obj_mod.tcl_obj_release(r.sub_obj);
    if (r.unknown_obj != 0) obj_mod.tcl_obj_release(r.unknown_obj);
    if (r.params_obj != 0) obj_mod.tcl_obj_release(r.params_obj);
    obj_mod.free_sized(addr, ENS_REC_SIZE);
}

/// Walk ``-map`` value pairs and rewrite each impl's first element
/// to a fully-qualified name against *target_ns*.  Tcl 9 stores the
/// FQ form so subsequent ``configure -map`` reads return what
/// reference Tcl reports.  Returns a fresh owning TclObj when any
/// element needed qualification; otherwise returns the input as-is
/// (caller must not release in that case).
fn qualify_map_impls(map_obj: i32, target_ns: u32) i32 {
    if (target_ns == 0) return map_obj;
    const ns_full = tcl_ns.ns_full_name(target_ns);
    if (ns_full.len < 2) return map_obj;
    const ms = obj_ensure_string(map_obj);
    if (ms.len == 0) return map_obj;
    const n = obj_mod.list_count_elements(ms.ptr, ms.len);
    if (n <= 0 or @rem(n, 2) != 0) return map_obj;
    // Build a new list element-by-element.  Use a Tcl-list helper to
    // properly quote elements containing whitespace.
    var i: i64 = 0;
    var result: i32 = obj_mod.obj_new_string(0, 0);
    while (i < n) : (i += 2) {
        const k_el = obj_mod.list_element_at(ms.ptr, ms.len, i);
        const v_el = obj_mod.list_element_at(ms.ptr, ms.len, i + 1);
        const key_obj = obj_mod.obj_new_string(@bitCast(ms.ptr + k_el.start), @bitCast(k_el.len));
        // Impl is itself a Tcl list (``cmd ?arg ...?``).  Qualify only
        // the FIRST element.
        const impl_ptr = ms.ptr + v_el.start;
        const impl_len = v_el.len;
        const impl_n = obj_mod.list_count_elements(impl_ptr, impl_len);
        var impl_obj: i32 = 0;
        if (impl_n > 0) {
            const first = obj_mod.list_element_at(impl_ptr, impl_len, 0);
            const fp: [*]const u8 = @ptrFromInt(impl_ptr + first.start);
            const already_fq = first.len >= 2 and fp[0] == ':' and fp[1] == ':';
            if (already_fq) {
                impl_obj = obj_mod.obj_new_string(@bitCast(impl_ptr), @bitCast(impl_len));
            } else {
                // Build ``<ns_full>::<first> rest...``.
                const list_mod = @import("../valtypes/tcl_list.zig");
                const fq_first = ns_join_name(ns_full, impl_ptr + first.start, first.len);
                var built: i32 = list_mod.tcl_list(obj_mod.obj_new_string(0, 0), fq_first);
                obj_mod.tcl_obj_release(fq_first);
                var j: i64 = 1;
                while (j < impl_n) : (j += 1) {
                    const rest_el = obj_mod.list_element_at(impl_ptr, impl_len, j);
                    const rest_obj = obj_mod.obj_new_string(@bitCast(impl_ptr + rest_el.start), @bitCast(rest_el.len));
                    const next = list_mod.tcl_list(built, rest_obj);
                    obj_mod.tcl_obj_release(built);
                    obj_mod.tcl_obj_release(rest_obj);
                    built = next;
                }
                impl_obj = built;
            }
        } else {
            impl_obj = obj_mod.obj_new_string(0, 0);
        }
        const list_mod = @import("../valtypes/tcl_list.zig");
        const r1 = list_mod.tcl_list(result, key_obj);
        obj_mod.tcl_obj_release(result);
        obj_mod.tcl_obj_release(key_obj);
        const r2 = list_mod.tcl_list(r1, impl_obj);
        obj_mod.tcl_obj_release(r1);
        obj_mod.tcl_obj_release(impl_obj);
        result = r2;
    }
    return result;
}

fn ensemble_set_obj_slot(slot: *i32, new_val: i32) void {
    if (new_val != 0) obj_mod.tcl_obj_retain(new_val);
    const old = slot.*;
    slot.* = new_val;
    if (old != 0) obj_mod.tcl_obj_release(old);
}

/// Look up a Command by qualified or bare name.  Returns 0 when not
/// found.  Used by ensemble-introspection paths.
fn ensemble_find_cmd(name_obj: i32) u32 {
    const sn = obj_ensure_string(name_obj);
    if (sn.len == 0 or sn.ptr == 0) return 0;
    return tcl_ns.ns_find_command(tcl_ns.ns_current(), sn.ptr, sn.len);
}

/// Read the EnsembleRec for *bucket*, or null if not an ensemble.
fn ensemble_rec_of(bucket: u32) ?*EnsembleRec {
    if (bucket == 0) return null;
    const flags: u32 = @bitCast(obj_mod.read_i32(bucket + procs.OFF_FLAGS));
    if ((flags & procs.CMD_ENSEMBLE) == 0) return null;
    const rec_addr: u32 = @bitCast(obj_mod.read_i32(bucket + procs.OFF_PARAMS_OBJ));
    if (rec_addr == 0) return null;
    return @ptrFromInt(rec_addr);
}

/// Append ``::`` then ``name`` to an existing FQN, producing a new
/// owning TclObj.  Used to render the default ensemble command name
/// (which inherits the namespace it was created in) and to qualify
/// ``-map`` impls that arrived as bare names.
fn ns_join_name(ns_full: anytype, simple_ptr: u32, simple_len: u32) i32 {
    const total: u32 = ns_full.len + 2 + simple_len;
    const buf = alloc(total);
    if (buf == 0) return 0;
    const dst: [*]u8 = @ptrFromInt(buf);
    const np: [*]const u8 = @ptrFromInt(ns_full.ptr);
    for (0..ns_full.len) |i| dst[i] = np[i];
    dst[ns_full.len] = ':';
    dst[ns_full.len + 1] = ':';
    if (simple_len > 0) {
        const sp: [*]const u8 = @ptrFromInt(simple_ptr);
        for (0..simple_len) |i| dst[ns_full.len + 2 + i] = sp[i];
    }
    return obj_mod.obj_new_string_take(buf, total, total);
}

fn eval_ns_ensemble_create(words: []const i32) i32 {
    // ``namespace ensemble create ?-OPT VAL?...`` — parse the option
    // list into a fresh EnsembleRec, then register a Command in the
    // ``-command`` namespace (default: current ns inherits its own
    // simple name) carrying the ENS_FLAG + EnsembleRec pointer.
    if (((words.len - 3) & 1) != 0) {
        raise_ns_error("wrong # args: should be \"namespace ensemble create ?option value ...?\"");
        return 0;
    }
    const rec_addr = ensemble_rec_alloc();
    const rec: *EnsembleRec = @ptrFromInt(rec_addr);
    rec.target_ns = tcl_ns.ns_current();
    var cmd_name: i32 = 0; // optional ``-command`` override
    var i: u32 = 3;
    while (i + 1 < words.len) : (i += 2) {
        const key = obj_ensure_string(words[i]);
        const val = obj_ensure_string(words[i + 1]);
        if (key.len == 0 or @as([*]const u8, @ptrFromInt(key.ptr))[0] != '-') {
            ensemble_rec_release(rec_addr);
            raise_ns_error("missing value to go with key");
            return 0;
        }
        if (slice_eq_ns(key.ptr, key.len, "-map")) {
            if (val.len > 0) {
                const n = obj_mod.list_count_elements(val.ptr, val.len);
                if (@rem(n, 2) != 0) {
                    ensemble_rec_release(rec_addr);
                    raise_ns_error("missing value to go with key");
                    return 0;
                }
                var k: i64 = 1;
                while (k < n) : (k += 2) {
                    const impl = obj_mod.list_element_at(val.ptr, val.len, k);
                    if (impl.len == 0) {
                        ensemble_rec_release(rec_addr);
                        raise_ns_error("ensemble subcommand implementations must be non-empty lists");
                        return 0;
                    }
                }
                // Tcl 9 auto-qualifies the first element of each impl
                // against the target namespace when it's not already
                // ``::``-anchored — so ``-map {A x}`` from inside
                // ``::ns`` stores ``A ::ns::x``.
                const qualified = qualify_map_impls(words[i + 1], rec.target_ns);
                ensemble_set_obj_slot(&rec.map_obj, qualified);
                if (qualified != words[i + 1]) obj_mod.tcl_obj_release(qualified);
            }
        } else if (slice_eq_ns(key.ptr, key.len, "-subcommands")) {
            if (val.len > 0) ensemble_set_obj_slot(&rec.sub_obj, words[i + 1]);
        } else if (slice_eq_ns(key.ptr, key.len, "-unknown")) {
            if (val.len > 0) ensemble_set_obj_slot(&rec.unknown_obj, words[i + 1]);
        } else if (slice_eq_ns(key.ptr, key.len, "-parameters")) {
            if (val.len > 0) ensemble_set_obj_slot(&rec.params_obj, words[i + 1]);
        } else if (slice_eq_ns(key.ptr, key.len, "-prefixes")) {
            const b = obj_get_int(words[i + 1]);
            rec.prefixes = if (b != 0) 1 else 0;
        } else if (slice_eq_ns(key.ptr, key.len, "-command")) {
            cmd_name = words[i + 1];
        } else if (slice_eq_ns(key.ptr, key.len, "-namespace")) {
            // Resolve the target namespace.  Bare name → child of
            // root; FQN → resolved from root.
            if (val.len > 0) {
                const ns_handle = tcl_ns.ns_create_from_fqn(val.ptr, val.len);
                if (ns_handle != 0) rec.target_ns = ns_handle;
            }
        }
    }
    if (i < words.len) {
        ensemble_rec_release(rec_addr);
        raise_ns_error("missing value to go with key");
        return 0;
    }

    // Determine the command name and target ns to register under.
    var register_ns: u32 = rec.target_ns;
    var simple_ptr: u32 = 0;
    var simple_len: u32 = 0;
    if (cmd_name != 0) {
        const cns = obj_ensure_string(cmd_name);
        const r = tcl_ns.ns_resolve_qualified_creating(tcl_ns.ns_current(), cns.ptr, cns.len);
        if (r.target_ns != 0 and r.simple_len > 0) {
            register_ns = r.target_ns;
            simple_ptr = r.simple_ptr;
            simple_len = r.simple_len;
        }
    } else {
        // Default: command name = simple-tail of the target namespace.
        const ns_full = tcl_ns.ns_full_name(rec.target_ns);
        // Strip the leading ``::`` then walk to the last ``::`` for
        // the simple name.
        var start: u32 = 0;
        if (ns_full.len >= 2) {
            const np: [*]const u8 = @ptrFromInt(ns_full.ptr);
            if (np[0] == ':' and np[1] == ':') start = 2;
        }
        const np2: [*]const u8 = @ptrFromInt(ns_full.ptr);
        var last: u32 = start;
        var k: u32 = start;
        while (k + 1 < ns_full.len) : (k += 1) {
            if (np2[k] == ':' and np2[k + 1] == ':') last = k + 2;
        }
        register_ns = rec.target_ns;
        if (last > start) {
            // The leaf name lives in the parent ns.  Walk back to find
            // the parent.
            const parent_full = obj_mod.obj_new_string(@bitCast(ns_full.ptr), @bitCast(last - 2));
            const parent_s = obj_ensure_string(parent_full);
            const parent_ns = tcl_ns.ns_create_from_fqn(parent_s.ptr, parent_s.len);
            obj_mod.tcl_obj_release(parent_full);
            if (parent_ns != 0) register_ns = parent_ns;
            simple_ptr = ns_full.ptr + last;
            simple_len = ns_full.len - last;
        } else {
            // Root-level ns ``::ns`` — register ``ns`` in root.
            register_ns = tcl_ns.ns_root();
            simple_ptr = ns_full.ptr + start;
            simple_len = ns_full.len - start;
        }
    }
    if (simple_len == 0 or register_ns == 0) {
        ensemble_rec_release(rec_addr);
        return obj_new_string(0, 0);
    }

    // Allocate / reuse the Command bucket.
    const existing = tcl_ns.ns_cmd_find(register_ns, simple_ptr, simple_len);
    var cmd: u32 = existing;
    if (cmd == 0) {
        cmd = procs.alloc_command_export(simple_ptr, simple_len);
        if (cmd == 0) {
            ensemble_rec_release(rec_addr);
            raise_ns_error("out of memory creating ensemble");
            return 0;
        }
        _ = tcl_ns.ns_cmd_put(register_ns, simple_ptr, simple_len, cmd);
    } else {
        // Re-use: if the existing bucket already held an ensemble
        // rec, release it before swapping in the new one.
        const old_flags: u32 = @bitCast(obj_mod.read_i32(cmd + procs.OFF_FLAGS));
        if ((old_flags & procs.CMD_ENSEMBLE) != 0) {
            const old_rec: u32 = @bitCast(obj_mod.read_i32(cmd + procs.OFF_PARAMS_OBJ));
            ensemble_rec_release(old_rec);
        }
    }
    obj_mod.write_i32(cmd + procs.OFF_FLAGS, @bitCast(procs.CMD_ENSEMBLE));
    obj_mod.write_i32(cmd + procs.OFF_PARAMS_OBJ, @bitCast(rec_addr));
    obj_mod.write_i32(cmd + procs.OFF_BODY_OBJ, 0);
    obj_mod.write_i32(cmd + procs.OFF_FUNC_IDX, 0);
    obj_mod.write_i32(cmd + procs.OFF_N_PARAMS, 0);
    obj_mod.write_i32(cmd + procs.OFF_ARGS_TAIL, 0);

    // Return the fully-qualified command name (the user-visible
    // identity of the new ensemble).  Build ``::<ns_full>::<simple>``.
    const ns_full = tcl_ns.ns_full_name(register_ns);
    var out_ptr: u32 = 0;
    var out_len: u32 = 0;
    if (ns_full.len > 2) {
        const total: u32 = ns_full.len + 2 + simple_len;
        const buf = alloc(total);
        if (buf == 0) return obj_new_string(0, 0);
        const dst: [*]u8 = @ptrFromInt(buf);
        const np: [*]const u8 = @ptrFromInt(ns_full.ptr);
        for (0..ns_full.len) |k| dst[k] = np[k];
        dst[ns_full.len] = ':';
        dst[ns_full.len + 1] = ':';
        const sp: [*]const u8 = @ptrFromInt(simple_ptr);
        for (0..simple_len) |k| dst[ns_full.len + 2 + k] = sp[k];
        out_ptr = buf;
        out_len = total;
    } else {
        // Root namespace ``::`` — ``::simple``.
        const total: u32 = 2 + simple_len;
        const buf = alloc(total);
        if (buf == 0) return obj_new_string(0, 0);
        const dst: [*]u8 = @ptrFromInt(buf);
        dst[0] = ':';
        dst[1] = ':';
        const sp: [*]const u8 = @ptrFromInt(simple_ptr);
        for (0..simple_len) |k| dst[2 + k] = sp[k];
        out_ptr = buf;
        out_len = total;
    }
    return obj_mod.obj_new_string_take(out_ptr, out_len, out_len);
}

/// Public entry: dispatch an ensemble call.  Invoked from
/// :func:`eval_proc_call_bucket` when a Command has ``CMD_ENSEMBLE``
/// set.  ``words`` is the full argv (``words[0]`` is the invoked
/// ensemble name, ``words[1]`` is the subcommand, the rest are
/// extra args).
pub fn dispatch_ensemble(bucket: i32, words: []const i32) i32 {
    const rec = ensemble_rec_of(@bitCast(bucket)) orelse return 0;
    if (words.len < 2) {
        raise_ensemble_wrong_args(words[0], rec);
        return 0;
    }
    const sub = obj_ensure_string(words[1]);
    if (sub.len == 0) {
        raise_ensemble_wrong_args(words[0], rec);
        return 0;
    }

    // 1. Resolve subcommand → impl.
    var impl_obj: i32 = 0;
    if (rec.map_obj != 0) {
        const ms = obj_ensure_string(rec.map_obj);
        const n = obj_mod.list_count_elements(ms.ptr, ms.len);
        if (n > 0 and @rem(n, 2) == 0) {
            const matched = ensemble_lookup_map(ms.ptr, ms.len, n, sub.ptr, sub.len, rec.prefixes);
            if (matched > 0) {
                const e = obj_mod.list_element_at(ms.ptr, ms.len, matched);
                impl_obj = obj_mod.obj_new_string(@bitCast(ms.ptr + e.start), @bitCast(e.len));
            } else if (matched == 0) {
                // No match — fall through to -unknown / no-export error
            }
        }
    }
    if (impl_obj == 0) {
        // Try exported commands of the target ns.
        impl_obj = ensemble_lookup_exported(rec, sub.ptr, sub.len);
    }
    if (impl_obj == 0) {
        // -unknown handler?
        if (rec.unknown_obj != 0) {
            return invoke_ensemble_unknown(bucket, rec, words);
        }
        raise_ensemble_unknown_sub(words[0], sub.ptr, sub.len, rec);
        return 0;
    }
    defer obj_mod.tcl_obj_release(impl_obj);

    // 2. Build the full call: impl-elements + words[2..].
    return invoke_ensemble_impl(impl_obj, words);
}

/// Match *sub* against the keys of the ``-map`` list.  Returns the
/// list index of the matched IMPL element (odd index) when found,
/// 0 on miss, -1 on ambiguous prefix when ``prefixes`` is set.
/// ``n`` is the pre-computed element count of *map*.
fn ensemble_lookup_map(
    map_ptr: u32,
    map_len: u32,
    n: i64,
    sub_ptr: u32,
    sub_len: u32,
    prefixes: u32,
) i64 {
    var exact: i64 = -2;
    var prefix_match: i64 = -2;
    var prefix_count: u32 = 0;
    var i: i64 = 0;
    while (i < n) : (i += 2) {
        const e = obj_mod.list_element_at(map_ptr, map_len, i);
        if (e.len == sub_len and bytes_equal(map_ptr + e.start, sub_ptr, sub_len)) {
            exact = i + 1;
            break;
        }
        if (prefixes != 0 and e.len > sub_len and bytes_equal(map_ptr + e.start, sub_ptr, sub_len)) {
            prefix_match = i + 1;
            prefix_count += 1;
        }
    }
    if (exact != -2) return exact;
    if (prefix_count == 1) return prefix_match;
    return 0;
}

fn bytes_equal(a: u32, b: u32, n: u32) bool {
    if (n == 0) return true;
    const ap: [*]const u8 = @ptrFromInt(a);
    const bp: [*]const u8 = @ptrFromInt(b);
    var i: u32 = 0;
    while (i < n) : (i += 1) {
        if (ap[i] != bp[i]) return false;
    }
    return true;
}

/// Walk the target ns's exported commands (or the ``-subcommands``
/// override list) for a match against *sub*.  Returns a fresh TclObj
/// with the FQ implementation name on hit, 0 on miss.
fn ensemble_lookup_exported(rec: *EnsembleRec, sub_ptr: u32, sub_len: u32) i32 {
    const ns = rec.target_ns;
    if (ns == 0) return 0;
    // Build candidate name list.
    if (rec.sub_obj != 0) {
        const ss = obj_ensure_string(rec.sub_obj);
        return ensemble_match_in_list(ns, ss.ptr, ss.len, sub_ptr, sub_len, rec.prefixes);
    }
    // No explicit -subcommands → use the namespace's export patterns.
    return ensemble_match_in_exports(ns, sub_ptr, sub_len, rec.prefixes);
}

fn ensemble_match_in_list(
    ns: u32,
    list_ptr: u32,
    list_len: u32,
    sub_ptr: u32,
    sub_len: u32,
    prefixes: u32,
) i32 {
    const n = obj_mod.list_count_elements(list_ptr, list_len);
    if (n <= 0) return 0;
    var i: i64 = 0;
    var exact_off: i64 = -1;
    var prefix_off: i64 = -1;
    var prefix_count: u32 = 0;
    while (i < n) : (i += 1) {
        const e = obj_mod.list_element_at(list_ptr, list_len, i);
        if (e.len == sub_len and bytes_equal(list_ptr + e.start, sub_ptr, sub_len)) {
            exact_off = i;
            break;
        }
        if (prefixes != 0 and e.len > sub_len and bytes_equal(list_ptr + e.start, sub_ptr, sub_len)) {
            prefix_off = i;
            prefix_count += 1;
        }
    }
    const which = if (exact_off >= 0) exact_off else if (prefix_count == 1) prefix_off else -1;
    if (which < 0) return 0;
    const e = obj_mod.list_element_at(list_ptr, list_len, which);
    // Qualify against target ns.
    const ns_full = tcl_ns.ns_full_name(ns);
    return ns_join_name(ns_full, list_ptr + e.start, e.len);
}

fn ensemble_match_in_exports(ns: u32, sub_ptr: u32, sub_len: u32, prefixes: u32) i32 {
    const namespace: *tcl_ns.Namespace = @ptrFromInt(ns);
    if (namespace.cmd_table.buf == 0) return 0;
    // Two passes — first looks for an exact match; second falls back
    // to a unique-prefix match (when ``-prefixes`` is on).  Reference
    // Tcl resolves exact subcommand names eagerly so that a longer
    // exported name can't shadow a literal exact spelling.
    var exact_np: u32 = 0;
    var exact_nl: u32 = 0;
    var prefix_np: u32 = 0;
    var prefix_nl: u32 = 0;
    var prefix_count: u32 = 0;
    var i: u32 = 0;
    while (i < namespace.cmd_table.cap) : (i += 1) {
        const bucket = namespace.cmd_table.buf + i * tcl_ns.NS_BUCKET_SIZE;
        const np: u32 = @bitCast(obj_mod.read_i32(bucket));
        if (np == 0) continue;
        const handle: u32 = @bitCast(obj_mod.read_i32(bucket + tcl_ns.OFF_HANDLE));
        if (handle == 0) continue; // dead bucket (rename → empty)
        const nl: u32 = @bitCast(obj_mod.read_i32(bucket + 4));
        if (!tcl_ns.ns_export_matches(ns, np, nl)) continue;
        if (nl == sub_len and bytes_equal(np, sub_ptr, sub_len)) {
            exact_np = np;
            exact_nl = nl;
            break;
        }
        if (prefixes != 0 and nl > sub_len and bytes_equal(np, sub_ptr, sub_len)) {
            prefix_np = np;
            prefix_nl = nl;
            prefix_count += 1;
        }
    }
    if (exact_nl != 0) {
        const ns_full = tcl_ns.ns_full_name(ns);
        return ns_join_name(ns_full, exact_np, exact_nl);
    }
    if (prefix_count == 1) {
        const ns_full = tcl_ns.ns_full_name(ns);
        return ns_join_name(ns_full, prefix_np, prefix_nl);
    }
    return 0;
}

/// Combine the ensemble impl prefix (a Tcl list) with the remaining
/// call args and invoke the result.
fn invoke_ensemble_impl(impl_obj: i32, words: []const i32) i32 {
    const is = obj_ensure_string(impl_obj);
    if (is.len == 0) return 0;
    const interp = @import("../interp/tcl_interp.zig");
    // The impl is a list ``cmd ?arg ...?``.  Split into element
    // TclObjs, then append words[2..].
    const n_impl = obj_mod.list_count_elements(is.ptr, is.len);
    const tail = if (words.len >= 2) words.len - 2 else 0;
    const total_args: u32 = @intCast(@as(i64, n_impl) + @as(i64, @intCast(tail)));
    if (total_args == 0) return 0;
    const argv = obj_mod.alloc(total_args * 4);
    if (argv == 0) return 0;
    defer obj_mod.free_sized(argv, total_args * 4);
    var idx: u32 = 0;
    var k: i64 = 0;
    while (k < n_impl) : (k += 1) {
        const e = obj_mod.list_element_at(is.ptr, is.len, k);
        const w = obj_mod.obj_new_string(@bitCast(is.ptr + e.start), @bitCast(e.len));
        obj_mod.write_i32(argv + idx * 4, w);
        idx += 1;
    }
    var wi: u32 = 2;
    while (wi < words.len) : (wi += 1) {
        obj_mod.tcl_obj_retain(words[wi]);
        obj_mod.write_i32(argv + idx * 4, words[wi]);
        idx += 1;
    }
    // Build a slice for interp.eval_command — it expects a []const i32.
    const argv_slice: []const i32 = @as([*]const i32, @ptrFromInt(argv))[0..total_args];
    const result = interp.eval_command(argv_slice);
    // Release each argv slot (we owned a retained ref to each).
    var j: u32 = 0;
    while (j < total_args) : (j += 1) {
        const v = obj_mod.read_i32(argv + j * 4);
        if (v != 0) obj_mod.tcl_obj_release(v);
    }
    return result;
}

fn invoke_ensemble_unknown(bucket: i32, rec: *EnsembleRec, words: []const i32) i32 {
    _ = bucket;
    // Build call: unknown_prefix + ensemble_name + words[1..]
    const us = obj_ensure_string(rec.unknown_obj);
    const interp = @import("../interp/tcl_interp.zig");
    const n_pref = obj_mod.list_count_elements(us.ptr, us.len);
    // Build the resolved ensemble FQN (words[0] may be a short name).
    const total_args: u32 = @intCast(@as(i64, n_pref) + 1 + @as(i64, @intCast(words.len - 1)));
    const argv = obj_mod.alloc(total_args * 4);
    if (argv == 0) return 0;
    defer obj_mod.free_sized(argv, total_args * 4);
    var idx: u32 = 0;
    var k: i64 = 0;
    while (k < n_pref) : (k += 1) {
        const e = obj_mod.list_element_at(us.ptr, us.len, k);
        const w = obj_mod.obj_new_string(@bitCast(us.ptr + e.start), @bitCast(e.len));
        obj_mod.write_i32(argv + idx * 4, w);
        idx += 1;
    }
    // Ensemble name (use words[0] as-is)
    obj_mod.tcl_obj_retain(words[0]);
    obj_mod.write_i32(argv + idx * 4, words[0]);
    idx += 1;
    var wi: u32 = 1;
    while (wi < words.len) : (wi += 1) {
        obj_mod.tcl_obj_retain(words[wi]);
        obj_mod.write_i32(argv + idx * 4, words[wi]);
        idx += 1;
    }
    const argv_slice: []const i32 = @as([*]const i32, @ptrFromInt(argv))[0..total_args];
    const result = interp.eval_command(argv_slice);
    var j: u32 = 0;
    while (j < total_args) : (j += 1) {
        const v = obj_mod.read_i32(argv + j * 4);
        if (v != 0) obj_mod.tcl_obj_release(v);
    }
    return result;
}

fn raise_ensemble_wrong_args(name: i32, rec: *EnsembleRec) void {
    _ = rec;
    const catch_mod = @import("../interp/tcl_catch.zig");
    const prefix: []const u8 = "wrong # args: should be \"";
    const suffix: []const u8 = " subcommand ?arg ...?\"";
    const sn = obj_ensure_string(name);
    const total: u32 = @intCast(prefix.len + sn.len + suffix.len);
    const buf = alloc(total);
    if (buf == 0) return;
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    if (sn.len > 0) {
        const np: [*]const u8 = @ptrFromInt(sn.ptr);
        for (0..sn.len) |i| dst[off + i] = np[i];
        off += sn.len;
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    catch_mod.tcl_cmd_error(obj_mod.obj_new_string_take(buf, total, total));
}

fn raise_ensemble_unknown_sub(name: i32, sub_ptr: u32, sub_len: u32, rec: *EnsembleRec) void {
    const catch_mod = @import("../interp/tcl_catch.zig");
    // Compute the candidate list for the error message.
    const avail = compute_available_subcmds(rec);
    defer if (avail != 0) obj_mod.tcl_obj_release(avail);
    const avail_s = obj_ensure_string(avail);
    const prefix: []const u8 = "unknown ";
    var middle1: []const u8 = "subcommand \"";
    if (rec.prefixes != 0) {
        middle1 = "or ambiguous subcommand \"";
    }
    var suffix: []const u8 = undefined;
    var suffix_buf: u32 = 0;
    var suffix_buf_len: u32 = 0;
    if (avail_s.len > 0) {
        const middle2: []const u8 = "\": must be ";
        // Build the full middle+avail tail dynamically.
        suffix_buf_len = @intCast(middle2.len + avail_s.len);
        suffix_buf = alloc(suffix_buf_len);
        if (suffix_buf == 0) return;
        const sb: [*]u8 = @ptrFromInt(suffix_buf);
        var off: u32 = 0;
        for (middle2) |c| {
            sb[off] = c;
            off += 1;
        }
        const ap: [*]const u8 = @ptrFromInt(avail_s.ptr);
        for (0..avail_s.len) |k| sb[off + k] = ap[k];
        suffix = sb[0..suffix_buf_len];
    } else {
        const ns_full = tcl_ns.ns_full_name(rec.target_ns);
        const msg2: []const u8 = "\": namespace ";
        const msg3: []const u8 = " does not export any commands";
        suffix_buf_len = @intCast(msg2.len + ns_full.len + msg3.len);
        suffix_buf = alloc(suffix_buf_len);
        if (suffix_buf == 0) return;
        const sb: [*]u8 = @ptrFromInt(suffix_buf);
        var off: u32 = 0;
        for (msg2) |c| {
            sb[off] = c;
            off += 1;
        }
        const np: [*]const u8 = @ptrFromInt(ns_full.ptr);
        for (0..ns_full.len) |k| sb[off + k] = np[k];
        off += ns_full.len;
        for (msg3) |c| {
            sb[off] = c;
            off += 1;
        }
        suffix = sb[0..suffix_buf_len];
        // Tighter ``unknown`` wording when nothing's exported (real
        // Tcl drops the "or ambiguous" prefix for this case).
        middle1 = "subcommand \"";
    }
    _ = name;
    const total: u32 = @intCast(prefix.len + middle1.len + sub_len + suffix.len);
    const buf = alloc(total);
    if (buf == 0) return;
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    for (middle1) |c| {
        dst[off] = c;
        off += 1;
    }
    if (sub_len > 0) {
        const sp: [*]const u8 = @ptrFromInt(sub_ptr);
        for (0..sub_len) |k| dst[off + k] = sp[k];
        off += sub_len;
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    obj_mod.free_sized(suffix_buf, suffix_buf_len);
    catch_mod.tcl_cmd_error(obj_mod.obj_new_string_take(buf, total, total));
}

/// Build a comma+`, or` separated list of available subcommands for
/// the diagnostic in :func:`raise_ensemble_unknown_sub`.
fn compute_available_subcmds(rec: *EnsembleRec) i32 {
    // 1. -map keys
    return std_keys_to_obj(rec);
}

fn std_keys_to_obj(rec: *EnsembleRec) i32 {
    // Collect subcommand keys from -map first, then from -subcommands,
    // then from the target ns's exported commands.  Returns a fresh
    // owning TclObj containing the joined ``a, b, or c`` form used in
    // ``unknown subcommand`` diagnostics.
    var names: [128][2]u32 = undefined;
    var count: u32 = 0;
    if (rec.map_obj != 0) {
        const ms = obj_ensure_string(rec.map_obj);
        const n = obj_mod.list_count_elements(ms.ptr, ms.len);
        var i: i64 = 0;
        while (i < n) : (i += 2) {
            if (count >= 128) break;
            const e = obj_mod.list_element_at(ms.ptr, ms.len, i);
            names[count] = .{ ms.ptr + e.start, e.len };
            count += 1;
        }
    } else if (rec.sub_obj != 0) {
        const ss = obj_ensure_string(rec.sub_obj);
        const n = obj_mod.list_count_elements(ss.ptr, ss.len);
        var i: i64 = 0;
        while (i < n) : (i += 1) {
            if (count >= 128) break;
            const e = obj_mod.list_element_at(ss.ptr, ss.len, i);
            names[count] = .{ ss.ptr + e.start, e.len };
            count += 1;
        }
    } else if (rec.target_ns != 0) {
        const namespace: *tcl_ns.Namespace = @ptrFromInt(rec.target_ns);
        if (namespace.cmd_table.buf != 0) {
            var i: u32 = 0;
            while (i < namespace.cmd_table.cap) : (i += 1) {
                if (count >= 128) break;
                const bucket = namespace.cmd_table.buf + i * tcl_ns.NS_BUCKET_SIZE;
                const np: u32 = @bitCast(obj_mod.read_i32(bucket));
                if (np == 0) continue;
                // ``ns_cmd_clear`` (rename → empty target) zeroes the
                // handle slot but leaves the name in place, so the
                // export-match check alone would resurrect the dead
                // bucket.  Skip handle == 0 too.
                const handle: u32 = @bitCast(obj_mod.read_i32(bucket + tcl_ns.OFF_HANDLE));
                if (handle == 0) continue;
                const nl: u32 = @bitCast(obj_mod.read_i32(bucket + 4));
                if (!tcl_ns.ns_export_matches(rec.target_ns, np, nl)) continue;
                names[count] = .{ np, nl };
                count += 1;
            }
        }
    }
    if (count == 0) return obj_new_string(0, 0);
    // Sort names for stable output (alphabetic).
    sort_names(&names, count);
    // Build "a, or b, or c, or d" — Tcl 9's ``InfoCommandsCmd``
    // uses a uniform ", or " separator before each item past the
    // first (so 2-element lists also render with ", or " — see
    // namespace-46.2's expected ``must be x1, or x2``).
    var total: u32 = 0;
    var i: u32 = 0;
    while (i < count) : (i += 1) {
        if (i + 1 == count and count > 1) {
            total += 5; // ", or "
        } else if (i > 0) {
            total += 2; // ", "
        }
        total += names[i][1];
    }
    const buf = alloc(total);
    if (buf == 0) return obj_new_string(0, 0);
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    i = 0;
    while (i < count) : (i += 1) {
        if (i + 1 == count and count > 1) {
            dst[off] = ',';
            dst[off + 1] = ' ';
            dst[off + 2] = 'o';
            dst[off + 3] = 'r';
            dst[off + 4] = ' ';
            off += 5;
        } else if (i > 0) {
            dst[off] = ',';
            dst[off + 1] = ' ';
            off += 2;
        }
        const sp: [*]const u8 = @ptrFromInt(names[i][0]);
        for (0..names[i][1]) |k| dst[off + k] = sp[k];
        off += names[i][1];
    }
    return obj_mod.obj_new_string_take(buf, off, total);
}

fn sort_names(names: *[128][2]u32, count: u32) void {
    // Insertion sort — N <= 128, lexicographic compare.
    var i: u32 = 1;
    while (i < count) : (i += 1) {
        const cur = names[i];
        var j: i64 = @as(i64, i) - 1;
        while (j >= 0 and name_lt(cur, names[@intCast(j)])) : (j -= 1) {
            names[@intCast(j + 1)] = names[@intCast(j)];
        }
        names[@intCast(j + 1)] = cur;
    }
}

fn name_lt(a: [2]u32, b: [2]u32) bool {
    const ap: [*]const u8 = @ptrFromInt(a[0]);
    const bp: [*]const u8 = @ptrFromInt(b[0]);
    const lim = if (a[1] < b[1]) a[1] else b[1];
    var i: u32 = 0;
    while (i < lim) : (i += 1) {
        if (ap[i] < bp[i]) return true;
        if (ap[i] > bp[i]) return false;
    }
    return a[1] < b[1];
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
