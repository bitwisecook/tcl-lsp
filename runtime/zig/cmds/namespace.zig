// ``namespace`` — namespace management command (eval, export, import, forget,
// which, current, path, qualifiers, tail, parent, exists, children subcommands).

const rt          = @import("../tcl_runtime.zig");
const tcl_ns      = @import("../interp/tcl_ns.zig");
const procs       = @import("../interp/tcl_procs.zig");
const interp_impl = @import("../dispatch/tcl_cmd_interp.zig");
const obj_mod     = @import("../value/tcl_obj.zig");
const reg         = @import("../dispatch/tcl_cmd_registry.zig");
const tcl_string  = @import("../value/tcl_string.zig");

const str_eq              = @import("../value/tcl_chars.zig").str_eq;
const alloc               = rt.alloc;
const memcpy              = rt.memcpy;
const obj_new_string      = rt.obj_new_string;
const obj_new_string_copy = rt.obj_new_string_copy;
const obj_ensure_string   = rt.obj_ensure_string;
const obj_new_int         = rt.obj_new_int;
const obj_get_int         = rt.obj_get_int;

fn eval_namespace(words: []const i32) i32 {
    if (words.len >= 2) {
        const interp = @import("../interp/tcl_interp.zig");
        const sub = obj_ensure_string(words[1]);
        if (sub.len == 4 and sub.ptr != 0) {
            const sp: [*]const u8 = @ptrFromInt(sub.ptr);
            if (sp[0] == 'e' and sp[1] == 'v' and sp[2] == 'a' and sp[3] == 'l') {
                if (words.len < 4) return 0;
                const ns_obj_s = obj_ensure_string(words[2]);
                const target_ns = tcl_ns.ns_create_from_fqn(ns_obj_s.ptr, ns_obj_s.len);
                const saved_ns = tcl_ns.current_ns;
                tcl_ns.current_ns = target_ns;
                defer tcl_ns.current_ns = saved_ns;
                if (words.len == 4) {
                    const bs = obj_ensure_string(words[3]);
                    if (bs.len > 0) return interp.eval_script(bs.ptr, bs.len);
                    return 0;
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
                return interp.eval_script(buf, total);
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
                return 0;
            }
            if (sp6[0] == 'i' and sp6[1] == 'm' and sp6[2] == 'p' and sp6[3] == 'o' and sp6[4] == 'r' and sp6[5] == 't') {
                var ii: u32 = 2;
                while (ii < words.len) : (ii += 1) {
                    const is = obj_ensure_string(words[ii]);
                    if (is.len == 6 and is.ptr != 0) {
                        const isp: [*]const u8 = @ptrFromInt(is.ptr);
                        if (isp[0] == '-' and isp[1] == 'f' and isp[2] == 'o' and isp[3] == 'r' and isp[4] == 'c' and isp[5] == 'e') continue;
                    }
                    const created = tcl_ns.ns_import(tcl_ns.ns_current(), is.ptr, is.len);
                    var bk: u32 = 0;
                    while (bk < created) : (bk += 1) procs.proc_count_bump();
                }
                return 0;
            }
            if (sp6[0] == 'f' and sp6[1] == 'o' and sp6[2] == 'r' and sp6[3] == 'g' and sp6[4] == 'e' and sp6[5] == 't') {
                var fi: u32 = 2;
                var any_forgotten: u32 = 0;
                while (fi < words.len) : (fi += 1) {
                    const fs = obj_ensure_string(words[fi]);
                    any_forgotten += tcl_ns.ns_forget(tcl_ns.ns_current(), fs.ptr, fs.len);
                }
                if (any_forgotten > 0) procs.lru_invalidate_all();
                return 0;
            }
        }
        if (str_eq(@ptrFromInt(sub.ptr), sub.len, "which")) {
            return interp_impl.eval_namespace_which(words);
        }
        if (str_eq(@ptrFromInt(sub.ptr), sub.len, "current")) {
            const nf = tcl_ns.ns_full_name(tcl_ns.ns_current());
            return obj_new_string(@bitCast(nf.ptr), @bitCast(nf.len));
        }
        if (sub.len == 4 and sub.ptr != 0) {
            const sp4: [*]const u8 = @ptrFromInt(sub.ptr);
            if (sp4[0] == 'p' and sp4[1] == 'a' and sp4[2] == 't' and sp4[3] == 'h') {
                if (words.len < 3) return 0;
                const ls = obj_ensure_string(words[2]);
                const count = obj_mod.list_count_elements(ls.ptr, ls.len);
                if (count == 0) {
                    tcl_ns.ns_set_path(tcl_ns.ns_current(), 0, 0);
                    return 0;
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
                return 0;
            }
            // ``namespace tail name`` — return the last simple component.
            if (sp4[0] == 't' and sp4[1] == 'a' and sp4[2] == 'i' and sp4[3] == 'l') {
                if (words.len < 3) return obj_new_string(0, 0);
                const ns_s = obj_ensure_string(words[2]);
                return ns_tail(ns_s.ptr, ns_s.len);
            }
        }
        // ``namespace qualifiers name``
        if (str_eq(@ptrFromInt(sub.ptr), sub.len, "qualifiers")) {
            if (words.len < 3) return obj_new_string(0, 0);
            const ns_s = obj_ensure_string(words[2]);
            return ns_qualifiers(ns_s.ptr, ns_s.len);
        }
        // ``namespace parent ?name?``
        if (str_eq(@ptrFromInt(sub.ptr), sub.len, "parent")) {
            if (words.len >= 3) {
                const ns_s = obj_ensure_string(words[2]);
                return ns_parent(ns_s.ptr, ns_s.len);
            } else {
                const nf = tcl_ns.ns_full_name(tcl_ns.ns_current());
                return ns_parent(nf.ptr, nf.len);
            }
        }
        // ``namespace exists name``
        if (str_eq(@ptrFromInt(sub.ptr), sub.len, "exists")) {
            if (words.len < 3) return obj_new_int(0);
            const ns_s = obj_ensure_string(words[2]);
            return ns_exists(ns_s.ptr, ns_s.len);
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
            return ns_children(ctx_ns, pat_ptr, pat_len);
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
            return 0;
        }
        // ``namespace inscope ns body ?arg...?`` — like ``namespace eval`` but
        // evaluates with additional arguments appended.
        if (str_eq(@ptrFromInt(sub.ptr), sub.len, "inscope")) {
            if (words.len < 4) return 0;
            const ns_obj_s = obj_ensure_string(words[2]);
            const target_ns = tcl_ns.ns_create_from_fqn(ns_obj_s.ptr, ns_obj_s.len);
            const saved_ns = tcl_ns.current_ns;
            tcl_ns.current_ns = target_ns;
            defer tcl_ns.current_ns = saved_ns;
            const bs = obj_ensure_string(words[3]);
            if (words.len == 4) {
                if (bs.len > 0) return interp.eval_script(bs.ptr, bs.len);
                return 0;
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
            return interp.eval_script(buf, total);
        }
        // ``namespace code script`` — wrap with [namespace inscope current script]
        // For our purposes, return the script unchanged (no-op approximation).
        if (str_eq(@ptrFromInt(sub.ptr), sub.len, "code")) {
            if (words.len < 3) return obj_new_string(0, 0);
            return words[2];
        }
    }
    return 0;
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
        if (pat_len > 0 and !tcl_string.glob_match(pat_ptr, pat_len, fqn.ptr, fqn.len)) continue;
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
        if (pat_len > 0 and !tcl_string.glob_match(pat_ptr, pat_len, fqn.ptr, fqn.len)) continue;
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
    .{ .name = "namespace", .handler = &eval_namespace },
};
