// Tcl interpreter — string, array, and dict built-in commands.
//
// Extracted from tcl_interp.zig.  These implementations are purely
// functional (no eval_script recursion) so they live outside the cyclic
// eval core.  Called from tcl_interp.zig via @import.

const rt = @import("tcl_runtime.zig");
const frames = @import("tcl_frames.zig");

const obj_ensure_string = rt.obj_ensure_string;
const obj_new_string = rt.obj_new_string;
const obj_new_int = rt.obj_new_int;

const str_eq = @import("tcl_chars.zig").str_eq;

pub fn eval_string_cmd(words: []const i32) i32 {
    if (words.len < 3) return 0;
    const sub = obj_ensure_string(words[1]);
    const sp: [*]const u8 = @ptrFromInt(sub.ptr);
    if (str_eq(sp, sub.len, "length")) return rt.string_length(words[2]);
    if (str_eq(sp, sub.len, "index") and words.len >= 4) return rt.string_index(words[2], words[3]);
    if (str_eq(sp, sub.len, "range") and words.len >= 5) return rt.string_range(words[2], words[3], words[4]);
    if (str_eq(sp, sub.len, "compare") and words.len >= 4) return rt.string_compare(words[2], words[3]);
    if (str_eq(sp, sub.len, "equal") and words.len >= 4) return rt.string_equal(words[2], words[3]);
    if (str_eq(sp, sub.len, "match") and words.len >= 4) return rt.string_match(words[2], words[3]);
    if (str_eq(sp, sub.len, "map") and words.len >= 4) return rt.string_map(words[2], words[3]);
    if (str_eq(sp, sub.len, "trim")) {
        const chars = if (words.len >= 4) words[3] else 0;
        return rt.string_trim(words[2], chars);
    }
    if (str_eq(sp, sub.len, "trimleft")) {
        const chars = if (words.len >= 4) words[3] else 0;
        return rt.string_trimleft(words[2], chars);
    }
    if (str_eq(sp, sub.len, "trimright")) {
        const chars = if (words.len >= 4) words[3] else 0;
        return rt.string_trimright(words[2], chars);
    }
    if (str_eq(sp, sub.len, "first") and words.len >= 4) return rt.string_first(words[2], words[3]);
    if (str_eq(sp, sub.len, "last") and words.len >= 4) return rt.string_last(words[2], words[3]);
    if (str_eq(sp, sub.len, "toupper")) return rt.string_toupper(words[2]);
    if (str_eq(sp, sub.len, "tolower")) return rt.string_tolower(words[2]);
    if (str_eq(sp, sub.len, "reverse")) return rt.string_reverse(words[2]);
    if (str_eq(sp, sub.len, "repeat") and words.len >= 4) return rt.string_repeat(words[2], words[3]);
    if (str_eq(sp, sub.len, "replace") and words.len >= 6) return rt.string_replace(words[2], words[3], words[4], words[5]);
    if (str_eq(sp, sub.len, "is")) {
        // ``string is class ?-strict? ?-failindex var? str``
        // Find the class name (words[2]) and the final string arg.
        // Skip any -strict / -failindex flags and their args.
        if (words.len < 4) return obj_new_int(1); // empty string: non-strict default is 1
        const cls = obj_ensure_string(words[2]);
        const clsp: [*]const u8 = @ptrFromInt(cls.ptr);
        var str_idx: u32 = 3;
        while (str_idx + 1 < words.len) {
            const a = obj_ensure_string(words[str_idx]);
            const ap: [*]const u8 = @ptrFromInt(a.ptr);
            if (a.len > 0 and ap[0] == '-') {
                // -strict: no extra arg; -failindex: consumes next arg
                if (str_eq(ap, a.len, "-failindex")) str_idx += 1;
                str_idx += 1;
            } else break;
        }
        if (str_idx >= words.len) return obj_new_int(1);
        const sv = obj_ensure_string(words[str_idx]);
        if (sv.len == 0) {
            // non-strict: empty is 1 for all; strict: 0
            return obj_new_int(1);
        }
        const svp: [*]const u8 = @ptrFromInt(sv.ptr);
        if (str_eq(clsp, cls.len, "print")) {
            // printable: 0x20-0x7E ASCII, or any multibyte UTF-8
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                const b = svp[i];
                if (b >= 0x80) continue; // multibyte UTF-8 — treat as printable
                if (b < 0x20 or b == 0x7F) return obj_new_int(0);
            }
            return obj_new_int(1);
        }
        if (str_eq(clsp, cls.len, "alpha")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                const b = svp[i];
                if (b >= 0x80) { i += 1; continue; }
                if (!((b >= 'a' and b <= 'z') or (b >= 'A' and b <= 'Z'))) return obj_new_int(0);
            }
            return obj_new_int(1);
        }
        if (str_eq(clsp, cls.len, "digit")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                if (svp[i] < '0' or svp[i] > '9') return obj_new_int(0);
            }
            return obj_new_int(1);
        }
        if (str_eq(clsp, cls.len, "alnum")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                const b = svp[i];
                if (b >= 0x80) { i += 1; continue; }
                if (!((b >= 'a' and b <= 'z') or (b >= 'A' and b <= 'Z') or (b >= '0' and b <= '9'))) return obj_new_int(0);
            }
            return obj_new_int(1);
        }
        if (str_eq(clsp, cls.len, "space") or str_eq(clsp, cls.len, "whitespace")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                const b = svp[i];
                if (b != ' ' and b != '\t' and b != '\n' and b != '\r' and b != 0x0C and b != 0x0B) return obj_new_int(0);
            }
            return obj_new_int(1);
        }
        if (str_eq(clsp, cls.len, "integer")) {
            var i: u32 = 0;
            while (i < sv.len and (svp[i] == ' ' or svp[i] == '\t')) i += 1;
            if (i < sv.len and (svp[i] == '+' or svp[i] == '-')) i += 1;
            if (i < sv.len and svp[i] == '0' and i + 1 < sv.len and (svp[i+1] == 'x' or svp[i+1] == 'X')) {
                i += 2;
                if (i >= sv.len) return obj_new_int(0);
                while (i < sv.len) : (i += 1) {
                    const b = svp[i];
                    if (!((b >= '0' and b <= '9') or (b >= 'a' and b <= 'f') or (b >= 'A' and b <= 'F'))) return obj_new_int(0);
                }
                return obj_new_int(1);
            }
            if (i >= sv.len) return obj_new_int(0);
            while (i < sv.len) : (i += 1) {
                if (svp[i] < '0' or svp[i] > '9') return obj_new_int(0);
            }
            return obj_new_int(1);
        }
        if (str_eq(clsp, cls.len, "boolean")) {
            if (str_eq(svp, sv.len, "1") or str_eq(svp, sv.len, "0") or
                str_eq(svp, sv.len, "true") or str_eq(svp, sv.len, "false") or
                str_eq(svp, sv.len, "yes") or str_eq(svp, sv.len, "no") or
                str_eq(svp, sv.len, "on") or str_eq(svp, sv.len, "off") or
                str_eq(svp, sv.len, "True") or str_eq(svp, sv.len, "False") or
                str_eq(svp, sv.len, "TRUE") or str_eq(svp, sv.len, "FALSE")) return obj_new_int(1);
            return obj_new_int(0);
        }
        if (str_eq(clsp, cls.len, "ascii")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                if (svp[i] > 0x7F) return obj_new_int(0);
            }
            return obj_new_int(1);
        }
        if (str_eq(clsp, cls.len, "control")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                const b = svp[i];
                if (b >= 0x80) return obj_new_int(0);
                if (b >= 0x20 and b != 0x7F) return obj_new_int(0);
            }
            return obj_new_int(1);
        }
        if (str_eq(clsp, cls.len, "graph")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                const b = svp[i];
                if (b >= 0x80) { i += 1; continue; }
                if (b <= 0x20 or b == 0x7F) return obj_new_int(0);
            }
            return obj_new_int(1);
        }
        if (str_eq(clsp, cls.len, "lower")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                const b = svp[i];
                if (b >= 0x80) { i += 1; continue; }
                if (b < 'a' or b > 'z') return obj_new_int(0);
            }
            return obj_new_int(1);
        }
        if (str_eq(clsp, cls.len, "upper")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                const b = svp[i];
                if (b >= 0x80) { i += 1; continue; }
                if (b < 'A' or b > 'Z') return obj_new_int(0);
            }
            return obj_new_int(1);
        }
        if (str_eq(clsp, cls.len, "punct")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                const b = svp[i];
                if (b >= 0x80) { i += 1; continue; }
                const is_punct = (b >= '!' and b <= '/') or (b >= ':' and b <= '@') or
                    (b >= '[' and b <= '`') or (b >= '{' and b <= '~');
                if (!is_punct) return obj_new_int(0);
            }
            return obj_new_int(1);
        }
        if (str_eq(clsp, cls.len, "xdigit")) {
            var i: u32 = 0;
            while (i < sv.len) : (i += 1) {
                const b = svp[i];
                if (!((b >= '0' and b <= '9') or (b >= 'a' and b <= 'f') or (b >= 'A' and b <= 'F'))) return obj_new_int(0);
            }
            return obj_new_int(1);
        }
        if (str_eq(clsp, cls.len, "double") or str_eq(clsp, cls.len, "float")) {
            // Very basic: try to parse as number with optional decimal/exponent
            var i: u32 = 0;
            while (i < sv.len and (svp[i] == ' ' or svp[i] == '\t')) i += 1;
            if (i < sv.len and (svp[i] == '+' or svp[i] == '-')) i += 1;
            var has_digit = false;
            while (i < sv.len and svp[i] >= '0' and svp[i] <= '9') { i += 1; has_digit = true; }
            if (i < sv.len and svp[i] == '.') {
                i += 1;
                while (i < sv.len and svp[i] >= '0' and svp[i] <= '9') { i += 1; has_digit = true; }
            }
            if (!has_digit) return obj_new_int(0);
            if (i < sv.len and (svp[i] == 'e' or svp[i] == 'E')) {
                i += 1;
                if (i < sv.len and (svp[i] == '+' or svp[i] == '-')) i += 1;
                if (i >= sv.len or svp[i] < '0' or svp[i] > '9') return obj_new_int(0);
                while (i < sv.len and svp[i] >= '0' and svp[i] <= '9') i += 1;
            }
            if (i != sv.len) return obj_new_int(0);
            return obj_new_int(1);
        }
        // Unknown class — return 0
        return obj_new_int(0);
    }
    return 0;
}

pub fn eval_array_cmd(words: []const i32) i32 {
    if (words.len < 3) return 0;
    const sub = obj_ensure_string(words[1]);
    const sp: [*]const u8 = @ptrFromInt(sub.ptr);
    const array_mod = @import("tcl_array.zig");
    if (str_eq(sp, sub.len, "get")) {
        if (words.len >= 4) return array_mod.array_get(words[2], words[3]);
        return array_mod.array_get(words[2], obj_new_string(0, 0));
    }
    if (str_eq(sp, sub.len, "set") and words.len >= 4) {
        // ``array set arr pairlist`` — the payload is a Tcl list
        // of ``{k v k v …}`` pairs.  We always route through
        // ``array_set_list`` because even a single-pair invocation
        // (``array set a {key value}``) is just a 2-element list.
        // The previous shape ``array_set(words[2], words[3], 0)``
        // stored the whole payload under one key with a null
        // value — that silently broke tcltest's
        // ``ArrayDefault numTests [list Total 0 …]``
        // initialisation (``incr numTests(Total)`` then ran on
        // an uninitialised element).
        return array_mod.array_set_list(words[2], words[3]);
    }
    if (str_eq(sp, sub.len, "exists")) return array_mod.array_exists(words[2]);
    if (str_eq(sp, sub.len, "names")) {
        // ``array names arr ?pattern? ?mode?`` — we handle the
        // first two positions; ``mode`` (``-exact`` / ``-glob`` /
        // ``-regexp``) beyond glob isn't wired yet.
        const pat: i32 = if (words.len >= 4) words[3] else 0;
        return array_mod.array_names(words[2], pat);
    }
    if (str_eq(sp, sub.len, "size")) return array_mod.array_size(words[2]);
    if (str_eq(sp, sub.len, "unset")) {
        if (words.len >= 4) return array_mod.array_unset_element(words[2], words[3]);
        return array_mod.array_unset(words[2]);
    }
    // Other subcommands (statistics, startsearch, …) not yet wired —
    // fall through to the stub dispatch which raises the exception.
    const stubs_mod = @import("tcl_stubs.zig");
    const sub_slice: []const u8 = (@as([*]const u8, @ptrFromInt(sub.ptr)))[0..sub.len];
    stubs_mod.unsupported_sub("array", sub_slice);
    return 0;
}

pub fn eval_dict_cmd(words: []const i32) i32 {
    if (words.len < 3) return 0;
    const sub = obj_ensure_string(words[1]);
    const sp: [*]const u8 = @ptrFromInt(sub.ptr);
    if (str_eq(sp, sub.len, "get") and words.len >= 4) return rt.dict_get(words[2], words[3]);
    if (str_eq(sp, sub.len, "set") and words.len >= 5) {
        const cur = frames.var_resolve(words[2]);
        const result = rt.dict_set(cur, words[3], words[4]);
        _ = frames.var_set(words[2], result);
        return result;
    }
    if (str_eq(sp, sub.len, "exists") and words.len >= 4) return rt.dict_exists(words[2], words[3]);
    if (str_eq(sp, sub.len, "keys")) return rt.dict_keys(words[2]);
    if (str_eq(sp, sub.len, "values")) return rt.dict_values(words[2]);
    if (str_eq(sp, sub.len, "size")) return rt.dict_size(words[2]);
    if (str_eq(sp, sub.len, "create")) return rt.dict_create();
    return 0;
}
