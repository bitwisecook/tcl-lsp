// ``namespace`` — namespace management command (eval, export, import, forget,
// which, current, path subcommands).

const rt          = @import("../tcl_runtime.zig");
const tcl_ns      = @import("../tcl_ns.zig");
const procs       = @import("../tcl_procs.zig");
const interp_impl = @import("../tcl_cmd_interp.zig");
const obj_mod     = @import("../tcl_obj.zig");
const reg         = @import("../tcl_cmd_registry.zig");

const str_eq            = @import("../tcl_chars.zig").str_eq;
const alloc             = rt.alloc;
const memcpy            = rt.memcpy;
const obj_new_string    = rt.obj_new_string;
const obj_ensure_string = rt.obj_ensure_string;

fn eval_namespace(words: []const i32) i32 {
    if (words.len >= 2) {
        const interp = @import("../tcl_interp.zig");
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
        }
    }
    return 0;
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "namespace", .handler = &eval_namespace },
};
