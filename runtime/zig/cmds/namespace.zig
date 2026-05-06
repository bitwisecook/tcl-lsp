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

fn eval_namespace(words: []const i32) result_mod.InterpResult {
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
                            const target = resolve_ns(src_ns_ptr, src_ns_len);
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
                    any_forgotten += tcl_ns.ns_forget(tcl_ns.ns_current(), fs.ptr, fs.len);
                }
                if (any_forgotten > 0) procs.lru_invalidate_all();
                return result_mod.from_globals(0);
            }
        }
        if (str_eq(@ptrFromInt(sub.ptr), sub.len, "which")) {
            return result_mod.from_globals(interp_impl.eval_namespace_which(words));
        }
        if (str_eq(@ptrFromInt(sub.ptr), sub.len, "current")) {
            const nf = tcl_ns.ns_full_name(tcl_ns.ns_current());
            return result_mod.from_globals(obj_new_string(@bitCast(nf.ptr), @bitCast(nf.len)));
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
                    const r = tcl_ns.ns_resolve_qualified(tcl_ns.ns_current(), elt.start, elt.len);
                    var resolved: u32 = r.target_ns;
                    if (r.simple_len > 0 and r.target_ns != 0) {
                        const child = tcl_ns.ns_lookup(r.target_ns, r.simple_ptr, r.simple_len);
                        resolved = child;
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
        // ``namespace parent ?name?``
        if (str_eq(@ptrFromInt(sub.ptr), sub.len, "parent")) {
            if (words.len >= 3) {
                const ns_s = obj_ensure_string(words[2]);
                return result_mod.from_globals(ns_parent(ns_s.ptr, ns_s.len));
            } else {
                const nf = tcl_ns.ns_full_name(tcl_ns.ns_current());
                return result_mod.from_globals(ns_parent(nf.ptr, nf.len));
            }
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
                break :blk resolve_ns(cs.ptr, cs.len);
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
                ns_delete(ds.ptr, ds.len);
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
            // Idempotency check — already wrapped scripts pass
            // through.  Match both the canonical ``::namespace
            // inscope `` and the unqualified ``namespace inscope ``
            // forms, mirroring reference Tcl.
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
            if (ss.len >= 18) {
                const sp: [*]const u8 = @ptrFromInt(ss.ptr);
                const bare = "namespace inscope ";
                var ok: bool = true;
                for (0..bare.len) |k| if (sp[k] != bare[k]) {
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
    const ns: *const tcl_ns.Namespace = @ptrFromInt(h);
    const parent = ns.parent;
    if (parent == 0) return; // can't delete root
    const parent_ns: *tcl_ns.Namespace = @ptrFromInt(parent);
    if (parent_ns.child_table.buf == 0) return;
    // Find the bucket for this child and zero its value.
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
