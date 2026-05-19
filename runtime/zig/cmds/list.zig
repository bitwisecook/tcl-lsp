// List manipulation commands: list, lappend, llength, lindex, lset,
// linsert, lreplace, lsort, lsearch, lrange, concat, join, split,
// lreverse, lrepeat, lassign, lmap, lseq.

const rt = @import("../tcl_runtime.zig");
const frames = @import("../interp/tcl_frames.zig");
const obj_mod = @import("../valtypes/tcl_obj.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");
const list_mod = @import("../valtypes/tcl_list.zig");
const interp = @import("../interp/tcl_interp.zig");
const result_mod = @import("../interp/tcl_result.zig");
const tcl_str = @import("../valtypes/tcl_string.zig");
const tcl_chars = @import("../valtypes/tcl_chars.zig");
const list_parse = @import("../valtypes/tcl_list_parse.zig");
const parse = @import("../parse/tcl_parse.zig");
const std = @import("std");

const alloc = rt.alloc;
const obj_new_string = rt.obj_new_string;
const obj_new_int = rt.obj_new_int;
const obj_ensure_string = rt.obj_ensure_string;

fn eval_list(words: []const i32) result_mod.InterpResult {
    if (words.len <= 1) return result_mod.from_globals(obj_new_string(0, 0));
    var max_total: u32 = 0;
    var ei: u32 = 1;
    while (ei < words.len) : (ei += 1) {
        const s = obj_ensure_string(words[ei]);
        max_total += s.len * 2 + 2;
        if (ei > 1) max_total += 1;
    }
    const alloc_size: u32 = max_total + 4;
    const buf = obj_mod.alloc(alloc_size);
    var off: u32 = 0;
    ei = 1;
    while (ei < words.len) : (ei += 1) {
        if (ei > 1) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        const s = obj_ensure_string(words[ei]);
        if (ei == 1) {
            off = obj_mod.list_elem_quote(buf, off, s.ptr, s.len);
        } else {
            off = obj_mod.list_elem_quote_nth(buf, off, s.ptr, s.len);
        }
    }
    // Issue #317: own the result buffer so the caller's release
    // reclaims it via ``free_sized``; without this every
    // ``[list ...]`` invocation leaked one buf, and the
    // resulting cap=0 also forced ``lappend``'s slow rebuild
    // path on every subsequent append.
    return result_mod.from_globals(obj_mod.obj_new_string_take(buf, off, alloc_size));
}

fn eval_lappend(words: []const i32) result_mod.InterpResult {
    if (words.len >= 2) {
        // Inside a proc frame an unqualified name must resolve LOCALLY
        // (Tcl semantics: unqualified vars in proc bodies are locals
        // unless explicitly aliased via ``global`` / ``upvar`` /
        // ``variable``).  ``frames.var_resolve`` falls back to the
        // global table when a local miss occurs — that gives
        // ``proc foo {} { lappend x }`` access to ``::x`` when no
        // ``global x`` was declared, contradicting reference Tcl and
        // causing appendComp-4.17/4.18 to inherit ``::x`` from a
        // prior test in the bundle.  Use ``local_get_silent`` (frame-
        // only, follows ``global`` / ``upvar`` aliases) in proc
        // context and only fall back to ``var_resolve`` for
        // script-level / namespace-context calls.
        const in_proc = frames.frame_depth > 0;
        const initial: i32 = if (in_proc)
            frames.local_get_silent(words[1])
        else
            frames.var_resolve(words[1]);
        var result = initial;
        var wi: u32 = 2;
        while (wi < words.len) : (wi += 1) {
            result = rt.tcl_cmd_lappend(result, words[wi]);
        }
        _ = frames.var_set(words[1], result);
        return result_mod.from_globals(result);
    }
    return result_mod.from_globals(0);
}

fn eval_llength(words: []const i32) result_mod.InterpResult {
    if (words.len != 2) {
        const stubs = @import("../stubs/tcl_stubs.zig");
        stubs.raise("wrong # args: should be \"llength list\"");
        return result_mod.from_globals(0);
    }
    return result_mod.from_globals(rt.tcl_cmd_list_length(words[1]));
}

fn eval_lindex(words: []const i32) result_mod.InterpResult {
    // ``lindex`` with no list argument is an arity error.
    if (words.len < 2) {
        const stubs = @import("../stubs/tcl_stubs.zig");
        stubs.raise("wrong # args: should be \"lindex list ?index ...?\"");
        return result_mod.from_globals(0);
    }
    // ``lindex LIST`` (1 arg form) returns the list verbatim.
    if (words.len == 2) return result_mod.from_globals(words[1]);
    const ls = obj_ensure_string(words[1]);
    if (list_parse.check_list_syntax(ls.ptr, ls.len) != 0)
        return result_mod.from_globals(0);
    // ``lindex LIST i1 i2 ...`` — Tcl 9 semantics: apply each
    // remaining index in turn, descending into the result of the
    // previous indexing.  ``lindex {a b {c d}} 2 1`` → ``d``.
    // A single index argument that itself is a list ``{i1 i2}`` is
    // also valid (e.g. ``lindex L {2 1}``); flatten such a wrapper
    // by routing through the same loop after splitting the index
    // list.  ``tcl_cmd_list_index`` handles bare numeric / ``end`` /
    // ``end-N`` indices; the multi-index walk is a simple fold.
    var current: i32 = words[1];
    if (words.len == 3) {
        // Single index argument — may be a flat ``5`` or a list
        // ``{2 1}`` per Tcl 9 semantics.  Walk the elements so the
        // list-form behaves like the explicit multi-arg form.
        const idx_arg = words[2];
        const is_s = obj_ensure_string(idx_arg);
        // ``lindex L {}`` — empty index list returns the list
        // verbatim (lindex-10.1: ``$x = ""`` should not error).
        if (is_s.len == 0) return result_mod.from_globals(current);
        const idx_count = rt.list_count_elements(is_s.ptr, is_s.len);
        if (idx_count == 0) return result_mod.from_globals(current);
        if (idx_count == 1) {
            // Single-element index list: if the arg was already a
            // bare scalar (no whitespace), validate its shape and
            // raise ``bad index`` on garbage like ``b`` or ``0a2``.
            // A scalar that happens to contain whitespace (e.g. the
            // wrapping-list form ``{2 1}``) routes through the
            // walk below and individual elements get validated
            // there.
            if (!has_list_whitespace(is_s.ptr, is_s.len) and !list_mod.is_valid_list_index(idx_arg)) {
                list_mod.raise_bad_list_index(idx_arg);
                return result_mod.from_globals(0);
            }
            return result_mod.from_globals(rt.tcl_cmd_list_index(current, idx_arg));
        }
        var k: i64 = 0;
        while (k < idx_count) : (k += 1) {
            const ei = rt.tcl_cmd_list_index(idx_arg, obj_new_int(k));
            if (!list_mod.is_valid_list_index(ei)) {
                list_mod.raise_bad_list_index(ei);
                return result_mod.from_globals(0);
            }
            current = rt.tcl_cmd_list_index(current, ei);
        }
        return result_mod.from_globals(current);
    }
    var ai: u32 = 2;
    while (ai < words.len) : (ai += 1) {
        if (!list_mod.is_valid_list_index(words[ai])) {
            list_mod.raise_bad_list_index(words[ai]);
            return result_mod.from_globals(0);
        }
        current = rt.tcl_cmd_list_index(current, words[ai]);
    }
    return result_mod.from_globals(current);
}

/// Cheap check for whitespace inside *idx*; used to disambiguate a
/// scalar index from a single-element list-form index.
fn has_list_whitespace(ptr: u32, len: u32) bool {
    if (ptr == 0 or len == 0) return false;
    const sp: [*]const u8 = @ptrFromInt(ptr);
    var i: u32 = 0;
    while (i < len) : (i += 1) {
        const c = sp[i];
        if (c == ' ' or c == '\t' or c == '\n' or c == '\r' or c == 0x0B or c == 0x0C) return true;
    }
    return false;
}

fn eval_lset(words: []const i32) result_mod.InterpResult {
    if (words.len < 3) {
        const stubs = @import("../stubs/tcl_stubs.zig");
        stubs.raise("wrong # args: should be \"lset listVar ?index? ?index ...? value\"");
        return result_mod.from_globals(0);
    }
    const current = frames.var_resolve(words[1]);
    const newval = words[words.len - 1];
    const indices: i32 = if (words.len == 3)
        obj_new_string(0, 0)
    else if (words.len == 4)
        words[2]
    else blk: {
        var acc: i32 = rt.tcl_list(words[2], words[3]);
        var wi: u32 = 4;
        while (wi + 1 < words.len) : (wi += 1) {
            acc = rt.tcl_list(acc, words[wi]);
        }
        break :blk acc;
    };
    const result = rt.tcl_cmd_list_set(current, indices, newval);
    _ = frames.var_set(words[1], result);
    return result_mod.from_globals(result);
}

fn eval_linsert(words: []const i32) result_mod.InterpResult {
    if (words.len < 3) {
        const stubs = @import("../stubs/tcl_stubs.zig");
        stubs.raise("wrong # args: should be \"linsert list index ?element ...?\"");
        return result_mod.from_globals(0);
    }
    const list_arg = words[1];
    const idx_arg = words[2];
    // Validate list syntax — ``linsert \{ 12 2`` must raise
    // ``unmatched open brace in list`` rather than silently producing
    // garbage (linsert-2.4).
    const ls = obj_ensure_string(list_arg);
    if (list_parse.check_list_syntax(ls.ptr, ls.len) != 0)
        return result_mod.from_globals(0);
    // Validate index — ``linsert a b`` must raise ``bad index "b"``
    // rather than treating ``b`` as 0 (linsert-2.2/2.3).
    if (!list_mod.is_valid_list_index(idx_arg)) {
        list_mod.raise_bad_list_index(idx_arg);
        return result_mod.from_globals(0);
    }
    // ``linsert LIST INDEX`` with no values still canonicalises the
    // input list — ``linsert "a\nb\nc" 0`` → ``a b c`` (linsert-2.5/
    // 2.6, TIP 323).  Round-trip through ``list_count_elements`` +
    // ``list_element_at`` so each element comes back canonically
    // quoted regardless of the input's whitespace flavour.
    if (words.len < 4) {
        return result_mod.from_globals(canonicalise_list(list_arg));
    }
    // Resolve the insertion position ONCE from the original list
    // length.  ``end`` semantically denotes the slot AFTER the last
    // element, so resolve as if the list had n+1 positions (matching
    // ``tcl_cmd_list_insert`` / reference ``Tcl_LinsertObjCmd``).
    const n_i64 = rt.list_count_elements(ls.ptr, ls.len);
    var resolved_idx: i64 = list_mod.resolve_list_index(idx_arg, n_i64 + 1);
    if (resolved_idx < 0) resolved_idx = 0;
    if (resolved_idx > n_i64) resolved_idx = n_i64;
    // Insert in FORWARD order with the position incrementing — each
    // value lands AFTER the previous one.  This unifies the two
    // earlier reverse-iteration branches whose clamping interacted
    // badly when the index exceeded the original length (linsert-1.10:
    // ``linsert {} 2 a b c`` was returning ``c b a``).
    var result: i32 = list_arg;
    var pos: i64 = resolved_idx;
    var wi: u32 = 3;
    while (wi < words.len) : (wi += 1) {
        result = rt.tcl_cmd_list_insert(result, obj_new_int(pos), words[wi]);
        pos += 1;
    }
    return result_mod.from_globals(result);
}

/// Build the canonical Tcl list representation of *list_arg*.  Used by
/// ``linsert LIST INDEX`` with no values (TIP 323) — the resulting
/// string round-trips every element through ``list_elem_quote`` so
/// whitespace-only sources (``"a\nb\nc"``) come back as ``a b c``.
fn canonicalise_list(list_arg: i32) i32 {
    const ls = obj_ensure_string(list_arg);
    const n_i64 = rt.list_count_elements(ls.ptr, ls.len);
    if (n_i64 == 0) return obj_new_string(0, 0);
    const n: u32 = @intCast(n_i64);
    // Worst case: every element doubles in length plus brace pair,
    // plus n-1 separators.
    const buf = alloc(ls.len * 2 + n * 3 + 4);
    var off: u32 = 0;
    var i: u32 = 0;
    while (i < n) : (i += 1) {
        const elem = rt.list_element_at(ls.ptr, ls.len, @intCast(i));
        if (off > 0) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        if (elem.braced) {
            // Braced elements are already literal — re-quote with
            // ``list_elem_quote`` so the result follows canonical
            // form (a brace-pair becomes ``{...}`` again if any
            // special chars are present).
            off = list_mod.append_list_element(buf, off, ls.ptr, elem, off == 0);
        } else {
            // Unbraced: decode any backslash escapes, then re-quote.
            const scratch = alloc(elem.len);
            if (scratch == 0) {
                const quoter: *const fn (u32, u32, u32, u32) u32 = if (off == 0) &obj_mod.list_elem_quote else &obj_mod.list_elem_quote_nth;
                off = quoter(buf, off, ls.ptr + elem.start, elem.len);
            } else {
                const dec_len = rt.copy_unbraced_elem(scratch, ls.ptr + elem.start, elem.len);
                const quoter: *const fn (u32, u32, u32, u32) u32 = if (off == 0) &obj_mod.list_elem_quote else &obj_mod.list_elem_quote_nth;
                off = quoter(buf, off, scratch, dec_len);
                obj_mod.free_sized(scratch, elem.len);
            }
        }
    }
    return obj_mod.obj_new_string_take(buf, off, ls.len * 2 + n * 3 + 4);
}

fn eval_lreplace(words: []const i32) result_mod.InterpResult {
    if (words.len < 4) {
        const stubs = @import("../stubs/tcl_stubs.zig");
        stubs.raise("wrong # args: should be \"lreplace list first last ?element ...?\"");
        return result_mod.from_globals(0);
    }
    // Validate list syntax — ``lreplace`` raises ``unmatched open
    // brace in list`` for malformed input before parsing indices,
    // matching reference Tcl (listobj-7.2/7.3).
    const ls = obj_ensure_string(words[1]);
    if (list_parse.check_list_syntax(ls.ptr, ls.len) != 0)
        return result_mod.from_globals(0);
    if (!list_mod.is_valid_list_index(words[2])) {
        list_mod.raise_bad_list_index(words[2]);
        return result_mod.from_globals(0);
    }
    if (!list_mod.is_valid_list_index(words[3])) {
        list_mod.raise_bad_list_index(words[3]);
        return result_mod.from_globals(0);
    }
    return result_mod.from_globals(do_lreplace(words[1], words[2], words[3], words, 4));
}

/// Shared lreplace implementation used by ``lreplace`` and ``ledit``.
/// ``insert_from`` is the index of the first element to insert in
/// ``words``; the trailing words from there to ``words.len`` are
/// inserted at position ``first`` after the [first..last] slice is
/// removed.  Inverted ranges (first > last) skip the removal and
/// behave as a pure ``linsert`` (reference Tcl semantics; covers
/// lreplace-4.11 / ledit-4.11).
fn do_lreplace(list_arg: i32, first_arg: i32, last_arg: i32, words: []const i32, insert_from: u32) i32 {
    // Resolve indices against the ORIGINAL list once, up front.
    // After the first ``tcl_cmd_list_replace`` shrinks the list,
    // any ``end-N`` index would re-resolve against the smaller
    // list and land at the wrong slot (bug: lreplace-4.7.1,
    // lreplace-4.11).  Operate on absolute integer positions from
    // here on.
    const ls = obj_ensure_string(list_arg);
    const n_i64 = rt.list_count_elements(ls.ptr, ls.len);
    var f = resolve_list_index_local(first_arg, n_i64);
    var l = resolve_list_index_local(last_arg, n_i64);
    if (f < 0) f = 0;
    if (l >= n_i64) l = n_i64 - 1;
    if (f > n_i64) f = n_i64;
    const has_inserts = insert_from < words.len;

    // First step: shrink the list to remove [f..l].  If the range
    // is inverted (l < f) the existing runtime helper already
    // treats it as a pure insert at f and emits no replacement —
    // we go through the same call to canonicalise the result.
    var result: i32 = undefined;
    if (has_inserts) {
        // Splice the FIRST insert at position f as part of the
        // removal step so we don't need to track ownership across
        // a separate insert call.
        result = rt.tcl_cmd_list_replace(list_arg, first_arg, last_arg, words[insert_from]);
        var wi: u32 = insert_from + 1;
        var pos: i64 = f + 1; // absolute index for the next insert
        while (wi < words.len) : (wi += 1) {
            const idx_obj = make_index_obj(pos);
            // ``tcl_cmd_list_insert`` returns a fresh TclObj — release
            // the previous intermediate ``result`` before overwriting
            // (PR #413 review: chained inserts otherwise leak one
            // header per extra value).
            const new_result = rt.tcl_cmd_list_insert(result, idx_obj, words[wi]);
            obj_mod.tcl_obj_release(result);
            result = new_result;
            obj_mod.tcl_obj_release(idx_obj);
            pos += 1;
        }
    } else {
        // Pure removal — canonicalise even when nothing changes
        // (e.g. ``lreplace { } 1 1`` collapses the single-space
        // input to the empty list, lreplace-4.2).
        result = rt.tcl_cmd_list_replace(list_arg, first_arg, last_arg, 0);
    }
    return result;
}

fn make_index_obj(idx: i64) i32 {
    var idx_buf: [24]u8 = undefined;
    var off: u32 = 0;
    var neg: bool = false;
    var v: i64 = idx;
    if (v < 0) {
        neg = true;
        v = -v;
    }
    var tmp: [24]u8 = undefined;
    var tlen: u32 = 0;
    if (v == 0) {
        tmp[0] = '0';
        tlen = 1;
    } else {
        while (v > 0) {
            tmp[tlen] = @intCast(@as(u32, '0') + @as(u32, @intCast(@mod(v, 10))));
            v = @divTrunc(v, 10);
            tlen += 1;
        }
    }
    if (neg) {
        idx_buf[0] = '-';
        off = 1;
    }
    var k: u32 = 0;
    while (k < tlen) : (k += 1) {
        idx_buf[off + k] = tmp[tlen - 1 - k];
    }
    off += tlen;
    return rt.obj_new_string_copy(@intFromPtr(&idx_buf), off);
}

fn resolve_list_index_local(o: i32, n: i64) i64 {
    if (o == 0) return -1;
    const s = obj_ensure_string(o);
    if (s.ptr == 0 or s.len == 0) return -1;
    const p: [*]const u8 = @ptrFromInt(s.ptr);
    // ``end`` / ``end-N`` / ``end+N``.
    if (s.len >= 3 and p[0] == 'e' and p[1] == 'n' and p[2] == 'd') {
        if (s.len == 3) return n - 1;
        if (s.len > 4 and (p[3] == '-' or p[3] == '+')) {
            var v: i64 = 0;
            const neg = p[3] == '-';
            var k: u32 = 4;
            while (k < s.len) : (k += 1) {
                if (p[k] < '0' or p[k] > '9') return n - 1;
                v = v * 10 + (p[k] - '0');
            }
            return if (neg) n - 1 - v else n - 1 + v;
        }
        return n - 1;
    }
    // Pure integer.
    return rt.obj_get_int(o);
}

/// ``ledit varName first last ?element ...?`` — same shape as
/// ``lreplace`` but mutates the variable in place AND returns the
/// new value.  Semantically ``set varName [lreplace [set varName]
/// first last ?element ...?]``.  Introduced in Tcl 9.0.
///
/// Routes through ``do_lreplace`` so the same end-relative /
/// inverted-range handling that fixes lreplace-1.15 / 4.7.1 / 4.11
/// applies (ledit-1.25, ledit-4.7.1, ledit-4.9.1, ledit-4.11).
fn eval_ledit(words: []const i32) result_mod.InterpResult {
    if (words.len < 4) {
        const stubs = @import("../stubs/tcl_stubs.zig");
        stubs.raise("wrong # args: should be \"ledit listVar first last ?element ...?\"");
        return result_mod.from_globals(0);
    }
    const var_arg = words[1];
    const first_arg = words[2];
    const last_arg = words[3];
    const list_arg = frames.var_resolve(var_arg);
    const result = do_lreplace(list_arg, first_arg, last_arg, words, 4);
    _ = frames.var_set(var_arg, result);
    return result_mod.from_globals(result);
}

// Full lsort implementation supporting -ascii / -dictionary / -integer
// / -real / -nocase / -increasing / -decreasing / -unique / -indices /
// -index INDEX / -stride N.  ``-command`` and ``-bisect`` fall back to
// the legacy single-key bytewise sort.  See cmdIL.test 1.x / 5.x for
// the option matrix.
const LsortMode = enum { ascii, integer, real, dictionary, command };
const LsortOpts = struct {
    mode: LsortMode = .ascii,
    nocase: bool = false,
    decreasing: bool = false,
    unique: bool = false,
    indices: bool = false,
    stride: u32 = 1,
    index_arg: i32 = 0, // 0 = no -index
    cmd_obj: i32 = 0, // 0 = no -command
};

// Tcl dictionary compare: walk both strings simultaneously, comparing
// digit runs as integers and non-digit runs lexicographically.  When
// nocase is set, non-digit chars compare case-insensitively.  Leading-
// zero runs are ordered by digit-run length (shorter = smaller) so
// ``1k 0k 10k`` sorts to ``0k 1k 10k`` not ``0k 10k 1k``.
fn lsort_dict_cmp(a_ptr: u32, a_len: u32, b_ptr: u32, b_len: u32, nocase: bool) i32 {
    const ap: [*]const u8 = @ptrFromInt(a_ptr);
    const bp: [*]const u8 = @ptrFromInt(b_ptr);
    var i: u32 = 0;
    var j: u32 = 0;
    while (i < a_len and j < b_len) {
        if (ap[i] >= '0' and ap[i] <= '9' and bp[j] >= '0' and bp[j] <= '9') {
            // Skip leading zeros to find significant digit start.
            const a_zero_start = i;
            const b_zero_start = j;
            while (i < a_len and ap[i] == '0') i += 1;
            while (j < b_len and bp[j] == '0') j += 1;
            const a_sig_start = i;
            const b_sig_start = j;
            while (i < a_len and ap[i] >= '0' and ap[i] <= '9') i += 1;
            while (j < b_len and bp[j] >= '0' and bp[j] <= '9') j += 1;
            const a_sig_len = i - a_sig_start;
            const b_sig_len = j - b_sig_start;
            // Longer significant run = bigger number.
            if (a_sig_len != b_sig_len) {
                return if (a_sig_len < b_sig_len) -1 else 1;
            }
            // Same length — compare digit-by-digit.
            var k: u32 = 0;
            while (k < a_sig_len) : (k += 1) {
                if (ap[a_sig_start + k] != bp[b_sig_start + k]) {
                    return if (ap[a_sig_start + k] < bp[b_sig_start + k]) -1 else 1;
                }
            }
            // Equal numerically — tie-break by zero-count (fewer
            // leading zeros sorts first).
            const a_zeros = a_sig_start - a_zero_start;
            const b_zeros = b_sig_start - b_zero_start;
            if (a_zeros != b_zeros) {
                return if (a_zeros < b_zeros) -1 else 1;
            }
        } else {
            var ac: u8 = ap[i];
            var bc: u8 = bp[j];
            if (nocase) {
                if (ac >= 'A' and ac <= 'Z') ac += 32;
                if (bc >= 'A' and bc <= 'Z') bc += 32;
            }
            if (ac != bc) return if (ac < bc) -1 else 1;
            i += 1;
            j += 1;
        }
    }
    if (i < a_len) return 1;
    if (j < b_len) return -1;
    return 0;
}

// Bytewise compare with optional case-folding.
fn lsort_ascii_cmp(a_ptr: u32, a_len: u32, b_ptr: u32, b_len: u32, nocase: bool) i32 {
    // Empty / null operands short-circuit.  Without this, casting a
    // null ``a_ptr`` to a non-allowzero ``[*]const u8`` traps in
    // ReleaseSafe; set-old.test reaches here via ``array names``
    // sorted output where some keys can resolve to null pointers.
    if (a_len == 0 and b_len == 0) return 0;
    if (a_len == 0) return -1;
    if (b_len == 0) return 1;
    const ap: [*]const u8 = @ptrFromInt(a_ptr);
    const bp: [*]const u8 = @ptrFromInt(b_ptr);
    const min = if (a_len < b_len) a_len else b_len;
    var i: u32 = 0;
    while (i < min) : (i += 1) {
        var ac = ap[i];
        var bc = bp[i];
        if (nocase) {
            if (ac >= 'A' and ac <= 'Z') ac += 32;
            if (bc >= 'A' and bc <= 'Z') bc += 32;
        }
        if (ac != bc) return if (ac < bc) -1 else 1;
    }
    if (a_len < b_len) return -1;
    if (a_len > b_len) return 1;
    return 0;
}

// Apply an -index PATH (list of indices) to *elem*, returning the
// extracted sub-element's bytes.  ``end`` / ``end-N`` indices are
// supported via ``resolve_list_index``.  Empty path returns *elem*.
fn lsort_apply_index(elem_obj: i32, path_obj: i32) i32 {
    if (path_obj == 0) return elem_obj;
    const ps = obj_ensure_string(path_obj);
    if (ps.len == 0) return elem_obj;
    const path_count = rt.list_count_elements(ps.ptr, ps.len);
    if (path_count == 0) return elem_obj;
    var current: i32 = elem_obj;
    var i: i64 = 0;
    while (i < path_count) : (i += 1) {
        const idx_obj = rt.tcl_cmd_list_index(path_obj, obj_new_int(i));
        current = rt.tcl_cmd_list_index(current, idx_obj);
    }
    return current;
}

// Sort key cached per element so the comparator doesn't re-parse the
// numeric value on every comparison.
const LsortKey = struct {
    obj: i32, // string-key (post -index extraction)
    str_ptr: u32, // cached string bytes
    str_len: u32,
    int_val: i64 = 0, // populated when mode == .integer
    float_val: f64 = 0.0, // populated when mode == .real
};

fn lsort_make_key(elem_obj: i32, opts: *const LsortOpts) LsortKey {
    const key_obj = lsort_apply_index(elem_obj, opts.index_arg);
    const s = obj_ensure_string(key_obj);
    var key: LsortKey = .{ .obj = key_obj, .str_ptr = s.ptr, .str_len = s.len };
    switch (opts.mode) {
        .integer => {
            if (obj_mod.try_parse_int(s.ptr, s.len)) |v| {
                key.int_val = v;
            }
        },
        .real => {
            if (obj_mod.try_parse_float(s.ptr, s.len)) |v| {
                key.float_val = v;
            } else if (obj_mod.try_parse_int(s.ptr, s.len)) |v| {
                key.float_val = @floatFromInt(v);
            }
        },
        else => {},
    }
    return key;
}

fn lsort_compare(a: *const LsortKey, b: *const LsortKey, opts: *const LsortOpts) i32 {
    const c: i32 = switch (opts.mode) {
        .integer => if (a.int_val < b.int_val) @as(i32, -1) else if (a.int_val > b.int_val) @as(i32, 1) else @as(i32, 0),
        .real => if (a.float_val < b.float_val) @as(i32, -1) else if (a.float_val > b.float_val) @as(i32, 1) else @as(i32, 0),
        .dictionary => lsort_dict_cmp(a.str_ptr, a.str_len, b.str_ptr, b.str_len, opts.nocase),
        // .command falls through to ascii (the script-call path isn't
        // wired through this helper — see eval_lsort fallback).
        else => lsort_ascii_cmp(a.str_ptr, a.str_len, b.str_ptr, b.str_len, opts.nocase),
    };
    return if (opts.decreasing) -c else c;
}

fn lsort_parse_opts(words: []const i32, opts: *LsortOpts) struct { ok: bool, list_idx: u32 } {
    const stubs = @import("../stubs/tcl_stubs.zig");
    var wi: u32 = 1;
    while (wi < words.len) : (wi += 1) {
        const sv = obj_ensure_string(words[wi]);
        if (sv.len == 0 or sv.ptr == 0) break;
        const sp: [*]const u8 = @ptrFromInt(sv.ptr);
        if (sp[0] != '-') break;
        if (ls_opt_eq(sv.ptr, sv.len, "--")) {
            wi += 1;
            break;
        } else if (ls_opt_eq(sv.ptr, sv.len, "-ascii")) {
            opts.mode = .ascii;
        } else if (ls_opt_eq(sv.ptr, sv.len, "-dictionary")) {
            opts.mode = .dictionary;
        } else if (ls_opt_eq(sv.ptr, sv.len, "-integer")) {
            opts.mode = .integer;
        } else if (ls_opt_eq(sv.ptr, sv.len, "-real")) {
            opts.mode = .real;
        } else if (ls_opt_eq(sv.ptr, sv.len, "-increasing")) {
            opts.decreasing = false;
        } else if (ls_opt_eq(sv.ptr, sv.len, "-decreasing")) {
            opts.decreasing = true;
        } else if (ls_opt_eq(sv.ptr, sv.len, "-nocase")) {
            opts.nocase = true;
        } else if (ls_opt_eq(sv.ptr, sv.len, "-unique")) {
            opts.unique = true;
        } else if (ls_opt_eq(sv.ptr, sv.len, "-indices")) {
            opts.indices = true;
        } else if (ls_opt_eq(sv.ptr, sv.len, "-stride")) {
            if (wi + 1 >= words.len) {
                stubs.raise("lsort: -stride requires an argument");
                return .{ .ok = false, .list_idx = 0 };
            }
            wi += 1;
            const v = obj_mod.obj_get_int(words[wi]);
            if (v < 1) {
                stubs.raise("stride length must be at least 1");
                return .{ .ok = false, .list_idx = 0 };
            }
            opts.stride = @intCast(v);
        } else if (ls_opt_eq(sv.ptr, sv.len, "-index")) {
            if (wi + 1 >= words.len) {
                stubs.raise("lsort: -index requires an argument");
                return .{ .ok = false, .list_idx = 0 };
            }
            wi += 1;
            opts.index_arg = words[wi];
        } else if (ls_opt_eq(sv.ptr, sv.len, "-command")) {
            if (wi + 1 >= words.len) {
                stubs.raise("lsort: -command requires an argument");
                return .{ .ok = false, .list_idx = 0 };
            }
            wi += 1;
            opts.mode = .command;
            opts.cmd_obj = words[wi];
        } else if (ls_opt_eq(sv.ptr, sv.len, "-bisect")) {
            // ``-bisect`` is an lsearch option (binary-search return
            // for an insertion point) that doesn't apply to lsort.
            // Reject it the same way C tcl does.
            stubs.raise("bad option \"-bisect\": must be -ascii, -command, -decreasing, -dictionary, -increasing, -index, -indices, -integer, -nocase, -real, -stride, or -unique");
            return .{ .ok = false, .list_idx = 0 };
        } else {
            const prefix = "bad option \"";
            const suffix = "\": must be -ascii, -command, -decreasing, -dictionary, -increasing, -index, -indices, -integer, -nocase, -real, -stride, or -unique";
            const total: u32 = @intCast(prefix.len + sv.len + suffix.len);
            const buf = obj_mod.alloc(total);
            if (buf != 0) {
                const bp: [*]u8 = @ptrFromInt(buf);
                var off: u32 = 0;
                for (prefix) |c| {
                    bp[off] = c;
                    off += 1;
                }
                for (0..sv.len) |k| {
                    bp[off] = sp[k];
                    off += 1;
                }
                for (suffix) |c| {
                    bp[off] = c;
                    off += 1;
                }
                const msg = obj_mod.obj_new_string_take(buf, total, total);
                const catch_mod = @import("../interp/tcl_catch.zig");
                catch_mod.tcl_cmd_error(msg);
            } else {
                stubs.raise("bad option to lsort");
            }
            return .{ .ok = false, .list_idx = 0 };
        }
    }
    return .{ .ok = true, .list_idx = wi };
}

fn eval_lsort(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) {
        const stubs = @import("../stubs/tcl_stubs.zig");
        stubs.raise("wrong # args: should be \"lsort ?-option value ...? list\"");
        return result_mod.from_globals(0);
    }
    var opts: LsortOpts = .{};
    const parsed = lsort_parse_opts(words, &opts);
    if (!parsed.ok) return result_mod.from_globals(0);
    if (parsed.list_idx >= words.len) {
        const stubs = @import("../stubs/tcl_stubs.zig");
        stubs.raise("wrong # args: should be \"lsort ?-option value ...? list\"");
        return result_mod.from_globals(0);
    }
    const list_obj = words[parsed.list_idx];

    // -command falls back to the legacy bytewise sort — proper
    // script-callback sort needs an interp re-entry per compare
    // and isn't wired through this helper.
    if (opts.mode == .command and opts.stride == 1 and opts.index_arg == 0 and !opts.unique and !opts.indices) {
        return result_mod.from_globals(rt.tcl_cmd_list_sort(list_obj));
    }

    const ls = obj_ensure_string(list_obj);
    if (list_parse.check_list_syntax(ls.ptr, ls.len) != 0)
        return result_mod.from_globals(0);
    const n_i64 = rt.list_count_elements(ls.ptr, ls.len);
    if (n_i64 <= 0) {
        if (opts.indices) return result_mod.from_globals(obj_new_string(0, 0));
        return result_mod.from_globals(obj_new_string(0, 0));
    }
    if (n_i64 > std.math.maxInt(u32)) {
        const stubs = @import("../stubs/tcl_stubs.zig");
        stubs.raise("lsort: list element count exceeds u32 range");
        return result_mod.from_globals(0);
    }
    const n: u32 = @intCast(n_i64);

    if (opts.stride > 1) {
        if (n % opts.stride != 0) {
            const stubs = @import("../stubs/tcl_stubs.zig");
            stubs.raise("list size must be a multiple of the stride length");
            return result_mod.from_globals(0);
        }
    }

    // Materialise the elements as TclObjs.  ``tcl_cmd_list_index``
    // already decodes braces / backslash escapes and returns a fresh
    // TclObj per call, which is what we want for the sort key cache.
    const groups: u32 = n / opts.stride;
    const elem_arr_size: u32 = n * 4;
    const elem_buf = obj_mod.alloc(elem_arr_size);
    if (elem_buf == 0) return result_mod.from_globals(0);
    defer obj_mod.free_sized(elem_buf, elem_arr_size);

    var i: u32 = 0;
    while (i < n) : (i += 1) {
        const e = rt.tcl_cmd_list_index(list_obj, obj_new_int(@intCast(i)));
        obj_mod.write_i32(elem_buf + i * 4, e);
    }

    // Build per-group sort keys (group's first element after -index).
    const key_arr_size: u32 = groups * @sizeOf(LsortKey);
    const key_buf = obj_mod.alloc(key_arr_size);
    if (key_buf == 0) return result_mod.from_globals(0);
    defer obj_mod.free_sized(key_buf, key_arr_size);
    const keys: [*]LsortKey = @ptrFromInt(key_buf);

    // Index table — parallel to ``keys``, used to recover the group's
    // original position after sorting.
    const idx_arr_size: u32 = groups * 4;
    const idx_buf = obj_mod.alloc(idx_arr_size);
    if (idx_buf == 0) return result_mod.from_globals(0);
    defer obj_mod.free_sized(idx_buf, idx_arr_size);

    var g: u32 = 0;
    while (g < groups) : (g += 1) {
        const first_obj = obj_mod.read_i32(elem_buf + g * opts.stride * 4);
        keys[g] = lsort_make_key(first_obj, &opts);
        obj_mod.write_i32(idx_buf + g * 4, @intCast(g));
    }

    // Insertion sort on the parallel idx table (stable, simple,
    // adequate for the bundle sizes lsort handles in tcltest).
    var ii: u32 = 1;
    while (ii < groups) : (ii += 1) {
        const cur_idx: u32 = @intCast(@as(i32, @bitCast(obj_mod.read_i32(idx_buf + ii * 4))));
        var jj: i32 = @intCast(ii);
        while (jj > 0) {
            const prev_idx: u32 = @intCast(@as(i32, @bitCast(obj_mod.read_i32(idx_buf + (@as(u32, @intCast(jj)) - 1) * 4))));
            if (lsort_compare(&keys[prev_idx], &keys[cur_idx], &opts) <= 0) break;
            obj_mod.write_i32(idx_buf + @as(u32, @intCast(jj)) * 4, @intCast(prev_idx));
            jj -= 1;
        }
        obj_mod.write_i32(idx_buf + @as(u32, @intCast(jj)) * 4, @intCast(cur_idx));
    }

    // -unique semantics: per the Tcl man page, "If a duplicate is
    // found, only the last one in the list is retained."  Walk the
    // sorted index table backwards to mark each duplicate group's
    // LAST sorted position as the keeper, then emit in forward order
    // skipping any non-keeper slots.
    const skip_arr_size: u32 = groups;
    var skip_buf: u32 = 0;
    if (opts.unique) {
        skip_buf = obj_mod.alloc(skip_arr_size);
        if (skip_buf == 0) return result_mod.from_globals(0);
        const skip: [*]u8 = @ptrFromInt(skip_buf);
        // Initialise: 0 = keep, 1 = drop.
        var z: u32 = 0;
        while (z < groups) : (z += 1) skip[z] = 0;
        // Walk forward: any element equal to the next sorted slot's
        // key gets dropped (since the next-or-later is the "last"
        // duplicate that should be retained).  Use the *active*
        // comparison mode so ``lsort -integer -unique {01 1 1}``
        // dedupes ``01`` and ``1`` (numerically equal) rather than
        // treating their textual representations as distinct.
        var p: u32 = 0;
        while (p + 1 < groups) : (p += 1) {
            const a_idx: u32 = @intCast(@as(i32, @bitCast(obj_mod.read_i32(idx_buf + p * 4))));
            const b_idx: u32 = @intCast(@as(i32, @bitCast(obj_mod.read_i32(idx_buf + (p + 1) * 4))));
            if (lsort_compare(&keys[a_idx], &keys[b_idx], &opts) == 0) {
                skip[p] = 1;
            }
        }
    }
    defer if (skip_buf != 0) obj_mod.free_sized(skip_buf, skip_arr_size);

    // Emit the result list — concatenate each group's elements (or
    // its original index when -indices).
    var result: i32 = obj_new_string(0, 0);
    var out_g: u32 = 0;
    while (out_g < groups) : (out_g += 1) {
        if (opts.unique) {
            const skip: [*]u8 = @ptrFromInt(skip_buf);
            if (skip[out_g] != 0) continue;
        }
        const src_g: u32 = @intCast(@as(i32, @bitCast(obj_mod.read_i32(idx_buf + out_g * 4))));
        if (opts.indices) {
            // Per cmdIL-1.28: for -stride > 1, indices return the
            // first-of-group position (src_g * stride).
            const base: i64 = @as(i64, src_g) * @as(i64, opts.stride);
            result = list_mod.tcl_cmd_lappend(result, obj_new_int(base));
        } else {
            // Append the whole group's elements in original order.
            var k: u32 = 0;
            while (k < opts.stride) : (k += 1) {
                const e = obj_mod.read_i32(elem_buf + (src_g * opts.stride + k) * 4);
                result = list_mod.tcl_cmd_lappend(result, e);
            }
        }
    }
    return result_mod.from_globals(result);
}

// lsearch full implementation.
// Supported: -exact -glob -all -not -nocase -inline -start -stride -index -subindices
// Recognised-but-linear: -sorted -decreasing -bisect -integer -real -dictionary
// Stub: -regexp (raises unsupported)
fn ls_opt_eq(ptr: u32, len: u32, lit: []const u8) bool {
    if (len != lit.len) return false;
    const sp: [*]const u8 = @ptrFromInt(ptr);
    for (lit, 0..) |c, k| if (sp[k] != c) return false;
    return true;
}

fn ls_tolower(c: u8) u8 {
    return if (c >= 'A' and c <= 'Z') c + 32 else c;
}

fn ls_match_exact(nocase: bool, pat_ptr: u32, pat_len: u32, ep: u32, elen: u32) bool {
    if (pat_len != elen) return false;
    if (pat_len == 0) return true;
    const pp: [*]const u8 = @ptrFromInt(pat_ptr);
    const vp: [*]const u8 = @ptrFromInt(ep);
    var k: u32 = 0;
    while (k < elen) : (k += 1) {
        const a = if (nocase) ls_tolower(pp[k]) else pp[k];
        const b = if (nocase) ls_tolower(vp[k]) else vp[k];
        if (a != b) return false;
    }
    return true;
}

fn ls_match_glob_nc(pat_ptr: u32, pat_len: u32, ep: u32, elen: u32) bool {
    // Case-insensitive glob: lowercase copies then call glob_match.
    const bp = alloc(pat_len + 1);
    const be = alloc(elen + 1);
    if (pat_len > 0 and bp != 0) {
        const src: [*]const u8 = @ptrFromInt(pat_ptr);
        const dst: [*]u8 = @ptrFromInt(bp);
        var k: u32 = 0;
        while (k < pat_len) : (k += 1) dst[k] = ls_tolower(src[k]);
    }
    if (elen > 0 and be != 0) {
        const src: [*]const u8 = @ptrFromInt(ep);
        const dst: [*]u8 = @ptrFromInt(be);
        var k: u32 = 0;
        while (k < elen) : (k += 1) dst[k] = ls_tolower(src[k]);
    }
    return tcl_str.glob_match(bp, pat_len, be, elen);
}

// Build a two-element index list "outer_idx inner_idx" for -subindices output.
fn ls_subindex_pair(outer: i64, inner: i64) i32 {
    return rt.tcl_list(rt.tcl_list(obj_new_string(0, 0), obj_new_int(outer)), obj_new_int(inner));
}

/// Comparison mode for ``lsearch -bisect`` / ``-sorted`` typed
/// comparisons.  ``string`` (the default) is the byte-wise compare;
/// ``integer`` / ``real`` parse both sides as numbers; ``dictionary``
/// is case-insensitive with embedded numeric runs compared numerically
/// (matching ``lsort -dictionary``).
const BisectCmp = enum { string, integer, real, dictionary };

/// Compare two byte-string spans under the given comparison mode.
/// Returns negative if a < b, zero if equal, positive if a > b.
/// Used by the ``lsearch -bisect`` search path; non-numeric inputs in
/// ``-integer`` / ``-real`` modes fall through to byte-wise compare
/// (matching reference Tcl's lenient ``Tcl_GetIntFromObj`` failure
/// handling — the test corpus only exercises well-formed numbers).
fn bisect_compare(a_ptr: u32, a_len: u32, b_ptr: u32, b_len: u32, kind: BisectCmp) i32 {
    const bignum = @import("../valtypes/tcl_bignum.zig");
    switch (kind) {
        .integer => {
            const av = bignum.parse_i128(a_ptr, a_len);
            const bv = bignum.parse_i128(b_ptr, b_len);
            if (av != null and bv != null) {
                const a = av.?;
                const b = bv.?;
                if (a < b) return -1;
                if (a > b) return 1;
                return 0;
            }
            // Fall through to byte-wise on parse failure.
        },
        .real => {
            const a_opt = parse_real(a_ptr, a_len);
            const b_opt = parse_real(b_ptr, b_len);
            if (a_opt != null and b_opt != null) {
                const a = a_opt.?;
                const b = b_opt.?;
                if (a < b) return -1;
                if (a > b) return 1;
                return 0;
            }
        },
        .dictionary => {
            return dictionary_compare(a_ptr, a_len, b_ptr, b_len);
        },
        .string => {},
    }
    return byte_compare(a_ptr, a_len, b_ptr, b_len);
}

fn byte_compare(a_ptr: u32, a_len: u32, b_ptr: u32, b_len: u32) i32 {
    if (a_ptr == 0 or b_ptr == 0) {
        if (a_len < b_len) return -1;
        if (a_len > b_len) return 1;
        return 0;
    }
    const ap: [*]const u8 = @ptrFromInt(a_ptr);
    const bp: [*]const u8 = @ptrFromInt(b_ptr);
    const n = if (a_len < b_len) a_len else b_len;
    var i: u32 = 0;
    while (i < n) : (i += 1) {
        if (ap[i] < bp[i]) return -1;
        if (ap[i] > bp[i]) return 1;
    }
    if (a_len < b_len) return -1;
    if (a_len > b_len) return 1;
    return 0;
}

/// Parse a Tcl real (decimal float).  Minimal parser — accepts the
/// shapes the lsearch-22 test corpus exercises.  Returns null when the
/// span isn't a clean float literal so the caller can fall back to
/// byte-wise compare.
fn parse_real(ptr: u32, len: u32) ?f64 {
    if (ptr == 0 or len == 0) return null;
    const sp: [*]const u8 = @ptrFromInt(ptr);
    var s: []const u8 = sp[0..len];
    // Trim surrounding ASCII whitespace.
    while (s.len > 0 and (s[0] == ' ' or s[0] == '\t' or s[0] == '\n' or s[0] == '\r')) s = s[1..];
    while (s.len > 0 and (s[s.len - 1] == ' ' or s[s.len - 1] == '\t' or s[s.len - 1] == '\n' or s[s.len - 1] == '\r')) s = s[0 .. s.len - 1];
    if (s.len == 0) return null;
    return std.fmt.parseFloat(f64, s) catch null;
}

/// Tcl ``lsort -dictionary`` comparator — case-insensitive ASCII with
/// embedded decimal runs compared numerically.  Mirrors the reference
/// ``DictionaryCompare`` in ``tclCmdIL.c`` for the cases the test
/// corpus exercises (ASCII letters, digits, plain whitespace).
fn dictionary_compare(a_ptr: u32, a_len: u32, b_ptr: u32, b_len: u32) i32 {
    if (a_ptr == 0 or b_ptr == 0) return byte_compare(a_ptr, a_len, b_ptr, b_len);
    const ap: [*]const u8 = @ptrFromInt(a_ptr);
    const bp: [*]const u8 = @ptrFromInt(b_ptr);
    var i: u32 = 0;
    var j: u32 = 0;
    while (i < a_len and j < b_len) {
        const ca = ap[i];
        const cb = bp[j];
        if (ca >= '0' and ca <= '9' and cb >= '0' and cb <= '9') {
            // Compare two decimal runs as numbers.  Strip leading
            // zeros so ``08`` and ``8`` compare equal in value.
            var ai = i;
            while (ai < a_len and ap[ai] == '0' and ai + 1 < a_len and ap[ai + 1] >= '0' and ap[ai + 1] <= '9') ai += 1;
            var bj = j;
            while (bj < b_len and bp[bj] == '0' and bj + 1 < b_len and bp[bj + 1] >= '0' and bp[bj + 1] <= '9') bj += 1;
            const a_start = ai;
            const b_start = bj;
            while (ai < a_len and ap[ai] >= '0' and ap[ai] <= '9') ai += 1;
            while (bj < b_len and bp[bj] >= '0' and bp[bj] <= '9') bj += 1;
            const a_run_len = ai - a_start;
            const b_run_len = bj - b_start;
            if (a_run_len != b_run_len) {
                return if (a_run_len < b_run_len) -1 else 1;
            }
            var k: u32 = 0;
            while (k < a_run_len) : (k += 1) {
                if (ap[a_start + k] != bp[b_start + k]) {
                    return if (ap[a_start + k] < bp[b_start + k]) -1 else 1;
                }
            }
            i = ai;
            j = bj;
            continue;
        }
        const la: u8 = if (ca >= 'A' and ca <= 'Z') ca + 32 else ca;
        const lb: u8 = if (cb >= 'A' and cb <= 'Z') cb + 32 else cb;
        if (la != lb) return if (la < lb) -1 else 1;
        i += 1;
        j += 1;
    }
    if (i < a_len) return 1;
    if (j < b_len) return -1;
    return 0;
}

/// Drill into element at ``outer_idx`` using ``index_arg`` (the path)
/// to produce the search-target bytes.  Same shape as
/// :func:`ls_get_match_target` but used by the bisect search path
/// which already knows the resolved outer index and walks the rest of
/// the path directly.
fn ls_bisect_target(
    ls_ptr: u32,
    ls_len: u32,
    outer_idx: i64,
    index_arg: i32,
    skip_first_path: bool,
) struct { ep: u32, elen: u32 } {
    const outer_elem = rt.list_element_at(ls_ptr, ls_len, outer_idx);
    var outer_ptr: u32 = ls_ptr + outer_elem.start;
    var outer_len: u32 = outer_elem.len;
    if (!outer_elem.braced and outer_len > 0) {
        const buf = alloc(outer_len + 4);
        if (buf != 0) {
            const n = rt.copy_unbraced_elem(buf, outer_ptr, outer_len);
            outer_ptr = buf;
            outer_len = n;
        }
    }
    if (index_arg == 0) {
        return .{ .ep = outer_ptr, .elen = outer_len };
    }
    const ps = obj_ensure_string(index_arg);
    if (ps.len == 0) {
        return .{ .ep = outer_ptr, .elen = outer_len };
    }
    const path_count = rt.list_count_elements(ps.ptr, ps.len);
    if (path_count == 0) {
        return .{ .ep = outer_ptr, .elen = outer_len };
    }
    var current: i32 = obj_new_string(@bitCast(outer_ptr), @bitCast(outer_len));
    var p: i64 = if (skip_first_path) 1 else 0;
    while (p < path_count) : (p += 1) {
        const idx_obj = rt.tcl_cmd_list_index(index_arg, obj_new_int(p));
        current = rt.tcl_cmd_list_index(current, idx_obj);
    }
    const final_s = obj_ensure_string(current);
    return .{ .ep = final_s.ptr, .elen = final_s.len };
}

// Build a list of sub-indices for ``lsearch -subindices`` output.
//
// With ``-stride 1`` (the default): output is ``{outer p0 p1 ...}`` —
// the outer index followed by every level of the ``-index`` path.
//
// With ``-stride > 1``: the FIRST path element selects the element
// inside the group (relative to ``outer``), so the result is
// ``{outer+p0 p1 p2 ...}`` — only ``p1...`` survive as sub-indices.
// Tracks reference Tcl 9 ``Tcl_LsearchObjCmd`` in ``tclCmdIL.c``
// (lsearch-26.* / 27.* expect the merged-first-index form).
//
// Each path level is resolved against the actual element at that
// level (rather than emitted verbatim), so ``-index end`` reports
// the numeric tail index — lsearch-19.7 expects ``{0 1}``, not
// ``{0 end}``.
fn ls_subindex_path(ls_ptr: u32, ls_len: u32, outer: i64, path_obj: i32, stride: i64) i32 {
    if (path_obj == 0) return obj_new_int(outer);
    const ps = obj_ensure_string(path_obj);
    if (ps.len == 0) return obj_new_int(outer);
    const path_count = rt.list_count_elements(ps.ptr, ps.len);
    if (path_count == 0) return obj_new_int(outer);
    var start_index: i64 = 0;
    var first_outer: i64 = outer;
    if (stride > 1) {
        const first_obj = rt.tcl_cmd_list_index(path_obj, obj_new_int(0));
        const offset = list_mod.resolve_list_index(first_obj, stride);
        first_outer = outer + offset;
        start_index = 1;
    }
    // Walk the same drilling path the matcher took so each ``end`` /
    // ``end-N`` / arithmetic index reports the absolute number it
    // resolved to.  ``current`` starts at the outer element (or, for
    // stride > 1, the within-group target).
    const elem = rt.list_element_at(ls_ptr, ls_len, first_outer);
    var current_ptr: u32 = ls_ptr + elem.start;
    var current_len: u32 = elem.len;
    if (!elem.braced and current_len > 0) {
        const buf = alloc(current_len + 4);
        if (buf != 0) {
            const decoded = rt.copy_unbraced_elem(buf, current_ptr, current_len);
            current_ptr = buf;
            current_len = decoded;
        }
    }
    var current_obj: i32 = obj_new_string(@bitCast(current_ptr), @bitCast(current_len));
    var result: i32 = rt.tcl_list(obj_new_string(0, 0), obj_new_int(first_outer));
    var i: i64 = start_index;
    while (i < path_count) : (i += 1) {
        const idx_obj = rt.tcl_cmd_list_index(path_obj, obj_new_int(i));
        const cs = obj_ensure_string(current_obj);
        const n_cur = rt.list_count_elements(cs.ptr, cs.len);
        const resolved = list_mod.resolve_list_index(idx_obj, n_cur);
        result = rt.tcl_list(result, obj_new_int(resolved));
        current_obj = rt.tcl_cmd_list_index(current_obj, idx_obj);
    }
    return result;
}

// Get the matchable bytes for the search target at *idx*, optionally
// drilling via an ``-index`` *path*.  ``stride`` distinguishes the two
// shapes of the path:
//
//   * ``stride == 1``: ``idx`` selects the outer element; the FULL
//     path drills into it.
//   * ``stride > 1``: ``idx`` is the group start, ``path[0]``
//     selects the element WITHIN the group (full position
//     ``idx + path[0]``) and the rest of the path drills into that
//     element.
//
// Returns (ep, elen, sub_idx) — sub_idx is the FIRST drill level's
// index, retained for the legacy ``ls_subindex_pair`` caller.  Multi-
// level subindices use :func:`ls_subindex_path` instead.
fn ls_get_match_target(
    ls_ptr: u32,
    ls_len: u32,
    idx: i64,
    index_arg: i32,
    stride: i64,
) struct { ep: u32, elen: u32, sub_idx: i64 } {
    // Compute the outer position the search target belongs to.  With
    // stride > 1 and a path, ``path[0]`` shifts the group-relative
    // offset.
    var outer_idx: i64 = idx;
    var skip_first: bool = false;
    if (stride > 1 and index_arg != 0) {
        const ps_check = obj_ensure_string(index_arg);
        if (ps_check.len > 0) {
            const pc_check = rt.list_count_elements(ps_check.ptr, ps_check.len);
            if (pc_check > 0) {
                const first_obj = rt.tcl_cmd_list_index(index_arg, obj_new_int(0));
                const offset = list_mod.resolve_list_index(first_obj, stride);
                outer_idx = idx + offset;
                skip_first = true;
            }
        }
    }
    const outer_elem = rt.list_element_at(ls_ptr, ls_len, outer_idx);
    var outer_ptr: u32 = ls_ptr + outer_elem.start;
    var outer_len: u32 = outer_elem.len;
    if (!outer_elem.braced and outer_len > 0) {
        const buf = alloc(outer_len + 4);
        if (buf != 0) {
            const n = rt.copy_unbraced_elem(buf, outer_ptr, outer_len);
            outer_ptr = buf;
            outer_len = n;
        }
    }
    if (index_arg == 0) {
        return .{ .ep = outer_ptr, .elen = outer_len, .sub_idx = 0 };
    }
    // Treat index_arg as an -index PATH: a list of indices walked
    // depth-first.  Empty path / non-list values fall through to the
    // legacy single-index behaviour (one drill level).
    const ps = obj_ensure_string(index_arg);
    if (ps.len == 0) {
        return .{ .ep = outer_ptr, .elen = outer_len, .sub_idx = 0 };
    }
    const path_count = rt.list_count_elements(ps.ptr, ps.len);
    if (path_count == 0) {
        return .{ .ep = outer_ptr, .elen = outer_len, .sub_idx = 0 };
    }
    // Build a TclObj for the outer element so tcl_cmd_list_index can
    // recursively descend.  ``obj_new_string`` here points into the
    // already-decoded scratch buffer, which is safe because the
    // caller keeps it alive for the duration of the search.
    var current: i32 = obj_new_string(@bitCast(outer_ptr), @bitCast(outer_len));
    var first_sub: i64 = 0;
    var p: i64 = if (skip_first) 1 else 0;
    var first_recorded = false;
    while (p < path_count) : (p += 1) {
        const idx_obj = rt.tcl_cmd_list_index(index_arg, obj_new_int(p));
        const cs = obj_ensure_string(current);
        const n_cur = rt.list_count_elements(cs.ptr, cs.len);
        const resolved = list_mod.resolve_list_index(idx_obj, n_cur);
        if (!first_recorded) {
            first_sub = resolved;
            first_recorded = true;
        }
        current = rt.tcl_cmd_list_index(current, idx_obj);
    }
    const final_s = obj_ensure_string(current);
    return .{
        .ep = final_s.ptr,
        .elen = final_s.len,
        .sub_idx = first_sub,
    };
}

/// ``lsearch -bisect`` implementation.  The list is assumed sorted in
/// ascending order (or descending when ``do_decreasing`` is set).
/// Returns the highest index where the element compares ``<=`` (or
/// ``>=`` for decreasing) the target under the active typing.  ``-1``
/// when no element satisfies the predicate.  ``-inline`` / ``-stride``
/// / ``-index`` interact the same way they do for the linear-search
/// path: ``-inline`` returns the value, ``-stride > 1`` walks one
/// group per step and the first ``-index`` element offsets within the
/// group.  Mirrors reference Tcl 9 ``Tcl_LsearchObjCmd`` for the
/// lsearch-22.* family.
fn do_lsearch_bisect(
    ls_ptr: u32,
    ls_len: u32,
    n: i64,
    pv_ptr: u32,
    pv_len: u32,
    start: i64,
    stride: i64,
    index_arg: i32,
    cmp_kind: BisectCmp,
    do_decreasing: bool,
    do_inline: bool,
    do_subindices: bool,
) i32 {
    var best: i64 = -1;
    // Walk the (sub-)list linearly — ``-bisect`` semantically expects
    // a sorted input, but we don't enforce it.  The linear walk
    // produces the same answer as a true binary search when the list
    // is sorted, at O(N) cost.  Large bisect workloads can swap to a
    // binary-search variant later without changing the return shape.
    var skip_first_path: bool = false;
    if (stride > 1 and index_arg != 0) {
        const ps = obj_ensure_string(index_arg);
        if (ps.len > 0) {
            const pc = rt.list_count_elements(ps.ptr, ps.len);
            if (pc > 0) skip_first_path = true;
        }
    }
    var idx: i64 = if (stride > 1) @divFloor(start, stride) * stride else start;
    while (idx < n) : (idx += stride) {
        var outer_idx: i64 = idx;
        if (stride > 1 and index_arg != 0 and skip_first_path) {
            const first_obj = rt.tcl_cmd_list_index(index_arg, obj_new_int(0));
            const offset = list_mod.resolve_list_index(first_obj, stride);
            outer_idx = idx + offset;
            if (outer_idx >= n) break;
        }
        const t = ls_bisect_target(ls_ptr, ls_len, outer_idx, index_arg, skip_first_path);
        const c = bisect_compare(t.ep, t.elen, pv_ptr, pv_len, cmp_kind);
        const ok = if (do_decreasing) c >= 0 else c <= 0;
        if (ok) {
            best = idx;
        } else {
            break;
        }
    }
    if (best < 0) {
        if (do_inline) return obj_new_string(0, 0);
        return obj_new_int(-1);
    }
    if (do_inline) {
        const t = ls_bisect_target(ls_ptr, ls_len, best, index_arg, skip_first_path);
        return obj_new_string(@bitCast(t.ep), @bitCast(t.elen));
    }
    if (do_subindices) {
        return ls_subindex_path(ls_ptr, ls_len, best, index_arg, stride);
    }
    return obj_new_int(best);
}

fn eval_lsearch(words: []const i32) result_mod.InterpResult {
    var mode: u8 = 'g'; // 'e'=exact 'g'=glob 'r'=regexp (stub)
    var find_all = false;
    var negate = false;
    var nocase = false;
    var do_inline = false;
    var start: i64 = 0;
    var start_obj: i32 = 0; // -start argument TclObj (0 = not given)
    var stride: i64 = 1;
    var index_arg: i32 = 0; // -index argument TclObj (0 = not given)
    var do_subindices = false;
    var do_bisect = false;
    var do_decreasing = false;
    var cmp_kind: BisectCmp = .string;

    const stubs = @import("../stubs/tcl_stubs.zig");
    var wi: u32 = 1;
    while (wi + 1 < words.len) : (wi += 1) {
        const sv = obj_ensure_string(words[wi]);
        if (sv.len == 0 or sv.ptr == 0) break;
        const sp: [*]const u8 = @ptrFromInt(sv.ptr);
        if (sp[0] != '-') break;
        if (ls_opt_eq(sv.ptr, sv.len, "--")) {
            wi += 1;
            break;
        } else if (ls_opt_eq(sv.ptr, sv.len, "-exact")) {
            mode = 'e';
        } else if (ls_opt_eq(sv.ptr, sv.len, "-glob")) {
            mode = 'g';
        } else if (ls_opt_eq(sv.ptr, sv.len, "-regexp")) {
            mode = 'r';
        } else if (ls_opt_eq(sv.ptr, sv.len, "-all")) {
            find_all = true;
        } else if (ls_opt_eq(sv.ptr, sv.len, "-not")) {
            negate = true;
        } else if (ls_opt_eq(sv.ptr, sv.len, "-nocase")) {
            nocase = true;
        } else if (ls_opt_eq(sv.ptr, sv.len, "-inline")) {
            do_inline = true;
        } else if (ls_opt_eq(sv.ptr, sv.len, "-subindices")) {
            do_subindices = true;
        } else if (ls_opt_eq(sv.ptr, sv.len, "-bisect")) {
            do_bisect = true;
        } else if (ls_opt_eq(sv.ptr, sv.len, "-decreasing")) {
            do_decreasing = true;
        } else if (ls_opt_eq(sv.ptr, sv.len, "-integer")) {
            cmp_kind = .integer;
        } else if (ls_opt_eq(sv.ptr, sv.len, "-real")) {
            cmp_kind = .real;
        } else if (ls_opt_eq(sv.ptr, sv.len, "-dictionary")) {
            cmp_kind = .dictionary;
        } else if (ls_opt_eq(sv.ptr, sv.len, "-sorted") or
            ls_opt_eq(sv.ptr, sv.len, "-increasing") or
            ls_opt_eq(sv.ptr, sv.len, "-ascii"))
        {
            // Recognised; fall back to linear search (correct result, slower).
        } else if (ls_opt_eq(sv.ptr, sv.len, "-start")) {
            wi += 1;
            if (wi + 1 >= words.len) break;
            // Resolve against the list length below, after we know
            // ``n``.  ``-start end-N`` (lsearch-10.3) and ``-start
            // 0xFF`` both need the list context.
            start_obj = words[wi];
        } else if (ls_opt_eq(sv.ptr, sv.len, "-stride")) {
            wi += 1;
            if (wi + 1 >= words.len) break;
            const v = obj_mod.obj_get_int(words[wi]);
            stride = if (v < 1) 1 else v;
        } else if (ls_opt_eq(sv.ptr, sv.len, "-index")) {
            wi += 1;
            if (wi + 1 >= words.len) break;
            index_arg = words[wi];
        } else {
            // Unknown option — raise a Tcl error rather than silently
            // misinterpreting the arg as the list.
            const stubs2 = @import("../stubs/tcl_stubs.zig");
            const prefix = "bad option \"";
            const suffix = "\": must be -all, -ascii, -bisect, -decreasing, -dictionary, -exact, -glob, -index, -inline, -integer, -nocase, -not, -real, -regexp, -sorted, -start, -stride, or -subindices";
            const name_len = sv.len;
            const total: u32 = @intCast(prefix.len + name_len + suffix.len);
            const buf = obj_mod.alloc(total);
            if (buf != 0) {
                const bp: [*]u8 = @ptrFromInt(buf);
                var off: u32 = 0;
                for (prefix) |c| {
                    bp[off] = c;
                    off += 1;
                }
                if (sv.ptr != 0) {
                    const sp2: [*]const u8 = @ptrFromInt(sv.ptr);
                    for (0..name_len) |k| {
                        bp[off] = sp2[k];
                        off += 1;
                    }
                }
                for (suffix) |c| {
                    bp[off] = c;
                    off += 1;
                }
                const msg = obj_mod.obj_new_string_take(buf, total, total);
                const catch_mod = @import("../interp/tcl_catch.zig");
                catch_mod.tcl_cmd_error(msg);
            } else {
                stubs2.raise("bad option to lsearch");
            }
            return result_mod.from_globals(0);
        }
    }

    if (wi + 1 >= words.len) return result_mod.from_globals(obj_new_int(-1));
    const list_obj = words[wi];
    const pat_obj = words[wi + 1];

    _ = stubs; // suppress unused warning when not used below

    const ls = obj_ensure_string(list_obj);
    const pv = obj_ensure_string(pat_obj);
    const n = rt.list_count_elements(ls.ptr, ls.len);

    // Resolve ``-start`` now that ``n`` is known.  Reference Tcl
    // accepts ``end[-+]N`` and integer arithmetic here (lsearch-10.3
    // / 10.4 / 10.5).  Negative resolutions clamp to 0.
    if (start_obj != 0) {
        const v = list_mod.resolve_list_index(start_obj, n);
        start = if (v < 0) 0 else v;
    }

    // ``-bisect`` uses a separate search path — the list is assumed
    // sorted in the direction implied by ``-increasing`` / ``-decreasing``
    // and the return value is the highest index whose element
    // compares ``<=`` (ascending) or ``>=`` (decreasing) the target
    // under the active ``-integer`` / ``-real`` / ``-dictionary`` /
    // ``-ascii`` typing.  ``-1`` when no element satisfies the
    // predicate.  Matches reference Tcl semantics for lsearch-22.*.
    if (do_bisect) {
        return result_mod.from_globals(do_lsearch_bisect(ls.ptr, ls.len, n, pv.ptr, pv.len, start, stride, index_arg, cmp_kind, do_decreasing, do_inline, do_subindices));
    }

    // Align idx to the first stride group that covers [start, n).
    var idx: i64 = if (stride > 1) @divFloor(start, stride) * stride else start;

    var acc: i32 = obj_new_string(0, 0); // -all accumulator

    if (index_arg == 0) {
        // Fast O(N) sequential scan via cursor — avoids the O(N²) that
        // repeated list_element_at(idx) would incur for large lists.
        var cur = list_parse.Cursor{ .pos = 0 };
        if (idx > 0)
            list_parse.cursor_skip(ls.ptr, ls.len, &cur, @intCast(idx));
        while (idx < n) {
            const elem = list_parse.cursor_next(ls.ptr, ls.len, &cur);
            // Decode the element (unbraced elements need backslash expansion).
            var ep: u32 = ls.ptr + elem.start;
            var elen: u32 = elem.len;
            if (!elem.braced and elen > 0) {
                const buf = alloc(elen + 4);
                if (buf != 0) {
                    const decoded = rt.copy_unbraced_elem(buf, ep, elen);
                    ep = buf;
                    elen = decoded;
                }
            }
            const raw: bool = switch (mode) {
                'e' => ls_match_exact(nocase, pv.ptr, pv.len, ep, elen),
                'r' => blk: {
                    const tcl_regex = @import("../valtypes/tcl_regex.zig");
                    break :blk tcl_regex.run_match_pub(pv.ptr, pv.len, ep, elen, nocase);
                },
                else => if (nocase)
                    ls_match_glob_nc(pv.ptr, pv.len, ep, elen)
                else
                    tcl_str.glob_match(pv.ptr, pv.len, ep, elen),
            };
            const matched = if (negate) !raw else raw;
            if (matched) {
                if (!find_all) {
                    if (do_inline) return result_mod.from_globals(obj_new_string(@bitCast(ep), @bitCast(elen)));
                    return result_mod.from_globals(obj_new_int(idx));
                }
                const entry: i32 = if (do_inline)
                    obj_new_string(@bitCast(ep), @bitCast(elen))
                else
                    obj_new_int(idx);
                acc = list_mod.tcl_cmd_lappend(acc, entry);
            }
            idx += stride;
            if (stride > 1 and idx < n)
                list_parse.cursor_skip(ls.ptr, ls.len, &cur, @intCast(stride - 1));
        }
    } else {
        // -index path: random-access (lists using -index are typically small).
        while (idx < n) : (idx += stride) {
            const t = ls_get_match_target(ls.ptr, ls.len, idx, index_arg, stride);
            const raw: bool = switch (mode) {
                'e' => ls_match_exact(nocase, pv.ptr, pv.len, t.ep, t.elen),
                'r' => blk: {
                    const tcl_regex = @import("../valtypes/tcl_regex.zig");
                    break :blk tcl_regex.run_match_pub(pv.ptr, pv.len, t.ep, t.elen, nocase);
                },
                else => if (nocase)
                    ls_match_glob_nc(pv.ptr, pv.len, t.ep, t.elen)
                else
                    tcl_str.glob_match(pv.ptr, pv.len, t.ep, t.elen),
            };
            const matched = if (negate) !raw else raw;
            if (!matched) continue;
            if (!find_all) {
                if (do_inline) return result_mod.from_globals(obj_new_string(@bitCast(t.ep), @bitCast(t.elen)));
                if (do_subindices) return result_mod.from_globals(ls_subindex_path(ls.ptr, ls.len, idx, index_arg, stride));
                return result_mod.from_globals(obj_new_int(idx));
            }
            const entry: i32 = if (do_inline)
                obj_new_string(@bitCast(t.ep), @bitCast(t.elen))
            else if (do_subindices)
                ls_subindex_path(ls.ptr, ls.len, idx, index_arg, stride)
            else
                obj_new_int(idx);
            acc = list_mod.tcl_cmd_lappend(acc, entry);
        }
    }

    if (find_all) return result_mod.from_globals(acc);
    if (do_inline) return result_mod.from_globals(obj_new_string(0, 0));
    return result_mod.from_globals(obj_new_int(-1));
}

fn eval_lrange(words: []const i32) result_mod.InterpResult {
    if (words.len != 4) {
        const stubs = @import("../stubs/tcl_stubs.zig");
        stubs.raise("wrong # args: should be \"lrange list first last\"");
        return result_mod.from_globals(0);
    }
    // Validate list syntax — raises ``unmatched open brace in list``
    // / ``list element in quotes followed by ... instead of space`` /
    // ``list element in braces followed by ... instead of space``
    // when *list* is malformed.  Reference Tcl: ``Tcl_LrangeObjCmd``
    // calls ``TclListObjGetElements`` which fails before any index
    // resolution.
    const ls = obj_ensure_string(words[1]);
    if (list_parse.check_list_syntax(ls.ptr, ls.len) != 0)
        return result_mod.from_globals(0);
    // Validate index syntax — ``lrange L b 6`` must raise ``bad
    // index "b"`` rather than silently treating ``b`` as 0.
    if (!list_mod.is_valid_list_index(words[2])) {
        list_mod.raise_bad_list_index(words[2]);
        return result_mod.from_globals(0);
    }
    if (!list_mod.is_valid_list_index(words[3])) {
        list_mod.raise_bad_list_index(words[3]);
        return result_mod.from_globals(0);
    }
    return result_mod.from_globals(rt.tcl_cmd_list_range(words[1], words[2], words[3]));
}

fn eval_concat(words: []const i32) result_mod.InterpResult {
    if (words.len <= 1) return result_mod.from_globals(obj_new_string(0, 0));
    var acc = words[1];
    var ci: usize = 2;
    while (ci < words.len) : (ci += 1) {
        acc = rt.tcl_cmd_concat(acc, words[ci]);
    }
    return result_mod.from_globals(acc);
}

fn eval_join(words: []const i32) result_mod.InterpResult {
    const stubs = @import("../stubs/tcl_stubs.zig");
    if (words.len < 2 or words.len > 3) {
        stubs.raise("wrong # args: should be \"join list ?joinString?\"");
        return result_mod.from_globals(0);
    }
    const ls = obj_ensure_string(words[1]);
    if (list_parse.check_list_syntax(ls.ptr, ls.len) != 0)
        return result_mod.from_globals(0);
    if (words.len == 3) return result_mod.from_globals(rt.tcl_cmd_join(words[1], words[2]));
    const sp = alloc(1);
    const d: [*]u8 = @ptrFromInt(sp);
    d[0] = ' ';
    return result_mod.from_globals(rt.tcl_cmd_join(words[1], obj_new_string(@bitCast(sp), 1)));
}

fn eval_split(words: []const i32) result_mod.InterpResult {
    const stubs = @import("../stubs/tcl_stubs.zig");
    if (words.len < 2 or words.len > 3) {
        stubs.raise("wrong # args: should be \"split string ?splitChars?\"");
        return result_mod.from_globals(0);
    }
    if (words.len == 3) return result_mod.from_globals(rt.tcl_cmd_split(words[1], words[2]));
    // Default split chars: the four whitespace bytes ``" \t\n\r"``.
    // Empty splitChars means a different thing in Tcl (split every
    // character into its own element); we must NOT route the no-arg
    // form through the empty-string fast path or split-1.1 / split-1.4
    // / split-1.7 fail with one-char-per-byte output instead of the
    // expected whitespace-tokenised list.
    const default_chars: []const u8 = " \t\n\r";
    const buf = alloc(@intCast(default_chars.len));
    if (buf == 0) return result_mod.from_globals(0);
    const d: [*]u8 = @ptrFromInt(buf);
    for (default_chars, 0..) |c, i| d[i] = c;
    const sep = rt.obj_new_string_copy(buf, @intCast(default_chars.len));
    obj_mod.free_sized(buf, @intCast(default_chars.len));
    const result = rt.tcl_cmd_split(words[1], sep);
    obj_mod.tcl_obj_release(sep);
    return result_mod.from_globals(result);
}

fn eval_lreverse(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) return result_mod.from_globals(obj_new_string(0, 0));
    return result_mod.from_globals(rt.tcl_cmd_list_reverse(words[1]));
}

fn eval_lrepeat(words: []const i32) result_mod.InterpResult {
    if (words.len < 3) return result_mod.from_globals(obj_new_string(0, 0));
    if (words.len == 3) return result_mod.from_globals(rt.tcl_cmd_list_repeat(words[1], words[2]));
    // Multi-value: build one cycle then repeat it count times.
    const count_val = rt.obj_get_int(words[1]);
    if (count_val <= 0) return result_mod.from_globals(obj_new_string(0, 0));
    const count: u32 = @intCast(count_val);
    var cycle_max: u32 = 0;
    var vi: u32 = 2;
    while (vi < words.len) : (vi += 1) {
        const s = obj_ensure_string(words[vi]);
        cycle_max += s.len * 2 + 2;
        if (vi > 2) cycle_max += 1;
    }
    const cycle_buf = alloc(cycle_max + 4);
    var cycle_off: u32 = 0;
    vi = 2;
    while (vi < words.len) : (vi += 1) {
        if (vi > 2) {
            const d: [*]u8 = @ptrFromInt(cycle_buf + cycle_off);
            d[0] = ' ';
            cycle_off += 1;
        }
        const s = obj_ensure_string(words[vi]);
        if (vi == 2) {
            cycle_off = obj_mod.list_elem_quote(cycle_buf, cycle_off, s.ptr, s.len);
        } else {
            cycle_off = obj_mod.list_elem_quote_nth(cycle_buf, cycle_off, s.ptr, s.len);
        }
    }
    if (cycle_off == 0) return result_mod.from_globals(obj_new_string(0, 0));
    const total: u32 = count * (cycle_off + 1);
    const result_buf = alloc(total);
    var off: u32 = 0;
    var ci: u32 = 0;
    while (ci < count) : (ci += 1) {
        if (ci > 0) {
            const d: [*]u8 = @ptrFromInt(result_buf + off);
            d[0] = ' ';
            off += 1;
        }
        rt.memcpy(result_buf + off, cycle_buf, cycle_off);
        off += cycle_off;
    }
    return result_mod.from_globals(obj_new_string(@bitCast(result_buf), @bitCast(off)));
}

fn eval_lassign(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) return result_mod.from_globals(obj_new_string(0, 0));
    const list_obj = words[1];
    const list_s = obj_ensure_string(list_obj);
    const n = rt.list_count_elements(list_s.ptr, list_s.len);
    var pi: u32 = 2;
    var i: i64 = 0;
    while (pi < words.len) : (pi += 1) {
        const val: i32 = if (i < n) blk: {
            const elem = rt.list_element_at(list_s.ptr, list_s.len, i);
            if (elem.braced) {
                break :blk rt.obj_new_string_copy(list_s.ptr + elem.start, elem.len);
            } else {
                const buf = alloc(elem.len + 4);
                const out_len = rt.copy_unbraced_elem(buf, list_s.ptr + elem.start, elem.len);
                break :blk obj_new_string(@bitCast(buf), @bitCast(out_len));
            }
        } else obj_new_string(0, 0);
        _ = frames.var_set(words[pi], val);
        i += 1;
    }
    const assigned: i64 = @intCast(words.len - 2);
    if (assigned >= n or n == 0) return result_mod.from_globals(obj_new_string(0, 0));
    const start_obj = obj_new_int(assigned);
    return result_mod.from_globals(list_mod.list_tail(list_obj, start_obj));
}

fn eval_lmap(words: []const i32) result_mod.InterpResult {
    // lmap varList1 list1 ?varList2 list2 ...? body — same multi-var
    // semantics as ``foreach``, but accumulates body results into a list
    // instead of discarding them.  ``varListN`` may itself be a list of
    // variable names, in which case each iteration consumes that many
    // elements from ``listN`` and binds them in parallel.  Iteration
    // count = ``max(ceil(len(listN)/len(varListN)))`` across pairs.
    const stubs = @import("../stubs/tcl_stubs.zig");
    const wrong_args = "wrong # args: should be \"lmap varList list ?varList list ...? command\"";
    if (words.len < 4) {
        stubs.raise(wrong_args);
        return result_mod.from_globals(0);
    }
    const pair_words = words.len - 2;
    if (pair_words % 2 != 0) {
        stubs.raise(wrong_args);
        return result_mod.from_globals(0);
    }
    const n_pairs = pair_words / 2;
    // Mirrors ``eval_foreach`` in ``tcl_interp.zig`` — derived from
    // ``parse.MAX_WORDS`` so the cap tracks the parser's word limit
    // and doesn't go stale when the parser cap moves (Copilot review
    // on PR #443).
    const MAX_PAIRS = (parse.MAX_WORDS - 2) / 2;
    if (n_pairs > MAX_PAIRS) return result_mod.from_globals(obj_new_string(0, 0));
    const body_s = obj_ensure_string(words[words.len - 1]);

    var list_lens: [MAX_PAIRS]i64 = [_]i64{0} ** MAX_PAIRS;
    var steps: [MAX_PAIRS]i64 = [_]i64{1} ** MAX_PAIRS;
    var n: i64 = 0;
    {
        var p: u32 = 0;
        while (p < n_pairs) : (p += 1) {
            const varlist_obj = words[1 + p * 2];
            const vls = obj_ensure_string(varlist_obj);
            if (list_parse.check_list_syntax(vls.ptr, vls.len) != 0)
                return result_mod.from_globals(0);
            const count = rt.list_count_elements(vls.ptr, vls.len);
            if (count == 0) {
                stubs.raise("lmap varlist is empty");
                return result_mod.from_globals(0);
            }
            steps[p] = count;

            const ls = obj_ensure_string(words[2 + p * 2]);
            if (list_parse.check_list_syntax(ls.ptr, ls.len) != 0)
                return result_mod.from_globals(0);
            const len = rt.list_count_elements(ls.ptr, ls.len);
            list_lens[p] = len;
            const iters: i64 = @divTrunc(len + count - 1, count);
            if (iters > n) n = iters;
        }
    }

    var result: i32 = obj_new_string(0, 0);
    var idx: i64 = 0;
    while (idx < n) : (idx += 1) {
        var p: u32 = 0;
        while (p < n_pairs) : (p += 1) {
            const varlist_obj = words[1 + p * 2];
            const list_obj = words[2 + p * 2];
            const vls = obj_ensure_string(varlist_obj);
            const ls = obj_ensure_string(list_obj);
            const step = steps[p];
            var j: i64 = 0;
            while (j < step) : (j += 1) {
                const elem_idx = idx * step + j;
                const var_elem = rt.list_element_at(vls.ptr, vls.len, j);
                const src_vname = vls.ptr + var_elem.start;
                const var_name_obj: i32 = if (var_elem.braced or src_vname == 0)
                    rt.obj_new_string_copy(src_vname, var_elem.len)
                else inner: {
                    const buf = alloc(var_elem.len + 1);
                    if (buf == 0) break :inner obj_new_string(0, 0);
                    const out_len = rt.copy_unbraced_elem(buf, src_vname, var_elem.len);
                    break :inner obj_mod.obj_new_string_take(buf, out_len, var_elem.len + 1);
                };
                const elem_val: i32 = if (elem_idx < list_lens[p]) blk: {
                    const elem = rt.list_element_at(ls.ptr, ls.len, elem_idx);
                    const src_elem = ls.ptr + elem.start;
                    break :blk if (elem.braced or src_elem == 0)
                        rt.obj_new_string_copy(src_elem, elem.len)
                    else inner2: {
                        const buf = alloc(elem.len + 1);
                        if (buf == 0) break :inner2 obj_new_string(0, 0);
                        const out_len = rt.copy_unbraced_elem(buf, src_elem, elem.len);
                        break :inner2 obj_mod.obj_new_string_take(buf, out_len, elem.len + 1);
                    };
                } else obj_new_string(0, 0);
                _ = frames.var_set(var_name_obj, elem_val);
                obj_mod.tcl_obj_release(elem_val);
                obj_mod.tcl_obj_release(var_name_obj);
            }
        }
        const item = interp.eval_script(body_s.ptr, body_s.len);
        const ir = result_mod.snapshot(item);
        switch (ir.code) {
            .OK => result = rt.tcl_list(result, item),
            .BREAK => {
                result_mod.consume(.BREAK);
                if (item != 0) obj_mod.tcl_obj_release(item);
                break;
            },
            .CONTINUE => {
                result_mod.consume(.CONTINUE);
                if (item != 0) obj_mod.tcl_obj_release(item);
                continue;
            },
            .ERROR, .RETURN => return result_mod.from_globals(item),
        }
    }
    return result_mod.from_globals(result);
}

/// Match a separator word (``to`` / ``..``) at index ``i``.  Returns
/// true iff the i-th word is exactly that separator.
fn lseq_match_word(words: []const i32, i: u32, expected: []const u8) bool {
    if (i >= words.len) return false;
    const s = obj_ensure_string(words[i]);
    if (s.len != expected.len) return false;
    const sp: [*]const u8 = @ptrFromInt(s.ptr);
    for (expected, 0..) |c, k| {
        if (sp[k] != c) return false;
    }
    return true;
}

/// Tcl 9 ``lseq``.  Forms supported (matching tcl9.0.3 builtin):
///
///   lseq N                    -> 0 .. N-1
///   lseq START END            -> START .. END (step inferred)
///   lseq START to END         -> same
///   lseq START .. END         -> same
///   lseq START END by STEP    -> step explicit
///   lseq START to END by STEP -> same
///   lseq START .. END by STEP -> same
///   lseq START by STEP        -> N=START items, step from 0
///
/// Step direction is inferred from end-vs-start when no explicit
/// step is given.  Floats are NOT supported yet — this impl is
/// integer-only; tests that need ``arithSeriesDouble`` are SKIPPED
/// by tcltest's constraint table because ``arithSeriesDouble``
/// isn't set.
fn eval_lseq(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) return result_mod.from_globals(obj_new_string(0, 0));
    var start_val: i64 = 0;
    var end_val: i64 = 0;
    var step_val: i64 = 1;
    var have_step = false;

    if (words.len == 2) {
        // ``lseq N`` -> 0 .. N-1
        const count = rt.obj_get_int(words[1]);
        if (count <= 0) return result_mod.from_globals(obj_new_string(0, 0));
        end_val = count - 1;
    } else {
        // Two-or-more forms.  After ``words[1] = START``, scan for an
        // optional ``to``/``..`` separator (or its absence) and an
        // optional ``by STEP`` suffix.
        start_val = rt.obj_get_int(words[1]);
        var idx: u32 = 2;
        if (lseq_match_word(words, idx, "to") or lseq_match_word(words, idx, "..")) {
            idx += 1;
        }
        if (idx >= words.len) return result_mod.from_globals(obj_new_string(0, 0));
        // ``lseq START by STEP`` (no end) means N=START items; the
        // separator-less ``words[2]`` could be ``by`` instead of an
        // end value.
        if (lseq_match_word(words, idx, "by")) {
            // ``lseq START by STEP`` — start=0, count=START, step
            const cnt = start_val;
            start_val = 0;
            if (cnt <= 0) return result_mod.from_globals(obj_new_string(0, 0));
            if (idx + 1 < words.len) step_val = rt.obj_get_int(words[idx + 1]);
            if (step_val == 0) return result_mod.from_globals(obj_new_string(0, 0));
            // (cnt - 1) * step can overflow when cnt or step were
            // clamped from out-of-range floats (e.g. ``1e50``).
            // Bail to an empty list rather than panicking on
            // i64-overflow safety checks.
            const cnt_m1 = @subWithOverflow(cnt, 1);
            if (cnt_m1[1] != 0) return result_mod.from_globals(obj_new_string(0, 0));
            const prod = @mulWithOverflow(cnt_m1[0], step_val);
            if (prod[1] != 0) return result_mod.from_globals(obj_new_string(0, 0));
            const sum = @addWithOverflow(start_val, prod[0]);
            if (sum[1] != 0) return result_mod.from_globals(obj_new_string(0, 0));
            end_val = sum[0];
            have_step = true;
        } else {
            end_val = rt.obj_get_int(words[idx]);
            idx += 1;
            if (lseq_match_word(words, idx, "by")) {
                if (idx + 1 < words.len) step_val = rt.obj_get_int(words[idx + 1]);
                have_step = true;
            } else if (idx < words.len) {
                // ``lseq START END STEP`` (or ``lseq START to END STEP``)
                // — Tcl 9 accepts the trailing positional as the step
                // without an explicit ``by`` keyword.  Without this
                // branch, ``lseq 1000000 2000000 100000`` falls through
                // with step=1 and tries to enumerate a million-element
                // sequence with O(N²) tcl_list appends — that's the
                // lseq.test hang at lseq-1.16.
                step_val = rt.obj_get_int(words[idx]);
                have_step = true;
            }
        }

        if (!have_step) {
            // Infer direction from start..end.
            if (end_val < start_val) step_val = -1 else step_val = 1;
        }
    }
    if (step_val == 0) return result_mod.from_globals(obj_new_string(0, 0));

    // Sanity bound: ``lseq 1e50 1e50+1`` and similar large-double
    // forms convert to out-of-range i64 via ``@intFromFloat`` and
    // can otherwise loop for billions of iterations before tripping
    // the wasmtime watchdog.  Cap the absolute span so a poorly
    // formed call returns an empty list (or partial result) instead
    // of hanging the test runner.  The cap is conservative — well
    // above any realistic production lseq — but small enough that
    // a runaway terminates in <100ms.
    //
    // Use overflow-safe subtraction: when start/end are at the i64
    // extremes (e.g. clamped from ``1e50``) the unguarded ``end - start``
    // would itself trip the integer-overflow safety panic.
    const max_count: i64 = 16 * 1024 * 1024; // 16 M elements
    const span_res = if (step_val > 0)
        @subWithOverflow(end_val, start_val)
    else
        @subWithOverflow(start_val, end_val);
    if (span_res[1] != 0) return result_mod.from_globals(obj_new_string(0, 0));
    const span: i64 = span_res[0];
    // ``abs(step_val)`` via overflow-safe negation: ``-i64_min``
    // would itself panic, so detect it explicitly and bail.
    const abs_step_res = if (step_val > 0)
        .{ step_val, @as(u1, 0) }
    else
        @subWithOverflow(@as(i64, 0), step_val);
    if (abs_step_res[1] != 0) return result_mod.from_globals(obj_new_string(0, 0));
    const abs_step: i64 = abs_step_res[0];
    if (span < 0 or @divTrunc(span, abs_step) > max_count) {
        return result_mod.from_globals(obj_new_string(0, 0));
    }

    var acc: i32 = obj_new_string(0, 0);
    var i: i64 = start_val;
    if (step_val > 0) {
        while (i <= end_val) {
            // ``tcl_list`` allocates a fresh accumulator each call
            // and does not consume its inputs (see
            // ``valtypes/tcl_list.zig``).  Without explicit releases
            // we'd leak one accumulator and one element obj per
            // iteration — up to 16 M of each at the cap above.
            const elem = obj_new_int(i);
            const new_acc = rt.tcl_list(acc, elem);
            obj_mod.tcl_obj_release(acc);
            obj_mod.tcl_obj_release(elem);
            acc = new_acc;
            // Stop before the increment overflows — this can happen
            // when ``end_val`` is at i64_max (clamped from a float)
            // and the loop emits the final element.
            const next = @addWithOverflow(i, step_val);
            if (next[1] != 0) break;
            i = next[0];
        }
    } else {
        while (i >= end_val) {
            const elem = obj_new_int(i);
            const new_acc = rt.tcl_list(acc, elem);
            obj_mod.tcl_obj_release(acc);
            obj_mod.tcl_obj_release(elem);
            acc = new_acc;
            const next = @addWithOverflow(i, step_val);
            if (next[1] != 0) break;
            i = next[0];
        }
    }
    return result_mod.from_globals(acc);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "list", .arity_min = 0, .arity_max = null, .handler = &eval_list },
    .{ .name = "lappend", .arity_min = 1, .arity_max = null, .handler = &eval_lappend },
    .{ .name = "llength", .arity_min = 1, .arity_max = 1, .handler = &eval_llength },
    .{ .name = "lindex", .arity_min = 1, .arity_max = null, .handler = &eval_lindex },
    .{ .name = "lset", .arity_min = 2, .arity_max = null, .handler = &eval_lset },
    .{ .name = "linsert", .arity_min = 2, .arity_max = null, .handler = &eval_linsert },
    .{ .name = "lreplace", .arity_min = 3, .arity_max = null, .handler = &eval_lreplace },
    .{ .name = "ledit", .arity_min = 3, .arity_max = null, .handler = &eval_ledit },
    .{ .name = "lsort", .arity_min = 1, .arity_max = null, .handler = &eval_lsort },
    .{ .name = "lsearch", .arity_min = 2, .arity_max = null, .handler = &eval_lsearch },
    .{ .name = "lrange", .arity_min = 3, .arity_max = 3, .handler = &eval_lrange },
    .{ .name = "concat", .arity_min = 0, .arity_max = null, .handler = &eval_concat },
    .{ .name = "join", .arity_min = 1, .arity_max = 2, .handler = &eval_join },
    .{ .name = "split", .arity_min = 1, .arity_max = 2, .handler = &eval_split },
    .{ .name = "lreverse", .arity_min = 1, .arity_max = 1, .handler = &eval_lreverse },
    .{ .name = "lrepeat", .arity_min = 1, .arity_max = null, .handler = &eval_lrepeat },
    .{ .name = "lassign", .arity_min = 1, .arity_max = null, .handler = &eval_lassign },
    .{ .name = "lmap", .arity_min = 3, .arity_max = null, .handler = &eval_lmap },
    .{ .name = "lseq", .arity_min = 1, .arity_max = 5, .handler = &eval_lseq },
};
