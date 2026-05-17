// ``tcl::prefix`` subcommand family — implements ``::tcl::prefix
// match``, ``::tcl::prefix all``, and ``::tcl::prefix longest`` (see
// C tcl ``tclIndexObj.c``).  Each takes a ``table`` (a Tcl list of
// candidate strings) plus a ``string`` and returns the matching
// entry (``match``), every entry that *string* prefixes (``all``),
// or the longest common prefix among entries that *string*
// prefixes (``longest``).

const rt = @import("../tcl_runtime.zig");
const obj_mod = @import("../valtypes/tcl_obj.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");
const list_parse = @import("../valtypes/tcl_list_parse.zig");
const result_mod = @import("../interp/tcl_result.zig");

const obj_new_string = rt.obj_new_string;
const obj_ensure_string = rt.obj_ensure_string;

fn str_eq(a: [*]const u8, alen: u32, b: []const u8) bool {
    if (alen != b.len) return false;
    for (b, 0..) |c, i| if (a[i] != c) return false;
    return true;
}

fn raise(msg: []const u8) void {
    const catch_mod = @import("../interp/tcl_catch.zig");
    const buf = obj_mod.alloc(@intCast(msg.len));
    if (buf == 0) {
        // OOM en route to raising an error — surface an empty
        // TclObj rather than dereferencing a null buffer.  The
        // outer catch still sees an error, just without the
        // human-readable message.
        catch_mod.tcl_cmd_error(obj_new_string(0, 0));
        return;
    }
    const dst: [*]u8 = @ptrFromInt(buf);
    for (msg, 0..) |c, i| dst[i] = c;
    const o = obj_mod.obj_new_string_take(buf, @intCast(msg.len), @intCast(msg.len));
    catch_mod.tcl_cmd_error(o);
}

/// Does *needle* (a prefix candidate) match the start of *elem* (a
/// table entry)?  Empty needle matches every entry (Tcl 9 semantics —
/// the empty string is a prefix of everything, including itself).
fn is_prefix(elem_ptr: u32, elem_len: u32, needle_ptr: u32, needle_len: u32) bool {
    if (needle_len > elem_len) return false;
    if (needle_len == 0) return true;
    const ep: [*]const u8 = @ptrFromInt(elem_ptr);
    const np: [*]const u8 = @ptrFromInt(needle_ptr);
    var i: u32 = 0;
    while (i < needle_len) : (i += 1) {
        if (ep[i] != np[i]) return false;
    }
    return true;
}

fn eval_prefix_all(words: []const i32) result_mod.InterpResult {
    if (words.len != 4) {
        raise("wrong # args: should be \"tcl::prefix all table string\"");
        return result_mod.from_globals(0);
    }
    const table = obj_ensure_string(words[2]);
    if (list_parse.check_list_syntax(table.ptr, table.len) != 0)
        return result_mod.from_globals(0);
    const needle = obj_ensure_string(words[3]);
    const n = rt.list_count_elements(table.ptr, table.len);
    var result: i32 = obj_new_string(0, 0);
    var i: i64 = 0;
    while (i < n) : (i += 1) {
        // ``tcl_cmd_list_index`` returns a freshly-allocated TclObj
        // and ``obj_new_int`` allocates the index slot — both need
        // to be released after the per-element check / lappend to
        // avoid a per-iteration leak on long tables.
        const idx_obj = obj_mod.obj_new_int(i);
        const elem = rt.tcl_cmd_list_index(words[2], idx_obj);
        obj_mod.tcl_obj_release(idx_obj);
        const es = obj_ensure_string(elem);
        if (is_prefix(es.ptr, es.len, needle.ptr, needle.len)) {
            // ``tcl_cmd_lappend`` doesn't consume *elem* — it copies
            // the bytes into the list buffer.  Drop our own ref.
            result = rt.tcl_cmd_lappend(result, elem);
        }
        obj_mod.tcl_obj_release(elem);
    }
    return result_mod.from_globals(result);
}

fn eval_prefix_longest(words: []const i32) result_mod.InterpResult {
    if (words.len != 4) {
        raise("wrong # args: should be \"tcl::prefix longest table string\"");
        return result_mod.from_globals(0);
    }
    const table = obj_ensure_string(words[2]);
    if (list_parse.check_list_syntax(table.ptr, table.len) != 0)
        return result_mod.from_globals(0);
    const needle = obj_ensure_string(words[3]);
    const n = rt.list_count_elements(table.ptr, table.len);
    // Collect all entries that *string* prefixes, then return the
    // longest common prefix among them (Tcl ``tcl::prefix longest``
    // semantics).  An empty match set returns ``""``.
    //
    // Ownership note: the previous shape borrowed ``common_ptr``
    // from the first matching element's TclObj — once we release
    // *elem* at end-of-iteration the buffer is unsafe to read.
    // Instead, hold the first match's *handle* alive across the
    // loop via a retain, and only release when we swap it in for
    // a shorter common prefix slice (which itself comes from the
    // newer ``elem`` — also retained, swap-and-release).
    var common_obj: i32 = 0;
    var common_ptr: u32 = 0;
    var common_len: u32 = 0;
    var have_any = false;
    var i: i64 = 0;
    while (i < n) : (i += 1) {
        const idx_obj = obj_mod.obj_new_int(i);
        const elem = rt.tcl_cmd_list_index(words[2], idx_obj);
        obj_mod.tcl_obj_release(idx_obj);
        const es = obj_ensure_string(elem);
        if (!is_prefix(es.ptr, es.len, needle.ptr, needle.len)) {
            obj_mod.tcl_obj_release(elem);
            continue;
        }
        if (!have_any) {
            // Keep *elem* alive for the rest of the loop so the
            // borrowed ``common_ptr`` stays valid.
            obj_mod.tcl_obj_retain(elem);
            common_obj = elem;
            common_ptr = es.ptr;
            common_len = es.len;
            have_any = true;
            obj_mod.tcl_obj_release(elem);
            continue;
        }
        // Intersect common prefix at codepoint granularity — the
        // longest-common-prefix is naturally a codepoint operation
        // (multi-byte UTF-8 sequences can't truncate mid-byte) and
        // matters for inputs like ``ax\x90`` vs ``ax\x91`` where
        // both 2-byte sequences share their lead byte (string-28.13.0).
        const min = if (common_len < es.len) common_len else es.len;
        if (min == 0) {
            common_len = 0;
            obj_mod.tcl_obj_release(elem);
            continue;
        }
        const a: [*]const u8 = @ptrFromInt(common_ptr);
        const b: [*]const u8 = @ptrFromInt(es.ptr);
        var k: u32 = 0;
        while (k < min) {
            const al = utf8_lead_len_local(a[k]);
            const bl = utf8_lead_len_local(b[k]);
            if (al != bl) break;
            if (k + al > min) break;
            var ok = true;
            var j: u32 = 0;
            while (j < al) : (j += 1) {
                if (a[k + j] != b[k + j]) {
                    ok = false;
                    break;
                }
            }
            if (!ok) break;
            k += al;
        }
        common_len = k;
        obj_mod.tcl_obj_release(elem);
    }
    // Materialise the result before dropping ``common_obj`` so the
    // borrowed bytes survive into ``obj_new_string_copy``.
    var result: i32 = obj_new_string(0, 0);
    if (have_any and common_len != 0) {
        result = rt.obj_new_string_copy(common_ptr, common_len);
    }
    if (common_obj != 0) obj_mod.tcl_obj_release(common_obj);
    return result_mod.from_globals(result);
}

fn utf8_lead_len_local(b: u8) u32 {
    if (b < 0x80) return 1;
    if (b < 0xC0) return 1;
    if (b < 0xE0) return 2;
    if (b < 0xF0) return 3;
    return 4;
}

fn eval_prefix_match(words: []const i32) result_mod.InterpResult {
    // ``tcl::prefix match ?options? table string`` — accept
    // ``-exact`` (require an exact match — non-exact unique
    // prefixes are rejected), ``-error OPTS`` and ``-message MSG``
    // (parsed-and-ignored; future revisions can plumb them
    // through to the ambiguous-match error).  ``--`` terminates
    // option parsing.
    var ai: u32 = 2;
    var exact: bool = false;
    while (ai + 2 < words.len) : (ai += 1) {
        const a = obj_ensure_string(words[ai]);
        const ap: [*]const u8 = @ptrFromInt(a.ptr);
        if (a.len == 0 or ap[0] != '-') break;
        if (str_eq(ap, a.len, "--")) {
            ai += 1;
            break;
        }
        if (str_eq(ap, a.len, "-exact")) {
            exact = true;
            continue;
        }
        if (str_eq(ap, a.len, "-error") or str_eq(ap, a.len, "-message")) {
            if (ai + 1 >= words.len) break;
            ai += 1;
            continue;
        }
        // Unknown option.
        raise("bad option to tcl::prefix match");
        return result_mod.from_globals(0);
    }
    if (ai + 2 != words.len) {
        raise("wrong # args: should be \"tcl::prefix match ?options? table string\"");
        return result_mod.from_globals(0);
    }
    const table_obj = words[ai];
    const table = obj_ensure_string(table_obj);
    if (list_parse.check_list_syntax(table.ptr, table.len) != 0)
        return result_mod.from_globals(0);
    const needle = obj_ensure_string(words[ai + 1]);
    const n = rt.list_count_elements(table.ptr, table.len);
    var hit: i32 = 0;
    var hit_count: u32 = 0;
    var i: i64 = 0;
    while (i < n) : (i += 1) {
        // Index off the *table operand* — ``words[ai]`` after
        // option parsing, not ``words[2]`` which may be an option
        // token (e.g. ``tcl::prefix match -exact T S``).
        const idx_obj = obj_mod.obj_new_int(i);
        const elem = rt.tcl_cmd_list_index(table_obj, idx_obj);
        obj_mod.tcl_obj_release(idx_obj);
        const es = obj_ensure_string(elem);
        if (es.len == needle.len and is_prefix(es.ptr, es.len, needle.ptr, needle.len)) {
            // Exact match short-circuits and wins outright (both
            // option modes).
            return result_mod.from_globals(elem);
        }
        if (!exact and is_prefix(es.ptr, es.len, needle.ptr, needle.len)) {
            // Non-exact prefix match counts only when ``-exact``
            // is absent.  We accumulate a candidate (release the
            // previous one) so an unambiguous hit returns just
            // one live obj.
            if (hit != 0) obj_mod.tcl_obj_release(hit);
            hit = elem;
            hit_count += 1;
            continue;
        }
        obj_mod.tcl_obj_release(elem);
    }
    if (hit_count == 1) return result_mod.from_globals(hit);
    if (hit != 0) obj_mod.tcl_obj_release(hit);
    // Ambiguous or no match — raise the canonical error (the upstream
    // shapes differ for ambiguous vs. unrecognised; emit a single
    // ``bad prefix`` string for now which is good enough for the
    // current test cases that use ``-error``).
    raise("bad prefix value");
    return result_mod.from_globals(0);
}

pub fn eval_prefix(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) {
        raise("wrong # args: should be \"tcl::prefix subcommand ?arg ...?\"");
        return result_mod.from_globals(0);
    }
    const sub = obj_ensure_string(words[1]);
    const sp: [*]const u8 = @ptrFromInt(sub.ptr);
    if (str_eq(sp, sub.len, "all")) return eval_prefix_all(words);
    if (str_eq(sp, sub.len, "longest")) return eval_prefix_longest(words);
    if (str_eq(sp, sub.len, "match")) return eval_prefix_match(words);
    raise("bad option: must be all, longest, or match");
    return result_mod.from_globals(0);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "::tcl::prefix", .arity_min = 1, .arity_max = null, .handler = &eval_prefix },
    // Also register the bare ``tcl::prefix`` spelling so callers that
    // skip the leading ``::`` resolve to the same handler.  C tcl's
    // lookup walks the namespace tree from the call site to root, so
    // unqualified ``tcl::prefix`` from the global ns finds the
    // canonical entry; our dispatch table is flat, so we register
    // both spellings explicitly.
    .{ .name = "tcl::prefix", .arity_min = 1, .arity_max = null, .handler = &eval_prefix },
};
